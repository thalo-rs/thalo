Examples need to be compiled to wasm components and published to the runtime.

## Prerequisites

- Rust with the `wasm32-wasi` targets
- [wasm-tools] cli
- `wasi_snapshot_preview1` adapter (included in this repo)

[wasm-tools]: https://github.com/bytecodealliance/wasm-tools

## Building & Running an Example

Modules need to be built to `wasm32-wasi`, and converted to a wasm component with `wasm-tools`.

Start by building the counter example.

```bash
cargo build -p counter --release --target wasm32-wasi
```

Then you can use the [wasm-tools] cli to convert this to a wasm component using the `wasi_snapshot_preview1.wasm` adapter.

```bash
mkdir ./modules
wasm-tools component new \
  ./target/wasm32-wasi/release/counter.wasm \
  --adapt ./wasi_snapshot_preview1.wasm \
  -o ./modules/counter.wasm
```

The generated `counter.wasm` file will be placed in `./modules`, and is read by thalo at startup.

Start thalo with `cargo r -p thalo_runtime` and you should see a log saying:

```
loaded module from file file="modules/counter.wasm"
```

This means thalo is ready to handle the `Increment` and `Decrement` commands defined in our counter module.

You can execute commands using the `thalo_cli` package:

```bash
cargo run -p thalo_cli -- execute Counter counter-1 Increment '{"amount":1}'
```
