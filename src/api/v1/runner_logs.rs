use crate::api::error::ApiErrorMap;
use crate::api::{api_resource, response_mapper};
use crate::database::{RunnerLogRepository, RunnerLogRepositoryImpl};
use crate::state::AppState;
use actix_web::web::{Data, ServiceConfig};
use actix_web::{web, Responder};
use calpol_model::api_v1::{ListRunnerLogsRequest, ListRunnerLogsResponse};
use futures::FutureExt;

pub fn configure(tests: &mut ServiceConfig) {
    tests.service(api_resource("").route(web::get().to(list)));
}

async fn list(
    state: Data<AppState>,
    json: actix_web_validator::Json<ListRunnerLogsRequest>,
) -> impl Responder {
    web::block(move || {
        let database = state.database();
        let log_repository = RunnerLogRepositoryImpl::new(&database);
        let logs = log_repository
            .find_all(json.limit, json.offset)
            .map_api_error()?;
        let response = ListRunnerLogsResponse {
            logs: logs.results.into_iter().map(|l| l.into()).collect(),
            total: logs.count,
        };
        Ok(response)
    })
    .map(response_mapper)
    .await
}