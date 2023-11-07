mod cli;

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

    // let message_store = MessageStore::open("./message-store").unwrap();
    // let registry = Registry::open("./registry").unwrap();
    // for res in registry.iter_all_latest() {
    //     let (name, version, _) = res.unwrap();
    //     println!("{name}@{version}");
    // }
    // let runtime = Runtime::new(message_store, "modules").await.unwrap();
    // let count = 100_000;
    // let start = Instant::now();
    // for _ in 0..count {
    //     runtime
    //         .execute(
    //             Category::new("counter").unwrap(),
    //             ID::new("123").unwrap(),
    //             "Increment".to_string(),
    //             json!({
    //                 "amount": 10
    //             }),
    //         )
    //         .await
    //         .unwrap();
    //     println!("{}", start.elapsed().as_micros());
    // }
    // let dur = start.elapsed();
    // println!(
    //     "TOTAL: {}      -  {} / sec",
    //     dur.as_millis(),
    //     count * 1000 / dur.as_millis()
    // );

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
        eprintln!("{err}");
        err.chain()
            .skip(1)
            .for_each(|cause| eprintln!("because: {}", cause));
        std::process::exit(1);
    }
}
