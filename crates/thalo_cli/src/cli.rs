//! This example demonstrates an HTTP client that requests files from a server.
//!
//! Checkout the `README.md` for guidance.

mod execute;
mod publish;

use anyhow::Result;
use clap::{Parser, Subcommand};
use thalo_runtime::rpc::client::*;

use self::execute::Execute;
use self::publish::Publish;

/// Thalo client
#[derive(Parser, Debug)]
struct Cli {
    /// Url of thalo runtime
    #[clap(short, long, default_value = "http://0.0.0.0:4433")]
    url: String,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Clone, Debug)]
enum Command {
    Execute(Execute),
    Publish(Publish),
}

pub async fn run() -> Result<()> {
    let cli = Cli::try_parse()?;

    let client = CommandCenterClient::connect(cli.url).await?;

    match cli.command {
        Command::Execute(cmd) => {
            cmd.execute(client).await?;
        }
        Command::Publish(cmd) => {
            cmd.publish(client).await?;
        }
    }

    Ok(())
}
