use crate::{Aggregate, Context, Error};

pub trait Events {
    type Aggregate: Aggregate;

    fn apply(
        state: &mut Self::Aggregate,
        ctx: Context,
        event_type: &str,
        payload: Vec<u8>,
    ) -> Result<(), Error>;
    fn event_type(&self) -> &'static str;
    fn payload(self) -> Result<Vec<u8>, Error>;
}

pub trait Event {
    type Aggregate: Aggregate;

    fn apply(self, state: &mut Self::Aggregate, ctx: Context);
    fn event_type() -> &'static str;
}
