mod database;
mod notify;
mod runnable;

use crate::database::Test;
use crate::model::tests::TestConfig;
use crate::state::AppState;
use crate::test_runner::runnable::Runnable;
use anyhow::Context;
use chrono::{DateTime, Utc};
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
            // Allows waking up early to immediately re-run tests
            message = rx.recv() => {message.unwrap()}
        }
    }
}

pub struct RunResults {
    passed: usize,
    failed: usize,
    skipped: usize,
}

pub struct TestRunResult {
    started: DateTime<Utc>,
    finished: DateTime<Utc>,
    result: anyhow::Result<()>,
}

async fn run_tests(state: &AppState, timeout: Instant) -> anyhow::Result<RunResults> {
    // Retrieve disabled and enabled tests from the database
    let (disabled, enabled): (Vec<_>, Vec<_>) = database::retrieve_tests(state.database())
        .await?
        .into_iter()
        .partition(|test| !test.enabled);

    // Deserialize the test config JSON
    let deserialized = enabled
        .into_iter()
        .map(|test| -> (_, anyhow::Result<TestConfig>) {
            let config = test.config.clone();
            (
                test,
                serde_json::from_value(config).context("Failed to deserialize test config"),
            )
        });

    // Run the tests
    let results = stream::iter(deserialized)
        .map(|(test, config)| async move {
            let run_result = match config {
                Ok(c) => {
                    let started = Utc::now();
                    let result = timeout_at(timeout, c.run(&test.name))
                        .await
                        .context("Cancelled due to global test timeout")
                        .and_then(std::convert::identity);
                    TestRunResult {
                        started,
                        finished: Utc::now(),
                        result,
                    }
                }
                Err(e) => TestRunResult {
                    started: Utc::now(),
                    finished: Utc::now(),
                    result: Err(e),
                },
            };
            (test, run_result)
        })
        .buffer_unordered(state.settings.runner.concurrency as usize)
        .collect::<Vec<(Test, TestRunResult)>>()
        .await;

    // Count the types of results
    let run_results = RunResults {
        passed: results.iter().filter(|(_, r)| r.result.is_ok()).count(),
        failed: results.iter().filter(|(_, r)| r.result.is_err()).count(),
        skipped: disabled.len(),
    };

    let processed = database::insert_test_results(state.database(), results).await?;

    let notification_targets = database::fetch_notification_targets(state.database()).await?;

    notify::send_notifications(&processed, notification_targets, state).await?;

    database::update_test_status(state.database(), processed).await?;

    database::delete_expired_records(state.database(), state.settings.clone()).await?;

    Ok(run_results)
}
