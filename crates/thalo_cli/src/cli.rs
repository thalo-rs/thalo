//! This example demonstrates an HTTP client that requests files from a server.
//!
//! Checkout the `README.md` for guidance.

mod execute;
mod publish;

use std::net::ToSocketAddrs;
use std::path::PathBuf;
use std::sync::Arc;
use std::{fs, io};

use anyhow::{anyhow, Context, Result};
use clap::{Parser, Subcommand};
use quinn::RecvStream;
use thalo_runtime::interface::message::{receive, ExecutedResult, Response};
use thalo_runtime::interface::quic::ALPN_QUIC_HTTP;
use tracing::{error, info, trace};
use url::Url;

use self::execute::Execute;
use self::publish::Publish;

/// Thalo client
#[derive(Parser, Debug)]
struct Cli {
    /// Perform NSS-compatible TLS key logging to the file specified in
    /// `SSLKEYLOGFILE`
    #[clap(long)]
    keylog: bool,
    /// Url of host runtime
    #[clap(short, long)]
    url: Url,
    /// Override hostname used for certificate verification
    #[clap(long)]
    host: Option<String>,
    /// Custom certificate authority to trust, in DER format
    #[clap(long)]
    ca: Option<PathBuf>,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Clone, Debug)]
enum Commands {
    Execute(Execute),
    Publish(Publish),
}

pub async fn start() -> Result<()> {
    let cli = Cli::try_parse()?;

    let remote = (
        cli.url.host_str().context("missing host in url")?,
        cli.url.port().unwrap_or(4433),
    )
        .to_socket_addrs()?
        .next()
        .ok_or_else(|| anyhow!("couldn't resolve to an address"))?;

    let mut roots = rustls::RootCertStore::empty();
    if let Some(ca_path) = &cli.ca {
        roots.add(&rustls::Certificate(fs::read(ca_path)?))?;
    } else {
        let dirs = directories_next::ProjectDirs::from("", "thalo", "thalo").unwrap();
        match fs::read(dirs.data_local_dir().join("cert.der")) {
            Ok(cert) => {
                roots.add(&rustls::Certificate(cert))?;
            }
            Err(ref e) if e.kind() == io::ErrorKind::NotFound => {
                info!("local server certificate not found");
            }
            Err(e) => {
                error!("failed to open local server certificate: {}", e);
            }
        }
    }
    let mut client_crypto = rustls::ClientConfig::builder()
        .with_safe_defaults()
        .with_root_certificates(roots)
        .with_no_client_auth();

    client_crypto.alpn_protocols = ALPN_QUIC_HTTP.iter().map(|&x| x.into()).collect();
    if cli.keylog {
        client_crypto.key_log = Arc::new(rustls::KeyLogFile::new());
    }

    let mut endpoint = quinn::Endpoint::client("[::]:0".parse().unwrap())?;
    endpoint.set_default_client_config(quinn::ClientConfig::new(Arc::new(client_crypto)));

    let host = cli
        .host
        .as_ref()
        .map_or_else(|| cli.url.host_str(), |x| Some(x))
        .ok_or_else(|| anyhow!("no hostname specified"))?;

    trace!("connecting to {} at {}", host, remote);
    let conn = endpoint
        .connect(remote, host)?
        .await
        .map_err(|e| anyhow!("failed to connect: {}", e))?;
    info!("connected");

    let (mut send, mut recv) = conn
        .open_bi()
        .await
        .map_err(|e| anyhow!("failed to open stream: {}", e))?;

    match cli.command.clone() {
        Commands::Execute(execute) => {
            execute.clone().execute(&mut send, &mut recv).await?;
        }
        Commands::Publish(publish) => publish.publish(&mut send, &mut recv).await?,
    }

    let _ = send.finish().await;

    conn.close(0u32.into(), b"done");

    // Give the server a fair chance to receive the close packet
    endpoint.wait_idle().await;

    Ok(())
}

async fn handle_response(recv: &mut RecvStream) -> Result<()> {
    let resp_result: Result<Response, String> = receive(recv).await?;

    let resp = resp_result.map_err(|err| anyhow!("{err}"))?;
    match resp {
        Response::Executed(executed_result) => match executed_result {
            ExecutedResult::Events(events) => {
                println!("executed with {} events:", events.len());
                for event in &events {
                    println!("    {}  {}", event.msg_type, event.data);
                }
            }
            ExecutedResult::TimedOut => {
                println!("timed out");
            }
        },
        Response::Published => {
            println!("published");
        }
    }

    Ok(())
}
