<p align="center">
  <a href="http://thalo.rs" target="_blank" rel="noopener noreferrer"><img width="124" src="https://raw.githubusercontent.com/thalo-rs/thalo/main/logo.png" alt="Thalo logo"></a>
</p>

<h1 align="center">Thalo</h1>
<p align="center">Event sourcing framework for building microservices.</p>

<p align="center">
  <a href="https://crates.io/crates/thalo"><img src="https://img.shields.io/crates/v/thalo?style=flat-square" alt="Crates.io"></a>
  <a href="https://docs.rs/thalo/latest/thalo/"><img src="https://img.shields.io/docsrs/thalo?style=flat-square" alt="Docs.io"></a>
  <img src="https://img.shields.io/crates/l/thalo?style=flat-square" alt="License">
  <a href="http://makeapullrequest.com"><img src="https://img.shields.io/badge/PRs-welcome-brightgreen.svg?style=flat-square" alt="Pull Requests Welcome"></a>
  <a href="https://github.com/thalo-rs/thalo/stargazers"><img src="https://img.shields.io/github/stars/thalo-rs/thalo?style=flat-square" alt="Stargazers"></a>
  <a href="https://github.com/thalo-rs/thalo/commits"><img src="https://img.shields.io/github/last-commit/thalo-rs/thalo/dev?style=flat-square" alt="Last Commit"></a>
  <a href="https://discord.gg/4Cq8NnPYPA"><img src="https://img.shields.io/discord/913402468895965264?color=%23414EED&label=Discord&logo=Discord&logoColor=%23FFFFFF&style=flat-square" alt="Discord"></a>
</p>

## Overview

Thalo is an event-sourcing framework for building large scale systems based on the following patterns:

- [**Event Sourcing**](https://microservices.io/patterns/data/event-sourcing.html)
- [**CQRS**](https://microservices.io/patterns/data/cqrs.html)
- [**Event Driven**](https://martinfowler.com/articles/201701-event-driven.html)
- [**Transactional Outbox**](https://microservices.io/patterns/data/transactional-outbox.html)
- [**DDD**](https://martinfowler.com/bliki/DomainDrivenDesign.html)

It's designed to be modular with additional crates implementing most functionality.

**Official Crates**

Core

- [thalo](https://docs.rs/thalo) - Core framework.
- [thalo-testing](https://docs.rs/thalo-testing) - Test utils for thalo apps.
- [thalo-macros](https://docs.rs/thalo-macros) - Macros for implementing traits. This can be enabled in the core crate with the `macros` feature flag.

Event stores

- [thalo-postgres](https://docs.rs/thalo-postgres) - Postgres implementation of [`EventStore`](https://docs.rs/thalo/latest/thalo/event_store/trait.EventStore.html).
- [thalo-inmemory](https://docs.rs/thalo-inmemory) - In-memory implementation of [`EventStore`](https://docs.rs/thalo/latest/thalo/event_store/trait.EventStore.html).
- [thalo-filestore](https://docs.rs/thalo-filestore) - Filestore implementation of [`EventStore`](https://docs.rs/thalo/latest/thalo/event_store/trait.EventStore.html).

Event streams

- [thalo-kafka](https://docs.rs/thalo-kafka) - Kafka implementation of [`EventStream`](https://docs.rs/thalo/latest/thalo/event_stream/trait.EventStream.html).

## What is Event Sourcing & CQRS

Event sourcing is a way of storing data as an immutable sequence of events.
Events are facts that occured in your system and cannot be undone.

Put simply, event sourcing is: `f(state, event) -> state`

Rather than the traditional state-oriented approach, your data consists of small events which occur in your system,
and your models can be built by replaying these events one by one to compute read models.

A common example of event sourcing is accounting. Your bank balance is the sum of all your transactions (aka events).
Another example you're likely familiar with that uses event sourcing is Git scm.

**What are the benefits?**

- **Scalability**

  Event sourced systems can operate in a very loosely coupled parallel style which provides excellent horizontal scalability and resilience to systems failure.

- **Time Travel**

  By storing immutable events, you have the ability to determine application state at any point in time.

- **100% Accurate Audit Logging**

  With event sourcing, each state change corresponds to one or more events, providing 100% accurate audit logging.

- **Expressive Models**

  Events are first class objects in your system and show intent behind data changes. It makes the implicit explicit.

- **Separation of Concerns**

  Commands and queries in your system are independent of eachother. This allows them to be scaled & developed independently.

**What are the down sides?**

As with anything in the tech world, everything is about tradeoffs.
There are some reasons not to use event sourcing in your system including:

- **Eventual Consistency**: With the separation of concerns comes eventual consistency in your system, meaning your data may not be up to date immediately, but eventually will become consistent.

- **High Disk Usage**: With all events being stored forever, disk usage will grow overtime. Though there are solutions to this such as [snapshotting](https://domaincentric.net/blog/event-sourcing-snapshotting).

- **Event Granularity**: Sometimes it can be difficult to judge an appropriate size for events. Having them too small may not provide enough data, while large events may slow down your system.

**Resources**

Here are some useful resources on event sourcing & CQRS:

- https://microservices.io/patterns/data/event-sourcing.html
- https://moduscreate.com/blog/cqrs-event-sourcing/
- https://medium.com/@hugo.oliveira.rocha/what-they-dont-tell-you-about-event-sourcing-6afc23c69e9a

## Why

With Rust being a younger language than most, the ecosystem is lacking Event Sourcing & CQRS frameworks. Many of which are abandoned, or just not feature rich. Thalo aims to provide a backbone and some core crates to build robust event sourced systems.

## [ESDL](https://github.com/thalo-rs/esdl) - Event-sourcing Schema Definition Language

Defining aggregates, commands and events are recommended to be done in the [ESDL schema language](https://github.com/thalo-rs/esdl).

This allows for more readable aggregate definitions and provides code generation to generate events,
command trait and custom types compatible with Thalo.

An example of an `.esdl` can be found in [`examples/bank-account/bank-account.esdl`](/examples/bank-account/bank-account.esdl).

## Examples

Examples include:

- [**bank-account**](/examples/bank-account): a bank account aggregate built with an ESDL schema file.
- [**protobuf**](/examples/protobuf): a gRPC client & server for sending syncronous commands.

All examples can be seen in the [`examples`](/examples) directory.

## Getting Help

As Thalo is in pre-release, the API is not stable yet.
If you'd like to get started using Thalo, you can checkout the [examples] directory,
or chat with us on our [Discord server].

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

[mit]: /LICENSE-MIT
[apache-2.0]: /LICENSE-APACHE

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in Thalo by you, shall be licensed as MIT, without any additional
terms or conditions.
