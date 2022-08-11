use crate::test_runner::runnable::{verify_certificate_expiry, Domain};
use anyhow::bail;
use anyhow::Context;
use calpol_model::tests::Http;
use http::method::Method;
use reqwest::redirect;
use std::str::FromStr;
use std::time::Duration;
use tokio::time::timeout;
use tokio_native_tls::TlsConnector;
use url::Url;

const HTTP_TIMEOUT: Duration = Duration::from_secs(5);
const TLS_TIMEOUT: Duration = Duration::from_secs(5);

pub async fn test_http(http: &Http, net_domain: Domain) -> anyhow::Result<()> {
    let client = reqwest::ClientBuilder::default()
        .danger_accept_invalid_certs(!http.verify_ssl)
        .local_address(net_domain.local_address())
        .timeout(HTTP_TIMEOUT)
        .user_agent(format!("calpol-test-server {}", env!("CARGO_PKG_VERSION")))
        .redirect(if http.follow_redirects {
            redirect::Policy::default()
        } else {
            redirect::Policy::none()
        })
        .build()
        .context("Failed to build http client")?;
    let method = Method::from_str(http.method.as_str()).context("Invalid http method")?;
    let response = client.request(method, http.url.clone()).send().await?;
    if !http
        .expected_code
        .map(|expected| expected == response.status().as_u16())
        .unwrap_or_else(|| response.status().is_success())
    {
        bail!(
            "Received unexpected http response code: {}",
            response.status().as_u16()
        )
    }
    if http.follow_redirects {
        if let Some(expected) = &http.expected_redirect_destination {
            if expected != response.url() {
                bail!(
                    "Redirects did not match. Expected: {}, Found {}",
                    expected,
                    response.url()
                )
            }
        }
    }
    if http.url.scheme() == "https" && http.minimum_certificate_expiry_hours > 0 {
        do_certificate_test(
            http.verify_ssl,
            http.url.clone(),
            net_domain,
            http.minimum_certificate_expiry_hours,
        )
        .await?;
    }
    Ok(())
}

async fn do_certificate_test(
    verify: bool,
    url: Url,
    domain: Domain,
    minimum_certificate_expiry_hours: u16,
) -> anyhow::Result<()> {
    let socket = domain.tcp_socket()?;
    let connector = TlsConnector::from(
        tokio_native_tls::native_tls::TlsConnector::builder()
            .danger_accept_invalid_certs(!verify)
            .build()
            .context("Failed to build TlsConnector")?,
    );

    let addr = domain.socket_addr_for_url(&url)?;
    let host = url.host_str().context("URL missing host")?;

    let stream = timeout(HTTP_TIMEOUT, socket.connect(addr))
        .await
        .context("Socket timed out")?
        .context("Failed to establish TCP stream")?;

    let stream = timeout(TLS_TIMEOUT, connector.connect(host, stream))
        .await
        .context("TLS timed out")?
        .context("Failed to establish TLS stream")?;

    let der = stream
        .get_ref()
        .peer_certificate()
        .context("Failed to get peer certificate")?
        .unwrap()
        .to_der()
        .unwrap();
    verify_certificate_expiry(der, minimum_certificate_expiry_hours)?;

    Ok(())
}
