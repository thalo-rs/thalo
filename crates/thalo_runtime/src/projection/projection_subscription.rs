use anyhow::{Context, Result};
use thalo_message_store::message::GenericMessage;
use tokio::sync::{mpsc, oneshot};

use super::ProjectionGatewayHandle;

#[derive(Clone)]
pub struct ProjectionSubscriptionHandle {
    sender: mpsc::Sender<ProjectionSubscriptionMsg>,
}

impl ProjectionSubscriptionHandle {
    pub fn new(
        name: String,
        projection_gateway: ProjectionGatewayHandle,
        tx: mpsc::Sender<GenericMessage<'static>>,
        last_acknowledged_id: Option<u64>,
    ) -> Self {
        let (sender, receiver) = mpsc::channel(8);
        tokio::spawn(run_projection_subscription(
            receiver,
            name,
            projection_gateway,
            tx,
            last_acknowledged_id,
        ));

        ProjectionSubscriptionHandle { sender }
    }

    pub async fn new_event(&self, event: GenericMessage<'static>) -> Result<()> {
        let (reply, recv) = oneshot::channel();
        let msg = ProjectionSubscriptionMsg::NewEvent { event, reply };
        let _ = self.sender.send(msg).await;
        recv.await
            .context("no response from projection subscription")?
    }

    pub async fn acknowledge_event(&self, global_id: u64) -> Result<()> {
        let (reply, recv) = oneshot::channel();
        let msg = ProjectionSubscriptionMsg::AcknowledgeEvent { global_id, reply };
        let _ = self.sender.send(msg).await;
        recv.await
            .context("no response from projection subscription")?
    }
}

enum ProjectionSubscriptionMsg {
    NewEvent {
        event: GenericMessage<'static>,
        reply: oneshot::Sender<Result<()>>,
    },
    AcknowledgeEvent {
        global_id: u64,
        reply: oneshot::Sender<Result<()>>,
    },
}

async fn run_projection_subscription(
    mut receiver: mpsc::Receiver<ProjectionSubscriptionMsg>,
    name: String,
    projection_gateway: ProjectionGatewayHandle,
    tx: mpsc::Sender<GenericMessage<'static>>,
    last_acknowledged_id: Option<u64>,
) -> Result<()> {
    let mut projection_subscription = ProjectionSubscription {
        tx,
        pending_events: Vec::new(),
        last_acknowledged_id,
        last_processed_id: None,
    };

    // Process initial missed events
    let initial_missed_events =
        message_store.fetch_missed_events(&subscription_name, last_acknowledged_id)?;
    for event in initial_missed_events {
        tx.send(event).await?;
    }

    // Start listening to new events and buffer them
    let mut live_event_buffer = Vec::new();
    projection_gateway
        .set_subscription_to_process_new_events(name)
        .await?;

    // Check for any new missed events and process them
    let new_missed_events = message_store.fetch_missed_events(
        &subscription_name,
        Some(tx.last_event_id().await?), // Assuming last_event_id() gives the ID of the last processed event
    )?;
    for event in new_missed_events {
        while listening && let Ok(Some(new_event)) = receiver.try_recv() {
            live_event_buffer.push(new_event);
        }
        tx.send(event).await?;
    }
    listening = false;

    // Process buffered new events
    for event in live_event_buffer {
        tx.send(event).await?;
    }

    while let Some(msg) = receiver.recv().await {
        match msg {
            ProjectionSubscriptionMsg::NewEvent { event, reply } => {
                let res = projection_subscription.new_event(event).await;
                let _ = reply.send(res);
            }
            ProjectionSubscriptionMsg::AcknowledgeEvent { global_id, reply } => {
                let res = projection_subscription.acknowledge_event(global_id).await;
                let _ = reply.send(res);
            }
        }
    }

    Ok(())
}

struct ProjectionSubscription {
    tx: mpsc::Sender<GenericMessage<'static>>,
    pending_events: Vec<GenericMessage<'static>>,
    last_acknowledged_id: Option<u64>,
    last_processed_id: Option<u64>,
}

impl ProjectionSubscription {
    async fn new_event(&mut self, event: GenericMessage<'static>) -> Result<()> {
        self.pending_events.push(event);
        self.process_pending_event().await
    }

    async fn acknowledge_event(&mut self, global_id: u64) -> Result<()> {
        self.last_acknowledged_id = Some(global_id);
        self.process_pending_event().await
    }

    async fn process_pending_event(&mut self) -> Result<()> {
        if !self.pending_events.is_empty()
            && (self.last_processed_id.is_none()
                || self.last_processed_id == self.last_acknowledged_id)
        {
            let next_event = self.pending_events.remove(0);
            let next_event_global_id = next_event.global_id;
            self.tx.send(next_event).await?;
            self.last_processed_id = Some(next_event_global_id);
        }

        Ok(())
    }
}
