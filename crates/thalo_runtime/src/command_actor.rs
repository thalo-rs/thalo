use std::collections::HashMap;

use async_trait::async_trait;
use moka::future::Cache;
use ractor::{Actor, ActorProcessingErr, ActorRef};
use serde_json::Value;
use thalo::{Category, StreamName};

use crate::runtime::Runtime;

pub struct CommandRouter;

pub enum CommandRouterMessage {
    PrintHelloWorld,
}

#[async_trait]
impl Actor for CommandRouter {
    type State = HashMap<Category<'static>, ActorRef<()>>;
    type Msg = CommandRouterMessage;
    type Arguments = ();

    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        _arguments: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        Ok(Cache::builder().max_capacity())
    }
}

pub struct StreamCommandHandler;

pub struct StreamCommandHandlerState {
    runtime: Runtime,
}

pub enum StreamCommandHandlerMessage {
    ExecuteCommand {
        stream_name: StreamName<'static>,
        command_name: String,
        payload: Value,
    },
}

#[async_trait]
impl Actor for StreamCommandHandler {
    type State = StreamCommandHandlerState;
    type Msg = StreamCommandHandlerMessage;
    type Arguments = ();

    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        _arguments: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        Ok(Cache::builder().max_capacity())
    }
}
