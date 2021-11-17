use std::{borrow::Cow, time::Duration};

use actix::{Actor, ActorFutureExt, Context, Handler, ResponseActFuture, WrapFuture};
use awto_es::{
    postgres::PgEventStore, AggregateCommandHandler, AggregateEventHandler, AggregateEventOwned,
    Error, ErrorKind, Event, EventStore,
};
use bb8_postgres::tokio_postgres::NoTls;
use rdkafka::producer::{FutureProducer, FutureRecord};
use tracing::{error, trace};

use super::{BankAccount, BankAccountCommand};

pub struct BankAccountActor {
    id: String,
    event_store: PgEventStore<NoTls>,
    producer: FutureProducer,
    aggregate: Option<BankAccount>,
}

impl BankAccountActor {
    pub fn new(id: String, event_store: PgEventStore<NoTls>, producer: FutureProducer) -> Self {
        Self {
            id,
            event_store,
            producer,
            aggregate: None,
        }
    }
}

impl Actor for BankAccountActor {
    type Context = Context<Self>;
}

impl actix::Message for BankAccountCommand {
    type Result = Result<Vec<AggregateEventOwned>, Error>;
}

impl Handler<BankAccountCommand> for BankAccountActor {
    type Result = ResponseActFuture<Self, Result<Vec<AggregateEventOwned>, Error>>;

    fn handle(&mut self, msg: BankAccountCommand, _ctx: &mut Self::Context) -> Self::Result {
        let id = self.id.clone();
        let aggregate_opt = &self.aggregate;
        let event_store = self.event_store.clone();
        let producer = self.producer.clone();

        Box::pin(
            async move {
                let mut aggregate = match aggregate_opt {
                    Some(aggregate) => Cow::Borrowed(aggregate),
                    None => Cow::Owned(event_store.load_aggregate(id.clone()).await?),
                };

                let events = aggregate.execute(msg)?;
                let agg_events: Vec<_> = events
                    .iter()
                    .map(|event| event.aggregate_event(&id))
                    .collect::<Result<_, _>>()?;
                let inserted_events = event_store.save_events(&agg_events).await?;
                events.into_iter().for_each(|event| aggregate.apply(event));

                let send_results: Vec<_> = inserted_events
                    .iter()
                    .map(|event| {
                        let id = id.as_str();
                        let producer_ref = &producer;
                        async move {
                            let delivery_status = producer_ref
                                .send(
                                    FutureRecord::to("bank-account-event")
                                        .payload(&serde_json::to_vec(&event).map_err(|err| {
                                            Error::new(ErrorKind::SerializeError, err)
                                        })?)
                                        .key(id),
                                    Duration::from_secs(0),
                                )
                                .await;

                            trace!(?event, "submitted event to broker");

                            Result::<_, Error>::Ok(delivery_status)
                        }
                    })
                    .collect();

                for result in send_results {
                    match result.await {
                        Ok(Ok((partition, offset))) => {
                            trace!(partition, offset, "produced message");
                        }
                        Ok(Err((err, _))) => {
                            error!("could not produce message: {}", err);
                        }
                        Err(err) => {
                            error!("could not produce message: {}", err);
                        }
                    }
                }

                Result::<_, Error>::Ok((aggregate, inserted_events))
            }
            .into_actor(self)
            .map(|res, act, _ctx| {
                let (aggregate, events) = res?;
                act.aggregate = Some(aggregate);
                Ok(events)
            }),
        )
    }
}
