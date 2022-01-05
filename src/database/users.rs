use crate::database::Connection;
use crate::schema::users;
use crate::schema::users::dsl as Users;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel_postgres::functions::{lower, strpos};
use diesel_postgres::limit::{CountedLimitDsl, CountedLimitResult};
use diesel_repository::{implement_crud_repository, CrudRepository};
use lettre::address::AddressError;
use lettre::message::Mailbox;

#[derive(Debug, Queryable, Identifiable, AsChangeset)]
#[changeset_options(treat_none_as_null = "true")]
pub struct User {
    pub id: i32,
    pub name: String,
    pub email: String,
    pub password_hash: Option<String>,
    pub password_reset_token: Option<String>,
    pub password_reset_token_creation: Option<DateTime<Utc>>,
    pub phone_number: Option<String>,
    pub sms_notifications: bool,
    pub email_notifications: bool,
}

impl User {
    pub fn get_mailbox(&self) -> Result<Mailbox, AddressError> {
        Ok(Mailbox::new(Some(self.name.clone()), self.email.parse()?))
    }
}

#[derive(Debug, Insertable)]
#[table_name = "users"]
pub struct NewUser {
    pub name: String,
    pub email: String,
    pub password_hash: Option<String>,
    pub sms_notifications: bool,
    pub email_notifications: bool,
}

implement_crud_repository!(UserRepositoryImpl, User, i32, Connection);

pub trait UserRepository: CrudRepository<User, i32> {
    fn find_all(&self, limit: u32, offset: u32) -> QueryResult<CountedLimitResult<User>>;
    fn find_by_email(&self, email: &str) -> QueryResult<Option<User>>;
    fn find_by_search(
        &self,
        limit: u32,
        offset: u32,
        search: &str,
    ) -> QueryResult<CountedLimitResult<User>>;
    fn find_by_reset_token(&self, reset_token: &str) -> QueryResult<Option<User>>;
}

impl UserRepository for UserRepositoryImpl<'_> {
    fn find_all(&self, limit: u32, offset: u32) -> QueryResult<CountedLimitResult<User>> {
        Users::users
            .counted_limit(limit)
            .offset(offset)
            .load_with_total::<User>(self.connection())
    }

    fn find_by_email(&self, email: &str) -> QueryResult<Option<User>> {
        Users::users
            .filter(lower(Users::email).eq(email.to_ascii_lowercase()))
            .first::<User>(self.connection())
            .optional()
    }

    fn find_by_search(
        &self,
        limit: u32,
        offset: u32,
        search: &str,
    ) -> QueryResult<CountedLimitResult<User>> {
        let like = format!("%{}%", search);
        Users::users
            .filter(Users::name.like(&like))
            .or_filter(Users::email.like(&like))
            .order(strpos(Users::name, search).asc())
            .then_order_by(strpos(Users::email, search).asc())
            .counted_limit(limit)
            .offset(offset)
            .load_with_total::<User>(self.connection())
    }

    fn find_by_reset_token(&self, reset_token: &str) -> QueryResult<Option<User>> {
        Users::users
            .filter(Users::password_reset_token.eq(reset_token))
            .first::<User>(self.connection())
            .optional()
    }
}
