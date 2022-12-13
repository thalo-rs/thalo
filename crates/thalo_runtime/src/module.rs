pub mod wit_aggregate;

use std::ops::DerefMut;
use std::path::Path;
use std::sync::Arc;
use std::{borrow, fmt, str};

use anyhow::{anyhow, bail, Context as AnyhowContext, Result};
use chrono::{TimeZone, Utc};
use derive_more::{Deref, DerefMut};
use esdl::schema::Schema;
use host::WasiCtx;
use semver::Version;
use serde::{Deserialize, Serialize};
use thalo::Context;
use tokio::sync::Mutex;
use tracing::trace;
use wasi_cap_std_sync::WasiCtxBuilder;
use wasmtime::component::{Component, Linker};
use wasmtime::{Engine, Store};

use self::wit_aggregate::{Aggregate, Command};

#[derive(Clone)]
pub struct Module {
    aggregate: Aggregate,
    id: ModuleID,
    store: Arc<Mutex<Store<WasiCtx>>>,
}

pub struct ModuleInstance {
    aggregate: Aggregate,
    id: ModuleID,
    state: Vec<u8>,
    store: Arc<Mutex<Store<WasiCtx>>>,
}

/// A module verified against a schema.
pub struct SchemaModule {
    schema: Schema,
    module: Vec<u8>,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ModuleID {
    pub name: ModuleName,
    pub version: Version,
}

#[derive(
    Clone, Debug, Deref, DerefMut, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize,
)]
pub struct ModuleName(String);

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecuteResult {
    Events(Vec<Event>),
    Ignored(Option<String>),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Event {
    pub ctx: Context,
    pub event_type: String,
    pub payload: Vec<u8>,
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize)]
pub struct EventRef<'a> {
    pub ctx: &'a Context,
    pub event_type: &'a str,
    pub payload: &'a [u8],
}

impl Module {
    pub async fn from_file<T>(engine: Engine, id: ModuleID, file: T) -> Result<Self>
    where
        T: AsRef<Path> + fmt::Debug,
    {
        let mut store = Store::new(&engine, WasiCtxBuilder::new().build());
        let component = Component::from_file(&engine, &file).unwrap();
        let mut linker = Linker::new(&engine);
        host::add_to_linker(&mut linker, |x| x)?;

        let (aggregate, _instance) =
            wit_aggregate::instantiate_async(&mut store, &component, &linker)
                .await
                .context("failed to instantiate module")?;

        trace!(?file, "loaded module from file");

        Ok(Module {
            aggregate,
            id,
            store: Arc::new(Mutex::new(store)),
        })
    }

    pub async fn from_binary(engine: Engine, id: ModuleID, binary: &[u8]) -> Result<Self> {
        let mut store = Store::new(&engine, WasiCtxBuilder::new().build());
        let component = Component::from_binary(&engine, binary).unwrap();
        let mut linker = Linker::new(&engine);
        host::add_to_linker(&mut linker, |x| x)?;

        let (aggregate, _instance) =
            wit_aggregate::instantiate_async(&mut store, &component, &linker)
                .await
                .context("failed to instantiate module")?;

        trace!("loaded module from binary");

        Ok(Module {
            aggregate,
            id,
            store: Arc::new(Mutex::new(store)),
        })
    }

    pub async fn init(&self, id: String) -> Result<ModuleInstance> {
        let aggregate = self.aggregate;
        let state = {
            let mut store = self.store.lock().await;
            aggregate.init(store.deref_mut(), &id).await??
        };

        trace!(%id, "initialized module");

        Ok(ModuleInstance {
            aggregate,
            id: self.id.clone(),
            state,
            store: Arc::clone(&self.store),
        })
    }
}

impl ModuleInstance {
    pub fn id(&self) -> &ModuleID {
        &self.id
    }

