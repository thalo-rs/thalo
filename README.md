<p align="center"><a href="http://thalo.rs" target="_blank" rel="noopener noreferrer"><img width="124" src="logo.png" alt="Thalo logo"></a></p>

<p align="center">
  <a href="https://crates.io/crates/thalo"><img src="https://img.shields.io/crates/v/thalo?style=flat-square" alt="Crates.io"></a>
  <a href="https://docs.rs/thalo/latest/thalo/"><img src="https://img.shields.io/docsrs/thalo?style=flat-square" alt="Docs.io"></a>
  <img src="https://img.shields.io/crates/l/thalo?style=flat-square" alt="License">
  <a href="http://makeapullrequest.com"><img src="https://img.shields.io/badge/PRs-welcome-brightgreen.svg?style=flat-square" alt="Pull Requests Welcome"></a>
  <a href="https://github.com/thalo-rs/thalo/stargazers"><img src="https://img.shields.io/github/stars/thalo-rs/thalo?style=flat-square" alt="Stargazers"></a>
  <a href="https://github.com/thalo-rs/thalo/commits"><img src="https://img.shields.io/github/last-commit/thalo-rs/thalo?style=flat-square" alt="Last Commit"></a>
  <a href="https://discord.gg/4Cq8NnPYPA"><img src="https://img.shields.io/discord/913402468895965264?color=%23414EED&label=Discord&logo=Discord&logoColor=%23FFFFFF&style=flat-square" alt="Discord"></a>
</p>

# Thalo

Event sourcing framework for building microservices on an event broker.

- **Fast**: Built with Rust, designed for high throughput and performance.

- **Reliable**: Tollerant to inconsistencies, built for reliability.

- **Scalable**: Follows the CQRS pattern and designed to integrate with Kafka/Redpanda.

## Overview

Thalo is an event-driven framework for building large scale systems based on the following patterns:

- [**Event Sourcing**](https://microservices.io/patterns/data/event-sourcing.html)
- [**CQRS**](https://microservices.io/patterns/data/cqrs.html)
- [**Transactional Outbox**](https://microservices.io/patterns/data/transactional-outbox.html)
- [**Event Driven**](https://martinfowler.com/articles/201701-event-driven.html)
- [**DDD**](https://martinfowler.com/bliki/DomainDrivenDesign.html).

It's designed to be used with [Kafka](https://kafka.apache.org/)/[Redpanda](https://github.com/vectorizedio/redpanda), and intends to keep the API simple with the use of macros.

## Why

With Rust being a little behind when it comes to maturity, the ecosystem is lacking Event Sourcing & CQRS frameworks. Many of which are abandoned, or just not feature rich. Thalo aims to provide everything needed to build event sourced services with an opinionated approach.

## Example

The following is an example of a bank, showing how to define a single command and event.

```rust
use thalo::{commands, events, Aggregate, AggregateType, Error};

#[derive(Aggregate, Clone, Debug, Default)]
pub struct BankAccount {
    #[identity]
    account_number: String,
    balance: f64,
    opened: bool,
}

#[commands]
impl BankAccount {
    /// Creates a command for opening an account
    pub fn open_account(&self, initial_balance: f64) -> Result<AccountOpenedEvent, Error> {
        if self.opened {
            return Err(Error::invariant_code("ACCOUNT_ALREADY_OPENED"));
        }

        // Reference the event created by the BankAccount::account_opened method
        Ok(AccountOpenedEvent { initial_balance })
    }
}

#[events]
impl BankAccount {
    /// Creates an event for when a user opened an account
    pub fn account_opened(&mut self, initial_balance: f64) {
        self.balance = initial_balance;
        self.opened = true;
    }
}
```

The `#[commands]` and `#[events]` attribute macros generate a lot based on your implementations including:

- `BankAccountCommand` enum
- `BankAccountEvent` enum
- `AccountOpenedEvent` struct

_Below are the enums & structs generated:_

```rust
pub enum BankAccountCommand {
    OpenAccount {
        initial_balance: f64,
    },
}

pub enum BankAccountEvent {
    AccountOpened(AccountOpenedEvent),
}

pub struct AccountOpenedEvent {
    initial_balance: f64,
}
```

A full example can be seen at [examples/bank](https://github.com/thalo-rs/thalo/tree/main/examples/bank).

## Getting Help

As Thalo is in pre-release, its API is not stable and documentation is lacking. If you'd like
to get started using Thalo, you can checkout the [examples] directory, or chat with us on our [Discord server].

[examples]: https://github.com/thalo-rs/thalo/tree/main/examples
[discord server]: https://discord.gg/4Cq8NnPYPA

## Contributing

:balloon: Thanks for your help improving the project! We are so happy to have
you! As we don't currently have a contributing guide, you can ping us on the
Discord server or open an issue for any questions/discussions.

## Release Schedule

Thalo doesn't follow a fixed release schedule, but as the project is in pre-release and active development,
you can expect commits on a near daily basis, and version updates evey few days.

## License

This project is licensed under the [MIT] OR [Apache-2.0] license.

[mit]: https://github.com/thalo-rs/thalo/blob/main/LICENSE-MIT
[apache-2.0]: https://github.com/thalo-rs/thalo/blob/main/LICENSE-APACHE

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in Thalo by you, shall be licensed as MIT, without any additional
terms or conditions.
