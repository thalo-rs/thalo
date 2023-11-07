//! Messages like events and commands are written to and read from streams. To
//! write and read from streams, the subject stream is identified by its name.
//!
//! A stream name not only identifies the stream, but also its purpose. A stream
//! name is a string that optionally includes an ID that is prefixed by a dash
//! (-) character, and may also include category *types* that indicate even
//! further specialized uses of the stream. The part of the stream preceding the
//! dash is the *category*, and the part following the dash is the ID.
//!
//! # Entity Stream Name
//!
//! An *entity* stream name contains all of the events for one specific entity.
//! For example, an `Account` entity with an ID of `123` would have the name,
//! `account-123`.
//!
//! # Category Stream Name
//!
//! A *category* stream name does not have an ID. For example, the stream name
//! for the category of all accounts is `account`.
//!
//! # Example Stream Names
//!
//! `account`
//!
//! Account category stream name. The name of the stream that has events for all
//! accounts.
//!
//! `account-123`
//!
//! Account entity stream name. The name of the stream that has events only for
//! the particular account with the ID 123.
//!
//! `account:command`
//!
//! Account command category stream name, or account command stream name for
//! short. This is a category stream name with a command type. This stream has
//! all commands for all accounts.
//!
//! `account:command-123`
//!
//! Account entity command stream name. This stream has all of the commands
//! specifically for the account with the ID 123.
//!
//! `account:command+position`
//!
//! Account command position category stream name. A consumer that is reading
//! commands from the account:command stream will periodically write the
//! position number of the last command processed to the position stream so that
//! all commands from all time do not have to be re-processed when the consumer
//! is restarted.
//!
//! `account:snapshot-123`
//!
//! Account entity snapshot stream name. Entity snapshots are periodically
//! recorded to disk as a performance optimization that eliminates the need to
//! project an event stream from its first-ever recorded event when entity is
//! not already in the in-memory cache.

mod category;
mod id;

use std::borrow::Cow;
use std::{fmt, str};

use serde::{Deserialize, Serialize};
use thiserror::Error;

pub use self::category::Category;
pub use self::id::ID;

/// A stream name containing a category, and optionally an ID.
#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct StreamName<'a>(Cow<'a, str>);

impl<'a> StreamName<'a> {
    /// ID separator.
    ///
    /// When a stream name contains an ID, it is separated by a
    /// colon (`-`) character.
    ///
    /// Only the first `-` is the separator, and all other `-` characters in an
    /// ID are valid.
    ///
    /// # Example
    ///
    /// `category-id`
    pub const ID_SEPARATOR: char = '-';

    pub fn new(stream_name: impl Into<Cow<'a, str>>) -> Result<Self, EmptyStreamName> {
        let stream_name = stream_name.into();

        // TODO: Valstream_nameate
        Ok(StreamName(stream_name))
    }

    pub fn from_parts(
        category: Category<'_>,
        id: Option<&ID<'_>>,
    ) -> Result<Self, EmptyStreamName> {
        let mut s = category.into_string();
        if let Some(id) = id {
            s.push(Self::ID_SEPARATOR);
            s.push_str(id);
        }

        Ok(StreamName(Cow::Owned(s)))
    }

    pub fn category(&self) -> Category<'_> {
        self.split_once(Self::ID_SEPARATOR)
            .map(|(category, _)| Category(Cow::Borrowed(category)))
            .unwrap_or(Category(Cow::Borrowed(self.as_ref())))
    }

    pub fn id(&self) -> Option<ID<'_>> {
        self.split_once(Self::ID_SEPARATOR)
            .map(|(_, id)| ID(Cow::Borrowed(id)))
    }

    /// Returns whether a `stream_name` is a category.
    pub fn is_category(stream_name: &str) -> bool {
        stream_name.contains(Self::ID_SEPARATOR)
    }

    // / Returns the category part of a `stream_name`.
    // pub fn category(stream_name: &str) -> &str {
    //     stream_name
    //         .split_once(Self::ID_SEPARATOR)
    //         .map(|(category, _)| category)
    //         .unwrap_or(stream_name)
    // }
}

impl_eq! { StreamName<'a>, &'b str }
impl_eq! { StreamName<'a>, String }
impl_as_ref_str! { StreamName, StreamName<'a>, StreamName<'static> }

// impl str::FromStr for StreamName<'_> {
//     type Err = EmptyStreamName;

//     fn from_str(s: &str) -> Result<Self, Self::Err> {
//         // category:type_a+type_b-id_a+id_b
//         match s.split_once(StreamName::ID_SEPARATOR) {
//             Some((category_part, id_part)) => {
//                 let category = category_part.parse()?;
//                 let id = id_part.parse()?;

//                 Ok(StreamName {
//                     category,
//                     id: Some(id),
//                 })
//             }
//             None => {
//                 let category = s.parse()?;

//                 Ok(StreamName { category, id: None })
//             }
//         }
//     }
// }

// impl Serialize for StreamName<'_> {
//     fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
//     where
//         S: serde::Serializer,
//     {
//         serializer.serialize_str(self)
//     }
// }

// impl<'de> Deserialize<'de> for StreamName<'de> {
//     fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
//     where
//         D: serde::Deserializer<'de>,
//     {
//         struct StreamNameVisitor;

//         impl<'de> Visitor<'de> for StreamNameVisitor {
//             type Value = StreamName<'de>;

//             fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
//                 formatter.write_str("StreamName")
//             }

//             fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
//             where
//                 E: de::Error,
//             {
//                 StreamName::new(v.to_string()).map_err(de::Error::custom)
//             }
//         }

//         deserializer.deserialize_str(StreamNameVisitor)
//     }
// }

#[derive(Clone, Copy, Debug, Error)]
#[error("empty stream name")]
pub struct EmptyStreamName;

// #[cfg(test)]
// mod tests {
//     use super::StreamName;

//     #[test]
//     fn it_works() {
//         let category = "category".parse().unwrap();
//         let id = "id".parse().unwrap();
//         let steam_name = StreamName {
//             category,
//             id: Some(id),
//         };
//         assert_eq!(steam_name.to_string(), "category-id");

//         let category = "category:type_a".parse().unwrap();
//         let id = "id".parse().unwrap();
//         let steam_name = StreamName {
//             category,
//             id: Some(id),
//         };
//         assert_eq!(steam_name.to_string(), "category:type_a-id");

//         let category = "category:type_a+type_b".parse().unwrap();
//         let id = "id".parse().unwrap();
//         let steam_name = StreamName {
//             category,
//             id: Some(id),
//         };
//         assert_eq!(steam_name.to_string(), "category:type_a+type_b-id");

//         let category = "category".parse().unwrap();
//         let id = "id_a+id_b".parse().unwrap();
//         let steam_name = StreamName {
//             category,
//             id: Some(id),
//         };
//         assert_eq!(steam_name.to_string(), "category-id_a+id_b");

//         let category = "category:type_a+type_b".parse().unwrap();
//         let id = "id_a+id_b".parse().unwrap();
//         let steam_name = StreamName {
//             category,
//             id: Some(id),
//         };
//         assert_eq!(steam_name.to_string(), "category:type_a+type_b-id_a+id_b");
//     }
// }
