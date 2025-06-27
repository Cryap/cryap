# Installation

## Dependencies
- :simple-postgresql: PostgreSQL
- :simple-redis: Redis

Download Cryap binary from CI/CD (soon) or [build Cryap yourself](../development/building_release.md).

## Configuring
Create configuration file by copying `config.toml.example` to the same directory as the binary under name `config.toml` and specify necessary data there such as the database. All possible config parameters can be found [here](configuration.md).

## Running
Run `./cryap`. If everything is ok, you have finished setting up and launched the social network. The database setup will be done automatically. Keep in mind that Cryap is under active development and not yet ready for production use.

Read more: [creating systemd service](linux/systemd.md)

## What's next?
- [Creating an account using RPC](../administation/rpc.md#registeruser)
