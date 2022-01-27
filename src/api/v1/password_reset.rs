use crate::api::error::{internal_server_error, ApiErrorMap, IntoApiError};
use crate::api::{api_resource, auth, response_mapper};
use crate::database::{User, UserRepository, UserRepositoryImpl};
use crate::settings::Settings;
use crate::state::AppState;
use actix_web::http::StatusCode;
use actix_web::web::{Data, ServiceConfig};
use actix_web::{web, Responder};
use calpol_model::api_v1::{ResetPasswordRequest, SubmitPasswordResetRequest};
use chrono::{Duration, Utc};
use diesel_repository::CrudRepository;
use futures::FutureExt;
use http_api_problem::ApiError;
use lettre::Message;

const TOKEN_EXPIRY_HOURS: i64 = 12;

pub fn configure(password_reset: &mut ServiceConfig) {
    password_reset.service(api_resource("request").route(web::post().to(request)));
    password_reset.service(api_resource("submit").route(web::post().to(submit)));
}

pub fn send_reset_email<M, E>(mailer: &M, user: &User, settings: &Settings) -> Result<(), ApiError>
where
    M: lettre::Transport<Error = E>,
    E: IntoApiError,
{
    let body = format!(
        "Hello {}\n\nYour reset token is: \n{}",
        user.name,
        user.password_reset_token.as_ref().unwrap_or(&String::new())
    );
    let message = Message::builder()
        .to(user
            .get_mailbox()
            .map_err(|e| internal_server_error("UserHasInvalidEmail", e))?)
        .from(settings.mailer.send_from.clone())
        .reply_to(settings.mailer.reply_to().clone())
        .subject("Calpol Password Reset")
        .body(body)
        .unwrap();
    mailer.send(&message).map_api_error()?;
    Ok(())
}

async fn request(
    state: Data<AppState>,
    json: actix_web_validator::Json<ResetPasswordRequest>,
) -> impl Responder {
    web::block(move || {
        let database = state.database();
        let mailer = state.mailer();
        let user_repository = UserRepositoryImpl::new(&database);
        let mut user = user_repository
            .find_by_email(&json.email)
            .map_api_error()?
            .ok_or_else(|| {
                ApiError::builder(StatusCode::BAD_REQUEST)
                    .message("Email not found")
                    .finish()
            })?;
        user.password_reset_token = Some(auth::generate_token(&user));
        user.password_reset_token_creation = Some(Utc::now());
        user_repository.update(&user).map_api_error()?;
        send_reset_email(&mailer, &user, state.settings())?;
        Ok(())
    })
    .map(response_mapper)
    .await
}

async fn submit(
    state: Data<AppState>,
    json: actix_web_validator::Json<SubmitPasswordResetRequest>,
) -> impl Responder {
    web::block(move || {
        let database = state.database();
        let user_repository = UserRepositoryImpl::new(&database);
        let mut user = user_repository
            .find_by_reset_token(&json.token)
            .map_api_error()?
            .ok_or_else(|| ApiError::builder(StatusCode::BAD_REQUEST).message("Invalid reset token"))?;
        user.password_reset_token_creation.map(|timestamp| {
            if Utc::now() - timestamp > Duration::hours(TOKEN_EXPIRY_HOURS) {
                Err(ApiError::builder(StatusCode::BAD_REQUEST)
                    .title("Token expired")
                    .message("The password reset token has expired, please try sending a new reset email")
                    .finish()
                )
            } else {
                Ok(())
            }
        }).unwrap_or_else(|| Err(internal_server_error("PasswordResetSubmit", "token creation time missing")))?;
        user.password_hash = Some(bcrypt::hash(&json.new_password, bcrypt::DEFAULT_COST).map_api_error()?);
        user.password_reset_token = None;
        user.password_reset_token_creation = None;
        user_repository.update(&user).map_api_error()?;
        Ok(())
    })
    .map(response_mapper)
    .await
}
