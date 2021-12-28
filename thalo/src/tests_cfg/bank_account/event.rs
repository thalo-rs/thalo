use serde::{Deserialize, Serialize};

use crate as thalo;
use crate::event::{EventType, IntoIterator};

#[derive(Clone, Debug, Deserialize, EventType, IntoIterator, Serialize)]
#[thalo(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BankAccountEvent {
    OpenedAccount { balance: f64 },
    DepositedFunds { amount: f64 },
    WithdrewFunds { amount: f64 },
}
