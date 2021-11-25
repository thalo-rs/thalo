use std::{env, fmt, pin::Pin, sync::Arc, time::Duration};

use actix::{dev::ToEnvelope, Actor};
#[cfg(feature = "outbox_relay")]
use bb8_postgres::{
    bb8::Pool,
    tokio_postgres::{
        tls::{MakeTlsConnect, TlsConnect},
        Socket,
    },
    PostgresConnectionManager,
};
use futures::{stream::FuturesUnordered, Future, FutureExt, StreamExt};
use rdkafka::{
    config::RDKafkaLogLevel,
    consumer::{Consumer, StreamConsumer},
    message::Headers,
    ClientConfig, Message,
};
use tokio::{
    signal,
    sync::watch::{self, Receiver},
};
use tracing::{debug, info, trace};

use crate::{
    MultiStreamTopic, StreamTopic,
    Aggregate, AggregateActor, AggregateActorPool, BaseAggregateActor, Error, EventEnvelope,
    EventHandler, EventStore, Projection,
};

type WorkerFuture = Pin<Box<dyn Future<Output = ()> + Send>>;
type WorkerFn<ES> = Box<dyn Fn(WorkerContext<ES>) -> Result<WorkerFuture, Error>>;

struct WorkerContext<'a, ES: EventStore> {
    consumer_config: &'a ClientConfig,
    event_store: &'a ES,
    on_error: &'a Option<for<'r> fn(Error)>,
    shutdown_recv: &'a Receiver<()>,
}

pub struct App<ES: EventStore> {
    consumer_config: ClientConfig,
    event_store: ES,
    on_error: Option<for<'r> fn(Error)>,
    #[cfg(feature = "outbox_relay")]
    redpanda_host: String,
    workers: Vec<WorkerFn<ES>>,
}

