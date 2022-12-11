use crate::api::auth::authenticator;
use crate::api::error::CalpolApiError;
use crate::api::models::{
    ListRunnerLogsRequest, ListRunnerLogsResponse, RunnerLog, DEFAULT_PAGE_SIZE,
};
use crate::api::{api_resource, api_scope};
use crate::database2::RunnerLogRepository;
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
    request_body = ListRunnerLogsRequest,
    responses(
        (status = 200, description = "List of test runner logs", body = ListRunnerLogsResponse),
        (status = "default", response = CalpolApiError)
    ),
)]
async fn list(
    state: Data<AppState>,
    params: actix_web_validator::Json<ListRunnerLogsRequest>,
) -> Result<HttpResponse, CalpolApiError> {
    let log_repository = RunnerLogRepository::new(&state.database);
    let results = log_repository
        .find(
            params.page_token.as_deref(),
            params.page_size.unwrap_or(DEFAULT_PAGE_SIZE).into(),
            params.sort_order.unwrap_or_default().into(),
        )
        .await?;
    let response = ListRunnerLogsResponse {
        items: results.rows.into_iter().map(RunnerLog::from).collect(),
        next_page: results.next_page,
    };
    Ok(HttpResponse::Ok().json(response))
}
