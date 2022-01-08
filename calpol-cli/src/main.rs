use clap::{Parser, Subcommand};
use lazy_static::lazy_static;
use reqwest::blocking::Client;
use response::UnknownApiError;
use serde::Serialize;
use std::path::PathBuf;
use std::time::Duration;
use thiserror::Error;

mod profile;
mod response;
mod subcommands;
mod util;

lazy_static! {
    static ref CLIENT: Client = Client::builder()
        .user_agent(format!("calpol-cli {}", env!("CARGO_PKG_VERSION")))
        .connect_timeout(Duration::from_secs(1))
        .timeout(Duration::from_secs(8))
        .build()
        .unwrap();
}

#[derive(Debug, Error)]
#[error("{0}")]
pub enum CalpolError {
    ClientError(
        #[source]
        #[from]
        ClientError,
    ),
    ConnectionError(
        #[source]
        #[from]
        reqwest::Error,
    ),
    ApiError(
        #[source]
        #[from]
        http_api_problem::HttpApiProblem,
    ),
    UnknownApiError(
        #[source]
        #[from]
        UnknownApiError,
    ),
}

#[derive(Debug, Error)]
pub enum ClientError {
    #[error("Failed to read profile file: {0}")]
    FailedToReadProfileFile(#[source] std::io::Error),
    #[error("Failed to parse profile file: {0}")]
    FailedToParseProfile(#[source] serde_json::Error),
    #[error("Failed to create profile directory: {0}")]
    FailedToCreateAppDirectory(#[source] std::io::Error),
    #[error("Failed to write profile file: {0}")]
    FailedToWriteProfileFile(#[source] std::io::Error),
    #[error("Failed to delete profile file: {0}")]
    FailedToDeleteProfileFile(#[source] std::io::Error),
    #[error("Profile file already exists")]
    ProfileAlreadyExists,
    #[error("Page number must be 1 or greater")]
    InvalidPageNumber,
    #[error("Failed to read from stdin: {0}")]
    FailedToReadStdin(#[source] std::io::Error),
    #[error("Failed to read input (dialoguer): {0}")]
    DialoguerError(#[source] std::io::Error),
    #[error("Invalid path variable: {0}")]
    FailedToParsePathVariable(#[source] url::ParseError),
    #[error("Failed to parse json input: {0}")]
    FailedToParseJsonInput(#[source] serde_json::Error),
    #[error("Failed to read argument file: {0}")]
    FailedToReadArgumentFile(#[source] std::io::Error),
    #[error("Invalid profile id, expected integer or `self`")]
    InvalidProfileId,
}

#[derive(Debug, Serialize)]
#[serde(tag = "variant")]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CalpolErrorSerializable {
    ClientError { message: String },
    ConnectionError { message: String },
    ApiError(http_api_problem::HttpApiProblem),
    UnknownApiError(UnknownApiError),
}

impl From<CalpolError> for CalpolErrorSerializable {
    fn from(e: CalpolError) -> Self {
        match e {
            CalpolError::ClientError(e) => Self::ClientError {
                message: format!("{}", e),
            },
            CalpolError::ConnectionError(e) => Self::ConnectionError {
                message: format!("{}", e),
            },
            CalpolError::ApiError(a) => Self::ApiError(a),
            CalpolError::UnknownApiError(e) => Self::UnknownApiError(e),
        }
    }
}

#[derive(Parser, Debug)]
#[clap(about, version, author)]
struct Args {
    #[clap(subcommand)]
    subcommand: SubCommand,
    #[clap(flatten)]
    global: GlobalOpts,
}

#[derive(Parser, Debug)]
struct GlobalOpts {
    /// Override the profile file
    #[clap(long)]
    profile: Option<PathBuf>,
    /// Page size for paginated requests
    #[clap(long, default_value_t = 10)]
    page_size: u32,
}

impl GlobalOpts {
    fn get_offset(&self, page_number: Option<u32>) -> Result<u32, ClientError> {
        match page_number {
            None => Ok(0),
            Some(page_number) => {
                if page_number < 1 {
                    return Err(ClientError::InvalidPageNumber);
                }
                Ok((page_number - 1) * self.page_size)
            }
        }
    }
}

#[derive(Subcommand, Debug)]
enum SubCommand {
    /// Login, logout, and session management
    Session(subcommands::Session),
    /// User account management
    Users(subcommands::Users),
    /// Password reset functions
    PasswordReset(subcommands::PasswordReset),
    /// Test management
    Tests(subcommands::Tests),
    /// Test results
    TestResults(subcommands::TestResults),
    /// Runner logs
    RunnerLogs(subcommands::RunnerLogs),
}

fn main() {
    let args: Args = Args::parse();
    std::process::exit(match args.subcommand.run(&args.global) {
        Ok(s) => {
            println!("{}", s);
            0
        }
        Err(e) => {
            let e = CalpolErrorSerializable::from(e);
            println!("{}", serde_json::to_string_pretty(&e).unwrap());
            1
        }
    });
}

trait Runnable {
    fn run(&self, opts: &GlobalOpts) -> Result<String, CalpolError>;
}

impl Runnable for SubCommand {
    fn run(&self, opts: &GlobalOpts) -> Result<String, CalpolError> {
        match &self {
            SubCommand::Session(a) => a.run(opts),
            SubCommand::Users(a) => a.run(opts),
            SubCommand::PasswordReset(a) => a.run(opts),
            SubCommand::Tests(a) => a.run(opts),
            SubCommand::TestResults(a) => a.run(opts),
            SubCommand::RunnerLogs(a) => a.run(opts),
        }
    }
}
