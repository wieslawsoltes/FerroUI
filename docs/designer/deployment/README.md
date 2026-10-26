# Hosted designer rooms

The collaboration executable reuses FerroUI MCP's HTTP transport. It binds to loopback, validates
Host/Origin, requires its transport bearer and then validates each request's administrator/member/
invitation credential. Application inspector tools, input simulation and logs are absent from this
endpoint. Put a TLS proxy in front of it for remote clients; the native peer rejects remote HTTP.

## Run as a process

```sh
cargo build --release -p ferroui-designer-collaboration-host --features server --bin ferroui-collaboration-server
./target/release/ferroui-collaboration-server --storage /private/ferroui-rooms \
  --bind 127.0.0.1:7491 --public-url https://design.example.org/mcp
```

Choose your own domain and storage directory. The process creates `administrator.json`,
`transport-key.json`, `endpoint.json` and checksummed room files in that private directory. The
administrator token creates rooms; each room has independent owner/member credentials. The bearer
authenticates transport access and does not grant room or administrator authority. Credentials are
not printed to stdout/stderr; startup reports their file locations. Keep the directory private and
back it up as a coherent snapshot while the process is stopped or with filesystem snapshot tooling.
Unix directories and files use modes 0700 and 0600. Windows files inherit directory ACLs; provision
a directory accessible only to the intended service account before starting the host. The service
does not create or tighten Windows ACLs. If `--endpoint-file` points outside the storage directory,
its parent must also be private: that file contains the transport bearer. Native sharing directories
and copied recovery files need the same protection.

Configure Caddy with the included `Caddyfile` and `COLLABORATION_DOMAIN`. The reverse proxy preserves
Authorization, MCP-Protocol-Version and Mcp-Session-Id and sets the upstream Host to the real
loopback address. Origin validation stays active; the native client sends no browser Origin header.
Point DNS at the machine and make the TLS proxy's ports accessible. The Rust service's port remains
loopback-only. An existing reverse proxy can use the same routing/header configuration.
`--public-url` must be an HTTPS URL without embedded credentials, a query or a fragment; the supplied
configuration uses `/mcp`. Advertising the URL does not configure DNS, TLS or the reverse proxy.

## Container deployment

```sh
COLLABORATION_DOMAIN=design.example.org \
  docker compose -f docs/designer/deployment/compose.yaml up --build -d
```

The two containers share a network namespace so Caddy can reach the service's loopback socket;
only HTTPS/HTTP challenge ports are published. The room volume belongs to an unprivileged service
user. Caddy stores certificates in separate persistent volumes. These files provide a runnable
deployment configuration; deploying to a real domain requires the operator's DNS/network setup.
Use Docker Compose with BuildKit. The accompanying `Dockerfile.dockerignore` allows the workspace
sources needed by Cargo and excludes local credentials, sharing checkpoints, repository history
and build output from the build context. Keep operational state outside the source directories.

## Creating rooms and invitations

The native **Share** panel can host a room directly. Select a private session directory, a fixed
loopback bind address if it should survive reconnects, and the public HTTPS endpoint when a proxy
fronts that native host. It creates the room and produces editor/viewer invitations. Join accepts
the invitation JSON and shows the remote source before replacing the current document. The owner
can change roles, revoke members or checkpoint a synchronized room from the same panel.

For the standalone service, use `load_private::<Secret>` for `administrator.json` and
`load_private::<Endpoint>` for `endpoint.json`. `McpPeer::request(Request::Create { request })`
creates a room using a client-generated `owner_token`; reusing that token makes lost-response retries
idempotent. `Request::Invite` issues an editor/viewer invite with expiry/use count. Serialize an
`Invitation` with the endpoint, room ID and invite token, then join through the native Share panel.
The public Rust request types are the JSON schema used by the `collaboration_request` MCP tool.

The same endpoint also exposes `collaboration_transfer` for authenticated expiring UTF-8 chunk
tickets. `McpPeer` handles these automatically for documents larger than one MCP message.

## Recovery

Stable transport credentials, room ACLs, operation logs and checkpoint epochs survive restart.
Peer checkpoints contain the exact baseline, local operation set and last acknowledged server
vector. Retry after a dropped reply resends the same operation identities. Native disconnect and
tab switch capture the final outbox and queue a worker write; detachment itself is immediate.
Continue polling until the sharing status reports that the final offline checkpoint is saved before
terminating the process or reusing that session directory. A write failure is reported in that status;
save the current document separately if it cannot be resolved. Invalid/conflicting source drafts are
included in successful checkpoints and can be reviewed when resuming. Checkpointing is not a
per-keystroke autosave guarantee.

An owner checkpoint requires the exact observed version. When historical operations conflict,
the owner supplies the explicitly reviewed `resolved_source`; the native button adopts the
current authored source as that resolution.
It produces a new epoch. Peers still on the old epoch retain their drafts and must review/rebase
against the new source. The native review saves both the prior document and prior sharing checkpoint
before applying a resolution, and rechecks the local revision after persistence completes.

Native review uses these checksummed files in the chosen sharing directory:

| File | Meaning |
| --- | --- |
| `peer.json` | Last successfully saved peer snapshot, source draft and server-acknowledged version; Resume reads this file. |
| `previous-document.json` | Local source and URI backed up before connection or a review decision. |
| `previous-sharing-checkpoint.json` | Exact prior replica, unacknowledged operations and source draft archived before preparing a new baseline. |
| `reviewed-resolution.json` | Durable prepared resolution; its presence alone does not mean UI acceptance or the final peer commit succeeded. |

After the UI revision and room-ownership checks succeed, the worker saves the accepted peer before
automatic synchronization resumes. If the UI check fails, the original remote source remains under
review and `peer.json` is unchanged. If the final save fails, the accepted source remains visible,
the prepared recovery file remains available, and a later synchronization retries local persistence
before sending operations. Preserve these files before manual recovery; `load_private::<serde_json::Value>`
verifies their envelope/checksum and exposes the source fields without depending on the native
checkpoint's private Rust type. They contain credentials as well as drafts and must remain private.

A downgrade to Viewer immediately blocks authoring. If an editor outbox or invalid draft is still
pending, the client retains it for review instead of repeatedly submitting operations the new role
cannot publish. Explicitly adopting the remote snapshot archives the old checkpoint before rebasing;
a Viewer cannot restore a different draft as the shared source.

An interrupted pre-rename write leaves the previous room intact. Files are synchronized before
replacement on every supported platform. Unix additionally synchronizes the parent directory;
Windows relies on the filesystem's replacement semantics without that additional directory barrier.
A failed Unix directory sync after rename is explicitly indeterminate; the process refuses later room access rather than overwriting
possibly committed state from stale memory. Restart reads and validates the checksummed file.
Corrupt files are reported instead of being silently replaced with an older checkpoint.
