use std::pin::Pin;

use futures::StreamExt as _;
use thalo::stream_name::{Category, ID};
use thalo_message_store::message::Message;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::{Stream, StreamExt};
use tonic::{Request, Response, Status};

use super::proto;
pub use super::proto::command_center_server::*;
pub use super::proto::projection_server::*;
use crate::Runtime;

#[tonic::async_trait]
impl proto::command_center_server::CommandCenter for Runtime {
    async fn execute(
        &self,
        request: Request<proto::ExecuteCommand>,
    ) -> Result<Response<proto::ExecuteResponse>, Status> {
        let proto::ExecuteCommand {
            name,
            id,
            command,
            payload,
        } = request.into_inner();
        let name = Category::new(name).map_err(|_| Status::invalid_argument("invalid name"))?;
        let id = ID::new(id).map_err(|_| Status::invalid_argument("invalid id"))?;
        let payload = serde_json::from_str(&payload)
            .map_err(|err| Status::invalid_argument(format!("invalid payload: {err}")))?;

        let resp = match self.execute(name, id, command, payload).await {
            Ok(Ok(events)) => proto::ExecuteResponse {
                success: true,
                events: events
                    .into_iter()
                    .map(proto::Message::try_from)
                    .collect::<Result<_, _>>()
                    .map_err(|err| Status::internal(err.to_string()))?,
                message: "ok".to_string(),
            },
            Ok(Err(err)) => proto::ExecuteResponse {
                success: false,
                events: vec![],
                message: serde_json::to_string(&err)
                    .map_err(|err| Status::internal(format!("failed to serialize error: {err}")))?,
            },
            Err(err) => return Err(Status::internal(err.to_string())),
        };

        Ok(Response::new(resp))
    }

    async fn publish(
        &self,
        request: Request<proto::PublishModule>,
    ) -> Result<Response<proto::PublishResponse>, Status> {
        let proto::PublishModule { name, module } = request.into_inner();
        let name = Category::new(name).map_err(|_| Status::invalid_argument("invalid name"))?;

        let resp = match self.save_module(name, module).await {
            Ok(()) => proto::PublishResponse {
                success: true,
                message: "ok".to_string(),
            },
            Err(err) => proto::PublishResponse {
                success: false,
                message: err.to_string(),
            },
        };

        Ok(Response::new(resp))
    }
}

#[tonic::async_trait]
impl proto::projection_server::Projection for Runtime {
    type SubscribeToEventsStream =
        Pin<Box<dyn Stream<Item = Result<proto::Message, Status>> + Send + 'static>>;

    async fn subscribe_to_events(
        &self,
        request: Request<proto::SubscriptionRequest>,
    ) -> Result<Response<Self::SubscribeToEventsStream>, Status> {
        let proto::SubscriptionRequest { name, events } = request.into_inner();

        let (tx, rx) = mpsc::channel::<Message>(1);
        let events = events
            .into_iter()
            .map(crate::projection::EventInterest::try_from)
            .collect::<Result<_, _>>()
            .map_err(|err| Status::invalid_argument(err.to_string()))?;
        self.start_projection(tx, name, events)
            .await
            .map_err(|err| Status::internal(err.to_string()))?;

        let resp = StreamExt::map(ReceiverStream::new(rx), |msg| {
            proto::Message::try_from(msg).map_err(|err| Status::internal(err.to_string()))
        })
        .boxed();

        Ok(Response::new(resp))
    }

    async fn acknowledge_event(
        &self,
        request: Request<proto::Acknowledgement>,
    ) -> Result<Response<proto::AckResponse>, Status> {
        let proto::Acknowledgement { name, global_id } = request.into_inner();

        self.acknowledge_event(name, global_id)
            .await
            .map_err(|err| Status::internal(err.to_string()))?;

        let resp = proto::AckResponse {
            success: true,
            message: "ok".to_string(),
        };

        Ok(Response::new(resp))
    }
}
