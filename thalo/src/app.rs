use std::{
    env, fmt,
    pin::Pin,
    sync::Arc,
    thread::{self, JoinHandle},
    time::Duration,
};

use actix::{dev::ToEnvelope, Actor, ArbiterHandle, System};
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
    ClientConfig, Message,
};
use tokio::{select, signal, sync::Notify};
use tracing::{debug, error, trace};

use crate::{
    Aggregate, AggregateActor, AggregateActorPool, AggregateChannel, BaseAggregateActor, Error,
    EventEnvelope, EventHandler, EventStore, MultiStreamTopic, Projection, SharedGlobal,
    StreamTopic,
};

type WorkerFuture = Pin<Box<dyn Future<Output = ()> + Send>>;
type WorkerFn<ES> = Box<dyn Fn(WorkerContext<ES>) -> Result<WorkerFuture, Error> + Send>;
type OutboxHandler = Pin<Box<dyn Future<Output = Result<(), Error>> + Send>>;

struct WorkerContext<'a, ES: EventStore> {
    arbiter: &'a ArbiterHandle,
    consumer_config: &'a ClientConfig,
    event_store: &'a ES,
    on_error: &'a Option<for<'r> fn(Error)>,
    shutdown_recv: &'a Arc<Notify>,
}

pub struct App<ES: EventStore> {
    consumer_config: ClientConfig,
    event_store: ES,
    on_error: Option<for<'e> fn(Error)>,
    #[cfg(feature = "outbox_relay")]
    redpanda_host: String,
    #[cfg(feature = "outbox_relay")]
    outbox_handler: Option<OutboxHandler>,
    workers: Vec<WorkerFn<ES>>,
}

