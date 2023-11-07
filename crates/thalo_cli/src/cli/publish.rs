use std::{path::PathBuf, time::Duration};

use anyhow::{anyhow, Result};
use clap::Args;
use quinn::{RecvStream, SendStream};
use thalo_runtime::interface::message::{pack, pack_raw, Request};
use tokio::fs;

use super::handle_response;

/// Publish a schema and module
#[derive(Args, Clone, Debug)]
pub struct Publish {
    /// Module name
    name: String,
    /// Path to wasm module
    module: PathBuf,
    /// Timeout in milliseconds
    timeout: Option<u64>,
}

impl Publish {
    pub async fn publish(self, send: &mut SendStream, recv: &mut RecvStream) -> Result<()> {
        // Ensure module exists
        fs::metadata(&self.module).await?;

        let request = Request::Publish {
            name: self.name,
            timeout: self.timeout.map(|timeout| Duration::from_millis(timeout)),
        };
        let mut request = pack(&request)?;
        send.write_all_chunks(&mut request)
            .await
            .map_err(|e| anyhow!("failed to send request: {}", e))?;

        let module_bytes = fs::read(self.module).await?;
        let mut module_pack = pack_raw(module_bytes)?;
        send.write_all_chunks(&mut module_pack).await?;

        handle_response(recv).await?;

        Ok(())
    }
}
