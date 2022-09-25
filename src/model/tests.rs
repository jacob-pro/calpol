use serde::{Deserialize, Serialize};
use url::Url;
use validator::{Validate, ValidationErrors};

#[derive(Clone, Serialize, Deserialize, Debug, Validate)]
pub struct TestConfig {
    #[serde(default)]
    pub ip_version: IpVersion,
    #[serde(flatten)]
    #[validate]
    pub variant: TestVariant,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum TestVariant {
    Http(Http),
    Smtp(Smtp),
    Tcp(Tcp),
}

impl Validate for TestVariant {
    fn validate(&self) -> Result<(), ValidationErrors> {
        match self {
            TestVariant::Http(t) => t.validate(),
            TestVariant::Smtp(t) => t.validate(),
            TestVariant::Tcp(t) => t.validate(),
        }
    }
}

#[derive(Copy, Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub enum IpVersion {
    V4,
    V6,
    Both,
}

impl Default for IpVersion {
    fn default() -> Self {
        Self::Both
    }
}

#[derive(Clone, Serialize, Deserialize, Debug, Validate)]
pub struct Http {
    pub url: Url,
    /// Fail if the certificate could not be verified. (Only applies to HTTPS URLs)
    #[serde(default = "default_http_verify_ssl")]
    pub verify_ssl: bool,
    /// Fail if the expiry date of the certificate is less than X hours in the future. (Only applies to HTTPS URLs)
    #[serde(default = "default_minimum_cert_expiry")]
    pub minimum_certificate_expiry_hours: u16,
    /// Whether to follow HTTP redirects.
    #[serde(default = "default_http_follow_redirects")]
    pub follow_redirects: bool,
    /// The URL we expect the server to redirect us to. (Only applies if redirects enabled)
    pub expected_redirect_destination: Option<Url>,
    /// HTTP request method.
    #[serde(default = "default_http_request_method")]
    pub method: String,
    /// Expected HTTP response code (defaults to any success)
    pub expected_code: Option<u16>,
}

fn default_http_verify_ssl() -> bool {
    true
}

fn default_minimum_cert_expiry() -> u16 {
    36
}

fn default_http_follow_redirects() -> bool {
    true
}

fn default_http_request_method() -> String {
    String::from("GET")
}

#[derive(Clone, Serialize, Deserialize, Debug, Validate)]
#[validate(schema(function = "validate_smtp", skip_on_field_errors = false))]
pub struct Smtp {
    #[validate(length(max = 253))]
    pub domain: String,
    #[serde(default)]
    pub encryption: SmtpEncryption,
    /// Fail if the expiry date of the certificate is less than X hours in the future. (Only applies if encryption enabled)
    #[serde(default = "default_minimum_cert_expiry")]
    pub minimum_certificate_expiry_hours: u16,
    #[serde(flatten)]
    pub r#type: SmtpServerType,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case", tag = "smtp_server_type")]
pub enum SmtpServerType {
    /// Port defaults to 465 for SMTPS, or 587 otherwise
    MailSubmissionAgent { port: Option<u16> },
    /// Resolves the domain's MX record, connects over port 25
    MailTransferAgent,
}

fn validate_smtp(smtp: &Smtp) -> Result<(), validator::ValidationError> {
    if let SmtpServerType::MailTransferAgent = smtp.r#type {
        if let SmtpEncryption::SMTPS = smtp.encryption {
            return Err(validator::ValidationError::new(
                "Incompatible options: SMTPS and MTA",
            ));
        }
    }
    Ok(())
}

#[derive(Copy, Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
#[allow(clippy::upper_case_acronyms)]
pub enum SmtpEncryption {
    None,
    STARTTLS,
    SMTPS,
}

impl Default for SmtpEncryption {
    fn default() -> Self {
        Self::STARTTLS
    }
}

#[derive(Clone, Serialize, Deserialize, Debug, Validate)]
pub struct Tcp {
    pub host: String,
    pub port: u16,
}
