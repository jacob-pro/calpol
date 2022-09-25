use crate::api::auth::authenticator;
use crate::api::error::CalpolApiError;
use crate::api::v1::converters;
use crate::api::v1::tests::retrieve_test;
use crate::api::{api_resource, api_scope, JsonResponse};
use crate::database::{TestRepositoryImpl, TestResultRepository, TestResultRepositoryImpl};
use crate::state::AppState;
use actix_web::web::{Data, Path, ServiceConfig};
use actix_web::{web, HttpResponse};
use actix_web_httpauth::middleware::HttpAuthentication;
use crate::model::api_v1::GetTestResultsRequest;
use diesel_repository::CrudRepository;

pub fn configure(v1: &mut ServiceConfig) {
    let auth = HttpAuthentication::with_fn(authenticator);
    v1.service(
        api_scope("test_results")
            .service(api_resource("").route(web::get().to(list)))
            .service(api_resource("{test_name}").route(web::get().to(get)))
            .wrap(auth),
    );
}

async fn list(state: Data<AppState>) -> Result<HttpResponse, CalpolApiError> {
    web::block(move || -> Result<_, CalpolApiError> {
        let database = state.database();
        let test_repository = TestRepositoryImpl::new(&database);
        let test_result_repository = TestResultRepositoryImpl::new(&database);
        let tests = test_repository
            .find_all()?
            .into_iter()
            .map(|test| {
                let latest = test_result_repository.find_latest_belonging_to(&test, 1)?;
                Ok((test, latest.into_iter().next()))
            })
            .collect::<Result<Vec<_>, CalpolApiError>>()?;
        let summaries = tests
            .into_iter()
            .filter_map(|(t, r)| r.map(|r| (t, r)))
            .map(|(t, r)| converters::test_and_result_to_summary(&t, r))
            .collect::<Vec<_>>();
        Ok(summaries)
    })
    .await?
    .map(JsonResponse::json_response)
}

async fn get(
    state: Data<AppState>,
    test_name: Path<String>,
    json: actix_web_validator::Json<GetTestResultsRequest>,
) -> Result<HttpResponse, CalpolApiError> {
    web::block(move || -> Result<_, CalpolApiError> {
        let database = state.database();
        let test_repository = TestRepositoryImpl::new(&database);
        let test_result_repository = TestResultRepositoryImpl::new(&database);
        let test = retrieve_test(&test_repository, test_name.as_ref())?;
        let results = test_result_repository
            .find_latest_belonging_to(&test, json.limit)?
            .into_iter()
            .map(|res| converters::test_and_result_to_summary(&test, res))
            .collect::<Vec<_>>();
        Ok(results)
    })
    .await?
    .map(JsonResponse::json_response)
}
