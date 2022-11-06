use crate::database2::{CrudRepository, DbResult};
use crate::implement_crud_repository;
use entity::{session, user};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

implement_crud_repository!(SessionRepository, session);

impl SessionRepository<'_> {
    async fn find_belonging_to_user_by_ip_and_agent(
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

    async fn find_by_token(&self, token: &str) -> DbResult<Option<(session::Model, user::Model)>> {
        session::Entity::find()
            .inner_join(user::Entity)
            .select_also(user::Entity)
            .filter(session::Column::Token.eq(token))
            .one(self.db())
            .await
            .map(|v| v.map(|(l, r)| (l, r.unwrap())))
    }

    async fn delete_belonging_to_user_by_id(&self, user: &user::Model, id: i64) -> DbResult<bool> {
        session::Entity::delete_many()
            .filter(session::Column::Id.eq(id))
            .filter(session::Column::UserId.eq(user.id))
            .exec(self.db())
            .await
            .map(|d| d.rows_affected > 0)
    }

    async fn delete_belonging_to_user(&self, user: &user::Model) -> DbResult<u64> {
        session::Entity::delete_many()
            .filter(session::Column::UserId.eq(user.id))
            .exec(self.db())
            .await
            .map(|d| d.rows_affected)
    }
}
