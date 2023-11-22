{
  description = "Dev shell";

  inputs = {
    nixpkgs.url      = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url  = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
        rust-bin-nightly = (pkgs.rust-bin.selectLatestNightlyWith (toolchain: toolchain.default)).override {
          extensions = [ "rust-src" "rustfmt" "rust-analyzer" ];
          targets = [ "wasm32-wasi" ];
        };
      in
      with pkgs;
      {
        devShells.default = mkShell {
          buildInputs = [
            pkg-config
            protobuf
            rust-bin-nightly
          ];
        };
      }
    );
}
