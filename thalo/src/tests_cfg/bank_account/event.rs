use serde::{Deserialize, Serialize};

use crate as thalo;
use crate::event::EventType;

#[derive(Clone, Debug, Deserialize, EventType, Serialize)]
pub enum BankAccountEvent {
    OpenedAccount(OpenedAccountEvent),
    DepositedFunds(DepositedFundsEvent),
    WithdrewFunds(WithdrewFundsEvent),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct OpenedAccountEvent {
    pub balance: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DepositedFundsEvent {
    pub amount: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct WithdrewFundsEvent {
    pub amount: f64,
}
