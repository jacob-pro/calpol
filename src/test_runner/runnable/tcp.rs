use std::time::Duration;
use anyhow::{Context};
use socket2::{Domain, Socket, Type};
use tokio::task::spawn_blocking;
use url::Url;
use calpol_model::tests::Tcp;
use crate::test_runner::runnable::{DomainExt, TIMEOUT_SEC};

pub async fn test_tcp(tcp: &Tcp, domain: Domain) -> anyhow::Result<()> {
    let url = Url::parse(&format!("tcp://{}:{}", tcp.host, tcp.port)).context("Invalid host")?;
    spawn_blocking(move || {
        let addr = domain.socket_addr_for_url(&url)?;
        let stream = Socket::new(domain, Type::STREAM, None).context("Failed to create socket")?;
        stream
            .connect_timeout(&addr.into(), Duration::from_secs(TIMEOUT_SEC))
            .context(format!("Failed to connect socket {}", addr))?;
        Ok(())
    }).await?
}
