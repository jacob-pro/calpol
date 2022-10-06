use actix_web::web::ServiceConfig;
use utoipa::openapi::security::{Http, HttpAuthScheme, SecurityScheme};
use utoipa::{Modify, OpenApi};
use utoipa_swagger_ui::SwaggerUi;

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
            crate::model::api_v1::ListRunnerLogsResponse,
            crate::model::api_v1::RunnerLog,
            crate::model::api_v1::SubmitPasswordResetRequest,
            crate::model::api_v1::PasswordResetRequest,
            crate::model::api_v1::LoginResponse,
            crate::model::api_v1::LoginRequest,
            crate::model::api_v1::UserSummary,
            crate::model::api_v1::SessionSummary,
            crate::model::api_v1::ListSessionsResponse,
            crate::model::api_v1::ListTestResultsResponse,
            crate::model::api_v1::ListTestsResponse,
            crate::model::api_v1::CreateTestRequest,
            crate::model::api_v1::TestSummary,
            crate::model::api_v1::UpdateTestRequest,
        ),
        responses(
            crate::api::error::CalpolApiError,
        )
    ),
    paths(
        crate::api::v1::password_reset::request,
        crate::api::v1::password_reset::submit,
        crate::api::v1::runner::queue,
        crate::api::v1::runner_logs::list,
        crate::api::v1::sessions::login,
        crate::api::v1::sessions::logout,
        crate::api::v1::sessions::list,
        crate::api::v1::sessions::delete,
        crate::api::v1::test_results::list,
        crate::api::v1::tests::list,
        crate::api::v1::tests::create,
        crate::api::v1::tests::update,
    ),
    modifiers(&SecurityAddon)
)]
struct UtoipaSpec;

/// YAML serialized OpenAPI spec generated from the Utoipa annotations.
pub fn api_yaml() -> String {
    serde_yaml::to_string(&UtoipaSpec::openapi()).unwrap()
}

/// Adds Swagger UI routes to the web server
pub fn configure_swagger_ui(api: &mut ServiceConfig) {
    api.service(
        SwaggerUi::new("/swagger-ui/{_:.*}").url("/api-doc/openapi.json", UtoipaSpec::openapi()),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    #[test]
    fn test_api_spec_matches() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("spec")
            .join("api.yaml");
        let on_disk = fs::read_to_string(&path)
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
