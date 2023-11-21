use std::{collections::HashMap, time::Duration};

use anyhow::{Context, Result};
use thalo_message_store::{message::GenericMessage, projection::Projection, MessageStore};
use tokio::{
    sync::{broadcast, mpsc, oneshot},
    time::interval,
};
use tracing::{error, warn};

use super::projection_subscription::ProjectionSubscriptionHandle;

const FLUSH_INTERVAL: Duration = Duration::from_millis(500);

#[derive(Clone)]
pub struct ProjectionGatewayHandle {
    sender: mpsc::Sender<ProjectionGatewayMsg>,
}

impl ProjectionGatewayHandle {
    pub fn new(
        message_store: MessageStore,
        subscriber: broadcast::Receiver<GenericMessage<'static>>,
    ) -> Self {
        let (sender, receiver) = mpsc::channel(8);
        tokio::spawn(run_projection_gateway(
            (sender.clone(), receiver),
            message_store,
            subscriber,
        ));

        ProjectionGatewayHandle { sender }
    }

    pub async fn acknowledge_event(&self, name: String, global_id: u64) -> Result<()> {
        let (reply, recv) = oneshot::channel();
        let msg = ProjectionGatewayMsg::AcknowledgeEvent {
            name,
            global_id,
            reply,
        };
        let _ = self.sender.send(msg).await;
        recv.await
            .context("no response from projection subscription")?
    }

    pub async fn start_projection(
        &self,
        tx: mpsc::Sender<GenericMessage<'static>>,
        name: String,
        events: Vec<String>,
    ) -> Result<()> {
        let (reply, recv) = oneshot::channel();
        let msg = ProjectionGatewayMsg::StartProjection {
            tx,
            name,
            events,
            reply,
        };
        let _ = self.sender.send(msg).await;
        recv.await.context("no response from projection gateway")?
    }

    pub(crate) async fn set_subscription_to_process_new_events(&self, name: String) -> Result<()> {
        let (reply, recv) = oneshot::channel();
        let msg = ProjectionGatewayMsg::SetProjectionToProcessNewEvents { name, reply };
        let _ = self.sender.send(msg).await;
        recv.await.context("no response from projection gateway")
    }
}

enum ProjectionGatewayMsg {
    AcknowledgeEvent {
        name: String,
        global_id: u64,
        reply: oneshot::Sender<Result<()>>,
    },
    StartProjection {
        tx: mpsc::Sender<GenericMessage<'static>>,
        name: String,
        events: Vec<String>,
        reply: oneshot::Sender<Result<()>>,
    },
    StopProjection {
        name: String,
    },
    SetProjectionToProcessNewEvents {
        name: String,
        reply: oneshot::Sender<()>,
    },
}

async fn run_projection_gateway(
    (sender, mut receiver): (
        mpsc::Sender<ProjectionGatewayMsg>,
        mpsc::Receiver<ProjectionGatewayMsg>,
    ),
    message_store: MessageStore,
    mut subscriber: broadcast::Receiver<GenericMessage<'static>>,
) {
    let mut projection_gateway = ProjectionGateway {
        sender,
        projections: HashMap::new(),
        message_store,
        is_dirty: false,
    };

    let mut timer = interval(FLUSH_INTERVAL);

    loop {
        tokio::select! {
            msg = receiver.recv() => match msg {
                Some(msg) => match msg {
                    ProjectionGatewayMsg::AcknowledgeEvent { name, global_id, reply } => {
                        let res = projection_gateway.acknowledge_event(name, global_id);
                        let _ = reply.send(res);
                    }
                    ProjectionGatewayMsg::StartProjection { tx, name, events, reply } => {
                        let res = projection_gateway.start_projection(tx, name, events);
                        let _ = reply.send(res);
                    }
                    ProjectionGatewayMsg::StopProjection { name } => {
                        projection_gateway.stop_projection(name);
                    }
                    ProjectionGatewayMsg::SetProjectionToProcessNewEvents { name, reply } => {
                        let res = projection_gateway.set_projection_to_process_new_events(name);
                        let _ = reply.send(res);
                    }
                }
                None => break,
            },
            event = subscriber.recv() => match event {
                Ok(event) => {
                    if let Err(err) = projection_gateway.new_event(event) {
                        error!("{err}");
                    }
                }
                Err(broadcast::error::RecvError::Closed) => {
                    error!("event broadcaster closed");
                    break;
                }
                Err(broadcast::error::RecvError::Lagged(_)) => {
                    error!("projection gateway lagged");
                    break;
                }
            },
            _ = timer.tick() => {
                if let Err(err) = projection_gateway.flush().await {
                    error!("{err}");
                }
            }
        }
    }
}

