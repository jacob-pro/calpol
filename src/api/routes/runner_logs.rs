use crate::api::auth::authenticator;
use crate::api::error::CalpolApiError;
use crate::api::models::{ListRunnerLogsRequest, ListRunnerLogsResponse};
use crate::api::{api_resource, api_scope, JsonResponse};
use crate::database::{RunnerLogRepository, RunnerLogRepositoryImpl};
use crate::state::AppState;
use actix_web::web::{Data, ServiceConfig};
use actix_web::{web, HttpResponse};
use actix_web_httpauth::middleware::HttpAuthentication;

pub fn configure(api: &mut ServiceConfig) {
    let auth = HttpAuthentication::with_fn(authenticator);
    api.service(
        api_scope("runner_logs")
            .service(api_resource("").route(web::get().to(list)))
            .wrap(auth),
    );
}

/// List the test runner logs
#[utoipa::path(
    get,
    path = "/api/runner_logs",
    tag = "RunnerLogs",
    operation_id = "ListRunnerLogs",
    responses(
        (status = 200, description = "List of test runner logs", body = ListRunnerLogsResponse),
        (status = "default", response = CalpolApiError)
    ),
)]
async fn list(
    state: Data<AppState>,
    json: actix_web_validator::Json<ListRunnerLogsRequest>,
) -> Result<HttpResponse, CalpolApiError> {
    web::block(move || -> Result<_, CalpolApiError> {
        let database = state.database();
        let log_repository = RunnerLogRepositoryImpl::new(&database);
        let logs = log_repository.find_all(json.limit, json.offset)?;
        let response = ListRunnerLogsResponse {
            items: logs.results.into_iter().map(|l| l.into()).collect(),
            total: logs.count,
        };
        Ok(response)
    })
    .await?
    .map(JsonResponse::json_response)
}
