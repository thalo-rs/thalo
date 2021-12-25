pub trait Command:
    serde::de::DeserializeOwned + serde::ser::Serialize + fmt::Debug + Send + Sync
{
    type Aggregate: Aggregate;

    fn command_type(&self) -> &'static str;
}
