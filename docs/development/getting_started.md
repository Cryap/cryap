# Getting started
We provide a `docker-compose.yml` file with all dependencies pre-configured. To get started install Docker and Docker Compose and run:
```shell
docker-compose up
```
Then compile and run in debug mode:
```shell
RUST_LOG=debug cargo run
```
Before committing changes, format your code using `cargo fmt` and follow the `cargo clippy` hints.