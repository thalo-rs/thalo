use async_trait::async_trait;
use bb8_postgres::{bb8::Pool, tokio_postgres::NoTls, PostgresConnectionManager};

use crate::Error;

#[async_trait]
pub trait PgRepository: Sized {
    type View;

    /// New from connection pool
    fn new(pool: Pool<PostgresConnectionManager<NoTls>>) -> Self;

    /// Connect to the database
    async fn connect(conn: &str, tls: NoTls) -> Result<Self, bb8_postgres::tokio_postgres::Error>;

    /// Insert or update view
    async fn save(
        &self,
        view: &Self::View,
        event_id: i64,
        event_sequence: i64,
    ) -> Result<(), Error>;

    /// Load an existing view
    async fn load(&self, id: &str) -> Result<Option<Self::View>, Error>;

    /// Load the latest event version number
    async fn last_event_id(&self) -> Result<Option<i64>, Error>;
}
