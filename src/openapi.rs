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

use crate::api::error::CalpolApiError;
use std::fs;
use std::path::PathBuf;
use utoipa::openapi::security::{Http, HttpAuthScheme, SecurityScheme};
use utoipa::openapi::{
    Content, ContentBuilder, Object, ObjectBuilder, Response, ResponseBuilder, Schema,
    SecurityRequirement,
};
use utoipa::{Modify, OpenApi, ToResponse};

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
            model::api_v1::SubmitPasswordResetRequest,
        ),
        responses(
            api::error::CalpolApiError,
        )
    ),
    paths(
        api::v1::password_reset::request,
        api::v1::password_reset::submit,
        api::v1::runner::queue,
        api::v1::runner_logs::list,
    ),
    modifiers(&SecurityAddon)
)]
struct ApiDoc;

fn api_yaml() -> String {
    serde_yaml::to_string(&ApiDoc::openapi()).unwrap()
}

fn path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("spec")
        .join("api.yaml")
}

fn main() {
    fs::write(path(), api_yaml());
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_api_spec_matches() {
        let on_disk = fs::read_to_string(path())
            .expect("Unable to read api spec")
            .lines()
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>();
        let actual = api_yaml()
            .lines()
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>();
        if on_disk != actual {
            panic!("API spec doesn't match. Run `make spec` to regenerate it")
        }
    }
}
