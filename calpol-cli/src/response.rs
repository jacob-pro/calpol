use crate::CalpolError;
use derive_new::new;
use http_api_problem::HttpApiProblem;
use reqwest::blocking::Response;
use serde::Serialize;
use thiserror::Error;

pub trait ResponseExt
where
    Self: Sized,
{
    fn verify_success(self) -> Result<Self, CalpolError>;
    fn json_pretty(self) -> Result<String, CalpolError>;
}

impl ResponseExt for Response {
    fn verify_success(self) -> Result<Self, CalpolError> {
        let status = self.status();
        if !status.is_success() {
            let text = self.text().map_err(|_| {
                UnknownApiError::new(
                    status.as_u16(),
                    String::from("Unable to parse response body"),
                )
            })?;
            let problem = serde_json::from_str::<HttpApiProblem>(&text)
                .map_err(|_| UnknownApiError::new(status.as_u16(), text))?;
            return Err(CalpolError::ApiError(problem));
        };
        Ok(self)
    }

    fn json_pretty(self) -> Result<String, CalpolError> {
        let code = self.status().as_u16();
        let value = self.json::<serde_json::Value>().map_err(|_| {
            UnknownApiError::new(code, String::from("Unable to parse response as JSON"))
        })?;
        Ok(serde_json::to_string_pretty(&value).unwrap())
    }
}

#[derive(Debug, Error, Serialize, new)]
#[error("status code: {status_code}, body: {body}")]
pub struct UnknownApiError {
    status_code: u16,
    body: String,
}
