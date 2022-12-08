// wit_bindgen_host_wasmtime_rust::generate!({
//     // name: "aggregate",
//     path: "aggregate.wit",
//     async: true
// });

use anyhow::anyhow;

pub type StateParam<'a> = &'a [u8];
pub type StateResult = Vec<u8>;
#[derive(wasmtime::component::ComponentType, wasmtime::component::Lower)]
#[component(record)]
#[derive(Clone, Debug)]
pub struct EventParam<'a> {
    #[component(name = "ctx")]
    pub ctx: ContextParam<'a>,
    #[component(name = "event-type")]
    pub event_type: &'a str,
    #[component(name = "payload")]
    pub payload: &'a [u8],
}

#[derive(
    wasmtime::component::ComponentType, wasmtime::component::Lift, wasmtime::component::Lower,
)]
#[component(record)]
#[derive(Clone, Debug)]
pub struct EventResult {
    #[component(name = "ctx")]
    pub ctx: ContextResult,
    #[component(name = "event-type")]
    pub event_type: String,
    #[component(name = "payload")]
    pub payload: Vec<u8>,
}

#[derive(wasmtime::component::ComponentType, wasmtime::component::Lower)]
#[component(record)]
#[derive(Clone, Debug)]
pub struct Command<'a> {
    #[component(name = "command")]
    pub command: &'a str,
    #[component(name = "payload")]
    pub payload: &'a [u8],
}

#[derive(wasmtime::component::ComponentType, wasmtime::component::Lower)]
#[component(record)]
#[derive(Clone, Debug)]
pub struct ContextParam<'a> {
    #[component(name = "id")]
    pub id: &'a str,
    #[component(name = "stream-name")]
    pub stream_name: &'a str,
    #[component(name = "position")]
    pub position: i64,
    #[component(name = "global-position")]
    pub global_position: i64,
    #[component(name = "metadata")]
    pub metadata: &'a [u8],
    #[component(name = "time")]
    pub time: i64,
}
#[derive(
    wasmtime::component::ComponentType, wasmtime::component::Lift, wasmtime::component::Lower,
)]
#[component(record)]
#[derive(Clone, Debug)]
pub struct ContextResult {
    #[component(name = "id")]
    pub id: String,
    #[component(name = "stream-name")]
    pub stream_name: String,
    #[component(name = "position")]
    pub position: i64,
    #[component(name = "global-position")]
    pub global_position: i64,
    #[component(name = "metadata")]
    pub metadata: Vec<u8>,
    #[component(name = "time")]
    pub time: i64,
}

impl ContextResult {
    pub fn as_param(&self) -> ContextParam {
        ContextParam {
            id: &self.id,
            stream_name: &self.stream_name,
            position: self.position,
            global_position: self.global_position,
            metadata: &self.metadata,
            time: self.time,
        }
    }
}

#[derive(
    wasmtime::component::ComponentType, wasmtime::component::Lift, wasmtime::component::Lower,
)]
#[component(variant)]
#[derive(Clone, Debug)]
pub enum Error {
    #[component(name = "command")]
    Command(String),
    #[component(name = "ignore")]
    Ignore(Option<String>),
    #[component(name = "deserialize-command")]
    DeserializeCommand(String),
    #[component(name = "deserialize-context")]
    DeserializeContext(String),
    #[component(name = "deserialize-event")]
    DeserializeEvent(String),
    #[component(name = "deserialize-state")]
    DeserializeState(String),
    #[component(name = "serialize-command")]
    SerializeCommand(String),
    #[component(name = "serialize-event")]
    SerializeEvent(String),
    #[component(name = "serialize-state")]
    SerializeState(String),
    #[component(name = "unknown-command")]
    UnknownCommand,
    #[component(name = "unknown-event")]
    UnknownEvent,
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Error::Command(msg) => write!(f, "command failed: {msg}"),
            Error::Ignore(reason) => match reason {
                Some(reason) => write!(f, "command ignored with reason: '{reason}'"),
                None => write!(f, "command ignored"),
            },
            Error::DeserializeCommand(msg) => write!(f, "failed to deserialize command: {msg}"),
            Error::DeserializeContext(msg) => write!(f, "failed to deserialize context: {msg}"),
            Error::DeserializeEvent(msg) => write!(f, "failed to deserialize event: {msg}"),
            Error::DeserializeState(msg) => write!(f, "failed to deserialize state: {msg}"),
            Error::SerializeCommand(msg) => write!(f, "failed to serialize command: {msg}"),
            Error::SerializeEvent(msg) => write!(f, "failed to serialize event: {msg}"),
            Error::SerializeState(msg) => write!(f, "failed to serialize state: {msg}"),
            Error::UnknownCommand => write!(f, "unknown command"),
            Error::UnknownEvent => write!(f, "unknown event"),
        }
    }
}
impl std::error::Error for Error {}

