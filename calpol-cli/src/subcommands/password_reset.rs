use crate::response::ResponseExt;
use crate::util::{read_input, read_password};
use crate::{CalpolError, GlobalOpts, Runnable, CLIENT};
use calpol_model::api_v1::{ResetPasswordRequest, SubmitPasswordResetRequest};
use clap::{Parser, Subcommand};
use url::Url;

#[derive(Parser, Debug)]
pub struct PasswordReset {
    #[clap(subcommand)]
    op: Operations,
}

#[derive(Subcommand, Debug)]
pub enum Operations {
    /// Request a password reset for an account
    Request(Request),
    /// Set a new password using a password reset token
    Submit(Submit),
}

impl Runnable for PasswordReset {
    fn run(&self, opts: &GlobalOpts) -> Result<String, CalpolError> {
        match &self.op {
            Operations::Request(a) => request(opts, a),
            Operations::Submit(a) => submit(opts, a),
        }
    }
}

#[derive(Parser, Debug)]
pub struct Request {
    /// URL of the calpol server
    #[clap(long)]
    url: Option<Url>,
    /// Account email address
    #[clap(long)]
    email: Option<String>,
}

fn request(_: &GlobalOpts, args: &Request) -> Result<String, CalpolError> {
    let url = args.url.clone().unwrap_or(read_input("Enter Server URL")?);
    let email = args
        .email
        .clone()
        .unwrap_or(read_input("Enter your email")?);
    CLIENT
        .post(url.join("api/v1/password_reset/request").unwrap())
        .json(&ResetPasswordRequest { email })
        .send()?
        .verify_success()?;
    Ok(String::from("Password reset has been sent"))
}

#[derive(Parser, Debug)]
pub struct Submit {
    /// URL of the calpol server
    #[clap(long)]
    url: Option<Url>,
    /// Password reset token
    #[clap(long)]
    token: Option<String>,
    /// Receive new password via stdin
    #[clap(long)]
    password_stdin: bool,
}

fn submit(_: &GlobalOpts, args: &Submit) -> Result<String, CalpolError> {
    let url = args.url.clone().unwrap_or(read_input("Enter Server URL")?);
    let token = args
        .token
        .clone()
        .unwrap_or(read_input("Enter the password reset token")?);
    let password = read_password(args.password_stdin)?;
    CLIENT
        .post(url.join("api/v1/password_reset/submit").unwrap())
        .json(&SubmitPasswordResetRequest {
            token,
            new_password: password,
        })
        .send()?
        .verify_success()?;
    Ok(String::from("Successfully reset password"))
}
