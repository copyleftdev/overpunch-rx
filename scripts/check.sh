#!/bin/sh
set -eu

# Uses already installed toolchains/targets; this script does not install tools.
export RUSTFLAGS="-D warnings"
export RUSTDOCFLAGS="-D warnings"
cargo fmt --all -- --check
cargo fmt --manifest-path benchmarks/Cargo.toml -- --check
cargo fmt --manifest-path fuzz/Cargo.toml -- --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
cargo test --release --locked
cargo +1.85.0 test --locked
cargo build --lib --target thumbv7em-none-eabihf --locked
cargo build --lib --target wasm32-unknown-unknown --locked
cargo doc --no-deps --locked
cargo run --example claim_amount --locked
cargo package --allow-dirty --locked
