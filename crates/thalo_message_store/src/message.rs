//! A message is a data structure that represents either an instruction to be
//! passed to a process (command), or a record of something that has happened
//! (event) - typically in response to the processing of a command.
//!
//! # Messages are just data objects
//!
//! Messages are just plain data structures. They have attributes and that's it.
//! They don't (and should not) have methods that do anything but declare
//! attributes, and set and get attribute values. Messages do not validate
//! themselves, transform or serialize themselves, send themselves, or save
//! themselves. All of these capabilities are external capabilities to a
//! message, and therefore are not behaviors of a message.
//!
//! # Events and commands are kinds of messages
//!
//! The only real difference between a command message and an event message is
//! the way that they are named. Command messages are named in the imperative
//! tense (eg: *DoSomething*) and event messages are named in the past tense
//! (eg: *SomethingDone*). Other kinds of messages in the Eventide toolkit
//! include entity snapshot messages and consumer position messages.
//!
//! # Messages are serialized as JSON when stored
//!
//! Messages are serialized to JSON when they are written to the message store,
//! and deserialized when they are read from the message store.
//!
//! # Messages are typically flat key/value structures
//!
//! Messages are not typically hierarchical tree structures with a root object
//! and references to other objects or list of objects. They are not rich
//! entity/relational models. They're key/value objects with attributes that
//! hold primitive values. Messages themselves are primitive, and every effort
//! should be made to keep them primitive.
//!
//! # Message names do not include namespaces
//!
//! Only a message's class name is considered a message's name, even if the
//! class is nested in an outer namespace. When a message is written to the
//! message store, any outer namespace is not included in the message name.
//! While it's possible to have two message classes with the same name but in
//! different namespaces, once those messages are written to the store, the
//! distinctness provided by namespaces will be eliminated. If you need to
//! differentiate between classes that have the same name, the name of the
//! message class should include a prefix or suffix.

use std::borrow::Cow;
use std::time::SystemTime;

// use chrono::serde::ts_milliseconds;
// use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thalo::{Metadata, StreamName};

/// Generic message JSON data.
pub type MessageData = serde_json::Value;
/// A generic message with any JSON data.
pub type GenericMessage<'a> = Message<'a, MessageData>;

/// A message used with the message store, containing data `T`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Message<'a, T>
where
    T: Clone,
{
    /// Unique monotonic identifier of the message.
    ///
    /// While this is monotonic, it may contain gaps.
    pub id: u64,
    /// Stream name.
    pub stream_name: StreamName<'a>,
    /// Message type.
    ///
    /// For commands, this is typically the command name.
    /// For events, this is typically the event name.
    pub msg_type: Cow<'a, str>,
    /// An incrementing gapless squence in the stream.
    pub position: u64,
    /// Message data.
    pub data: Cow<'a, T>,
    /// Message metadata.
    pub metadata: Metadata<'a>,
    /// Time message was saved to the message store.
    #[serde(with = "ts_milliseconds")]
    pub time: SystemTime,
}

impl<'a, T> Message<'a, T>
where
    T: Clone,
{
    /// Maps a messages data from one type to another.
    ///
    /// # Example
    ///
    /// ```
    /// # use message_db::message::{Message, Metadata};
    /// # use chrono::Utc;
    /// # use uuid::Uuid;
    /// #
    /// # fn main() -> message_db::Result<()> {
    /// # let message = Message {
    /// #     id: Uuid::new_v4(),
    /// #     stream_name: "category-id".parse().unwrap(),
    /// #     msg_type: "foo".to_string(),
    /// #     position: 0,
    /// #     global_position: 0,
    /// #     data: Foo { num: 10 },
    /// #     metadata: Metadata::default(),
    /// #     time: Utc::now(),
    /// # };
    /// struct Foo { num: i32 }
    /// struct Bar { num: String }
    ///
    /// let message: Message<Bar> = message.map_data(|foo| Bar {
    ///     num: foo.num.to_string(),
    /// });
    /// # Ok(())
    /// # }
    /// ```
    pub fn map_data<'b, U, F>(self, f: F) -> Message<'b, U>
    where
        F: FnOnce(Cow<'a, T>) -> Cow<'b, U>,
        U: Clone,
        'a: 'b,
    {
        let new_data = f(self.data);
        Message {
            id: self.id,
            stream_name: self.stream_name,
            msg_type: self.msg_type,
            position: self.position,
            data: new_data,
            metadata: self.metadata,
            time: self.time,
        }
    }
}

// impl<'de> GenericMessage<'de> {
//     /// Deserializes message data into `T`, returning a new `Message<T>`.
//     pub fn deserialize_data<T>(self) -> Result<Message<'de, T>, serde_cbor::Error>
//     where
//         T: Clone + Deserialize<'de>,
//     {
//         let data = serde_cbor::from_slice(&self.data)?;
//         Ok(Message {
//             id: self.id,
//             stream_name: self.stream_name,
//             msg_type: self.msg_type,
//             position: self.position,
//             data,
//             metadata: self.metadata,
//             time: self.time,
//         })
//     }
// }

// pub(crate) trait DeserializeMessage<T>
// where
//     T: for<'de> Deserialize<'de>,
// {
//     type Output;

//     fn deserialize_messages(self) -> Result<Self::Output>;
// }

// impl<'de, T> DeserializeMessage<'de, T> for Option<GenericMessage>
// where
//     T: for<'de> Deserialize<'de>,
// {
//     type Output = Option<Message<T>>;

//     fn deserialize_messages(self) -> Result<Self::Output> {
//         self.map(|message| message.deserialize_data())
//             .transpose()
//             .map_err(Error::DeserializeData)
//     }
// }

// impl<T> DeserializeMessage<T> for Vec<GenericMessage>
// where
//     T: for<'de> Deserialize<'de>,
// {
//     type Output = Vec<Message<T>>;

//     fn deserialize_messages(self) -> Result<Self::Output> {
//         self.into_iter()
//             .map(|message| message.deserialize_data())
//             .collect::<Result<Vec<_>, _>>()
//             .map_err(Error::DeserializeData)
//     }
// }

mod ts_milliseconds {
    use core::fmt;
    use serde::{de, ser};
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    pub fn serialize<S>(dt: &SystemTime, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        serializer.serialize_i64(
            dt.duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as i64,
        )
    }

    pub fn deserialize<'de, D>(d: D) -> Result<SystemTime, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        d.deserialize_i64(MillisecondsVisitor)
    }

    struct MillisecondsVisitor;

    impl<'de> de::Visitor<'de> for MillisecondsVisitor {
        type Value = SystemTime;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a unix timestamp in milliseconds")
        }

        /// Deserialize a timestamp in milliseconds since the epoch
        fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(UNIX_EPOCH + Duration::from_millis(value as u64))
        }

        /// Deserialize a timestamp in milliseconds since the epoch
        fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(UNIX_EPOCH + Duration::from_millis(value))
        }
    }
}
