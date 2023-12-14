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

use chrono::serde::ts_milliseconds;
use chrono::{DateTime, Utc};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use crate::stream_name::StreamName;

/// A message used with the message store, containing data `T`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Message<'a, T = serde_json::Value> {
    /// Stream name.
    pub stream_name: StreamName<'a>,
    /// An incrementing gapless squence in the stream.
    pub stream_sequence: u64,
    /// An incrementing gapless squence the entire event log.
    pub global_sequence: u64,
    /// Unique identifier.
    pub id: Uuid,
    /// Event type.
    pub event_type: Cow<'a, str>,
    /// Message data.
    pub data: Cow<'a, serde_json::Value>,
    /// Time message was saved to the message store.
    #[serde(with = "ts_milliseconds")]
    pub timestamp: DateTime<Utc>,
    /// Marker type for the event.
    #[serde(skip)]
    pub _marker: PhantomData<T>,
}

impl<'a, T> Message<'a, T> {
    pub fn new(
        stream_name: StreamName<'a>,
        stream_sequence: u64,
        global_sequence: u64,
        event_type: Cow<'a, str>,
        event_data: Cow<'a, serde_json::Value>,
    ) -> Self {
        Message {
            stream_name,
            stream_sequence,
            global_sequence,
            id: Uuid::new_v4(),
            event_type,
            data: event_data,
            timestamp: Utc::now(),
            _marker: PhantomData,
        }
    }

    pub fn event(&self) -> Result<T, serde_json::Error>
    where
        T: DeserializeOwned,
    {
        let event_type = self.event_type.clone().into_owned();
        serde_json::from_value(json!({ event_type: self.data }))
    }

    pub fn into_event(self) -> Result<T, serde_json::Error>
    where
        T: DeserializeOwned,
    {
        let event_type = self.event_type.into_owned();
        serde_json::from_value(json!({ event_type: self.data }))
    }

    pub fn as_event_type<U>(self) -> Message<'a, U> {
        Message {
            stream_name: self.stream_name,
            stream_sequence: self.stream_sequence,
            global_sequence: self.global_sequence,
            id: self.id,
            event_type: self.event_type,
            data: self.data,
            timestamp: self.timestamp,
            _marker: PhantomData,
        }
    }

    pub fn into_owned(self) -> Message<'static, T> {
        Message {
            stream_name: self.stream_name.into_owned(),
            stream_sequence: self.stream_sequence,
            global_sequence: self.global_sequence,
            id: self.id,
            event_type: Cow::Owned(self.event_type.into_owned()),
            data: Cow::Owned(self.data.into_owned()),
            timestamp: self.timestamp,
            _marker: self._marker,
        }
    }
}
