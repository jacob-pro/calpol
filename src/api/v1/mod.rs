mod converters;
mod password_reset;
mod runner_logs;
mod sessions;
mod test_results;
mod tests;
mod users;

use crate::api::auth::authenticator;
use crate::api::error::CalpolApiError;
use crate::api::{api_resource, api_scope, JsonResponse};
use crate::AppState;
use actix_extensible_rate_limit::backend::memory::InMemoryBackend;
use actix_web::web::{Data, ServiceConfig};
use actix_web::{web, HttpResponse};
use actix_web_httpauth::middleware::HttpAuthentication;

pub fn configure(api: &mut ServiceConfig, rate_limit_backend: &InMemoryBackend) {
    let auth = HttpAuthentication::with_fn(authenticator);
    api.service(
        api_scope("v1")
            .configure(|v1| sessions::configure(v1, rate_limit_backend))
            .configure(users::configure)
            .configure(|v1| password_reset::configure(v1, rate_limit_backend))
            .configure(tests::configure)
            .configure(test_results::configure)
            .configure(runner_logs::configure)
            .service(
                api_resource("re_run")
                    .route(web::post().to(re_run))
                    .wrap(auth),
            ),
    );
}

async fn re_run(state: Data<AppState>) -> Result<HttpResponse, CalpolApiError> {
    state.queue_test_run();
    Ok(().json_response())
}
