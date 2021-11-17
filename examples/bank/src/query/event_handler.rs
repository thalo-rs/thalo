use awto_es::{
    postgres::PgEventStore, AggregateEventOwned, Error, ErrorKind, EventHandler, EventStore,
    InternalError,
};
use bb8_postgres::tokio_postgres::NoTls;
use rdkafka::{
    consumer::{Consumer, StreamConsumer},
    Message,
};
use tracing::{debug, span, trace, Instrument, Level};

use crate::{command::BankAccountEvent, util::loop_result_async};

use super::{BankAccountProjector, BankAccountViewRepository};

pub async fn event_handler(
    event_store: PgEventStore<NoTls>,
    repository: BankAccountViewRepository,
    consumer: StreamConsumer,
) {
    let mut projector = BankAccountProjector::new(event_store.clone(), repository.clone());

    consumer
        .subscribe(&["bank-account-event"])
        .expect("Can't subscribe to bank-account");

    event_store
        .resync_projection(&mut projector)
        .await
        .expect("could not resync projection");

    loop_result_async(|| {
        async {
            let msg = consumer
                .recv()
                .await
                .internal_error("could not receive message from event handler")?;
            let offset = msg.offset();
            let key = msg
                .key_view::<str>()
                .transpose()
                .internal_error("could not read key as str")?
                .ok_or_else(|| Error::new_simple(ErrorKind::MissingKey))?;

            if let Some(payload) = msg.payload() {
                let event_envelope: AggregateEventOwned = serde_json::from_slice(payload)
                    .map_err(|err| Error::new(ErrorKind::DeserializeError, err))?;
                let event_id = event_envelope.id;
                let event_sequence = event_envelope.sequence;
                let event: BankAccountEvent = serde_json::from_value(event_envelope.event_data)
                    .map_err(|err| Error::new(ErrorKind::DeserializeError, err))?;
                debug!(
                    offset,
                    key,
                    event_id,
                    event_sequence,
                    ?event,
                    "received event"
                );

                projector
                    .clone()
                    .handle(key.to_string(), event, event_id, event_sequence)
                    .await?;

                trace!("handled projector");
            }

            Result::<_, awto_es::Error>::Ok(())
        }
        .instrument(span!(Level::DEBUG, "event_handler"))
    })
    .await;
}
