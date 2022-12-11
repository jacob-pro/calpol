mod runner_log;
mod session;
mod test;
mod test_result;
mod user;

pub use runner_log::RunnerLogRepository;
pub use session::SessionRepository;
pub use test::TestRepository;
pub use test_result::TestResultRepository;
pub use user::UserRepository;

use async_trait::async_trait;
use sea_orm::sea_query::SimpleExpr;
use sea_orm::{
    ActiveModelBehavior, ActiveModelTrait, ColumnTrait, DatabaseConnection, DbErr, EntityTrait,
    IntoActiveModel, ModelTrait, PrimaryKeyTrait, Value,
};
use serde::de::DeserializeOwned;
use serde::Serialize;
use thiserror::Error;

pub type DbResult<T> = Result<T, DbErr>;
pub type PaginatedDbResult<T> = Result<Paginated<T>, PaginatedErr>;

#[derive(Debug)]
pub struct Paginated<T> {
    pub rows: Vec<T>,
    pub next_page: Option<String>,
}

impl<T> Paginated<T> {
    fn from_rows<F, E>(rows: Vec<T>, page_size: u64, next_page_fn: F) -> Self
    where
        F: FnOnce(&T) -> E,
        E: Serialize,
    {
        assert!(rows.len() as u64 <= page_size);
        let mut next_page = None;
        if let Some(last) = rows.last() {
            if rows.len() as u64 == page_size {
                next_page = Some(encode_token(&next_page_fn(last)));
            }
        }
        Self { rows, next_page }
    }
}

#[derive(Copy, Clone, Default)]
pub enum SortOrder {
    #[default]
    Ascending,
    Descending,
}

impl SortOrder {
    fn after<C: ColumnTrait, V: Into<Value>>(&self, c: C, v: V) -> SimpleExpr {
        match self {
            SortOrder::Ascending => c.gt(v),
            SortOrder::Descending => c.lt(v),
        }
    }
}

impl From<SortOrder> for sea_orm::Order {
    fn from(s: SortOrder) -> Self {
        match s {
            SortOrder::Ascending => Self::Asc,
            SortOrder::Descending => Self::Desc,
        }
    }
}

#[derive(Debug, Error)]
#[error("Invalid page token")]
pub struct InvalidTokenError;

fn decode_token<T: DeserializeOwned>(token: &str) -> Result<T, InvalidTokenError> {
    let bytes = base64::decode(token).map_err(|_| InvalidTokenError)?;
    serde_json::from_slice(&bytes).map_err(|_| InvalidTokenError)
}

fn encode_token<T: Serialize>(token: &T) -> String {
    let json = serde_json::to_string(token).unwrap();
    base64::encode(&json)
}

#[derive(Debug, Error)]
pub enum PaginatedErr {
    #[error("{0}")]
    DbErr(
        #[from]
        #[source]
        DbErr,
    ),
    #[error("{0}")]
    Token(
        #[from]
        #[source]
        InvalidTokenError,
    ),
}

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
use implement_crud_repository;
