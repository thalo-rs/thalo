use std::borrow::Cow;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use anyhow::{anyhow, bail, Result};
// use message_db::database::{MessageStore, SubscribeToCategoryOpts, WriteMessageOpts};
// use message_db::message::MessageData;
// use message_db::stream_name::{Category, StreamName};
use semver::{Version, VersionReq};
use serde_json::Value;
use thalo::{Category, Context, Metadata, StreamName};
use thalo_message_store::message::MessageData;
use thalo_message_store::message_store::MessageStore;
use thalo_registry::Registry;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use tracing::{error, info, instrument, trace, warn};
use wasmtime::Engine;

use crate::command::CommandRouter;
use crate::module::{ExecuteResult, Module, ModuleID};

#[derive(Clone)]
pub struct Runtime {
    engine: Engine,
    modules: Arc<RwLock<BTreeMap<ModuleID, Arc<Module>>>>,
    command_router: CommandRouter,
    message_store: MessageStore,
    registry: Registry,
}

impl Runtime {
    pub fn new(message_store: MessageStore, registry: Registry) -> Self {
        let mut config = wasmtime::Config::new();
        config.async_support(true).wasm_component_model(true);
        let engine = Engine::new(&config).unwrap();

        Runtime {
            engine,
            modules: Arc::new(RwLock::new(BTreeMap::new())),
            command_router: CommandRouter::start(),
            message_store,
            registry,
        }
    }

    pub fn message_store(&self) -> &MessageStore {
        &self.message_store
    }

    pub async fn start(&self) -> Result<()> {
        for res in self.registry.iter_all_latest() {
            let (name, _version, _module) = res?;
            self.start_module(&name)?;
        }
        Ok(())
    }

    fn start_module(&self, module_name: &str) -> Result<JoinHandle<Result<()>>> {
        let runtime = self.clone();
        let category = Category::from_parts(Category::normalize(&module_name), &["command"])?;
        let stream_name = StreamName::from_parts(category, None)?;
        let stream = runtime.message_store.stream(stream_name.as_borrowed())?;
        let mut subscriber = stream.watch::<MessageData>();

        trace!("subscribing to category '{}'", stream_name.category());
        let handle = tokio::spawn(async move {
            while let Some(raw_message) = (&mut subscriber).await {
                match raw_message.message() {
                    Ok(message) => {
                        let id = match message.metadata.properties.get(&Cow::Borrowed("id")) {
                            Some(id) => match id.as_str() {
                                Some(id) => id,
                                None => {
                                    warn!(command_id = ?message.id, "command id is not a string");
                                    continue;
                                }
                            },
                            None => {
                                warn!(command_id = ?message.id, "missing id from command");
                                continue;
                            }
                        };
                        // let stream_id = message.stream_name.id();
                        // let id = match &stream_id {
                        //     Some(id) => id.cardinal_id(),
                        //     None => {
                        //         warn!(command_id = ?message.id, "missing id from command");
                        //         continue;
                        //     }
                        // };

                        trace!(
                            command_id = ?message.id,
                            command_type = %message.msg_type,
                            entity_name =
                                %message.stream_name.category().entity_name(),
                            entity_id = %id,
                            "handling command"
                        );

                        let name = stream_name.category().entity_name().to_string();
                        let id = id.to_string();
                        let ctx = Context {
                            id: message.id,
                            stream_name: message.stream_name.into_owned(),
                            position: message.position,
                            metadata: message.metadata.into_owned(),
                            time: message.time,
                        };

                        if let Err(err) = runtime
                            .execute(
                                name,
                                id,
                                ctx,
                                message.msg_type.into_owned(),
                                message.data.into_owned(),
                            )
                            .await
                        {
                            error!("{err}");
                        }
                    }
                    Err(err) => {
                        error!("{err}");
                    }
                }
            }

            Ok::<_, anyhow::Error>(())
        });

        Ok(handle)
    }

    #[instrument(skip(self, ctx, payload))]
    pub async fn execute(
        &self,
        name: String,
        id: String,
        ctx: Context<'static>,
        command: String,
        payload: Value,
    ) -> Result<ExecuteResult> {
        let result = self
            .command_router
            .execute(
                self.clone(),
                self.message_store.clone(),
                name,
                id,
                ctx,
                command,
                payload,
            )
            .await;

        match &result {
            Ok(ExecuteResult::Events(events)) => {
                info!("{} events saved", events.len());
            }
            Ok(ExecuteResult::Ignored(_)) => {}
            Err(err) => {
                error!("command failed: {err}");
            }
        }

        result
    }

    pub async fn submit_command(
        &self,
        name: &str,
        id: &str,
        command: &str,
        data: &Value,
    ) -> Result<u64> {
        let category = Category::from_parts(Category::normalize(name), &["command"])?;
        // let id = ID::new(id)?;
        let stream_name = StreamName::from_parts(category, None)?;
        let mut stream = self.message_store.stream(stream_name.as_borrowed())?;
        let position = stream
            .write_message(
                command,
                data,
                &Metadata {
                    stream_name: stream_name.as_borrowed(),
                    position: 0,
                    causation_message_stream_name: stream_name.as_borrowed(),
                    causation_message_position: 0,
                    reply_stream_name: None,
                    schema_version: None,
                    properties: HashMap::from_iter([(
                        Cow::Borrowed("id"),
                        Cow::Owned(Value::String(id.to_string())),
                    )]),
                },
                None,
            )
            .await?;

        Ok(position)
    }

    pub async fn publish_module(&self, name: &str, version: Version, module: &[u8]) -> Result<()> {
        let module_versions = self.registry.module(name)?;
        if let Some((latest_version, _)) = module_versions.get_latest()? {
            if version <= latest_version {
                bail!("version must be greater than latest version {latest_version}");
            }
        }

        module_versions.insert(version, module).await?;

        self.start_module(name)?;

        Ok(())
    }

    pub async fn load_module(
        &self,
        name: &str,
        version: &VersionReq,
    ) -> Result<(ModuleID, Arc<Module>)> {
        if let Some((module_id, module)) = self.load_module_opt(name, version).await {
            return Ok((module_id, module));
        }

        let module_versions = self.registry.module(name)?;
        let (version, binary) = module_versions
            .get(version)?
            .ok_or_else(|| anyhow!("module does not exist"))?;

        let module = Module::from_binary(
            self.engine.clone(),
            ModuleID::new(name.to_string(), version.clone()),
            &binary,
        )
        .await?;
        let module_id = ModuleID::new(name.to_string(), version);
        let mut modules = self.modules.write().await;
        let module = modules
            .entry(module_id.clone())
            .or_insert_with(|| Arc::new(module));

        Ok((module_id, Arc::clone(module)))
    }

    async fn load_module_opt(
        &self,
        name: &str,
        version: &VersionReq,
    ) -> Option<(ModuleID, Arc<Module>)> {
        let modules_read = self.modules.read().await;
        modules_read.iter().find_map(|(module_id, module)| {
            if &module_id.name == name && version.matches(&module_id.version) {
                Some((module_id.clone(), Arc::clone(module)))
            } else {
                None
            }
        })
    }
}
