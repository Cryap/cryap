# Client API
Cryap implements a client API that is compatible with the [Mastodon API](https://docs.joinmastodon.org/client/intro), with a few Cryap-specific extensions and differences:

- **`Account` entity**: `is_cat` attribute
- **`/api/v1/accounts/update_credentials`**: `is_cat` body param
- **`Instance` and `V1::Instance` entities**: `cryap_version` attribute
