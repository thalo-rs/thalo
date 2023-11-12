use std::time::Duration;

use anyhow::{anyhow, Result};
use clap::Args;
use quinn::{RecvStream, SendStream};
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
    payload: String,
    /// Timeout in milliseconds
    timeout: Option<u64>,
}

impl Execute {
    pub async fn execute(self, send: &mut SendStream, recv: &mut RecvStream) -> Result<()> {
        let request = Request::Execute {
            name: self.name,
            id: self.id,
            command: self.command,
            payload: self.payload,
            timeout: self.timeout.map(|timeout| Duration::from_millis(timeout)),
        };
        let mut request = pack(&request)?;

        send.write_all_chunks(&mut request)
            .await
            .map_err(|e| anyhow!("failed to send request: {}", e))?;

        handle_response(recv).await?;

        Ok(())
    }
}
