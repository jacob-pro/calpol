use lettre::Address;
use crate::model::tests::TestConfig;
use serde::{Deserialize, Serialize};
use validator::Validate;

const DEFAULT_LIMIT: u32 = 50;

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserSummary {
    pub id: i32,
    pub name: String,
    pub email: String,
    pub phone_number: Option<String>,
    pub sms_notifications: bool,
    pub email_notifications: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    pub id: i32,
    pub created: i64,
    pub last_used: i64,
    pub last_ip: String,
    pub user_agent: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginResponse {
    pub user: UserSummary,
    pub session: SessionSummary,
    pub token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
#[serde(default)]
pub struct ListUsersRequest {
    #[validate(range(min = 1, max = 100))]
    pub limit: u32,
    pub offset: u32,
    pub search: Option<String>,
}

impl Default for ListUsersRequest {
    fn default() -> Self {
        ListUsersRequest {
            limit: DEFAULT_LIMIT,
            offset: 0,
            search: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListUsersResponse {
    pub users: Vec<UserSummary>,
    pub total: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct CreateUserRequest {
    #[validate(length(min = 1, max = 255))]
    pub name: String,
    pub email: Address,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct UpdateUserRequest {
    #[validate(length(min = 1, max = 255))]
    pub name: Option<String>,
    pub email: Option<Address>,
    #[validate(phone)]
    pub phone_number: Option<String>,
    pub sms_notifications: Option<bool>,
    pub email_notifications: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct ResetPasswordRequest {
    pub email: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct SubmitPasswordResetRequest {
    pub token: String,
    #[validate(length(min = 16, max = 255))]
    pub new_password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct CreateTestRequest {
    pub name: String,
    pub config: TestConfig,
    #[serde(default = "default_test_enabled")]
    pub enabled: bool,
    #[serde(default = "default_test_failure_threshold")]
    #[validate(range(min = 1))]
    pub failure_threshold: u8,
}

fn default_test_failure_threshold() -> u8 {
    2
}

fn default_test_enabled() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct UpdateTestRequest {
    #[validate]
    pub config: Option<TestConfig>,
    pub enabled: Option<bool>,
    pub failure_threshold: Option<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestSummary {
    pub name: String,
    pub config: TestConfig,
    pub enabled: bool,
    pub failure_threshold: u8,
    pub failing: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResultSummary {
    pub test_name: String,
    pub success: bool,
    pub failure_reason: Option<String>,
    pub time_started: String,
    pub time_finished: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct GetTestResultsRequest {
    pub limit: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct ListRunnerLogsRequest {
    #[validate(range(min = 1, max = 100))]
    pub limit: u32,
    pub offset: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListRunnerLogsResponse {
    pub logs: Vec<RunnerLogSummary>,
    pub total: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunnerLogSummary {
    pub id: i32,
    pub time_started: String,
    pub time_finished: String,
    pub success: bool,
    pub failure_reason: Option<String>,
    pub tests_passed: Option<i32>,
    pub tests_failed: Option<i32>,
    pub tests_skipped: Option<i32>,
}
