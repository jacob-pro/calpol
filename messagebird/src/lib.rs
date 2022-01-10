use reqwest::header;
use reqwest::header::{HeaderValue, InvalidHeaderValue};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use thiserror::Error;

const SMS_ENDPOINT: &'static str = "https://rest.messagebird.com/messages";
const TIMEOUT_SEC: u64 = 5;

#[derive(Clone)]
// TODO: Upgrade this to Tokio 1.0 once Actix supports it
pub struct MessageBirdClient {
    inner: Client,
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("Bad Access Key: {0}")]
    BadAccessKey(#[source] InvalidHeaderValue),
    #[error("Request Error: {0}")]
    Reqwest(
        #[from]
        #[source]
        reqwest::Error,
    ),
    #[error("API Error: {0}")]
    ApiError(
        #[from]
        #[source]
        ApiError,
    ),
    #[error("Received unexpected response: {0}")]
    UnexpectedResponse(String),
}

#[derive(Serialize)]
struct SendSms {
    originator: String,
    body: String,
    recipients: Vec<String>,
}

#[derive(Error, Deserialize, Debug)]
#[error("errors: {errors:?}")]
pub struct ApiError {
    errors: Vec<ApiErrorInner>,
}

#[derive(Error, Deserialize, Debug)]
#[error("{description} (Code: {code}, Parameter: {parameter:?}")]
pub struct ApiErrorInner {
    code: i64,
    description: String,
    parameter: Option<String>,
}

#[derive(Deserialize, Debug)]
#[allow(unused)]
pub struct SendSmsResponse {
    id: String,
    href: String,
}

impl MessageBirdClient {
    pub fn new(access_key: &str) -> Result<Self, Error> {
        let mut headers = header::HeaderMap::new();
        let value = format!("AccessKey {}", access_key);
        let value = HeaderValue::from_str(&value).map_err(Error::BadAccessKey)?;
        headers.insert(header::AUTHORIZATION, value);
        Ok(Self {
            inner: Client::builder()
                .default_headers(headers)
                .timeout(Duration::from_secs(TIMEOUT_SEC))
                .build()?,
        })
    }

    pub async fn send_message(
        &self,
        body: &str,
        recipients: Vec<String>,
    ) -> Result<SendSmsResponse, Error> {
        let json = SendSms {
            originator: "inbox".to_string(),
            body: body.to_string(),
            recipients,
        };
        let response = self.inner.post(SMS_ENDPOINT).json(&json).send().await?;
        let status = response.status();
        let body = response.text().await?;
        if !status.is_success() {
            Err(serde_json::from_str::<ApiError>(&body)
                .map_err(|_| Error::UnexpectedResponse(body))?
                .into())
        } else {
            Ok(serde_json::from_str::<SendSmsResponse>(&body)
                .map_err(|_| Error::UnexpectedResponse(body))?)
        }
    }
}
