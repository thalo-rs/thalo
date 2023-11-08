mod cli;

#[tokio::main]
async fn main() {
    if let Err(err) = cli::start().await {
        eprintln!("{err}");
        err.chain()
            .skip(1)
            .for_each(|cause| eprintln!("because: {}", cause));
        std::process::exit(1);
    }
}
