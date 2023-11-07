use std::borrow::Cow;
use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::stream_name::StreamName;

/// A message's metadata object contains information about the stream where the
/// message resides, the previous message in a series of messages that make up a
/// messaging workflow, the originating process to which the message belongs, as
/// well as other data that are pertinent to understanding the provenance and
/// disposition of the message.
///
/// Where as a message's data represents information pertinent to the business
/// process that the message is involved with, a message's metadata contains
/// information that is mechanical and infrastructural. Message metadata is data
/// about messaging machinery, like message schema version, source stream,
/// positions, provenance, reply address, and the like.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Metadata<'a> {
    /// The name of the stream where the message resides.
    pub stream_name: StreamName<'a>,
    /// The sequential position of the message in its stream.
    pub position: u64,
    /// Name of a stream where a reply should be sent as a result of processing
    /// the message.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reply_stream_name: Option<StreamName<'a>>,
    // /// Timestamp that the message was written to the message store.
    // #[serde(with = "ts_milliseconds")]
    // pub time: Option<DateTime<Utc>>,
    /// Version identifier of the message schema itself.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema_version: Option<Cow<'a, str>>,
    /// Additional properties.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub properties: HashMap<Cow<'a, str>, Cow<'a, Value>>,
}

impl<'a> Metadata<'a> {
    pub fn as_borrowed(&self) -> Metadata<'_> {
        Metadata {
            stream_name: self.stream_name.as_borrowed(),
            position: self.position,
            reply_stream_name: self
                .reply_stream_name
                .as_ref()
                .map(|stream_name| stream_name.as_borrowed()),
            schema_version: self.schema_version.as_deref().map(|v| Cow::Borrowed(v)),
            properties: self
                .properties
                .iter()
                .map(|(k, v)| {
                    let k: &str = &k;
                    let v: &Value = &v;
                    (Cow::Borrowed(k), Cow::Borrowed(v))
                })
                .collect(),
        }
    }

    pub fn into_owned(self) -> Metadata<'static> {
        Metadata {
            stream_name: self.stream_name.into_owned(),
            position: self.position,
            reply_stream_name: self.reply_stream_name.map(StreamName::into_owned),
            schema_version: self.schema_version.map(|v| Cow::Owned(v.into_owned())),
            properties: self
                .properties
                .into_iter()
                .map(|(k, v)| (Cow::Owned(k.into_owned()), Cow::Owned(v.into_owned())))
                .collect(),
        }
    }

    /// The de facto unique identifier for a message is a combination of the
    /// message's stream name and the message's position number within that
    /// stream.
    ///
    /// Returns the identifier is formatted as a URI fragment of the form
    /// stream_name/position.
    pub fn identifier(&self) -> String {
        format!("{}/{}", self.stream_name, self.position)
    }

    /// Clears the reply stream name, setting it to None.
    pub fn clear_reply_stream_name(&mut self) {
        self.reply_stream_name = None;
    }

    /// Is a reply.
    pub fn is_reply(&self) -> bool {
        self.reply_stream_name.is_some()
    }
}
