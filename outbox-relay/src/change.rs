use std::time::Duration;

use bb8_postgres::{
    bb8::Pool,
    tokio_postgres::{
        tls::{MakeTlsConnect, TlsConnect},
        Socket,
    },
    PostgresConnectionManager,
};
use rdkafka::producer::{FutureProducer, FutureRecord};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use tracing::trace;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SlotChange {
    pub change: Vec<Change>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Change {
    pub kind: String,
    pub schema: String,
    pub table: String,
    pub columnnames: Option<Vec<String>>,
    pub columntypes: Option<Vec<String>>,
    pub columnvalues: Option<Vec<Value>>,
}

#[derive(Debug, Serialize)]
pub struct Event {
    pub id: i64,
    pub created_at: String,
    pub aggregate_type: String,
    pub aggregate_id: String,
    pub sequence: i64,
    pub event_type: String,
    pub event_data: Map<String, Value>,
}

impl TryFrom<Vec<Value>> for Event {
    type Error = i8;

    fn try_from(values: Vec<Value>) -> Result<Self, Self::Error> {
        macro_rules! get_value {
            ($values: ident, $idx: expr, $f: ident) => {
                $values.get($idx).ok_or($idx)?.$f().ok_or($idx)?
            };
        }

        let event_data: Map<String, Value> = match values.get(6).ok_or(6)? {
            Value::String(s) => serde_json::from_str(s).map_err(|_| 10)?,
            Value::Object(map) => map.clone(),
            _ => return Err(12),
        };

        Ok(Event {
            id: get_value!(values, 0, as_i64),
            created_at: get_value!(values, 1, as_str).to_string(),
            aggregate_type: get_value!(values, 2, as_str).to_string(),
            aggregate_id: get_value!(values, 3, as_str).to_string(),
            sequence: get_value!(values, 4, as_i64),
            event_type: get_value!(values, 5, as_str).to_string(),
            event_data,
        })
    }
}

pub async fn handle_event<Tls>(
    event: Event,
    producer: FutureProducer,
    pool: Pool<PostgresConnectionManager<Tls>>,
) -> Result<(), Box<dyn std::error::Error>>
where
    Tls: MakeTlsConnect<Socket> + Clone + Send + Sync + 'static,
    <Tls as MakeTlsConnect<Socket>>::Stream: Send + Sync,
    <Tls as MakeTlsConnect<Socket>>::TlsConnect: Send,
    <<Tls as MakeTlsConnect<Socket>>::TlsConnect as TlsConnect<Socket>>::Future: Send,
{
    let topic = format!("{}_event", &event.aggregate_type);
    let payload = serde_json::to_vec(&event)?;

    let delivery_status = producer
        .send(
            FutureRecord::to(&topic)
                .payload(&payload)
                .key(&event.aggregate_id),
            Duration::from_secs(5),
        )
        .await
        .map_err(|(kafka_error, _)| kafka_error)?;
    trace!(partition = delivery_status.0, offset = delivery_status.1, %topic, key = %event.aggregate_id, "sent message");

    let conn = pool.get().await?;
    conn.execute(
        r#"
        DELETE FROM "outbox"
        WHERE "id" = $1
        "#,
        &[&event.id],
    )
    .await?;

    Ok(())
}