struct ProjectionGateway {
    sender: mpsc::Sender<ProjectionGatewayMsg>,
    projections: HashMap<String, Subscription>,
    message_store: MessageStore,
    is_dirty: bool,
}

impl ProjectionGateway {
    fn acknowledge_event(&mut self, name: String, global_id: u64) -> Result<()> {
        if let Some(subscription) = self.projections.get_mut(&name) {
            subscription.projection.acknowledge_event(global_id, true)?;
            self.is_dirty = true;

            let projection_subscription = subscription.projection_subscription.clone();
            let sender = self.sender.clone();
            tokio::spawn(async move {
                if let Err(err) = projection_subscription.acknowledge_event(global_id).await {
                    warn!("failed to acknowledge event with projection subscription: {err}");
                    let _ = sender.try_send(ProjectionGatewayMsg::StopProjection { name });
                }
            });
        } else {
            let mut projection = self.message_store.projection(&name)?;
            projection.acknowledge_event(global_id, true)?;
            self.is_dirty = true;
        }

        Ok(())
    }

    fn start_projection(
        &mut self,
        tx: mpsc::Sender<GenericMessage<'static>>,
        name: String,
        events: Vec<String>,
    ) -> Result<()> {
        let projection = self.message_store.projection(name.clone())?;
        let projection_subscription = ProjectionSubscriptionHandle::new(
            name.clone(),
            ProjectionGatewayHandle {
                sender: self.sender.clone(),
            },
            events.clone(),
            tx,
            projection.last_relevant_event_id(),
            self.message_store.global_event_log()?,
        );

        let subscription = Subscription {
            projection_subscription,
            projection,
            events,
            process_new_events: false,
        };
        self.projections.insert(name, subscription);

        Ok(())
    }

    fn new_event(&mut self, event: GenericMessage<'static>) -> Result<()> {
        for (name, subscription) in &mut self.projections {
            if !subscription.process_new_events {
                continue;
            }

            let is_relevant = subscription.events.is_empty()
                || subscription
                    .events
                    .iter()
                    .any(|event_name| event_name == &event.msg_type);
            subscription
                .projection
                .acknowledge_event(event.global_id, false)?;
            self.is_dirty = true;

            if is_relevant {
                let projection_subscription = subscription.projection_subscription.clone();
                let sender = self.sender.clone();
                let event = event.clone();
                let name = name.clone();
                tokio::spawn(async move {
                    if let Err(err) = projection_subscription.new_event(event).await {
                        warn!("failed to send new event to projection subscription: {err}");
                        let _ = sender.try_send(ProjectionGatewayMsg::StopProjection { name });
                    }
                });
            }
        }

        Ok(())
    }

    async fn flush(&self) -> Result<()> {
        self.message_store.flush_projections().await?;

        Ok(())
    }

    fn stop_projection(&mut self, name: String) {
        self.projections.remove(&name);
    }

    fn set_projection_to_process_new_events(&mut self, name: String) {
        if let Some(subscription) = self.projections.get_mut(&name) {
            subscription.process_new_events = true;
        }
    }
}

struct Subscription {
    projection_subscription: ProjectionSubscriptionHandle,
    projection: Projection,
    events: Vec<String>,
    process_new_events: bool,
}
