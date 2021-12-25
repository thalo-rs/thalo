use std::{collections::HashMap, marker::PhantomData, ops::Range};

use async_trait::async_trait;
use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};
use tracing::{debug, trace};

use crate::{
    Aggregate, AggregateEventHandler, CombinedEvent, Error, Event, EventHandler, Identity,
    Projection,
};

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct AggregateEvent<'a, A: Aggregate> {
    aggregate: PhantomData<A>,
    pub aggregate_id: &'a str,
    pub event: &'a <A as Aggregate>::Event,
}

impl<'a, A> AggregateEvent<'a, A>
where
    A: Aggregate,
{
    pub fn new(aggregate_id: &'a str, event: &'a <A as Aggregate>::Event) -> Self {
        Self {
            aggregate: PhantomData,
            aggregate_id,
            event,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BatchSize(usize);

impl BatchSize {
    pub fn new(size: usize) -> Self {
        BatchSize(size)
    }

    pub fn size(&self) -> usize {
        self.0
    }
}

impl Default for BatchSize {
    fn default() -> Self {
        BatchSize(10)
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct EventEnvelope<E> {
    pub id: i64,
    #[serde(with = "created_at_format")]
    pub created_at: DateTime<FixedOffset>,
    pub aggregate_type: String,
    pub aggregate_id: String,
    pub sequence: i64,
    pub event: E,
}

#[async_trait]
pub trait EventStore {
    async fn commit<A: Aggregate>(
        &self,
        events: Vec<<A as AggregateEventHandler>::Event>,
        agg: &mut A,
    ) -> Result<(), Error>;

    async fn save_events<A: Aggregate>(
        &self,
        events: Vec<AggregateEvent<'_, A>>,
    ) -> Result<Vec<EventEnvelope<<A as Aggregate>::Event>>, Error>;

    async fn get_aggregate_events<E: CombinedEvent>(
        &self,
        aggregate_type: &[&str],
        aggregate_id: Option<&str>,
        range: Range<i64>,
    ) -> Result<Vec<EventEnvelope<E>>, Error>;

    async fn get_all_events<E: Event>(
        &self,
        range: Range<i64>,
    ) -> Result<Vec<EventEnvelope<E>>, Error>;

    async fn get_event_by_aggregate_sequence<A: Aggregate>(
        &self,
        sequence: i64,
    ) -> Result<Option<EventEnvelope<<A as Aggregate>::Event>>, Error>;

    async fn load_aggregate<A: Aggregate>(&self, id: &str) -> Result<A, Error>;

    /// Replays a projection from it's first event to the latest event.
    ///
    /// `projection` should be a default instance of the projection which the events will be applied to.
    ///
    /// `batch_size` allows events to be queries in batches.
    async fn replay_projection<P>(
        &self,
        id: <<P as EventHandler>::View as Identity>::ID,
        projection: &mut P,
        batch_size: BatchSize,
    ) -> Result<<P as EventHandler>::View, Error>
    where
        P: Projection + Send + Sync,
        <P as EventHandler>::View: Identity,
    {
        let aggregate_types = <<P as EventHandler>::Event as CombinedEvent>::aggregate_types();
        let mut last_event_version = -1;
        let projection_type = <P as Projection>::projection_type();
        trace!(%projection_type, last_event_version, ?aggregate_types, "loading projection");

        let mut view = <P as EventHandler>::View::new_with_id(id);

        loop {
            let events = self
                .get_aggregate_events::<<P as EventHandler>::Event>(
                    &aggregate_types,
                    None,
                    last_event_version + 1..last_event_version + 1 + batch_size.size() as i64,
                )
                .await?;

            if events.is_empty() {
                break;
            }

            for event in events {
                projection
                    .handle(&mut view, event.event.clone(), event.id, event.sequence)
                    .await?;
                last_event_version = event.id;
            }
        }

        Ok(view)
    }

    /// Replays a projection and syncs it in the database based on the projections [`Projection::last_event_id`].
    /// After every event is applied, [`EventHandler::commit`] is called which typically saves the projection to the database.
    ///
    /// `projection` should be a default instance of the projection which the events will be applied to.
    ///
    /// `batch_size` allows events to be queries in batches.
    async fn resync_projection<P>(
        &self,
        projection: &mut P,
        batch_size: BatchSize,
    ) -> Result<(), Error>
    where
        P: Projection + Send + Sync,
    {
        let aggregate_types = <<P as EventHandler>::Event as CombinedEvent>::aggregate_types();
        let mut last_event_version = projection.last_event_id().await?.unwrap_or(-1);
        let projection_type = <P as Projection>::projection_type();
        trace!(%projection_type, last_event_version, ?aggregate_types, "resyncing projection");

        loop {
            let mut views = HashMap::<String, <P as EventHandler>::View>::new();

            let missing_events = self
                .get_aggregate_events::<<P as EventHandler>::Event>(
                    &aggregate_types,
                    None,
                    last_event_version + 1..last_event_version + 1 + batch_size.size() as i64,
                )
                .await?;

            if missing_events.is_empty() {
                break;
            }

            for missing_event in missing_events {
                let mut view = match views.get_mut(&missing_event.aggregate_id) {
                    Some(view) => view,
                    None => {
                        let view = projection.load_view(&missing_event.aggregate_id).await?;
                        let entry = views.entry(missing_event.aggregate_id.clone());
                        entry.or_insert(view)
                    }
                };

                projection
                    .handle(
                        &mut view,
                        missing_event.event.clone(),
                        missing_event.id,
                        missing_event.sequence,
                    )
                    .await?;
                projection
                    .commit(
                        &missing_event.aggregate_id,
                        &view,
                        missing_event.id,
                        missing_event.sequence,
                    )
                    .await?;

                last_event_version = missing_event.id;
                debug!(%projection_type, last_event_version, ?missing_event, "handled missing event");
            }
        }

        Ok(())
    }
}

mod created_at_format {
    use chrono::{DateTime, FixedOffset};
    use serde::{self, Deserialize, Deserializer, Serializer};

    const FORMAT_SER: &str = "%F %T.%f%z";
    const FORMAT_DE: &str = "%F %T.%f%#z";

    pub fn serialize<S>(date: &DateTime<FixedOffset>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = format!("{}", date.format(FORMAT_SER));
        serializer.serialize_str(&s)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<DateTime<FixedOffset>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        DateTime::parse_from_str(&s, FORMAT_DE).map_err(serde::de::Error::custom)
    }
}
