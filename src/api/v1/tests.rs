use crate::api::auth::authenticator;
use crate::api::error::{CalpolApiError, MapDieselUniqueViolation};
use crate::api::{api_resource, api_scope, JsonResponse};
use crate::database::{
    NewTest, Test, TestRepository, TestRepositoryImpl, TestResultRepository,
    TestResultRepositoryImpl,
};
use crate::model::api_v1::{CreateTestRequest, ListTestsResponse, TestSummary, UpdateTestRequest};
use crate::model::tests::TestConfig;
use crate::state::AppState;
use actix_web::http::StatusCode;
use actix_web::web::{Data, Path, ServiceConfig};
use actix_web::{web, HttpResponse};
use actix_web_httpauth::middleware::HttpAuthentication;
use diesel::Connection;
use diesel_repository::CrudRepository;
use http_api_problem::ApiError;
use validator::Validate;

pub fn configure(v1: &mut ServiceConfig) {
    let auth = HttpAuthentication::with_fn(authenticator);
    v1.service(
        api_scope("tests")
            .service(
                api_resource("")
                    .route(web::get().to(list))
                    .route(web::post().to(create)),
            )
            .service(
                api_resource("{test_name}")
                    .route(web::get().to(get))
                    .route(web::delete().to(delete))
                    .route(web::put().to(update)),
            )
            .wrap(auth),
    );
}

/// List tests
#[utoipa::path(
    get,
    path = "/v1/tests",
    tag = "Tests",
    operation_id = "ListTests",
    responses(
        (status = 200, description = "List of test results", body = ListTestsResponse),
        (status = "default", response = CalpolApiError)
    ),
)]
async fn list(state: Data<AppState>) -> Result<HttpResponse, CalpolApiError> {
    web::block(move || {
        let database = state.database();
        let test_repository = TestRepositoryImpl::new(&database);
        let tests = test_repository
            .find_all()?
            .into_iter()
            .map(TestSummary::from)
            .collect::<Vec<_>>();
        Ok(ListTestsResponse { items: tests })
    })
    .await?
    .map(JsonResponse::json_response)
}

/// Create a new test
#[utoipa::path(
    post,
    path = "/v1/tests",
    tag = "Tests",
    operation_id = "CreateTest",
    request_body = CreateTestRequest,
    responses(
        (status = 200, description = "The created test", body = TestSummary),
        (status = "default", response = CalpolApiError)
    ),
)]
async fn create(
    state: Data<AppState>,
    json: actix_web_validator::Json<CreateTestRequest>,
) -> Result<HttpResponse, CalpolApiError> {
    web::block(move || {
        let json = json.into_inner();
        let config = parse_config(json.config)?;
        let database = state.database();
        let test_repository = TestRepositoryImpl::new(&database);
        let test = test_repository
            .insert(&NewTest {
                name: json.name.clone(),
                enabled: json.enabled,
                config: serde_json::to_value(&config).unwrap(),
                failing: false,
                failure_threshold: json.failure_threshold as i32,
            })
            .map_unique_violation(|_| {
                ApiError::builder(StatusCode::CONFLICT)
                    .message("Test with this name already exists")
                    .finish()
                    .into()
            })?;
        Ok(TestSummary::from(test))
    })
    .await?
    .map(JsonResponse::json_response)
}

pub fn retrieve_test<'t, T>(test_repository: &T, test_name: &str) -> Result<Test, CalpolApiError>
where
    T: TestRepository + 't,
{
    test_repository.find_by_name(test_name)?.ok_or_else(|| {
        ApiError::builder(StatusCode::NOT_FOUND)
            .message("Test name not found")
            .finish()
            .into()
    })
}

/// Retrieve a test
#[utoipa::path(
    get,
    path = "/v1/tests/{name}",
    params(
        ("name" = String, Path, description = "Test name to get")
    ),
    tag = "Tests",
    operation_id = "GetTest",
    responses(
        (status = 200, description = "The created test", body = TestSummary),
        (status = "default", response = CalpolApiError)
    ),
)]
async fn get(
    state: Data<AppState>,
    test_name: Path<String>,
) -> Result<HttpResponse, CalpolApiError> {
    web::block(move || {
        let database = state.database();
        let test_repository = TestRepositoryImpl::new(&database);
        let test = retrieve_test(&test_repository, test_name.as_ref())?;
        Ok(TestSummary::from(test))
    })
    .await?
    .map(JsonResponse::json_response)
}

/// Update a test
#[utoipa::path(
    get,
    path = "/v1/tests/{name}",
    params(
        ("name" = String, Path, description = "Test name to update")
    ),
    tag = "Tests",
    operation_id = "UpdateTest",
    request_body = UpdateTestRequest,
    responses(
        (status = 200, description = "The created test", body = TestSummary),
        (status = "default", response = CalpolApiError)
    ),
)]
async fn update(
    test_name: Path<String>,
    json: actix_web_validator::Json<UpdateTestRequest>,
    state: Data<AppState>,
) -> Result<HttpResponse, CalpolApiError> {
    web::block(move || {
        let database = state.database();
        let body = json.into_inner();
        let test_repository = TestRepositoryImpl::new(&database);
        let mut test = retrieve_test(&test_repository, test_name.as_str())?;
        if let Some(enabled) = body.enabled {
            test.enabled = enabled;
        }
        if let Some(config) = body.config {
            let config = parse_config(config)?;
            test.config = serde_json::to_value(config).unwrap();
        }
        if let Some(failure_threshold) = body.failure_threshold {
            test.failure_threshold = failure_threshold as i32;
        }
        test_repository.update(&test)?;
        Ok(TestSummary::from(test))
    })
    .await?
    .map(JsonResponse::json_response)
}

/// Delete a test
#[utoipa::path(
    delete,
    path = "/v1/tests/{name}",
    params(
        ("name" = String, Path, description = "Test name to delete")
    ),
    tag = "Tests",
    operation_id = "DeleteTest",
    responses(
        (status = 200, description = "Success"),
        (status = "default", response = CalpolApiError)
    ),
)]
async fn delete(
    test_name: Path<String>,
    state: Data<AppState>,
) -> Result<HttpResponse, CalpolApiError> {
    web::block(move || -> Result<_, CalpolApiError> {
        let database = state.database();
        let test_repository = TestRepositoryImpl::new(&database);
        let test_results_repository = TestResultRepositoryImpl::new(&database);
        let test = retrieve_test(&test_repository, test_name.as_ref())?;
        database.transaction(|| -> Result<_, CalpolApiError> {
            test_results_repository.delete_all_belonging_to(&test)?;
            test_repository.delete(test)?;
            Ok(())
        })
    })
    .await?
    .map(JsonResponse::json_response)
}

fn parse_config(value: serde_json::Value) -> Result<TestConfig, CalpolApiError> {
    let config = serde_json::from_value::<TestConfig>(value).map_err(|e| {
        ApiError::builder(StatusCode::BAD_REQUEST)
            .message(format!("Invalid test config: {}", e))
            .finish()
    })?;
    config.validate()?;
    Ok(config)
}
