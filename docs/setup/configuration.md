# Configuration
!!! warning

    It is not possible to easily switch between different fediverse software on the same domain name (like example.org). If you do, you'll run into problems with federation - other servers may get confused or stop communicating properly with yours. Also, federation can break due to a bug in Cryap, since Cryap is not ready for production use.

    For this reason, it is better to use a separate subdomain for testing Cryap, for example cryap-test.example.org.

Cryap uses [TOML format](https://toml.io/en) for configuration. A sample configuration file is available [here](https://codeberg.org/cryap/cryap/src/branch/main/config.toml.example).

The configuration file contains four main sections: `web`, `database`, `redis` and `instance`.

## Web section
The `web` section configures the web server settings.

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `domain` | String | Yes | - | The domain name for the web server |
| `port` | Integer | Yes | - | The port number the web server will listen on |
| `host` | String | No | "0.0.0.0" | The host address to bind to |

### Example
```toml
[web]
domain = "example.com"
port = 8080
host = "127.0.0.1"  # Optional, defaults to "0.0.0.0"
```

## Database section
The `database` section configures PostgreSQL connection.

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `uri` | String | Yes | - | PostgreSQL connection URI |

### Example
```toml
[database]
uri = "postgresql://user:password@localhost/dbname"
```

## Redis section
The `redis` section configures Redis connection.

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `uri` | String | Yes | - | Redis connection URI |

### Example
```toml
[redis]
uri = "redis://127.0.0.1"
```

## Instance Configuration
The `instance` section configures application instance settings and limits.

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `title` | String | Yes | - | The title of the instance |
| `description` | String | No | "" | Description of the instance |
| `languages` | Array of Strings | No | `["en"]` | Supported languages in ISO 639-1 format |
| `rules` | Array of Strings | Yes | - | Instance rules |
| `max_characters` | Integer | No | 200 | Maximum characters allowed in posts |
| `display_name_max_characters` | Integer | No | 30 | Maximum characters for display names |
| `bio_max_characters` | Integer | No | 500 | Maximum characters for user bios |

### Example
```toml
[instance]
title = "My Cryap Instance"
description = "A friendly community for everyone"
languages = ["en", "es", "fr"]
rules = [
    "Be respectful to others",
    "No spam or advertising",
    "Follow local laws"
]
max_characters = 500
display_name_max_characters = 50
bio_max_characters = 1000
```