use crate::test_runner::runnable::verify_certificate_expiry;
use anyhow::{anyhow, bail, Context};
use calpol_model::tests::{Smtp, SmtpEncryption, SmtpServerType};
use lettre::transport::smtp::client::{AsyncSmtpConnection, TlsParameters};
use lettre::transport::smtp::extension::ClientId;
use socket2::Domain;
use std::time::Duration;
use trust_dns_resolver::config::{ResolverConfig, ResolverOpts};
use trust_dns_resolver::AsyncResolver;

pub async fn test_smtp(smtp: &Smtp, _domain: Domain) -> anyhow::Result<()> {
    // TODO: Use correct net domain: https://github.com/lettre/lettre/issues/715
    let host = get_host(smtp).await?;
    let port = get_port(smtp);
    let timeout = Duration::from_secs(5);
    let client_id = ClientId::default();
    let tls_parameters = if let SmtpEncryption::SMTPS = smtp.encryption {
        Some(TlsParameters::new(host.clone()).context("Failed to build tls parameters")?)
    } else {
        None
    };
    let mut connection = AsyncSmtpConnection::connect_tokio1(
        (host.clone(), port),
        Some(timeout),
        &client_id,
        tls_parameters,
    )
    .await
    .context("Failed to connect to the smtp server")?;
    if let SmtpEncryption::STARTTLS = smtp.encryption {
        connection
            .starttls(
                TlsParameters::new(host.clone()).context("Failed to build tls parameters")?,
                &client_id,
            )
            .await
            .context("Failed to starttls")?;
    }

    if !connection.test_connected().await {
        bail!("Testing smtp connection failed")
    }

    if smtp.encryption != SmtpEncryption::None && smtp.minimum_certificate_expiry_hours > 0 {
        let der = connection
            .peer_certificate()
            .context("Failed to get certificate")?;
        verify_certificate_expiry(der, smtp.minimum_certificate_expiry_hours)?;
    }

    Ok(())
}

async fn get_host(smtp: &Smtp) -> anyhow::Result<String> {
    Ok(if let SmtpServerType::MailTransferAgent = smtp.r#type {
        let mut opts = ResolverOpts::default();
        opts.timeout = Duration::from_secs(5);
        let resolver = AsyncResolver::tokio(ResolverConfig::google(), opts)
            .context("Failed to get resolver")?;
        let mx_results = resolver
            .mx_lookup(smtp.domain.as_str())
            .await
            .context("Failed to lookup mx record")?;
        let exchange = mx_results
            .iter()
            .next()
            .ok_or(anyhow!("No mx records found"))?
            .exchange()
            .to_utf8();
        exchange
    } else {
        smtp.domain.clone()
    })
}

fn get_port(smtp: &Smtp) -> u16 {
    match &smtp.r#type {
        SmtpServerType::MailSubmissionAgent { port } => {
            if let Some(port) = port {
                return *port;
            }
            match smtp.encryption {
                SmtpEncryption::None => 587,
                SmtpEncryption::STARTTLS => 587,
                SmtpEncryption::SMTPS => 465,
            }
        }
        SmtpServerType::MailTransferAgent => 25,
    }
}
