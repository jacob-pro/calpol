use actix_web::http::StatusCode;
use http_api_problem::ApiError;
use thiserror::Error;

pub trait ApiErrorMap<T> {
    fn map_api_error(self) -> Result<T, ApiError>;
}

impl<T, E: IntoApiError> ApiErrorMap<T> for Result<T, E> {
    fn map_api_error(self) -> Result<T, ApiError> {
        self.map_err(|e| e.into_api_error())
    }
}

pub trait IntoApiError {
    fn into_api_error(self) -> ApiError;
}

pub fn internal_server_error<E: std::fmt::Display>(prefix: &str, error: E) -> ApiError {
    let builder = ApiError::builder(StatusCode::INTERNAL_SERVER_ERROR);
    if cfg!(debug_assertions) {
        builder.message(format!("{}: {}", prefix, error)).finish()
    } else {
        builder.finish()
    }
}

impl IntoApiError for actix_utils::real_ip::RealIpAddressError {
    fn into_api_error(self) -> ApiError {
        internal_server_error("RealIpAddressError", self)
    }
}

impl IntoApiError for diesel::result::Error {
    fn into_api_error(self) -> ApiError {
        internal_server_error("DieselError", self)
    }
}

impl IntoApiError for actix_web::error::BlockingError<ApiError> {
    fn into_api_error(self) -> ApiError {
        match self {
            actix_web::error::BlockingError::Error(e) => e,
            actix_web::error::BlockingError::Canceled => {
                internal_server_error("ActixBlockingError", self)
            }
        }
    }
}

impl IntoApiError for bcrypt::BcryptError {
    fn into_api_error(self) -> ApiError {
        internal_server_error("BcryptError", self)
    }
}

impl IntoApiError for actix_web_validator::Error {
    fn into_api_error(self) -> ApiError {
        match self {
            actix_web_validator::Error::Validate(v) => v.into_api_error(),
            _ => ApiError::builder(StatusCode::BAD_REQUEST)
                .message(format!("{}", self))
                .finish(),
        }
    }
}

impl IntoApiError for lettre::transport::smtp::Error {
    fn into_api_error(self) -> ApiError {
        internal_server_error("SmtpError", self)
    }
}

impl IntoApiError for validator::ValidationErrors {
    fn into_api_error(self) -> ApiError {
        ApiError::builder(StatusCode::BAD_REQUEST)
            .message("One or more fields failed validation")
            .field("invalid-params", self.into_errors())
            .finish()
    }
}

impl IntoApiError for actix_web::error::PathError {
    fn into_api_error(self) -> ApiError {
        ApiError::builder(StatusCode::NOT_FOUND)
            .message(format!("Unable to parse path parameter: {:#}", self))
            .finish()
    }
}

// https://github.com/diesel-rs/diesel/issues/2342
#[derive(Debug, Error)]
pub enum DieselTransactionError {
    #[error("Diesel error: {0}")]
    DieselError(
        #[from]
        #[source]
        diesel::result::Error,
    ),
    #[error("Api error: {0}")]
    ApiError(
        #[from]
        #[source]
        ApiError,
    ),
}

impl From<DieselTransactionError> for ApiError {
    fn from(err: DieselTransactionError) -> Self {
        match err {
            DieselTransactionError::DieselError(e) => e.into_api_error(),
            DieselTransactionError::ApiError(e) => e,
        }
    }
}

pub trait MapDieselUniqueViolation<T, F> {
    fn map_unique_violation(self, f: F) -> Result<T, ApiError>;
}

impl<T, F> MapDieselUniqueViolation<T, F> for diesel::QueryResult<T>
where
    F: Fn(&dyn diesel::result::DatabaseErrorInformation) -> ApiError,
{
    fn map_unique_violation(self, f: F) -> Result<T, ApiError> {
        self.map_err(|e| {
            if let diesel::result::Error::DatabaseError(k, m) = &e {
                if let diesel::result::DatabaseErrorKind::UniqueViolation = k {
                    return f(m.as_ref());
                }
            }
            e.into_api_error()
        })
    }
}
