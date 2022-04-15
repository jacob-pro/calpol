use actix_web::error::{BlockingError, PathError};
use actix_web::http::StatusCode;
use actix_web::ResponseError;
use bcrypt::BcryptError;
use http_api_problem::ApiError;
use lettre::transport::smtp;
use std::fmt::Debug;
use thiserror::Error;
use validator::ValidationErrors;

#[derive(Debug, Error)]
#[error("{0}")]
pub struct CalpolApiError(
    #[source]
    #[from]
    ApiError,
);

impl ResponseError for CalpolApiError {
    fn error_response(&self) -> actix_web::HttpResponse {
        self.0.error_response()
    }
}

pub fn internal_server_error<E: std::fmt::Display>(prefix: &str, error: E) -> CalpolApiError {
    let builder = ApiError::builder(StatusCode::INTERNAL_SERVER_ERROR);
    CalpolApiError(if cfg!(debug_assertions) {
        builder.message(format!("{}: {}", prefix, error)).finish()
    } else {
        builder.finish()
    })
}

impl From<diesel::result::Error> for CalpolApiError {
    fn from(e: diesel::result::Error) -> Self {
        internal_server_error("DieselError", e)
    }
}

impl From<BlockingError> for CalpolApiError {
    fn from(e: BlockingError) -> Self {
        internal_server_error("ActixBlockingError", e)
    }
}

impl From<BcryptError> for CalpolApiError {
    fn from(e: BcryptError) -> Self {
        internal_server_error("BcryptError", e)
    }
}

impl From<actix_web_validator::Error> for CalpolApiError {
    fn from(e: actix_web_validator::Error) -> Self {
        match e {
            actix_web_validator::Error::Validate(v) => v.into(),
            _ => ApiError::builder(StatusCode::BAD_REQUEST)
                .message(format!("{}", e))
                .finish()
                .into(),
        }
    }
}

impl From<smtp::Error> for CalpolApiError {
    fn from(e: smtp::Error) -> Self {
        internal_server_error("SmtpError", e)
    }
}

impl From<ValidationErrors> for CalpolApiError {
    fn from(e: ValidationErrors) -> Self {
        CalpolApiError(
            ApiError::builder(StatusCode::BAD_REQUEST)
                .message("One or more fields failed validation")
                .field("invalid-params", e.into_errors())
                .finish(),
        )
    }
}

impl From<PathError> for CalpolApiError {
    fn from(e: PathError) -> Self {
        CalpolApiError(
            ApiError::builder(StatusCode::NOT_FOUND)
                .message(format!("Unable to parse path parameter: {:#}", e))
                .finish(),
        )
    }
}

impl From<messagebird::Error> for CalpolApiError {
    fn from(e: messagebird::Error) -> Self {
        internal_server_error("MessageBirdError", e)
    }
}

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

pub fn get_mailbox_for_user(
    user: &crate::database::User,
) -> Result<lettre::message::Mailbox, CalpolApiError> {
    // This is an internal error because users with bad emails should never be created
    user.get_mailbox()
        .map_err(|e| internal_server_error("UserHasInvalidEmail", e))
}
