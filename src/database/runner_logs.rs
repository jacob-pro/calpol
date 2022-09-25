use crate::database::Connection;
use crate::schema::runner_logs::dsl as RunnerLogs;
use crate::schema::*;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel_postgres::limit::{CountedLimitDsl, CountedLimitResult};
use diesel_repository::{implement_crud_repository, CrudRepository};

#[derive(Queryable, Debug, Identifiable, Insertable, AsChangeset)]
pub struct RunnerLog {
    pub id: i32,
    pub time_started: DateTime<Utc>,
    pub time_finished: DateTime<Utc>,
    pub success: bool,
    pub failure_reason: Option<String>,
    pub tests_passed: Option<i32>,
    pub tests_failed: Option<i32>,
    pub tests_skipped: Option<i32>,
}

#[derive(Queryable, Debug, Insertable, AsChangeset)]
#[table_name = "runner_logs"]
pub struct NewRunnerLog {
    pub time_started: DateTime<Utc>,
    pub time_finished: DateTime<Utc>,
    pub success: bool,
    pub failure_reason: Option<String>,
    pub tests_passed: Option<i32>,
    pub tests_failed: Option<i32>,
    pub tests_skipped: Option<i32>,
}

implement_crud_repository!(RunnerLogRepositoryImpl, RunnerLog, i32, Connection);

pub trait RunnerLogRepository: CrudRepository<RunnerLog, i32> {
    fn delete_all_older_than(&self, age: DateTime<Utc>) -> QueryResult<usize>;
    fn find_all(&self, limit: u32, offset: u32) -> QueryResult<CountedLimitResult<RunnerLog>>;
}

impl RunnerLogRepository for RunnerLogRepositoryImpl<'_> {
    fn delete_all_older_than(&self, age: DateTime<Utc>) -> QueryResult<usize> {
        diesel::delete(RunnerLogs::runner_logs.filter(RunnerLogs::time_finished.lt(age)))
            .execute(self.connection())
    }

    fn find_all(&self, limit: u32, offset: u32) -> QueryResult<CountedLimitResult<RunnerLog>> {
        RunnerLogs::runner_logs
            .order(RunnerLogs::id.desc())
            .counted_limit(limit)
            .offset(offset)
            .load_with_total::<RunnerLog>(self.connection())
    }
}
