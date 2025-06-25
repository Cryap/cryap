# Federation
Cryap is not yet ready for production use, so some implementations may not be complete. This document is subject to change.
## Supported federation protocols and standards
- [ActivityPub](https://www.w3.org/TR/activitypub) (Server-to-Server)
- [WebFinger](https://webfinger.net)
- [Http Signatures](https://datatracker.ietf.org/doc/html/draft-cavage-http-signatures)
- [NodeInfo](https://nodeinfo.diaspora.softwares)

## Supported FEPs
- [FEP-67ff: FEDERATION.md](https://codeberg.org/fediverse/fep/src/branch/main/fep/67ff/fep-67ff.md)
- [FEP-f1d5: NodeInfo in Fediverse Software](https://codeberg.org/fediverse/fep/src/branch/main/fep/f1d5/fep-f1d5.md)
- [FEP-fe34: Origin-based security model](https://codeberg.org/fediverse/fep/src/branch/main/fep/fe34/fep-fe34.md)

## ActivityPub
The following activities and object types are currently supported:
- `Follow(Actor)`, `Accept(Follow)`, `Reject(Follow)`, `Undo(Follow)`.
- `Create(Note)`
- `Like()`, `Undo(Like)`.
- `Announce(Note)`, `Undo(Announce)`.
- `Update(Actor)`.

Activities are implemented in way that is compatible with Mastodon, Pleroma and other popular ActivityPub social network servers.

Cryap does not perform JSON-LD processing.
