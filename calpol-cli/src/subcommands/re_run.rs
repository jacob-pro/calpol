use crate::profile::Profile;
use crate::response::ResponseExt;
use crate::{CalpolError, GlobalOpts, Runnable, CLIENT};
use clap::Parser;

#[derive(Parser, Debug)]
pub struct ReRun {}

impl Runnable for ReRun {
    fn run(&self, opts: &GlobalOpts) -> Result<String, CalpolError> {
        let profile = Profile::load_profile(opts.profile.as_ref())?;
        CLIENT
            .post(profile.route_url("api/v1/re_run"))
            .bearer_auth(&profile.token)
            .send()?
            .verify_success()?;
        Ok(String::from("Queued re-run"))
    }
}
