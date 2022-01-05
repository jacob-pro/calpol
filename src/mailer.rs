use crate::settings::MailerSetting;
use lettre::transport::smtp::authentication::Credentials;
use lettre::transport::smtp::PoolConfig;
use lettre::{AsyncSmtpTransport, SmtpTransport, Tokio1Executor};

pub fn lettre_sync(setting: &MailerSetting) -> anyhow::Result<SmtpTransport> {
    Ok(SmtpTransport::starttls_relay(setting.host.as_str())?
        .port(setting.port)
        .credentials(Credentials::new(
            setting.username.clone(),
            setting.password.clone(),
        ))
        .pool_config(PoolConfig::default().max_size(5))
        .build())
}

pub fn lettre_async(setting: &MailerSetting) -> anyhow::Result<AsyncSmtpTransport<Tokio1Executor>> {
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
