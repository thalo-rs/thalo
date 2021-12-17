use std::borrow::Cow;

use thiserror::Error;
use tracing::{metadata::LevelFilter, Level};

#[derive(Error, Debug)]
pub enum Error {
    #[error("aggregate '{0}' channel not initialised")]
    AggregateChannelNotInitialised(&'static str),
    #[error("create consumer error: {0}")]
    CreateConsumerError(rdkafka::error::KafkaError),
    #[error("deserialize database event {}error: {1}", .0.as_ref().map(|ty| format!("'{}' ", ty)).unwrap_or_default())]
    DeserializeDbEventError(Option<String>, serde_json::Error),
    #[error(transparent)]
    DbError(#[from] bb8_postgres::tokio_postgres::Error),
    #[error("event {0} already handled")]
    EventAlreadyHandled(i64),
    #[error("event '{0}' missing from view")]
    EventMissing(&'static str),
    #[error("fetch topic metadata error: {0}")]
    FetchTopicMetadataError(rdkafka::error::KafkaError),
    #[error("get connection from database pool error: {0}")]
    GetDbPoolConnectionError(bb8_postgres::bb8::RunError<bb8_postgres::tokio_postgres::Error>),
    #[error("{0}{}", .1.as_ref().map(|msg| format!(": {}", msg)).unwrap_or_default())]
    Invariant(Cow<'static, str>, Option<Cow<'static, str>>),
    #[error(transparent)]
    MailboxError(#[from] actix::MailboxError),
    #[error("message key utf8 error: {0}")]
    MessageKeyUtf8Error(std::str::Utf8Error),
    #[error("failed to decode message json: {0}")]
    MessageJsonDeserializeError(serde_json::Error),
    #[error("message key missing")]
    MessageKeyMissing,
    #[error("missing actix system")]
    MissingActixSystem,
    #[cfg(feature = "outbox-relay")]
    #[error(transparent)]
    OutboxRelayError(#[from] outbox_relay::error::Error),
    #[error("could not parse identity")]
    ParseIdentity,
    #[error("receive message error: {0}")]
    RecieveMessageError(rdkafka::error::KafkaError),
    #[error("resource not found{}", .0.as_ref().map(|msg| format!(": {}", msg.as_ref())).unwrap_or_default())]
    ResourceNotFound(Option<Cow<'static, str>>),
    #[error("send shutdown notification error: {0}")]
    SendShutdownNotificationError(#[from] tokio::sync::watch::error::SendError<()>),
    #[error("serialize event error: {0}")]
    SerializeEventError(serde_json::Error),
    #[error("subscribe topic error: {0}")]
    SubscribeTopicError(rdkafka::error::KafkaError),
    #[error(transparent)]
    TokioJoinError(#[from] tokio::task::JoinError),
}

impl Error {
    /// Invariant error code and message.
    ///
    /// Returns the [Error::Invariant] variant.
    ///
    /// Typically used in aggregate command handlers to indicate
    /// the failure of a command due to business rules.
    pub fn invariant<C, M>(code: C, msg: M) -> Self
    where
        C: Into<Cow<'static, str>>,
        M: Into<Cow<'static, str>>,
    {
        Self::Invariant(code.into(), Some(msg.into()))
    }

    /// Invariant error code only.
    ///
    /// Returns the [Error::Invariant] variant.
    ///
    /// Typically used in aggregate command handlers to indicate
    /// the failure of a command due to business rules.
    pub fn invariant_code<C>(code: C) -> Self
    where
        C: Into<Cow<'static, str>>,
    {
        Self::Invariant(code.into(), None)
    }

    /// Resource not found.
    ///
    /// Returns the [Error::ResourceNotFound] variant.
    pub fn not_found<M>(msg: Option<M>) -> Self
    where
        M: Into<Cow<'static, str>>,
    {
        Self::ResourceNotFound(msg.map(|m| m.into()))
    }

    /// Recommended log level for the current error.
    pub fn level(&self) -> LevelFilter {
        use Error::*;

        match self {
            AggregateChannelNotInitialised(_) => LevelFilter::ERROR,
            CreateConsumerError(_) => LevelFilter::ERROR,
            DeserializeDbEventError(_, _) => LevelFilter::ERROR,
            DbError(_) => LevelFilter::ERROR,
            EventAlreadyHandled(_) => LevelFilter::DEBUG,
            EventMissing(_) => LevelFilter::ERROR,
            FetchTopicMetadataError(_) => LevelFilter::ERROR,
            GetDbPoolConnectionError(_) => LevelFilter::ERROR,
            Invariant(_, _) => LevelFilter::WARN,
            MailboxError(_) => LevelFilter::ERROR,
            MessageKeyUtf8Error(_) => LevelFilter::ERROR,
            MessageJsonDeserializeError(_) => LevelFilter::WARN,
            MessageKeyMissing => LevelFilter::WARN,
            MissingActixSystem => LevelFilter::ERROR,
            #[cfg(feature = "outbox-relay")]
            OutboxRelayError(_) => LevelFilter::ERROR,
            ParseIdentity => LevelFilter::ERROR,
            RecieveMessageError(_) => LevelFilter::ERROR,
            ResourceNotFound(_) => LevelFilter::WARN,
            SendShutdownNotificationError(_) => LevelFilter::WARN,
            SerializeEventError(_) => LevelFilter::ERROR,
            SubscribeTopicError(_) => LevelFilter::ERROR,
            TokioJoinError(_) => LevelFilter::ERROR,
        }
    }

    /// Log the error based on the recommended level.
    pub fn log(&self) {
        use tracing::{debug, error, info, trace, warn};

        let level = self.level();
        if level == Level::ERROR {
            error!(%self);
        } else if level == Level::WARN {
            warn!(%self);
        } else if level == Level::INFO {
            info!(%self);
        } else if level == Level::DEBUG {
            debug!(%self);
        } else if level == Level::TRACE {
            trace!(%self);
        }
    }
}
