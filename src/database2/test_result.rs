use crate::database2::{
    implement_crud_repository, CrudRepository, DbResult, PaginatedDbResult, SortOrder,
};
use chrono::{DateTime, FixedOffset};
use entity::{test, test_result};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};

implement_crud_repository!(TestResultRepository, test_result);

impl TestResultRepository<'_> {
    pub async fn delete_all_older_than(&self, age: DateTime<FixedOffset>) -> DbResult<u64> {
        test_result::Entity::delete_many()
            .filter(test_result::Column::TimeFinished.lt(age))
            .exec(self.db())
            .await
            .map(|x| x.rows_affected)
    }

    pub async fn delete_all_belonging_to_test(&self, test: &test::Model) -> DbResult<u64> {
        test_result::Entity::delete_many()
            .filter(test_result::Column::TestId.eq(test.id))
            .exec(self.db())
            .await
            .map(|x| x.rows_affected)
    }

    pub async fn find(
        &self,
        _test_id: Option<i64>,
        _page_token: Option<&str>,
        _page_size: u64,
        _sort_order: SortOrder,
    ) -> PaginatedDbResult<test_result::Model> {
        #[derive(Serialize, Deserialize)]
        struct Token {
            id: i64,
        }
        unimplemented!()
    }
}
