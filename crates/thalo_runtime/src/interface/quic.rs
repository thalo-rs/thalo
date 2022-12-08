use std::io;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{anyhow, bail, Context, Result};
use esdl::schema::Schema;
use futures::{StreamExt, TryFutureExt};
use message_db::database::{MessageStore, SubscribeToCategoryOpts};
use message_db::message::MessageData;
use message_db::stream_name::{Category, StreamName};
use quinn::{RecvStream, SendStream};
use rustls::PrivateKey;
use tokio::fs;
use tracing::{error, info, info_span, warn, Instrument};

use crate::interface::message::{pack, receive, receive_raw, ExecutedResult, Request, Response};
use crate::module::{ModuleName, SchemaModule};
use crate::runtime::Runtime;

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
        info!("connection incoming");
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
        let dirs = directories_next::ProjectDirs::from("", "thalo", "thalo").unwrap();
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
    let span = info_span!(
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
        info!("established");

        // Each stream initiated by the client constitutes a new request.
        loop {
            let stream = connection.accept_bi().await;
            let stream = match stream {
                Err(quinn::ConnectionError::ApplicationClosed { .. }) => {
                    info!("connection closed");
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
                .instrument(info_span!("request")),
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
            data,
        } => handle_execute(&runtime, name, id, command, data).await,
        Request::Publish {} => handle_publish(&runtime, &mut recv).await,
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

    info!("handled request");
    Ok(())
}

pub async fn handle_execute(
    runtime: &Runtime,
    name: ModuleName,
    id: String,
    command: String,
    data: Vec<u8>,
) -> Result<Response> {
    let data = serde_json::from_slice(&data).context("invalid command data json")?;
    let command_position = runtime.submit_command(&name, &id, &command, &data).await?;

    let event_category = Category::normalize(&name);

    let mut command_category: Category = event_category.parse()?;
    command_category.types.push("command".to_string());
    let command_stream_name = StreamName {
        category: command_category,
        id: Some(id.parse()?),
    };

    let condition = format!(
        "metadata->>'causation_message_stream_name' = '{command_stream_name}' AND metadata->>'causation_message_position' = '{command_position}'"
    );

    let mut stream = MessageStore::subscribe_to_category::<MessageData, _>(
        runtime.message_store(),
        &event_category,
        &SubscribeToCategoryOpts::builder()
            .position_update_interval(0)
            .condition(&condition)
            .build(),
    )
    .await?;

    tokio::select! {
        Some(result) = stream.next() => {
            let events = result?;
            Ok(Response::Executed(ExecutedResult::Events(events)))
        }
        _ = tokio::time::sleep(Duration::from_secs(10)) => {
            warn!("command timed out when waiting for causation events");
            Ok(Response::Executed(ExecutedResult::TimedOut))
        }
    }
}

pub async fn handle_publish(runtime: &Runtime, recv: &mut RecvStream) -> Result<Response> {
    let schema: Schema = receive(recv).await?;
    let module = receive_raw(recv).await?;

    runtime
        .publish_module(SchemaModule::new(schema, module)?)
        .await?;

    Ok(Response::Published {})
}
