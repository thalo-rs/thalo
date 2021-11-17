use commanded::Aggregate;
use serde::de::Error;

// mod awto;
// mod message;
mod commanded;

pub struct OpenAccount {
    pub account_number: String,
    pub initial_balance: f64,
}

impl commanded::Command for OpenAccount {
    fn identity(&self) -> &String {
        &self.account_number
    }
}

pub enum BankAccountCommand {
    OpenAccount(OpenAccount),
}

pub struct AccountOpened {
    pub account_number: String,
    pub initial_balance: f64,
}

pub enum BankAccountEvent {
    AccountOpened(AccountOpened),
}

pub struct BankAccount {
    account_number: String,
    balance: f64,
}

impl BankAccount {
    pub fn open_account(
        &self,
        account_number: String,
        initial_balance: f64,
    ) -> Result<Vec<BankAccountEvent>, ()> {
        Ok(vec![BankAccountEvent::AccountOpened(AccountOpened {
            account_number,
            initial_balance,
        })])
    }
}

impl Aggregate for BankAccount {
    type Command = BankAccountCommand;
    type Event = BankAccountEvent;
    type Error = ();

    fn execute(&self, command: Self::Command) -> Result<Vec<Self::Event>, Self::Error> {
        match command {
            BankAccountCommand::OpenAccount(payload) => {
                self.open_account(payload.account_number, payload.initial_balance)
            }
        }
    }

    fn apply(&mut self, event: Self::Event) {
        match event {
            BankAccountEvent::AccountOpened(payload) => {
                self.account_number = payload.account_number;
                self.balance = payload.initial_balance;
            }
        }
    }
}

// pub struct A;
// impl awto::DomainAggregate for A {
//     type Event = message::Event;
//     type Command = message::Command;
// }
// pub struct B;
// impl awto::DomainAggregate for B {
//     type Event = message::Event;
//     type Command = message::Command;
// }
// pub struct C;
// impl awto::DomainAggregate for C {
//     type Event = message::Event;
//     type Command = message::Command;
// }

// pub struct Service;

// impl awto::DomainService for Service {
//     type Aggregates = ();
//     type Projections = ();
// }

// https://jaysoo.ca/2015/02/06/what-the-flux/
// There are several object roles in CQRS.
//
// Query Model
// Query Processor
// Command Model (Aggregate)
// Commands
// Command Handler
// Domain Event
// Domain Event Publisher
// Event Subscriber
//
// https://stackoverflow.com/questions/31386244/cqrs-event-sourcing-check-username-is-unique-or-not-from-eventstore-while-sendin
