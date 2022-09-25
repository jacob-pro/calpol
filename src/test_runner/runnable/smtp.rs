use crate::model::tests::{Smtp, SmtpEncryption, SmtpServerType};
use crate::test_runner::runnable::{verify_certificate_expiry, Domain};
use anyhow::{bail, Context};
use lettre::transport::smtp::client::{AsyncSmtpConnection, TlsParameters};
use lettre::transport::smtp::extension::ClientId;
use std::time::Duration;
use trust_dns_resolver::config::{ResolverConfig, ResolverOpts};
use trust_dns_resolver::AsyncResolver;

const SMTP_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const DNS_TIMEOUT: Duration = Duration::from_secs(5);

pub async fn test_smtp(smtp: &Smtp, domain: Domain, test_name: &str) -> anyhow::Result<()> {
    let host = get_host(smtp).await?;
    let port = get_port(smtp);
    log::info!("{}: Connecting to {}:{}", test_name, host, port);
    let client_id = ClientId::default();
    let tls_parameters = if let SmtpEncryption::SMTPS = smtp.encryption {
        Some(TlsParameters::new(host.clone()).context("Failed to build tls parameters")?)
    } else {
        None
    };
    let mut connection = AsyncSmtpConnection::connect_tokio1(
        (host.clone(), port),
        Some(SMTP_CONNECT_TIMEOUT),
        &client_id,
        tls_parameters,
        Some(domain.local_address()),
    )
    .await
    .context("Failed to connect to the smtp server")?;
    log::info!(
        "{}: Server banner {}",
        test_name,
        connection.server_info().name()
    );
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
        let opts = ResolverOpts {
            timeout: DNS_TIMEOUT,
            ..Default::default()
        };
        let resolver = AsyncResolver::tokio(ResolverConfig::google(), opts)
            .context("Failed to get resolver")?;
        let mx_results = resolver
            .mx_lookup(smtp.domain.as_str())
            .await
            .context("Failed to lookup mx record")?;
        let mut exchange = mx_results
            .iter()
            .next()
            .context("No mx records found")?
            .exchange()
            .to_utf8();
        // If domain ends with a dot it breaks the certificate hostname verification
        if exchange.ends_with('.') {
            exchange.pop();
        }
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
