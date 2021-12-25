use std::{marker::PhantomData, sync::Arc};

use actix::{
    Actor, ActorFutureExt, Addr, ArbiterHandle, Context, Handler, Message, ResponseActFuture,
    System, WrapFuture,
};
use lru::LruCache;
use tokio::sync::Mutex;
use tracing::{debug, trace};

use crate::{
    Aggregate, AggregateCommandHandler, AggregateEvent, AggregateType, Error, Event, EventEnvelope,
    EventStore, StreamTopic,
};

pub trait AggregateActor<ES, A>
where
    Self: Actor<Context = Context<Self>> + Handler<<A as Aggregate>::Command>,
    ES: EventStore,
    A: Aggregate,
    <A as Aggregate>::Command:
        Message<Result = Result<Vec<EventEnvelope<<A as Aggregate>::Event>>, Error>>,
{
    fn new(id: String, event_store: ES) -> Self;
}

#[derive(Clone)]
pub struct BaseAggregateActor<ES: EventStore, A: Aggregate> {
    id: String,
    event_store: ES,
    aggregate: Option<A>,
}

impl<ES, A, C, E> AggregateActor<ES, A> for BaseAggregateActor<ES, A>
where
    ES: EventStore + Clone + Unpin + 'static,
    A: Aggregate<Command = C, Event = E> + Clone + Unpin + 'static,
    E: Event<Aggregate = A> + StreamTopic + Unpin + 'static,
    C: Message<Result = Result<Vec<EventEnvelope<<A as Aggregate>::Event>>, Error>>
        + Unpin
        + 'static,
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

impl<ES, A, E> actix::Message for BaseAggregateActor<ES, A>
where
    ES: EventStore,
    A: Aggregate<Event = E> + 'static,
    E: Event<Aggregate = A> + 'static,
{
    type Result = Result<Vec<EventEnvelope<E>>, Error>;
}

impl<ES, A, C, E> Handler<<A as AggregateCommandHandler>::Command> for BaseAggregateActor<ES, A>
where
    ES: EventStore + Clone + Unpin + 'static,
    A: Aggregate<Command = C, Event = E> + Clone + Unpin + 'static,
    C: Message<Result = Result<Vec<EventEnvelope<E>>, Error>> + Unpin + 'static,
    E: Event<Aggregate = A> + StreamTopic + Unpin + 'static,
{
    type Result = ResponseActFuture<Self, Result<Vec<EventEnvelope<E>>, Error>>;

    fn handle(&mut self, msg: C, _ctx: &mut Self::Context) -> Self::Result {
        let id = self.id.clone();
        let aggregate_opt = self.aggregate.clone();
        let event_store = self.event_store.clone();

        Box::pin(
            async move {
                let mut aggregate = match aggregate_opt {
                    Some(aggregate) => {
                        trace!(%id, aggregate_type = %<A as AggregateType>::aggregate_type(), "loaded aggregate from cache");
                        aggregate
                    }
                    None => {
                        let aggregate = event_store.load_aggregate(&id).await?;
                        trace!(%id, aggregate_type = %<A as AggregateType>::aggregate_type(), "loaded aggregate from event store");
                        aggregate
                    }
                };

                let events = aggregate.execute(msg).map_err(|err| {
                    debug!(%id, %err, "aggregate command failed");
                    err
                })?;
                let agg_events: Vec<_> = events
                    .iter()
                    .map(|event| AggregateEvent::<'_, <E as Event>::Aggregate>::new(&id, event))
                    .collect();
                let inserted_events = event_store.save_events(agg_events).await?;
                events.into_iter().for_each(|event| aggregate.apply(event));

                Result::<_, Error>::Ok((aggregate, inserted_events))
            }
            .into_actor(self)
            .map(|res, act, _ctx| {
                let (aggregate, events) = res?;
                act.aggregate = Some(aggregate); // TODO: This doesnt work
                Ok(events)
            }),
        )
    }
}

#[derive(Clone)]
pub struct AggregateActorPool<Act, ES, A>
where
    Act: AggregateActor<ES, A>,
    ES: EventStore,
    A: Aggregate,
    <A as Aggregate>::Command:
        Message<Result = Result<Vec<EventEnvelope<<A as Aggregate>::Event>>, Error>>,
{
    actors_cache: Arc<Mutex<LruCache<String, Addr<Act>>>>,
    aggregate: PhantomData<A>,
    arbiter: ArbiterHandle,
    event_store: ES,
}

impl<Act, ES, A> AggregateActorPool<Act, ES, A>
where
    Act: AggregateActor<ES, A>,
    ES: EventStore + Clone + Unpin + Send + 'static,
    A: Aggregate + Clone + Unpin + 'static,
    <A as Aggregate>::Command: Message<Result = Result<Vec<EventEnvelope<<A as Aggregate>::Event>>, Error>>
        + Clone
        + Unpin
        + 'static,
    <A as Aggregate>::Event: Event<Aggregate = A> + StreamTopic + Unpin + 'static,
{
    pub fn new(event_store: ES, cache_cap: usize) -> Result<Self, Error> {
        let actors_cache: Arc<Mutex<LruCache<String, Addr<Act>>>> =
            Arc::new(Mutex::new(LruCache::new(cache_cap)));

        Ok(AggregateActorPool {
            actors_cache,
            aggregate: PhantomData,
            arbiter: System::try_current()
                .ok_or(Error::MissingActixSystem)?
                .arbiter()
                .clone(),
            event_store,
        })
    }

    pub fn new_in_arbiter(arbiter: ArbiterHandle, event_store: ES, cache_cap: usize) -> Self {
        let actors_cache: Arc<Mutex<LruCache<String, Addr<Act>>>> =
            Arc::new(Mutex::new(LruCache::new(cache_cap)));

        AggregateActorPool {
            actors_cache,
            aggregate: PhantomData,
            arbiter,
            event_store,
        }
    }

    /// Send a command without waiting for the response.
    pub async fn do_send(&self, id: &str, command: <A as Aggregate>::Command) -> Result<(), Error> {
        let mut guard = self.actors_cache.lock().await;
        let actor = match guard.get(id) {
            Some(actor) => actor,
            None => {
                let id_string = id.to_string();
                let event_store = self.event_store.clone();
                let addr =
                    Act::start_in_arbiter(&self.arbiter, move |_| Act::new(id_string, event_store));

                guard.put(id.to_string(), addr);
                guard.get(id).expect("item should exist")
            }
        };
        actor.do_send(command);
        Ok(())
    }

    /// Send a command and wait for the response.
    pub async fn send(
        &self,
        id: &str,
        command: <A as Aggregate>::Command,
    ) -> Result<Vec<EventEnvelope<<A as Aggregate>::Event>>, Error> {
        {
            let mut guard = self.actors_cache.lock().await;
            let actor = match guard.get(id) {
                Some(actor) => actor,
                None => {
                    let id_string = id.to_string();
                    let event_store = self.event_store.clone();
                    let addr = Act::start_in_arbiter(&self.arbiter, move |_| {
                        Act::new(id_string, event_store)
                    });

                    guard.put(id.to_string(), addr);
                    guard.get(id).expect("item should exist")
                }
            };
            actor.send(command)
        }
        .await?
    }
}
