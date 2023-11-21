use anyhow::Result;
use clap::Args;
use thalo::stream_name::{Category, ID};
use thalo_runtime::rpc::client::*;
use tonic::transport::Channel;

/// Execute a command for a given module
#[derive(Args, Clone, Debug)]
pub struct Execute {
    /// Name of aggregate
    name: String,
    /// ID of aggregate instance
    id: String,
    /// Command to execute
    command: String,
    /// Command data in JSON
    payload: String,
}

impl Execute {
    pub async fn execute(self, mut client: CommandCenterClient<Channel>) -> Result<()> {
        let name = Category::new(self.name)?;
        let id = ID::new(self.id)?;
        let payload = serde_json::from_str(&self.payload)?;
        let events =
            CommandCenterClientExt::execute(&mut client, name, id, self.command, payload).await?;

        println!("Executed with {} events:", events.len());
        for event in &events {
            println!("    {}  {}", event.msg_type, event.data);
        }

        Ok(())
    }
}
