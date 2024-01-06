use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use clap::Parser;
use message_db::database::{GetStreamMessagesOpts, MessageStore};
use scylla::{Session, SessionBuilder};
use serde_json::{json, Value};
use thalo_runtime::{rpc, Config, Runtime};
use thalo_scylla::ScyllaEventStore;
use tokio::time::sleep;
use tonic::transport::Server;
use tracing_subscriber::EnvFilter;

/// Thalo runtime - event sourcing runtime
#[derive(Parser, Debug)]
#[command(name = "thalo", version, about, long_about = None)]
struct Cli {
    /// Path to aggregate wasm modules directory
    #[clap(short = 'm', long, env, default_value = "modules")]
    modules_path: PathBuf,
    /// Cache size of streams (LRU)
    #[clap(long, env, default_value = "10000")]
    streams_cache_size: u64,
    /// Address to listen on
    #[clap(long, env, default_value = "0.0.0.0:4433")]
    addr: SocketAddr,
    /// Scylla DB hostname
    #[clap(long, env, default_value = "127.0.0.1:9042")]
    scylla_hostname: String,
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

    // let message_store =
    //     MessageStore::connect("postgres://postgres:hello@localhost:5432/postgres"
    // ).await?;

    // return Ok(());

    // let foo = MessageStore::get_stream_messages::<Value, _>(
    //     &message_store,
    //     "foo-bar",
    //     &GetStreamMessagesOpts::default(),
    // )
    // .await?;
    // dbg!(foo);

    // let settings: ClientSettings =
    //     "esdb://admin:changeit@localhost:2113?tls=false&tlsverifycert=false".
    // parse()?; let client = Client::new(settings)?;
    // let event_store = thalo_eventstore::EventStoreDb::new(client);

    let session: Session = SessionBuilder::new()
        // .known_node("172.17.0.2:9042")
        // .known_node("127.0.0.1:9042")
        .known_node(cli.scylla_hostname)
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
    let session = Arc::new(session);
    // let event_store = ScyllaEventStore::new(session).await?;
    let event_store = thalo_event_indexer::ScyllaEventStore::new(session).await?;

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
    let runtime = Runtime::new(
        event_store,
        Config::new()
            .modules_path(cli.modules_path)
            .streams_cache_size(cli.streams_cache_size),
    )
    .await?;

    // for i in 0..1000 {
    //     let runtime = runtime.clone();
    //     // tokio::spawn(async move {
    //     // sleep(Duration::from_millis(i)).await;
    let msgs = runtime
        .execute(
            "counter".to_string(),
            "123".to_string(),
            "Increment".to_string(),
            json!({ "amount": 1 }),
            3,
        )
        .await;
    if let Err(err) = msgs {
        println!("{err}");
    }
    //     // dbg!(msgs);
    //     // });
    // }

    // sleep(Duration::from_secs(5)).await;
    let command_center_server = rpc::server::CommandCenterServer::new(runtime.clone());
    // let projection_server = rpc::server::ProjectionServer::new(runtime);

    Server::builder()
        .add_service(command_center_server)
        // .add_service(projection_server)
        .serve(cli.addr)
        .await?;

    Ok(())
}

// #[derive(Clone, Debug, Default)]
// struct InMemoryEventStore {
//     streams: Arc<RwLock<HashMap<String, Vec<Message<'static>>>>>,
//     global_index: Arc<RwLock<Vec<(String, usize)>>>,
// }

// #[async_trait]
// impl EventStore for InMemoryEventStore {
//     type Event = Message<'static>;
//     type EventStream = BoxStream<'static, Result<Self::Event, Self::Error>>;
//     type Error = Infallible;

//     async fn append_to_stream(
//         &self,
//         stream_name: &str,
//         events: Vec<NewEvent>,
//         _expected_sequence: Option<u64>,
//     ) -> Result<Vec<Message<'static>>, Self::Error> {
//         let mut streams = self.streams.write().await;
//         let mut global_index = self.global_index.write().await;
//         let stream = streams.entry(stream_name.to_string()).or_default();
//         let mut written_messages = Vec::with_capacity(events.len());
//         for event in events {
//             let stream_sequence = stream.len();
//             let message = Message::new(
//                 StreamName::new(stream_name.to_string()).unwrap(),
//                 stream_sequence as u64,
//                 global_index.len() as u64,
//                 Cow::Owned(event.event_type),
//                 Cow::Owned(event.data),
//             );
//             stream.push(message.clone());
//             written_messages.push(message);
//             global_index.push((stream_name.to_string(), stream_sequence));
//         }

//         Ok(written_messages)
//     }

//     async fn iter_stream(&self, stream_name: &str) ->
// Result<Self::EventStream, Self::Error> {         let streams =
// self.streams.read().await;         let messages: Vec<_> = streams
//             .get(stream_name)
//             .map(|messages| {
//                 messages
//                     .into_iter()
//                     .map(|message| Ok::<_, Self::Error>(message.clone()))
//                     .collect()
//             })
//             .unwrap_or_default();

//         Ok(Box::pin(futures::stream::iter(messages)))
//     }
// }
