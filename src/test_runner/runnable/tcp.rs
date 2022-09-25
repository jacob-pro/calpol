use crate::model::tests::Tcp;
use crate::test_runner::runnable::Domain;
use anyhow::Context;
use std::time::Duration;
use tokio::time::timeout;
use url::Url;

const TCP_TIMEOUT: Duration = Duration::from_secs(5);

pub async fn test_tcp(tcp: &Tcp, domain: Domain) -> anyhow::Result<()> {
    let url = Url::parse(&format!("tcp://{}:{}", tcp.host, tcp.port)).context("Invalid host")?;
    let addr = domain.socket_addr_for_url(&url)?;

    let socket = domain.tcp_socket()?;
    timeout(TCP_TIMEOUT, socket.connect(addr))
        .await
        .context("Socket timed out")?
        .context("Failed to establish TCP stream")?;

    Ok(())
}
