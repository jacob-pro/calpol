mod database;
mod notify;
mod runnable;

use crate::database::Test;
use crate::state::AppState;
use crate::test_runner::runnable::Runnable;
use anyhow::Context;
use calpol_model::tests::TestConfig;
use chrono::{DateTime, Utc};
use derive_new::new;
use futures::{stream, StreamExt};
use tokio::sync::mpsc;
use tokio::time::{sleep_until, timeout_at, Instant};

pub fn make_channel() -> (mpsc::Sender<()>, mpsc::Receiver<()>) {
    mpsc::channel(1)
}

pub async fn start(state: AppState, mut rx: mpsc::Receiver<()>) -> anyhow::Result<()> {
    loop {
        log::info!("Beginning test run");
        let start_instant = Instant::now();
        let start_time = Utc::now();
        let max_run_time = start_instant + state.settings.runner.timeout_duration();
        let result = run_tests(&state, max_run_time).await;
        if let Err(e) = database::insert_runner_log(state.database(), result, start_time).await {
            log::error!("Failed to write runner log: {}", e);
        };
        let next_run = start_instant + state.settings.runner.interval_duration();
        tokio::select! {
            _ = sleep_until(next_run) => {},
            message = rx.recv() => {message.unwrap()}
        }
    }
}

#[derive(new)]
pub struct RunResults {
    passed: usize,
    failed: usize,
    skipped: usize,
}

pub struct RunResult {
    started: DateTime<Utc>,
    finished: DateTime<Utc>,
    result: anyhow::Result<()>,
}

async fn run_tests(state: &AppState, timeout: Instant) -> anyhow::Result<RunResults> {
    let tests = database::retrieve_tests(state.database()).await?;
    let skipped = tests.iter().filter(|t| !t.enabled).count();
    let results = stream::iter(tests.into_iter().filter(|t| t.enabled).map(
        |test| -> (_, anyhow::Result<TestConfig>) {
            let config = test.config.clone();
            (
                test,
                serde_json::from_value(config).context("Failed to deserialize test config"),
            )
        },
    ))
    .map(|(test, config)| async move {
        let run_result = match config {
            Ok(c) => {
                let started = Utc::now();
                let result = timeout_at(timeout, c.run(&test.name))
                    .await
                    .context("Cancelled due to global test timeout")
                    .and_then(std::convert::identity);
                RunResult {
                    started,
                    finished: Utc::now(),
                    result,
                }
            }
            Err(e) => RunResult {
                started: Utc::now(),
                finished: Utc::now(),
                result: Err(e),
            },
        };
        (test, run_result)
    })
    .buffer_unordered(state.settings.runner.concurrency as usize)
    .collect::<Vec<(Test, RunResult)>>()
    .await;
    let passed = results.iter().filter(|(_, r)| r.result.is_ok()).count();
    let failed = results.iter().filter(|(_, r)| r.result.is_err()).count();

    let processed = database::insert_test_results(state.database(), results).await?;

    let notification_targets = database::fetch_notification_targets(state.database()).await?;

    notify::send_notifications(&processed.now_failing, notification_targets, state).await?;

    database::update_test_status(state.database(), processed).await?;

    database::delete_expired_records(state.database(), state.settings.clone()).await?;

    Ok(RunResults::new(passed, failed, skipped))
}
