use crate::database2::{
    implement_crud_repository, CrudRepository, DbResult, PaginatedDbResult, SortOrder,
};
use entity::test;
use sea_orm::sea_query::{Expr, Func};
use sea_orm::{EntityTrait, QueryFilter};

implement_crud_repository!(TestRepository, test);

impl TestRepository<'_> {
    pub async fn find(
        &self,
        _page_token: Option<&str>,
        _page_size: u64,
        _sort_order: SortOrder,
    ) -> PaginatedDbResult<test::Model> {
        unimplemented!()
    }

    pub async fn find_by_name(&self, name: &str) -> DbResult<Option<test::Model>> {
        test::Entity::find()
            .filter(Func::lower(Expr::col(test::Column::Name)).equals(name))
            .one(self.db())
            .await
    }
}
