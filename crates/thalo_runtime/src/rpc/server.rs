use std::time::Duration;

use ractor::rpc::CallResult;
use thalo::{Category, ID};
use tonic::{Request, Response, Status};

use crate::Runtime;

use super::proto;
pub use super::proto::command_center_server::*;
pub use super::proto::event_subscription_server::*;

const DEFAULT_TIMEOUT_MS: u64 = 3_000;

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
        let payload = payload
            .map(proto::prost_value_to_json_value)
            .transpose()
            .map_err(|err: proto::InfiniteOrNaNNumberError| {
                Status::invalid_argument(format!("invalid payload: {err}"))
            })?
            .unwrap_or(serde_json::Value::Null);
        let timeout = Some(Duration::from_millis(DEFAULT_TIMEOUT_MS));

        let resp = match self.execute_wait(name, id, command, payload, timeout).await {
            Ok(CallResult::Success(Ok(events))) => proto::ExecuteResponse {
                success: true,
                events: events
                    .into_iter()
                    .map(proto::Message::try_from)
                    .collect::<Result<_, _>>()
                    .map_err(|err| Status::internal(err.to_string()))?,
                message: "ok".to_string(),
            },
            Ok(CallResult::Success(Err(err))) => proto::ExecuteResponse {
                success: false,
                events: vec![],
                message: err.to_string(),
            },
            Ok(CallResult::Timeout) => proto::ExecuteResponse {
                success: false,
                events: vec![],
                message: "timeout".to_string(),
            },
            Ok(CallResult::SenderError) => proto::ExecuteResponse {
                success: false,
                events: vec![],
                message: "sender error".to_string(),
            },
            Err(err) => proto::ExecuteResponse {
                success: false,
                events: vec![],
                message: err.to_string(),
            },
        };

        Ok(Response::new(resp))
    }

    async fn publish(
        &self,
        request: Request<proto::PublishModule>,
    ) -> Result<Response<proto::PublishResponse>, Status> {
        let proto::PublishModule { name, module } = request.into_inner();
        let name = Category::new(name).map_err(|_| Status::invalid_argument("invalid name"))?;
        let timeout = Some(Duration::from_millis(DEFAULT_TIMEOUT_MS));

        let resp = match self.save_module_wait(name, module, timeout).await {
            Ok(CallResult::Success(())) => proto::PublishResponse {
                success: true,
                message: "ok".to_string(),
            },
            Ok(CallResult::Timeout) => proto::PublishResponse {
                success: false,
                message: "timeout".to_string(),
            },
            Ok(CallResult::SenderError) => proto::PublishResponse {
                success: false,
                message: "sender error".to_string(),
            },
            Err(err) => proto::PublishResponse {
                success: false,
                message: err.to_string(),
            },
        };

        Ok(Response::new(resp))
    }
}
