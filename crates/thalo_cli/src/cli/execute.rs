use std::str;

use anyhow::{anyhow, Result};
use clap::Args;
use quinn::{RecvStream, SendStream};
use serde_json::Value;
use thalo_runtime::interface::message::{pack, Request};
use uuid::Uuid;

use super::handle_response;

/// Execute a command for a given module
#[derive(Args, Clone, Debug)]
pub struct Execute {
    /// Name of aggregate
    name: String,
    /// ID of aggregate instance
    id: String,
    /// Command to execute
    #[clap(long)]
    command_id: Option<Uuid>,
    /// Command to execute
    command: String,
    /// Command data in JSON
    data: Payload,
}

#[derive(Clone, Debug)]
struct Payload(Vec<u8>);

impl Execute {
    pub async fn execute(self, send: &mut SendStream, recv: &mut RecvStream) -> Result<()> {
        let request = Request::Execute {
            name: self.name,
            id: self.id,
            command: self.command,
            data: self.data.0,
        };
        let mut request = pack(&request)?;

        send.write_all_chunks(&mut request)
            .await
            .map_err(|e| anyhow!("failed to send request: {}", e))?;

        handle_response(recv).await?;

        Ok(())
    }
}

impl str::FromStr for Payload {
    type Err = anyhow::Error;

    fn from_str(payload: &str) -> Result<Self, Self::Err> {
        let payload_json: Value = serde_json::from_str(payload)?;
        Ok(Payload(serde_json::to_vec(&payload_json)?))
    }
}
