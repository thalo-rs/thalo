pub trait Command: serde::de::DeserializeOwned + serde::ser::Serialize + Send + Sync {
    fn command_type(&self) -> &'static str;
}
