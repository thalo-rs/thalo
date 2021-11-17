use std::time::Duration;

use actix::{Actor, ActorFutureExt, Context, Handler, Message, ResponseActFuture, WrapFuture};
use rdkafka::producer::{FutureProducer, FutureRecord};
use tracing::{error, trace};

use crate::{
    message::StreamTopic, Aggregate, AggregateCommandHandler, AggregateEventHandler,
    AggregateEventOwned, Error, ErrorKind, Event, EventStore,
};

pub trait AggregateActor<ES, A>
where
    Self: Actor<Context = Context<Self>> + Handler<<A as Aggregate>::Command>,
    ES: EventStore,
    A: Aggregate,
    <A as Aggregate>::Command: Message<Result = Result<Vec<AggregateEventOwned>, Error>>,
{
    fn new(id: String, event_store: ES, producer: FutureProducer) -> Self;
}

pub struct BaseAggregateActor<ES: EventStore, A: Aggregate> {
    id: String,
    event_store: ES,
    producer: FutureProducer,
    aggregate: Option<A>,
}

impl<ES, A> AggregateActor<ES, A> for BaseAggregateActor<ES, A>
where
    ES: EventStore + Clone + Unpin + 'static,
    A: Aggregate + Clone + Unpin + 'static,
    <A as Aggregate>::Command: Message<Result = Result<Vec<AggregateEventOwned>, Error>> + Unpin,
    <A as Aggregate>::Event: StreamTopic + Unpin,
{
    fn new(id: String, event_store: ES, producer: FutureProducer) -> Self {
        Self {
            id,
            event_store,
            producer,
            aggregate: None,
        }
    }
}

impl<ES, A> Actor for BaseAggregateActor<ES, A>
where
    ES: EventStore + Unpin + 'static,
    A: Aggregate + Unpin + 'static,
{
    type Context = Context<Self>;
}

impl<ES, A> actix::Message for BaseAggregateActor<ES, A>
where
    ES: EventStore,
    A: Aggregate + 'static,
{
    type Result = Result<Vec<AggregateEventOwned>, Error>;
}

impl<ES, A> Handler<<A as AggregateCommandHandler>::Command> for BaseAggregateActor<ES, A>
where
    ES: EventStore + Clone + Unpin + 'static,
    A: Aggregate + Clone + Unpin + 'static,
    <A as Aggregate>::Command:
        Message<Result = Result<Vec<AggregateEventOwned>, Error>> + Unpin + 'static,
    <A as Aggregate>::Event: StreamTopic + Unpin + 'static,
{
    type Result = ResponseActFuture<Self, Result<Vec<AggregateEventOwned>, Error>>;

    fn handle(
        &mut self,
        msg: <A as AggregateCommandHandler>::Command,
        _ctx: &mut Self::Context,
    ) -> Self::Result {
        let id = self.id.clone();
        let aggregate_opt = self.aggregate.clone();
        let event_store = self.event_store.clone();
        let producer = self.producer.clone();

        Box::pin(
            async move {
                let mut aggregate = match aggregate_opt {
                    Some(aggregate) => aggregate,
                    None => event_store.load_aggregate(id.clone()).await?,
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
                                    FutureRecord::to(
                                        <A as AggregateEventHandler>::Event::stream_topic(),
                                    )
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
