use std::convert::Into;

use anyhow::anyhow;
use async_trait::async_trait;
use thalo::{Category, ID};
use thalo_message_store::GenericMessage;
use tonic::{codegen::*, transport::Channel, Request};

use super::proto;
pub use super::proto::command_center_client::*;
pub use super::proto::event_subscription_client::*;

#[async_trait(?Send)]
pub trait CommandCenterClientExt {
    async fn execute(
        &mut self,
        name: Category<'static>,
        id: ID<'static>,
        command: String,
        payload: serde_json::Value,
    ) -> anyhow::Result<Vec<GenericMessage>>;
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
            payload: Some(proto::json_value_to_prost_value(payload)?),
        });
        let resp = CommandCenterClient::execute(self, req).await?.into_inner();
        dbg!(&resp);
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
}
