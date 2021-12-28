/// A Kafka topic.
pub trait Topic {
    /// Returns a list of kafka topics.
    fn topics() -> Vec<&'static str>;
}
