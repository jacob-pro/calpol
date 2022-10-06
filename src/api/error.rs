use actix_web::error::{BlockingError, PathError};
use actix_web::http::StatusCode;
use actix_web::{HttpResponse, ResponseError};
use bcrypt::BcryptError;
use http_api_problem::ApiError;
use lettre::transport::smtp;
use std::fmt::Debug;
use thiserror::Error;
use validator::ValidationErrors;

#[derive(Debug, Error)]
pub enum CalpolApiError {
    #[error("{0}")]
    ApiError(
        #[source]
        #[from]
        ApiError,
    ),
    #[error("{0}: {1}")]
    InternalServerError(&'static str, #[source] Box<dyn std::error::Error + Send>),
}

impl utoipa::ToResponse for CalpolApiError {
    fn response() -> (String, utoipa::openapi::Response) {
        let json = include_str!("../../resources/api_problem.yaml");
        (
            String::from("ApiError"),
            serde_yaml::from_str(json).unwrap(),
        )
    }
}

impl ResponseError for CalpolApiError {
    fn status_code(&self) -> StatusCode {
        match &self {
            CalpolApiError::ApiError(e) => e.status(),
            CalpolApiError::InternalServerError(_, _) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        match &self {
            CalpolApiError::ApiError(a) => a.error_response(),
            CalpolApiError::InternalServerError(_, _) => {
                // This will be overridden by 500 handler
                HttpResponse::InternalServerError().finish()
            }
        }
    }
}

// 4xx Errors

impl From<ValidationErrors> for CalpolApiError {
    fn from(e: ValidationErrors) -> Self {
        ApiError::builder(StatusCode::BAD_REQUEST)
            .message("One or more fields failed validation")
            .field("invalid-params", e.into_errors())
            .finish()
            .into()
    }
}

impl From<actix_web_validator::Error> for CalpolApiError {
    fn from(e: actix_web_validator::Error) -> Self {
        match e {
            actix_web_validator::Error::Validate(v) => v.into(),
            _ => ApiError::builder(StatusCode::BAD_REQUEST)
                .message(e.to_string())
                .finish()
                .into(),
        }
    }
}

impl From<PathError> for CalpolApiError {
    fn from(e: PathError) -> Self {
        ApiError::builder(StatusCode::NOT_FOUND)
            .message(format!("Unable to parse path parameter: {:#}", e))
            .finish()
            .into()
    }
}

// 5xx Errors

/// Various unexpected errors that could occur in the Calpol API.
/// These indicate some sort of programming error.
#[derive(Debug, Error)]
pub enum UnexpectedError {
    #[error("Missing auth data")]
    MissingAuthData,
    #[error("User has invalid email: {0}")]
    InvalidUserEmail(#[source] lettre::address::AddressError),
    #[error("Password reset token missing")]
    PasswordResetTokenMissing,
}

impl From<UnexpectedError> for CalpolApiError {
    fn from(e: UnexpectedError) -> Self {
        CalpolApiError::InternalServerError("Calpol", Box::new(e))
    }
}

impl From<diesel::result::Error> for CalpolApiError {
    fn from(e: diesel::result::Error) -> Self {
        CalpolApiError::InternalServerError("Diesel", Box::new(e))
    }
}

impl From<BlockingError> for CalpolApiError {
    fn from(e: BlockingError) -> Self {
        CalpolApiError::InternalServerError("BlockingError", Box::new(e))
    }
}

impl From<BcryptError> for CalpolApiError {
    fn from(e: BcryptError) -> Self {
        CalpolApiError::InternalServerError("BcryptError", Box::new(e))
    }
}

impl From<smtp::Error> for CalpolApiError {
    fn from(e: smtp::Error) -> Self {
        CalpolApiError::InternalServerError("smtp::Error", Box::new(e))
    }
}

impl From<crate::messagebird::Error> for CalpolApiError {
    fn from(e: crate::messagebird::Error) -> Self {
        CalpolApiError::InternalServerError("messagebird::Error", Box::new(e))
    }
}

// Extensions

pub trait MapDieselUniqueViolation<T, F> {
    fn map_unique_violation(self, f: F) -> Result<T, CalpolApiError>;
}

impl<T, F> MapDieselUniqueViolation<T, F> for diesel::QueryResult<T>
where
    F: Fn(&dyn diesel::result::DatabaseErrorInformation) -> CalpolApiError,
{
    fn map_unique_violation(self, f: F) -> Result<T, CalpolApiError> {
        self.map_err(|e| {
            if let diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UniqueViolation,
                m,
            ) = &e
            {
                return f(m.as_ref());
            }
            CalpolApiError::from(e)
        })
    }
}
