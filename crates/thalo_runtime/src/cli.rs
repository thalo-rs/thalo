use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::HashMap;
use std::convert::Infallible;
use std::error::Error as StdError;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use async_trait::async_trait;
use clap::Parser;
use eventstore::{Client, ClientSettings, EventData};
use futures::stream::BoxStream;
use redis::streams::StreamMaxlen;
use scylla::{Session, SessionBuilder};
use serde::{Deserialize, Serialize};
use serde_json::json;
use thalo::event_store::message::Message;
use thalo::event_store::{EventStore, NewEvent};
use thalo::stream_name::StreamName;
// use thalo_message_store::MessageStore;
// use thalo_runtime::relay::{RedisRelay, Relay};
use thalo_runtime::Runtime;
use thalo_scylla::ScyllaEventStore;
use thiserror::Error;
use tokio::sync::RwLock;
use tonic::transport::Server;
use tracing_subscriber::EnvFilter;

/// Thalo runtime - event sourcing runtime
#[derive(Parser, Debug)]
#[command(name = "thalo", version, about, long_about = None)]
struct Cli {
    /// Message store path
    #[clap(short = 's', long, default_value = "message-store.db")]
    message_store_path: PathBuf,
    /// Path to aggregate wasm modules directory
    #[clap(short = 'm', long, default_value = "modules")]
    modules_path: PathBuf,
    /// Cache size of aggregates (LRU)
    #[clap(long, default_value = "10000")]
    cache_size: u64,
    /// Redis relay
    #[clap(long)]
    redis: Option<String>,
    /// Redis stream max len
    #[clap(long, default_value = "10000")]
    redis_stream_max_len: usize,
    /// Template for redis streams (only `{category}` is supported)
    #[clap(long, default_value = "{category}_events")]
    redis_stream_name_template: String,
    /// Address to listen on
    #[clap(long, default_value = "[::1]:4433")]
    addr: SocketAddr,
    /// Log levels
    #[clap(
        long,
        env,
        default_value = "thalo_runtime=info,thalo_message_store=info,warn"
    )]
    log: String,
}

pub async fn start() -> Result<()> {
    let cli = Cli::parse();

    tracing_subscriber::fmt()
        .with_target(false)
        .with_file(false)
        .with_line_number(false)
        .with_env_filter(EnvFilter::builder().parse_lossy(cli.log))
        .init();

    // let settings: ClientSettings =
    //     "esdb://admin:changeit@localhost:2113?tls=false&tlsverifycert=false".
    // parse()?; let client = Client::new(settings)?;
    // let event_store = thalo_eventstore::EventStoreDb::new(client);

    let session: Session = SessionBuilder::new()
        .known_node("172.17.0.2:9042")
        .keyspaces_to_fetch(["thalo"])
        // .known_node("127.0.0.72:4321")
        // .known_node("localhost:8000")
        .connection_timeout(Duration::from_secs(3))
        .cluster_metadata_refresh_interval(Duration::from_secs(10))
        // .known_node_addr(SocketAddr::new(
        //     IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
        //     9000,
        // ))
        .build()
        .await?;
    let event_store = ScyllaEventStore::new(Arc::new(session)).await?;

    // return Ok(());

    // let message_store =
    //     Arc::new(Box::new(MessageStore::open(&cli.message_store_path)?) as
    // BoxEventStore);
    // let relay = match cli.redis {
    //     Some(params) => {
    //         let conn = redis::Client::open(params)?;
    //         Relay::Redis(RedisRelay::new(
    //             conn.get_multiplexed_tokio_connection().await?,
    //             StreamMaxlen::Approx(cli.redis_stream_max_len),
    //             cli.redis_stream_name_template,
    //         ))
    //     }
    //     None => Relay::Noop,
    // };
    let runtime = Runtime::new(event_store, cli.modules_path, cli.cache_size).await?;

    for _ in 0..1 {
        let msgs = runtime
            .execute(
                "counter".to_string(),
                "123".to_string(),
                "Increment".to_string(),
                json!({ "amount": 2 }),
            )
            .await?
            .unwrap();
        dbg!(msgs);
    }

    // let command_center_server =
    // rpc::server::CommandCenterServer::new(runtime.clone());
    // let projection_server = rpc::server::ProjectionServer::new(runtime);

    // Server::builder()
    //     .add_service(command_center_server)
    //     .add_service(projection_server)
    //     .serve(cli.addr)
    //     .await?;

    Ok(())
}

#[derive(Clone, Debug, Default)]
struct InMemoryEventStore {
    streams: Arc<RwLock<HashMap<String, Vec<Message<'static>>>>>,
    global_index: Arc<RwLock<Vec<(String, usize)>>>,
}

#[async_trait]
impl EventStore for InMemoryEventStore {
    type Event = Message<'static>;
    type EventStream = BoxStream<'static, Result<Self::Event, Self::Error>>;
    type Error = Infallible;

    async fn append_to_stream(
        &self,
        stream_name: &str,
        events: Vec<NewEvent>,
        _expected_sequence: Option<u64>,
    ) -> Result<Vec<Message<'static>>, Self::Error> {
        let mut streams = self.streams.write().await;
        let mut global_index = self.global_index.write().await;
        let stream = streams.entry(stream_name.to_string()).or_default();
        let mut written_messages = Vec::with_capacity(events.len());
        for event in events {
            let stream_sequence = stream.len();
            let message = Message::new(
                StreamName::new(stream_name.to_string()).unwrap(),
                stream_sequence as u64,
                global_index.len() as u64,
                Cow::Owned(event.event_type),
                Cow::Owned(event.data),
            );
            stream.push(message.clone());
            written_messages.push(message);
            global_index.push((stream_name.to_string(), stream_sequence));
        }

        Ok(written_messages)
    }

    async fn iter_stream(&self, stream_name: &str) -> Result<Self::EventStream, Self::Error> {
        let streams = self.streams.read().await;
        let messages: Vec<_> = streams
            .get(stream_name)
            .map(|messages| {
                messages
                    .into_iter()
                    .map(|message| Ok::<_, Self::Error>(message.clone()))
                    .collect()
            })
            .unwrap_or_default();

        Ok(Box::pin(futures::stream::iter(messages)))
    }
}
