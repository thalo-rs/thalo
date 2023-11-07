mod handler;

use std::collections::HashMap;

use anyhow::{anyhow, Result};
use semver::VersionReq;
use serde_json::Value;
use thalo::{Category, Context, StreamName, ID};
use thalo_message_store::message_store::MessageStore;
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio::sync::oneshot;

use self::handler::CommandHandler;
use crate::module::ExecuteResult;
use crate::runtime::Runtime;

#[derive(Clone, Debug)]
pub struct CommandRouter {
    tx: Sender<ExecuteMsg>,
}

struct ExecuteMsg {
    tx: oneshot::Sender<Result<ExecuteResult>>,
    runtime: Runtime,
    message_store: MessageStore,
    name: String,
    id: String,
    ctx: Context<'static>,
    command: String,
    payload: Value,
}

impl CommandRouter {
    pub fn start() -> Self {
        let (tx, rx) = mpsc::channel(1024);
        tokio::spawn(command_router(rx));
        CommandRouter { tx }
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn execute(
        &self,
        runtime: Runtime,
        message_store: MessageStore,
        name: String,
        id: String,
        ctx: Context<'static>,
        command: String,
        payload: Value,
    ) -> Result<ExecuteResult> {
        let (tx, rx) = oneshot::channel();
        self.do_execute(tx, runtime, message_store, name, id, ctx, command, payload)
            .await?;
        rx.await?
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn do_execute(
        &self,
        tx: oneshot::Sender<Result<ExecuteResult>>,
        runtime: Runtime,
        message_store: MessageStore,
        name: String,
        id: String,
        ctx: Context<'static>,
        command: String,
        payload: Value,
    ) -> Result<()> {
        self.tx
            .send(ExecuteMsg {
                tx,
                runtime,
                message_store,
                name,
                id,
                ctx,
                command,
                payload,
            })
            .await
            .map_err(|err| anyhow!("failed to send execute msg: {err}"))
    }
}

async fn command_router(mut rx: Receiver<ExecuteMsg>) {
    let mut streams: HashMap<StreamName<'static>, CommandHandler> = HashMap::new();

    while let Some(req) = rx.recv().await {
        let stream_name =
            match Category::new(Category::normalize(&req.name)).and_then(|category: Category| {
                StreamName::from_parts(category, Some(&ID::new(&req.id)?))
            }) {
                Ok(stream_name) => stream_name,
                Err(err) => {
                    let _ = req.tx.send(Err(err.into()));
                    continue;
                }
            };

        match streams.get(&stream_name) {
            Some(handler) => {
                let _ = handler
                    .do_execute(req.tx, req.ctx, req.command, req.payload)
                    .await;
            }
            None => {
                match CommandHandler::start(
                    &req.runtime,
                    req.message_store,
                    &req.name,
                    stream_name.clone(),
                    &VersionReq::STAR,
                )
                .await
                {
                    Ok(handler) => {
                        let _ = handler
                            .do_execute(req.tx, req.ctx, req.command, req.payload)
                            .await;
                        streams.insert(stream_name, handler);
                    }
                    Err(err) => {
                        let _ = req.tx.send(Err(anyhow!("failed to start handler: {err}")));
                    }
                };
            }
        }
    }
}
