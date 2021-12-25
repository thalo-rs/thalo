use api::bank_account_server::BankAccountServer;
use api::BankAccountService;
use futures_util::stream::StreamExt;
use thalo::tests_cfg::bank_account::{BankAccount, BankAccountProjection};
use thalo::{event::EventHandler, event_stream::EventStream};
use thalo_inmemory::InMemoryEventStore;
use tokio::sync::broadcast;
use tonic::transport::Server;

mod api;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "[::1]:50051".parse().unwrap();

    // Event stream channel
    let (event_stream, mut event_stream_rx1) = broadcast::channel(16);

    let bank_account_service = BankAccountService::new(event_stream.clone());

    // Projection handler
    {
        let bank_account_service2 = bank_account_service.clone();
        let bank_account_projection = BankAccountProjection::default();
        print_tables(&bank_account_service2.event_store, &bank_account_projection);

        tokio::spawn(async move {
            let mut events = EventStream::<BankAccount>::listen_events(&mut event_stream_rx1);
            while let Some(Ok(event)) = events.next().await {
                bank_account_projection.handle(event).await.unwrap();
                print_tables(&bank_account_service2.event_store, &bank_account_projection);
            }
        });
    }

    // Accept commands through rpc
    Server::builder()
        .add_service(BankAccountServer::new(bank_account_service))
        .serve(addr)
        .await?;

    Ok(())
}

fn print_tables(event_store: &InMemoryEventStore, bank_account_projection: &BankAccountProjection) {
    print!("{esc}[2J{esc}[1;1H", esc = 27 as char);

    println!("Event Store");
    event_store.print();

    println!("\nBank Account Projection");
    bank_account_projection.print_bank_accounts();
}
