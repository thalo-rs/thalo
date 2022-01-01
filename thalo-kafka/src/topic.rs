/// A Kafka topic.
pub trait Topic {
    /// Returns a kafka topic.
    fn topic(&self) -> &'static str;
}
