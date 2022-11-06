mod session;

use async_trait::async_trait;
use sea_orm::{
    ActiveModelBehavior, ActiveModelTrait, DatabaseConnection, DbErr, EntityTrait, IntoActiveModel,
    ModelTrait, PrimaryKeyTrait,
};

pub type DbResult<T> = std::result::Result<T, DbErr>;

#[async_trait(?Send)]
pub trait CrudRepository<E, M, A>
where
    E: EntityTrait<Model = M>,
    M: ModelTrait<Entity = E> + IntoActiveModel<A>,
    A: ActiveModelBehavior + ActiveModelTrait<Entity = E> + Send,
{
    fn db(&self) -> &DatabaseConnection;

    async fn find_by_id(
        &self,
        id: <E::PrimaryKey as PrimaryKeyTrait>::ValueType,
    ) -> DbResult<Option<M>> {
        E::find_by_id(id).one(self.db()).await
    }

    async fn delete<'a>(&self, model: M) -> DbResult<bool>
    where
        M: 'a,
    {
        model.delete(self.db()).await.map(|r| r.rows_affected > 0)
    }

    async fn delete_by_id(
        &self,
        id: <E::PrimaryKey as PrimaryKeyTrait>::ValueType,
    ) -> DbResult<bool> {
        E::delete_by_id(id)
            .exec(self.db())
            .await
            .map(|r| r.rows_affected > 0)
    }

    async fn update<'a>(&self, active_model: A) -> DbResult<M>
    where
        A: 'a,
    {
        active_model.update(self.db()).await
    }

    async fn insert<'a>(&self, active_model: A) -> DbResult<M>
    where
        A: 'a,
    {
        active_model.insert(self.db()).await
    }
}

#[macro_export]
/// Generates a structure that implements `CrudRepository`
/// # Arguments
/// * `name` - The name of the implementation to generate
/// * `module` - The sea_orm entity module
macro_rules! implement_crud_repository {
    ( $name:ident, $module:ident ) => {
        pub struct $name<'l>(&'l sea_orm::DatabaseConnection);

        impl<'l> $name<'l> {
            pub fn new(connection: &'l sea_orm::DatabaseConnection) -> Self {
                Self(connection)
            }
        }

        impl crate::database2::CrudRepository<$module::Entity, $module::Model, $module::ActiveModel>
            for $name<'_>
        {
            fn db(&self) -> &sea_orm::DatabaseConnection {
                &self.0
            }
        }
    };
}
pub use implement_crud_repository;
