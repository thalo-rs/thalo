pub trait StreamTopic {
    fn stream_topic() -> &'static str;
}

pub trait MultiStreamTopic {
    fn stream_topics() -> Vec<&'static str>;
}

impl<T: StreamTopic> MultiStreamTopic for T {
    fn stream_topics() -> Vec<&'static str> {
        vec![T::stream_topic()]
    }
}
