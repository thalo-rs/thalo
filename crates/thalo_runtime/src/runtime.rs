use std::collections::BTreeMap;
use std::sync::Arc;

use anyhow::{anyhow, bail, Context as AnyhowContext, Result};
use futures::StreamExt;
use message_db::database::{MessageStore, SubscribeToCategoryOpts, WriteMessageOpts};
use message_db::message::MessageData;
use message_db::stream_name::{Category, StreamName};
use semver::VersionReq;
use serde_json::Value;
use thalo::Context;
use thalo_registry::Registry as RegistryStore;
use tokio::sync::RwLock;
use tracing::{error, info, instrument, trace, warn};
use wasmtime::Engine;

use crate::command::CommandRouter;
use crate::module::{ExecuteResult, Module, ModuleID, ModuleName, SchemaModule};
use crate::registry::Registry;

#[derive(Clone)]
pub struct Runtime {
    engine: Engine,
    modules: Arc<RwLock<BTreeMap<ModuleID, Arc<Module>>>>,
    registry: Arc<RwLock<Registry>>,
    command_router: CommandRouter,
    message_store: MessageStore,
    registry_store: RegistryStore,
}

impl Runtime {
    pub fn new(message_store: MessageStore, registry_store: RegistryStore) -> Self {
        let mut config = wasmtime::Config::new();
        config.async_support(true).wasm_component_model(true);
        let engine = Engine::new(&config).unwrap();

        Runtime {
            engine,
            modules: Arc::new(RwLock::new(BTreeMap::new())),
            registry: Arc::new(RwLock::new(Registry::default())),
            command_router: CommandRouter::start(),
            message_store,
            registry_store,
        }
    }

    pub fn message_store(&self) -> &MessageStore {
        &self.message_store
    }

    pub async fn init(&self) -> Result<()> {
        let modules = self
            .registry_store
            .load_all_schema_modules()
            .await
            .context("failed to load all modules from registry")?;
        for module in modules {
            let module_id = ModuleID::new(module.name.parse()?, module.version);
            let mut registry = self.registry.write().await;
            registry.add_module(module_id, module.module);
        }

        Ok(())
    }

    pub async fn start(&self) {
        for module_name in self.registry.read().await.modules.keys().cloned() {
            self.start_module(module_name);
        }
    }

    fn start_module(&self, module_name: ModuleName) {
        let runtime = self.clone();
        let category_name = format!("{}:command", Category::normalize(&module_name.to_string()));

        trace!("subscribing to category '{category_name}'");
        tokio::spawn(async move {
            let mut stream = MessageStore::subscribe_to_category::<MessageData, _>(
                &runtime.message_store,
                &category_name,
                &SubscribeToCategoryOpts::builder()
                    .identifier("thalo_runtime")
                    .build(),
            )
            .await
            .unwrap();
            while let Some(batch) = stream.next().await {
                match batch {
                    Ok(commands) => {
                        for command in commands {
                            let id = match &command.stream_name.id {
                                Some(id) => id.cardinal_id().to_string(),
                                None => {
                                    warn!(command_id = ?command.id, "missing id from command");
                                    continue;
                                }
                            };

                            trace!(
                                command_id = ?command.id,
                                command_type = %command.msg_type,
                                entity_name = %command.stream_name.category.entity_name,
                                entity_id = %id,
                                "handling command"
                            );

                            let ctx = Context {
                                id: command.id,
                                stream_name: command.stream_name,
                                position: command.position,
                                global_position: command.global_position,
                                metadata: command.metadata,
                                time: command.time,
                            };

                            let _ = runtime
                                .execute(
                                    module_name.clone(),
                                    id,
                                    ctx,
                                    command.msg_type.clone(),
                                    command.data,
                                )
                                .await;
                        }
                    }
                    Err(err) => {
                        error!(stream = %category_name, "failed to read command: {err}");
                    }
                }
            }
        });
    }

    #[instrument(skip(self, ctx, payload))]
    pub async fn execute(
        &self,
        name: ModuleName,
        id: String,
        ctx: Context,
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
        name: &ModuleName,
        id: &str,
        command: &str,
        data: &Value,
    ) -> Result<i64> {
        let category = Category::new(Category::normalize(name), vec!["command".to_string()])?;
        let stream_name = StreamName {
            category,
            id: Some(id.parse()?),
        };

        let position = MessageStore::write_message(
            &self.message_store,
            &stream_name.to_string(),
            command,
            data,
            &WriteMessageOpts::default(),
        )
        .await?;

        Ok(position)
    }

    pub async fn publish_module(&self, schema_module: SchemaModule) -> Result<()> {
        let mut registry = self.registry.write().await;

        let latest_version = self
            .registry_store
            .load_module_latest_version(&schema_module.schema().aggregate.name)
            .await?;
        if let Some(latest_version) = latest_version {
            if schema_module.schema().version <= latest_version {
                bail!("version must be greater than latest version {latest_version}");
            }
        }

        self.registry_store
            .save_schema_module(schema_module.schema(), schema_module.module())
            .await?;

        let (schema, module) = schema_module.into_inner();
        let module_name: ModuleName = schema.aggregate.name.parse()?;
        let id = ModuleID::new(module_name.clone(), schema.version);
        registry.add_module(id, module);

        self.start_module(module_name);

        Ok(())
    }

    pub async fn load_module(
        &self,
        name: &ModuleName,
        version: &VersionReq,
    ) -> Result<(ModuleID, Arc<Module>)> {
        if let Some((module_id, module)) = self.load_module_opt(name, version).await {
            return Ok((module_id, module));
        }

        let (module, module_id) = {
            let registry = self.registry.read().await;
            let (version, binary) = registry
                .get_module(name, version)
                .ok_or_else(|| anyhow!("module does not exist"))?;
            let module = Module::from_binary(
                self.engine.clone(),
                ModuleID::new(name.clone(), version.clone()),
                binary,
            )
            .await?;
            let module_id = ModuleID::new(name.clone(), version.clone());
            (module, module_id)
        };
        let mut modules = self.modules.write().await;
        let module = modules
            .entry(module_id.clone())
            .or_insert_with(|| Arc::new(module));

        Ok((module_id, Arc::clone(module)))
    }

    async fn load_module_opt(
        &self,
        name: &ModuleName,
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
