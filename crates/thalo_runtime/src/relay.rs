use std::sync::Arc;

use redis::aio::MultiplexedConnection;
use redis::streams::StreamMaxlen;
use redis::ToRedisArgs;
use thalo::event_store::message::Message;
use thalo::stream_name::Category;

#[derive(Clone)]
pub enum Relay {
    Noop,
    Redis(RedisRelay),
}

impl Relay {
    pub fn stream_name(&self, category: Category<'_>) -> String {
        match self {
            Relay::Noop => category.into_string(),
            Relay::Redis(redis_relay) => redis_relay.stream_name(category),
        }
    }

    pub async fn relay<'a>(
        &mut self,
        stream_name: &str,
        batch: Vec<Message<'a>>,
    ) -> anyhow::Result<()> {
        match self {
            Relay::Noop => Ok(()),
            Relay::Redis(redis_relay) => redis_relay.relay(stream_name, batch).await,
        }
    }
}

#[derive(Clone)]
pub struct RedisRelay {
    conn: MultiplexedConnection,
    stream_max_len: StreamMaxlen,
    stream_name_template: Arc<str>,
}

impl RedisRelay {
    pub fn new(
        conn: MultiplexedConnection,
        stream_max_len: StreamMaxlen,
        stream_name_template: impl Into<Arc<str>>,
    ) -> Self {
        RedisRelay {
            conn,
            stream_max_len,
            stream_name_template: stream_name_template.into(),
        }
    }

    pub fn stream_name(&self, category: Category<'_>) -> String {
        self.stream_name_template.replace("{category}", &category)
    }

    pub async fn relay<'a>(
        &mut self,
        stream_name: &str,
        batch: Vec<Message<'a>>,
    ) -> anyhow::Result<()> {
        if !batch.is_empty() {
            let mut pipe = redis::pipe();
            for msg in batch {
                let msg_data = serde_json::to_string(&msg)?;
                pipe.xadd_maxlen(
                    stream_name.to_redis_args(),
                    self.stream_max_len,
                    "*",
                    &[
                        ("event_type", (&*msg.msg_type).to_redis_args()),
                        ("event", msg_data.to_redis_args()),
                    ],
                );
            }
            pipe.query_async(&mut self.conn).await?;
        }

        Ok(())
    }
}
