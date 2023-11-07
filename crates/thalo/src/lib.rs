#[macro_use]
mod macros;
mod metadata;
mod stream_name;

pub use metadata::*;
pub use stream_name::*;
pub use thalo_derive::*;

use std::{fmt, time};

use serde::{de::DeserializeOwned, Deserialize, Serialize};

pub trait Aggregate {
    type ID: From<String>;
    type Command: DeserializeOwned;
    type Events: Events;
    type Error: fmt::Display;

    fn init(id: Self::ID) -> Self;
    fn apply(&mut self, evt: <Self::Events as Events>::Event);
    fn handle(
        &self,
        cmd: Self::Command,
    ) -> Result<Vec<<Self::Events as Events>::Event>, Self::Error>;
}

pub trait Events {
    type Event: Serialize + DeserializeOwned;
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Context<'a> {
    pub id: u64,
    pub stream_name: StreamName<'a>,
    pub position: u64,
    pub metadata: Metadata<'a>,
    pub time: time::SystemTime,
}

impl Default for Context<'_> {
    fn default() -> Self {
        Context {
            id: 0,
            stream_name: StreamName::default(),
            position: 0,
            metadata: Metadata::default(),
            time: time::SystemTime::now(),
        }
    }
}

#[doc(hidden)]
pub mod __macro_helpers {
    pub use serde_json;
    pub use uuid;
    pub use wit_bindgen;
}
