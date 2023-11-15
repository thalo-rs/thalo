use std::io;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{anyhow, bail, Context, Result};
use futures::TryFutureExt;
use quinn::{RecvStream, SendStream};
use ractor::rpc::CallResult;
use rustls::PrivateKey;
use thalo::{Category, ID};
use tokio::fs;
use tracing::{error, info, trace, trace_span, Instrument};

use crate::interface::message::{pack, receive, receive_raw, Request, Response};
use crate::runtime::Runtime;

use super::message::ExecutedResult;

pub const ALPN_QUIC_HTTP: &[&[u8]] = &[b"hq-29"];

pub async fn run(
    certs: Vec<rustls::Certificate>,
    key: PrivateKey,
    keylog: bool,
    stateless_retry: bool,
    listen: SocketAddr,
    runtime: Runtime,
) -> Result<()> {
    let mut server_crypto = rustls::ServerConfig::builder()
        .with_safe_defaults()
        .with_no_client_auth()
        .with_single_cert(certs, key)?;
    server_crypto.alpn_protocols = ALPN_QUIC_HTTP.iter().map(|&x| x.into()).collect();
    if keylog {
        server_crypto.key_log = Arc::new(rustls::KeyLogFile::new());
    }

    let mut server_config = quinn::ServerConfig::with_crypto(Arc::new(server_crypto));
    Arc::get_mut(&mut server_config.transport)
        .unwrap()
        .max_concurrent_uni_streams(0_u8.into());
    if stateless_retry {
        server_config.use_retry(true);
    }

    let endpoint = quinn::Endpoint::server(server_config, listen)?;
    info!("listening on {}", endpoint.local_addr()?);

    while let Some(conn) = endpoint.accept().await {
        trace!("connection incoming");
        let fut = handle_connection(runtime.clone(), conn);
        tokio::spawn(async move {
            if let Err(e) = fut.await {
                error!("connection failed: {reason}", reason = e.to_string())
            }
        });
    }

    Ok(())
}

pub async fn load_certs(
    key: Option<PathBuf>,
    cert: Option<PathBuf>,
) -> Result<(Vec<rustls::Certificate>, PrivateKey)> {
    if let (Some(key_path), Some(cert_path)) = (key, cert) {
        let key = fs::read(&key_path)
            .await
            .context("failed to read private key")?;
        let key = if key_path.extension().map_or(false, |x| x == "der") {
            rustls::PrivateKey(key)
        } else {
            let pkcs8 = rustls_pemfile::pkcs8_private_keys(&mut &*key)
                .context("malformed PKCS #8 private key")?;
            match pkcs8.into_iter().next() {
                Some(x) => rustls::PrivateKey(x),
                None => {
                    let rsa = rustls_pemfile::rsa_private_keys(&mut &*key)
                        .context("malformed PKCS #1 private key")?;
                    match rsa.into_iter().next() {
                        Some(x) => rustls::PrivateKey(x),
                        None => {
                            anyhow::bail!("no private keys found");
                        }
                    }
                }
            }
        };
        let cert_chain = fs::read(&cert_path)
            .await
            .context("failed to read certificate chain")?;
        let cert_chain = if cert_path.extension().map_or(false, |x| x == "der") {
            vec![rustls::Certificate(cert_chain)]
        } else {
            rustls_pemfile::certs(&mut &*cert_chain)
                .context("invalid PEM-encoded certificate")?
                .into_iter()
                .map(rustls::Certificate)
                .collect()
        };

        Ok((cert_chain, key))
    } else {
        let dirs = directories_next::ProjectDirs::from("", "thalo", "thalo_runtime")
            .ok_or_else(|| anyhow!("failed to determine home directory"))?;
        let path = dirs.data_local_dir();
        let cert_path = path.join("cert.der");
        let key_path = path.join("key.der");
        let (cert, key) = match fs::read(&cert_path)
            .and_then(|x| {
                let key_path = key_path.clone();
                async move { Ok((x, fs::read(&key_path).await?)) }
            })
            .await
        {
            Ok(x) => x,
            Err(ref e) if e.kind() == io::ErrorKind::NotFound => {
                info!("generating self-signed certificate");
                let cert = rcgen::generate_simple_self_signed(vec!["localhost".into()])?;
                let key = cert.serialize_private_key_der();
                let cert = cert.serialize_der()?;
                fs::create_dir_all(path)
                    .await
                    .context("failed to create certificate directory")?;
                fs::write(&cert_path, &cert)
                    .await
                    .context("failed to write certificate")?;
                fs::write(&key_path, &key)
                    .await
                    .context("failed to write private key")?;
                (cert, key)
            }
            Err(e) => {
                bail!("failed to read certificate: {}", e);
            }
        };

        let key = rustls::PrivateKey(key);
        let cert = rustls::Certificate(cert);
        Ok((vec![cert], key))
    }
}

