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
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Metadata<'a> {
    // pub id: u64,
    /// The name of the stream where the message resides.
    pub stream_name: StreamName<'a>,
    /// The sequential position of the message in its stream.
    pub position: u64,
    // pub causation_message_id: u64,
    /// The stream name of the message that precedes the message in a sequential
    /// [message flow](http://docs.eventide-project.org/user-guide/messages-and-message-data/messages.html#message-workflows).
    pub causation_message_stream_name: StreamName<'a>,
    /// The sequential position of the causation message in its stream.
    pub causation_message_position: u64,
    // /// Name of the stream that represents an encompassing business process that
    // /// coordinates the sub-process that the message is a part of.
    // #[serde(skip_serializing_if = "Option::is_none")]
    // pub correlation_stream_name: Option<StreamName<'a>>,
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
    // /// Additional local properties.
    // #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    // pub local_properties: HashMap<Cow<'a, str>, Cow<'a, Value>>,
}

impl<'a> Metadata<'a> {
    pub fn as_borrowed(&self) -> Metadata<'_> {
        Metadata {
            // id: self.id,
            stream_name: self.stream_name.as_borrowed(),
            position: self.position,
            // causation_message_id: self.causation_message_id,
            causation_message_stream_name: self.causation_message_stream_name.as_borrowed(),
            causation_message_position: self.causation_message_position,
            // correlation_stream_name: self
            //     .correlation_stream_name
            //     .as_ref()
            //     .map(|stream_name| stream_name.as_borrowed()),
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
            // local_properties: self
            //     .local_properties
            //     .iter()
            //     .map(|(k, v)| {
            //         let k: &str = &k;
            //         let v: &Value = &v;
            //         (Cow::Borrowed(k), Cow::Borrowed(v))
            //     })
            //     .collect(),
        }
    }

    pub fn into_owned(self) -> Metadata<'static> {
        Metadata {
            stream_name: self.stream_name.into_owned(),
            position: self.position,
            causation_message_stream_name: self.causation_message_stream_name.into_owned(),
            causation_message_position: self.causation_message_position,
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

    /// The unique identifier for a message's causation message is a combination
    /// of the causation message's stream name and the causation message's
    /// position number within that stream.
    ///
    /// Returns the identifier is formatted as a URI fragment of the form
    /// `causation_message_stream_name/causation_message_position`.
    pub fn causation_message_identifier(&self) -> String {
        format!(
            "{}/{}",
            self.causation_message_stream_name, self.causation_message_position
        )
    }

    /// When messages represent subsequent steps in a workflow, a subsequent
    /// message's metadata records elements of the preceding message's metadata.
    /// Each message in a workflow carries provenance data of the message that
    /// precedes it.
    ///
    /// The message's implementation of `follow` specifically manages the
    /// transfer of message data from the preceding message to the
    /// subsequent method, and then delegates to the metadata object to
    /// manage the transfer of message flow and provenance data between the
    /// two metadata objects.
    ///
    /// There are three metadata attributes that comprise the identifying
    /// information of a message's preceding message. They are collectively
    /// referred to as causation data.
    ///
    /// - `causation_message_stream_name`
    /// - `causation_message_position`
    /// - `causation_message_global_position`
    ///
    /// Each message's metadata in a workflow may also carry identifying
    /// information about the overall or coordinating workflow that the messages
    /// participates in. That identifying information is referred to as
    /// correlation data.
    ///
    /// - `correlation_stream_name`
    ///
    /// Additionally, a message's metadata may carry a *reply address*:
    ///
    /// - `reply_stream_name`
    pub fn follow<'b: 'a>(&'a mut self, preceding_metadata: Metadata<'b>) {
        self.causation_message_stream_name = preceding_metadata.stream_name;
        self.causation_message_position = preceding_metadata.position;

        // self.correlation_stream_name = preceding_metadata.correlation_stream_name;

        self.reply_stream_name = preceding_metadata.reply_stream_name;

        self.properties.extend(preceding_metadata.properties);
    }

    /// Metadata objects can be determined to follow each other using the
    /// metadata's follows? predicate method.
    ///
    /// Returns `true` when the metadata's causation and provenance attributes
    /// match the metadata argument's message source attributes.
    pub fn follows(&self, preceding_metadata: &Metadata) -> bool {
        if self.causation_message_stream_name != preceding_metadata.stream_name {
            return false;
        }

        if self.causation_message_position != preceding_metadata.position {
            return false;
        }

        // if preceding_metadata.correlation_stream_name.is_some()
        //     && self.correlation_stream_name != preceding_metadata.correlation_stream_name
        // {
        //     return false;
        // }

        if preceding_metadata.reply_stream_name.is_some()
            && self.reply_stream_name != preceding_metadata.reply_stream_name
        {
            return false;
        }

        true
    }

    /// Clears the reply stream name, setting it to None.
    pub fn clear_reply_stream_name(&mut self) {
        self.reply_stream_name = None;
    }

    /// Is a reply.
    pub fn is_reply(&self) -> bool {
        self.reply_stream_name.is_some()
    }

    // / Is correlated with another stream name.
    // pub fn is_correlated(&self, stream_name: &str) -> bool {
    //     let Some(correlation_stream_name) = &self.correlation_stream_name else {
    //         return false;
    //     };

    //     let stream_name = Category::normalize(stream_name);

    //     if StreamName::is_category(&stream_name) {
    //         correlation_stream_name.category() == stream_name
    //     } else {
    //         correlation_stream_name == &stream_name
    //     }
    // }
}

// impl TryFrom<Option<Value>> for Metadata<'_> {
//     type Error = serde_json::Error;

//     fn try_from(value: Option<Value>) -> Result<Self, Self::Error> {
//         match value {
//             Some(value) => serde_json::from_value(value),
//             None => Ok(Metadata::default()),
//         }
//     }
// }
