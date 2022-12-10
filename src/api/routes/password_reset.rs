use crate::api::error::{CalpolApiError, UnexpectedError};
use crate::api::models::{PasswordResetRequest, SubmitPasswordResetRequest};
use crate::api::{api_resource, api_scope, auth, auth_rate_limiter};
use crate::database2::{CrudRepository, UserRepository};
use crate::settings::Settings;
use crate::state::AppState;
use actix_extensible_rate_limit::backend::memory::InMemoryBackend;
use actix_web::http::StatusCode;
use actix_web::web::{Data, ServiceConfig};
use actix_web::{web, HttpResponse};
use chrono::{DateTime, Duration, Utc};
use http_api_problem::ApiError;
use lettre::Message;
use sea_orm::{IntoActiveModel, Set};

const TOKEN_EXPIRY_HOURS: i64 = 12;

pub fn configure(api: &mut ServiceConfig, rl_backend: &InMemoryBackend) {
    api.service(
        api_scope("password_reset")
            .service(api_resource("").route(web::post().to(request)))
            .service(api_resource("submit").route(web::post().to(submit)))
            .wrap(auth_rate_limiter(rl_backend)),
    );
}

pub async fn send_reset_email<M, E>(
    mailer: &M,
    user: &entity::user::Model,
    settings: &Settings,
) -> Result<(), CalpolApiError>
where
    M: lettre::AsyncTransport<Error = E> + Sync,
    E: Into<CalpolApiError>,
{
    let body = format!(
        "Hello {}\n\nYour reset token is: \n{}",
        user.name,
        user.password_reset_token.as_ref().unwrap_or(&String::new())
    );
    let message = Message::builder()
        .to(user
            .get_mailbox()
            .map_err(UnexpectedError::InvalidUserEmail)?)
        .from(settings.mailer.send_from.clone())
        .reply_to(settings.mailer.reply_to().clone())
        .subject("Calpol Password Reset")
        .body(body)
        .unwrap();
    mailer.send(message).await.map_err(|e| e.into())?;
    Ok(())
}

/// Request a password reset token
#[utoipa::path(
    post,
    path = "/api/password_reset/",
    tag = "PasswordReset",
    operation_id = "Request",
    request_body = PasswordResetRequest,
    responses(
        (status = 200, description = "Success"),
        (status = "default", response = CalpolApiError)
    ),
    security(),
)]
async fn request(
    state: Data<AppState>,
    json: actix_web_validator::Json<PasswordResetRequest>,
) -> Result<HttpResponse, CalpolApiError> {
    let user_repository = UserRepository::new(&state.database);
    let user = user_repository
        .find_by_email(&json.email)
        .await?
        .ok_or_else(|| {
            ApiError::builder(StatusCode::BAD_REQUEST)
                .message("Email not found")
                .finish()
        })?;
    let mut model = user.into_active_model();
    model.password_reset_token = Set(Some(auth::generate_token()));
    model.password_reset_token_creation = Set(Some(Utc::now().into()));
    let user = user_repository.update(model).await?;
    send_reset_email(&state.mailer, &user, &state.settings).await?;
    Ok(HttpResponse::Ok().json(()))
}

/// Submit a password reset token
#[utoipa::path(
    post,
    path = "/api/password_reset/submit",
    tag = "PasswordReset",
    operation_id = "Submit",
    request_body = SubmitPasswordResetRequest,
    responses(
        (status = 200, description = "Success"),
        (status = "default", response = CalpolApiError)
    ),
    security(),
)]
async fn submit(
    state: Data<AppState>,
    json: actix_web_validator::Json<SubmitPasswordResetRequest>,
) -> Result<HttpResponse, CalpolApiError> {
    let user_repository = UserRepository::new(&state.database);
    let user = user_repository
        .find_by_reset_token(&json.token)
        .await?
        .ok_or_else(|| {
            ApiError::builder(StatusCode::BAD_REQUEST)
                .message("Invalid reset token")
                .finish()
        })?;
    let creation = user
        .password_reset_token_creation
        .ok_or(UnexpectedError::PasswordResetTokenMissing)?;
    if DateTime::from(Utc::now()) - creation > Duration::hours(TOKEN_EXPIRY_HOURS) {
        return Err(ApiError::builder(StatusCode::BAD_REQUEST)
            .title("Token expired")
            .message("The password reset token has expired, please try sending a new reset email")
            .finish()
            .into());
    }
    let mut model = user.into_active_model();
    let new_password_hash = bcrypt::hash(&json.new_password, bcrypt::DEFAULT_COST)?;
    model.password_hash = Set(Some(new_password_hash));
    model.password_reset_token = Set(None);
    model.password_reset_token_creation = Set(None);
    user_repository.update(model).await?;
    Ok(HttpResponse::Ok().json(()))
}