#[derive(Clone, Copy, Debug)]
pub struct Aggregate {
    init: wasmtime::component::Func,
    apply: wasmtime::component::Func,
    handle: wasmtime::component::Func,
}
impl Aggregate {
    pub fn new(
        exports: &mut wasmtime::component::ExportInstance<'_, '_>,
    ) -> anyhow::Result<Aggregate> {
        let init = *exports
            .typed_func::<(&str,), (Result<StateResult, Error>,)>("init")?
            .func();
        let apply = *exports
            .typed_func::<(StateParam<'_>, &[EventParam<'_>]), (Result<StateResult, Error>,)>(
                "apply",
            )?
            .func();
        let handle = *exports
            .typed_func::<(StateParam<'_>, ContextParam<'_,>, Command<'_>), (Result<Vec<EventResult>, Error>,)>(
                "handle",
            )?
            .func();
        Ok(Aggregate {
            init,
            apply,
            handle,
        })
    }
    pub async fn init<S: wasmtime::AsContextMut>(
        &self,
        mut store: S,
        arg0: &str,
    ) -> anyhow::Result<Result<StateResult, Error>>
    where
        <S as wasmtime::AsContext>::Data: Send,
    {
        let callee = unsafe {
            wasmtime::component::TypedFunc::<(&str,), (Result<StateResult, Error>,)>::new_unchecked(
                self.init,
            )
        };
        let (ret0,) = callee.call_async(store.as_context_mut(), (arg0,)).await?;
        callee.post_return_async(store.as_context_mut()).await?;
        Ok(ret0)
    }
    pub async fn apply<S: wasmtime::AsContextMut>(
        &self,
        mut store: S,
        arg0: StateParam<'_>,
        arg1: &[EventParam<'_>],
    ) -> anyhow::Result<Result<StateResult, Error>>
    where
        <S as wasmtime::AsContext>::Data: Send,
    {
        let callee = unsafe {
            wasmtime::component::TypedFunc::<
                (StateParam<'_>, &[EventParam<'_>]),
                (Result<StateResult, Error>,),
            >::new_unchecked(self.apply)
        };
        let (ret0,) = callee
            .call_async(store.as_context_mut(), (arg0, arg1))
            .await?;
        callee.post_return_async(store.as_context_mut()).await?;
        Ok(ret0)
    }
    pub async fn handle<S: wasmtime::AsContextMut>(
        &self,
        mut store: S,
        arg0: StateParam<'_>,
        arg1: ContextParam<'_>,
        arg2: Command<'_>,
    ) -> anyhow::Result<Result<Vec<EventResult>, Error>>
    where
        <S as wasmtime::AsContext>::Data: Send,
    {
        let callee = unsafe {
            wasmtime::component::TypedFunc::<
                (StateParam<'_>, ContextParam<'_>, Command<'_>),
                (Result<Vec<EventResult>, Error>,),
            >::new_unchecked(self.handle)
        };
        let (ret0,) = callee
            .call_async(store.as_context_mut(), (arg0, arg1, arg2))
            .await?;
        callee.post_return_async(store.as_context_mut()).await?;
        Ok(ret0)
    }
}

/// Instantiates the provided `module` using the specified
/// parameters, wrapping up the result in a structure that
/// translates between wasm and the host.
pub async fn instantiate_async<T: Send>(
    mut store: impl wasmtime::AsContextMut<Data = T>,
    component: &wasmtime::component::Component,
    linker: &wasmtime::component::Linker<T>,
) -> anyhow::Result<(Aggregate, wasmtime::component::Instance)> {
    let instance = linker.instantiate_async(&mut store, component).await?;
    Ok((new(store, &instance)?, instance))
}

/// Low-level creation wrapper for wrapping up the exports
/// of the `instance` provided in this structure of wasm
/// exports.
///
/// This function will extract exports from the `instance`
/// defined within `store` and wrap them all up in the
/// returned structure which can be used to interact with
/// the wasm module.
pub fn new(
    mut store: impl wasmtime::AsContextMut,
    instance: &wasmtime::component::Instance,
) -> anyhow::Result<Aggregate> {
    let mut store = store.as_context_mut();
    let mut exports = instance.exports(&mut store);
    let mut exports = exports.root();
    let aggregate = Aggregate::new(
        &mut exports
            .instance("aggregate")
            .ok_or_else(|| anyhow!("no exported instance \"aggregate\""))?,
    )?;
    Ok(aggregate)
}

const _: &str = "";
