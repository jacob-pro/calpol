use crate::mailer::lettre_sync;
use crate::settings::{MessageBirdSetting, Settings};
use diesel::r2d2::ConnectionManager;
use diesel::{r2d2, PgConnection};
use lettre::SmtpTransport;
use messagebird::MessageBirdClient;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    database: r2d2::Pool<ConnectionManager<PgConnection>>,
    mailer: SmtpTransport,
    message_bird: Option<MessageBirdClient>,
    settings: Arc<Settings>,
}

impl AppState {
    pub fn new(
        settings: Arc<Settings>,
        database: r2d2::Pool<ConnectionManager<PgConnection>>,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            database,
            mailer: lettre_sync(&settings.mailer)?,
            message_bird: message_bird_client(settings.message_bird.as_ref())?,
            settings,
        })
    }

    pub fn database(&self) -> crate::database::Connection {
        self.database.get().unwrap()
    }

    pub fn mailer(&self) -> SmtpTransport {
        self.mailer.clone()
    }

    pub fn settings(&self) -> &Arc<Settings> {
        &self.settings
    }

    pub fn message_bird(&self) -> Option<&MessageBirdClient> {
        self.message_bird.as_ref()
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
