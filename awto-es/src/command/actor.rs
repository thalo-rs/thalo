use actix::{Actor, ActorFutureExt, Context, Handler, Message, ResponseActFuture, WrapFuture};

use crate::{
    message::StreamTopic, Aggregate, AggregateCommandHandler, AggregateEventOwned, Error, Event,
    EventStore,
};

pub trait AggregateActor<ES, A>
where
    Self: Actor<Context = Context<Self>> + Handler<<A as Aggregate>::Command>,
    ES: EventStore,
    A: Aggregate,
    <A as Aggregate>::Command: Message<Result = Result<Vec<AggregateEventOwned>, Error>>,
{
    fn new(id: String, event_store: ES) -> Self;
}

pub struct BaseAggregateActor<ES: EventStore, A: Aggregate> {
    id: String,
    event_store: ES,
    aggregate: Option<A>,
}

impl<ES, A> AggregateActor<ES, A> for BaseAggregateActor<ES, A>
where
    ES: EventStore + Clone + Unpin + 'static,
    A: Aggregate + Clone + Unpin + 'static,
    <A as Aggregate>::Command: Message<Result = Result<Vec<AggregateEventOwned>, Error>> + Unpin,
    <A as Aggregate>::Event: StreamTopic + Unpin,
{
    fn new(id: String, event_store: ES) -> Self {
        Self {
            id,
            event_store,
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
