## EventStoreDB EventStore for Thalo

Demo repo to be found [here](https://github.com/Shearerbeard/event-sourcing-pizza-delivery-demo-rs).

### Example Useage
```rust
use eventstore::{Client, ClientSettings, SubscribeToAllOptions, SubscriptionFilter};
use thalo_eventstoredb::{ESDBEventStore, ESDBEventPayload};
use order::aggregate::Order;
use order::projection::OrderProjection;
use uuid::Uuid;

#[derive(Debug)]
pub enum Error {
    CouldNotPlaceOrder(aggregate::Error),
    EventStore(thalo_eventstoredb::Error),
    NotFound,
}

// Initialize EventStoreDB EventStore
let client = Client::new(
        "esdb://localhost:2113?tls=false"
            .parse::<ClientSettings>()
            .unwrap(),
    )
    .unwrap();
let orders_store = event_store: ESDBEventStore::new(client);


// Initialize a Thalo Projection
let orders_projection = OrderProjection::default()

// Subscribe and hydrate your Thalo projections with ESDB's built in subs
tokio::spawn(async move {
    let sub_options = SubscribeToAllOptions::default()
        .filter(SubscriptionFilter::on_stream_name().add_prefix("order"));

    let mut sub = client.clone().subscribe_to_all(&sub_options).await;

    loop {
        let event = sub.next().await.unwrap();
        let event_data = event.get_original_event();
        let ee = event_data
            .as_json::<ESDBEventPayload>()
            .unwrap()
            .event_envelope::<Order>(event_data.revision as usize)
            .unwrap();

        if orders_projection.handle(ee).await.is_ok() {
            println!("Projection handled Sub Event!");
        }
    }
});


// Write an event
let result = store.execute(Uuid::new_v4().to_string(), |order: &Order| {
  order.order_placed(order_type.clone(), line_items.clone(), address.clone())
})
.await
.map_err(Error::EventStore)?
.map_err(Error::CouldNotPlaceOrder);

```
