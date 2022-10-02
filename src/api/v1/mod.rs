mod converters;
pub mod password_reset;
pub mod runner;
pub mod runner_logs;
pub mod sessions;
pub mod test_results;
pub mod tests;
pub mod users;

use crate::api::api_scope;
use actix_extensible_rate_limit::backend::memory::InMemoryBackend;
use actix_web::web::ServiceConfig;

pub fn configure(api: &mut ServiceConfig, rate_limit_backend: &InMemoryBackend) {
    api.service(
        api_scope("v1")
            .configure(|v1| sessions::configure(v1, rate_limit_backend))
            .configure(users::configure)
            .configure(|v1| password_reset::configure(v1, rate_limit_backend))
            .configure(tests::configure)
            .configure(test_results::configure)
            .configure(runner_logs::configure)
            .configure(runner::configure),
    );
}
