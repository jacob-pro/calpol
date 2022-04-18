use crate::database::{
    Connection, NewRunnerLog, NewTestResult, RunnerLogRepository, RunnerLogRepositoryImpl, Test,
    TestRepositoryImpl, TestResultRepository, TestResultRepositoryImpl, UserRepositoryImpl,
};
use crate::settings::{RunnerSetting, Settings};
use crate::test_runner::{RunResults, TestRunResult};
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

/// Inserts run results into the database.
pub async fn insert_runner_log(
    database: Connection,
    result: anyhow::Result<RunResults>,
    start_time: DateTime<Utc>,
) -> anyhow::Result<()> {
    spawn_blocking(move || -> anyhow::Result<()> {
        let runner_log_repository = RunnerLogRepositoryImpl::new(&database);
        let mut log = NewRunnerLog {
            time_started: start_time,
            time_finished: Utc::now(),
            success: result.is_ok(),
            failure_reason: None,
            tests_passed: None,
            tests_failed: None,
            tests_skipped: None,
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
        runner_log_repository.insert(log)?;
        Ok(())
    }).await?
}

/// Retrieves all tests (including disabled) from the database.
pub async fn retrieve_tests(database: Connection) -> anyhow::Result<Vec<Test>> {
    spawn_blocking(move || {
        let test_repository = TestRepositoryImpl::new(&database);
        test_repository.find_all().context("Failed to load tests")
    })
    .await?
}

pub struct ProcessedTests {
    /// Tests that were previously failing but have now transitioned to a passing state.
    pub now_passing: Vec<Test>,
    /// Tests that were previously passing but have now transitioned into a failing state.
    /// Includes the latest error indicating why they are failing.
    pub now_failing: Vec<(Test, anyhow::Error)>,
}

/// Inserts test results into the database, and retrieves the new status of the tests
pub async fn insert_test_results(
    database: Connection,
    results: Vec<(Test, TestRunResult)>,
) -> anyhow::Result<ProcessedTests> {
    spawn_blocking(move || -> anyhow::Result<_> {
        let test_result_repository = TestResultRepositoryImpl::new(&database);

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

        // Check the most recent test results to determine if the test has reached the failure threshold
        let results = results.into_iter().map(|(test, result)| {
            let latest = test_result_repository
                .find_latest_belonging_to(&test, test.failure_threshold as u32)
                .context("Loading test results")?;
            let failing = (latest.len() == test.failure_threshold as usize)
                && latest.iter().all(|r| !r.success);
            Ok((test, result, failing))
        }).collect::<anyhow::Result<Vec<_>>>()?;

        // Filter tests that have now transitioned to a passing state
        let (now_passing, remaining): (Vec<_>, Vec<_>) = results.into_iter()
            .partition(|(test, _, failing_now)| {
                // Was previously failing, but is now passing
                test.failing && !(*failing_now)
            });

        // Filter tests that have now transitioned into a failing state
        let now_failing = remaining.into_iter()
            .filter(|(test, _, failing_now)| {
                // Was previously passing, but is now failing
                !test.failing && *failing_now
            }).map(|(test, result, _)| (test, result.result.err().unwrap())).collect();

        Ok(ProcessedTests {
            now_passing: now_passing.into_iter().map(|x| x.0).collect(),
            now_failing
        })
    })
    .await?
}

/// Mark tests as passing / failing in the database.
/// This should be done only once notifications have been successfully sent.
pub async fn update_test_status(
    database: Connection,
    processed: ProcessedTests,
) -> anyhow::Result<()> {
    spawn_blocking(move || -> anyhow::Result<()> {
        let test_repository = TestRepositoryImpl::new(&database);
        for (mut test, _) in processed.now_failing {
            test.failing = true;
            test_repository
                .update(&test)
                .context("Updating test state to failing")?;
        }
        for mut test in processed.now_passing {
            test.failing = false;
            test_repository
                .update(&test)
                .context("Updating test state to passing")?;
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

/// Fetches a list of emails and phone numbers that need to be notified of test results
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

/// Cleans up test results and runner logs that are older than the minimum log age.
pub async fn delete_expired_records(
    database: Connection,
    settings: Arc<Settings>,
) -> anyhow::Result<()> {
    spawn_blocking(move || -> anyhow::Result<_> {
        let test_result_repository = TestResultRepositoryImpl::new(&database);
        test_result_repository
            .delete_all_older_than(settings.runner.minimum_log_age())
            .context("Failed to delete old test results")?;

        let runner_log_repository = RunnerLogRepositoryImpl::new(&database);
        if let Err(e) =
            runner_log_repository.delete_all_older_than(settings.runner.minimum_log_age())
        {
            log::error!("Failed to clean old runner logs: {:#}", e);
        }

        Ok(())
    })
    .await?
}
