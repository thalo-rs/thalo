use std::path::PathBuf;

use anyhow::Result;
use clap::Args;
use thalo::stream_name::Category;
use thalo_runtime::rpc::client::*;
use tokio::fs;
use tonic::transport::Channel;

/// Publish a schema and module
#[derive(Args, Clone, Debug)]
pub struct Publish {
    /// Module name
    name: String,
    /// Path to wasm module
    module: PathBuf,
}

impl Publish {
    pub async fn publish(self, mut client: CommandCenterClient<Channel>) -> Result<()> {
        let name = Category::new(self.name)?;
        let module_bytes = fs::read(self.module).await?;

        CommandCenterClientExt::publish(&mut client, name, module_bytes).await?;

        println!("Module published");

        Ok(())
    }
}
