use async_trait::async_trait;
use ractor::{Actor, ActorProcessingErr, ActorRef};
use thalo_message_store::message::GenericMessage;
use tokio::sync::mpsc;

pub type ProjectionSubscriptionRef = ActorRef<ProjectionSubscriptionMsg>;

pub struct ProjectionSubscription;

pub struct ProjectionSubscriptionState {
    tx: mpsc::Sender<GenericMessage<'static>>,
    pending_events: Vec<GenericMessage<'static>>,
    last_acknowledged_id: Option<u64>,
    last_processed_id: Option<u64>,
}

pub enum ProjectionSubscriptionMsg {
    NewEvent(GenericMessage<'static>),
    ProcessPendingEvent,
    AcknowledgeEvent { global_id: u64 },
}

pub struct ProjectionSubscriptionArgs {
    pub tx: mpsc::Sender<GenericMessage<'static>>,
    pub last_acknowledged_id: Option<u64>,
}

#[async_trait]
impl Actor for ProjectionSubscription {
    type State = ProjectionSubscriptionState;
    type Msg = ProjectionSubscriptionMsg;
    type Arguments = ProjectionSubscriptionArgs;

    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        ProjectionSubscriptionArgs {
            tx,
            last_acknowledged_id,
        }: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        Ok(ProjectionSubscriptionState {
            tx,
            pending_events: Vec::new(),
            last_acknowledged_id,
            last_processed_id: None,
        })
    }

    async fn handle(
        &self,
        myself: ActorRef<Self::Msg>,
        msg: ProjectionSubscriptionMsg,
        ProjectionSubscriptionState {
            tx,
            pending_events,
            last_acknowledged_id,
            last_processed_id,
        }: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match msg {
            ProjectionSubscriptionMsg::NewEvent(event) => {
                dbg!(*last_acknowledged_id);
                pending_events.push(event);
                myself.cast(ProjectionSubscriptionMsg::ProcessPendingEvent)?;
            }
            ProjectionSubscriptionMsg::ProcessPendingEvent => {
                println!("process pending event: {:?}", last_acknowledged_id);

                if !pending_events.is_empty()
                    && (last_processed_id.is_none() || last_processed_id == last_acknowledged_id)
                {
                    let next_event = pending_events.remove(0);
                    let next_event_global_id = next_event.global_id;
                    tx.send(next_event).await?;
                    *last_processed_id = Some(next_event_global_id);
                }
            }
            ProjectionSubscriptionMsg::AcknowledgeEvent { global_id } => {
                println!("acknowledge event");
                *last_acknowledged_id = Some(global_id);
                myself.cast(ProjectionSubscriptionMsg::ProcessPendingEvent)?;
            }
        }

        Ok(())
    }
}
