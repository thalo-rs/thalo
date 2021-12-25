//! Actors

use actix::{Actor, Context};

/// TODO Docs
pub struct AggregateActor {}

impl Actor for AggregateActor {
    type Context = Context<Self>;
}
