mod database;
mod notify;
mod runnable;

use crate::database::Test;
use crate::mailer::lettre_async;
use crate::state::AppState;
use crate::test_runner::runnable::Runnable;
use anyhow::Context;
use calpol_model::tests::TestConfig;
use chrono::{DateTime, Utc};
use derive_new::new;
use futures::{stream, StreamExt};
use lettre::{AsyncSmtpTransport, Tokio1Executor};
use std::sync::Arc;
use std::time::Instant;
use tokio::time::{sleep_until, timeout_at};

pub async fn start(state: AppState) -> anyhow::Result<()> {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap()
        .spawn(run_loop(state))
        .await?
}

// TODO: Move mailer directly into AppState once Actix uses Tokio 1.0
pub struct RunnerContext {
    mailer: AsyncSmtpTransport<Tokio1Executor>,
    state: AppState,
}

async fn run_loop(state: AppState) -> anyhow::Result<()> {
    let ctx = RunnerContext {
        mailer: lettre_async(&state.settings().mailer)?,
        state,
    };
    loop {
        log::info!("Beginning test run");
        let start_instant = Instant::now();
        let start_time = Utc::now();
        let max_run_time = start_instant + ctx.state.settings().runner.timeout_duration();
        let result = run_tests(&ctx, max_run_time).await;
        database::write_runner_log(
            ctx.state.database(),
            Arc::clone(ctx.state.settings()),
            result,
            start_time,
        )
        .await;
        let next_run = start_instant + ctx.state.settings().runner.interval_duration();
        sleep_until(next_run.into()).await;
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

async fn run_tests(ctx: &RunnerContext, timeout: Instant) -> anyhow::Result<RunResults> {
    let tests = database::get_tests(ctx.state.database()).await?;
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
        (
            test,
            match config {
                Ok(c) => {
                    let started = Utc::now();
                    let result = timeout_at(timeout.into(), c.run())
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
            },
        )
    })
    .buffer_unordered(ctx.state.settings().runner.concurrency as usize)
    .collect::<Vec<(Test, RunResult)>>()
    .await;
    let passed = results.iter().filter(|(_, r)| r.result.is_ok()).count();
    let failed = results.iter().filter(|(_, r)| r.result.is_err()).count();

    let now_failing = database::process_test_results(
        ctx.state.database(),
        Arc::clone(ctx.state.settings()),
        results,
    )
    .await?;
    let notification_targets = database::fetch_notification_targets(ctx.state.database()).await?;

    notify::send_notifications(&now_failing, notification_targets, &ctx).await?;

    database::mark_tests_as_failing(
        ctx.state.database(),
        now_failing.into_iter().map(|(t, _)| t).collect(),
    )
    .await?;

    Ok(RunResults::new(passed, failed, skipped))
}
