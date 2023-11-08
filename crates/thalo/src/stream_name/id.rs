use std::{borrow::Cow, fmt, str};

use serde::{Deserialize, Serialize};

use super::EmptyStreamName;

/// A stream ID or list of IDs.
///
/// # Examples
///
/// `account1`
///
/// A single stream ID.
///
/// `account1+account2`
///
/// A compound stream ID.
#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ID<'a>(pub(crate) Cow<'a, str>);

impl<'a> ID<'a> {
    /// Compound ID separator.
    ///
    /// When multiple IDs are present, they are separated by a plus (`+`)
    /// character.
    ///
    /// # Example
    ///
    /// `account1+account2`
    pub const COMPOUND_ID_SEPARATOR: char = '+';

    /// Creates a new ID from a string.
    ///
    /// The provided `id` string is split by `+` characters if present to create
    /// a compound ID.
    ///
    /// Returns an error if the ID is empty.
    pub fn new(id: impl Into<Cow<'a, str>>) -> Result<Self, EmptyStreamName> {
        let id = id.into();
        Ok(ID(id))
        // if id.contains(Self::COMPOUND_ID_SEPARATOR) {
        //     let ids: Vec<_> = id
        //         .split(Self::COMPOUND_ID_SEPARATOR)
        //         .map(|id| Cow::Borrowed(id))
        //         .collect();

        //     Self::new_compound(ids)
        // } else {
        //     if id.is_empty() {
        //         return Err(EmptyStreamName);
        //     }

        //     Ok(ID(vec![Cow::Borrowed(id)]))
        // }
    }

    // Returns the cardinal ID.
    //
    // This is the first ID. If there is only one ID present, that is the
    // cardinal ID.
    //
    // # Example
    //
    // ```
    // # use message_db::stream_name::ID;
    // #
    // # fn main() -> message_db::Result<()> {
    // let id = ID::new("account1+account2")?;
    // assert_eq!(id.cardinal_id(), "account1");
    // # Ok(())
    // # }
    pub fn cardinal_id(&self) -> &str {
        self.split_once(Self::COMPOUND_ID_SEPARATOR)
            .map(|(id, _)| id)
            .unwrap_or(&self.0)
    }
}

impl_eq! { ID<'a>, &'b str }
impl_eq! { ID<'a>, String }
impl_as_ref_str! { ID, ID<'a>, ID<'static> }
