use crate::api::auth::{authenticator, Auth};
use crate::api::error::{CalpolApiError, MapDieselUniqueViolation, UnexpectedError};
use crate::api::models::{
    CreateUserRequest, ListUsersRequest, ListUsersResponse, UpdateUserRequest, UserSummary,
};
use crate::api::routes::password_reset::send_reset_email;
use crate::api::{api_resource, api_scope, auth, JsonResponse};
use crate::database::{
    NewUser, SessionRepository, SessionRepositoryImpl, User, UserRepository, UserRepositoryImpl,
};
use crate::state::AppState;
use actix_web::http::StatusCode;
use actix_web::web::{Data, Path, ServiceConfig};
use actix_web::{web, HttpResponse};
use actix_web_httpauth::middleware::HttpAuthentication;
use chrono::Utc;
use diesel::Connection;
use diesel_repository::CrudRepository;
use http_api_problem::ApiError;
use lettre::{AsyncTransport, Message};

pub fn configure(api: &mut ServiceConfig) {
    let auth = HttpAuthentication::with_fn(authenticator);
    api.service(
        api_scope("users")
            .service(
                api_resource("")
                    .route(web::get().to(list))
                    .route(web::post().to(create)),
            )
            .service(
                api_resource("{user_id}")
                    .route(web::get().to(get))
                    .route(web::put().to(update))
                    .route(web::delete().to(delete)),
            )
            .service(api_resource("{user_id}/test_email").route(web::post().to(test_email)))
            .service(api_resource("{user_id}/test_sms").route(web::post().to(test_sms)))
            .wrap(auth),
    );
}

/// List users
#[utoipa::path(
    get,
    path = "/api/users",
    tag = "Users",
    operation_id = "ListUsers",
    request_body = ListUsersRequest,
    responses(
        (status = 200, description = "List of test results", body = ListUsersResponse),
        (status = "default", response = CalpolApiError)
    ),
)]
async fn list(
    _auth: Auth,
    json: actix_web_validator::Json<ListUsersRequest>,
    state: Data<AppState>,
) -> Result<HttpResponse, CalpolApiError> {
    web::block(move || -> Result<_, CalpolApiError> {
        let database = state.database();
        let user_repository = UserRepositoryImpl::new(&database);
        let result = json
            .search
            .as_ref()
            .map(|search| user_repository.find_by_search(json.limit, json.offset, search))
            .unwrap_or_else(|| {
                UserRepository::find_all(&user_repository, json.limit, json.offset)
            })?;
        Ok(ListUsersResponse {
            items: result.results.into_iter().map(|x| x.into()).collect(),
            total: result.count,
        })
    })
    .await?
    .map(JsonResponse::json_response)
}

/// Create a new user
#[utoipa::path(
    post,
    path = "/api/users",
    tag = "Users",
    operation_id = "CreateUser",
    request_body = CreateUserRequest,
    responses(
        (status = 200, description = "The created user", body = UserSummary),
        (status = "default", response = CalpolApiError)
    ),
)]
pub async fn create(
    _auth: Auth,
    json: actix_web_validator::Json<CreateUserRequest>,
    state: Data<AppState>,
) -> Result<HttpResponse, CalpolApiError> {
    let database = state.database();
    let user = web::block(move || -> Result<_, CalpolApiError> {
        let user_repository = UserRepositoryImpl::new(&database);
        let user = user_repository
            .insert(NewUser {
                name: json.name.clone(),
                email: json.email.to_string().to_ascii_lowercase(),
                password_hash: None,
                sms_notifications: false,
                email_notifications: false,
                password_reset_token: Some(auth::generate_token()),
                password_reset_token_creation: Some(Utc::now()),
            })
            .map_unique_violation(|_| {
                ApiError::builder(StatusCode::CONFLICT)
                    .title("Email Taken")
                    .message("This email is already in use by another user")
                    .finish()
                    .into()
            })?;
        Ok(user)
    })
    .await??;
    if let Err(e) = send_reset_email(&state.mailer, &user, &state.settings).await {
        log::error!("Unable to send reset email on account creation: {}", e)
    }
    Ok(UserSummary::from(user).json_response())
}

fn retrieve_user<'u, U>(user_repository: &U, user_id: i32) -> Result<User, CalpolApiError>
where
    U: UserRepository + 'u,
{
    user_repository.find_by_id(user_id)?.ok_or_else(|| {
        ApiError::builder(StatusCode::NOT_FOUND)
            .message("User id not found")
            .finish()
            .into()
    })
}

/// Get a user by id
#[utoipa::path(
    get,
    path = "/api/users/{id}",
    params(
        ("id" = i32, Path, description = "User ID to get")
    ),
    tag = "Users",
    operation_id = "GetUser",
    responses(
        (status = 200, description = "The user", body = UserSummary),
        (status = "default", response = CalpolApiError)
    ),
)]
async fn get(
    _auth: Auth,
    user_id: Path<i32>,
    state: Data<AppState>,
) -> Result<HttpResponse, CalpolApiError> {
    web::block(move || -> Result<_, CalpolApiError> {
        let database = state.database();
        let user_repository = UserRepositoryImpl::new(&database);
        let user = retrieve_user(&user_repository, *user_id)?;
        Ok(UserSummary::from(user))
    })
    .await?
    .map(JsonResponse::json_response)
}

