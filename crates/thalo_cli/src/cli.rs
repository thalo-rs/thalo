//! This example demonstrates an HTTP client that requests files from a server.
//!
//! Checkout the `README.md` for guidance.

mod build;
mod execute;
// mod publish;

use std::time::Instant;

use anyhow::Result;
use clap::{Parser, Subcommand};

use self::build::Build;
use self::execute::Execute;
// use self::publish::Publish;

/// Thalo cli
#[derive(Parser, Debug)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Clone, Debug)]
enum Command {
    #[clap(alias = "b")]
    Build(Build),
    Execute(Execute),
    // Publish(Publish),
}

pub async fn run() -> Result<()> {
    let cli = Cli::try_parse()?;

    match cli.command {
        Command::Build(cmd) => {
            cmd.build().await?;
        }
        Command::Execute(cmd) => {
            let start = Instant::now();
            for _ in 0..1_000 {
                cmd.clone().execute().await?;
            }

            println!("{}ms", start.elapsed().as_millis());
        } /* Command::Publish(cmd) => {
           *     cmd.publish().await?;
           * } */
    }

    Ok(())
}
