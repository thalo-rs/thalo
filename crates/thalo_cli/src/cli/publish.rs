use std::path::PathBuf;

use anyhow::{anyhow, Result};
use clap::Args;
use futures::TryFutureExt;
use quinn::{RecvStream, SendStream};
use thalo_runtime::interface::message::{pack, pack_raw, Request};
use tokio::fs;

use super::handle_response;

/// Publish a schema and module
#[derive(Args, Clone, Debug)]
pub struct Publish {
    /// Path to ESDL schema
    schema: PathBuf,
    /// Path to wasm module
    module: PathBuf,
}

impl Publish {
    pub async fn publish(self, send: &mut SendStream, recv: &mut RecvStream) -> Result<()> {
        let schema_content = fs::read_to_string(self.schema).await?;
        let schema = esdl::parse(&schema_content)?;
        let schema_encoded = rmp_serde::to_vec(&schema)?;

        // Ensure module exists
        fs::metadata(&self.module).await?;

        let request = Request::Publish {};
        let mut request = pack(&request)?;
        send.write_all_chunks(&mut request)
            .await
            .map_err(|e| anyhow!("failed to send request: {}", e))?;

        let mut schema_pack = pack_raw(schema_encoded)?;
        let (_, module_bytes) = tokio::try_join!(
            send.write_all_chunks(&mut schema_pack)
                .map_err(anyhow::Error::from),
            fs::read(self.module).map_err(anyhow::Error::from)
        )?;

        let mut module_pack = pack_raw(module_bytes)?;
        send.write_all_chunks(&mut module_pack).await?;

        handle_response(recv).await?;

        Ok(())
    }
}
