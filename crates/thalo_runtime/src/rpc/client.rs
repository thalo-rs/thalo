use std::convert::Into;
use std::mem;

use anyhow::anyhow;
use async_trait::async_trait;
use serde::Serialize;
use thalo::stream_name::{Category, ID};
use thalo::Aggregate;
use thalo_message_store::message::Message;
use tonic::codegen::*;
use tonic::Request;

use super::proto;
pub use super::proto::command_center_client::*;
pub use super::proto::projection_client::*;

#[async_trait]
pub trait CommandCenterClientExt {
    async fn execute_anonymous_command(
        &mut self,
        name: Category<'static>,
        id: ID<'static>,
        cmd: String,
        payload: &serde_json::Value,
    ) -> anyhow::Result<Vec<Message>>;

    async fn execute<A, C>(
        &mut self,
        name: Category<'static>,
        id: ID<'static>,
        cmd: C,
    ) -> anyhow::Result<Vec<Message<A::Event>>>
    where
        A: Aggregate,
        A::Command: Serialize,
        C: Into<A::Command> + Send,
    {
        let cmd: A::Command = cmd.into();
        let cmd_value = serde_json::to_value(cmd)?;
        let (cmd, payload) = thalo::__macro_helpers::extract_event_name_payload(cmd_value)
            .map_err(|err| anyhow!("{err}"))?;
        let messages = Self::execute_anonymous_command(self, name, id, cmd, &payload).await?;
        Ok(unsafe { mem::transmute(messages) })
    }

    async fn publish(&mut self, name: Category<'static>, module: Vec<u8>) -> anyhow::Result<()>;
}

#[async_trait]
impl<T> CommandCenterClientExt for CommandCenterClient<T>
where
    T: tonic::client::GrpcService<tonic::body::BoxBody> + Send,
    <T as tonic::client::GrpcService<tonic::body::BoxBody>>::Future: Send,
    T::Error: Into<StdError>,
    T::ResponseBody: Body<Data = tonic::codegen::Bytes> + Send + 'static,
    <T::ResponseBody as Body>::Error: Into<StdError> + Send,
{
    async fn execute_anonymous_command(
        &mut self,
        name: Category<'static>,
        id: ID<'static>,
        cmd: String,
        payload: &serde_json::Value,
    ) -> anyhow::Result<Vec<Message>> {
        let payload = serde_json::to_string(&payload)?;

        let req = Request::new(proto::ExecuteCommand {
            name: name.into_string(),
            id: id.into_string(),
            command: cmd,
            payload,
        });
        let resp = CommandCenterClient::execute(self, req).await?.into_inner();
        if resp.success {
            let events = resp
                .events
                .into_iter()
                .map(Message::try_from)
                .collect::<Result<_, _>>()?;
            Ok(events)
        } else {
            Err(anyhow!("{}", resp.message))
        }
    }

    async fn publish(&mut self, name: Category<'static>, module: Vec<u8>) -> anyhow::Result<()> {
        let req = Request::new(proto::PublishModule {
            name: name.into_string(),
            module,
        });
        let resp = CommandCenterClient::publish(self, req).await?.into_inner();
        if resp.success {
            Ok(())
        } else {
            Err(anyhow!("{}", resp.message))
        }
    }
}
