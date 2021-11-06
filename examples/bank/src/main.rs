use std::error;

use aggregate::{BankAccount, BankAccountEvent};
use awto_es::{
    postgres::{tls::NoTls, EventStore},
    AggregateEvent, AggregateType, Event, Repository,
};

mod aggregate;

#[tokio::main]
async fn main() -> Result<(), Box<dyn error::Error>> {
    let event_store = EventStore::connect(
        "postgres://postgres:mysecretpassword@0.0.0.0:5433/postgres",
        NoTls,
    )
    .await?;

    let mut agg: BankAccount = event_store.load_aggregate("elfo".to_string()).await?;
    println!("{:#?}", agg);

    event_store.commit(agg.open_account(0.0)?, &mut agg).await?;
    println!("{:#?}", agg);

    event_store
        .commit(agg.deposit_funds(500.0)?, &mut agg)
        .await?;
    println!("{:#?}", agg);

    event_store
        .commit(agg.withdraw_funds(200.0)?, &mut agg)
        .await?;
    println!("{:#?}", agg);

    event_store
        .commit(agg.withdraw_funds(600.0)?, &mut agg)
        .await?;
    println!("{:#?}", agg);

    Ok(())
}
