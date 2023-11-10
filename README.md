[![Thalo — Event sourcing runtime for wasm][splash]](/)

[splash]: https://raw.githubusercontent.com/thalo-rs/thalo/main/splash.svg

[![Crates.io][crates-badge]][crates-url]
[![Documentation][docs-badge]][docs-url]
![MIT OR Apache-2.0][license-badge]
[![Stargazers][stars-badge]][stars-url]
[![Last commit][commits-badge]][commits-url]
[![Discord][discord-badge]][discord-url]

[crates-badge]: https://img.shields.io/crates/v/thalo.svg
[crates-url]: https://crates.io/crates/thalo
[docs-badge]: https://docs.rs/thalo/badge.svg
[docs-url]: https://docs.rs/thalo
[license-badge]: https://img.shields.io/crates/l/thalo
[stars-badge]: https://img.shields.io/github/stars/thalo-rs/thalo.svg
[stars-url]: https://github.com/thalo-rs/thalo/stargazers
[commits-badge]: https://img.shields.io/github/last-commit/thalo-rs/thalo.svg
[commits-url]: https://github.com/thalo-rs/thalo/commits
[discord-badge]: https://img.shields.io/discord/913402468895965264?color=%23414EED&label=Discord&logo=Discord&logoColor=%23FFFFFF
[discord-url]: https://discord.gg/4Cq8NnPYPA

Thalo is an event sourcing runtime that leverages the power of WebAssembly (wasm) through [wasmtime], combined with [sled] as an embedded event store.
It is designed to handle commands using compiled aggregate wasm components and to persist the resulting events, efficiently managing the rebuilding of aggregate states from previous events.

[wasmtime]: https://wasmtime.dev/
[sled]: https://sled.rs/

## Key Features

- **Wasmtime Integration:** High-performance, isolated aggregate computation using WebAssembly.
- **Sled Event Store:** Fast, reliable embedded database for event storage.
- **Actor-Based Aggregates:** Efficient, lock-free management supporting over 10,000 commands/sec.
- **LRU Caching:** Fast aggregate rebuilding with optimized memory usage.
- **Streamlined State Management:** Automated reconstruction of aggregate states from events.
- **Multi-Language Support:** Compile aggregates to wasm, enabling diverse language use (currently Rust supported).
- **Easy Deployment:** Simplified setup focusing on business logic.

## Prerequisites

1. **Rust:** Latest stable version of the Rust programming language.
2. **[Cargo Component]:** A CLI tool for building and managing wasm components.
    - Install with: `cargo install cargo-component`
3. **Thalo Runtime:** Required for running the Thalo event sourcing environment.
    - Install with: `cargo install thalo_runtime`

Thalo can be started by running `thalo`.

For a list of supported arguments, use `thalo --help`.

[cargo component]: https://github.com/bytecodealliance/cargo-component

## Writing an Aggregate

An aggregate is a regular rust crates which exposes the type implementing `thalo::Aggregate` with the `thalo::export_aggregate!` macro.

```rust
thalo::export_aggregate!(Counter);
struct Counter { ... }
impl thalo::Aggregate for Counter { ... }
```

The crate type needs to be set to `cdylib` in the `Cargo.toml`.

```toml
[lib]
crate-type = ["cdylib"]
```

Finally, it can be compiled with [cargo component].

```bash
cargo component build --release
```

The generated `.wasm` module (`target/wasm32-wasi/release/<name>.wasm`) can be placed in a `modules` directory where `thalo` is started.
Thalo will automatically load all modules within this directory, and use them to handle commands.

[cargo component]: https://github.com/bytecodealliance/cargo-component

<details>
  <summary><a href="examples/counter/src/lib.rs">Counter Example</a></summary>

```rust
use serde::{Deserialize, Serialize};
use thalo::{export_aggregate, Aggregate};

export_aggregate!(Counter);

pub struct Counter {
    count: i64,
}

impl Aggregate for Counter {
    type ID = String;
    type Command = Command;
    type Events = Events;
    type Error = &'static str;

    fn init(_id: Self::ID) -> Self {
        Counter { count: 0 }
    }

    fn apply(&mut self, evt: Event) {
        match evt {
            Event::Incremented(IncrementedV1 { amount }) => self.count += amount,
            Event::Decremented(DecrementedV1 { amount }) => self.count -= amount,
        }
    }

    fn handle(&self, cmd: Self::Command) -> Result<Vec<Event>, Self::Error> {
        match cmd {
            Command::Increment { amount } => Ok(vec![Event::Incremented(IncrementedV1 { amount })]),
            Command::Decrement { amount } => Ok(vec![Event::Decremented(DecrementedV1 { amount })]),
        }
    }
}

#[derive(Deserialize)]
#[serde(tag = "command", content = "payload")]
pub enum Command {
    Increment { amount: i64 },
    Decrement { amount: i64 },
}

#[derive(thalo::Events)]
pub enum Events {
    Incremented(IncrementedV1),
    Decremented(DecrementedV1),
}

#[derive(Serialize, Deserialize)]
pub struct IncrementedV1 {
    amount: i64,
}

#[derive(Serialize, Deserialize)]
pub struct DecrementedV1 {
    amount: i64,
}
```
</details>

## Understanding Event Sourcing

Event sourcing is a design pattern in which changes to the application state are stored as a sequence of events. Instead of storing just the current state, event sourcing involves persisting the full series of actions taken on the data. This allows for:

- **Complete State History:** Every change is recorded, enabling full traceability and audit trails.
- **Easy Reversion:** The ability to revert to any previous state by replaying events.
- **Complex Event Reconstruction:** States can be rebuilt or projections created by processing the event log.
- **Improved Scalability and Performance:** Decouples data commands from updates, often leading to better system performance.

Event sourcing is particularly useful in systems where understanding the history of operations is crucial, such as financial systems, order management systems, and other domains where auditability and traceability are important.

## Getting Help

As Thalo is in pre-release, the API is not stable yet.
If you'd like to get started using Thalo, you can checkout the [examples] directory, or chat on the [Discord server].

#### Examples

Examples can found in the [`examples`](examples) directory.

[examples]: https://github.com/thalo-rs/thalo/tree/main/examples
[discord server]: https://discord.gg/4Cq8NnPYPA

## Contributing

:balloon: Thanks for your help improving the project! As we don't currently have a contributing guide, you can ping us on the
Discord server or open an issue.

## License

This project is licensed under the [MIT] OR [Apache-2.0] license.

[mit]: /LICENSE-MIT
[apache-2.0]: /LICENSE-APACHE

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in Thalo by you, shall be licensed as MIT, without any additional
terms or conditions.
