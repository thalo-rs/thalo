use std::{collections::HashMap, time::Duration};

use async_trait::async_trait;
use ractor::{Actor, ActorProcessingErr, ActorRef};
use ractor_actors::streams::{spawn_loop, IterationResult, Operation};
use thalo_message_store::{message::GenericMessage, projection::Projection, MessageStore};
use tokio::sync::{broadcast, mpsc};
use tracing::error;

use crate::flusher::{Flusher, FlusherMsg, FlusherRef};

use super::projection_subscription::{
    ProjectionSubscription, ProjectionSubscriptionArgs, ProjectionSubscriptionMsg,
    ProjectionSubscriptionRef,
};

const FLUSH_INTERVAL: Duration = Duration::from_millis(500);

pub type ProjectionGatewayRef = ActorRef<ProjectionGatewayMsg>;

pub struct ProjectionGateway;

pub struct ProjectionGatewayState {
    projections: HashMap<String, Subscription>,
    message_store: MessageStore,
    flusher: FlusherRef,
}

struct Subscription {
    projection_subscription: ProjectionSubscriptionRef,
    projection: Projection,
    events: Vec<String>,
}

pub enum ProjectionGatewayMsg {
    StartProjection {
        tx: mpsc::Sender<GenericMessage<'static>>,
        name: String,
        events: Vec<String>,
    },
    AcknowledgeEvent {
        name: String,
        global_id: u64,
    },
    Event(GenericMessage<'static>),
}

pub struct ProjectionGatewayArgs {
    pub message_store: MessageStore,
    pub subscriber: broadcast::Receiver<GenericMessage<'static>>,
}

#[async_trait]
impl Actor for ProjectionGateway {
    type State = ProjectionGatewayState;
    type Msg = ProjectionGatewayMsg;
    type Arguments = ProjectionGatewayArgs;

    async fn pre_start(
        &self,
        myself: ActorRef<Self::Msg>,
        ProjectionGatewayArgs {
            message_store,
            subscriber,
        }: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        let (flusher, _) = Flusher::spawn_linked(
            Some("projections_flusher".to_string()),
            myself.get_cell(),
            FLUSH_INTERVAL,
            {
                let message_store = message_store.clone();
                move || {
                    let message_store = message_store.clone();
                    async move {
                        message_store.flush_projections().await?;
                        Ok(())
                    }
                }
            },
        )
        .await?;

        let myself_cell = myself.get_cell();
        spawn_loop(
            ProjectionGatewaySubscription,
            ProjectionGatewaySubscriptionState {
                subscriber,
                projection_gateway: myself,
            },
            Some(myself_cell),
        )
        .await?;

        Ok(ProjectionGatewayState {
            projections: HashMap::new(),
            message_store,
            flusher,
        })
    }

    async fn handle(
        &self,
        myself: ActorRef<Self::Msg>,
        msg: ProjectionGatewayMsg,
        ProjectionGatewayState {
            ref mut projections,
            message_store,
            flusher,
        }: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match msg {
            ProjectionGatewayMsg::StartProjection { name, events, tx } => {
                let projection = message_store.projection(name.clone())?;
                let (projection_subscription, _) = Actor::spawn_linked(
                    Some(format!("{name}_projection")),
                    ProjectionSubscription,
                    ProjectionSubscriptionArgs {
                        tx,
                        last_acknowledged_id: projection.last_relevant_event_id(),
                    },
                    myself.get_cell(),
                )
                .await?;
                let subscription = Subscription {
                    projection_subscription,
                    projection: message_store.projection(name.clone())?,
                    events,
                };
                if let Some(old_subscription) = projections.insert(name, subscription) {
                    old_subscription.projection_subscription.stop(None);
                }
            }
            ProjectionGatewayMsg::AcknowledgeEvent { name, global_id } => {
                if let Some(subscription) = projections.get_mut(&name) {
                    subscription.projection.acknowledge_event(global_id, true)?;
                    flusher.cast(FlusherMsg::MarkDirty)?;

                    if let Err(err) = subscription
                        .projection_subscription
                        .cast(ProjectionSubscriptionMsg::AcknowledgeEvent { global_id })
                    {
                        subscription.projection_subscription.kill();
                        projections.remove(&name);
                        error!("{err}");
                    }
                } else {
                    let mut projection = message_store.projection(&name)?;
                    projection.acknowledge_event(global_id, true)?;
                    flusher.cast(FlusherMsg::MarkDirty)?;
                }
            }
            ProjectionGatewayMsg::Event(event) => {
                for (_name, subscription) in projections {
                    let is_relevant = subscription
                        .events
                        .iter()
                        .any(|event_name| event_name == &event.msg_type);
                    subscription
                        .projection
                        .acknowledge_event(event.global_id, false)?;
                    flusher.cast(FlusherMsg::MarkDirty)?;

                    if is_relevant {
                        subscription
                            .projection_subscription
                            .cast(ProjectionSubscriptionMsg::NewEvent(event.clone()))?;
                    }
                }
            }
        }

        Ok(())
    }
}

struct ProjectionGatewaySubscription;

struct ProjectionGatewaySubscriptionState {
    subscriber: broadcast::Receiver<GenericMessage<'static>>,
    projection_gateway: ProjectionGatewayRef,
}

#[async_trait]
impl Operation for ProjectionGatewaySubscription {
    type State = ProjectionGatewaySubscriptionState;

    async fn work(&self, state: &mut Self::State) -> Result<IterationResult, ActorProcessingErr> {
        let event = state.subscriber.recv().await?;
        state
            .projection_gateway
            .cast(ProjectionGatewayMsg::Event(event))?;

        Ok(IterationResult::Continue)
    }
}
