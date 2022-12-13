use std::fmt;

use chrono::{DateTime, Utc};
use message_db::message::Metadata;
use message_db::stream_name::StreamName;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{Aggregate, Error};

pub trait Commands {
    type Aggregate: Aggregate;

    fn handle(
        state: &Self::Aggregate,
        ctx: &mut Context,
        command_name: &str,
        payload: Vec<u8>,
    ) -> Result<Vec<<Self::Aggregate as Aggregate>::Event>, Error>;
}

pub trait Command<C, E>
where
    Self: Aggregate,
    E: fmt::Display,
{
    fn handle(&self, ctx: &mut Context, command: C) -> Result<Vec<Self::Event>, E>;
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Context {
    pub id: Uuid,
    pub stream_name: StreamName,
    pub position: i64,
    pub global_position: i64,
    pub metadata: Metadata,
    pub time: DateTime<Utc>,
}

impl Context {
    pub fn processed(&self, sequence: i64) -> bool {
        sequence >= self.position
    }
}
