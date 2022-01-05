use crate::profile::Profile;
use crate::response::ResponseExt;
use crate::{CalpolError, GlobalOpts, Runnable, CLIENT};
use calpol_model::api_v1::{CreateUserRequest, ListUsersRequest, UpdateUserRequest};
use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
pub struct Users {
    #[clap(subcommand)]
    op: Operations,
}

#[derive(Subcommand, Debug)]
pub enum Operations {
    /// List user accounts
    List(List),
    /// Create a new user account
    Create(Create),
    /// Get a user by id
    Get(Get),
    /// Delete a user by id
    Delete(Delete),
    /// Update a user by id
    Update(Update),
}

impl Runnable for Users {
    fn run(&self, opts: &GlobalOpts) -> Result<String, CalpolError> {
        let profile = Profile::load_profile(opts.profile.as_ref())?;
        match &self.op {
            Operations::List(l) => list(opts, &profile, l),
            Operations::Create(c) => create(opts, &profile, c),
            Operations::Get(g) => get(opts, &profile, g),
            Operations::Delete(d) => delete(opts, &profile, d),
            Operations::Update(u) => update(opts, &profile, u),
        }
    }
}

#[derive(Parser, Debug)]
pub struct List {
    /// Page number
    page: Option<u32>,
    /// Search query
    search: Option<String>,
}

fn list(opts: &GlobalOpts, profile: &Profile, args: &List) -> Result<String, CalpolError> {
    CLIENT
        .get(profile.route_url("api/v1/users"))
        .bearer_auth(&profile.token)
        .json(&ListUsersRequest {
            limit: opts.page_size,
            offset: opts.get_offset(args.page)?,
            search: None,
        })
        .send()?
        .verify_success()?
        .json_pretty()
}

#[derive(Parser, Debug)]
pub struct Create {
    name: String,
    email: String,
}

fn create(_: &GlobalOpts, profile: &Profile, args: &Create) -> Result<String, CalpolError> {
    let item = CreateUserRequest {
        name: args.name.clone(),
        email: args.email.clone(),
    };
    CLIENT
        .post(profile.route_url("api/v1/users"))
        .bearer_auth(&profile.token)
        .json(&item)
        .send()?
        .verify_success()?
        .json_pretty()
}

#[derive(Parser, Debug)]
pub struct Get {
    /// ID of user to get
    id: i32,
}

fn get(_: &GlobalOpts, profile: &Profile, args: &Get) -> Result<String, CalpolError> {
    CLIENT
        .get(profile.route_url_with_id("api/v1/users/", &args.id))
        .bearer_auth(&profile.token)
        .send()?
        .json_pretty()
}

#[derive(Parser, Debug)]
pub struct Update {
    /// ID of user to update
    id: i32,
    #[clap(long)]
    name: Option<String>,
    #[clap(long)]
    email: Option<String>,
    #[clap(long)]
    phone_number: Option<String>,
    #[clap(long)]
    sms_notifications: Option<bool>,
    #[clap(long)]
    email_notifications: Option<bool>,
}

fn update(_: &GlobalOpts, profile: &Profile, args: &Update) -> Result<String, CalpolError> {
    let item = UpdateUserRequest {
        name: args.name.clone(),
        email: args.email.clone(),
        phone_number: args.phone_number.clone(),
        sms_notifications: args.sms_notifications.clone(),
        email_notifications: args.email_notifications.clone(),
    };
    CLIENT
        .put(profile.route_url_with_id("api/v1/users/", &args.id))
        .bearer_auth(&profile.token)
        .json(&item)
        .send()?
        .verify_success()?
        .json_pretty()
}

#[derive(Parser, Debug)]
pub struct Delete {
    /// ID of user to delete
    id: i32,
}

fn delete(_: &GlobalOpts, profile: &Profile, args: &Delete) -> Result<String, CalpolError> {
    CLIENT
        .delete(profile.route_url_with_id("api/v1/users/", &args.id))
        .bearer_auth(&profile.token)
        .send()?;
    Ok(format!("Successfully deleted user {}", args.id))
}
