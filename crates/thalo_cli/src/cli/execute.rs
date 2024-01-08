use anyhow::Result;
use clap::Args;
use serde_json::Value;
use thalo_runtime::rpc::client::*;

/// Execute a command for a given module
#[derive(Args, Clone, Debug)]
pub struct Execute {
    /// Url of thalo runtime
    #[clap(short, long, default_value = "http://localhost:4433")]
    url: String,
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
    pub async fn execute(self) -> Result<()> {
        let payload = serde_json::from_str(&self.payload)?;
        let mut client = CommandCenterClient::connect(self.url).await?;
        let res = CommandCenterClientExt::execute_anonymous_command::<Value>(
            &mut client,
            self.name,
            self.id,
            self.command,
            &payload,
        )
        .await;
        match res {
            Ok(Ok(events)) => {
                println!("Executed with {} events:", events.len());
                for event in &events {
                    println!("{}", serde_json::to_string_pretty(event).unwrap());
                }
            }
            Ok(Err(err)) => {
                let err = serde_json::to_string_pretty(&err)?;
                println!("Failed to execute command: {err}");
            }
            Err(err) => {
                println!("Failed to execute command with status {}:", err.code());
                println!("{}", err.message());
            }
        }

        Ok(())
    }
}
