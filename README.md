# Cryap 🦆
[Mastodon (until Cryap is ready for this)](https://mastodon.social/@cryap) | [Matrix Space](https://matrix.to/#/#cryap:matrix.org)

Cryap is a [federated](https://en.wikipedia.org/wiki/Fediverse) social network written in Rust, currently in development. It speaks ActivityPub, which means it can federate with platforms like Mastodon, Pleroma, and others. Cryap also supports the Mastodon API, making it compatible with popular clients such as [Semaphore](https://semaphore.social) and [Tusky](https://tusky.app) - and we're building our own frontend too!
## Why another social network?
We plan that Cryap will combine the advantages of all popular ActivityPub microblogging engines. For example:
- Simplicity of Mastodon
- Versatility of Misskey
- Lightness more than Pleroma, because Cryap is written in Rust

We also plan to implement such functionality as [cat ears for avatars](https://github.com/mastodon/mastodon/issues/18337), articles and much more.
# Status
It is possible to publish posts without media, read them and interact with posts and users. There is support for OAuth2 (but no scopes yet). Soon we will reach a level that allows daily use and we will be able to start developing our own frontend. You can help us achieve this 😊
# Setup
Cryap uses PostgreSQL and Redis for storage. To set it up manually:
1. Install Rust, PostgreSQL and Redis.
2. Clone the repository:
```shell
git clone https://codeberg.org/cryap/cryap
cd cryap
```
3. Build the project:
```shell
cargo build --release
```
4. Copy the binary `target/release/cryap` to any directory.
5. Create configuration file by copying `config.toml.example` to the same directory under name `config.toml` and specify necessary data there such as the database. All possible config parameters as long as we do not have complete documentation can be found [here](https://codeberg.org/cryap/cryap/src/branch/main/crates/web/src/config.rs).
5. Run `./cryap`. If everything is ok, you have finished setting up and launched the social network. The database setup will be done automatically. Keep in mind that Cryap is under active development and not yet ready for production use.
## Development
We provide a `docker-compose.yml` file with all dependencies pre-configured. To get started install Docker and Docker Compose and run:
```shell
docker-compose up
```
Then compile and run in debug mode:
```shell
RUST_LOG=debug cargo run
```
Before committing changes, format your code using `cargo fmt` and follow the `cargo clippy` hints.
