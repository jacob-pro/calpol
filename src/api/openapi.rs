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
            crate::api::models::ListRunnerLogsResponse,
            crate::api::models::RunnerLog,
            crate::api::models::SubmitPasswordResetRequest,
            crate::api::models::PasswordResetRequest,
            crate::api::models::LoginResponse,
            crate::api::models::LoginRequest,
            crate::api::models::UserSummary,
            crate::api::models::SessionSummary,
            crate::api::models::ListSessionsResponse,
            crate::api::models::ListTestResultsResponse,
            crate::api::models::TestResultSummary,
            crate::api::models::ListTestsResponse,
            crate::api::models::CreateTestRequest,
            crate::api::models::TestSummary,
            crate::api::models::UpdateTestRequest,
            crate::api::models::ListUsersRequest,
            crate::api::models::ListUsersResponse,
            crate::api::models::UpdateUserRequest,
            crate::api::models::CreateUserRequest,
        ),
        responses(
            crate::api::error::CalpolApiError,
        )
    ),
    paths(
        crate::api::routes::password_reset::request,
        crate::api::routes::password_reset::submit,
        crate::api::routes::runner::queue,
        crate::api::routes::runner_logs::list,
        crate::api::routes::sessions::login,
        crate::api::routes::sessions::logout,
        crate::api::routes::sessions::list,
        crate::api::routes::sessions::delete,
        crate::api::routes::test_results::list,
        crate::api::routes::tests::list,
        crate::api::routes::tests::create,
        crate::api::routes::tests::get,
        crate::api::routes::tests::update,
        crate::api::routes::tests::delete,
        crate::api::routes::users::list,
        crate::api::routes::users::create,
        crate::api::routes::users::get,
        crate::api::routes::users::update,
        crate::api::routes::users::delete,
        crate::api::routes::users::test_email,
        crate::api::routes::users::test_sms,
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
