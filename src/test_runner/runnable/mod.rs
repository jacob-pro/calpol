mod http;
mod smtp;

use crate::test_runner::runnable::http::test_http;
use crate::test_runner::runnable::smtp::test_smtp;
use anyhow::{anyhow, bail, Context};
use async_trait::async_trait;
use calpol_model::tests::{IpVersion, TestConfig, TestVariant};
use chrono::Duration;
use socket2::Domain;
use std::net::{IpAddr, SocketAddr};
use url::Url;
use x509_parser::certificate::X509Certificate;
use x509_parser::traits::FromDer;

#[async_trait]
pub trait Runnable {
    async fn run(&self) -> anyhow::Result<()>;
}

#[async_trait]
impl Runnable for TestConfig {
    async fn run(&self) -> anyhow::Result<()> {
        for net_domain in Domain::from_model(self.ip_version) {
            run_variant(&self.variant, net_domain)
                .await
                .context(format!("({})", net_domain.name()))?
        }
        Ok(())
    }
}

async fn run_variant(variant: &TestVariant, net_domain: Domain) -> anyhow::Result<()> {
    match &variant {
        TestVariant::Http(http) => test_http(http, net_domain).await?,
        TestVariant::Smtp(smtp) => test_smtp(smtp, net_domain).await?,
    }
    Ok(())
}

trait DomainExt
where
    Self: Sized,
{
    fn local_address(self) -> IpAddr;
    fn from_model(version: IpVersion) -> Vec<Self>;
    fn socket_addr_for_url(self, url: &Url) -> anyhow::Result<SocketAddr>;
    fn name(self) -> &'static str;
}

impl DomainExt for Domain {
    /// https://github.com/seanmonstar/reqwest/issues/584
    fn local_address(self) -> IpAddr {
        match self {
            Domain::IPV4 => "0.0.0.0".parse().unwrap(),
            Domain::IPV6 => "::".parse().unwrap(),
            _ => panic!("unknown domain"),
        }
    }

    fn from_model(version: IpVersion) -> Vec<Self> {
        match version {
            IpVersion::V4 => vec![Domain::IPV4],
            IpVersion::V6 => vec![Domain::IPV6],
            IpVersion::Both => vec![Domain::IPV4, Domain::IPV6],
        }
    }

    fn socket_addr_for_url(self, url: &Url) -> anyhow::Result<SocketAddr> {
        let addr = url
            .socket_addrs(|| None)
            .context("Failed to get address from url")?
            .into_iter()
            .filter(|addr| match self {
                Domain::IPV4 => addr.is_ipv4(),
                Domain::IPV6 => addr.is_ipv6(),
                _ => false,
            })
            .collect::<Vec<_>>();
        let first = addr
            .first()
            .ok_or(anyhow!("Url socket didn't resolve to matching IP Version"))?;
        Ok(first.clone())
    }

    fn name(self) -> &'static str {
        match self {
            Domain::IPV4 => "IPV4",
            Domain::IPV6 => "IPV6",
            _ => panic!("unknown domain"),
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
            .ok_or(anyhow!("Certificate has expired"))?,
    )
    .unwrap();
    if expiry < minimum_expiry {
        bail!("Certificate will expire in {} hours", expiry.num_hours())
    }
    Ok(())
}
