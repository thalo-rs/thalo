use counter::Incremented;
use serde::Deserialize;
use serde_json::json;
use thalo_runtime::rpc::{client::ProjectionClient, Acknowledgement, SubscriptionRequest};

const LAST_GLOBAL_ID_KEY: [u8; 1] = [0];
const COUNT_KEY: [u8; 1] = [1];

#[derive(Deserialize)]
enum ListeningEvent {
    Incremented(Incremented),
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let db = sled::open("projection.db")?;

    let mut last_global_id = db.get(LAST_GLOBAL_ID_KEY)?.map(|value| {
        let slice = value.as_ref().try_into().unwrap();
        u64::from_be_bytes(slice)
    });
    let mut count = db
        .get(COUNT_KEY)?
        .map(|value| {
            let slice = value.as_ref().try_into().unwrap();
            u64::from_be_bytes(slice)
        })
        .unwrap_or(0);

    println!("Count: {}", count);

    let mut client = ProjectionClient::connect("http://localhost:4433").await?;
    let mut streaming = client
        .subscribe_to_events(SubscriptionRequest {
            name: "foo".to_string(),
            events: vec!["Incremented".to_string()],
        })
        .await?
        .into_inner();

    while let Some(message) = streaming.message().await? {
        if last_global_id.map_or(false, |last_global_id| message.global_id <= last_global_id) {
            // Ignore, since we've already handled this event.
            // This logic keeps the projection idempotent, which is important since
            // projections have an at-least-once guarantee, meaning if a connection issue occurs,
            // we might reprocess event we've already seen.
            println!("ignoring since we've handled this message");
            continue;
        }

        // Update the count
        let event = serde_json::from_str::<serde_json::Value>(&message.data)
            .and_then(|payload| serde_json::from_value(json!({ message.msg_type: payload })))?;
        match event {
            ListeningEvent::Incremented(event) => {
                count += event.amount;
            }
        }

        // Save the count
        db.insert(COUNT_KEY, count.to_be_bytes().to_vec())?;
        println!("Count: {}", count);

        // Update last global ID
        db.insert(LAST_GLOBAL_ID_KEY, message.global_id.to_be_bytes().to_vec())?;
        last_global_id = Some(message.global_id);

        // Acknowledge we've handled this event
        client
            .acknowledge_event(Acknowledgement {
                name: "foo".to_string(),
                global_id: message.global_id,
            })
            .await?;
    }

    Ok(())
}
