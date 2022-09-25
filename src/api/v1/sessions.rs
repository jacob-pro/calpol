use crate::api::auth::{self, authenticator, Auth};
use crate::api::error::CalpolApiError;
use crate::api::{api_resource, api_scope, auth_rate_limiter, JsonResponse};
use crate::database::{
    NewSession, SessionRepository, SessionRepositoryImpl, UserRepository, UserRepositoryImpl,
};
use crate::state::AppState;
use actix_extensible_rate_limit::backend::memory::InMemoryBackend;
use actix_web::http::StatusCode;
use actix_web::web::{Data, Path, ServiceConfig};
use actix_web::{web, HttpRequest, HttpResponse};
use actix_web_httpauth::middleware::HttpAuthentication;
use crate::model::api_v1::{LoginRequest, LoginResponse, SessionSummary};
use chrono::Utc;
use diesel_repository::CrudRepository;
use http_api_problem::ApiError;
use std::net::IpAddr;

pub fn configure(v1: &mut ServiceConfig, rate_limit_backend: &InMemoryBackend) {
    let auth = HttpAuthentication::with_fn(authenticator);
    v1.service(
        api_scope("sessions")
            .service(
                api_resource("login")
                    .route(web::post().to(login))
                    .wrap(auth_rate_limiter(rate_limit_backend)),
            )
            .service(
                api_resource("logout")
                    .route(web::delete().to(logout))
                    .wrap(auth.clone()),
            )
            .service(
                api_resource("")
                    .route(web::get().to(list))
                    .wrap(auth.clone()),
            )
            .service(
                api_resource("{session_id}")
                    .route(web::delete().to(delete))
                    .wrap(auth),
            ),
    );
}

async fn login(
    json: actix_web_validator::Json<LoginRequest>,
    state: Data<AppState>,
    req: HttpRequest,
) -> Result<HttpResponse, CalpolApiError> {
    let ip_addr = req
        .connection_info()
        .realip_remote_addr()
        .unwrap()
        .parse::<IpAddr>()
        .unwrap();
    let user_agent = auth::get_user_agent(req.headers())?;
    web::block(move || -> Result<_, CalpolApiError> {
        let database = state.database();
        let user_repository = UserRepositoryImpl::new(&database);
        let user = user_repository.find_by_email(&json.email)?.ok_or_else(|| {
            ApiError::builder(StatusCode::UNAUTHORIZED)
                .message("Incorrect email or password")
                .finish()
        })?;
        let hashed = user.password_hash.as_ref().ok_or_else(|| {
            ApiError::builder(StatusCode::UNAUTHORIZED)
                .message("You need to reset your account password")
                .finish()
        })?;
        if !(bcrypt::verify(&json.password, hashed)?) {
            return Err(ApiError::builder(StatusCode::UNAUTHORIZED)
                .message("Incorrect email or password")
                .finish()
                .into());
        };
        let session_repository = SessionRepositoryImpl::new(&database);
        let ip_addr = ip_addr;
        let ip_bin = bincode::serialize(&ip_addr).unwrap();
        let existing_session = session_repository.find_belonging_to_user_by_ip_and_agent(
            &user,
            &ip_bin,
            &user_agent,
        )?;
        let session = match existing_session {
            Some(mut existing_session) => {
                existing_session.last_used = Utc::now();
                session_repository.update(&existing_session)?;
                existing_session
            }
            None => session_repository.insert(NewSession {
                user_id: user.id,
                token: auth::generate_token(),
                last_ip: ip_bin,
                user_agent,
            })?,
        };
        Ok(LoginResponse {
            user: user.into(),
            token: session.token.clone(),
            session: session.into(),
        })
    })
    .await?
    .map(JsonResponse::json_response)
}

async fn logout(auth: Auth, state: Data<AppState>) -> Result<HttpResponse, CalpolApiError> {
    web::block(move || -> Result<_, CalpolApiError> {
        let database = state.database();
        let session_repository = SessionRepositoryImpl::new(&database);
        session_repository.delete(auth.session)?;
        Ok(())
    })
    .await?
    .map(JsonResponse::json_response)
}

async fn list(auth: Auth, state: Data<AppState>) -> Result<HttpResponse, CalpolApiError> {
    web::block(move || -> Result<_, CalpolApiError> {
        let database = state.database();
        let session_repository = SessionRepositoryImpl::new(&database);
        let sessions: Vec<SessionSummary> = session_repository
            .find_all_belonging_to_user(&auth.user)?
            .into_iter()
            .map(|s| s.into())
            .collect();
        Ok(sessions)
    })
    .await?
    .map(JsonResponse::json_response)
}

async fn delete(
    auth: Auth,
    session_id: Path<i32>,
    state: Data<AppState>,
) -> Result<HttpResponse, CalpolApiError> {
    web::block(move || -> Result<_, CalpolApiError> {
        let database = state.database();
        let session_repository = SessionRepositoryImpl::new(&database);
        if !session_repository.delete_by_id_and_user(*session_id, &auth.user)? {
            return Err(ApiError::new(StatusCode::NOT_FOUND).into());
        }
        Ok(())
    })
    .await?
    .map(JsonResponse::json_response)
}
