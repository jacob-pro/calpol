use crate::messagebird::MessageBirdClient;
use crate::settings::{MailerSetting, MessageBirdSetting, Settings};
use lettre::transport::smtp::authentication::Credentials;
use lettre::transport::smtp::PoolConfig;
use lettre::{AsyncSmtpTransport, Tokio1Executor};
use sea_orm::DatabaseConnection;
use std::sync::Arc;
use tokio::sync::mpsc;

#[derive(Clone)]
pub struct AppState {
    pub database: DatabaseConnection,
    pub mailer: AsyncSmtpTransport<Tokio1Executor>,
    pub message_bird: Option<MessageBirdClient>,
    pub settings: Arc<Settings>,
    test_runner: mpsc::Sender<()>,
}

impl AppState {
    pub fn new(
        settings: Arc<Settings>,
        database: DatabaseConnection,
        test_runner: mpsc::Sender<()>,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            database,
            mailer: lettre_client(&settings.mailer)?,
            message_bird: message_bird_client(settings.message_bird.as_ref())?,
            settings,
            test_runner,
        })
    }

    pub fn database(&self) -> crate::database::Connection {
        unimplemented!()
    }

    pub fn queue_test_run(&self) {
        self.test_runner.try_send(()).ok();
    }
}

fn message_bird_client(
    setting: Option<&MessageBirdSetting>,
) -> anyhow::Result<Option<MessageBirdClient>> {
    Ok(if let Some(setting) = setting {
        Some(MessageBirdClient::new(&setting.access_key)?)
    } else {
        None
    })
}

fn lettre_client(setting: &MailerSetting) -> anyhow::Result<AsyncSmtpTransport<Tokio1Executor>> {
    Ok(
        AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(setting.host.as_str())?
            .port(setting.port)
            .credentials(Credentials::new(
                setting.username.clone(),
                setting.password.clone(),
            ))
            .pool_config(PoolConfig::default().max_size(5))
            .build(),
    )
}
