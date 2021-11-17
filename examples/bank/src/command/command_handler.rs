use std::env;

use actix::{Actor, Addr};
use awto_es::{postgres::PgEventStore, Error, ErrorKind, InternalError};
use bb8_postgres::tokio_postgres::NoTls;
use lazy_static::lazy_static;
use lru::LruCache;
use rdkafka::{
    consumer::{Consumer, StreamConsumer},
    producer::FutureProducer,
    Message,
};
use tokio::sync::Mutex;
use tracing::{debug, info, span, Instrument, Level};

use crate::{command::BankAccountCommand, util::loop_result_async};

use super::BankAccountActor;

lazy_static! {
    static ref BANK_ACCOUNT_ACTORS: Mutex<LruCache<String, Addr<BankAccountActor>>> =
        Mutex::new(LruCache::new(
            env::var("AGGREGATE_LRU_CAP").map_or(256_usize, |cap| cap.parse().unwrap_or(256_usize))
        ));
}

pub async fn command_handler(
    event_store: PgEventStore<NoTls>,
    consumer: StreamConsumer,
    producer: FutureProducer,
) {
    consumer
        .subscribe(&["bank-account-command"])
        .expect("Can't subscribe to bank-account-command");
    println!("{:?}", consumer.subscription());

    loop_result_async(|| {
        async {
            debug!("waiting for command...");
            let msg = consumer
                .recv()
                .await
                .internal_error("could not receive message from command handler")?;
            info!("received command");
            let offset = msg.offset();
            let key = msg
                .key_view::<str>()
                .transpose()
                .internal_error("could not read key as str")?
                .ok_or_else(|| Error::new_simple(ErrorKind::MissingKey))?;

            if let Some(payload) = msg.payload() {
                let command = serde_json::from_slice::<BankAccountCommand>(payload)
                    .map_err(|err| Error::new(ErrorKind::DeserializeError, err))?;
                debug!(
                    target: "received command",
                    offset,
                    key,
                    ?command,
                );

                {
                    let mut guard = BANK_ACCOUNT_ACTORS.lock().await;
                    let actor = match guard.get(key) {
                        Some(actor) => {
                            info!("actor exists in cache");
                            actor
                        }
                        None => {
                            info!("actor does not in cahce, loading...");
                            let actor = BankAccountActor::new(
                                key.to_string(),
                                event_store.clone(),
                                producer.clone(),
                            );
                            guard.put(key.to_string(), actor.start());
                            guard.get(key).expect("item should exist")
                        }
                    };

                    actor.send(command)
                }
                .await
                .map_err(|_err| Error::new_simple(ErrorKind::MailboxFull))??;
            }

            Result::<_, awto_es::Error>::Ok(())
        }
        .instrument(span!(Level::DEBUG, "command_handler"))
    })
    .await;
}
