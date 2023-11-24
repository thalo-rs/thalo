use std::borrow::Cow;
use std::marker::PhantomData;
use std::time::{Duration, UNIX_EPOCH};

use thalo::stream_name::{Category, EmptyStreamName, StreamName};
use thiserror::Error;

tonic::include_proto!("thalo");

impl<T> TryFrom<thalo_message_store::message::Message<'static, T>> for Message {
    type Error = serde_json::Error;

    fn try_from(
        msg: thalo_message_store::message::Message<'static, T>,
    ) -> Result<Self, Self::Error> {
        Ok(Message {
            id: msg.id,
            global_id: msg.global_id,
            position: msg.position,
            stream_name: msg.stream_name.into_string(),
            msg_type: msg.msg_type.into_owned(),
            data: serde_json::to_string(&msg.data)?,
            time: msg
                .time
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        })
    }
}

#[derive(Debug, Error)]
pub enum TryFromMessageError {
    #[error("failed to deserialize data: {0}")]
    DeserializeData(#[from] serde_json::Error),
    #[error("invalid stream name")]
    InvalidStreamName,
}

impl<T> TryFrom<Message> for thalo_message_store::message::Message<'static, T> {
    type Error = TryFromMessageError;

    fn try_from(msg: Message) -> Result<Self, Self::Error> {
        Ok(thalo_message_store::message::Message {
            id: msg.id,
            global_id: msg.global_id,
            position: msg.position,
            stream_name: StreamName::new(msg.stream_name)
                .map_err(|_| TryFromMessageError::InvalidStreamName)?,
            msg_type: Cow::Owned(msg.msg_type),
            data: serde_json::from_str(&msg.data)?,
            time: UNIX_EPOCH + Duration::from_millis(msg.time),
            _marker: PhantomData,
        })
    }
}

impl TryFrom<EventInterest> for crate::projection::EventInterest<'static> {
    type Error = EmptyStreamName;

    fn try_from(event_interest: EventInterest) -> Result<Self, Self::Error> {
        let category = if event_interest.category == "*" {
            crate::projection::CategoryInterest::Any
        } else {
            crate::projection::CategoryInterest::Category(Category::new(event_interest.category)?)
        };
        Ok(crate::projection::EventInterest {
            category,
            event: event_interest.event,
        })
    }
}
