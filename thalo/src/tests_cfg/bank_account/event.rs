use serde::{Deserialize, Serialize};

use crate as thalo;
use crate::event::{EventType, IntoEvents};

#[derive(Clone, Debug, Deserialize, Serialize, EventType)]
#[thalo(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BankAccountEvent {
    OpenedAccount { balance: f64 },
    DepositedFunds { amount: f64 },
    WithdrewFunds { amount: f64 },
}

impl IntoEvents for BankAccountEvent {
    type Event = Self;

    fn into_events(self) -> Vec<Self::Event> {
        vec![self]
    }
}
