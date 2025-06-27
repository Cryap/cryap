# RPC API
Cryap provides a [Unix domain sockets](https://en.wikipedia.org/wiki/Unix_domain_socket) based interface for instance administration.

The RPC server listens on a Unix domain socket at `cryap.rpc` in the current working directory. You can connect to it using tools like `socat`. In the future, a more convenient interface for interacting with the RPC API will be provided, but for now, this is what it is.
```shell
# Interactive connection
socat - ./cryap.rpc
```
```shell
# Send a single command and wait for answer
echo '{"type":"UserFetch","content":"user@example.com"}' | socat -t 5 - ./cryap.rpc
```

All messages are JSON objects with the following structure:
```json
// Request
{
    "type": "CommandType",
    "content": <command-specific-data>
}
```
```json
// Response
{
    "type": "CommandType",
    "content": <response-data>
}
```
## Available commands
### UserFetch
Fetches and resolves a user using a WebFinger indentifier. Request content is required to be the WebFinger identifier of the user to fetch (e.g., "user@example.com"). Example:
```json
{
    "type": "UserFetch",
    "content": "user@example.com"
}
```
If the command was executed successfully, the response will be as follows:
```json
{
    "type": "UserFetch",
    "content": {
        "ok": true
    }
}
```
Response fields:

- `ok` (boolean): `true` if the user was successfully resolved, `false` if there was an error
### RegisterUser
Registers a new user account on the instance. For now, this is the only way to create a new user in Cryap. Request content is required to be an object with the following fields:

- `name` (string, required): The username for the new account
- `password` (string, required): The password for the new account
- `bio` (string, optional): The bio for the user
- `display_name` (string, optional): The display name

Example:
```json
{
    "type": "RegisterUser",
    "content": {
        "name": "admin",
        "password": "averystrongpassword",
        "bio": "Admin of this instance",
        "display_name": "Powerful Admin"
    }
}
```
If the command was executed successfully, the response will be as follows:
```json
{
    "type": "RegisterUser",
    "content": {
        "ok": true
    }
}
```
Response fields:

- `ok` (boolean): `true` if the user was successfully registered, `false` if there was an error