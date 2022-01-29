use crate::profile::Profile;
use crate::response::ResponseExt;
use crate::{CalpolError, ClientError, GlobalOpts, Runnable, CLIENT};
use calpol_model::api_v1::{CreateTestRequest, UpdateTestRequest};
use clap::{Parser, Subcommand};
use serde::de::DeserializeOwned;
use std::fs;
use std::io::{stdin, Read};
use std::path::{Path, PathBuf};

#[derive(Parser, Debug)]
pub struct Tests {
    #[clap(subcommand)]
    op: Operations,
}

#[derive(Subcommand, Debug)]
pub enum Operations {
    /// List tests
    List(List),
    /// Create a new test
    Create(Create),
    /// Get a test by name
    Get(Get),
    /// Delete a test by name
    Delete(Delete),
    /// Update a test by name
    Update(Update),
    /// Upsert a test (update or insert if not exists)
    Upsert(Upsert),
}

impl Runnable for Tests {
    fn run(&self, opts: &GlobalOpts) -> Result<String, CalpolError> {
        let profile = Profile::load_profile(opts.profile.as_ref())?;
        match &self.op {
            Operations::List(l) => list(opts, &profile, l),
            Operations::Create(c) => create(opts, &profile, c),
            Operations::Get(g) => get(opts, &profile, g),
            Operations::Delete(d) => delete(opts, &profile, d),
            Operations::Update(u) => update(opts, &profile, u),
            Operations::Upsert(u) => upsert(opts, &profile, u),
        }
    }
}

#[derive(Parser, Debug)]
pub struct List {}

fn list(_: &GlobalOpts, profile: &Profile, _: &List) -> Result<String, CalpolError> {
    CLIENT
        .get(profile.route_url("api/v1/tests"))
        .bearer_auth(&profile.token)
        .send()?
        .verify_success()?
        .json_pretty()
}

#[derive(Parser, Debug)]
pub struct Create {
    /// Create user request (JSON file) (defaults to stdin)
    request: Option<PathBuf>,
}

fn create(_: &GlobalOpts, profile: &Profile, args: &Create) -> Result<String, CalpolError> {
    let item: CreateTestRequest = parse_json_from_arg_or_stdin(args.request.as_ref())?;
    CLIENT
        .post(profile.route_url("api/v1/tests"))
        .bearer_auth(&profile.token)
        .json(&item)
        .send()?
        .verify_success()?
        .json_pretty()
}

#[derive(Parser, Debug)]
pub struct Get {
    /// Name of test to get
    name: String,
}

fn get(_: &GlobalOpts, profile: &Profile, args: &Get) -> Result<String, CalpolError> {
    CLIENT
        .get(profile.route_url_with_id("api/v1/tests/", &args.name))
        .bearer_auth(&profile.token)
        .send()?
        .verify_success()?
        .json_pretty()
}

#[derive(Parser, Debug)]
pub struct Update {
    /// Name of test to update
    name: String,
    /// Update test request (JSON file) (defaults to stdin)
    request: Option<PathBuf>,
}

fn update(_: &GlobalOpts, profile: &Profile, args: &Update) -> Result<String, CalpolError> {
    let item: UpdateTestRequest = parse_json_from_arg_or_stdin(args.request.as_ref())?;
    CLIENT
        .put(profile.route_url_with_id("api/v1/tests/", &args.name))
        .bearer_auth(&profile.token)
        .json(&item)
        .send()?
        .verify_success()?
        .json_pretty()
}

#[derive(Parser, Debug)]
pub struct Delete {
    /// Name of test to delete
    name: String,
}

fn delete(_: &GlobalOpts, profile: &Profile, args: &Delete) -> Result<String, CalpolError> {
    CLIENT
        .delete(profile.route_url_with_id("api/v1/tests/", &args.name))
        .bearer_auth(&profile.token)
        .send()?
        .verify_success()?;
    Ok(format!("Successfully deleted test {}", args.name))
}

#[derive(Parser, Debug)]
pub struct Upsert {
    /// Update test request (JSON) (defaults to stdin)
    request: Option<PathBuf>,
}

fn upsert(_: &GlobalOpts, profile: &Profile, args: &Upsert) -> Result<String, CalpolError> {
    let item: CreateTestRequest = parse_json_from_arg_or_stdin(args.request.as_ref())?;
    let exists = CLIENT
        .get(profile.route_url_with_id("api/v1/tests/", &item.name))
        .bearer_auth(&profile.token)
        .send()?
        .status()
        .is_success();
    if exists {
        let update = UpdateTestRequest {
            config: Some(item.config),
            enabled: Some(item.enabled),
            failure_threshold: Some(item.failure_threshold),
        };
        CLIENT
            .put(profile.route_url_with_id("api/v1/tests/", &item.name))
            .bearer_auth(&profile.token)
            .json(&update)
    } else {
        CLIENT
            .post(profile.route_url("api/v1/tests"))
            .bearer_auth(&profile.token)
            .json(&item)
    }
    .send()?
    .verify_success()?
    .json_pretty()
}

fn parse_json_from_arg_or_stdin<S, T>(arg: Option<&S>) -> Result<T, ClientError>
where
    S: AsRef<Path>,
    T: DeserializeOwned,
{
    let string = match arg {
        None => {
            let mut buf = String::new();
            stdin()
                .read_to_string(&mut buf)
                .map_err(ClientError::FailedToReadStdin)?;
            buf
        }
        Some(s) => fs::read_to_string(s).map_err(ClientError::FailedToReadArgumentFile)?,
    };
    serde_json::from_str(&string).map_err(ClientError::FailedToParseJsonInput)
}
