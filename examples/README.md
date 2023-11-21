Examples need to be compiled to wasm components and published to the runtime.

## Prerequisites

1. **Rust:** Latest stable version of the Rust programming language.
2. **Thalo:** Required for building aggregates to wasm.
    - Install with: `cargo install thalo_cli`
2. **Thalo Runtime:** Required for running the Thalo event sourcing environment.
    - Install with: `cargo install thalo_runtime`

## Building & Running an Example

Aggregates need to be build to wasm components, and placed in the `./modules` directory where thalo_runtime is started.

Start by building the counter example.

```bash
thalo build counter -o ./modules
```

The generated `counter.wasm` file will be placed in `./modules`, and is read by thalo at startup.

Start thalo with `cargo r -p thalo_runtime` and you should see a log saying:

```
loaded module from file file="modules/counter.wasm"
```

This means thalo is ready to handle the `Increment` command defined in our counter module.

You can execute commands using the `thalo_cli` package:

```bash
cargo run -p thalo_cli -- execute Counter abc123 Increment '{"amount":10}'
```
