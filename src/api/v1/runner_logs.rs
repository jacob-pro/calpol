use crate::api::error::CalpolApiError;
use crate::api::{api_resource, JsonResponse};
use crate::database::{RunnerLogRepository, RunnerLogRepositoryImpl};
use crate::state::AppState;
use actix_web::web::{Data, ServiceConfig};
use actix_web::{web, HttpResponse};
use calpol_model::api_v1::{ListRunnerLogsRequest, ListRunnerLogsResponse};

pub fn configure(tests: &mut ServiceConfig) {
    tests.service(api_resource("").route(web::get().to(list)));
}

async fn list(
    state: Data<AppState>,
    json: actix_web_validator::Json<ListRunnerLogsRequest>,
) -> Result<HttpResponse, CalpolApiError> {
    web::block(move || -> Result<_, CalpolApiError> {
        let database = state.database();
        let log_repository = RunnerLogRepositoryImpl::new(&database);
        let logs = log_repository.find_all(json.limit, json.offset)?;
        let response = ListRunnerLogsResponse {
            logs: logs.results.into_iter().map(|l| l.into()).collect(),
            total: logs.count,
        };
        Ok(response)
    })
    .await?
    .map(JsonResponse::json_response)
}
