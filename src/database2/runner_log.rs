use crate::database2::{
    implement_crud_repository, CrudRepository, DbResult, PaginatedDbResult, SortOrder,
};
use chrono::{DateTime, FixedOffset};
use entity::runner_log;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};

implement_crud_repository!(RunnerLogRepository, runner_log);

impl RunnerLogRepository<'_> {
    pub async fn delete_all_older_than(&self, age: DateTime<FixedOffset>) -> DbResult<u64> {
        runner_log::Entity::delete_many()
            .filter(runner_log::Column::TimeFinished.lt(age))
            .exec(self.db())
            .await
            .map(|x| x.rows_affected)
    }

    pub async fn find(
        &self,
        _page_token: Option<&str>,
        _page_size: u64,
        _sort_order: SortOrder,
    ) -> PaginatedDbResult<runner_log::Model> {
        #[derive(Serialize, Deserialize)]
        struct Token {
            id: i64,
        }
        unimplemented!()
    }
}
