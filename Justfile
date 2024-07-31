default:
    just --list
run:
    RUST_LOG=debug cargo run
build:
    cargo build
test:
    cargo nextest run --workspace
check:
    cargo check
clippy:
    cargo clippy
fmt:
    cargo fmt
cargo-toml-fmt:
    taplo fmt Cargo.toml crates/*/Cargo.toml
