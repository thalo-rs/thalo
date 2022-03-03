pub use event_store::{ESDBEventStore, ESDBEventPayload};
pub use error::Error;

mod error;
mod event_store;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
