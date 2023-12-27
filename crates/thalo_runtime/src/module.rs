pub mod wit_aggregate;

use std::borrow::Cow;
use std::ops::DerefMut;
use std::path::Path;
use std::sync::Arc;
use std::{fmt, str};

use anyhow::{anyhow, bail, Context as AnyhowContext, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::Mutex;
use tracing::{info, trace};
use tracing_tunnel::TracingEventReceiver;
use wasmtime::component::{Component, InstancePre, Linker, ResourceAny};
use wasmtime::{Engine, Store};
use wasmtime_wasi::preview2::{command, Stdout, Table, WasiCtx, WasiCtxBuilder, WasiView};

use self::wit_aggregate::Aggregate;
use crate::module::wit_aggregate::{tracing as wit_tracing, AggregateError};

#[derive(Clone)]
pub struct Module {
    // TODO: This Arc shouldn't be necessary, but `wasmtime::component::bindgen` doesn't generate
    // Clone implementations.
    aggregate: Arc<Aggregate>,
    engine: Engine,
    store: Arc<Mutex<Store<CommandCtx>>>,
    instance_pre: InstancePre<CommandCtx>,
}

#[derive(Clone)]
pub struct ModuleInstance {
    aggregate: Arc<Aggregate>,
    store: Arc<Mutex<Store<CommandCtx>>>,
    resource: ResourceAny,
    sequence: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Event<'a> {
    pub event: Cow<'a, str>,
    pub data: Cow<'a, str>,
}

pub struct CommandCtx {
    table: Table,
    tracing_subscriber: TracingSubscriber,
    wasi: WasiCtx,
}

impl CommandCtx {
    pub fn new() -> Self {
        let table = Table::new();
        let tracing_subscriber = TracingSubscriber(TracingEventReceiver::default());
        let wasi = WasiCtxBuilder::new().stdout(Stdout).build();
        CommandCtx {
            table,
            tracing_subscriber,
            wasi,
        }
    }
}

impl Default for CommandCtx {
    fn default() -> Self {
        CommandCtx::new()
    }
}

#[derive(Debug, Default)]
struct TracingSubscriber(TracingEventReceiver);

#[async_trait]
impl wit_tracing::Host for TracingSubscriber {
    async fn send_event(&mut self, event: Vec<u8>) -> wasmtime::Result<()> {
        let event = serde_json::from_slice(&event).map_err(wasmtime::Error::new)?;
        self.0.try_receive(event).map_err(wasmtime::Error::new)?;
        Ok(())
    }
}

impl Module {
    pub async fn new(engine: Engine, component: Component) -> Result<Self> {
        let ctx = CommandCtx::default();
        let mut store = Store::new(&engine, ctx);
        let mut linker: Linker<CommandCtx> = Linker::new(&engine);
        command::add_to_linker(&mut linker)?;
        wit_tracing::add_to_linker(&mut linker, |ctx| &mut ctx.tracing_subscriber)?;

        let instance_pre = linker.instantiate_pre(&component)?;
        let (aggregate, _instance) =
            wit_aggregate::Aggregate::instantiate_pre(&mut store, &instance_pre).await?;

        Ok(Module {
            aggregate: Arc::new(aggregate),
            engine,
            store: Arc::new(Mutex::new(store)),
            instance_pre,
        })
    }

    pub async fn from_file<T>(engine: Engine, file: T) -> Result<Self>
    where
        T: AsRef<Path> + fmt::Debug,
    {
        let component = Component::from_file(&engine, &file)?;
        let module = Module::new(engine, component).await?;

        info!(?file, "loaded module from file");

        Ok(module)
    }

    pub async fn reinitialize(&mut self) -> Result<()> {
        let ctx = CommandCtx::default();
        let mut store = Store::new(&self.engine, ctx);
        let (aggregate, _instance) =
            wit_aggregate::Aggregate::instantiate_pre(&mut store, &self.instance_pre).await?;

        self.aggregate = Arc::new(aggregate);
        self.store = Arc::new(Mutex::new(store));

        Ok(())
    }

    pub async fn init(&self, id: &str) -> Result<ModuleInstance> {
        let resource = {
            let mut store = self.store.lock().await;
            self.aggregate
                .aggregate()
                .entity()
                .call_constructor(store.deref_mut(), id)
                .await?
        };

        trace!(%id, "initialized module");

        Ok(ModuleInstance::new(
            Arc::clone(&self.store),
            Arc::clone(&self.aggregate),
            resource,
        ))
    }
}

impl ModuleInstance {
    pub fn new(
        store: Arc<Mutex<Store<CommandCtx>>>,
        aggregate: Arc<Aggregate>,
        resource: ResourceAny,
    ) -> Self {
        ModuleInstance {
            aggregate,
            store,
            resource,
            sequence: None,
        }
    }

    pub fn sequence(&self) -> Option<u64> {
        self.sequence
    }

    pub async fn apply(&mut self, events: &[(u64, Event<'_>)]) -> Result<()> {
        if events.is_empty() {
            return Ok(());
        }

        // Validate event positions
        let original_sequence = self.sequence;
        let events: Vec<_> = events
            .into_iter()
            .map(|(position, event)| {
                match self.sequence {
                    Some(seq) if seq + 1 == *position => {
                        // Apply the event as the sequence is correct.
                        self.sequence = Some(*position); // Update sequence.
                    }
                    None if *position == 0 => {
                        // This is the first event, so apply it and set the sequence to 1.
                        self.sequence = Some(0);
                    }
                    _ => bail!(
                        "wrong event position {position}, expected {}",
                        self.sequence.map(|s| s + 1).unwrap_or(0)
                    ),
                }

                Ok(wit_aggregate::EventParam {
                    event: &event.event,
                    data: &event.data,
                })
            })
            .collect::<Result<_>>()?;

        let mut store = self.store.lock().await;
        let res = self
            .aggregate
            .aggregate()
            .entity()
            .call_apply(store.deref_mut(), self.resource, &events)
            .await
            .map(|res| {
                res.map_err(AggregateError::from)
                    .map_err(anyhow::Error::from)
            })
            .map_err(anyhow::Error::from);
        if let Err(err) | Ok(Err(err)) = res {
            self.sequence = original_sequence;
            return Err(err);
        }

        trace!("applied {} event(s)", events.len());

        Ok(())
    }

    pub async fn handle(
        &self,
        command: &str,
        payload: &str,
    ) -> Result<Result<Vec<Event<'static>>, Value>> {
        let command = wit_aggregate::Command { command, payload };

        let result = {
            let mut store = self.store.lock().await;
            self.aggregate
                .aggregate()
                .entity()
                .call_handle(store.deref_mut(), self.resource, command)
                .await?
                .map_err(AggregateError::from)
        };
        match result {
            Ok(events) => Ok(Ok(events
                .into_iter()
                .map(Event::try_from)
                .collect::<Result<Vec<_>, _>>()?)),
            Err(AggregateError::Command { command, error }) => {
                Ok(Err(serde_json::from_str(&error).with_context(|| {
                    format!("failed to error returned from command '{command}'")
                })?))
            }
            Err(err) => Err(anyhow!(err)),
        }
    }

    pub async fn resource_drop(&self) -> Result<()> {
        let mut store = self.store.lock().await;
        self.resource.resource_drop_async(store.deref_mut()).await?;
        Ok(())
    }
}

impl WasiView for CommandCtx {
    fn table(&self) -> &Table {
        &self.table
    }

    fn table_mut(&mut self) -> &mut Table {
        &mut self.table
    }

    fn ctx(&self) -> &WasiCtx {
        &self.wasi
    }

    fn ctx_mut(&mut self) -> &mut WasiCtx {
        &mut self.wasi
    }
}
