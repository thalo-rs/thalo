pub mod wit_aggregate;

use std::borrow::Cow;
use std::ops::DerefMut;
use std::path::Path;
use std::sync::Arc;
use std::{fmt, str};

use anyhow::{anyhow, Context as AnyhowContext, Result};
use semver::Version;
use serde::{Deserialize, Serialize};
use thalo::Context;
use tokio::sync::Mutex;
use tracing::trace;
// use wasi_common::{Table, WasiCtx, WasiCtxBuilder, WasiView};
use wasmtime::component::{Component, Linker};
use wasmtime::{Engine, Store};
use wasmtime_wasi::preview2::{command, Table, WasiCtx, WasiCtxBuilder, WasiView};

use self::wit_aggregate::Aggregate;

#[derive(Clone)]
pub struct Module {
    // TODO: This Arc shouldn't be necessary, but `wasmtime::component::bindgen` doesn't generate
    // Clone implementations.
    aggregate: Arc<Aggregate>,
    module_id: ModuleID,
    store: Arc<Mutex<Store<CommandCtx>>>,
}

pub struct ModuleInstance {
    aggregate: Arc<Aggregate>,
    module_id: ModuleID,
    store: Arc<Mutex<Store<CommandCtx>>>,
    instance_id: String,
}

/// A module verified against a schema.
// pub struct SchemaModule {
//     schema: Schema,
//     module: Vec<u8>,
// }

#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ModuleID {
    pub name: String,
    pub version: Version,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecuteResult {
    Events(Vec<Event<'static>>),
    Ignored(Option<String>),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Event<'a> {
    pub ctx: Context<'a>,
    pub event: Cow<'a, str>,
    pub payload: Cow<'a, str>,
}

pub struct CommandCtx {
    table: Table,
    wasi: WasiCtx,
    // sockets: WasiSocketsCtx,
}

impl Module {
    pub async fn from_file<T>(engine: Engine, id: ModuleID, file: T) -> Result<Self>
    where
        T: AsRef<Path> + fmt::Debug,
    {
        let table = Table::new();
        let wasi = WasiCtxBuilder::new().build();
        let mut store = Store::new(&engine, CommandCtx { table, wasi });
        let component = Component::from_file(&engine, &file).unwrap();
        let mut linker = Linker::new(&engine);
        command::add_to_linker(&mut linker)?;

        let (aggregate, _instance) =
            wit_aggregate::Aggregate::instantiate_async(&mut store, &component, &linker)
                .await
                .context("failed to instantiate module")?;

        trace!(?file, "loaded module from file");

        Ok(Module {
            aggregate: Arc::new(aggregate),
            module_id: id,
            store: Arc::new(Mutex::new(store)),
        })
    }

    pub async fn from_binary(engine: Engine, id: ModuleID, binary: &[u8]) -> Result<Self> {
        let table = Table::new();
        let wasi = WasiCtxBuilder::new().build();
        let mut store = Store::new(&engine, CommandCtx { table, wasi });
        let component = Component::from_binary(&engine, binary).unwrap();
        let mut linker = Linker::new(&engine);
        command::add_to_linker(&mut linker)?;

        let (aggregate, _instance) =
            wit_aggregate::Aggregate::instantiate_async(&mut store, &component, &linker)
                .await
                .context("failed to instantiate module")?;

        trace!("loaded module from binary");

        Ok(Module {
            aggregate: Arc::new(aggregate),
            module_id: id,
            store: Arc::new(Mutex::new(store)),
        })
    }

    pub async fn init(&self, id: String) -> Result<ModuleInstance> {
        {
            let mut store = self.store.lock().await;
            self.aggregate
                .aggregate()
                .call_init(store.deref_mut(), &id)
                .await??
        };

        trace!(%id, "initialized module");

        Ok(ModuleInstance {
            aggregate: Arc::clone(&self.aggregate),
            module_id: self.module_id.clone(),
            instance_id: id,
            store: Arc::clone(&self.store),
        })
    }
}

impl ModuleInstance {
    pub fn id(&self) -> &ModuleID {
        &self.module_id
    }

    pub async fn apply(&self, events: &[Event<'_>]) -> Result<()> {
        if events.is_empty() {
            return Ok(());
        }

        let metadatas = events
            .iter()
            .map(|event| serde_json::to_string(&event.ctx.metadata))
            .collect::<Result<Vec<_>, _>>()?;
        let event_ctxs: Vec<_> = events
            .iter()
            .zip(metadatas.iter())
            .map(|(event, metadata)| {
                (
                    wit_aggregate::ContextParam::from((&event.ctx, metadata.as_str())),
                    event,
                )
            })
            .collect();
        let events: Vec<_> = event_ctxs
            .into_iter()
            .map(|(ctx, event)| wit_aggregate::EventParam {
                ctx,
                event: &event.event,
                payload: &event.payload,
            })
            .collect();

        let mut store = self.store.lock().await;
        self.aggregate
            .aggregate()
            .call_apply(store.deref_mut(), &self.instance_id, &events)
            .await??;

        trace!("applied {} event(s)", events.len());

        Ok(())
    }

    pub async fn handle(
        &self,
        ctx: &Context<'_>,
        command: &str,
        payload: &str,
    ) -> Result<ExecuteResult> {
        let metadata = serde_json::to_string(&ctx.metadata)?;
        let command = wit_aggregate::Command {
            ctx: wit_aggregate::ContextParam::from((ctx, metadata.as_str())),
            command,
            payload,
        };

        let result = {
            let mut store = self.store.lock().await;
            self.aggregate
                .aggregate()
                .call_handle(store.deref_mut(), &self.instance_id, command)
                .await?
        };
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
        ctx: &Context<'_>,
        command: &str,
        payload: &str,
    ) -> Result<ExecuteResult> {
        let result = self.handle(ctx, command, payload).await?;
        self.apply(result.events()).await?;
        Ok(result)
    }
}

// impl SchemaModule {
//     pub fn new(schema: Schema, module: Vec<u8>) -> Result<Self> {
//         // TODO: Verify schema with module
//         Ok(SchemaModule { schema, module })
//     }

//     pub fn schema(&self) -> &Schema {
//         &self.schema
//     }

//     pub fn module(&self) -> &[u8] {
//         &self.module
//     }

//     pub fn into_inner(self) -> (Schema, Vec<u8>) {
//         (self.schema, self.module)
//     }
// }

impl ModuleID {
    pub fn new(name: String, version: Version) -> Self {
        ModuleID { name, version }
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

// impl<'a> From<EventRef<'a>> for wit_aggregate::EventParam<'a> {
//     fn from(event: EventRef<'a>) -> Self {
//         wit_aggregate::EventParam {
//             ctx: wit_aggregate::ContextResult::from(event.ctx).as_param(),
//             event_type: event.event_type,
//             payload: event.payload,
//         }
//     }
// }

// impl wit_aggregate::ContextResult {
//     pub fn as_context(&self) -> Result<Context> {
//         let Some(time) = Utc
//             .timestamp_opt(self.time / 1_000, (self.time % 1_000_000) as u32)
//             .single()
//         else {
//             bail!("invalid timestamp");
//         };

//         Ok(Context {
//             id: self.id.parse()?,
//             stream_name: self.stream_name.parse()?,
//             position: self.position,
//             global_position: self.global_position,
//             metadata: serde_json::from_slice(&self.metadata)?,
//             time,
//         })
//     }
// }

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
