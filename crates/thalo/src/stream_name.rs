//! Stream names are strings containing a category, and optional entity
//! identifier.
//!
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

use std::borrow::Cow;
use std::{fmt, str};

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// A stream name containing a category, and optionally an ID.
#[derive(Clone, Debug, Default, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
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

    pub fn from_parts(category: String, id: Option<&str>) -> Result<Self, EmptyStreamName> {
        let mut s = category;
        if let Some(id) = id {
            s.push(Self::ID_SEPARATOR);
            s.push_str(id);
        }

        Ok(StreamName(Cow::Owned(s)))
    }

    pub fn category(&self) -> &str {
        self.split_once(Self::ID_SEPARATOR)
            .map(|(category, _)| category)
            .unwrap_or(self)
    }

    pub fn id(&self) -> Option<&str> {
        self.split_once(Self::ID_SEPARATOR).map(|(_, id)| id)
    }

    /// Returns whether a `stream_name` is a category.
    pub fn is_category(stream_name: &str) -> bool {
        stream_name.contains(Self::ID_SEPARATOR)
    }
}

#[derive(Clone, Copy, Debug, Error)]
#[error("empty stream name")]
pub struct EmptyStreamName;

impl_eq! { StreamName<'a>, &'b str }
impl_eq! { StreamName<'a>, String }
impl_as_ref_str! { StreamName, StreamName<'a>, StreamName<'static> }
