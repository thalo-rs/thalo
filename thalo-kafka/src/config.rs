use rdkafka::{config::RDKafkaLogLevel, ClientConfig};

/// A wrapper around [`rdkafka::ClientConfig`] for convenience.
///
/// Provides recommended config with `KafkaClientConfig::new_recommended` which is configured for
/// at-least-once kafka semantic. Offsets should be manually added to the store.
/// For more info, see the [`rust-rdkafka/examples/at_least_once.rs`](https://github.com/fede1024/rust-rdkafka/blob/master/examples/at_least_once.rs) example.
pub struct KafkaClientConfig(ClientConfig);

impl KafkaClientConfig {
    /// Create a new `KafkaClientConfig` from an existing [`rdkafka::ClientConfig`].
    pub fn new(config: ClientConfig) -> Self {
        KafkaClientConfig(config)
    }

    /// Create a new `KafkaClientConfig` with a recommended configuration with at-least-once semantics.
    ///
    /// Along with `group.id` and `bootstrap.servers`, the following config items are set:
    ///
    /// - `enable.partition.eof=false`
    /// - `session.timeout.ms=6000`
    /// - `enable.auto.commit=true`
    /// - `auto.commit.interval.ms=5000`
    /// - `enable.auto.offset.store=false`
    pub fn new_recommended(group_id: impl Into<String>, brokers: impl Into<String>) -> Self {
        let mut config = ClientConfig::new();
        config
            .set("group.id", group_id)
            .set("bootstrap.servers", brokers)
            .set("enable.partition.eof", "false")
            .set("session.timeout.ms", "6000")
            .set("enable.auto.commit", "true")
            .set("auto.commit.interval.ms", "5000")
            .set("enable.auto.offset.store", "false")
            .set_log_level(RDKafkaLogLevel::Debug);

        KafkaClientConfig(config)
    }

    /// Return the inner [`rdkafka::ClientConfig`].
    pub fn into_inner(self) -> ClientConfig {
        self.0
    }

    /// Sets a configuration key on the inner [`rdkafka::ClientConfig`].
    pub fn set<K, V>(&mut self, key: K, value: V) -> &mut Self
    where
        K: Into<String>,
        V: Into<String>,
    {
        self.as_mut().set(key, value);
        self
    }
}

impl From<KafkaClientConfig> for ClientConfig {
    fn from(config: KafkaClientConfig) -> Self {
        config.into_inner()
    }
}

impl AsRef<ClientConfig> for KafkaClientConfig {
    fn as_ref(&self) -> &ClientConfig {
        &self.0
    }
}

impl AsMut<ClientConfig> for KafkaClientConfig {
    fn as_mut(&mut self) -> &mut ClientConfig {
        &mut self.0
    }
}

#[cfg(test)]
mod tests {
    use super::KafkaClientConfig;

    #[test]
    fn new_recommended() {
        let config = KafkaClientConfig::new_recommended("my_group", "broker1,broker2");
        let inner_config = config.into_inner();
        assert_eq!(inner_config.get("group.id"), Some("my_group"));
        assert_eq!(
            inner_config.get("bootstrap.servers"),
            Some("broker1,broker2")
        );
        assert_eq!(inner_config.get("enable.partition.eof"), Some("false"));
        assert_eq!(inner_config.get("session.timeout.ms"), Some("6000"));
        assert_eq!(inner_config.get("enable.auto.commit"), Some("true"));
        assert_eq!(inner_config.get("auto.commit.interval.ms"), Some("5000"));
        assert_eq!(inner_config.get("enable.auto.offset.store"), Some("false"));
    }

    #[test]
    fn set() {
        let mut config = KafkaClientConfig::new_recommended("my_group", "broker1,broker2");
        config.set("group.id", "overwritten");

        let inner_config = config.into_inner();
        assert_eq!(inner_config.get("group.id"), Some("overwritten"));
    }
}
