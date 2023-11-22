use std::borrow::Cow;
use std::{fmt, str};

use heck::ToLowerCamelCase;
use serde::{Deserialize, Serialize};

use super::EmptyStreamName;

/// A stream category containing an entity name, and optionally category types.
///
/// # Examples
///
/// `account`
///
/// Account entity with no category types.
///
/// `account:command`
///
/// Commands for the account entity.
///
/// `account:command+snapshot`
///
/// Commands and snapshots for the account entity.
#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Category<'a>(pub(crate) Cow<'a, str>);

impl<'a> Category<'a> {
    /// Category type separator.
    ///
    /// When a stream category contains a category type, it is separated by a
    /// colon (`:`) character.
    ///
    /// # Example
    ///
    /// `category:command`
    pub const CATEGORY_TYPE_SEPARATOR: char = ':';

    /// Compound type separator.
    ///
    /// When one or more category types are present, they are separated by a
    /// plus (`+`) character.
    ///
    /// # Example
    ///
    /// `category:command+snapshot`
    pub const COMPOUNT_TYPE_SEPARATOR: char = '+';

    /// Create a new Category from a string slice, validating its format.
    pub fn new(category: impl Into<Cow<'a, str>>) -> Result<Self, EmptyStreamName> {
        let category = category.into();

        // TODO: Validate
        // if Self::is_valid_format(&id_cow) {
        Ok(Category(category))
        // } else {
        //     Err("Invalid category format")
        // }
    }

    pub fn from_parts(
        entity_name: impl Into<String>,
        types: &[&str],
    ) -> Result<Self, EmptyStreamName> {
        let mut s = entity_name.into();
        if s.is_empty() {
            return Err(EmptyStreamName);
        }

        if !types.is_empty() {
            s.push(Self::CATEGORY_TYPE_SEPARATOR);
            for (i, ty) in types.into_iter().enumerate() {
                if i > 0 {
                    s.push(Self::COMPOUNT_TYPE_SEPARATOR);
                }
                s.push_str(ty);
            }
        }

        Ok(Category(Cow::Owned(s)))
    }

    pub fn into_static(self) -> Category<'static> {
        Category(Cow::Owned(self.0.into_owned()))
    }

    /// Normalizes a category into camelCase.
    ///
    /// # Example
    ///
    /// ```
    /// # use message_db::stream_name::Category;
    /// #
    /// let category = Category::normalize("Bank_Account");
    /// assert_eq!(category, "bankAccount");
    /// ```
    pub fn normalize(category: &str) -> String {
        category.to_lower_camel_case()
    }

    /// Category entity name.
    pub fn entity_name(&self) -> &str {
        self.split_once(Self::CATEGORY_TYPE_SEPARATOR)
            .map(|(name, _)| name)
            .unwrap_or(&self.0)
    }
}

impl_eq! { Category<'a>, &'b str }
impl_eq! { Category<'a>, String }
impl_as_ref_str! { Category, Category<'a>, Category<'static> }
