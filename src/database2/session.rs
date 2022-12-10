use crate::database2::{
    decode_token, implement_crud_repository, CrudRepository, DbResult, Paginated,
    PaginatedDbResult, SortOrder,
};
use entity::{session, user};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect};
use serde::{Deserialize, Serialize};

implement_crud_repository!(SessionRepository, session);

impl SessionRepository<'_> {
    pub async fn find_belonging_to_user_by_ip_and_agent(
        &self,
        user: &user::Model,
        last_ip: &str,
        user_agent: &str,
    ) -> DbResult<Option<session::Model>> {
        session::Entity::find()
            .filter(session::Column::UserId.eq(user.id))
            .filter(session::Column::LastIp.eq(last_ip))
            .filter(session::Column::UserAgent.eq(user_agent))
            .one(self.db())
            .await
    }

    pub async fn find_by_token(
        &self,
        token: &str,
    ) -> DbResult<Option<(session::Model, user::Model)>> {
        session::Entity::find()
            .inner_join(user::Entity)
            .select_also(user::Entity)
            .filter(session::Column::Token.eq(token))
            .one(self.db())
            .await
            .map(|v| v.map(|(l, r)| (l, r.unwrap())))
    }

    pub async fn delete_belonging_to_user_by_id(
        &self,
        user: &user::Model,
        id: i64,
    ) -> DbResult<bool> {
        session::Entity::delete_many()
            .filter(session::Column::Id.eq(id))
            .filter(session::Column::UserId.eq(user.id))
            .exec(self.db())
            .await
            .map(|d| d.rows_affected > 0)
    }

    pub async fn delete_belonging_to_user(&self, user: &user::Model) -> DbResult<u64> {
        session::Entity::delete_many()
            .filter(session::Column::UserId.eq(user.id))
            .exec(self.db())
            .await
            .map(|d| d.rows_affected)
    }

    pub async fn find_belonging_to_user(
        &self,
        user: &user::Model,
        page_token: Option<&str>,
        page_size: u64,
        sort_order: SortOrder,
    ) -> PaginatedDbResult<session::Model> {
        #[derive(Serialize, Deserialize)]
        struct Token {
            id: i64,
        }
        let mut query = session::Entity::find()
            .filter(session::Column::UserId.eq(user.id))
            .order_by(session::Column::Id, sort_order.into());
        if let Some(page_token) = page_token {
            let page_token = decode_token::<Token>(&page_token)?;
            query = match sort_order {
                SortOrder::Ascending => query.filter(session::Column::Id.gt(page_token.id)),
                SortOrder::Descending => query.filter(session::Column::Id.lt(page_token.id)),
            };
        }
        let results = query.limit(page_size).all(self.db()).await?;
        Ok(Paginated::from_results(results, page_size, |last| Token {
            id: last.id,
        }))
    }
}
