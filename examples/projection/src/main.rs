mod projection;

use counter::Incremented;
use serde::Deserialize;
use thalo_runtime::rpc::client::{ProjectionClient, ProjectionClientExt};
use thalo_runtime::rpc::EventInterest;

use crate::projection::CountProjection;

#[derive(Deserialize)]
enum ListeningEvent {
    Incremented(Incremented),
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut client = ProjectionClient::connect("http://localhost:4433").await?;
    let db = sled::open("projection.db")?;
    let count_projection = CountProjection::new(db)?;

    client
        .start_projection(
            env!("CARGO_CRATE_NAME"),
            count_projection,
            vec![EventInterest {
                category: "counter".to_string(),
                event: "Incremented".to_string(),
            }],
        )
        .await?;

    Ok(())
}
