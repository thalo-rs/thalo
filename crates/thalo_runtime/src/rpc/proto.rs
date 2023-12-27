use std::borrow::Cow;
use std::marker::PhantomData;
use std::time::{Duration, UNIX_EPOCH};

use chrono::{TimeZone, Utc};
use thalo::stream_name::{EmptyStreamName, StreamName};
use thiserror::Error;

tonic::include_proto!("thalo");

// impl<T> TryFrom<thalo::event_store::message::Message<'static, T>> for Message
// {     type Error = serde_json::Error;

//     fn try_from(
//         msg: thalo::event_store::message::Message<'static, T>,
//     ) -> Result<Self, Self::Error> {
//         Ok(Message {
//             stream_name: msg.stream_name.into_string(),
//             stream_sequence: msg.stream_sequence,
//             global_sequence: msg.global_sequence,
//             id: msg.id.to_string(),
//             event_type: msg.event_type.into_owned(),
//             data: serde_json::to_string(&msg.data)?,
//             timestamp: msg
//                 .timestamp
//                 .timestamp_millis()
//                 .try_into()
//                 .unwrap_or_default(),
//         })
//     }
// }

// #[derive(Debug, Error)]
// pub enum TryFromMessageError {
//     #[error("failed to deserialize data: {0}")]
//     DeserializeData(#[from] serde_json::Error),
//     #[error("invalid stream name")]
//     InvalidStreamName,
//     #[error("invalid uuid: {0}")]
//     ParseUuidError(#[from] uuid::Error),
// }

// impl<T> TryFrom<Message> for thalo::event_store::message::Message<'static, T>
// {     type Error = TryFromMessageError;

//     fn try_from(msg: Message) -> Result<Self, Self::Error> {
//         Ok(thalo::event_store::message::Message {
//             stream_name: StreamName::new(msg.stream_name)
//                 .map_err(|_| TryFromMessageError::InvalidStreamName)?,
//             stream_sequence: msg.stream_sequence,
//             global_sequence: msg.global_sequence,
//             id: msg.id.parse()?,
//             event_type: Cow::Owned(msg.event_type),
//             data: serde_json::from_str(&msg.data)?,
//             timestamp: Utc
//                 .timestamp_millis_opt(msg.timestamp as i64)
//                 .single()
//                 .unwrap_or_default(),
//             _marker: PhantomData,
//         })
//     }
// }

// impl TryFrom<EventInterest> for crate::projection::EventInterest<'static> {
//     type Error = EmptyStreamName;

//     fn try_from(event_interest: EventInterest) -> Result<Self, Self::Error> {
//         let category = if event_interest.category == "*" {
//             crate::projection::CategoryInterest::Any
//         } else {
//
// crate::projection::CategoryInterest::Category(Category::new(event_interest.
// category)?)         };
//         Ok(crate::projection::EventInterest {
//             category,
//             event: event_interest.event,
//         })
//     }
// }
