mod converters;
mod password_reset;
mod runner_logs;
mod sessions;
mod test_results;
mod tests;
mod users;

use crate::api::auth::authenticator;
use crate::api::error::CalpolApiError;
use crate::api::{api_resource, api_scope, auth_rate_limiter, JsonResponse};
use crate::AppState;
use actix_extensible_rate_limit::backend::memory::InMemoryBackend;
use actix_web::web::{Data, ServiceConfig};
use actix_web::{web, HttpResponse};
use actix_web_httpauth::middleware::HttpAuthentication;

pub fn configure(v1: &mut ServiceConfig, rate_limit_backend: &InMemoryBackend) {
    let auth = HttpAuthentication::with_fn(authenticator);
    v1.service(api_scope("sessions").configure(|c| sessions::configure(c, rate_limit_backend)));
    v1.service(
        api_scope("users")
            .configure(users::configure)
            .wrap(auth.clone()),
    );
    v1.service(
        api_scope("password_reset")
            .configure(password_reset::configure)
            .wrap(auth_rate_limiter(rate_limit_backend)),
    );
    v1.service(
        api_scope("tests")
            .configure(tests::configure)
            .wrap(auth.clone()),
    );
    v1.service(
        api_scope("test_results")
            .configure(test_results::configure)
            .wrap(auth.clone()),
    );
    v1.service(
        api_scope("runner_logs")
            .configure(runner_logs::configure)
            .wrap(auth.clone()),
    );
    v1.service(
        api_resource("re_run")
            .route(web::post().to(re_run))
            .wrap(auth),
    );
}

async fn re_run(state: Data<AppState>) -> Result<HttpResponse, CalpolApiError> {
    state.queue_test_run();
    Ok(().json_response())
}
