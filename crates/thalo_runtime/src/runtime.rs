use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use anyhow::{anyhow, Error, Result};
use dashmap::DashMap;
use futures::stream::FuturesUnordered;
use futures::Future;
use moka::future::Cache;
use serde_json::Value;
use thalo::event_store::EventStore;
use thalo::stream_name::StreamName;
use tokio::fs;
use tokio::sync::{oneshot, Notify};
use tokio_stream::StreamExt;
use tracing::{error, instrument, warn};
use wasmtime::{Engine, Trap};

use crate::module::Module;
use crate::stream::{spawn_stream, Execute, StreamHandle};

#[derive(Clone)]
pub struct Runtime<E: EventStore> {
    event_store: E,
    engine: Engine,
    modules: Arc<DashMap<String, ModuleData>>,
    streams: Cache<StreamName<'static>, StreamHandle<E>>,
    command_timeout: Duration,
    modules_path: PathBuf,
}

impl<E> Runtime<E>
where
    E: EventStore + Clone + Send + 'static,
    E::Event: Send,
    E::EventStream: Send + Unpin,
    E::Error: Send + Sync,
{
    pub async fn new(event_store: E, config: Config) -> Result<Self> {
        let engine = {
            let mut config = wasmtime::Config::new();
            config.async_support(true).wasm_component_model(true);
            Engine::new(&config)?
        };

        let modules = Arc::new(load_modules_in_dir(&engine, &config.modules_path).await?);
        let streams = Cache::builder()
            .max_capacity(config.streams_cache_size)
            .support_invalidation_closures()
            .build();

        Ok(Runtime {
            event_store,
            engine,
            modules,
            streams,
            command_timeout: config.command_timeout,
            modules_path: config.modules_path,
        })
    }

    pub fn event_store(&self) -> &E {
        &self.event_store
    }

    #[instrument(skip(self, payload))]
    pub async fn execute(
        &self,
        name: String,
        id: String,
        command: String,
        payload: Value,
        max_attempts: u8,
    ) -> Result<Result<Vec<E::Event>, Value>> {
        let Ok(stream_name) = StreamName::from_parts(name, Some(&id)) else {
            return Err(anyhow!("invalid name or id"));
        };

        let stream_res = {
            let module_data = {
                let Some(module_entry) = self.modules.get(stream_name.category()) else {
                    return Err(anyhow!(
                        "aggregate '{}' does not exist or is not running",
                        stream_name.category()
                    ));
                };

                module_entry.value().clone()
            };
            let was_paused =
                ModuleData::wait_if_paused(module_data.paused, module_data.notify).await;
            let module = if was_paused {
                let Some(module_entry) = self.modules.get(stream_name.category()) else {
                    return Err(anyhow!(
                        "aggregate '{}' does not exist or is not running",
                        stream_name.category()
                    ));
                };

                module_entry.module.clone()
            } else {
                module_data.module
            };

            self.streams
                .entry_by_ref(&stream_name)
                .or_try_insert_with(spawn_stream(&self.event_store, &stream_name, &module))
                .await
                .map_err(ArcError::Arc)
        };
        let stream = self
            .handle_trap(stream_res, stream_name.category())
            .await
            .map_err(|err| anyhow!("{err}"))?;

        let (reply, recv) = oneshot::channel();
        stream
            .value()
            .send_timeout(
                Execute {
                    command,
                    payload,
                    max_attempts,
                    reply,
                },
                self.command_timeout,
            )
            .await?;

        let res = recv.await?.map_err(ArcError::Error);
        self.handle_trap(res, stream_name.category()).await
    }

    pub async fn save_module(&self, name: String, module: impl AsRef<[u8]>) -> Result<()> {
        let path = self.modules_path.join(&format!("{name}.wasm"));
        fs::write(&path, module).await?;

        let module = Module::from_file(self.engine.clone(), path).await?;
        self.modules.insert(name, ModuleData::new(module));

        Ok(())
    }

    async fn handle_trap<T>(&self, res: Result<T, ArcError>, name: &str) -> Result<T, Error> {
        if let Err(err) = &res {
            if let Some(trap) = err.as_ref().root_cause().downcast_ref::<Trap>().copied() {
                error!("module {name} trapped: {trap}");
                // tokio::spawn({
                let runtime = self.clone();
                let name = name.to_string();
                // async move {
                if let Err(err) = runtime.restart_module(name).await {
                    error!("failed to restart module: {err}");
                }
                // }
                // });

                return Err(anyhow!("{trap}"));
            }
        }

        Ok(res?)
    }

    async fn restart_module(&self, name: String) -> Result<()> {
        if let Some(module_data) = self.modules.get(&name) {
            let pause_guard = module_data.pause();
            let streams = self.streams.clone();
            tokio::spawn({
                let modules = Arc::clone(&self.modules);
                async move {
                    let _guard = pause_guard;

                    // Invalidate streams for this module
                    streams.invalidate_entries_if({
                        let name = name.clone();
                        move |k, _| k.category() == name
                    })?;
                    streams.run_pending_tasks().await;

                    if let Some(mut module_data) = modules.get_mut(&name) {
                        module_data.module.reinitialize().await?;
                    }

                    Result::<_, Error>::Ok(())
                }
            });

            // module_data
            //     .value_mut()
            //     .execute_paused(move |module| {
            //         let name = name.to_string();
            //         async move {
            //             // Invalidate streams for this module
            //             self.streams
            //                 .invalidate_entries_if(move |k, _| k.category()
            // == name)?;
            // self.streams.run_pending_tasks().await;

            //             module.reinitialize().await?;
            //             println!("reinitialized... {}", module.count);

            //             Result::<_, anyhow::Error>::Ok(())
            //         }
            //     })
            //     .await?;
        }

        Ok(())
    }
}

