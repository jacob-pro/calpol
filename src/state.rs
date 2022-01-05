use crate::settings::Settings;
use derive_new::new;
use diesel::r2d2::ConnectionManager;
use diesel::{r2d2, PgConnection};
use lettre::SmtpTransport;
use std::sync::Arc;

#[derive(Clone, new)]
pub struct AppState {
    database: r2d2::Pool<ConnectionManager<PgConnection>>,
    mailer: SmtpTransport,
    settings: Arc<Settings>,
}

impl AppState {
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
