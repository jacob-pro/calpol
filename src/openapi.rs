#![allow(unused)]

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

use utoipa::openapi::security::{Http, HttpAuthScheme, SecurityScheme};
use utoipa::openapi::SecurityRequirement;
use utoipa::{Modify, OpenApi};

struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi.components.as_mut().unwrap(); // we can unwrap safely since there already is components registered.
        components.add_security_scheme(
            "basicAuth",
            SecurityScheme::Http(Http::new(HttpAuthScheme::Bearer)),
        );
    }
}

#[derive(OpenApi)]
#[openapi(
    security(("basicAuth" = [])),
    components(
        schemas(
            model::api_v1::ListRunnerLogsResponse,
            model::api_v1::RunnerLog,
        )
    ),
    paths(
        api::v1::re_run,
        api::v1::runner_logs::list,
    ),
    modifiers(&SecurityAddon)
)]
struct ApiDoc;

fn main() {
    println!("{}", ApiDoc::openapi().to_pretty_json().unwrap());
}
