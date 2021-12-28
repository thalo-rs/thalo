# Changelog
All notable changes to this project will be documented in this file. See [conventional commits](https://www.conventionalcommits.org/) for commit guidelines.

- - -
## [0.3.3](https://github.com/thalo-rs/thalo/compare/0.3.2..0.3.3) - 2021-12-28
#### Build system
- add thalo-testing to pre bump hook - ([738e011](https://github.com/thalo-rs/thalo/commit/738e0112bcc2b076ef3b69cf67dad633252a9741)) - [@tqwewe](https://github.com/tqwewe)
#### Documentation
- **(thalo)** add inconsistent missing periods - ([dc0b78a](https://github.com/thalo-rs/thalo/commit/dc0b78a7774d21b326547bcff09f0fe1186c04d4)) - [@tqwewe](https://github.com/tqwewe)
#### Features
- **(thalo-testing)** add package `thalo-testing` - ([bcf11a9](https://github.com/thalo-rs/thalo/commit/bcf11a9fa270cbcd32afb57b6454ef2876f8e10e)) - [@tqwewe](https://github.com/tqwewe)
#### Miscellaneous Chores
- merge branch 'dev' - ([5cae915](https://github.com/thalo-rs/thalo/commit/5cae915b342bf171b7c14f5407761c367b462a0a)) - [@tqwewe](https://github.com/tqwewe)
- merge branch 'main' into dev - ([4858067](https://github.com/thalo-rs/thalo/commit/48580671bf21fd7cfb04bd1f6b14795a4f659cd8)) - [@tqwewe](https://github.com/tqwewe)
- merge branch 'dev' - ([0f03acc](https://github.com/thalo-rs/thalo/commit/0f03acc5b8a4361d69a298cde8e21fb7fb3f369f)) - [@tqwewe](https://github.com/tqwewe)
- merge branch 'main' into dev - ([d94d279](https://github.com/thalo-rs/thalo/commit/d94d2795c128c9f7467a60c0026824a0038481dd)) - [@tqwewe](https://github.com/tqwewe)
- - -

## [0.3.2](https://github.com/thalo-rs/thalo/compare/0.3.1..0.3.2) - 2021-12-27
#### Build system
- **(thalo)** add feature flags in documentation - ([055d15d](https://github.com/thalo-rs/thalo/commit/055d15d2f6fe1486e2f488f6a55354cf1baa219b)) - [@tqwewe](https://github.com/tqwewe)
- remove delay between publishing packages - ([823699d](https://github.com/thalo-rs/thalo/commit/823699de1eccefb3f73cffc29b7aab6d2a9b1917)) - [@tqwewe](https://github.com/tqwewe)
#### Features
- **(thalo)** add `Item` associated type to `EventStream` - ([b9484e5](https://github.com/thalo-rs/thalo/commit/b9484e563bd1d8536a18d6c36cf657181f0f7fde)) - [@tqwewe](https://github.com/tqwewe)
- **(thalo-kafka)** add `KafkaEventStream::watch_event_handler` method to handle event handlers - ([c7c1c16](https://github.com/thalo-rs/thalo/commit/c7c1c16eec8e42546daf8c47626596c8347da2fc)) - [@tqwewe](https://github.com/tqwewe)
- **(thalo-kafka)** add `auto.offset.reset=earliest` to recommended kafka config - ([eb65b95](https://github.com/thalo-rs/thalo/commit/eb65b95b10b70c911579fa3da6bbbde91b64f0f5)) - [@tqwewe](https://github.com/tqwewe)
- **(thalo-kafka)** add `Clone`, `Debug` derives to `KafkaClientConfig` - ([eb4a5ee](https://github.com/thalo-rs/thalo/commit/eb4a5eee902e6e008a13f5a3cd0020f1ae6722a6)) - [@tqwewe](https://github.com/tqwewe)
- **(thalo-kafka)** add `KafkaEventStream::consumer` - ([960461d](https://github.com/thalo-rs/thalo/commit/960461db3881c9a0b75e9e45e91e31767d0a79ff)) - [@tqwewe](https://github.com/tqwewe)
- **(thalo-kafka)** add `KafkaEventMessage` - ([6d5fc66](https://github.com/thalo-rs/thalo/commit/6d5fc66dc234d2346172d86cd6d511e144deb3d6)) - [@tqwewe](https://github.com/tqwewe)
- **(thalo-kafka)** take items that implement `fmt::Display` for `KafkaEventStream::new` - ([a2ce28b](https://github.com/thalo-rs/thalo/commit/a2ce28b74b4d0675e4c4b88848b3bbba8109fb99)) - [@tqwewe](https://github.com/tqwewe)
#### Miscellaneous Chores
- merge branch 'dev' - ([7107ed8](https://github.com/thalo-rs/thalo/commit/7107ed8c5066c5bf1b01c32e484a590993408715)) - [@tqwewe](https://github.com/tqwewe)
- merge branch 'main' into dev - ([6ed6047](https://github.com/thalo-rs/thalo/commit/6ed6047b4735c6398009566aa9244b48827bf46f)) - [@tqwewe](https://github.com/tqwewe)
- merge branch 'dev' - ([0d79eab](https://github.com/thalo-rs/thalo/commit/0d79eab51ab9ed2ad940e0ca28db33e2c048af0c)) - [@tqwewe](https://github.com/tqwewe)
- - -

## [0.3.1](https://github.com/thalo-rs/thalo/compare/0.3.0..0.3.1) - 2021-12-27
#### Bug Fixes
- **(examples/protobuf)** unwrap result of event stream - ([c009315](https://github.com/thalo-rs/thalo/commit/c00931555bb04e656d07e479a0247523eb08ca1c)) - [@tqwewe](https://github.com/tqwewe)
#### Build system
- **(thalo)** enable all features on docs.rs - ([0c6df2f](https://github.com/thalo-rs/thalo/commit/0c6df2fd29787221c87f245174c5dc255d5afa23)) - [@tqwewe](https://github.com/tqwewe)
- **(thalo-postgres)** exclude docker image from cargo package - ([22b11ab](https://github.com/thalo-rs/thalo/commit/22b11ab0396af1d619c311f816654f79481dab4f)) - [@tqwewe](https://github.com/tqwewe)
- add thalo-kafka to bump hooks - ([a097491](https://github.com/thalo-rs/thalo/commit/a0974913234f409ac064e37a8b4944abe11fa51e)) - [@tqwewe](https://github.com/tqwewe)
#### Documentation
- **(thalo-kafka)** add docs to `KafkaEventStream` - ([26c9562](https://github.com/thalo-rs/thalo/commit/26c95625373a04b7de8a4d5fb63d998e9c87a96e)) - [@tqwewe](https://github.com/tqwewe)
- **(thalo-macros)** use absolute url links in docs - ([038d2f9](https://github.com/thalo-rs/thalo/commit/038d2f95cec26fcd8fd63d2304c393cb5fa9e54d)) - [@tqwewe](https://github.com/tqwewe)
#### Features
- **(thalo)** add result to `EventStream::listen_events` - ([d55fc09](https://github.com/thalo-rs/thalo/commit/d55fc097f387fccaf868b676aac8e67f875976c2)) - [@tqwewe](https://github.com/tqwewe)
- **(thalo-kafka)** add `KafkaClientConfig` - ([ebaa7ef](https://github.com/thalo-rs/thalo/commit/ebaa7ef0316fb031790e84c75179b352fed045ab)) - [@tqwewe](https://github.com/tqwewe)
- **(thalo-kafka)** add topic subscription to `KafkaEventStream` - ([4b10932](https://github.com/thalo-rs/thalo/commit/4b10932cd028e59f516a4bd6f85c54480c37af41)) - [@tqwewe](https://github.com/tqwewe)
#### Miscellaneous Chores
- merge branch 'dev' - ([8693ca7](https://github.com/thalo-rs/thalo/commit/8693ca7df4a7a1703e98b5ca9a543f4e2fd29c59)) - [@tqwewe](https://github.com/tqwewe)
- merge branch 'main' into dev - ([f627459](https://github.com/thalo-rs/thalo/commit/f6274596041f47b7131b9a296e340d5eba153d67)) - [@tqwewe](https://github.com/tqwewe)
- - -

## [0.3.0](https://github.com/thalo-rs/thalo/compare/0.2.2..0.3.0) - 2021-12-26
#### Bug Fixes
- **(thalo-kafka)** fix invalid `EventStream` implementation - ([25f6973](https://github.com/thalo-rs/thalo/commit/25f6973aa7e2fb1f14f8ee624ba2f937d4a2e60d)) - [@tqwewe](https://github.com/tqwewe)
- **(thalo-macros)** `AggregateChannel` not using expressive call to `AggregateType::aggregate_type()` - ([a006ad2](https://github.com/thalo-rs/thalo/commit/a006ad29948ef5bbca1579eea572c614944f6113)) - [@tqwewe](https://github.com/tqwewe)
- **(thalo-postgres)** database queries - ([ddc6617](https://github.com/thalo-rs/thalo/commit/ddc66179c82699a921e74eddaf66e34dc2abe02f)) - [@tqwewe](https://github.com/tqwewe)
#### Build system
- add publish to cog post bump hooks - ([2f1af85](https://github.com/thalo-rs/thalo/commit/2f1af8543230b5e4f0547670312a52ac53ba1f3f)) - [@tqwewe](https://github.com/tqwewe)
- add pre cog bump hooks for all packages - ([c9b0ae4](https://github.com/thalo-rs/thalo/commit/c9b0ae4f1605613f6083fb2de5b656136d0b6bff)) - [@tqwewe](https://github.com/tqwewe)
#### Documentation
- **(examples/postgres)** add gif preview - ([13dafd9](https://github.com/thalo-rs/thalo/commit/13dafd9356124544320c7120bbb782c3b49cc3f4)) - [@tqwewe](https://github.com/tqwewe)
- **(examples/protobuf)** add README.md and screenshot - ([fd001b8](https://github.com/thalo-rs/thalo/commit/fd001b89bae5bbdfa8d4e8b7d49feb41619f2a18)) - [@tqwewe](https://github.com/tqwewe)
- **(thalo)** add event documentation - ([f471351](https://github.com/thalo-rs/thalo/commit/f47135168cfb2711593772cef9384771683928d7)) - [@tqwewe](https://github.com/tqwewe)
- **(thalo)** update module documentation - ([7abf5b6](https://github.com/thalo-rs/thalo/commit/7abf5b6e4139edaa42071202de43074e7b63c592)) - [@tqwewe](https://github.com/tqwewe)
- **(thalo)** update description - ([6b84c04](https://github.com/thalo-rs/thalo/commit/6b84c046bdb77399523c1626bc0b48ae085fe77b)) - [@tqwewe](https://github.com/tqwewe)
- **(thalo)** update doc links and correct grammar - ([bc1c8cb](https://github.com/thalo-rs/thalo/commit/bc1c8cbf6a04707e491a0bc38878d97af732ec7f)) - [@tqwewe](https://github.com/tqwewe)
- **(thalo-inmemory)** add docs - ([7212875](https://github.com/thalo-rs/thalo/commit/721287550dac43f2c87e4fa33300c72c4fe2c706)) - [@tqwewe](https://github.com/tqwewe)
- **(thalo-kafka)** add package thalo-kafka - ([70d8171](https://github.com/thalo-rs/thalo/commit/70d8171fc945d773a8c1bc7cf1ef85d3dfa89897)) - [@tqwewe](https://github.com/tqwewe)
- add information to crate Cargo.toml files - ([d29e3b8](https://github.com/thalo-rs/thalo/commit/d29e3b875bdfca01c5840f1b6c30aa7b47bbb143)) - [@tqwewe](https://github.com/tqwewe)
- update README.md - ([0d64db5](https://github.com/thalo-rs/thalo/commit/0d64db5ee2c0b88eb089064e2057c72f57dff277)) - [@tqwewe](https://github.com/tqwewe)
- remove stray period from README.md - ([d0a7580](https://github.com/thalo-rs/thalo/commit/d0a75804bbb4efc75a60cff01cd97fc54b5a6c8c)) - [@tqwewe](https://github.com/tqwewe)
- center title and remove 3 points on README.md - ([dcbea3e](https://github.com/thalo-rs/thalo/commit/dcbea3e1364dfa01bd0d611e39573d3b50c9f7d9)) - [@tqwewe](https://github.com/tqwewe)
- add logo to README.md - ([09c951f](https://github.com/thalo-rs/thalo/commit/09c951f9be6ec35775d7e85a9afa4d0b8602ffc7)) - [@tqwewe](https://github.com/tqwewe)
- add detailed README.md content - ([382514a](https://github.com/thalo-rs/thalo/commit/382514a12182df30e8f5895f518d41d23581db0d)) - [@tqwewe](https://github.com/tqwewe)
#### Features
- **(examples/postgres)** use `BroadcastStream` - ([6c759e4](https://github.com/thalo-rs/thalo/commit/6c759e4e56ff483146993d37df786cf7759dbc88)) - [@tqwewe](https://github.com/tqwewe)
- **(examples/protobuf)** add interactive demo - ([01f96e1](https://github.com/thalo-rs/thalo/commit/01f96e196d5aecffc529285dfa5f3e0d5063c897)) - [@tqwewe](https://github.com/tqwewe)
- **(examples/protobuf)** add example protobuf - ([de9028e](https://github.com/thalo-rs/thalo/commit/de9028e7e25294884db4ccaa7e33c3df53e91e1b)) - [@tqwewe](https://github.com/tqwewe)
- **(thalo)** add thalo-macros to thalo crate - ([2bac184](https://github.com/thalo-rs/thalo/commit/2bac1840d4d84f4f7b205fd4e29d698264f79e7a)) - [@tqwewe](https://github.com/tqwewe)
- **(thalo)** implement `EventStream` for tokio channel streams - ([26b6823](https://github.com/thalo-rs/thalo/commit/26b6823f6236abf9c08b93c5f084ac6ce2776a37)) - [@tqwewe](https://github.com/tqwewe)
- **(thalo)** add `load_events_by_id` to `EventStore` - ([320bda4](https://github.com/thalo-rs/thalo/commit/320bda480960143867264d0e40b41bc581b7c234)) - [@tqwewe](https://github.com/tqwewe)
- **(thalo)** add event stream - ([5e3af98](https://github.com/thalo-rs/thalo/commit/5e3af98ef888f9474bbe53761822314aff904939)) - [@tqwewe](https://github.com/tqwewe)
- **(thalo-inmemory)** add package `thalo-inmemory` - ([c85b366](https://github.com/thalo-rs/thalo/commit/c85b366b7c443287470c3cef81a914de57f74e3c)) - [@tqwewe](https://github.com/tqwewe)
- **(thalo-macros)** add derive `EventType` - ([aad337b](https://github.com/thalo-rs/thalo/commit/aad337bd96cad5b8462b756ca42539b90bfbfed2)) - [@tqwewe](https://github.com/tqwewe)
- **(thalo-macros)** add derive `Aggregate` - ([7b265d8](https://github.com/thalo-rs/thalo/commit/7b265d8184f8c8f34ed4c4a207a6cfb42d5aa260)) - [@tqwewe](https://github.com/tqwewe)
- **(thalo-macros)** add derive `TypeId` - ([c6e71bd](https://github.com/thalo-rs/thalo/commit/c6e71bd59d6e2fd196e1c7ce170436f707f7df0b)) - [@tqwewe](https://github.com/tqwewe)
- **(thalo-macros)** add package `thalo-macros` - ([efd90f4](https://github.com/thalo-rs/thalo/commit/efd90f49894e8ca903433729710ecef75522d425)) - [@tqwewe](https://github.com/tqwewe)
- **(thalo-postgres)** implement `load_events_by_id` - ([a52f4af](https://github.com/thalo-rs/thalo/commit/a52f4afe5d6cb2b4637eddd7ea104f76ce7904cb)) - [@tqwewe](https://github.com/tqwewe)
- **(thaloto)** add package `thaloto` - ([3498733](https://github.com/thalo-rs/thalo/commit/3498733ca319dfe36d99c2c39301b6ea1d1dc949)) - [@tqwewe](https://github.com/tqwewe)
- **(thaloto-postgres)** add package `thaloto-postgres` - ([a074bdf](https://github.com/thalo-rs/thalo/commit/a074bdfcb3b538c30b7ad4158b1f884ba190ce3d)) - [@tqwewe](https://github.com/tqwewe)
- add projections and event handler - ([7d42cbf](https://github.com/thalo-rs/thalo/commit/7d42cbf78abb0497a14fd323b01a11bcc7f56239)) - [@tqwewe](https://github.com/tqwewe)
#### Miscellaneous Chores
- **(examples/protobuf)** remove unused trait `BankAccountCommand` - ([3d7917b](https://github.com/thalo-rs/thalo/commit/3d7917b109e98dbfb328517e6f456d7815142b32)) - [@tqwewe](https://github.com/tqwewe)
- **(outbox-relay)** delete outbox relay (moved to new repo thalo-rs/outbox-relay) - ([fa339dd](https://github.com/thalo-rs/thalo/commit/fa339ddedfdf2e99986f18af79a5ffb5467fa1cd)) - [@tqwewe](https://github.com/tqwewe)
- merge branch 'dev' - ([424e7da](https://github.com/thalo-rs/thalo/commit/424e7da46139dba9c9ba3ce1b5508364ea6121be)) - [@tqwewe](https://github.com/tqwewe)
- merge branch 'feat/rewrite' - ([08af956](https://github.com/thalo-rs/thalo/commit/08af9565c182e2fef6a671f21274b28e0ffc9ecf)) - [@tqwewe](https://github.com/tqwewe)
- merge branch 'main' into feat/rewrite - ([6555444](https://github.com/thalo-rs/thalo/commit/6555444e7881cf88cad85fc7448547b91d0d2589)) - [@tqwewe](https://github.com/tqwewe)
- update versions to 0.3 - ([f0bc4eb](https://github.com/thalo-rs/thalo/commit/f0bc4eb5fcc4f04a6446aadedfdd017aebb7f486)) - [@tqwewe](https://github.com/tqwewe)
- rename thaloto packages to thalo - ([ce4f597](https://github.com/thalo-rs/thalo/commit/ce4f5970147b44935bc1e51c3d7c282f80dc5504)) - [@tqwewe](https://github.com/tqwewe)
- thalo & thalo-macros updates wip - ([8fb805f](https://github.com/thalo-rs/thalo/commit/8fb805f305b98ddacba412f137f07ed49d85ee8f)) - [@tqwewe](https://github.com/tqwewe)
- merge branch 'dev' - ([fc98472](https://github.com/thalo-rs/thalo/commit/fc984725d6d20e9972321c395b88a00854ed3f25)) - [@tqwewe](https://github.com/tqwewe)
- merge branch 'main' into dev - ([046edd7](https://github.com/thalo-rs/thalo/commit/046edd70f3b2c8f637c851e2ca4e63ff7f7f19ff)) - [@tqwewe](https://github.com/tqwewe)
- merge branch 'dev' - ([74c8c12](https://github.com/thalo-rs/thalo/commit/74c8c12fdf5557f36236709f1a6919d077cbcf6d)) - [@tqwewe](https://github.com/tqwewe)
- merge branch 'main' into dev - ([3d193ef](https://github.com/thalo-rs/thalo/commit/3d193efaf1a15580d7743428b1feb3b757d180fe)) - [@tqwewe](https://github.com/tqwewe)
- increase logo size on README.md - ([43b273c](https://github.com/thalo-rs/thalo/commit/43b273c69541a6ea4e1bc53587ac5509aca6b469)) - [@tqwewe](https://github.com/tqwewe)
- rename thalo.png to logo.png - ([5f217f4](https://github.com/thalo-rs/thalo/commit/5f217f48e42850fcd417db038c3b4509aee8e4b0)) - [@tqwewe](https://github.com/tqwewe)
- merge branch 'dev' - ([4e805fb](https://github.com/thalo-rs/thalo/commit/4e805fb4f5db7003e2108037a6a1f4c631d43bc1)) - [@tqwewe](https://github.com/tqwewe)
- merge branch 'main' into dev - ([597473f](https://github.com/thalo-rs/thalo/commit/597473f63f44b70ab5677c22cbb595349b27a799)) - [@tqwewe](https://github.com/tqwewe)
#### Refactoring
- **(examples/bank)** remove unused `AggregateType` import - ([cd79d27](https://github.com/thalo-rs/thalo/commit/cd79d27f3fa938900a410326dd0e06ca4bc11e44)) - [@tqwewe](https://github.com/tqwewe)
- **(thalo)** remove unused module `actor` - ([51ead6e](https://github.com/thalo-rs/thalo/commit/51ead6ecab3a07423dff8e4066a0eb04b087138a)) - [@tqwewe](https://github.com/tqwewe)
- **(thalo)** move `EventEnvelope` to `event` module - ([ffa2b39](https://github.com/thalo-rs/thalo/commit/ffa2b39e6e9c24b64177ac1f39486c1bda2d82b3)) - [@tqwewe](https://github.com/tqwewe)
- - -

## [0.2.2](https://github.com/thalo-rs/thalo/compare/0.2.1..0.2.2) - 2021-12-18
#### Build system
- **(examples/bank)** update Cargo.lock - ([bda7bde](https://github.com/thalo-rs/thalo/commit/bda7bde119e96d26c8ebff96790e7b93a79c3bc4)) - [@tqwewe](https://github.com/tqwewe)
- increase delay between publishing packages - ([d254b3a](https://github.com/thalo-rs/thalo/commit/d254b3ac997fa93f81cd2467fa0fc6024a42bd2c)) - [@tqwewe](https://github.com/tqwewe)
- delete cargo-puglic script - ([39cbc53](https://github.com/thalo-rs/thalo/commit/39cbc530dc48fc46e1e7827fcb3fb5db442d2fc6)) - [@tqwewe](https://github.com/tqwewe)
#### Documentation
- **(docs)** add index.html - ([6345cd1](https://github.com/thalo-rs/thalo/commit/6345cd15729b05eb4590ae8b3fd2718ebb4a2c26)) - [@tqwewe](https://github.com/tqwewe)
- **(thalo)** add information example to documentation - ([6f78420](https://github.com/thalo-rs/thalo/commit/6f784208be661b439b7660d88a4692beeb3702b4)) - [@tqwewe](https://github.com/tqwewe)
- **(thalo)** add and fix documentation - ([bd5c732](https://github.com/thalo-rs/thalo/commit/bd5c732de7ac70c469ede91bcb96887ab0d9ffb8)) - [@tqwewe](https://github.com/tqwewe)
#### Features
- **(docs)** add responsive styles - ([911f4d2](https://github.com/thalo-rs/thalo/commit/911f4d21970a6cb0ebfe7921f50e95784d71be4f)) - [@tqwewe](https://github.com/tqwewe)
- **(docs)** add favicon and description - ([e57ea29](https://github.com/thalo-rs/thalo/commit/e57ea2944ef7babe8494b63b0ae1a9347b626b5d)) - [@tqwewe](https://github.com/tqwewe)
- **(examples/bank)** use invariant codes without message - ([2280c1f](https://github.com/thalo-rs/thalo/commit/2280c1f6c2a853b39286ac03b5d8c2ce4f5d5812)) - [@tqwewe](https://github.com/tqwewe)
- **(examples/bank)** add invairant codes - ([7f83823](https://github.com/thalo-rs/thalo/commit/7f83823413d9393799eed0387a595da6ddfd15a4)) - [@tqwewe](https://github.com/tqwewe)
- **(thalo)** add `Error::invariant_code` method - ([3e312fa](https://github.com/thalo-rs/thalo/commit/3e312fac0720fb7e54c211888cf5730ed0fbd6a4)) - [@tqwewe](https://github.com/tqwewe)
- **(thalo)** add code to `Error::Invariant` - ([971ca8c](https://github.com/thalo-rs/thalo/commit/971ca8cbd06047836a163f280a83b386016cded2)) - [@tqwewe](https://github.com/tqwewe)
- add `code` and `message` to tarpc error response - ([4cb13cf](https://github.com/thalo-rs/thalo/commit/4cb13cf1e8349c21e676196f62ed7d222f5ddd6d)) - [@tqwewe](https://github.com/tqwewe)
#### Miscellaneous Chores
- **(docs)** create CNAME - ([2130eac](https://github.com/thalo-rs/thalo/commit/2130eac1ca9d0f40b4a65e44adae89e7efcec844)) - [@tqwewe](https://github.com/tqwewe)
- merge branch 'dev' - ([32231e8](https://github.com/thalo-rs/thalo/commit/32231e8ce8e78061982d549c0bf68bac4e63b918)) - [@tqwewe](https://github.com/tqwewe)
- merge branch 'dev' of github.com:thalo-rs/thalo - ([d8aca78](https://github.com/thalo-rs/thalo/commit/d8aca782fac2b9444f20efae1a697d6f49a76250)) - [@tqwewe](https://github.com/tqwewe)
- merge branch 'dev' of github.com:thalo-rs/thalo - ([eacee47](https://github.com/thalo-rs/thalo/commit/eacee474379c9b76bab08a7ef29329a904e7c8de)) - [@tqwewe](https://github.com/tqwewe)
- merge branch 'main' of github.com:thalo-rs/thalo into dev - ([a2efb1e](https://github.com/thalo-rs/thalo/commit/a2efb1ecb0ea53bff9857acd40d3ba8567cb4bd9)) - [@tqwewe](https://github.com/tqwewe)
- create CNAME - ([b7fe556](https://github.com/thalo-rs/thalo/commit/b7fe556f35bf45526b76e9e3103cd0c7f6e15254)) - [@tqwewe](https://github.com/tqwewe)
#### Refactoring
- **(thalo)** rename file `alias.rs` to `command.rs` - ([716d046](https://github.com/thalo-rs/thalo/commit/716d0469c4cc92096951fe6e61fd819638986485)) - [@tqwewe](https://github.com/tqwewe)
- **(thalo)** remove unused macro `send` - ([ac52b26](https://github.com/thalo-rs/thalo/commit/ac52b26e3599a3b8660803966d9b153f76f1271c)) - [@tqwewe](https://github.com/tqwewe)
- - -

## [0.2.1](https://github.com/thalo-rs/thalo/compare/0.2.0..0.2.1) - 2021-12-15
#### Build system
- fix invalid links - ([3cb1519](https://github.com/thalo-rs/thalo/commit/3cb15198781d4720263ee3a0bf113c5b89326443)) - [@tqwewe](https://github.com/tqwewe)
- add sleep between publishing packages - ([2af0708](https://github.com/thalo-rs/thalo/commit/2af07083d1cc501ed1f7995ddaf5c100bc5d85c0)) - [@tqwewe](https://github.com/tqwewe)
#### Miscellaneous Chores
- merge branch 'main' of github.com:thalo-rs/thalo into dev - ([0a13f0b](https://github.com/thalo-rs/thalo/commit/0a13f0b2cf5d07a8cad5ac54326e76d89e74ac43)) - [@tqwewe](https://github.com/tqwewe)
- - -

## [0.2.0](https://github.com/thalo-rs/thalo/compare/0.1.2..0.2.0) - 2021-12-15
#### Bug Fixes
- **(examples/bank)** outbox relay feature flag typo - ([e43aca4](https://github.com/thalo-rs/thalo/commit/e43aca477abf8b960ca8d6f1bb66282917d4aee0)) - [@tqwewe](https://github.com/tqwewe)
#### Build system
- add cog.toml bump hooks and changelog info - ([a087830](https://github.com/thalo-rs/thalo/commit/a0878301acb85304ebaacd825489fb07c523c6cd)) - [@tqwewe](https://github.com/tqwewe)
- add cog.toml - ([c50932a](https://github.com/thalo-rs/thalo/commit/c50932a91b75182efe9e94076c15314eb82dca30)) - [@tqwewe](https://github.com/tqwewe)
#### Features
- **(docker-image)** add table `projection` - ([b821cc1](https://github.com/thalo-rs/thalo/commit/b821cc124453426530c7d482970ab3974fc47ec6)) - [@tqwewe](https://github.com/tqwewe)
- **(examples/bank)** use `projection` table - ([39f3ec2](https://github.com/thalo-rs/thalo/commit/39f3ec28f1e2d9e30bc6bc5fb95a278f562c43e1)) - [@tqwewe](https://github.com/tqwewe)
- **(outbox-relay)** add --no-loop flag to pg_recvlogical - ([94204d2](https://github.com/thalo-rs/thalo/commit/94204d2e855174539daa0966a6aab3fb85fc469a)) - [@tqwewe](https://github.com/tqwewe)
- **(outbox-relay)** delete existing slot on startup - ([87f217d](https://github.com/thalo-rs/thalo/commit/87f217dce8babd7ad5c4881e1bbb231398a8c336)) - [@tqwewe](https://github.com/tqwewe)
- **(thalo)** add associated id type to  trait - ([722d13f](https://github.com/thalo-rs/thalo/commit/722d13fd22de31d1b1d5ad23f2ac6a6acbe37abf)) - [@tqwewe](https://github.com/tqwewe)
- **(thalo)** use `projection` table for `event_id` and `event_sequence` - ([865a037](https://github.com/thalo-rs/thalo/commit/865a03777e666b9b0c64d4e3e22ede852d54ab04)) - [@tqwewe](https://github.com/tqwewe)
- **(thalo-macros)** add tarpc service generation to aggregate_commands macro - ([497d08a](https://github.com/thalo-rs/thalo/commit/497d08a9a8fd7ee0b3a1a4d3660dd3cc6824c75b)) - [@tqwewe](https://github.com/tqwewe)
#### Miscellaneous Chores
- **(outbox-relay)** re-organise directory and move to separate workspace - ([020b098](https://github.com/thalo-rs/thalo/commit/020b09808f43901a2f120973a170edc45fc645b6)) - [@tqwewe](https://github.com/tqwewe)
- **(thalo)** update rdkafka to 0.28 - ([b0ab2c4](https://github.com/thalo-rs/thalo/commit/b0ab2c4c983b2c8125245813c26f12cf6d0f9c4a)) - [@tqwewe](https://github.com/tqwewe)
- add cog hooks - ([83b0e1c](https://github.com/thalo-rs/thalo/commit/83b0e1c5e7a6e8aca5c12b3842d05a4bdbeb87c6)) - [@tqwewe](https://github.com/tqwewe)
- add .DS_Store to .gitignore - ([1562b80](https://github.com/thalo-rs/thalo/commit/1562b802f84ef3f7e80540b1f0a6b8955554bcfe)) - [@tqwewe](https://github.com/tqwewe)
- Cargo.lock - ([8b633e7](https://github.com/thalo-rs/thalo/commit/8b633e73b0bb131881f68bcbe5327d393976976c)) - [@tqwewe](https://github.com/tqwewe)
#### Refactoring
- **(examples/bank)** clean return type for `open_account` - ([2c4dcd2](https://github.com/thalo-rs/thalo/commit/2c4dcd2da3ef7de8547ae362b17924d2ea0969b7)) - [@tqwewe](https://github.com/tqwewe)
- **(thalo)** rename feature to 'outbox-relay' - ([cfb0806](https://github.com/thalo-rs/thalo/commit/cfb0806984222f5d6d9e1fd4a18df545a2c866ed)) - [@tqwewe](https://github.com/tqwewe)
- rename aggregate_commands/aggregate_events to commands/events - ([0b2db93](https://github.com/thalo-rs/thalo/commit/0b2db935ba43e6db6698306a82466f8a8f3b6e2f)) - [@tqwewe](https://github.com/tqwewe)
#### Revert
- chore(outbox-relay): re-organise directory and move to separate workspaceThis reverts commit dcb9b288090d4bf112d579df76caac68c12a6f2e. - ([73f17ae](https://github.com/thalo-rs/thalo/commit/73f17ae38e6c38c583b9c3a6bbb2a863d6eabd2f)) - [@tqwewe](https://github.com/tqwewe)
- - -

Changelog generated by [cocogitto](https://github.com/cocogitto/cocogitto).