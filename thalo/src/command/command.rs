use std::fmt;

use crate::Aggregate;

pub trait Command:
    serde::de::DeserializeOwned + serde::ser::Serialize + fmt::Debug + Send + Sync
{
    type Aggregate: Aggregate;

    fn command_type(&self) -> &'static str;
}

#[derive(Clone, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct AggregateCommand<C> {
    pub aggregate_id: String,
    pub command: C,
}
