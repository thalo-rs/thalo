//! A message is a data structure that represents either an instruction to be
//! passed to a process (command), or a record of something that has happened
//! (event) - typically in response to the processing of a command.
//!
//! # Events and commands are kinds of messages
//!
//! The only real difference between a command message and an event message is
//! the way that they are named. Command messages are named in the imperative
//! tense (eg: *DoSomething*) and event messages are named in the past tense
//! (eg: *SomethingDone*). Other kinds of messages in the Eventide toolkit
//! include entity snapshot messages and consumer position messages.
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
use std::marker::PhantomData;
use std::time::SystemTime;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::json;
use thalo::stream_name::StreamName;

/// A message used with the message store, containing data `T`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Message<'a, T = ()> {
    /// Unique monotonic identifier of the message.
    ///
    /// While this is monotonic, it may contain gaps.
    pub id: u64,
    /// An incrementing gapless squence the entire event log.
    pub global_id: u64,
    /// An incrementing gapless squence in the stream.
    pub position: u64,
    /// Stream name.
    pub stream_name: StreamName<'a>,
    /// Message type.
    ///
    /// For commands, this is typically the command name.
    /// For events, this is typically the event name.
    pub msg_type: Cow<'a, str>,
    /// Message data.
    pub data: Cow<'a, serde_json::Value>,
    /// Time message was saved to the message store.
    #[serde(with = "ts_milliseconds")]
    pub time: SystemTime,
    /// Marker type for the event.
    #[serde(skip)]
    pub _marker: PhantomData<T>,
}

impl<'a, T> Message<'a, T> {
    pub fn event(&self) -> Result<T, serde_json::Error>
    where
        T: DeserializeOwned,
    {
        let msg_type = self.msg_type.clone().into_owned();
        serde_json::from_value(json!({ msg_type: self.data }))
    }

    pub fn into_event(self) -> Result<T, serde_json::Error>
    where
        T: DeserializeOwned,
    {
        let msg_type = self.msg_type.into_owned();
        serde_json::from_value(json!({ msg_type: self.data }))
    }

    pub fn as_event_type<U>(self) -> Message<'a, U> {
        Message {
            id: self.id,
            global_id: self.global_id,
            position: self.position,
            stream_name: self.stream_name,
            msg_type: self.msg_type,
            data: self.data,
            time: self.time,
            _marker: PhantomData,
        }
    }

    pub fn into_owned(self) -> Message<'static, T> {
        Message {
            id: self.id,
            global_id: self.global_id,
            position: self.position,
            stream_name: self.stream_name.into_owned(),
            msg_type: Cow::Owned(self.msg_type.into_owned()),
            data: Cow::Owned(self.data.into_owned()),
            time: self.time,
            _marker: self._marker,
        }
    }
}

mod ts_milliseconds {
    use core::fmt;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    use serde::{de, ser};

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
