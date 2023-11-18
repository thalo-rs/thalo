use thalo_runtime::rpc::{client::ProjectionClient, SubscriptionRequest};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut client = ProjectionClient::connect("http://localhost:4433").await?;
    let mut streaming = client
        .subscribe_to_events(SubscriptionRequest {
            name: "foo".to_string(),
            events: vec!["IncrementedV1".to_string()],
        })
        .await?
        .into_inner();
    while let Some(message) = streaming.message().await? {
        println!("{message:#?}");
    }

    Ok(())
}
