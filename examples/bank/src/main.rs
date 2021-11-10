use std::{error, io::stdin};

use awto_es::{
    postgres::{tls::NoTls, PgEventStore},
    Aggregate, AggregateEvent, AggregateStateMutator, Event, EventHandler, EventStore, Identity,
    Repository,
};
use command::BankAccount;

use crate::{
    command::BankAccountEvent,
    query::{BankAccountProjector, BankAccountViewRepository},
};

mod command;
mod query;

#[tokio::main]
async fn main() -> Result<(), Box<dyn error::Error>> {
    let conn = "postgres://postgres:mysecretpassword@0.0.0.0:5433/postgres";
    let tls = NoTls;

    let bank_account_repository = BankAccountViewRepository::connect(conn, tls).await?;
    let event_store = PgEventStore::connect(conn, tls).await?;
    let bank_account_projector =
        BankAccountProjector::new(event_store.clone(), bank_account_repository);
    let agg: BankAccount = event_store.load_aggregate("jimmy".to_string()).await?;
    println!("{:#?}", agg);

    let mut app = App {
        event_store,
        agg,
        proj: bank_account_projector,
    };

    // let events = app.agg.deposit_funds(6.0)?;
    // app.submit_events("jimmy".to_string(), events).await?;

    // let events = app.agg.open_account(0.0)?;
    // app.submit_events("jimmy".to_string(), events).await?;

    // let mut line = String::new();
    // stdin().read_line(&mut line)?;

    // let events = app.agg.deposit_funds(500.0)?;
    // app.submit_events("jimmy".to_string(), events).await?;

    // let mut line = String::new();
    // stdin().read_line(&mut line)?;

    // let events = app.agg.withdraw_funds(200.0)?;
    // app.submit_events("jimmy".to_string(), events).await?;

    // let mut line = String::new();
    // stdin().read_line(&mut line)?;

    // let events = app.agg.withdraw_funds(600.0)?;
    // app.submit_events("jimmy".to_string(), events).await?;

    // event_store
    //     .commit(agg.deposit_funds(500.0)?, &mut agg)
    //     .await?;
    // println!("{:#?}", agg);

    // event_store
    //     .commit(agg.withdraw_funds(200.0)?, &mut agg)
    //     .await?;
    // println!("{:#?}", agg);

    // event_store
    //     .commit(agg.withdraw_funds(600.0)?, &mut agg)
    //     .await?;
    // println!("{:#?}", agg);

    Ok(())
}

struct App<ES, Agg, Proj>
where
    ES: EventStore,
    Agg: Aggregate,
    Proj: EventHandler,
{
    pub event_store: ES,
    pub agg: Agg,
    pub proj: Proj,
}

impl<ES, Agg, Proj, Event> App<ES, Agg, Proj>
where
    ES: EventStore,
    Agg: Aggregate<Event = Event> + AggregateStateMutator<Event = Event>,
    <Agg as Identity>::Identity: AsRef<str>,
    Proj: EventHandler<Event = Event>,
    <Proj as awto_es::EventHandler>::Id: Clone,
    Event: awto_es::Event,
{
    async fn submit_events(
        &mut self,
        id: <Proj as awto_es::EventHandler>::Id,
        events: Vec<Event>,
    ) -> Result<(), Box<dyn error::Error>> {
        for event in events {
            let agg_event = event.aggregate_event(Identity::identity(&self.agg).as_ref())?;
            let last_event_sequence = self.event_store.save_events(&[agg_event]).await?.unwrap();
            self.agg.apply(event.clone());
            self.proj
                .handle(id.clone(), event, last_event_sequence)
                .await?;
        }
        Ok(())
    }
}
