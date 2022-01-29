use crate::profile::Profile;
use crate::response::ResponseExt;
use crate::util::{read_input, read_password};
use crate::{CalpolError, ClientError, GlobalOpts, Runnable, CLIENT};
use calpol_model::api_v1::{LoginRequest, LoginResponse};
use clap::{Parser, Subcommand};
use url::Url;

#[derive(Parser, Debug)]
pub struct Session {
    #[clap(subcommand)]
    op: Operations,
}

#[derive(Subcommand, Debug)]
pub enum Operations {
    /// Login and save profile
    Login(Login),
    /// Logout and delete profile
    Logout,
    /// List the current user's sessions
    List,
    /// Logs out a particular session id
    Delete(Delete),
    /// Shows the current profile
    Show,
}

impl Runnable for Session {
    fn run(&self, opts: &GlobalOpts) -> Result<String, CalpolError> {
        match &self.op {
            Operations::Login(l) => login(opts, l),
            Operations::Logout => logout(opts),
            Operations::List => list(opts),
            Operations::Delete(d) => delete(opts, d),
            Operations::Show => show(opts),
        }
    }
}

#[derive(Parser, Debug)]
pub struct Login {
    /// URL of the calpol server
    #[clap(long)]
    url: Option<Url>,
    /// Account email address
    #[clap(long)]
    email: Option<String>,
    /// Provide your password via stdin
    #[clap(long)]
    password_stdin: bool,
    /// Silently overwrite any existing profile file
    #[clap(long)]
    overwrite: bool,
}

fn login(opts: &GlobalOpts, args: &Login) -> Result<String, CalpolError> {
    if !args.overwrite && Profile::exists(opts.profile.as_ref()) {
        return Err(ClientError::ProfileAlreadyExists.into());
    }
    let url = args.url.clone().unwrap_or(read_input("Enter Server URL")?);
    let email = args
        .email
        .clone()
        .unwrap_or(read_input("Enter your email")?);
    let password = read_password(args.password_stdin)?;
    let response = CLIENT
        .post(url.join("api/v1/sessions/login").unwrap())
        .json(&LoginRequest { email, password })
        .send()?
        .verify_success()?
        .json::<LoginResponse>()?;
    let profile = Profile {
        token: response.token,
        user: response.user,
        url,
    };
    profile.save_profile(opts.profile.as_ref())?;
    Ok(format!("Successfully logged in as: {}", profile.user.name))
}

fn logout(opts: &GlobalOpts) -> Result<String, CalpolError> {
    if !Profile::exists(opts.profile.as_ref()) {
        return Ok("Note: No profile exists to logout".to_string());
    }
    let profile = Profile::load_profile(opts.profile.as_ref())?;
    CLIENT
        .delete(profile.route_url("api/v1/sessions/logout"))
        .bearer_auth(profile.token)
        .send()?
        .verify_success()?;
    Profile::delete(opts.profile.as_ref())?;
    Ok("Successfully logged out".to_string())
}

fn list(opts: &GlobalOpts) -> Result<String, CalpolError> {
    let profile = Profile::load_profile(opts.profile.as_ref())?;
    CLIENT
        .get(profile.route_url("api/v1/sessions"))
        .bearer_auth(profile.token)
        .send()?
        .verify_success()?
        .json_pretty()
}

#[derive(Parser, Debug)]
pub struct Delete {
    /// ID of session to delete
    id: i32,
}

fn delete(opts: &GlobalOpts, args: &Delete) -> Result<String, CalpolError> {
    let profile = Profile::load_profile(opts.profile.as_ref())?;
    CLIENT
        .delete(profile.route_url_with_id("api/v1/sessions/", &args.id))
        .bearer_auth(profile.token)
        .send()?
        .verify_success()?;
    Ok(format!("Successfully deleted session {}", args.id))
}

fn show(opts: &GlobalOpts) -> Result<String, CalpolError> {
    let profile = Profile::load_profile(opts.profile.as_ref())?;
    Ok(serde_json::to_string_pretty(&profile).unwrap())
}
