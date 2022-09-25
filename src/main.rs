#[macro_use]
extern crate diesel_migrations;
embed_migrations!();
#[macro_use]
extern crate diesel;

mod api;
mod database;
mod messagebird;
mod model;
mod schema;
mod settings;
mod state;
mod test_runner;

use crate::database::{Connection, NewUser, UserRepositoryImpl};
use crate::model::api_v1::UserSummary;
use crate::settings::Settings;
use actix_extensible_rate_limit::backend::memory::InMemoryBackend;
use actix_web::web::Data;
use actix_web::{middleware, App, HttpServer};
use anyhow::Context;
use clap::{Parser, Subcommand};
use diesel::r2d2::ConnectionManager;
use diesel::{r2d2, PgConnection};
use diesel_repository::CrudRepository;
use env_logger::Env;
use state::AppState;
use std::sync::Arc;

#[derive(Parser)]
#[clap(about, version, author)]
struct Opts {
    #[clap(subcommand)]
    subcommand: SubCommand,
    /// Config file
    #[clap(long, short)]
    config: Option<String>,
}

#[derive(Subcommand)]
enum SubCommand {
    /// Run the calpol server
    Server,
    /// Create a new user account
    CreateUser(CreateUser),
}

#[derive(Parser)]
struct CreateUser {
    #[clap(long)]
    email: String,
    #[clap(long)]
    password: String,
}

#[actix_web::main]
async fn main() -> anyhow::Result<()> {
    let opts: Opts = Opts::parse();
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    if cfg!(debug_assertions) {
        log::warn!("This is a debug build")
    };
    let settings = Arc::new(Settings::new(opts.config.as_ref())?);

    let manager = ConnectionManager::<PgConnection>::new(&settings.database_url);
    let pool = r2d2::Pool::builder().build(manager)?;
    embedded_migrations::run_with_output(&pool.get().unwrap(), &mut std::io::stdout())?;

    match opts.subcommand {
        SubCommand::Server => {
            let (tx, rx) = test_runner::make_channel();
            let state = AppState::new(Arc::clone(&settings), pool, tx)?;
            let rl_backend = InMemoryBackend::builder().build();
            let runner = test_runner::start(state.clone(), rx);
            let server = HttpServer::new(move || {
                App::new()
                    .app_data(Data::new(state.clone()))
                    .configure(|cfg| api::configure(cfg, &rl_backend))
                    .wrap(middleware::Logger::default())
            })
            .bind(&settings.api_socket)?
            .run();
            tokio::select! {
                result = runner => result.context("Test runner failed")?,
                result = server => result.context("Http Server failed")?,
            };
        }
        SubCommand::CreateUser(u) => create_user(pool.get().unwrap(), u)?,
    }
    Ok(())
}

fn create_user(connection: Connection, user: CreateUser) -> anyhow::Result<()> {
    let user_repository = UserRepositoryImpl::new(&connection);
    user_repository
        .insert(NewUser {
            name: "".to_string(),
            email: user.email.to_ascii_lowercase(),
            password_hash: Some(bcrypt::hash(user.password, bcrypt::DEFAULT_COST)?),
            sms_notifications: false,
            email_notifications: false,
            password_reset_token: None,
            password_reset_token_creation: None,
        })
        .map_err(|e| e.into())
        .map(|u| {
            println!(
                "{}",
                serde_json::to_string_pretty(&UserSummary::from(u)).unwrap()
            )
        })
}
