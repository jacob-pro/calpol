use anyhow::Context;
use config::{Config, Environment, File, FileFormat};
use lettre::message::Mailbox;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::time::Duration;
use validator::{Validate, ValidationError};

#[derive(Debug, Deserialize, Validate)]
pub struct Settings {
    /// Socket the API should listen on.
    #[serde(default = "default_api_socket")]
    pub api_socket: SocketAddr,
    /// Postgres connection URL
    pub database_url: String,
    #[validate]
    pub mailer: MailerSetting,
    #[validate]
    pub runner: RunnerSetting,
    #[validate]
    pub twilio: Option<TwilioSetting>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct MailerSetting {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    /// Mailbox Calpol emails will be sent from
    pub send_from: Mailbox,
    /// Reply-To field on emails, defaults to the `send_from` Mailbox
    pub reply_to: Option<Mailbox>,
}

impl MailerSetting {
    pub fn reply_to(&self) -> &Mailbox {
        self.reply_to.as_ref().unwrap_or(&self.send_from)
    }
}

#[derive(Debug, Serialize, Deserialize, Validate)]
#[validate(schema(function = "validate_runner_setting", skip_on_field_errors = false))]
pub struct RunnerSetting {
    /// How frequently to run the test suite in minutes
    #[serde(default = "default_runner_interval")]
    pub interval: u8,
    /// Maximum time the test suite is allowed to run in minutes
    #[serde(default = "default_runner_timeout")]
    #[validate(range(min = 1))]
    pub timeout: u8,
    /// How many tests may be run at once
    #[serde(default = "default_runner_concurrency")]
    #[validate(range(min = 1))]
    pub concurrency: u8,
    /// Max log age in days
    #[serde(default = "default_runner_log_age")]
    pub log_age: u16,
}

impl RunnerSetting {
    pub fn timeout_duration(&self) -> Duration {
        chrono::Duration::minutes(self.timeout as i64)
            .to_std()
            .unwrap()
    }
    pub fn interval_duration(&self) -> Duration {
        chrono::Duration::minutes(self.interval as i64)
            .to_std()
            .unwrap()
    }
}

#[derive(Debug, Deserialize, Validate)]
pub struct TwilioSetting {
    pub account_id: String,
    pub auth_token: String,
    pub send_from: String,
}

impl Settings {
    pub fn new(file: Option<&String>) -> anyhow::Result<Self> {
        let mut s = Config::new();
        match file {
            None => {}
            Some(f) => {
                s.merge(File::with_name(f).format(FileFormat::Toml))
                    .context("Failed to loading config from file")?;
            }
        }
        s.merge(Environment::new())
            .context("Failed loading config from environment")?;
        let r: Settings = s.try_into().context("Failed to load config")?;
        r.validate().context("Failed to validate config")?;
        Ok(r)
    }
}

fn default_api_socket() -> SocketAddr {
    "0.0.0.0:80".parse().unwrap()
}

fn default_runner_interval() -> u8 {
    15
}

fn default_runner_timeout() -> u8 {
    10
}

fn default_runner_concurrency() -> u8 {
    4
}

fn default_runner_log_age() -> u16 {
    30
}

fn validate_runner_setting(runner_setting: &RunnerSetting) -> Result<(), ValidationError> {
    if runner_setting.timeout >= runner_setting.interval {
        return Err(ValidationError::new(
            "Timeout must be less than the run interval",
        ));
    }
    Ok(())
}

impl TwilioSetting {
    pub fn new_client(&self) -> twilio::Client {
        twilio::Client::new(&self.account_id, &self.auth_token)
    }
}