async fn handle_connection(runtime: Runtime, conn: quinn::Connecting) -> Result<()> {
    let connection = conn.await?;
    let span = trace_span!(
        "connection",
        remote = %connection.remote_address(),
        protocol = %connection
            .handshake_data()
            .unwrap()
            .downcast::<quinn::crypto::rustls::HandshakeData>().unwrap()
            .protocol
            .map_or_else(|| "<none>".into(), |x| String::from_utf8_lossy(&x).into_owned())
    );
    async {
        trace!("established connection");

        // Each stream initiated by the client constitutes a new request.
        loop {
            let stream = connection.accept_bi().await;
            let stream = match stream {
                Err(quinn::ConnectionError::ApplicationClosed { .. }) => {
                    trace!("connection closed");
                    return Ok(());
                }
                Err(e) => {
                    return Err(e);
                }
                Ok(s) => s,
            };
            let fut = handle_request(runtime.clone(), stream);
            tokio::spawn(
                async move {
                    if let Err(e) = fut.await {
                        error!("failed: {reason}", reason = e.to_string());
                    }
                }
                .instrument(trace_span!("request")),
            );
        }
    }
    .instrument(span)
    .await?;
    Ok(())
}

async fn handle_request(
    runtime: Runtime,
    (mut send, mut recv): (SendStream, RecvStream),
) -> Result<()> {
    let req: Request = receive(&mut recv).await?;
    let resp = match req {
        Request::Execute {
            name,
            id,
            command,
            payload,
            timeout,
        } => handle_execute(&runtime, name, id, command, payload, timeout).await,
        Request::Publish { name, timeout } => {
            handle_publish(&runtime, name, timeout, &mut recv).await
        }
    };

    let resp = resp.map_err(|err| {
        err.chain()
            .map(|err| err.to_string())
            .collect::<Vec<_>>()
            .join(" - ")
    });
    let mut reply = pack(&resp)?;
    send.write_all_chunks(&mut reply).await?;

    // Gracefully terminate the stream
    send.finish()
        .await
        .map_err(|e| anyhow!("failed to shutdown stream: {}", e))?;

    trace!("handled request");
    Ok(())
}

pub async fn handle_execute(
    runtime: &Runtime,
    name: String,
    id: String,
    command: String,
    payload: String,
    timeout: Option<Duration>,
) -> Result<Response> {
    let payload = serde_json::from_str(&payload).context("invalid command payload json")?;
    let result = runtime
        .execute_wait(
            Category::new(Category::normalize(&name))?,
            ID::new(id)?,
            command,
            payload,
            timeout,
        )
        .await?;

    match result {
        CallResult::Success(events) => {
            Ok(Response::Executed(ExecutedResult::Events(events.unwrap())))
        }
        CallResult::Timeout => Ok(Response::Executed(ExecutedResult::TimedOut)),
        CallResult::SenderError => Err(anyhow!("command failed to execute")),
    }
}

pub async fn handle_publish(
    runtime: &Runtime,
    name: String,
    timeout: Option<Duration>,
    recv: &mut RecvStream,
) -> Result<Response> {
    let name = Category::new(Category::normalize(&name))?;
    let module = receive_raw(recv).await?;

    runtime.save_module_wait(name, &module, timeout).await?;

    Ok(Response::Published {})
}
