use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use anyhow::{anyhow, Error, Result};
use dashmap::mapref::one::Ref;
use dashmap::DashMap;
use futures::stream::FuturesUnordered;
use moka::future::Cache;
use moka::Entry;
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

    #[instrument(skip(self))]
    pub async fn execute(
        &self,
        name: String,
        id: String,
        command: String,
        payload: Value,
        max_attempts: u8,
    ) -> Result<Result<Vec<E::Event>, Value>> {
        // let Ok(stream_name) = StreamName::from_parts(name, Some(&id)) else {
        //     return Err(anyhow!("invalid name or id"));
        // };
        let stream_name = StreamName::from_parts(name, Some(&id));
        let stream = self.get_or_create_stream(&stream_name).await?;

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

    async fn get_or_create_stream(
        &self,
        stream_name: &StreamName<'static>,
    ) -> Result<Entry<StreamName<'static>, StreamHandle<E>>> {
        let module_data = self.get_module(stream_name.category())?.clone();
        let was_paused = module_data.wait_if_paused().await;
        let module = if was_paused {
            // Module was paused, we should get a new copy of the module
            self.get_module(stream_name.category())?.module.clone()
        } else {
            module_data.module
        };

        let res = self
            .streams
            .entry_by_ref(&stream_name)
            .or_try_insert_with(spawn_stream(&self.event_store, &stream_name, &module))
            .await
            .map_err(ArcError::Arc);
        let stream = self
            .handle_trap(res, stream_name.category())
            .await
            .map_err(|err| anyhow!("{err}"))?;

        Ok(stream)
    }

    async fn handle_trap<T>(&self, res: Result<T, ArcError>, name: &str) -> Result<T, Error> {
        if let Err(err) = &res {
            if let Some(trap) = err.as_ref().root_cause().downcast_ref::<Trap>().copied() {
                error!("module {name} trapped: {trap}");
                let name = name.to_string();
                if let Err(err) = self.restart_module(name).await {
                    error!("failed to restart module: {err}");
                }

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
        }

        Ok(())
    }

    fn get_module(&self, name: &str) -> Result<Ref<String, ModuleData>> {
        self.modules
            .get(name)
            .ok_or_else(|| anyhow!("aggregate '{name}' does not exist or is not running"))
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

    /// Specifies the timeout for commands if no response is received.
    ///
    /// Defaults to 5 seconds.
    pub fn command_timeout(mut self, timeout: Duration) -> Self {
        self.command_timeout = timeout;
        self
    }

    /// Specifies the directory containing the aggregate wasm modules.
    ///
    /// Defaults to `./modules`.
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
    /// Defaults to `10,000`.
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

    async fn wait_if_paused(&self) -> bool {
        let mut was_paused = false;
        loop {
            let notified = self.notify.notified();
            if !self.paused.load(Ordering::Acquire) {
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
