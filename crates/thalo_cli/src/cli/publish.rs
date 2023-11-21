use std::path::PathBuf;

use anyhow::Result;
use clap::Args;
use thalo::stream_name::Category;
use thalo_runtime::rpc::client::*;
use tokio::fs;

/// Publish a schema and module
#[derive(Args, Clone, Debug)]
pub struct Publish {
    /// Url of thalo runtime
    #[clap(short, long, default_value = "http://localhost:4433")]
    url: String,
    /// Module name
    name: String,
    /// Path to wasm module
    module: PathBuf,
}

impl Publish {
    pub async fn publish(self) -> Result<()> {
        let name = Category::new(self.name)?;
        let module_bytes = fs::read(self.module).await?;

        let mut client = CommandCenterClient::connect(self.url).await?;
        CommandCenterClientExt::publish(&mut client, name, module_bytes).await?;

        println!("Module published");

        Ok(())
    }
}
