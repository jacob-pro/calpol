use crate::database2::{
    decode_token, implement_crud_repository, CrudRepository, DbResult, Paginated,
    PaginatedDbResult, SortOrder,
};
use entity::user;
use sea_orm::sea_query::{Expr, Func};
use sea_orm::{ColumnTrait, Condition, EntityTrait, QueryFilter, QueryOrder, QuerySelect};
use serde::{Deserialize, Serialize};

implement_crud_repository!(UserRepository, user);

#[derive(Debug, Clone, Copy)]
pub enum SortBy {
    Id,
    Name,
    Email,
}

impl UserRepository<'_> {
    async fn find(
        &self,
        page_token: Option<&str>,
        page_size: u64,
        sort_order: SortOrder,
        sort_by: SortBy,
        search_filter: Option<&str>,
    ) -> PaginatedDbResult<user::Model> {
        #[derive(Serialize, Deserialize)]
        struct Token {
            id: i64,
            email: String,
            name: String,
        }
        let mut query = user::Entity::find();
        query = match sort_by {
            SortBy::Id => query.order_by(user::Column::Id, sort_order.into()),
            SortBy::Name => query
                .order_by(user::Column::Name, sort_order.into())
                .order_by(user::Column::Id, sort_order.into()),
            SortBy::Email => query.order_by(user::Column::Email, sort_order.into()),
        };
        if let Some(page_token) = page_token {
            let page_token = decode_token::<Token>(&page_token)?;
            query = match sort_by {
                SortBy::Id => query.filter(sort_order.after(user::Column::Id, page_token.id)),
                SortBy::Name => query.filter(
                    Condition::any()
                        .add(sort_order.after(user::Column::Name, page_token.name.as_str()))
                        .add(
                            Condition::all()
                                .add(user::Column::Name.eq(page_token.name.as_str()))
                                .add(sort_order.after(user::Column::Id, page_token.id)),
                        ),
                ),
                SortBy::Email => query.filter(sort_order.after(user::Column::Email, page_token.id)),
            }
        }
        if let Some(search_filter) = search_filter {
            let like = format!("%{}%", search_filter);
            query = query.filter(
                Condition::any()
                    .add(user::Column::Name.like(&like))
                    .add(user::Column::Email.like(&like)),
            );
        }
        let results = query.limit(page_size).all(self.db()).await?;
        Ok(Paginated::from_results(results, page_size, |last| Token {
            id: last.id,
            email: last.email.clone(),
            name: last.name.clone(),
        }))
    }

    async fn find_by_email(&self, email: &str) -> DbResult<Option<user::Model>> {
        let lower = email.to_lowercase().trim();
        user::Entity::find()
            .filter(Func::lower(Expr::col(user::Column::Name)).equals(lower))
            .one(self.db())
            .await
    }

    async fn find_by_reset_token(&self, reset_token: &str) -> DbResult<Option<user::Model>> {
        user::Entity::find()
            .filter(user::Column::PasswordResetToken.eq(reset_token))
            .one(self.db())
            .await
    }
}
