use crate::{Aggregate, AggregateEvent, Error};

pub trait Event: serde::de::DeserializeOwned + serde::ser::Serialize + Send + Sync {
    type Aggregate: Aggregate;

    fn event_type(&self) -> &'static str;

    fn aggregate_event<'a>(&self, aggregate_id: &'a str) -> Result<AggregateEvent<'a>, Error>;
}