impl<ES> App<ES>
where
    ES: EventStore + Clone + Send + Sync + Unpin + 'static,
{
    pub fn build(event_store: ES, redpanda_host: impl Into<String> + Clone) -> AppBuilder<ES> {
        AppBuilder::new(event_store, redpanda_host)
    }

    pub async fn run(&mut self) -> Result<(), Error> {
        let (shutdown_send, shutdown_recv) = watch::channel(());
        let mut workers = FuturesUnordered::new();

        for worker in &self.workers {
            let ctx = WorkerContext {
                consumer_config: &self.consumer_config,
                event_store: &self.event_store,
                on_error: &self.on_error,
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

    #[cfg(feature = "outbox_relay")]
    pub async fn run_with_outbox_relay<Tls>(
        &mut self,
        pool: Pool<PostgresConnectionManager<Tls>>,
        database_url: &str,
        slot: &str,
    ) -> Result<(), Error>
    where
        Tls: MakeTlsConnect<Socket> + Clone + Send + Sync + 'static,
        <Tls as MakeTlsConnect<Socket>>::Stream: Send + Sync,
        <Tls as MakeTlsConnect<Socket>>::TlsConnect: Send,
        <<Tls as MakeTlsConnect<Socket>>::TlsConnect as TlsConnect<Socket>>::Future: Send,
    {
        use tokio::select;

        let (shutdown_send, shutdown_recv) = watch::channel(());
        let mut workers = FuturesUnordered::new();

        for worker in &self.workers {
            let ctx = WorkerContext {
                consumer_config: &self.consumer_config,
                event_store: &self.event_store,
                on_error: &self.on_error,
                shutdown_recv: &shutdown_recv,
            };
            workers.push(worker(ctx)?.boxed());
        }

        actix::spawn(async move { while workers.next().await.is_some() {} });

        let database_url = database_url.to_string();
        let redpanda_host = self.redpanda_host.clone();
        let slot = slot.to_string();
        let outbox_join_handle = tokio::spawn(async move {
            outbox_relay::outbox_relay(pool, &database_url, redpanda_host, &slot).await
        });

        select! {
            res = outbox_join_handle => { res??; },
            _ = signal::ctrl_c() => {},
        };

        shutdown_send.send(())?;
        tokio::time::sleep(Duration::from_millis(500)).await;

        Ok(())
    }

    #[cfg(feature = "outbox_relay")]
    pub async fn run_with_outbox_relay_from_stringlike<Tls>(
        &mut self,
        conn: &str,
        tls: Tls,
        slot: &str,
    ) -> Result<(), Error>
    where
        Tls: MakeTlsConnect<Socket> + Clone + Send + Sync + 'static,
        <Tls as MakeTlsConnect<Socket>>::Stream: Send + Sync,
        <Tls as MakeTlsConnect<Socket>>::TlsConnect: Send,
        <<Tls as MakeTlsConnect<Socket>>::TlsConnect as TlsConnect<Socket>>::Future: Send,
    {
        let manager = PostgresConnectionManager::new_from_stringlike(conn, tls)?;
        let pool = Pool::builder().build(manager).await?;

        self.run_with_outbox_relay(pool, conn, slot).await
    }
}

pub struct AppBuilder<ES>
where
    ES: EventStore + Clone + Send + Sync + Unpin + 'static,
{
    consumer_config: ClientConfig,
    event_store: ES,
    on_error: Option<fn(Error)>,
    #[cfg(feature = "outbox_relay")]
    redpanda_host: String,
    workers: Vec<WorkerFn<ES>>,
}

impl<ES> AppBuilder<ES>
where
    ES: EventStore + Clone + Send + Sync + Unpin + 'static,
{
    pub fn new(event_store: ES, redpanda_host: impl Into<String> + Clone) -> AppBuilder<ES> {
        let mut consumer_config = ClientConfig::new();
        consumer_config
            .set("group.id", env!("CARGO_PKG_NAME"))
            .set("allow.auto.create.topics", "true")
            .set("enable.auto.commit", "true")
            .set("auto.offset.reset", "earliest")
            .set_log_level(RDKafkaLogLevel::Debug);
        #[cfg(not(feature = "outbox_relay"))]
        consumer_config.set("bootstrap.servers", redpanda_host);
        #[cfg(feature = "outbox_relay")]
        consumer_config.set("bootstrap.servers", redpanda_host.clone());

        Self {
            consumer_config,
            event_store,
            on_error: None,
            #[cfg(feature = "outbox_relay")]
            redpanda_host: redpanda_host.into(),
            workers: Vec::new(),
        }
    }

    pub fn build(self) -> App<ES> {
        App {
            consumer_config: self.consumer_config,
            event_store: self.event_store,
            on_error: self.on_error,
            #[cfg(feature = "outbox_relay")]
            redpanda_host: self.redpanda_host,
            workers: self.workers,
        }
    }

    pub fn on_error(mut self, cb: fn(Error)) -> Self {
        self.on_error = Some(cb);
        self
    }

    pub fn aggregate<A>(self, cache_cap: usize) -> Self
    where
        A: Aggregate + Clone + Unpin + 'static,
        <A as Aggregate>::Command: actix::Message<Result = Result<Vec<EventEnvelope<<A as Aggregate>::Event>>, Error>>
            + StreamTopic
            + Clone
            + Unpin,
        <A as Aggregate>::Event: StreamTopic + Unpin,
    {
        self.aggregate_actor::<BaseAggregateActor<ES, A>, A>(cache_cap)
    }

    pub fn aggregate_actor<Act, A>(mut self, cache_cap: usize) -> Self
    where
        Act: AggregateActor<ES, A> + Clone,
        A: Aggregate + Clone + Unpin + 'static,
        <A as Aggregate>::Command: actix::Message<Result = Result<Vec<EventEnvelope<<A as Aggregate>::Event>>, Error>>
            + StreamTopic
            + Clone
            + Unpin
            + 'static,
        <A as Aggregate>::Event: StreamTopic + Unpin,
        <Act as Actor>::Context: ToEnvelope<Act, <A as Aggregate>::Command>,
    {
        self.workers.push(Box::new(move |ctx| {
            let event_store = ctx.event_store.to_owned();
            let on_error = ctx.on_error.to_owned();

            let agg_actor_pool = AggregateActorPool::<Act, _, _>::new(event_store, cache_cap);

            let topic = <A as Aggregate>::Command::stream_topic();

            let consumer: Arc<StreamConsumer> = Arc::new(ctx.consumer_config.create().map_err(Error::CreateConsumerError)?);
            consumer.fetch_metadata(Some(topic), Duration::from_secs(10)).map_err(Error::FetchTopicMetadataError)?; // Creates the topic if it doesn't exist
            consumer.subscribe(&[topic]).map_err(Error::SubscribeTopicError)?;
            debug!(
                topic = topic,
                "subscribed to command topic"
            );

            {
                let mut shutdown_recv = ctx.shutdown_recv.clone();
                let consumer = Arc::clone(&consumer);
                tokio::spawn(async move {
                    while shutdown_recv.changed().await.is_ok() {
                        consumer.unsubscribe();
                        debug!(
                            topic,
                            "unsubscribed from command topic"
                        )
                    }
                });
            }

            Ok(async move {
                let agg_actor_pool = agg_actor_pool.clone();

                loop_result_async(on_error, || async {
                    let agg_actor_pool = agg_actor_pool.clone();
                    let msg = consumer
                        .recv()
                        .await
                        .map_err(Error::RecieveMessageError)?;
                    let msg_type = msg.headers().and_then(|headers| {
                        for i in 0..headers.count() {
                            if let Some((k, v)) = headers.get(i) {
                                if k == "type" {
                                    return Some(v)
                                }
                            }
                        }
                        None
                    }).and_then(|msg_type| std::str::from_utf8(msg_type).ok());
                    info!(?msg_type, "message type");
                    let topic = msg.topic();
                    let offset = msg.offset();
                    let key_dbg = msg
                        .key_view::<str>()
                        .transpose()
                        .ok()
                        .flatten()
                        .unwrap_or_default();
                    let payload_dbg = msg.payload_view::<str>().transpose().ok().flatten().unwrap_or("");
                    trace!(%topic, offset, key = key_dbg, payload = payload_dbg, "received message");
                    let key = msg
                        .key_view::<str>()
                        .transpose()
                        .map_err(Error::MessageKeyUtf8Error)?
                        .ok_or(Error::MessageKeyMissing)?;

                    if let Some(payload) = msg.payload() {
                        let command = serde_json::from_slice::<<A as Aggregate>::Command>(payload)
                            .map_err(Error::MessageJsonDeserializeError)?;
                        debug!(topic, offset, key, ?command, "received command");

                        agg_actor_pool.handle(key, command).await?;
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
        <P as EventHandler>::Event: MultiStreamTopic + fmt::Debug,
    {
        let projection_type = P::projection_type();

        self.workers.push(Box::new(move |ctx| {
            let event_store = ctx.event_store.to_owned();
            let on_error = ctx.on_error.to_owned();
            let mut projection = projection.clone();

            let mut consumer_config = ctx.consumer_config.clone();
            consumer_config.set("group.id", projection_type);
            let consumer: Arc<StreamConsumer> = Arc::new(consumer_config.create().map_err(Error::CreateConsumerError)?);
            let topics = <P as EventHandler>::Event::stream_topics();
            for topic in &topics {
                consumer.fetch_metadata(Some(*topic), Duration::from_secs(10)).map_err(Error::FetchTopicMetadataError)?; // Creates the topic if it doesn't exist
            }
            consumer.subscribe(&topics).map_err(Error::SubscribeTopicError)?;
            debug!(
                projection = projection_type,
                ?topics,
                "subscribed to event topics"
            );

            {
                let mut shutdown_recv = ctx.shutdown_recv.clone();
                let consumer = Arc::clone(&consumer);
                tokio::spawn(async move {
                    while shutdown_recv.changed().await.is_ok() {
                        consumer.unsubscribe();
                        debug!(
                            projection = projection_type,
                            ?topics,
                            "unsubscribed from event topic"
                        )
                    }
                });
            }

            Ok(async move {
                if let Err(err) = event_store.resync_projection(&mut projection).await {
                    if let Some(on_error) = on_error {
                        on_error(err);
                    }
                }

                loop_result_async(on_error, || async {
                    let msg = consumer
                        .recv()
                        .await
                        .map_err(Error::RecieveMessageError)?;
                    let topic = msg.topic();
                    let offset = msg.offset();
                    let key_dbg = msg
                        .key_view::<str>()
                        .transpose()
                        .ok()
                        .flatten()
                        .unwrap_or("");
                    let payload_dbg = msg.payload_view::<str>().transpose().ok().flatten().unwrap_or("");
                    trace!(projection = projection_type, %topic, offset, key = key_dbg, payload = payload_dbg, "received message");
                    let key = msg
                        .key_view::<str>()
                        .transpose()
                        .map_err(Error::MessageKeyUtf8Error)?
                        .ok_or(Error::MessageKeyMissing)?;

                    if let Some(payload) = msg.payload() {
                        let event_envelope: EventEnvelope<<P as EventHandler>::Event> =
                            serde_json::from_slice(payload).map_err(Error::MessageJsonDeserializeError)?;
                        let event_id = event_envelope.id;
                        let event_sequence = event_envelope.sequence;
                        let event = event_envelope.event;
                        debug!(
                            projection = projection_type,
                            topic,
                            offset,
                            key,
                            event_id,
                            event_sequence,
                            ?event,
                            "received event"
                        );


                        let mut projection = projection.clone();
                        let view = projection.handle(key.to_string(), event, event_id, event_sequence).await?;
                        projection
                            .commit(key, view, event_id, event_sequence)
                            .await?;

                        trace!(projection = projection_type, key, event_id, "handled projectoion");
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

pub async fn loop_result_async<Fut, F>(on_error: Option<fn(Error)>, f: F)
where
    Fut: Future<Output = Result<(), Error>>,
    F: Fn() -> Fut,
{
    loop {
        if let Err(err) = f().await {
            if let Some(on_error) = on_error {
                on_error(err)
            }
        }
    }
}
