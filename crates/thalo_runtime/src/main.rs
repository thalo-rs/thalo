mod cli;

use std::collections::HashMap;
use std::time::{Duration, UNIX_EPOCH};

use serde_json::json;
use thalo::{Context, Metadata, StreamName};
use thalo_message_store::message::MessageData;
use thalo_message_store::message_store::MessageStore;
use thalo_registry::Registry;
use thalo_runtime::runtime::Runtime;
use tracing::error;
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive("thalo_runtime=info".parse().unwrap())
                .from_env_lossy(),
        )
        // .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
        .init();

    // let message_store = MessageStore::open("./message-store")?;
    // let registry = Registry::open("./registry").unwrap();
    // for res in registry.iter_all_latest() {
    //     let (name, version, _) = res.unwrap();
    //     println!("{name}@{version}");
    // }
    // let runtime = Runtime::new(message_store, registry);

    // let res = runtime
    //     .execute(
    //         "counter".to_string(),
    //         "123".to_string(),
    //         Context {
    //             id: 1,
    //             stream_name: StreamName::new("counter-123").unwrap(),
    //             position: 1,
    //             metadata: Metadata {
    //                 stream_name: StreamName::new("couner-123").unwrap(),
    //                 position: 0,
    //                 causation_message_stream_name: StreamName::new("counter-123").unwrap(),
    //                 causation_message_position: 0,
    //                 reply_stream_name: None,
    //                 schema_version: None,
    //                 properties: HashMap::new(),
    //             },
    //             time: UNIX_EPOCH + Duration::from_millis(1000000),
    //         },
    //         "Increment".to_string(),
    //         json!({
    //             "amount": 10
    //         }),
    //     )
    //     .await?;
    // dbg!(res);

    // for res in registry.iter_all() {
    //     let (name, version, _) = res?;
    //     println!("{name}@{version}");
    // }

    // runtime
    //     .publish_module(
    //         "counter",
    //         Version::parse("0.1.1").unwrap(),
    //         include_bytes!("../../../counter.wasm"),
    //     )
    //     .await?;

    // Ok(())

    if let Err(err) = cli::start().await {
        error!("{err}");
        err.chain()
            .skip(1)
            .for_each(|cause| eprintln!("because: {}", cause));
        std::process::exit(1);
    }
}
