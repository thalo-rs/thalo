use std::convert::Into;

use anyhow::anyhow;
use async_trait::async_trait;
use thalo::stream_name::{Category, ID};
use thalo_message_store::message::GenericMessage;
use tonic::codegen::*;
use tonic::Request;

use super::proto;
pub use super::proto::command_center_client::*;
pub use super::proto::projection_client::*;

#[async_trait(?Send)]
pub trait CommandCenterClientExt {
    async fn execute(
        &mut self,
        name: Category<'static>,
        id: ID<'static>,
        command: String,
        payload: serde_json::Value,
    ) -> anyhow::Result<Vec<GenericMessage>>;

    async fn publish(&mut self, name: Category<'static>, module: Vec<u8>) -> anyhow::Result<()>;
}

#[async_trait(?Send)]
impl<T> CommandCenterClientExt for CommandCenterClient<T>
where
    T: tonic::client::GrpcService<tonic::body::BoxBody>,
    T::Error: Into<StdError>,
    T::ResponseBody: Body<Data = tonic::codegen::Bytes> + Send + 'static,
    <T::ResponseBody as Body>::Error: Into<StdError> + Send,
{
    async fn execute(
        &mut self,
        name: Category<'static>,
        id: ID<'static>,
        command: String,
        payload: serde_json::Value,
    ) -> anyhow::Result<Vec<GenericMessage>> {
        let req = Request::new(proto::ExecuteCommand {
            name: name.into_string(),
            id: id.into_string(),
            command,
            payload: serde_json::to_string(&payload)?,
        });
        let resp = CommandCenterClient::execute(self, req).await?.into_inner();
        if resp.success {
            let events = resp
                .events
                .into_iter()
                .map(GenericMessage::try_from)
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
