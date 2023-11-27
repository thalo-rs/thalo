use std::iter::Skip;
use std::time::Duration;

use anyhow::{bail, Context, Result};
use async_recursion::async_recursion;
use thalo_message_store::global_event_log::{GlobalEventLog, GlobalEventLogIter};
use thalo_message_store::message::Message;
use tokio::sync::{mpsc, oneshot};
use tokio::time::interval;
use tracing::{error, info, trace};

use super::{EventInterest, ProjectionGatewayHandle};

#[derive(Clone)]
pub struct ProjectionSubscriptionHandle {
    sender: mpsc::Sender<ProjectionSubscriptionMsg>,
}

impl ProjectionSubscriptionHandle {
    pub fn new(
        name: String,
        projection_gateway: ProjectionGatewayHandle,
        events: Vec<EventInterest<'static>>,
        tx: mpsc::Sender<Message<'static>>,
        last_acknowledged_id: Option<u64>,
        global_event_log: GlobalEventLog,
    ) -> Self {
        let (sender, receiver) = mpsc::channel(1024);
        tokio::spawn(run_projection_subscription(
            receiver,
            name,
            projection_gateway,
            events,
            tx,
            last_acknowledged_id,
            global_event_log,
        ));

        ProjectionSubscriptionHandle { sender }
    }

    pub async fn new_event(
        &self,
        event: Message<'static>,
    ) -> Result<oneshot::Receiver<Result<()>>> {
        let this = self.clone();
        let (reply, recv) = oneshot::channel();
        let msg = ProjectionSubscriptionMsg::NewEvent { event, reply };
        let _ = this.sender.send(msg).await?;
        Ok(recv)
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
        event: Message<'static>,
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
    events: Vec<EventInterest<'static>>,
    tx: mpsc::Sender<Message<'static>>,
    last_acknowledged_id: Option<u64>,
    global_event_log: GlobalEventLog,
) -> Result<()> {
    let iter = global_event_log.iter_all_messages().skip(
        last_acknowledged_id
            .map(|global_id| (global_id as usize) + 1)
            .unwrap_or(0),
    );

    let mut projection_subscription = ProjectionSubscription {
        tx,
        name,
        projection_gateway,
        events,
        last_acknowledged_id,
        last_processed_id: None,
        pending_events: Vec::new(),
        state: ProjectionSubscriptionState::ProcessingMissedEvents,
        iter,
    };

    projection_subscription.process_pending_event().await?;

    let mut timer = interval(Duration::from_secs(1));

    loop {
        tokio::select! {
            msg = receiver.recv() => match msg {
                Some(msg) => match msg {
                    ProjectionSubscriptionMsg::NewEvent { event, reply } => {
                        let res = projection_subscription.new_event(event).await;
                        let _ = reply.send(res);
                    }
                    ProjectionSubscriptionMsg::AcknowledgeEvent { global_id, reply } => {
                        let res = projection_subscription.acknowledge_event(global_id).await;
                        let _ = reply.send(res);
                    }
                }
                None => break,
            },
            _ = timer.tick() => {
                if projection_subscription.tx.is_closed() {
                    break;
                }
            }
        }
    }

    trace!(name = %projection_subscription.name, "projection subscription stopping");

    Ok(())
}

struct ProjectionSubscription {
    tx: mpsc::Sender<Message<'static>>,
    name: String,
    projection_gateway: ProjectionGatewayHandle,
    events: Vec<EventInterest<'static>>,
    last_acknowledged_id: Option<u64>,
    last_processed_id: Option<u64>,
    pending_events: Vec<Message<'static>>,
    state: ProjectionSubscriptionState,
    iter: Skip<GlobalEventLogIter>,
}

impl ProjectionSubscription {
    async fn new_event(&mut self, event: Message<'static>) -> Result<()> {
        match self.state {
            ProjectionSubscriptionState::ProcessingMissedEvents => {
                bail!("received new event while still processing missed events - this should not happen, as we have not subscribed to listen to new events")
            }
            ProjectionSubscriptionState::BufferingLiveEvents => {
                self.pending_events.push(event);
            }
            ProjectionSubscriptionState::ProcessingLiveEvents => {
                self.pending_events.push(event);
                self.process_pending_event().await?;
            }
        }

        Ok(())
    }

