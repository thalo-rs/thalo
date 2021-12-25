use rdkafka::consumer::StreamConsumer;

/// Kafka config.
pub struct KafkaConfig {
    pub(crate) consumer: StreamConsumer,
}

impl KafkaConfig {
    pub fn new_from_projection() {}
}