/// Update a user
#[utoipa::path(
    put,
    path = "/api/users/{id}",
    params(
        ("id" = i32, Path, description = "User ID to update")
    ),
    tag = "Users",
    operation_id = "UpdateUser",
    request_body = UpdateUserRequest,
    responses(
        (status = 200, description = "The user", body = UserSummary),
        (status = "default", response = CalpolApiError)
    ),
)]
async fn update(
    _auth: Auth,
    user_id: Path<i32>,
    json: actix_web_validator::Json<UpdateUserRequest>,
    state: Data<AppState>,
) -> Result<HttpResponse, CalpolApiError> {
    web::block(move || -> Result<_, CalpolApiError> {
        let database = state.database();
        let json = json.into_inner();
        let user_repository = UserRepositoryImpl::new(&database);
        let mut user = retrieve_user(&user_repository, *user_id)?;
        if let Some(email) = json.email {
            user.email = email.to_string().to_ascii_lowercase();
        }
        if let Some(name) = json.name {
            user.name = name;
        }
        if let Some(sms_notifications) = json.sms_notifications {
            user.sms_notifications = sms_notifications;
        }
        if let Some(email_notifications) = json.email_notifications {
            user.email_notifications = email_notifications;
        }
        if let Some(phone_number) = json.phone_number {
            user.phone_number = phone_number;
        }
        user_repository.update(&user).map_unique_violation(|_| {
            ApiError::builder(StatusCode::CONFLICT)
                .title("Email Taken")
                .message("This email is already in use by another user")
                .finish()
                .into()
        })?;
        Ok(UserSummary::from(user))
    })
    .await?
    .map(JsonResponse::json_response)
}

/// Delete a user
#[utoipa::path(
    delete,
    path = "/api/users/{id}",
    params(
        ("id" = i32, Path, description = "User ID to delete")
    ),
    tag = "Users",
    operation_id = "DeleteUser",
    responses(
        (status = 200, description = "Success"),
        (status = "default", response = CalpolApiError)
    ),
)]
async fn delete(
    _auth: Auth,
    user_id: Path<i32>,
    state: Data<AppState>,
) -> Result<HttpResponse, CalpolApiError> {
    web::block(move || -> Result<_, CalpolApiError> {
        let database = state.database();
        let user_repository = UserRepositoryImpl::new(&database);
        let session_repository = SessionRepositoryImpl::new(&database);
        let user = retrieve_user(&user_repository, *user_id)?;
        database.transaction(|| -> Result<_, CalpolApiError> {
            session_repository.delete_all_belonging_to(&user)?;
            user_repository.delete(user)?;
            Ok(())
        })
    })
    .await?
    .map(JsonResponse::json_response)
}

/// Send a test email to a user's email address
#[utoipa::path(
    post,
    path = "/api/users/{id}/test_email",
    params(
        ("id" = i32, Path, description = "User ID to send email to")
    ),
    tag = "Users",
    operation_id = "SendTestEmail",
    responses(
        (status = 200, description = "Success"),
        (status = "default", response = CalpolApiError)
    ),
)]
async fn test_email(
    _auth: Auth,
    user_id: Path<i32>,
    state: Data<AppState>,
) -> Result<HttpResponse, CalpolApiError> {
    let database = state.database();
    let user = web::block(move || -> Result<_, CalpolApiError> {
        let user_repository = UserRepositoryImpl::new(&database);
        retrieve_user(&user_repository, *user_id)
    })
    .await??;
    let message = Message::builder()
        .to(user
            .get_mailbox()
            .map_err(UnexpectedError::InvalidUserEmail)?)
        .from(state.settings.mailer.send_from.clone())
        .reply_to(state.settings.mailer.reply_to().clone())
        .subject("Calpol Test Email")
        .body("Calpol Test Email".to_string())
        .unwrap();
    state.mailer.send(message).await?;
    Ok(().json_response())
}

/// Send a test SMS message to a user's phone number
#[utoipa::path(
    post,
    path = "/api/users/{id}/test_sms",
    params(
        ("id" = i32, Path, description = "User ID to send SMS message to")
    ),
    tag = "Users",
    operation_id = "SendTestSms",
    responses(
        (status = 200, description = "Success"),
        (status = "default", response = CalpolApiError)
    ),
)]
async fn test_sms(
    _auth: Auth,
    user_id: Path<i32>,
    state: Data<AppState>,
) -> Result<HttpResponse, CalpolApiError> {
    let database = state.database();
    let user = web::block(move || {
        let user_repository = UserRepositoryImpl::new(&database);
        retrieve_user(&user_repository, *user_id)
    })
    .await??;
    if let Some(phone) = user.phone_number {
        if let Some(message_bird) = &state.message_bird {
            let body = "Calpol Test SMS";
            let res = message_bird.send_message(body, vec![phone.clone()]).await?;
            log::info!("Sent test SMS to {}: {:?}", phone, res);
            Ok(().json_response())
        } else {
            Err(ApiError::builder(StatusCode::BAD_REQUEST)
                .message("SMS is not enabled on the server")
                .finish()
                .into())
        }
    } else {
        Err(ApiError::builder(StatusCode::BAD_REQUEST)
            .message("User doesn't have a phone number set")
            .finish()
            .into())
    }
}