    pub async fn apply(&mut self, events: &[EventRef<'_>]) -> Result<()> {
        if events.is_empty() {
            return Ok(());
        }

        let event_ctxs: Vec<_> = events
            .iter()
            .map(|event| (wit_aggregate::ContextResult::from(event.ctx), event))
            .collect();
        let events: Vec<_> = event_ctxs
            .iter()
            .map(|(ctx, event)| wit_aggregate::EventParam {
                ctx: ctx.as_param(),
                event_type: event.event_type,
                payload: event.payload,
            })
            .collect();

        self.state = {
            let mut store = self.store.lock().await;
            self.aggregate
                .apply(store.deref_mut(), &self.state, &events)
                .await??
        };

        trace!("applied {} event(s)", events.len());

        Ok(())
    }

    pub async fn handle(
        &self,
        ctx: &Context,
        command_name: &str,
        payload: &[u8],
    ) -> Result<ExecuteResult> {
        let command = Command {
            command: command_name,
            payload,
        };

        let mut store = self.store.lock().await;
        let metadata = serde_json::to_vec(&ctx.metadata).unwrap();
        let ctx = self::wit_aggregate::ContextParam {
            id: &ctx.id.to_string(),
            stream_name: &ctx.stream_name.to_string(),
            position: ctx.position,
            global_position: ctx.global_position,
            metadata: &metadata,
            time: ctx.time.timestamp_millis(),
        };
        let result = self
            .aggregate
            .handle(store.deref_mut(), &self.state, ctx, command)
            .await?;
        match result {
            Ok(events) => events
                .into_iter()
                .map(Event::try_from)
                .collect::<Result<Vec<_>, _>>()
                .map(ExecuteResult::Events),
            Err(wit_aggregate::Error::Ignore(reason)) => {
                match &reason {
                    Some(reason) => trace!("command ignored with reason: '{reason}'"),
                    None => trace!("command ignored"),
                }
                Ok(ExecuteResult::Ignored(reason))
            }
            Err(err) => Err(anyhow!(err)),
        }
    }

    pub async fn handle_and_apply(
        &mut self,
        ctx: &Context,
        command: &str,
        payload: &[u8],
    ) -> Result<ExecuteResult> {
        let result = self.handle(ctx, command, payload).await?;
        let event_refs: Vec<_> = result.events().iter().map(Event::as_ref).collect();
        self.apply(&event_refs).await?;
        Ok(result)
    }
}

impl SchemaModule {
    pub fn new(schema: Schema, module: Vec<u8>) -> Result<Self> {
        // TODO: Verify schema with module
        Ok(SchemaModule { schema, module })
    }

    pub fn schema(&self) -> &Schema {
        &self.schema
    }

    pub fn module(&self) -> &[u8] {
        &self.module
    }

    pub fn into_inner(self) -> (Schema, Vec<u8>) {
        (self.schema, self.module)
    }
}

impl ModuleID {
    pub fn new(name: ModuleName, version: Version) -> Self {
        ModuleID { name, version }
    }
}

impl ModuleName {
    pub fn new<T>(name: T) -> Result<Self>
    where
        T: AsRef<str> + Into<String>,
    {
        if !name
            .as_ref()
            .chars()
            .all(|c| c.is_ascii_alphabetic() || c == '_')
        {
            return Err(anyhow!(
                "module names must contain only alphabetic and `_` characters: '{}'",
                name.as_ref()
            ));
        }

        Ok(ModuleName(name.into()))
    }

    pub fn into_inner(self) -> String {
        self.0
    }
}

impl str::FromStr for ModuleName {
    type Err = anyhow::Error;

    fn from_str(name: &str) -> Result<Self, Self::Err> {
        ModuleName::new(name)
    }
}

impl fmt::Display for ModuleName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl borrow::Borrow<str> for ModuleName {
    fn borrow(&self) -> &str {
        self.0.borrow()
    }
}

impl ExecuteResult {
    pub fn events(&self) -> &[Event] {
        match self {
            ExecuteResult::Events(events) => events,
            ExecuteResult::Ignored(_) => &[],
        }
    }
}

impl Event {
    pub fn as_ref(&self) -> EventRef {
        EventRef {
            ctx: &self.ctx,
            event_type: &self.event_type,
            payload: &self.payload,
        }
    }
}

impl TryFrom<wit_aggregate::EventResult> for Event {
    type Error = anyhow::Error;

    fn try_from(event: wit_aggregate::EventResult) -> Result<Self, Self::Error> {
        Ok(Event {
            ctx: event.ctx.as_context()?,
            event_type: event.event_type,
            payload: event.payload,
        })
    }
}

// impl<'a> From<EventRef<'a>> for wit_aggregate::EventParam<'a> {
//     fn from(event: EventRef<'a>) -> Self {
//         wit_aggregate::EventParam {
//             ctx: wit_aggregate::ContextResult::from(event.ctx).as_param(),
//             event_type: event.event_type,
//             payload: event.payload,
//         }
//     }
// }

impl wit_aggregate::ContextResult {
    pub fn as_context(&self) -> Result<Context> {
        let Some(time) = Utc.timestamp_opt(self.time / 1_000, (self.time % 1_000_000) as u32).single() else {
            bail!("invalid timestamp");
        };

        Ok(Context {
            id: self.id.parse()?,
            stream_name: self.stream_name.parse()?,
            position: self.position,
            global_position: self.global_position,
            metadata: serde_json::from_slice(&self.metadata)?,
            time,
        })
    }
}

impl From<&Context> for wit_aggregate::ContextResult {
    fn from(ctx: &Context) -> Self {
        wit_aggregate::ContextResult {
            id: ctx.id.to_string(),
            stream_name: ctx.stream_name.to_string(),
            position: ctx.position,
            global_position: ctx.global_position,
            metadata: serde_json::to_vec(&ctx.metadata).unwrap(),
            time: ctx.time.timestamp_millis(),
        }
    }
}
