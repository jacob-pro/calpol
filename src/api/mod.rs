mod auth;
mod error;
mod v1;

use crate::api::error::CalpolApiError;
use actix_extensible_rate_limit::backend::memory::InMemoryBackend;
use actix_extensible_rate_limit::backend::{SimpleInput, SimpleInputFunctionBuilder, SimpleOutput};
use actix_extensible_rate_limit::RateLimiter;
use actix_web::dev::{ServiceRequest, ServiceResponse};
use actix_web::http::StatusCode;
use actix_web::middleware::{ErrorHandlerResponse, ErrorHandlers};
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
    api.service(api_resource("").route(web::get().to(|| async { "Calpol API".to_string() })));
}

pub fn error_handlers<B: 'static>() -> ErrorHandlers<B> {
    ErrorHandlers::new().handler(StatusCode::INTERNAL_SERVER_ERROR, handle_500)
}

pub trait JsonResponse {
    fn json_response(self) -> HttpResponse;
}

impl<T: Serialize> JsonResponse for T {
    fn json_response(self) -> HttpResponse {
        HttpResponse::Ok().json(self)
    }
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

/// Handle Internal Server Errors:
/// - Hide the error message (unless in a debug build).
/// - Log the error.
/// - Return in RFC7807 format.
fn handle_500<B>(res: ServiceResponse<B>) -> Result<ErrorHandlerResponse<B>, actix_web::Error> {
    let message = (|| {
        if let Some(err) = res.response().error() {
            log::error!("Internal Server Error: {}", err);
            if cfg!(debug_assertions) {
                return format!("{}", err);
            }
        } else {
            log::error!("Unknown Internal Server Error");
        }
        "Internal Server Error".to_string()
    })();
    Ok(ErrorHandlerResponse::Response(
        res.error_response(
            ApiError::builder(StatusCode::INTERNAL_SERVER_ERROR)
                .message(message)
                .finish(),
        )
        .map_into_right_body(),
    ))
}

/// Builds a rate limiter to protect authentication routes
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
