# Building release binary

## Dependencies

- :simple-rust: Rust
- [Trunk](https://trunkrs.dev)

## Clone & Setup
1. Clone the repository:
```shell
git clone https://codeberg.org/cryap/cryap
cd cryap
```
2. Build the frontend:
```shell
cd crates/frontend
trunk build
cd ../..
```
3. Build the project:
```shell
cargo build --release
```
4. Copy the binary `target/release/cryap` to any directory.
