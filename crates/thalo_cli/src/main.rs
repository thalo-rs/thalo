mod cli;

use tracing::error;
use tracing_subscriber::EnvFilter;

#[derive(Clone, Debug)]
struct Payload(Vec<u8>);

#[tokio::main]
async fn main() {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive("thalo_cli=info".parse().unwrap())
                .from_env_lossy(),
        )
        .init();

    if let Err(err) = cli::run().await {
        error!("[error]: {err}");
        err.chain()
            .skip(1)
            .for_each(|cause| error!("because: {}", cause));
        std::process::exit(1);
    }
}
