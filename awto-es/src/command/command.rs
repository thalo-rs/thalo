use std::fmt;

pub trait Command:
    serde::de::DeserializeOwned + serde::ser::Serialize + fmt::Debug + Send + Sync
{
    fn command_type(&self) -> &'static str;
}

#[derive(Clone, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct AggregateCommand<C> {
    pub aggregate_id: String,
    pub command: C,
}
