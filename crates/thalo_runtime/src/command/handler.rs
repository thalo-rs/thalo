use std::collections::HashMap;

use anyhow::{anyhow, Context as AnyhowContext, Result};
use futures::TryFutureExt;
use messagedb::database::{GetStreamMessagesOpts, MessageStore, WriteMessageOpts};
use messagedb::message::{MessageData, MetadataRef};
use messagedb::stream_name::StreamName;
use semver::VersionReq;
use serde_json::Value;
use thalo::Context;
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio::sync::oneshot;

use crate::module::{Event, EventRef, ExecuteResult, ModuleInstance, ModuleName};
use crate::runtime::Runtime;

pub struct CommandHandler {
    tx: Sender<ExecuteMsg>,
}

struct ExecuteMsg {
    tx: oneshot::Sender<Result<ExecuteResult>>,
    ctx: Context,
    command: String,
    payload: Value,
}

impl CommandHandler {
    pub async fn start(
        runtime: &Runtime,
        message_store: MessageStore,
        module_name: &ModuleName,
        stream_name: StreamName,
        version: &VersionReq,
    ) -> Result<Self> {
        let module_fut =
            runtime
                .load_module(module_name, version)
                .and_then(|(_module_id, module)| {
                    let id = stream_name.id.as_ref().unwrap().to_string();
                    async move {
                        let instance = module.init(id).await?;
                        Ok(instance)
                    }
                });

        let stream_name_string = stream_name.to_string();
        let opts = GetStreamMessagesOpts::default();
        let (module_res, events_res) = tokio::join!(
            module_fut,
            MessageStore::get_stream_messages::<MessageData, _>(
                &message_store,
                &stream_name_string,
                &opts
            )
        );
        let mut instance = module_res?;

        // Apply events
        let events = events_res?;
        let version = events.last().map(|event| event.position).unwrap_or(-1);
        let events: Vec<_> = events
            .into_iter()
            .map(|mut event| {
                let ctx_json = event
                    .metadata
                    .properties
                    .remove("ctx")
                    .ok_or_else(|| anyhow!("missing ctx in event metadata"))?;
                let ctx = serde_json::from_value(ctx_json)
                    .context("failed to deserialize ctx in metadata")?;
                Ok((ctx, event.msg_type, serde_json::to_vec(&event.data)?))
            })
            .collect::<Result<_>>()?;

        let event_refs: Vec<_> = events
            .iter()
            .map(|(ctx, event_type, payload)| EventRef {
                ctx,
                event_type,
                payload,
            })
            .collect();

        instance.apply(&event_refs).await?;

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
        ctx: Context,
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
    stream_name: StreamName,
    mut version: i64,
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
    stream_name: &StreamName,
    version: &mut i64,
    ctx: Context,
    command: String,
    payload: Value,
) -> Result<ExecuteResult> {
    let command_payload = serde_json::to_vec(&payload)?;
    let result = instance
        .handle_and_apply(&ctx, &command, &command_payload)
        .await?;

    if let ExecuteResult::Events(events) = &result {
        if !events.is_empty() {
            *version = save_events(message_store, stream_name, *version, &ctx, events).await?;
        }
    }

    Ok(result)
}

async fn save_events(
    message_store: &MessageStore,
    stream_name: &StreamName,
    version: i64,
    ctx: &Context,
    events: &[Event],
) -> Result<i64> {
    let stream_name_string = stream_name.to_string();

    let event_ctxs: Vec<_> = events
        .iter()
        .map(|event| {
            let event_ctx = serde_json::to_value(event.ctx.clone()).unwrap();
            (event, event_ctx)
        })
        .collect();
    let event_options: Vec<_> = event_ctxs
        .iter()
        .enumerate()
        .map(|(i, (event, event_ctx))| {
            let data = serde_json::from_slice(&event.payload)
                .map_err(messagedb::Error::DeserializeData)?;
            let opts = WriteMessageOpts::builder()
                .expected_version(version + i as i64)
                .metadata(MetadataRef {
                    stream_name: Some(stream_name),
                    causation_message_stream_name: Some(&ctx.stream_name),
                    causation_message_position: Some(ctx.position),
                    causation_message_global_position: Some(ctx.global_position),
                    properties: HashMap::from_iter([("ctx", event_ctx)]),
                    ..Default::default()
                })
                .build();
            Ok((event, data, opts))
        })
        .collect::<Result<_>>()?;
    let messages: Vec<_> = event_options
        .iter()
        .map(|(event, data, opts)| (event.event_type.as_str(), data, opts))
        .collect();

    let version = message_store
        .write_messages(&stream_name_string, &messages)
        .await?;

    Ok(version)
}
