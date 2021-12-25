use serde::{Deserialize, Serialize};

use crate::event::{EventType, IntoEvents};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum BankAccountEvent {
    OpenedAccount { balance: f64 },
    DepositedFunds { amount: f64 },
    WithdrewFunds { amount: f64 },
}

impl EventType for BankAccountEvent {
    fn event_type(&self) -> &'static str {
        use BankAccountEvent::*;

        match self {
            OpenedAccount { .. } => "OpenedAccount",
            DepositedFunds { .. } => "DepositedFunds",
            WithdrewFunds { .. } => "WithdrewFunds",
        }
    }
}

impl IntoEvents for BankAccountEvent {
    type Event = Self;

    fn into_events(self) -> Vec<Self::Event> {
        vec![self]
    }
}
