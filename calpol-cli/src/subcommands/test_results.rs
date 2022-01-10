use crate::profile::Profile;
use crate::response::ResponseExt;
use crate::{CalpolError, GlobalOpts, Runnable, CLIENT};
use calpol_model::api_v1::{GetTestResultsRequest, TestResultSummary};
use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
pub struct TestResults {
    #[clap(subcommand)]
    op: Operations,
}

#[derive(Subcommand, Debug)]
pub enum Operations {
    /// Lists latest result for all tests
    List(List),
    /// Get results for a particular test
    Get(Get),
}

impl Runnable for TestResults {
    fn run(&self, opts: &GlobalOpts) -> Result<String, CalpolError> {
        let profile = Profile::load_profile(opts.profile.as_ref())?;
        match &self.op {
            Operations::List(l) => list(opts, &profile, l),
            Operations::Get(g) => get(opts, &profile, g),
        }
    }
}

#[derive(Parser, Debug)]
pub struct List {
    /// Only show failing
    #[clap(long)]
    failing: bool,
}

fn list(_: &GlobalOpts, profile: &Profile, args: &List) -> Result<String, CalpolError> {
    let results: Vec<TestResultSummary> = CLIENT
        .get(profile.route_url("api/v1/test_results"))
        .bearer_auth(&profile.token)
        .send()?
        .verify_success()?
        .json()?;
    let results = results
        .into_iter()
        .filter(|r| {
            if args.failing {
                return !r.success;
            }
            true
        })
        .collect::<Vec<_>>();
    Ok(serde_json::to_string_pretty(&results).unwrap())
}

#[derive(Parser, Debug)]
pub struct Get {
    /// Name of test to get results for
    name: String,
    /// Limit
    #[clap(default_value_t = 3)]
    limit: u32,
}

fn get(_: &GlobalOpts, profile: &Profile, args: &Get) -> Result<String, CalpolError> {
    let item = GetTestResultsRequest { limit: args.limit };
    CLIENT
        .get(profile.route_url_with_id("api/v1/test_results/", &args.name))
        .bearer_auth(&profile.token)
        .json(&item)
        .send()?
        .verify_success()?
        .json_pretty()
}
