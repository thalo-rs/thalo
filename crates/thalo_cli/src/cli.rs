//! This example demonstrates an HTTP client that requests files from a server.
//!
//! Checkout the `README.md` for guidance.

mod build;
mod execute;
mod publish;

use anyhow::Result;
use clap::{Parser, Subcommand};

use self::build::Build;
use self::execute::Execute;
use self::publish::Publish;

/// Thalo client
#[derive(Parser, Debug)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Clone, Debug)]
enum Command {
    Build(Build),
    Execute(Execute),
    Publish(Publish),
}

pub async fn run() -> Result<()> {
    let cli = Cli::try_parse()?;

    match cli.command {
        Command::Build(cmd) => {
            cmd.build().await?;
        }
        Command::Execute(cmd) => {
            cmd.execute().await?;
        }
        Command::Publish(cmd) => {
            cmd.publish().await?;
        }
    }

    Ok(())
}
