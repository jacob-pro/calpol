use crate::mailer::lettre_sync;
use crate::settings::Settings;
use diesel::r2d2::ConnectionManager;
use diesel::{r2d2, PgConnection};
use lettre::SmtpTransport;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    database: r2d2::Pool<ConnectionManager<PgConnection>>,
    mailer: SmtpTransport,
    settings: Arc<Settings>,
}

impl AppState {
    pub fn new(
        settings: Arc<Settings>,
        database: r2d2::Pool<ConnectionManager<PgConnection>>,
    ) -> anyhow::Result<AppState> {
        Ok(AppState {
            database,
            mailer: lettre_sync(&settings.mailer)?,
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
}