enum ArcError {
    Arc(Arc<Error>),
    Error(Error),
}

impl From<ArcError> for Error {
    fn from(err: ArcError) -> Self {
        match err {
            ArcError::Arc(err) => anyhow!(err),
            ArcError::Error(err) => err,
        }
    }
}

impl AsRef<Error> for ArcError {
    fn as_ref(&self) -> &Error {
        match self {
            ArcError::Arc(err) => &err,
            ArcError::Error(err) => &err,
        }
    }
}

trait AsTrap {
    fn as_trap(&self) -> Option<Trap>;
}

impl AsTrap for Error {
    fn as_trap(&self) -> Option<Trap> {
        self.root_cause().downcast_ref().copied()
    }
}

impl AsTrap for Arc<Error> {
    fn as_trap(&self) -> Option<Trap> {
        self.root_cause().downcast_ref().copied()
    }
}

#[derive(Clone, Debug)]
pub struct Config {
    command_timeout: Duration,
    modules_path: PathBuf,
    streams_cache_size: u64,
}

impl Config {
    /// Creates a new configuration object with the default configuration
    /// specified.
    pub fn new() -> Self {
        Config {
            command_timeout: Duration::from_secs(5),
            modules_path: "./modules".into(),
            streams_cache_size: 10_000,
        }
    }

    pub fn command_timeout(mut self, timeout: Duration) -> Self {
        self.command_timeout = timeout;
        self
    }

    /// Specifies the directory containing the aggregate wasm modules.
    ///
    /// Defaults to ./modules
    pub fn modules_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.modules_path = path.into();
        self
    }

    /// Sets the maximum number of streams to be cached.
    ///
    /// The cache is used to avoid querying the event store to replay events
    /// when a command is executed. When a stream is not in the cache, all
    /// events will be replayed from the event store to rebuild the aggregate
    /// state.
    ///
    /// Defaults to 10,000
    pub fn streams_cache_size(mut self, size: u64) -> Self {
        self.streams_cache_size = size;
        self
    }
}

#[derive(Clone)]
struct ModuleData {
    module: Module,
    paused: Arc<AtomicBool>,
    notify: Arc<Notify>,
}

impl ModuleData {
    fn new(module: Module) -> Self {
        ModuleData {
            module,
            paused: Arc::new(AtomicBool::new(false)),
            notify: Arc::new(Notify::new()),
        }
    }

    fn pause(&self) -> PauseGuard {
        PauseGuard::new(self.paused.clone(), self.notify.clone())
    }

    // async fn execute_paused<'a, T, F, Fu>(&'a mut self, mut f: F) -> T
    // where
    //     F: FnMut(&'a mut Module) -> Fu + 'a,
    //     Fu: Future<Output = T>,
    // {
    //     self.paused.store(true, Ordering::Relaxed);

    //     let res = f(&mut self.module).await;

    //     self.paused.store(false, Ordering::Relaxed);
    //     self.notify.notify_waiters();

    //     res
    // }

    async fn wait_if_paused(paused: Arc<AtomicBool>, notify: Arc<Notify>) -> bool {
        let mut was_paused = false;
        loop {
            let notified = notify.notified();
            if !paused.load(Ordering::Acquire) {
                break;
            }
            was_paused = true;
            notified.await;
        }
        was_paused
    }
}

struct PauseGuard {
    paused: Arc<AtomicBool>,
    notify: Arc<Notify>,
}

impl PauseGuard {
    fn new(paused: Arc<AtomicBool>, notify: Arc<Notify>) -> Self {
        paused.store(true, Ordering::Relaxed);

        PauseGuard { paused, notify }
    }
}

impl Drop for PauseGuard {
    fn drop(&mut self) {
        self.paused.store(false, Ordering::Relaxed);
        self.notify.notify_waiters();
    }
}

async fn load_modules_in_dir(
    engine: &Engine,
    path: impl AsRef<Path>,
) -> Result<DashMap<String, ModuleData>> {
    let mut read_dir = fs::read_dir(path).await?;
    let tasks = FuturesUnordered::new();

    while let Some(dir_entry) = read_dir.next_entry().await? {
        let Ok(file_name) = dir_entry.file_name().into_string() else {
            warn!("ignoring module with invalid file name");
            continue;
        };
        let Some((module_name, suffix)) = file_name.split_once('.') else {
            warn!("ignoring module {file_name}");
            continue;
        };
        if suffix != "wasm" {
            warn!("ignoring module {file_name} - does not end in .wasm");
            continue;
        }

        let module_name = module_name.to_string();
        let path = dir_entry.path();
        let engine = engine.clone();
        tasks.push(async move {
            let module = Module::from_file(engine, path).await?;
            let module_data = ModuleData::new(module);
            Result::<_, anyhow::Error>::Ok((module_name, module_data))
        });
    }

    let modules: Vec<_> = StreamExt::collect::<Result<_>>(tasks).await?;
    Ok(modules.into_iter().collect())
}
