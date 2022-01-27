use crate::api::auth::{self, authenticator, Auth};
use crate::api::error::ApiErrorMap;
use crate::api::{api_resource, auth_rate_limiter, response_mapper};
use crate::database::{
    NewSession, SessionRepository, SessionRepositoryImpl, UserRepository, UserRepositoryImpl,
};
use crate::state::AppState;
use actix_ratelimit::MemoryStore;
use actix_utils::real_ip::RealIpExtension;
use actix_web::http::StatusCode;
use actix_web::web::{Data, Path, ServiceConfig};
use actix_web::{web, HttpRequest, Responder};
use actix_web_httpauth::middleware::HttpAuthentication;
use calpol_model::api_v1::{LoginRequest, LoginResponse, SessionSummary};
use chrono::Utc;
use diesel_repository::CrudRepository;
use futures::FutureExt;
use http_api_problem::ApiError;

pub fn configure(sessions: &mut ServiceConfig, rate_limit_store: &MemoryStore) {
    let auth = HttpAuthentication::with_fn(authenticator);
    sessions.service(
        api_resource("login")
            .route(web::post().to(login))
            .wrap(auth_rate_limiter(rate_limit_store)),
    );
    sessions.service(
        api_resource("logout")
            .route(web::delete().to(logout))
            .wrap(auth.clone()),
    );
    sessions.service(
        api_resource("")
            .route(web::get().to(list))
            .wrap(auth.clone()),
    );
    sessions.service(
        api_resource("{session_id}")
            .route(web::delete().to(delete))
            .wrap(auth),
    );
}

async fn login(
    json: actix_web_validator::Json<LoginRequest>,
    state: Data<AppState>,
    req: HttpRequest,
) -> impl Responder {
    let ip_addr = req.connection_info().real_ip_address().map_api_error()?;
    let user_agent = auth::get_user_agent(req.headers())?;
    web::block(move || {
        let database = state.database();
        let user_repository = UserRepositoryImpl::new(&database);
        let user = user_repository
            .find_by_email(&json.email)
            .map_api_error()?
            .ok_or_else(|| {
                ApiError::builder(StatusCode::UNAUTHORIZED)
                    .message("Incorrect email or password")
                    .finish()
            })?;
        let hashed = user.password_hash.as_ref().ok_or_else(|| {
            ApiError::builder(StatusCode::UNAUTHORIZED)
                .message("You need to reset your account password")
                .finish()
        })?;
        if !(bcrypt::verify(&json.password, hashed).map_api_error()?) {
            return Err(ApiError::builder(StatusCode::UNAUTHORIZED)
                .message("Incorrect email or password")
                .finish());
        };
        let session_repository = SessionRepositoryImpl::new(&database);
        let ip_addr = ip_addr;
        let ip_bin = bincode::serialize(&ip_addr).unwrap();
        let existing_session = session_repository
            .find_belonging_to_user_by_ip_and_agent(&user, &ip_bin, &user_agent)
            .map_api_error()?;
        let session = match existing_session {
            Some(mut existing_session) => {
                existing_session.last_used = Utc::now();
                session_repository
                    .update(&existing_session)
                    .map_api_error()?;
                existing_session
            }
            None => session_repository
                .insert(NewSession {
                    user_id: user.id,
                    token: auth::generate_token(&user),
                    last_ip: ip_bin,
                    user_agent,
                })
                .map_api_error()?,
        };
        Ok(LoginResponse {
            user: user.into(),
            token: session.token.clone(),
            session: session.into(),
        })
    })
    .map(response_mapper)
    .await
}

async fn logout(auth: Auth, state: Data<AppState>) -> impl Responder {
    web::block(move || {
        let database = state.database();
        let session_repository = SessionRepositoryImpl::new(&database);
        session_repository.delete(auth.session).map_api_error()?;
        Ok(())
    })
    .map(response_mapper)
    .await
}

async fn list(auth: Auth, state: Data<AppState>) -> impl Responder {
    web::block(move || {
        let database = state.database();
        let session_repository = SessionRepositoryImpl::new(&database);
        let sessions: Vec<SessionSummary> = session_repository
            .find_all_belonging_to_user(&auth.user)
            .map_api_error()?
            .into_iter()
            .map(|s| s.into())
            .collect();
        Ok(sessions)
    })
    .map(response_mapper)
    .await
}

async fn delete(auth: Auth, session_id: Path<i32>, state: Data<AppState>) -> impl Responder {
    web::block(move || {
        let database = state.database();
        let session_repository = SessionRepositoryImpl::new(&database);
        if !session_repository
            .delete_by_id_and_user(*session_id, &auth.user)
            .map_api_error()?
        {
            return Err(ApiError::new(StatusCode::NOT_FOUND));
        }
        Ok(())
    })
    .map(response_mapper)
    .await
}
