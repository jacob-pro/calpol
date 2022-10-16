use crate::api::models::*;
use crate::database;
use std::net::IpAddr;

impl From<database::User> for User {
    fn from(user: database::User) -> Self {
        Self {
            id: user.id,
            name: user.name,
            email: user.email,
            phone_number: user.phone_number,
            sms_notifications: user.sms_notifications,
            email_notifications: user.email_notifications,
        }
    }
}

impl From<database::Session> for Session {
    fn from(session: database::Session) -> Self {
        Self {
            id: session.id,
            created: session.created,
            last_used: session.last_used,
            last_ip: bincode::deserialize::<IpAddr>(&session.last_ip)
                .ok()
                .map(|ip| ip.to_string())
                .unwrap_or_else(|| "".to_string()),
            user_agent: session.user_agent,
        }
    }
}

impl From<database::Test> for Test {
    fn from(test: database::Test) -> Self {
        Self {
            name: test.name,
            config: test.config,
            enabled: test.enabled,
            failure_threshold: test.failure_threshold as u8,
            failing: test.failing,
        }
    }
}

pub fn test_and_result_to_summary(
    test: &database::Test,
    result: database::TestResult,
) -> TestResult {
    TestResult {
        test_name: test.name.clone(),
        success: result.success,
        failure_reason: result.failure_reason,
        time_started: result.time_started,
        time_finished: result.time_finished,
    }
}

impl From<database::RunnerLog> for RunnerLog {
    fn from(log: database::RunnerLog) -> Self {
        RunnerLog {
            id: log.id,
            time_started: log.time_started,
            time_finished: log.time_finished,
            failure_reason: log.failure_reason,
            tests_passed: log.tests_passed,
            tests_failed: log.tests_failed,
            tests_skipped: log.tests_skipped,
        }
    }
}
