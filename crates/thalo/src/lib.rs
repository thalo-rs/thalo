#[macro_use]
mod macros;
mod metadata;
mod stream_name;

pub use metadata::*;
pub use stream_name::*;

use std::{fmt, time};

use serde::{Deserialize, Serialize};

pub trait Aggregate {
    type ID: TryFrom<String>;
    type Command: for<'de> Deserialize<'de>;
    type Event: Serialize + for<'de> Deserialize<'de>;
    type Error: fmt::Display;

    fn init(id: Self::ID) -> Self;
    fn apply(&mut self, ctx: Context, evt: Self::Event);
    fn handle(&self, ctx: Context, cmd: Self::Command) -> Result<Vec<Self::Event>, Self::Error>;
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Context<'a> {
    pub id: u64,
    pub stream_name: StreamName<'a>,
    pub position: u64,
    pub metadata: Metadata<'a>,
    pub time: time::SystemTime,
}

impl Context<'_> {
    #[inline(always)]
    pub fn processed(&self, sequence: u64) -> bool {
        sequence >= self.position
    }
}

#[doc(hidden)]
pub mod __macro_helpers {
    pub use serde_json;
    pub use uuid;
    pub use wit_bindgen;
}
