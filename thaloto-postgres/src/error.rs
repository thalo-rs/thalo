use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    DbError(#[from] bb8_postgres::tokio_postgres::Error),
    #[error("deserialize database event {}error: {1}", .0.as_ref().map(|ty| format!("'{}' ", ty)).unwrap_or_default())]
    DeserializeDbEvent(Option<String>, serde_json::Error),
    #[error("get connection from database pool error: {0}")]
    GetDbPoolConnectionError(bb8_postgres::bb8::RunError<bb8_postgres::tokio_postgres::Error>),
    #[error("serialize event error: {0}")]
    SerializeEventError(serde_json::Error),
}
