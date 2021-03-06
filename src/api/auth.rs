use crate::api::error::{CalpolApiError, UnexpectedError};
use crate::database::{Session, SessionRepository, SessionRepositoryImpl, User};
use crate::state::AppState;
use actix_web::dev::{Payload, ServiceRequest};
use actix_web::http::header::{HeaderMap, USER_AGENT};
use actix_web::http::StatusCode;
use actix_web::web::Data;
use actix_web::{web, FromRequest, HttpMessage, HttpRequest};
use actix_web_httpauth::extractors::bearer::BearerAuth;
use chrono::Utc;
use diesel_repository::CrudRepository;
use futures::future::{err, ok, Ready};
use http_api_problem::ApiError;
use rand::distributions::Standard;
use rand::Rng;
use std::net::IpAddr;

pub struct Auth {
    pub session: Session,
    pub user: User,
}

pub fn get_user_agent(map: &HeaderMap) -> Result<String, ApiError> {
    map.get(USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .map(|u| {
            let mut u = u.to_owned();
            u.truncate(512);
            u
        })
        .ok_or_else(|| {
            ApiError::builder(StatusCode::BAD_REQUEST)
                .title("Bad Agent")
                .message("A valid user agent header is required")
                .finish()
        })
}

pub fn generate_token() -> String {
    const TOKEN_LENGTH_BYTES: usize = 32;
    let rng = rand::thread_rng();
    let v: Vec<u8> = rng
        .sample_iter(&Standard)
        .take(TOKEN_LENGTH_BYTES)
        .collect();
    base64::encode(&v)
}

pub async fn authenticator(
    req: ServiceRequest,
    auth: Option<BearerAuth>,
) -> Result<ServiceRequest, actix_web::Error> {
    let auth = auth.ok_or_else(|| {
        ApiError::builder(StatusCode::UNAUTHORIZED)
            .message("Missing bearer token")
            .finish()
    })?;
    let state = req
        .app_data::<Data<AppState>>()
        .expect("AppState missing")
        .clone();
    let ip_addr = req
        .connection_info()
        .realip_remote_addr()
        .unwrap()
        .parse::<IpAddr>()
        .unwrap();
    let user_agent = get_user_agent(req.headers())?;
    let auth = web::block(move || -> Result<_, CalpolApiError> {
        let ip_addr = ip_addr;
        let ip_bin = bincode::serialize(&ip_addr).unwrap();
        let database = state.database();
        let session_repository = SessionRepositoryImpl::new(&database);
        let (mut session, user) =
            session_repository
                .find_by_token(auth.token())?
                .ok_or_else(|| {
                    ApiError::builder(StatusCode::UNAUTHORIZED)
                        .message("Invalid session token")
                        .finish()
                })?;
        session.last_ip = ip_bin;
        session.user_agent = user_agent;
        session.last_used = Utc::now();
        session_repository.update(&session)?;
        Ok(Auth { session, user })
    })
    .await??;
    req.extensions_mut().insert(auth);
    Ok(req)
}

impl FromRequest for Auth {
    type Error = CalpolApiError;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        if let Some(user) = req.extensions_mut().remove::<Auth>() {
            ok(user)
        } else {
            err(UnexpectedError::MissingAuthData.into())
        }
    }
}
