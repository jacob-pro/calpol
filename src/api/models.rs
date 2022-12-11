use chrono::{DateTime, FixedOffset, Utc};
use lettre::Address;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

const DEFAULT_LIMIT: u32 = 50;
pub const DEFAULT_PAGE_SIZE: u32 = 50;

#[derive(Copy, Clone, Default, Debug, Deserialize, ToSchema)]
pub enum SortOrder {
    #[default]
    Ascending,
    Descending,
}

impl Into<crate::database2::SortOrder> for SortOrder {
    fn into(self) -> crate::database2::SortOrder {
        match self {
            SortOrder::Ascending => crate::database2::SortOrder::Ascending,
            SortOrder::Descending => crate::database2::SortOrder::Descending,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct LoginRequest {
    pub email: String,
    #[schema(format=Password)]
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct User {
    pub id: i32,
    pub name: String,
    pub email: String,
    pub phone_number: Option<String>,
    pub sms_notifications: bool,
    pub email_notifications: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Session {
    pub id: i32,
    pub created: DateTime<Utc>,
    pub last_used: DateTime<Utc>,
    pub last_ip: String,
    pub user_agent: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ListSessionsResponse {
    pub items: Vec<Session>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LoginResponse {
    pub user: User,
    pub session: Session,
    pub token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
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

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ListUsersResponse {
    pub items: Vec<User>,
    pub total: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct CreateUserRequest {
    #[validate(length(min = 1, max = 255))]
    pub name: String,
    #[schema(value_type = String, format = "email")]
    pub email: Address,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct UpdateUserRequest {
    #[validate(length(min = 1, max = 255))]
    pub name: Option<String>,
    #[schema(value_type = String, format = "email")]
    pub email: Option<Address>,
    #[validate(phone)]
    #[schema(nullable)]
    pub phone_number: Option<Option<String>>,
    pub sms_notifications: Option<bool>,
    pub email_notifications: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct PasswordResetRequest {
    pub email: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct SubmitPasswordResetRequest {
    pub token: String,
    #[validate(length(min = 16, max = 255))]
    #[schema(format=Password)]
    pub new_password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct CreateTestRequest {
    pub name: String,
    #[schema(value_type=Object)]
    pub config: serde_json::Value,
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

#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct UpdateTestRequest {
    #[schema(value_type=Object)]
    pub config: Option<serde_json::Value>,
    pub enabled: Option<bool>,
    pub failure_threshold: Option<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Test {
    pub name: String,
    #[schema(value_type=Object)]
    pub config: serde_json::Value,
    pub enabled: bool,
    pub failure_threshold: u8,
    pub failing: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ListTestsResponse {
    pub items: Vec<Test>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TestResult {
    pub test_name: String,
    pub success: bool,
    pub failure_reason: Option<String>,
    pub time_started: DateTime<Utc>,
    pub time_finished: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ListTestResultsResponse {
    pub items: Vec<TestResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct GetTestResultsRequest {
    pub limit: u32,
}

#[derive(Debug, Clone, Deserialize, Validate, ToSchema)]
pub struct ListRunnerLogsRequest {
    #[validate(range(min = 1, max = 100))]
    pub page_size: Option<u32>,
    pub page_token: Option<String>,
    pub sort_order: Option<SortOrder>,
}

#[derive(ToSchema, Debug, Clone, Serialize, Deserialize)]
pub struct ListRunnerLogsResponse {
    pub items: Vec<RunnerLog>,
    pub next_page: Option<String>,
}

#[derive(ToSchema, Debug, Clone, Serialize, Deserialize)]
pub struct RunnerLog {
    pub id: i64,
    pub time_started: DateTime<FixedOffset>,
    pub time_finished: DateTime<FixedOffset>,
    pub failure: Option<String>,
    pub tests_passed: Option<i32>,
    pub tests_failed: Option<i32>,
    pub tests_skipped: Option<i32>,
}
