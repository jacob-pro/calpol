use crate::api::auth::Auth;
use crate::api::error::{get_mailbox_for_user, CalpolApiError, MapDieselUniqueViolation};
use crate::api::v1::password_reset::send_reset_email;
use crate::api::{api_resource, auth, response_mapper};
use crate::database::{
    NewUser, SessionRepository, SessionRepositoryImpl, User, UserRepository, UserRepositoryImpl,
};
use crate::state::AppState;
use actix_web::http::StatusCode;
use actix_web::web::{Data, Path, ServiceConfig};
use actix_web::{web, HttpResponse, Responder};
use calpol_model::api_v1::{
    CreateUserRequest, ListUsersRequest, ListUsersResponse, UpdateUserRequest, UserSummary,
};
use chrono::Utc;
use diesel::Connection;
use diesel_repository::CrudRepository;
use futures::FutureExt;
use http_api_problem::ApiError;
use lettre::{Message, Transport};

pub fn configure(users: &mut ServiceConfig) {
    users.service(
        api_resource("")
            .route(web::get().to(list))
            .route(web::post().to(create)),
    );
    users.service(
        api_resource("{user_id}")
            .route(web::get().to(get))
            .route(web::put().to(update))
            .route(web::delete().to(delete)),
    );
    users.service(api_resource("{user_id}/test_email").route(web::post().to(test_email)));
    users.service(api_resource("{user_id}/test_sms").route(web::post().to(test_sms)));
}

async fn list(
    _auth: Auth,
    query: actix_web_validator::Query<ListUsersRequest>,
    state: Data<AppState>,
) -> impl Responder {
    web::block(move || -> Result<_, CalpolApiError> {
        let database = state.database();
        let user_repository = UserRepositoryImpl::new(&database);
        let result = query
            .search
            .as_ref()
            .map(|search| user_repository.find_by_search(query.limit, query.offset, search))
            .unwrap_or_else(|| {
                UserRepository::find_all(&user_repository, query.limit, query.offset)
            })?;
        Ok(ListUsersResponse {
            users: result.results.into_iter().map(|x| x.into()).collect(),
            total: result.count,
        })
    })
    .map(response_mapper)
    .await
}

pub async fn create(
    _auth: Auth,
    json: actix_web_validator::Json<CreateUserRequest>,
    state: Data<AppState>,
) -> impl Responder {
    web::block(move || -> Result<_, CalpolApiError> {
        let database = state.database();
        let mailer = state.mailer();
        let user_repository = UserRepositoryImpl::new(&database);
        let mut user = user_repository
            .insert(NewUser {
                name: json.name.clone(),
                email: json.email.to_string().to_ascii_lowercase(),
                password_hash: None,
                sms_notifications: false,
                email_notifications: false,
            })
            .map_unique_violation(|_| {
                ApiError::builder(StatusCode::CONFLICT)
                    .title("Email Taken")
                    .message("This email is already in use by another user")
                    .finish()
                    .into()
            })?;
        if let Err(e) = (|| -> Result<(), CalpolApiError> {
            user.password_reset_token = Some(auth::generate_token(&user));
            user.password_reset_token_creation = Some(Utc::now());
            user_repository.update(&user)?;
            send_reset_email(&mailer, &user, state.settings())?;
            Ok(())
        })() {
            log::error!("Unable to send reset email on account creation: {}", e)
        }
        Ok(UserSummary::from(user))
    })
    .map(response_mapper)
    .await
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

async fn get(_auth: Auth, user_id: Path<i32>, state: Data<AppState>) -> impl Responder {
    web::block(move || -> Result<_, CalpolApiError> {
        let database = state.database();
        let user_repository = UserRepositoryImpl::new(&database);
        let user = retrieve_user(&user_repository, user_id.0)?;
        Ok(UserSummary::from(user))
    })
    .map(response_mapper)
    .await
}

async fn update(
    _auth: Auth,
    user_id: Path<i32>,
    json: actix_web_validator::Json<UpdateUserRequest>,
    state: Data<AppState>,
) -> impl Responder {
    web::block(move || -> Result<_, CalpolApiError> {
        let database = state.database();
        let user_repository = UserRepositoryImpl::new(&database);
        let mut user = retrieve_user(&user_repository, user_id.0)?;
        if let Some(email) = &json.email {
            user.email = email.to_string().to_ascii_lowercase();
        }
        if let Some(name) = &json.name {
            user.name = name.clone();
        }
        if let Some(sms_notifications) = json.sms_notifications {
            user.sms_notifications = sms_notifications;
        }
        if let Some(email_notifications) = json.email_notifications {
            user.email_notifications = email_notifications;
        }
        if let Some(phone_number) = &json.phone_number {
            user.phone_number = Some(phone_number.clone());
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
    .map(response_mapper)
    .await
}

async fn delete(_auth: Auth, user_id: Path<i32>, state: Data<AppState>) -> impl Responder {
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
    .map(response_mapper)
    .await
}

async fn test_email(_auth: Auth, user_id: Path<i32>, state: Data<AppState>) -> impl Responder {
    web::block(move || -> Result<_, CalpolApiError> {
        let database = state.database();
        let user_repository = UserRepositoryImpl::new(&database);
        let user = retrieve_user(&user_repository, *user_id)?;
        let message = Message::builder()
            .to(get_mailbox_for_user(&user)?)
            .from(state.settings().mailer.send_from.clone())
            .reply_to(state.settings().mailer.reply_to().clone())
            .subject("Calpol Test Email")
            .body("Calpol Test Email".to_string())
            .unwrap();
        state.mailer().send(&message)?;
        Ok(())
    })
    .map(response_mapper)
    .await
}

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
    .await?;
    if let Some(phone) = user.phone_number {
        if let Some(messagebird) = state.message_bird() {
            let body = "Calpol Test SMS";
            let res = messagebird.send_message(body, vec![phone.clone()]).await?;
            log::info!("Sent test SMS to {}: {:?}", phone, res);
            Ok(HttpResponse::Ok().json(()))
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
