use crate::api::error::ApiErrorMap;
use crate::api::v1::converters;
use crate::api::v1::tests::retrieve_test;
use crate::api::{api_resource, response_mapper};
use crate::database::{TestRepositoryImpl, TestResultRepository, TestResultRepositoryImpl};
use crate::state::AppState;
use actix_web::web::{Data, Path, ServiceConfig};
use actix_web::{web, Responder};
use calpol_model::api_v1::GetTestResultsRequest;
use diesel_repository::CrudRepository;
use futures::FutureExt;
use http_api_problem::ApiError;

pub fn configure(tests: &mut ServiceConfig) {
    tests.service(api_resource("").route(web::get().to(list)));
    tests.service(api_resource("{test_name}").route(web::get().to(get)));
}

async fn list(state: Data<AppState>) -> impl Responder {
    web::block(move || {
        let database = state.database();
        let test_repository = TestRepositoryImpl::new(&database);
        let test_result_repository = TestResultRepositoryImpl::new(&database);
        let tests = test_repository
            .find_all()
            .map_api_error()?
            .into_iter()
            .map(|test| {
                let latest = test_result_repository
                    .find_latest_belonging_to(&test, 1)
                    .map_api_error()?;
                Ok((test, latest.into_iter().next()))
            })
            .collect::<Result<Vec<_>, ApiError>>()?;
        let summaries = tests
            .into_iter()
            .filter_map(|(t, r)| r.map(|r| (t, r)))
            .map(|(t, r)| converters::test_and_result_to_summary(&t, r))
            .collect::<Vec<_>>();
        Ok(summaries)
    })
    .map(response_mapper)
    .await
}

async fn get(
    state: Data<AppState>,
    test_name: Path<String>,
    json: actix_web_validator::Json<GetTestResultsRequest>,
) -> impl Responder {
    web::block(move || {
        let database = state.database();
        let test_repository = TestRepositoryImpl::new(&database);
        let test_result_repository = TestResultRepositoryImpl::new(&database);
        let test = retrieve_test(&test_repository, test_name.as_ref())?;
        let results = test_result_repository
            .find_latest_belonging_to(&test, json.limit)
            .map_api_error()?
            .into_iter()
            .map(|res| converters::test_and_result_to_summary(&test, res))
            .collect::<Vec<_>>();
        Ok(results)
    })
    .map(response_mapper)
    .await
}
