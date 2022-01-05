use crate::database::users::User;
use crate::database::Connection;
use crate::schema::sessions::dsl as Sessions;
use crate::schema::users::dsl as Users;
use crate::schema::*;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::QueryResult;
use diesel_repository::{implement_crud_repository, CrudRepository};

#[derive(Debug, Queryable, Identifiable, Associations, AsChangeset)]
#[belongs_to(User)]
pub struct Session {
    pub id: i32,
    pub user_id: i32,
    pub token: String,
    pub created: DateTime<Utc>,
    pub last_used: DateTime<Utc>,
    pub last_ip: Vec<u8>,
    pub user_agent: String,
}

#[derive(Debug, Insertable)]
#[table_name = "sessions"]
pub struct NewSession {
    pub user_id: i32,
    pub token: String,
    pub last_ip: Vec<u8>,
    pub user_agent: String,
}

implement_crud_repository!(SessionRepositoryImpl, Session, i32, Connection);

pub trait SessionRepository: CrudRepository<Session, i32> {
    fn find_belonging_to_user_by_ip_and_agent(
        &self,
        user: &User,
        last_ip: &Vec<u8>,
        user_agent: &str,
    ) -> QueryResult<Option<Session>>;

    fn find_by_token(&self, token: &str) -> QueryResult<Option<(Session, User)>>;

    fn find_all_belonging_to_user(&self, user: &User) -> QueryResult<Vec<Session>>;

    fn delete_by_id_and_user(&self, session_id: i32, user: &User) -> QueryResult<bool>;

    fn delete_all_belonging_to(&self, user: &User) -> QueryResult<usize>;
}

impl SessionRepository for SessionRepositoryImpl<'_> {
    fn find_belonging_to_user_by_ip_and_agent(
        &self,
        user: &User,
        last_ip: &Vec<u8>,
        user_agent: &str,
    ) -> QueryResult<Option<Session>> {
        Session::belonging_to(user)
            .filter(Sessions::last_ip.eq(last_ip))
            .filter(Sessions::user_agent.eq(user_agent))
            .first::<Session>(self.connection())
            .optional()
    }

    fn find_by_token(&self, token: &str) -> QueryResult<Option<(Session, User)>> {
        Sessions::sessions
            .filter(Sessions::token.eq(token))
            .inner_join(Users::users)
            .first::<(Session, User)>(self.connection())
            .optional()
    }

    fn find_all_belonging_to_user(&self, user: &User) -> QueryResult<Vec<Session>> {
        Session::belonging_to(user).load(self.connection())
    }

    fn delete_by_id_and_user(&self, session_id: i32, user: &User) -> QueryResult<bool> {
        diesel::delete(
            Sessions::sessions.filter(
                Sessions::user_id
                    .eq(user.id)
                    .and(Sessions::id.eq(session_id)),
            ),
        )
        .execute(self.connection())
        .map(|affected| affected > 0)
    }

    fn delete_all_belonging_to(&self, user: &User) -> QueryResult<usize> {
        diesel::delete(Sessions::sessions.filter(Sessions::user_id.eq(user.id)))
            .execute(self.connection())
    }
}
