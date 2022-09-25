use crate::api::error::{CalpolApiError, UnexpectedError};
use crate::api::{api_resource, api_scope, auth, auth_rate_limiter, JsonResponse};
use crate::database::{User, UserRepository, UserRepositoryImpl};
use crate::settings::Settings;
use crate::state::AppState;
use actix_extensible_rate_limit::backend::memory::InMemoryBackend;
use actix_web::http::StatusCode;
use actix_web::web::{Data, ServiceConfig};
use actix_web::{web, HttpResponse};
use crate::model::api_v1::{ResetPasswordRequest, SubmitPasswordResetRequest};
use chrono::{Duration, Utc};
use diesel_repository::CrudRepository;
use http_api_problem::ApiError;
use lettre::Message;

const TOKEN_EXPIRY_HOURS: i64 = 12;

pub fn configure(v1: &mut ServiceConfig, rl_backend: &InMemoryBackend) {
    v1.service(
        api_scope("password_reset")
            .service(api_resource("request").route(web::post().to(request)))
            .service(api_resource("submit").route(web::post().to(submit)))
            .wrap(auth_rate_limiter(rl_backend)),
    );
}

pub async fn send_reset_email<M, E>(
    mailer: &M,
    user: &User,
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

async fn request(
    state: Data<AppState>,
    json: actix_web_validator::Json<ResetPasswordRequest>,
) -> Result<HttpResponse, CalpolApiError> {
    let database = state.database();
    let user = web::block(move || -> Result<_, CalpolApiError> {
        let user_repository = UserRepositoryImpl::new(&database);
        let mut user = user_repository.find_by_email(&json.email)?.ok_or_else(|| {
            ApiError::builder(StatusCode::BAD_REQUEST)
                .message("Email not found")
                .finish()
        })?;
        user.password_reset_token = Some(auth::generate_token());
        user.password_reset_token_creation = Some(Utc::now());
        user_repository.update(&user)?;
        Ok(user)
    })
    .await??;
    send_reset_email(&state.mailer, &user, &state.settings).await?;
    Ok(().json_response())
}

async fn submit(
    state: Data<AppState>,
    json: actix_web_validator::Json<SubmitPasswordResetRequest>,
) -> Result<HttpResponse, CalpolApiError> {
    web::block(move || -> Result<_, CalpolApiError> {
        let database = state.database();
        let user_repository = UserRepositoryImpl::new(&database);
        let mut user = user_repository
            .find_by_reset_token(&json.token)?
            .ok_or_else(|| {
                ApiError::builder(StatusCode::BAD_REQUEST)
                    .message("Invalid reset token")
                    .finish()
            })?;
        let creation = user
            .password_reset_token_creation
            .ok_or(UnexpectedError::PasswordResetTokenMissing)?;
        if Utc::now() - creation > Duration::hours(TOKEN_EXPIRY_HOURS) {
            return Err(ApiError::builder(StatusCode::BAD_REQUEST)
                .title("Token expired")
                .message(
                    "The password reset token has expired, please try sending a new reset email",
                )
                .finish()
                .into());
        }
        user.password_hash = Some(bcrypt::hash(&json.new_password, bcrypt::DEFAULT_COST)?);
        user.password_reset_token = None;
        user.password_reset_token_creation = None;
        user_repository.update(&user)?;
        Ok(())
    })
    .await?
    .map(JsonResponse::json_response)
}
