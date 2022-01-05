#![allow(unused)]
#![allow(clippy::all)]

use chrono::offset::Utc;
use chrono::DateTime;
use diesel::sql_types::Jsonb;

#[derive(Queryable, Debug)]
pub struct Result {
    pub id: i32,
    pub test_id: i32,
    pub success: bool,
    pub failure_reason: Option<String>,
}

#[derive(Queryable, Debug)]
pub struct Test {
    pub id: i32,
    pub name: String,
    pub enabled: bool,
    pub config: Jsonb,
}
