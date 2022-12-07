Examples need to be compiled to wasm components and published to the runtime.

## Prerequisites

- Rust with the `wasm32-wasi` and `wasm32-unknown-unknown` targets
- [wasm-tools] cli
- [`wasi_snapshot_preview1` adapter]

[`wasi_snapshot_preview1` adapter]: #building-wasi_snapshot_preview1-adapter

## Building `wasi_snapshot_preview1` adapter

At the time of writing, the adapater can be compiled from the [bytecodealliance/preview2-prototyping] repository.

```bash
git clone https://github.com/bytecodealliance/preview2-prototyping.git
cd preview2-prototyping
```

The `wasi_snapshot_preview1.wasm` module needs to be built to the `wasm32-unknown-unknown` target.

```bash
cargo build --target wasm32-unknown-unknown --release
```

This will build a wasm file in `target/wasm32-unknown-unknown/release/wasi_snapshot_preview1.wasm`.
This can be used as the adapater to convert the wasm module to a component with the wasm-tools cli.

## Building an example

Modules need to be built to wasm32-wasi, and converted to a wasm component.

Start by building the counter example.

```bash
cargo build -p counter --release --target wasm32-wasi
```

Then you can use the [wasm-tools] cli to convert this to a wasm component using the `wasi_snapshot_preview1.wasm` adapter.

```bash
wasm-tools component new \
  ./target/wasm32-wasi/release/counter.wasm \
  --adapt ../../bytecodealliance/preview2-prototyping/target/wasm32-unknown-unknown/release/wasi_snapshot_preview1.wasm \
  -o ./counter.component.wasm
```

You should be left with a `counter.component.wasm` file. You can publish this to the runtime using thalo-cli.

```bash
cargo run -p thalo_cli -- --url "http://localhost:4433" publish ./examples/counter/counter.esdl ./counter.component.wasm
```

Once published, you can finally execute a command.

```bash
cargo run -p thalo_cli -- --url "http://localhost:4433" execute Counter counter-1 increment '{"amount":1}'
```

[wasm-tools]: https://github.com/bytecodealliance/wasm-tools
[bytecodealliance/preview2-prototyping]: https://github.com/bytecodealliance/preview2-prototyping
[wit-component]: https://github.com/bytecodealliance/wit-bindgen/tree/main/crates/wit-component
[`wasi_snapshot_preview1`]: https://github.com/bytecodealliance/wit-bindgen/tree/main/crates/wasi_snapshot_preview1
