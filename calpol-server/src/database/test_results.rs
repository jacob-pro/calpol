use crate::database::{Connection, Test};
use crate::schema::test_results::dsl as TestResults;
use crate::schema::*;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel_repository::{implement_crud_repository, CrudRepository};

#[derive(Queryable, Debug, Identifiable, Insertable, AsChangeset)]
pub struct TestResult {
    pub id: i32,
    pub test_id: i32,
    pub success: bool,
    pub failure_reason: Option<String>,
    pub time_started: DateTime<Utc>,
    pub time_finished: DateTime<Utc>,
}

#[derive(Queryable, Debug, Insertable, AsChangeset)]
#[table_name = "test_results"]
pub struct NewTestResult {
    pub test_id: i32,
    pub success: bool,
    pub failure_reason: Option<String>,
    pub time_started: DateTime<Utc>,
    pub time_finished: DateTime<Utc>,
}

implement_crud_repository!(TestResultRepositoryImpl, TestResult, i32, Connection);

pub trait TestResultRepository: CrudRepository<TestResult, i32> {
    fn delete_all_older_than(&self, age: DateTime<Utc>) -> QueryResult<usize>;
    fn delete_all_belonging_to(&self, test: &Test) -> QueryResult<usize>;
    fn find_latest_belonging_to(&self, test: &Test, limit: u32) -> QueryResult<Vec<TestResult>>;
}

impl TestResultRepository for TestResultRepositoryImpl<'_> {
    fn delete_all_older_than(&self, age: DateTime<Utc>) -> QueryResult<usize> {
        diesel::delete(TestResults::test_results.filter(TestResults::time_finished.lt(age)))
            .execute(self.connection())
    }

    fn delete_all_belonging_to(&self, test: &Test) -> QueryResult<usize> {
        diesel::delete(TestResults::test_results.filter(TestResults::test_id.eq(test.id)))
            .execute(self.connection())
    }

    fn find_latest_belonging_to(&self, test: &Test, limit: u32) -> QueryResult<Vec<TestResult>> {
        TestResults::test_results
            .filter(TestResults::test_id.eq(test.id))
            .limit(limit as i64)
            .order(TestResults::id.desc())
            .load(self.connection())
    }
}
