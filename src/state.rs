use crate::settings::{MailerSetting, MessageBirdSetting, Settings};
use diesel::r2d2::ConnectionManager;
use diesel::{r2d2, PgConnection};
use lettre::transport::smtp::authentication::Credentials;
use lettre::transport::smtp::PoolConfig;
use lettre::{AsyncSmtpTransport, Tokio1Executor};
use messagebird::MessageBirdClient;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    database: r2d2::Pool<ConnectionManager<PgConnection>>,
    pub mailer: AsyncSmtpTransport<Tokio1Executor>,
    pub message_bird: Option<MessageBirdClient>,
    pub settings: Arc<Settings>,
}

impl AppState {
    pub fn new(
        settings: Arc<Settings>,
        database: r2d2::Pool<ConnectionManager<PgConnection>>,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            database,
            mailer: lettre_client(&settings.mailer)?,
            message_bird: message_bird_client(settings.message_bird.as_ref())?,
            settings,
        })
    }

    pub fn database(&self) -> crate::database::Connection {
        self.database.get().unwrap()
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
