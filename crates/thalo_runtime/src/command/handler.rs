use std::borrow::Cow;
use std::collections::HashMap;

use anyhow::{anyhow, Context as AnyhowContext, Result};
use futures::TryFutureExt;
// use message_db::database::{GetStreamMessagesOpts, MessageStore, WriteMessageOpts};
// use message_db::message::{MessageData, MetadataRef};
// use message_db::stream_name::StreamName;
use semver::VersionReq;
use serde_json::Value;
use thalo::{Context, Metadata, StreamName};
use thalo_message_store::message::MessageData;
use thalo_message_store::message_store::MessageStore;
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio::sync::oneshot;

use crate::module::{Event, ExecuteResult, ModuleInstance};
use crate::runtime::Runtime;

pub struct CommandHandler {
    tx: Sender<ExecuteMsg>,
}

struct ExecuteMsg {
    tx: oneshot::Sender<Result<ExecuteResult>>,
    ctx: Context<'static>,
    command: String,
    payload: Value,
}

impl CommandHandler {
    pub async fn start(
        runtime: &Runtime,
        message_store: MessageStore,
        module_name: &str,
        stream_name: StreamName<'static>,
        version: &VersionReq,
    ) -> Result<CommandHandler> {
        let instance = runtime
            .load_module(module_name, version)
            .and_then(|(_module_id, module)| {
                let id = stream_name.id().unwrap().to_string();
                async move {
                    let instance = module.init(id).await?;
                    Ok(instance)
                }
            })
            .await?;

        let mut stream = message_store.stream(stream_name.as_borrowed())?;
        for res in stream.iter_all_messages::<MessageData>() {
            let raw_message = res?;
            let mut message = raw_message.message()?;
            let ctx_value = message
                .metadata
                .properties
                .remove("ctx")
                .ok_or_else(|| anyhow!("missing ctx in event metadata"))?;
            let ctx = serde_json::from_value(ctx_value.into_owned())
                .context("failed to deserialize ctx in metadata")?;
            let event = Event {
                ctx,
                event: message.msg_type,
                payload: Cow::Owned(serde_json::to_string(&message.data)?),
            };
            instance.apply(&[event]).await?;
        }
        // for chunk in &stream.iter_all_messages::<MessageData>().chunks(100) {
        //     let batch = chunk.collect::<Result<Vec<_>, _>>()?;
        //     let events = batch
        //         .iter()
        //         .map(|raw_message| {
        //             let mut message = raw_message.message()?;
        //             let ctx_value = message
        //                 .metadata
        //                 .properties
        //                 .remove("ctx")
        //                 .ok_or_else(|| anyhow!("missing ctx in event metadata"))?;
        //             let ctx = serde_json::from_value(ctx_value.into_owned())
        //                 .context("failed to deserialize ctx in metadata")?;
        //             Ok(Event {
        //                 ctx,
        //                 event: message.msg_type,
        //                 payload: Cow::Owned(serde_json::to_string(&message.data)?),
        //             })
        //         })
        //         .collect::<Result<Vec<_>>>()?;
        //     instance.apply(&events).await?;
        // }
        let version = stream.version()?;

        let (tx, rx) = mpsc::channel(64);
        tokio::spawn(command_handler(
            rx,
            message_store,
            instance,
            stream_name,
            version,
        ));

        Ok(CommandHandler { tx })
    }

    // pub async fn execute(&self, command: String, payload: Vec<u8>) ->
    // Result<ExecuteResult> {     let (tx, rx) = oneshot::channel();
    //     self.do_execute(tx, command, payload).await?;
    //     rx.await?
    // }

    pub async fn do_execute(
        &self,
        tx: oneshot::Sender<Result<ExecuteResult>>,
        ctx: Context<'static>,
        command: String,
        payload: Value,
    ) -> Result<()> {
        self.tx
            .send(ExecuteMsg {
                tx,
                ctx,
                command,
                payload,
            })
            .await
            .map_err(|err| anyhow!("failed to send execute msg: {err}"))
    }
}

async fn command_handler(
    mut rx: Receiver<ExecuteMsg>,
    message_store: MessageStore,
    mut instance: ModuleInstance,
    stream_name: StreamName<'_>,
    mut version: Option<u64>,
) {
    while let Some(req) = rx.recv().await {
        let res = handle_command(
            &message_store,
            &mut instance,
            &stream_name,
            &mut version,
            req.ctx,
            req.command,
            req.payload,
        )
        .await;
        let _ = req.tx.send(res);
    }
}

async fn handle_command(
    message_store: &MessageStore,
    instance: &mut ModuleInstance,
    stream_name: &StreamName<'_>,
    version: &mut Option<u64>,
    ctx: Context<'_>,
    command: String,
    payload: Value,
) -> Result<ExecuteResult> {
    let command_payload = serde_json::to_string(&payload)?;
    let result = instance
        .handle_and_apply(&ctx, &command, &command_payload)
        .await?;

    if let ExecuteResult::Events(events) = &result {
        if !events.is_empty() {
            *version = Some(save_events(message_store, stream_name, *version, &ctx, events).await?);
        }
    }

    Ok(result)
}

async fn save_events(
    message_store: &MessageStore,
    stream_name: &StreamName<'_>,
    version: Option<u64>,
    ctx: &Context<'_>,
    events: &[Event<'_>],
) -> Result<u64> {
    let event_ctxs: Vec<_> = events
        .into_iter()
        .map(|event| {
            let event_ctx = serde_json::to_value(event.ctx.clone()).unwrap();
            (event, event_ctx)
        })
        .collect();
    let messages: Vec<_> = event_ctxs
        .into_iter()
        .map(|(event, event_ctx)| {
            let data: Value = serde_json::from_str(&event.payload)?;
            let metadata = Metadata {
                stream_name: stream_name.as_borrowed(),
                position: version.map(|v| v + 1).unwrap_or(0),
                causation_message_stream_name: ctx.stream_name.as_borrowed(),
                causation_message_position: ctx.position,
                reply_stream_name: None,
                schema_version: None,
                properties: HashMap::from_iter([(Cow::Borrowed("ctx"), Cow::Owned(event_ctx))]),
            };

            Ok((event, data, metadata))
        })
        .collect::<Result<_>>()?;
    let messages = messages
        .iter()
        .map(|(event, data, metadata)| (event.event.as_ref(), data, metadata));

    let version = message_store
        .stream(stream_name.as_borrowed())?
        .write_messages(messages, version)
        .await?;

    Ok(version)
}
