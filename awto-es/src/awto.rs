use std::{env, error, fmt, pin::Pin, sync::Arc, time::Duration};

use actix::{dev::ToEnvelope, Actor, Addr};
use futures::{stream::FuturesUnordered, Future, FutureExt, StreamExt};
use lru::LruCache;
use rdkafka::{
    config::RDKafkaLogLevel,
    consumer::{Consumer, StreamConsumer},
    producer::FutureProducer,
    ClientConfig, Message,
};
use tokio::{
    signal,
    sync::{
        watch::{self, Receiver},
        Mutex,
    },
};
use tracing::{debug, trace};

use crate::{
    message::StreamTopic, Aggregate, AggregateActor, AggregateEventOwned, BaseAggregateActor,
    Error, ErrorKind, EventHandler, EventStore, InternalError, Projection,
};

type WorkerFuture = Pin<Box<dyn Future<Output = ()> + Send>>;
type WorkerFn<ES> = Box<dyn Fn(WorkerContext<ES>) -> Result<WorkerFuture, Box<dyn error::Error>>>;

struct WorkerContext<'a, ES: EventStore> {
    consumer_config: &'a ClientConfig,
    event_store: &'a ES,
    on_error: &'a Option<for<'r> fn(&'r dyn error::Error)>,
    producer_config: &'a ClientConfig,
    shutdown_recv: &'a Receiver<()>,
}

pub struct Awto<ES: EventStore> {
    consumer_config: ClientConfig,
    event_store: ES,
    on_error: Option<for<'r> fn(&'r dyn error::Error)>,
    producer_config: ClientConfig,
    workers: Vec<WorkerFn<ES>>,
}

impl<ES> Awto<ES>
where
    ES: EventStore + Clone + Send + Sync + Unpin + 'static,
{
    pub fn build(event_store: ES, redpanda_host: impl Into<String> + Clone) -> AwtoBuilder<ES> {
        AwtoBuilder::new(event_store, redpanda_host)
    }

    pub async fn run(&mut self) -> Result<(), Box<dyn error::Error>> {
        let (shutdown_send, shutdown_recv) = watch::channel(());
        let mut workers = FuturesUnordered::new();

        for worker in &self.workers {
            let ctx = WorkerContext {
                consumer_config: &self.consumer_config,
                event_store: &self.event_store,
                on_error: &self.on_error,
                producer_config: &self.producer_config,
                shutdown_recv: &shutdown_recv,
            };
            workers.push(worker(ctx)?.boxed());
        }

        actix::spawn(async move { while workers.next().await.is_some() {} });

        signal::ctrl_c().await.ok();

        shutdown_send.send(())?;
        tokio::time::sleep(Duration::from_millis(500)).await;

        Ok(())
    }
}

pub struct AwtoBuilder<ES>
where
    ES: EventStore + Clone + Send + Sync + Unpin + 'static,
{
    consumer_config: ClientConfig,
    event_store: ES,
    on_error: Option<fn(&dyn error::Error)>,
    producer_config: ClientConfig,
    workers: Vec<WorkerFn<ES>>,
}

