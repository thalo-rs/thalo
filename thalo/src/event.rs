use std::fmt;

use async_trait::async_trait;

use crate::Error;

/// An `Event` is something that happened as a result to a [`Command`].
///
/// Typically a type which implements this trait would be an enum.
///
/// # Examples
///
/// ```no_run
/// #[derive(Event)]
/// enum BankAccountEvent {
///     AccountOpened(AccountOpenedEvent),
///     FundsDeposited(FundsDepositedEvent),
///     FundsWithdrawn(FundsWithdrawnEvent),
/// }
/// ```
pub trait Event:
    serde::de::DeserializeOwned + serde::ser::Serialize + Clone + fmt::Debug + PartialEq + Send + Sync
{
    /// The event varaint identifier type.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// fn event_type(&self) -> &'static str {
    ///     match self {
    ///         BankAccountEvent::AccountOpened(_) => "AccountOpened",
    ///         BankAccountEvent::FundsDeposited(_) => "FundsDeposited",
    ///         BankAccountEvent::FundsWithdrawn(_) => "FundsWithdrawn",
    ///     }
    /// }
    /// ```
    fn event_type(&self) -> &'static str;
}

/// An EventHandler handles new events to update its internal state.
#[async_trait]
pub trait EventHandler {
    type Event;
    type Error;

    /// Handle an event and return an updated view.
    async fn handle(&mut self, event: Self::Event) -> Result<(), Error>;
}

/// Helper trait to extract an event from a list of events.
pub trait EventView<E> {
    /// Extract an event from a list of events, and error if it isn't present.
    ///
    /// # Example
    ///
    /// ```no_run
    /// let event = EventView::<FundsDepositedEvent>::view(&events)?;
    /// println!("Amount: {}", event.amount);
    /// ```
    fn view(&self) -> Result<&E, Error>;

    /// Extract an event from a list of events. If the event is not present,
    /// `None` will be returned.
    ///
    /// # Example
    ///
    /// ```no_run
    /// let event = EventView::<FundsDepositedEvent>::view(&events);
    /// match event {
    ///     Some(event) => println!("Amount: {}", event.amount),
    ///     None => println!("Event wasn't present"),
    /// }
    /// ```
    fn view_opt(&self) -> Option<&E>;
}
