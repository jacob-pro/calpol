use crate::database::Connection;
use crate::schema::tests::dsl as Tests;
use crate::schema::*;
use diesel::prelude::*;
use diesel_repository::{implement_crud_repository, CrudRepository};

#[derive(Queryable, Debug, Identifiable, Insertable, AsChangeset)]
pub struct Test {
    pub id: i32,
    pub name: String,
    pub enabled: bool,
    pub config: serde_json::Value,
    pub failing: bool,
    pub failure_threshold: i32,
}

#[derive(Queryable, Debug, Insertable, AsChangeset)]
#[table_name = "tests"]
pub struct NewTest {
    pub name: String,
    pub enabled: bool,
    pub config: serde_json::Value,
    pub failing: bool,
    pub failure_threshold: i32,
}

implement_crud_repository!(TestRepositoryImpl, Test, i32, Connection);

pub trait TestRepository: CrudRepository<Test, i32> {
    fn find_by_name(&self, name: &str) -> QueryResult<Option<Test>>;
}

impl TestRepository for TestRepositoryImpl<'_> {
    fn find_by_name(&self, name: &str) -> QueryResult<Option<Test>> {
        Tests::tests
            .filter(Tests::name.eq(name))
            .first::<_>(self.connection())
            .optional()
    }
}
