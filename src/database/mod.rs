use diesel::r2d2::{ConnectionManager, PooledConnection};
use diesel::PgConnection;

mod runner_logs;
mod sessions;
mod test_results;
mod tests;
mod users;

pub use runner_logs::*;
pub use sessions::*;
pub use test_results::*;
pub use tests::*;
pub use users::*;

pub type Connection = PooledConnection<ConnectionManager<PgConnection>>;
