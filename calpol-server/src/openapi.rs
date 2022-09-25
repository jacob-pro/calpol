#![allow(unused)]

#[macro_use]
extern crate diesel_migrations;
embed_migrations!();
#[macro_use]
extern crate diesel;

mod api;
mod database;
mod messagebird;
mod schema;
mod settings;
mod state;
mod test_runner;

use utoipa::OpenApi;


#[derive(OpenApi)]
#[openapi(
    paths(api::v1::re_run)
)]
struct ApiDoc;

fn main() {
    println!("{}", ApiDoc::openapi().to_pretty_json().unwrap());
}
