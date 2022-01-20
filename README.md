<p align="center">
  <a href="http://thalo.rs" target="_blank" rel="noopener noreferrer"><img width="124" src="https://raw.githubusercontent.com/thalo-rs/thalo/dev/logo.png" alt="Thalo logo"></a>
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
- [thalo-schema](https://docs.rs/thalo-schema) - Build aggregate schemas into Rust code.
- [thalo-testing](https://docs.rs/thalo-testing) - Test utils for thalo apps.
- [thalo-macros](https://docs.rs/thalo-macros) - Macros for implementing traits. This can be enabled in the core crate with the `macros` feature flag.

Event stores

- [thalo-postgres](https://docs.rs/thalo-kafka) - Postgres implementation of [`EventStore`](https://docs.rs/thalo/latest/thalo/event_store/trait.EventStore.html).
- [thalo-inmemory](https://docs.rs/thalo-inmemory) - In-memory implementation of [`EventStore`](https://docs.rs/thalo/latest/thalo/event_store/trait.EventStore.html).
- [thalo-filestore](https://docs.rs/thalo-filestore) - Filestore implementation of [`EventStore`](https://docs.rs/thalo/latest/thalo/event_store/trait.EventStore.html).

Event streams

- [thalo-kafka](https://docs.rs/thalo-kafka) - Kafka implementation of [`EventStream`](https://docs.rs/thalo/latest/thalo/event_stream/trait.EventStream.html).

## Why

With Rust being a younger language than most, the ecosystem is lacking Event Sourcing & CQRS frameworks. Many of which are abandoned, or just not feature rich. Thalo aims to provide a backbone and some core crates to build robust event sourced systems.

## Examples

Examples can be seen in the [`examples`](/examples) directory.

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
