# FerroUI Designer collaboration host

Reusable authenticated rooms, durable snapshots, invitations, owner/editor/viewer roles,
actor-bound operation validation, deterministic synchronization, presence and offline outboxes.
The `server` feature exposes the room service through the existing `ferroui-mcp` transport.
The `client` feature supplies an HTTPS MCP peer and a durable reconnecting replica.

```sh
cargo run -p ferroui-designer-collaboration-host --features server --bin ferroui-collaboration-server -- --storage ./private-rooms
```

The storage directory is private and process-locked. Administrator, transport and member
credentials are independent. On Unix, directories use mode 0700 and credential files use mode
0600; on Windows, files inherit the directory's ACL, so provision a directory restricted to the
service account. Room files contain credential hashes. Keep endpoint/invitation strings private.
The service reuses MCP's loopback
binding, bearer authentication, Host/Origin validation, cancellation and bounded message handling.
For remote teams, terminate HTTPS at a reverse proxy and pass `--public-url https://host/mcp`.
The proxy must preserve Authorization/Mcp-Session-Id/MCP-Protocol-Version and set the upstream
Host to the loopback address. Stable transport credentials survive server restarts.

Calls acknowledge only after an atomic durable room write. Peer snapshots retain unacknowledged
operations and the last server version; lost responses can be retried. Checkpointing starts a
new epoch with an exact version precondition and no unresolved sequence holes. An operation beyond
a hole cannot be erased using a vector that still describes an earlier observed prefix.
Offline peers retain their old drafts and explicitly resolve/rebase them before joining the new
epoch. Source replacements use the core engine's CAS
conflict semantics. Presence is transient and expires on the receiving server's monotonic clock.
If an outbox's post-rename directory sync fails, `ReconnectingPeer` blocks further writes and sync
until restored from its saved checkpoint, preventing rolled-back memory from overwriting a file
whose replacement may already be durable.
The native sharing worker also stops, blocks editing/retries, and suppresses a final checkpoint
replacement until the user detaches and resumes the saved session.

Large requests and responses use authenticated, checksummed, expiring 64 KiB transfers rather
than changing MCP's request limit. Operation exchange additionally uses pages bounded to 32 MiB of
JSON; a retained 64 MiB checkpoint can span several exchanges and survive restart. Native UI code
performs network and persistence work on a worker thread. `PeerTransport` permits alternative
transports without changing replica semantics.

See [the designer guide](../../docs/designer/README.md) for native sharing and deployment.
