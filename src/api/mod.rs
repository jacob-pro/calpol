mod auth;
mod error;
mod v1;

use crate::api::error::CalpolApiError;
use actix_extensible_rate_limit::backend::memory::InMemoryBackend;
use actix_extensible_rate_limit::backend::{SimpleInput, SimpleInputFunctionBuilder, SimpleOutput};
use actix_extensible_rate_limit::RateLimiter;
use actix_web::dev::ServiceRequest;
use actix_web::error::BlockingError;
use actix_web::http::StatusCode;
use actix_web::web::ServiceConfig;
use actix_web::{web, HttpResponse};
use http_api_problem::ApiError;
use serde::Serialize;
use std::future::Ready;
use std::time::Duration;

pub fn configure(api: &mut ServiceConfig, rate_limit_store: &InMemoryBackend) {
    api.app_data(
        actix_web::web::PathConfig::default().error_handler(|e, _| CalpolApiError::from(e).into()),
    );
    api.app_data(
        actix_web_validator::QueryConfig::default()
            .error_handler(|e, _| CalpolApiError::from(e).into()),
    );
    api.app_data(
        actix_web_validator::JsonConfig::default()
            .error_handler(|e, _| CalpolApiError::from(e).into()),
    );
    api.service(api_scope("v1").configure(|v1| v1::configure(v1, rate_limit_store)));
    api.service(api_resource("").route(web::get().to(index)));
}

pub fn response_mapper<T, E>(
    response: Result<Result<T, E>, BlockingError>,
) -> Result<HttpResponse, CalpolApiError>
where
    T: Serialize,
    E: Into<CalpolApiError>,
{
    response?
        .map(|value| HttpResponse::Ok().json(value))
        .map_err(|e| e.into())
}

fn api_scope(path: &str) -> actix_web::Scope {
    web::scope(path).default_service(web::route().to(|| async {
        ApiError::builder(StatusCode::NOT_FOUND)
            .finish()
            .into_actix_web_response()
    }))
}

fn api_resource(path: &str) -> actix_web::Resource {
    web::resource(path).default_service(web::route().to(|| async {
        ApiError::builder(StatusCode::METHOD_NOT_ALLOWED)
            .finish()
            .into_actix_web_response()
    }))
}

/// A rate limiter to protect authentication routes
fn auth_rate_limiter(
    backend: &InMemoryBackend,
) -> RateLimiter<
    InMemoryBackend,
    SimpleOutput,
    impl Fn(&ServiceRequest) -> Ready<Result<SimpleInput, actix_web::Error>>,
> {
    let input = SimpleInputFunctionBuilder::new(Duration::from_secs(60), 5)
        .real_ip_key()
        .custom_key("auth")
        .build();
    RateLimiter::builder(backend.clone(), input)
        .add_headers()
        .build()
}

async fn index() -> String {
    "Calpol API".to_string()
}