    async fn acknowledge_event(&mut self, global_id: u64) -> Result<()> {
        self.last_acknowledged_id = Some(global_id);
        if self.last_processed_id == self.last_acknowledged_id {
            self.process_pending_event().await?;
        }

        Ok(())
    }

    #[async_recursion]
    async fn process_pending_event(&mut self) -> Result<()> {
        match self.state {
            ProjectionSubscriptionState::ProcessingMissedEvents
            | ProjectionSubscriptionState::BufferingLiveEvents => {
                // Only if the last processed id has been acknowledged then:
                //   1. Process iter until we reach the next event of interest
                //   2. Send this event to the `tx`
                //   3. Update last_processed_id

                while let Some(res) = self.iter.next() {
                    let event = match res.and_then(|raw_message| {
                        raw_message.message().map(|message| message.into_owned())
                    }) {
                        Ok(event) => event,
                        Err(err) => {
                            error!("{err}");
                            continue;
                        }
                    };

                    if self.is_event_of_interest(&event) {
                        // Check if this is the first event being processed or if the last processed
                        // event has been acknowledged
                        if self.last_processed_id.is_none()
                            || self.last_processed_id == self.last_acknowledged_id
                        {
                            let event_global_id = event.global_id;
                            self.tx.send(event).await?;
                            self.last_processed_id = Some(event_global_id);
                            return Ok(());
                        }
                    }
                }

                // Iterator finished and is empty, transition to next state
                self.state.next();
                if matches!(self.state, ProjectionSubscriptionState::BufferingLiveEvents) {
                    // Tell the projection gateway that we're interested in receiving live events
                    self.projection_gateway
                        .set_subscription_to_process_new_events(self.name.clone())
                        .await?;
                }
                self.process_pending_event().await?;
            }
            ProjectionSubscriptionState::ProcessingLiveEvents => {
                // Only if the last processed id has been acknowledged then:
                //   1. Process next pending event
                //   2. Update last processed id

                if !self.pending_events.is_empty()
                    && (self.last_processed_id.is_none()
                        || self.last_processed_id == self.last_acknowledged_id)
                {
                    let next_event = self.pending_events.remove(0);
                    let next_event_global_id = next_event.global_id;
                    self.tx.send(next_event).await?;
                    self.last_processed_id = Some(next_event_global_id);
                }
            }
        }

        Ok(())
    }

    fn is_event_of_interest(&self, message: &Message) -> bool {
        self.events.is_empty()
            || self
                .events
                .iter()
                .any(|event_interest| event_interest.is_interested(message))
    }
}

#[derive(Clone, Copy)]
enum ProjectionSubscriptionState {
    /// Initially processing missed events from the event store.
    ProcessingMissedEvents,
    /// Buffering new live events while still processing any remaining missed
    /// events.
    BufferingLiveEvents,
    /// Processing new live events as they arrive, but only after the previous
    /// event is acknowledged.
    ProcessingLiveEvents,
}

impl ProjectionSubscriptionState {
    fn next(&mut self) {
        match self {
            ProjectionSubscriptionState::ProcessingMissedEvents => {
                info!("transitioning to buffering live events");
                *self = ProjectionSubscriptionState::BufferingLiveEvents
            }
            ProjectionSubscriptionState::BufferingLiveEvents => {
                info!("transitioning to processing live events");
                *self = ProjectionSubscriptionState::ProcessingLiveEvents
            }
            ProjectionSubscriptionState::ProcessingLiveEvents => {}
        }
    }
}
