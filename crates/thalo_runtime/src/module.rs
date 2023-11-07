pub mod wit_aggregate;

use std::borrow::Cow;
use std::ops::DerefMut;
use std::path::Path;
use std::sync::Arc;
use std::{fmt, str};

use anyhow::{anyhow, bail, Context as AnyhowContext, Result};
use semver::Version;
use serde::{Deserialize, Serialize};
use thalo::Context;
use tokio::sync::Mutex;
use tracing::{info, trace};
// use wasi_common::{Table, WasiCtx, WasiCtxBuilder, WasiView};
use wasmtime::component::{Component, Linker, ResourceAny};
use wasmtime::{Engine, Store};
use wasmtime_wasi::preview2::{command, Stdout, Table, WasiCtx, WasiCtxBuilder, WasiView};

use self::wit_aggregate::Aggregate;

#[derive(Clone)]
pub struct Module {
    // TODO: This Arc shouldn't be necessary, but `wasmtime::component::bindgen` doesn't generate
    // Clone implementations.
    aggregate: Arc<Aggregate>,
    store: Arc<Mutex<Store<CommandCtx>>>,
}

#[derive(Clone)]
pub struct ModuleInstance {
    aggregate: Arc<Aggregate>,
    store: Arc<Mutex<Store<CommandCtx>>>,
    resource: ResourceAny,
    sequence: Option<u64>,
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

// #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
// pub enum ExecuteResult {
//     Events(Vec<Event<'static>>),
//     Ignored(Option<String>),
// }

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Event<'a> {
    // pub ctx: Context<'a>,
    pub event: Cow<'a, str>,
    pub payload: Cow<'a, str>,
}

pub struct CommandCtx {
    table: Table,
    wasi: WasiCtx,
    // sockets: WasiSocketsCtx,
}

impl Module {
    pub async fn from_file<T>(engine: Engine, file: T) -> Result<Self>
    where
        T: AsRef<Path> + fmt::Debug,
    {
        let table = Table::new();
        let wasi = WasiCtxBuilder::new().stdout(Stdout).build();
        let mut store = Store::new(&engine, CommandCtx { table, wasi });
        let component = Component::from_file(&engine, &file).unwrap();
        let mut linker = Linker::new(&engine);
        command::add_to_linker(&mut linker)?;

        let (aggregate, _instance) =
            wit_aggregate::Aggregate::instantiate_async(&mut store, &component, &linker)
                .await
                .context("failed to instantiate module")?;

        info!(?file, "loaded module from file");

        Ok(Module {
            aggregate: Arc::new(aggregate),
            store: Arc::new(Mutex::new(store)),
        })
    }

    pub async fn init(&self, id: &str) -> Result<ModuleInstance> {
        let resource = {
            let mut store = self.store.lock().await;
            self.aggregate
                .aggregate()
                .agg()
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

    pub async fn apply(&mut self, events: &[(Context<'_>, Event<'_>)]) -> Result<()> {
        if events.is_empty() {
            return Ok(());
        }

        // let metadatas = events
        //     .iter()
        //     .map(|(ctx, event)| serde_json::to_string(&ctx.metadata))
        //     .collect::<Result<Vec<_>, _>>()?;
        // let event_ctxs: Vec<_> = events
        //     .iter()
        //     .zip(metadatas.iter())
        //     .map(|((ctx, event), metadata)| {
        //         (
        //             // wit_aggregate::ContextParam::from((&ctx, metadata.as_str())),
        //             event,
        //         )
        //     })
        //     .collect();
        // let events: Vec<_> = event_ctxs
        //     .into_iter()
        //     .map(|(ctx, event)| wit_aggregate::EventParam {
        //         // ctx,
        //         event: &event.event,
        //         payload: &event.payload,
        //     })
        //     .collect();

        // Validate event positions
        let original_sequence = self.sequence;
        let events: Vec<_> = events
            .into_iter()
            .map(|(ctx, event)| {
                match self.sequence {
                    Some(seq) if seq + 1 == ctx.position => {
                        // Apply the event as the sequence is correct.
                        self.sequence = Some(ctx.position); // Update sequence.
                    }
                    None if ctx.position == 0 => {
                        // This is the first event, so apply it and set the sequence to 1.
                        self.sequence = Some(0);
                    }
                    _ => bail!(
                        "wrong event position {}, expected {}",
                        ctx.position,
                        self.sequence.map(|s| s + 1).unwrap_or(0)
                    ),
                }

                Ok(wit_aggregate::EventParam {
                    event: &event.event,
                    payload: &event.payload,
                })
            })
            .collect::<Result<_>>()?;
        // let mut transformed_events = Vec::with_capacity(events.len());
        // for (ctx, event) in events {
        //     match self.sequence {
        //         Some(seq) if seq + 1 == ctx.position => {
        //             // Apply the event as the sequence is correct.
        //             self.sequence = Some(ctx.position); // Update sequence.
        //         }
        //         None if ctx.position == 0 => {
        //             // This is the first event, so apply it and set the sequence to 1.
        //             self.sequence = Some(0);
        //         }
        //         _ => bail!(
        //             "wrong event position {}, expected {}",
        //             ctx.position,
        //             self.sequence.map(|s| s + 1).unwrap_or(0)
        //         ),
        //     }
        // }

        let mut store = self.store.lock().await;
        let res = self
            .aggregate
            .aggregate()
            .agg()
            .call_apply(store.deref_mut(), self.resource, &events)
            .await
            .map(|res| res.map_err(anyhow::Error::from))
            .map_err(anyhow::Error::from);
        if let Err(err) | Ok(Err(err)) = res {
            self.sequence = original_sequence;
            return Err(err);
        }

        trace!("applied {} event(s)", events.len());

        Ok(())
    }

    pub async fn handle(&self, command: &str, payload: &str) -> Result<Vec<Event<'static>>> {
        // let metadata = serde_json::to_string(&ctx.metadata)?;
        let command = wit_aggregate::Command {
            // ctx: wit_aggregate::ContextParam::from((ctx, metadata.as_str())),
            command,
            payload,
        };

        let result = {
            let mut store = self.store.lock().await;
            self.aggregate
                .aggregate()
                .agg()
                .call_handle(store.deref_mut(), self.resource, command)
                .await?
        };
        match result {
            Ok(events) => events
                .into_iter()
                .map(Event::try_from)
                .collect::<Result<Vec<_>, _>>(),
            // Err(wit_aggregate::Error::Ignore(reason)) => {
            //     match &reason {
            //         Some(reason) => trace!("command ignored with reason: '{reason}'"),
            //         None => trace!("command ignored"),
            //     }
            //     Ok(ExecuteResult::Ignored(reason))
            // }
            Err(err) => Err(anyhow!(err)),
        }
    }

    // pub async fn handle_and_apply(
    //     &mut self,
    //     // ctx: &Context<'_>,
    //     command: &str,
    //     payload: &str,
    // ) -> Result<Vec<Event<'static>>> {
    //     let events = self.handle(command, payload).await?;
    //     self.apply(&events).await?;
    //     Ok(result)
    // }

    pub async fn resource_drop(&self) -> Result<()> {
        let mut store = self.store.lock().await;
        self.resource.resource_drop_async(store.deref_mut()).await?;
        Ok(())
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

// impl ExecuteResult {
//     pub fn events(&self) -> &[Event] {
//         match self {
//             ExecuteResult::Events(events) => events,
//             ExecuteResult::Ignored(_) => &[],
//         }
//     }
// }

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