impl<ES> AwtoBuilder<ES>
where
    ES: EventStore + Clone + Send + Sync + Unpin + 'static,
{
    pub fn new(event_store: ES, redpanda_host: impl Into<String> + Clone) -> AwtoBuilder<ES> {
        let mut consumer_config = ClientConfig::new();
        consumer_config
            .set("group.id", env!("CARGO_PKG_NAME"))
            .set("bootstrap.servers", redpanda_host.clone())
            .set("allow.auto.create.topics", "true")
            .set("enable.auto.commit", "true")
            .set("auto.offset.reset", "earliest")
            .set_log_level(RDKafkaLogLevel::Debug);

        let mut producer_config = ClientConfig::new();
        producer_config
            .set("bootstrap.servers", redpanda_host)
            .set_log_level(RDKafkaLogLevel::Debug);

        Self {
            consumer_config,
            event_store,
            on_error: None,
            producer_config,
            workers: Vec::new(),
        }
    }

    pub async fn run(self) -> Result<(), Box<dyn error::Error>> {
        let mut app = Awto {
            consumer_config: self.consumer_config,
            event_store: self.event_store,
            on_error: self.on_error,
            producer_config: self.producer_config,
            workers: self.workers,
        };

        app.run().await
    }

    pub fn on_error(mut self, cb: fn(&dyn error::Error)) -> Self {
        self.on_error = Some(cb);
        self
    }

    pub fn aggregate<A>(self, cache_cap: usize) -> Self
    where
        A: Aggregate + Clone + Unpin + 'static,
        <A as Aggregate>::Command:
            actix::Message<Result = Result<Vec<AggregateEventOwned>, Error>> + StreamTopic + Unpin,
        <A as Aggregate>::Event: StreamTopic + Unpin,
    {
        self.aggregate_actor::<BaseAggregateActor<ES, A>, A>(cache_cap)
    }

    pub fn aggregate_actor<Act, A>(mut self, cache_cap: usize) -> Self
    where
        Act: AggregateActor<ES, A>,
        A: Aggregate,
        <A as Aggregate>::Command: actix::Message<Result = Result<Vec<AggregateEventOwned>, Error>>
            + StreamTopic
            + Unpin
            + 'static,
        <A as Aggregate>::Event: StreamTopic + Unpin,
        <Act as Actor>::Context: ToEnvelope<Act, <A as Aggregate>::Command>,
    {
        self.workers.push(Box::new(move |ctx| {
            let event_store = ctx.event_store.to_owned();
            let on_error = ctx.on_error.to_owned();

            let actors_cache: Arc<Mutex<LruCache<String, Addr<Act>>>> =
                Arc::new(Mutex::new(LruCache::new(cache_cap)));

            let consumer: Arc<StreamConsumer> = Arc::new(ctx.consumer_config.create()?);
            let producer: FutureProducer = ctx.producer_config.create()?;

            consumer.subscribe(&[<A as Aggregate>::Command::stream_topic()])?;
            debug!(
                topic = <A as Aggregate>::Command::stream_topic(),
                "subscribed to command topic"
            );

            {
                let mut shutdown_recv = ctx.shutdown_recv.clone();
                let consumer = Arc::clone(&consumer);
                tokio::spawn(async move {
                    while shutdown_recv.changed().await.is_ok() {
                        consumer.unsubscribe();
                        debug!(
                            topic = <A as Aggregate>::Command::stream_topic(),
                            "unsubscribed from command topic"
                        )
                    }
                });
            }

            Ok(async move {
                let actors_cache = Arc::clone(&actors_cache);

                loop_result_async(on_error, || async {
                    let actors_cache = Arc::clone(&actors_cache);
                    let msg = consumer
                        .recv()
                        .await
                        .internal_error("could not receive message from command handler")?;
                    let topic = msg.topic();
                    let offset = msg.offset();
                    let key = msg
                        .key_view::<str>()
                        .transpose()
                        .internal_error("could not read key as str")?
                        .ok_or_else(|| Error::new_simple(ErrorKind::MissingKey))?;

                    if let Some(payload) = msg.payload() {
                        let command = serde_json::from_slice::<<A as Aggregate>::Command>(payload)
                            .map_err(|err| Error::new(ErrorKind::DeserializeError, err))?;
                        debug!(topic, offset, key, ?command, "received command");

                        {
                            let mut guard = actors_cache.lock().await;
                            let actor = match guard.get(key) {
                                Some(actor) => actor,
                                None => {
                                    let actor = Act::new(
                                        key.to_string(),
                                        event_store.clone(),
                                        producer.clone(),
                                    );
                                    guard.put(key.to_string(), actor.start());
                                    guard.get(key).expect("item should exist")
                                }
                            };

                            actor.send(command)
                        }
                        .await
                        .map_err(|_err| Error::new_simple(ErrorKind::MailboxFull))??;
                    }

                    Ok(())
                })
                .await
            }
            .boxed())
        }));

        self
    }

    pub fn projection<P>(mut self, projection: P) -> Self
    where
        P: Projection + Clone + Send + Sync + 'static,
        <P as EventHandler>::Event: StreamTopic + fmt::Debug,
    {
        self.workers.push(Box::new(move |ctx| {
            let event_store = ctx.event_store.to_owned();
            let on_error = ctx.on_error.to_owned();
            let mut projection = projection.clone();

            let consumer: Arc<StreamConsumer> = Arc::new(ctx.consumer_config.create()?);
            consumer.subscribe(&[<P as EventHandler>::Event::stream_topic()])?;
            debug!(
                topic = <P as EventHandler>::Event::stream_topic(),
                "subscribed to event topic"
            );

            {
                let mut shutdown_recv = ctx.shutdown_recv.clone();
                let consumer = Arc::clone(&consumer);
                tokio::spawn(async move {
                    while shutdown_recv.changed().await.is_ok() {
                        consumer.unsubscribe();
                        debug!(
                            topic = <P as EventHandler>::Event::stream_topic(),
                            "unsubscribed from event topic"
                        )
                    }
                });
            }

            Ok(async move {
                if let Err(err) = event_store.resync_projection(&mut projection).await {
                    if let Some(on_error) = on_error {
                        on_error(&err);
                    }
                }

                loop_result_async(on_error, || async {
                    let msg = consumer
                        .recv()
                        .await
                        .internal_error("could not receive message from event handler")?;
                    let topic = msg.topic();
                    let offset = msg.offset();
                    let key = msg
                        .key_view::<str>()
                        .transpose()
                        .internal_error("could not read key as str")?
                        .ok_or_else(|| Error::new_simple(ErrorKind::MissingKey))?;

                    if let Some(payload) = msg.payload() {
                        let event_envelope: AggregateEventOwned =
                            serde_json::from_slice(payload)
                                .map_err(|err| Error::new(ErrorKind::DeserializeError, err))?;
                        let event_id = event_envelope.id;
                        let event_sequence = event_envelope.sequence;
                        let event: <P as EventHandler>::Event =
                            serde_json::from_value(event_envelope.event_data)
                                .map_err(|err| Error::new(ErrorKind::DeserializeError, err))?;
                        debug!(
                            topic,
                            offset,
                            key,
                            event_id,
                            event_sequence,
                            ?event,
                            "received event"
                        );

                        projection
                            .clone()
                            .handle(key.to_string(), event, event_id, event_sequence)
                            .await?;

                        trace!(key, event_id, "handled projectoion");
                    }

                    Ok(())
                })
                .await
            }
            .boxed())
        }));

        self
    }
}

pub async fn loop_result_async<Fut, F>(on_error: Option<fn(&dyn error::Error)>, f: F)
where
    Fut: Future<Output = Result<(), Box<dyn error::Error>>>,
    F: Fn() -> Fut,
{
    loop {
        if let Err(err) = f().await {
            if let Some(on_error) = on_error {
                on_error(err.as_ref())
            }
        }
    }
}
