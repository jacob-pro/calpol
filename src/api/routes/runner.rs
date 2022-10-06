use crate::api::auth::authenticator;
use crate::api::error::CalpolApiError;
use crate::api::{api_resource, api_scope, JsonResponse};
use crate::state::AppState;
use actix_web::web::{Data, ServiceConfig};
use actix_web::{web, HttpResponse};
use actix_web_httpauth::middleware::HttpAuthentication;

pub fn configure(api: &mut ServiceConfig) {
    let auth = HttpAuthentication::with_fn(authenticator);
    api.service(
        api_scope("runner")
            .service(api_resource("queue").route(web::post().to(queue)))
            .wrap(auth),
    );
}

/// Queue the test runner to immediately re-run.
#[utoipa::path(
    post,
    path = "/api/runner/queue",
    tag = "Runner",
    operation_id = "Queue",
    responses(
        (status = 200, description = "Success"),
        (status = "default", response = CalpolApiError),
    ),
)]
pub async fn queue(state: Data<AppState>) -> Result<HttpResponse, CalpolApiError> {
    state.queue_test_run();
    Ok(().json_response())
}
