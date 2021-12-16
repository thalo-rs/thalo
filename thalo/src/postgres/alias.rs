use actix::Message;
use async_trait::async_trait;
use bb8_postgres::tokio_postgres::NoTls;

use crate::{
    Aggregate, AggregateChannel, BaseAggregateActor, Command, Error, EventEnvelope, StreamTopic,
};

use super::PgEventStore;

#[async_trait]
pub trait SendCommand<C>
where
    C: Command,
{
    async fn do_send_command(id: &str, command: C) -> Result<(), Error>;

    async fn send_command(
        id: &str,
        command: C,
    ) -> Result<Vec<EventEnvelope<<<C as Command>::Aggregate as Aggregate>::Event>>, Error>;
}

#[async_trait]
impl<A> SendCommand<<A as Aggregate>::Command> for A
where
    A: Aggregate
        + AggregateChannel<BaseAggregateActor<PgEventStore<NoTls>, A>, PgEventStore<NoTls>>
        + Clone
        + Unpin
        + 'static,
    <A as Aggregate>::Command:
        Message<Result = Result<Vec<EventEnvelope<<Self as Aggregate>::Event>>, Error>> + Unpin,
    <A as Aggregate>::Event: StreamTopic + Unpin + 'static,
{
    async fn do_send_command(id: &str, command: <A as Aggregate>::Command) -> Result<(), Error> {
        AggregateChannel::<BaseAggregateActor<PgEventStore<NoTls>, A>, PgEventStore<NoTls>>::do_send(
            id, command,
        )
        .await
    }

    async fn send_command(
        id: &str,
        command: <A as Aggregate>::Command,
    ) -> Result<Vec<EventEnvelope<<A as Aggregate>::Event>>, Error> {
        AggregateChannel::<BaseAggregateActor<PgEventStore<NoTls>, A>, PgEventStore<NoTls>>::send(
            id, command,
        )
        .await
    }
}
