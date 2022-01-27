use crate::database::{
    Connection, NewRunnerLog, NewTestResult, RunnerLogRepository, RunnerLogRepositoryImpl, Test,
    TestRepositoryImpl, TestResultRepository, TestResultRepositoryImpl, UserRepositoryImpl,
};
use crate::settings::{RunnerSetting, Settings};
use crate::test_runner::{RunResult, RunResults};
use anyhow::Context;
use chrono::{DateTime, Duration, Utc};
use diesel::QueryResult;
use diesel_repository::CrudRepository;
use lettre::message::Mailbox;
use std::sync::Arc;
use tokio::task::spawn_blocking;

impl RunnerSetting {
    fn minimum_log_age(&self) -> DateTime<Utc> {
        Utc::now() - Duration::days(self.log_age as i64)
    }
}

pub async fn write_runner_log(
    database: Connection,
    settings: Arc<Settings>,
    result: anyhow::Result<RunResults>,
    start_time: DateTime<Utc>,
) {
    if let Err(e) = spawn_blocking(move || {
        let runner_log_repository = RunnerLogRepositoryImpl::new(&database);
        let mut log = NewRunnerLog {
            time_started: start_time,
            time_finished: Utc::now(),
            success: result.is_ok(),
            failure_reason: None,
            tests_passed: None,
            tests_failed: None,
            tests_skipped: None
        };
        let duration = log.time_finished - log.time_started;
        match result {
            Ok(r) => {
                log.tests_passed = Some(r.passed as i32);
                log.tests_failed = Some(r.failed as i32);
                log.tests_skipped = Some(r.skipped as i32);
                log::info!(
                    "Test runner completed in {}:{:02}, {} tests passed, {} tests failed, {} tests skipped",
                    duration.num_minutes(),
                    duration.num_seconds(),
                    r.passed,
                    r.failed,
                    r.skipped
                );
            }
            Err(e) => {
                log.failure_reason = Some(format!("{:#}", e));
                log::error!(
                    "Test runner failed in {}:{}, error: {:#}",
                    duration.num_minutes(),
                    duration.num_seconds(),
                    e
                );
            }
        }
        if let Err(e) = runner_log_repository.insert(log) {
            log::error!("Failed to write runner log to the database: {:#}", e);
        }
        if let Err(e) = runner_log_repository.delete_all_older_than(settings.runner.minimum_log_age()) {
            log::error!("Failed to clean old runner logs: {:#}", e);
        }
    })
    .await {
        log::error!("Failed to spawn runner log task: {}", e);
    }
}

pub async fn get_tests(database: Connection) -> anyhow::Result<Vec<Test>> {
    spawn_blocking(move || {
        let test_repository = TestRepositoryImpl::new(&database);
        test_repository.find_all().context("Failed to load tests")
    })
    .await?
}

/// Updates the database state, returns only the tests that have transitioned to a failing state
pub async fn process_test_results(
    database: Connection,
    settings: Arc<Settings>,
    results: Vec<(Test, RunResult)>,
) -> anyhow::Result<Vec<(Test, anyhow::Error)>> {
    spawn_blocking(move || -> anyhow::Result<_> {
        let test_result_repository = TestResultRepositoryImpl::new(&database);
        let test_repository = TestRepositoryImpl::new(&database);

        // Write the result of this test run to the database
        for (test, result) in &results {
            let test_result = NewTestResult {
                test_id: test.id,
                success: result.result.is_ok(),
                failure_reason: result.result.as_ref().err().map(|e| format!("{:#}", e)),
                time_started: result.started,
                time_finished: result.finished,
            };
            test_result_repository.insert(test_result).allow_foreign_key_violation(|info| {
                log::warn!(
                    "Failed to insert result for test {}, due to foreign key violation: {}, it may have been deleted",
                    test.id,
                    info.table_name().unwrap_or("")
                );
            }).context("Failed to insert test result")?;
        }
        test_result_repository
            .delete_all_older_than(settings.runner.minimum_log_age())
            .context("Failed to delete old test results")?;

        // Check the most recent test results to determine if the test has reached the failure threshold
        let results = results.into_iter().map(|(test, result)| {
            let latest = test_result_repository
                .find_latest_belonging_to(&test, test.failure_threshold as u32)
                .context("Loading test results")?;
            let failing = (latest.len() == test.failure_threshold as usize)
                && latest.iter().all(|r| !r.success);
            Ok((test, result, failing))
        }).collect::<anyhow::Result<Vec<_>>>()?;

        // Find previously failing tests that have now transitioned to a non failing state and update the database
        let (now_passing, remaining): (Vec<_>, Vec<_>) = results.into_iter()
            .partition(|(test, _, failing_now)| {
                test.failing && !(*failing_now)
            });
        for (mut test, _, _) in now_passing {
            test.failing = false;
            test_repository.update(&test).context("Updating test state to passing")?;
        }

        // Filter tests that weren't failing before but have now transitioned into a failing state
        let now_failing = remaining.into_iter()
            .filter(|(test, _, failing_now)| {
                !test.failing && *failing_now
            }).map(|(test, result, _)| (test, result.result.err().unwrap())).collect();

        Ok(now_failing)
    })
    .await?
}

/// Mark tests as failing in the database.\
/// This should be done only once notifications have been successfully sent
pub async fn mark_tests_as_failing(database: Connection, failed: Vec<Test>) -> anyhow::Result<()> {
    spawn_blocking(move || -> anyhow::Result<()> {
        let test_repository = TestRepositoryImpl::new(&database);
        for mut test in failed {
            test.failing = true;
            test_repository
                .update(&test)
                .context("Updating test state to failing")?;
        }
        Ok(())
    })
    .await?
}

trait AllowDieselForeignKeyViolation<T, F> {
    fn allow_foreign_key_violation(self, f: F) -> QueryResult<()>;
}

impl<T, F> AllowDieselForeignKeyViolation<T, F> for diesel::QueryResult<T>
where
    F: Fn(&dyn diesel::result::DatabaseErrorInformation),
{
    fn allow_foreign_key_violation(self, f: F) -> QueryResult<()> {
        if let Err(diesel::result::Error::DatabaseError(
            diesel::result::DatabaseErrorKind::ForeignKeyViolation,
            m,
        )) = &self
        {
            f(m.as_ref());
            return Ok(());
        }
        self.map(|_| ())
    }
}

#[derive(Default)]
pub struct NotificationTargets {
    pub emails: Vec<Mailbox>,
    pub sms: Vec<String>,
}

pub async fn fetch_notification_targets(
    database: Connection,
) -> anyhow::Result<NotificationTargets> {
    spawn_blocking(move || -> anyhow::Result<_> {
        let mut targets = NotificationTargets::default();
        let user_repository = UserRepositoryImpl::new(&database);
        for user in user_repository.find_all().context("Failed to load users")? {
            if user.email_notifications {
                match user.get_mailbox() {
                    Ok(m) => targets.emails.push(m),
                    Err(e) => log::error!("Failed to get mailbox for user {}: {}", user.id, e),
                }
            }
            if let Some(number) = user.phone_number {
                if user.sms_notifications {
                    targets.sms.push(number);
                }
            }
        }
        Ok(targets)
    })
    .await?
}
