use async_trait::async_trait;
use bb8_postgres::{bb8::Pool, tokio_postgres::NoTls, PostgresConnectionManager};

use crate::{Error, Identity};

#[async_trait]
pub trait PgRepository: Sized {
    type View;

    /// New from connection pool.
    fn new(pool: Pool<PostgresConnectionManager<NoTls>>) -> Self;

    /// Connect to the database.
    async fn connect(conn: &str, tls: NoTls) -> Result<Self, bb8_postgres::tokio_postgres::Error>;

    /// Insert or update view.
    async fn save(
        &self,
        id: &str,
        view: Option<&Self::View>,
        event_id: i64,
        event_sequence: i64,
    ) -> Result<(), Error>;

    /// Update view last_event_id and last_event_sequence only.
    async fn update_last_event(
        &self,
        id: &str,
        event_id: i64,
        event_sequence: i64,
    ) -> Result<(), Error>;

    /// Load an existing view with a given `event_id` ensuring the event hasn't already been handled.
    /// If the resource is not found, [Error::ResourceNotFound] error will be returned.
    ///
    /// If the event has been handled already (ie. view's last_event_id is >= event_id),
    /// then [ErrorKind::AlreadyHandledEvent] error will be returned.
    async fn load(&self, id: &str, event_id: i64) -> Result<Self::View, Error> {
        Ok(self
            .load_opt(id, event_id)
            .await?
            .ok_or_else(|| Error::not_found(Some(format!("id = {}", id))))?)
    }

    /// Load a view as an [Option] with a given `event_id` ensuring the event hasn't already been handled.
    ///
    /// If the event has been handled already (ie. view's last_event_id is >= event_id),
    /// then [ErrorKind::AlreadyHandledEvent] error will be returned.
    async fn load_opt(&self, id: &str, event_id: i64) -> Result<Option<Self::View>, Error> {
        let result = self.load_with_last_event_id(id).await?;
        match result {
            Some((view, last_event_id)) => {
                if event_id <= last_event_id {
                    return Err(Error::EventAlreadyHandled(event_id));
                }
                Ok(Some(view))
            }
            None => Ok(None),
        }
    }

    /// Load a view as with a given `event_id` ensuring the event hasn't already been handled.
    /// If the view does not exist, a new one will be created with [Identity::new_with_id].
    ///
    /// If the event has been handled already (ie. view's last_event_id is >= event_id),
    /// then [ErrorKind::AlreadyHandledEvent] error will be returned.
    async fn load_or_new(&self, id: &str, event_id: i64) -> Result<Self::View, Error>
    where
        Self::View: Identity,
    {
        let view_id: <Self::View as Identity>::ID = id.parse().map_err(|_| Error::ParseIdentity)?;
        Ok(self
            .load_opt(id, event_id)
            .await?
            .unwrap_or_else(|| Self::View::new_with_id(view_id)))
    }

    /// Load an existing view and it's last event id without checking if it's already been handled.
    /// > Warning: loading views with this will not ensure events are not handled more than once and is not idempotent.
    async fn load_with_last_event_id(&self, id: &str) -> Result<Option<(Self::View, i64)>, Error>;

    /// Load the latest event version number.
    async fn last_event_id(&self) -> Result<Option<i64>, Error>;

    /// Delete a view with a given id.
    async fn delete(&self, id: &str) -> Result<(), Error>;
}
