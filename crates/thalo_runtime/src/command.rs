mod handler;

use std::collections::HashMap;

use anyhow::{anyhow, Result};
use message_db::database::MessageStore;
use message_db::stream_name::{Category, StreamName, ID};
use semver::VersionReq;
use serde_json::Value;
use thalo::Context;
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio::sync::oneshot;

use self::handler::CommandHandler;
use crate::module::{ExecuteResult, ModuleName};
use crate::runtime::Runtime;

#[derive(Clone, Debug)]
pub struct CommandRouter {
    tx: Sender<ExecuteMsg>,
}

struct ExecuteMsg {
    tx: oneshot::Sender<Result<ExecuteResult>>,
    runtime: Runtime,
    message_store: MessageStore,
    name: ModuleName,
    id: String,
    ctx: Context,
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
        name: ModuleName,
        id: String,
        ctx: Context,
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
        name: ModuleName,
        id: String,
        ctx: Context,
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
    let mut streams: HashMap<Category, CommandHandler> = HashMap::new();

    while let Some(req) = rx.recv().await {
        let stream_name = match Category::normalize(&req.name)
            .parse()
            .and_then(|category: Category| Ok((category, ID::new(req.id)?)))
        {
            Ok((category, id)) => StreamName {
                category,
                id: Some(id),
            },
            Err(err) => {
                let _ = req.tx.send(Err(err.into()));
                continue;
            }
        };

        match streams.get(&stream_name.category) {
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
                        streams.insert(stream_name.category, handler);
                    }
                    Err(err) => {
                        let _ = req.tx.send(Err(anyhow!("failed to start handler: {err}")));
                    }
                };
            }
        }
    }
}