impl<ES> App<ES>
where
    ES: EventStore + Clone + Send + Sync + Unpin + 'static,
{
    pub fn new(event_store: ES, redpanda_host: impl Into<String> + Clone) -> App<ES> {
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
            #[cfg(feature = "outbox_relay")]
            outbox_handler: None,
            workers: Vec::new(),
        }
    }

    /// Runs on the current Actix system.
    ///
    /// Suitable when using `#[actix::main]`.
    pub async fn run(self) -> Result<(), Error> {
        let sys = System::try_current().ok_or_else(|| {
            error!("Could not run the thalo app as no actix system was found.\n\nIf your app uses `#[tokio::main]`, try `.run_isolated()` or `.run_in_system()`.");
            Error::MissingActixSystem
        })?;
        self.run_in_system(&sys).await
    }

    /// Runs in a separate thread.
    ///
    /// Suitable when **not** using `#[actix::main]`.
    pub fn run_isolated(self) -> JoinHandle<Result<(), Error>> {
        thread::spawn(move || {
            System::new().block_on(async move { self.run_in_system(&System::current()).await })
        })
    }

    /// Runs in an existing Actix system.
    ///
    /// Suitable for when you have access to your own Actix system.
    #[allow(unused_mut)]
    pub async fn run_in_system(mut self, sys: &System) -> Result<(), Error> {
        let shutdown_send = Arc::new(Notify::new());
        let mut workers = FuturesUnordered::new();

        for worker in &self.workers {
            let ctx = WorkerContext {
                arbiter: sys.arbiter(),
                consumer_config: &self.consumer_config,
                event_store: &self.event_store,
                on_error: &self.on_error,
                shutdown_recv: &shutdown_send,
            };
            workers.push(worker(ctx)?.boxed());
        }

        actix::spawn(async move { while workers.next().await.is_some() {} });

        #[cfg(feature = "outbox_relay")]
        let outbox_handler = self.outbox_handler.take();
        #[cfg(not(feature = "outbox_relay"))]
        let outbox_handler: Option<OutboxHandler> = None;
        match outbox_handler {
            Some(outbox_handler) => {
                let outbox_join_handle = tokio::spawn(outbox_handler);

                select! {
                    res = outbox_join_handle => { res??; },
                    _ = signal::ctrl_c() => {},
                };
            }
            None => {
                signal::ctrl_c().await.ok();
            }
        }

        shutdown_send.notify_waiters();
        tokio::time::sleep(Duration::from_millis(500)).await;
        std::process::exit(0);
    }

    #[cfg(feature = "outbox_relay")]
    pub fn with_outbox_relay<Tls>(
        mut self,
        pool: Pool<PostgresConnectionManager<Tls>>,
        database_url: &str,
        slot: &str,
    ) -> App<ES>
    where
        Tls: MakeTlsConnect<Socket> + Clone + Send + Sync + 'static,
        <Tls as MakeTlsConnect<Socket>>::Stream: Send + Sync,
        <Tls as MakeTlsConnect<Socket>>::TlsConnect: Send,
        <<Tls as MakeTlsConnect<Socket>>::TlsConnect as TlsConnect<Socket>>::Future: Send,
    {
        let database_url = database_url.to_string();
        let redpanda_host = self.redpanda_host.clone();
        let slot = slot.to_string();
        self.outbox_handler = Some(Box::pin(
            async move {
                outbox_relay::outbox_relay(pool, &database_url, redpanda_host, &slot)
                    .await
                    .map_err(Error::OutboxRelayError)
            }
            .boxed(),
        ));

        self
    }

    #[cfg(feature = "outbox_relay")]
    pub async fn with_outbox_relay_from_stringlike<Tls>(
        self,
        conn: &str,
        tls: Tls,
        slot: &str,
    ) -> Result<App<ES>, Error>
    where
        Tls: MakeTlsConnect<Socket> + Clone + Send + Sync + 'static,
        <Tls as MakeTlsConnect<Socket>>::Stream: Send + Sync,
        <Tls as MakeTlsConnect<Socket>>::TlsConnect: Send,
        <<Tls as MakeTlsConnect<Socket>>::TlsConnect as TlsConnect<Socket>>::Future: Send,
    {
        let manager = PostgresConnectionManager::new_from_stringlike(conn, tls)?;
        let pool = Pool::builder().build(manager).await?;

        Ok(self.with_outbox_relay(pool, conn, slot))
    }

    pub fn on_error(mut self, cb: fn(Error)) -> Self {
        self.on_error = Some(cb);
        self
    }

    pub fn aggregate<A>(self, cache_cap: usize) -> Self
    where
        A: Aggregate
            + AggregateChannel<BaseAggregateActor<ES, A>, ES>
            + SharedGlobal<
                BaseAggregateActor<ES, A>,
                ES,
                Value = AggregateActorPool<BaseAggregateActor<ES, A>, ES, A>,
            > + Clone
            + Unpin
            + 'static,
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
        A: Aggregate
            + AggregateChannel<Act, ES>
            + SharedGlobal<Act, ES, Value = AggregateActorPool<Act, ES, A>>
            + Clone
            + Unpin
            + 'static,
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

            <A as SharedGlobal<Act, ES>>::set(AggregateActorPool::<Act, ES, A>::new_in_arbiter(ctx.arbiter.clone(), event_store, cache_cap));

            let topic = <A as Aggregate>::Command::stream_topic();

            let consumer: Arc<StreamConsumer> = Arc::new(ctx.consumer_config.create().map_err(Error::CreateConsumerError)?);
            consumer.fetch_metadata(Some(topic), Duration::from_secs(10)).map_err(Error::FetchTopicMetadataError)?; // Creates the topic if it doesn't exist
            consumer.subscribe(&[topic]).map_err(Error::SubscribeTopicError)?;
            debug!(
                topic = topic,
                "subscribed to command topic"
            );

            {
                let shutdown_recv = Arc::clone(ctx.shutdown_recv);
                let consumer = Arc::clone(&consumer);
                tokio::spawn(async move {
                    shutdown_recv.notified().await;
                    consumer.unsubscribe();
                    debug!(
                        topic,
                        "unsubscribed from command topic"
                    )
                });
            }

            Ok(async move {
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

                        A::send(key, command).await?;
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
                let shutdown_recv = Arc::clone(ctx.shutdown_recv);
                let consumer = Arc::clone(&consumer);
                tokio::spawn(async move {
                    shutdown_recv.notified().await;
                    consumer.unsubscribe();
                    debug!(
                        projection = projection_type,
                        ?topics,
                        "unsubscribed from event topic"
                    )
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
                        trace!(
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

                        trace!(projection = projection_type, key, event_id, "handled projection");
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
