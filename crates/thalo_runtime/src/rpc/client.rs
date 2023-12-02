use std::convert::Into;
use std::mem;

use async_trait::async_trait;
use proto::Acknowledgement;
use serde::de::DeserializeOwned;
use serde::Serialize;
use thalo::stream_name::{Category, ID};
use thalo::{Aggregate, Handle};
use thalo_message_store::message::Message;
use tonic::codegen::*;
use tonic::{Request, Status};

pub use super::proto::command_center_client::*;
pub use super::proto::projection_client::*;
use super::{proto, EventInterest, SubscriptionRequest};
use crate::projection::Projection;

#[async_trait]
pub trait CommandCenterClientExt {
    async fn execute_anonymous_command(
        &mut self,
        name: Category<'static>,
        id: ID<'static>,
        cmd: String,
        payload: &serde_json::Value,
    ) -> Result<Result<Vec<Message>, serde_json::Value>, Status>;

    async fn execute<A, C>(
        &mut self,
        name: Category<'static>,
        id: ID<'static>,
        cmd: C,
    ) -> Result<Result<Vec<Message<A::Event>>, <A as Handle<C>>::Error>, Status>
    where
        A: Aggregate,
        A::Command: Serialize,
        A: Handle<C>,
        <A as Handle<C>>::Error: DeserializeOwned,
        C: Into<A::Command> + Send,
    {
        let cmd: A::Command = cmd.into();
        let cmd_value = serde_json::to_value(cmd).map_err(|err| {
            Status::invalid_argument(format!("failed to serialize command: {err}"))
        })?;
        let (cmd, payload) = thalo::__macro_helpers::extract_event_name_payload(cmd_value)
            .map_err(|err| Status::invalid_argument(err.to_string()))?;
        match Self::execute_anonymous_command(self, name, id, cmd, &payload).await? {
            Ok(messages) => Ok(Ok(unsafe { mem::transmute(messages) })),
            Err(err) => {
                let err = serde_json::from_value(err).map_err(|err| {
                    Status::internal(format!(
                        "command failed, but the error was unable to be deserialized: {err}"
                    ))
                })?;
                Ok(Err(err))
            }
        }
    }

    async fn publish(&mut self, name: Category<'static>, module: Vec<u8>) -> Result<(), Status>;
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
    ) -> Result<Result<Vec<Message>, serde_json::Value>, Status> {
        let payload = serde_json::to_string(&payload).map_err(|err| {
            Status::invalid_argument(format!("failed to serialize payload: {err}"))
        })?;

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
                .collect::<Result<_, _>>()
                .map_err(|err| Status::internal(err.to_string()))?;
            Ok(Ok(events))
        } else {
            let err = serde_json::from_str(&resp.message)
                .map_err(|err| Status::internal(format!("failed to deserialize error: {err}")))?;
            Ok(Err(err))
        }
    }

    async fn publish(&mut self, name: Category<'static>, module: Vec<u8>) -> Result<(), Status> {
        let req = Request::new(proto::PublishModule {
            name: name.into_string(),
            module,
        });
        let resp = CommandCenterClient::publish(self, req).await?.into_inner();
        if resp.success {
            Ok(())
        } else {
            Err(Status::internal(resp.message))
        }
    }
}

#[async_trait]
pub trait ProjectionClientExt {
    async fn start_projection<P>(
        &mut self,
        name: &str,
        projection: P,
        events: Vec<EventInterest>,
    ) -> anyhow::Result<()>
    where
        P: Projection + Send;
}

#[async_trait]
impl<T> ProjectionClientExt for ProjectionClient<T>
where
    T: tonic::client::GrpcService<tonic::body::BoxBody> + Send,
    <T as tonic::client::GrpcService<tonic::body::BoxBody>>::Future: Send,
    T::Error: Into<StdError>,
    T::ResponseBody: Body<Data = tonic::codegen::Bytes> + Send + 'static,
    <T::ResponseBody as Body>::Error: Into<StdError> + Send,
{
    async fn start_projection<P>(
        &mut self,
        name: &str,
        mut projection: P,
        events: Vec<EventInterest>,
    ) -> anyhow::Result<()>
    where
        P: Projection + Send,
    {
        let mut streaming = self
            .subscribe_to_events(SubscriptionRequest {
                name: name.to_string(),
                events,
            })
            .await?
            .into_inner();

        while let Some(message) = streaming.message().await? {
            let global_id = message.global_id;

            if projection
                .last_global_id()
                .await?
                .map_or(false, |last_global_id| message.global_id <= last_global_id)
            {
                // Ignore, since we've already handled this event.
                // This logic keeps the projection idempotent, which is important since
                // projections have an at-least-once guarantee, meaning if a connection issue
                // occurs, we might reprocess event we've already seen.
                self.acknowledge_event(Acknowledgement {
                    name: name.to_string(),
                    global_id,
                })
                .await?;
                continue;
            }

            let message = message.try_into()?;
            projection.handle(message).await?;

            // Acknowledge we've handled this event
            self.acknowledge_event(Acknowledgement {
                name: name.to_string(),
                global_id,
            })
            .await?;
        }

        Ok(())
    }
}
