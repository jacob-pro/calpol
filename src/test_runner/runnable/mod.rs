mod http;
mod smtp;
mod tcp;

use crate::model::tests::{IpVersion, TestConfig, TestVariant};
use crate::test_runner::runnable::http::test_http;
use crate::test_runner::runnable::smtp::test_smtp;
use crate::test_runner::runnable::tcp::test_tcp;
use anyhow::{bail, Context};
use async_trait::async_trait;
use chrono::Duration;
use std::fmt::{Display, Formatter};
use std::net::{IpAddr, SocketAddr};
use tokio::net::TcpSocket;
use url::Url;
use x509_parser::certificate::X509Certificate;
use x509_parser::traits::FromDer;

#[async_trait]
pub trait Runnable {
    async fn run(&self, test_name: &str) -> anyhow::Result<()>;
}

#[async_trait]
impl Runnable for TestConfig {
    async fn run(&self, test_name: &str) -> anyhow::Result<()> {
        for net_domain in Domain::from_model(self.ip_version) {
            run_variant(&self.variant, net_domain, test_name)
                .await
                .context(format!("({})", net_domain))?
        }
        Ok(())
    }
}

async fn run_variant(
    variant: &TestVariant,
    net_domain: Domain,
    test_name: &str,
) -> anyhow::Result<()> {
    match &variant {
        TestVariant::Http(http) => test_http(http, net_domain).await?,
        TestVariant::Smtp(smtp) => test_smtp(smtp, net_domain, test_name).await?,
        TestVariant::Tcp(tcp) => test_tcp(tcp, net_domain).await?,
    }
    Ok(())
}

#[derive(Clone, Copy, Debug)]
pub enum Domain {
    IpV4,
    IpV6,
}

impl Domain {
    /// https://github.com/seanmonstar/reqwest/issues/584
    fn local_address(self) -> IpAddr {
        match self {
            Domain::IpV4 => "0.0.0.0".parse().unwrap(),
            Domain::IpV6 => "::".parse().unwrap(),
        }
    }

    fn from_model(version: IpVersion) -> Vec<Self> {
        match version {
            IpVersion::V4 => vec![Domain::IpV4],
            IpVersion::V6 => vec![Domain::IpV6],
            IpVersion::Both => vec![Domain::IpV4, Domain::IpV6],
        }
    }

    fn socket_addr_for_url(self, url: &Url) -> anyhow::Result<SocketAddr> {
        url.socket_addrs(|| None)
            .context("Failed to resolve socket address")?
            .into_iter()
            .find(|addr| match self {
                Domain::IpV4 => addr.is_ipv4(),
                Domain::IpV6 => addr.is_ipv6(),
            })
            .context("Failed to resolve socket address")
    }

    fn tcp_socket(self) -> anyhow::Result<TcpSocket> {
        Ok(match self {
            Domain::IpV4 => TcpSocket::new_v4()?,
            Domain::IpV6 => TcpSocket::new_v6()?,
        })
    }
}

impl Display for Domain {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Domain::IpV4 => f.write_str("IPV4"),
            Domain::IpV6 => f.write_str("IPV6"),
        }
    }
}

fn verify_certificate_expiry(der: Vec<u8>, minimum_expiry_hours: u16) -> anyhow::Result<()> {
    let cert = X509Certificate::from_der(der.as_slice())
        .context("Failed to parse certificate")?
        .1;
    let minimum_expiry = Duration::hours(minimum_expiry_hours as i64);
    let expiry = Duration::from_std(
        cert.validity()
            .time_to_expiration()
            .context("Certificate has expired")?,
    )
    .unwrap();
    if expiry < minimum_expiry {
        bail!("Certificate will expire in {} hours", expiry.num_hours())
    }
    Ok(())
}
