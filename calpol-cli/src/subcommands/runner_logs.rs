use crate::profile::Profile;
use crate::response::ResponseExt;
use crate::{CalpolError, GlobalOpts, Runnable, CLIENT};
use calpol_model::api_v1::ListRunnerLogsRequest;
use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
pub struct RunnerLogs {
    #[clap(subcommand)]
    op: Operations,
}

#[derive(Subcommand, Debug)]
pub enum Operations {
    /// Lists runner logs
    List(List),
}

impl Runnable for RunnerLogs {
    fn run(&self, opts: &GlobalOpts) -> Result<String, CalpolError> {
        let profile = Profile::load_profile(opts.profile.as_ref())?;
        match &self.op {
            Operations::List(l) => list(opts, &profile, l),
        }
    }
}

#[derive(Parser, Debug)]
pub struct List {
    /// Page number
    page: Option<u32>,
}

fn list(opts: &GlobalOpts, profile: &Profile, args: &List) -> Result<String, CalpolError> {
    CLIENT
        .get(profile.route_url("api/v1/runner_logs"))
        .bearer_auth(&profile.token)
        .json(&ListRunnerLogsRequest {
            limit: opts.page_size,
            offset: opts.get_offset(args.page)?,
        })
        .send()?
        .verify_success()?
        .json_pretty()
}
