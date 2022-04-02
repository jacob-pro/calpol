use crate::api::error::{CalpolApiError, MapDieselUniqueViolation};
use crate::api::{api_resource, response_mapper};
use crate::database::{
    NewTest, Test, TestRepository, TestRepositoryImpl, TestResultRepository,
    TestResultRepositoryImpl,
};
use crate::state::AppState;
use actix_web::http::StatusCode;
use actix_web::web::{Data, Path, ServiceConfig};
use actix_web::{web, Responder};
use calpol_model::api_v1::{CreateTestRequest, TestSummary, UpdateTestRequest};
use diesel::Connection;
use diesel_repository::CrudRepository;
use futures::FutureExt;
use http_api_problem::ApiError;
use std::convert::TryFrom;

pub fn configure(tests: &mut ServiceConfig) {
    tests.service(
        api_resource("")
            .route(web::get().to(list))
            .route(web::post().to(create)),
    );
    tests.service(
        api_resource("{test_name}")
            .route(web::get().to(get))
            .route(web::delete().to(delete))
            .route(web::put().to(update)),
    );
}

async fn list(state: Data<AppState>) -> impl Responder {
    web::block(move || {
        let database = state.database();
        let test_repository = TestRepositoryImpl::new(&database);
        let tests: Result<Vec<_>, _> = test_repository
            .find_all()?
            .into_iter()
            .map(TestSummary::try_from)
            .collect();
        tests
    })
    .map(response_mapper)
    .await
}

async fn create(
    state: Data<AppState>,
    json: actix_web_validator::Json<CreateTestRequest>,
) -> impl Responder {
    web::block(move || {
        let database = state.database();
        let test_repository = TestRepositoryImpl::new(&database);
        let test = test_repository
            .insert(&NewTest {
                name: json.name.clone(),
                enabled: json.enabled,
                config: serde_json::to_value(&json.config).unwrap(),
                failing: false,
                failure_threshold: json.failure_threshold as i32,
            })
            .map_unique_violation(|_| {
                ApiError::builder(StatusCode::CONFLICT)
                    .message("Test with this name already exists")
                    .finish()
                    .into()
            })?;
        TestSummary::try_from(test)
    })
    .map(response_mapper)
    .await
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

async fn get(state: Data<AppState>, test_name: Path<String>) -> impl Responder {
    web::block(move || {
        let database = state.database();
        let test_repository = TestRepositoryImpl::new(&database);
        let test = retrieve_test(&test_repository, test_name.as_ref())?;
        TestSummary::try_from(test)
    })
    .map(response_mapper)
    .await
}

async fn update(
    test_name: Path<String>,
    json: actix_web_validator::Json<UpdateTestRequest>,
    state: Data<AppState>,
) -> impl Responder {
    web::block(move || {
        let database = state.database();
        let body = json.into_inner();
        let test_repository = TestRepositoryImpl::new(&database);
        let mut test = retrieve_test(&test_repository, test_name.as_str())?;
        if let Some(enabled) = body.enabled {
            test.enabled = enabled;
        }
        if let Some(config) = body.config {
            test.config = serde_json::to_value(config).unwrap();
        }
        if let Some(failure_threshold) = body.failure_threshold {
            test.failure_threshold = failure_threshold as i32;
        }
        test_repository.update(&test)?;
        TestSummary::try_from(test)
    })
    .map(response_mapper)
    .await
}

async fn delete(test_name: Path<String>, state: Data<AppState>) -> impl Responder {
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
    .map(response_mapper)
    .await
}
