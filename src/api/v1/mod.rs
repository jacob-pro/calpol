mod converters;
mod password_reset;
mod runner_logs;
mod sessions;
mod test_results;
mod tests;
mod users;

use crate::api::auth::authenticator;
use crate::api::{api_scope, auth_rate_limiter};
use actix_ratelimit::MemoryStore;
use actix_web::web::ServiceConfig;
use actix_web_httpauth::middleware::HttpAuthentication;

pub fn configure(v1: &mut ServiceConfig, rate_limit_store: &MemoryStore) {
    let auth = HttpAuthentication::with_fn(authenticator);
    v1.service(api_scope("sessions").configure(|c| sessions::configure(c, rate_limit_store)));
    v1.service(
        api_scope("users")
            .configure(users::configure)
            .wrap(auth.clone()),
    );
    v1.service(
        api_scope("password_reset")
            .configure(password_reset::configure)
            .wrap(auth_rate_limiter(rate_limit_store)),
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
            .wrap(auth),
    );
}
