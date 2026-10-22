# Inspecting and driving FerroUI with MCP

`ferroui-inspect` is the UI-thread inspection core. `ferroui-mcp` exposes it to agents through
Model Context Protocol (MCP), with newline-delimited JSON-RPC over stdio or Streamable HTTP.
Neither crate starts a service merely by being linked. Applications must opt in and retain the
handle returned by `ferroui_mcp::serve`. Property writes and input are disabled by default.

## ControlCatalog and Claude Code

Build the optional integration:

```sh
cargo build -p control-catalog-desktop --features mcp
```

Register the built executable, using its **absolute path**, with Claude Code:

```sh
claude mcp add --transport stdio ferroui -- /absolute/path/to/FerroUI/target/debug/control-catalog-desktop --mcp-stdio --mcp-headless --mcp-allow-input --mcp-allow-writes
```

The client launches the catalog. `--mcp-headless` uses the headless windowing platform and the
real renderer, so inspection, input and PNG screenshots work without a desktop. Omit this flag
to inspect visible windows. Omit either permission flag to disable that category of mutation.
For another MCP client, use the same executable and arguments in its stdio server configuration.

To attach to a running catalog instead:

```sh
cargo run -p control-catalog-desktop --features mcp -- --mcp-http=8765 --mcp-allow-input --mcp-allow-writes
```

Startup prints its loopback URL and a fresh bearer token to stderr. Port `0` chooses a free port.
Copy the token into the client's HTTP configuration:

```sh
claude mcp add --transport http ferroui http://127.0.0.1:8765/mcp --header "Authorization: Bearer TOKEN_FROM_STARTUP"
```

The token changes on every run. Do not commit it. HTTP binds only to loopback, validates Host and
Origin, requires the bearer token for every method, and does not enable cross-origin access.

Example agent request: “Find SearchBox, type Button, inspect the resulting tree, and show me a
screenshot.” The schema descriptions returned by `tools/list` describe all arguments.

## Embedding in an application

Make the dependency optional in the application's Cargo.toml:

```toml
[dependencies]
ferroui-mcp = { path = "../../src/FerroUI.Mcp", optional = true }

[features]
mcp = ["dep:ferroui-mcp"]
```

After application setup, on its UI thread, start the server and retain the handle until shutdown:

```rust,ignore
#[cfg(feature = "mcp")]
let mcp = ferroui_mcp::serve(ferroui_mcp::Options {
    permissions: ferroui_mcp::Permissions {
        allow_property_writes: false,
        allow_input: false,
    },
    ..Default::default() // HTTP on 127.0.0.1, ephemeral port
})?;
// Run the application's dispatcher/lifetime here, keeping `mcp` alive.
```

This is available in release builds only when the application includes the dependency and makes
the call; there is no automatic release listener or debug listener. The catalog requires both
`--features mcp` and a transport switch.

Desktop lifetime windows are discovered on every request. For a custom lifetime, construct an
`Inspector`, call `register_root(&top_level)`, and pass it to `serve_with_inspector`. The inspector
holds only weak object/root references. IDs remain stable within a server lifetime, are never
reused, and return a tool error after the object is destroyed. Automation trees retain separate
peer IDs, including peers without visual owners. Property and input tools map a control peer to
its owner. A nonvisual peer still supports its automation patterns.

For stdio, reserve stdout **before platform/application initialization**:

```rust,ignore
let output = ferroui_mcp::reserve_stdout()?;
// Set up the application, then on its UI thread:
let mcp = ferroui_mcp::serve(ferroui_mcp::Options {
    transport: ferroui_mcp::Transport::Stdio,
    stdio_output: Some(output),
    ..Default::default()
})?;
```

Reserving stdout redirects ordinary stdout to stderr for the rest of the process and returns a
private protocol writer. On Unix this includes native/library output (descriptor 1 is
redirected); on Windows it covers output through the process's standard handle, such as Rust's
`println!`, while the C runtime's descriptor 1 keeps the original handle. Without a supplied writer, `serve`
reserves stdout when called. Start only one stdio server per process. Stdio EOF stops requests
(and closes ControlCatalog in stdio mode);
dropping the handle stops HTTP acceptance, invalidates queued operations, and releases inspection
state. An OS-blocked stdio reader remains parked until EOF or process exit.

## Tools and resources

| Tool | Behavior |
| --- | --- |
| `list_windows` | Current windows, node IDs, bounds and automation metadata |
| `get_tree` | Visual, logical or automation snapshot; optional root, depth and property filters |
| `find_elements` | Combined type, Name, automation ID, text substring and parsed FerroUI selector filters |
| `get_properties` | Registered and attached properties, values, types, read-only flags, binding presence, effective priorities |
| `set_property` | Validated local value or clear; booleans, numbers, null, and strings through framework converters |
| `get_styles` | Applied style/theme/template/local value frames, selectors, active state, priority and setters |
| `get_resources` | Scoped lookup of specified string keys, or local dictionary keys including type keys |
| `click` | Raw pointer move/down/up at node center or local x/y; left, right or middle button |
| `type_text` | Focus the control, then deliver raw text input |
| `press_key` | Focus and deliver key down/up for a framework key gesture such as `Ctrl+A` or `Return` |
| `scroll` | Raw wheel event at the node center with horizontal/vertical deltas |
| `invoke_pattern` | Invoke, Toggle, Expand/Collapse, SelectionItem, or Value provider actions |
| `screenshot` | PNG of a visual at an optional scale; first window when no ID is supplied |
| `wait_for` | Exists, absent, visible, enabled, or property equality, with a bounded timeout |
| `get_logs` | Up to 1024 recent diagnostics, including binding errors; incremental `after` cursor |
| `get_frame_stats` | Observed rendered scene updates, update interval and last layout pass timing |

`set_property` requires `allow_property_writes`. Input and automation actions require
`allow_input`; the Value provider's `set_value` requires both. Disabled/hidden controls reject
raw input. Unsupported automation patterns and invalid values return explicit tool errors.
`click` uses hit testing, so another control covering the target receives the click as with a
real pointer. Text input inserts at the selection/caret; it does not replace the entire value.

Property priority distinguishes local, style, template, inherited and default values; `get_styles`
identifies theme frames. Binding presence is reported separately because a binding can occupy a
local or style priority. Opaque values are represented as `{"opaque":true}` instead of pretending
they are editable JSON. Attached properties use `Owner.Property` names.

The frame tracker observes composition scene updates after rendering, starting when a window is
first inspected. It does not claim to count idle display refreshes or report GPU presentation
latency. Layout counts/timing come from the renderer's existing diagnostics.

`ferroui://tree` and `ferroui://logs` are JSON resources that can be re-read. Use
`logging/setLevel` to receive `notifications/message` on stdio or an HTTP GET SSE stream. The log
sink forwards to the previous application sink and restores it when the server handle is dropped;
thread-local log sinks still take precedence. Messages preserve templates and values separately.
The ring is bounded, so a slow reader can miss older entries.

## Protocol and execution

The implementation uses a small JSON-RPC adapter rather than an async MCP SDK: UI objects are
thread-affine and the dispatcher already owns scheduling. The protocol version is `2025-11-25`,
with `2025-06-18` and `2025-03-26` accepted. Send `initialize`, then
`notifications/initialized`, before tool/resource requests. HTTP initialization returns
`Mcp-Session-Id`; include it and `MCP-Protocol-Version` on subsequent POST/GET/DELETE requests.
Sessions expire after 30 minutes without HTTP activity. Each HTTP connection carries one request
(`Connection: close`); bodies need a `Content-Length` of at most 1 MiB, and a connection that
stalls for 10 seconds is closed. DELETE ends a session. GET streams log
notifications; POST returns JSON responses or 202 for accepted notifications. The service does
not advertise resource subscriptions, resumable SSE, prompts, sampling, or tasks.

Requests marshal JSON to the UI dispatcher, never moving UI references between threads.
`wait_for` polls on the transport worker so input, timers, layout and rendering can continue.
`notifications/cancelled` cancels waits and queued operations. Dispatcher queue timeouts invalidate
unstarted callbacks; an action that has already begun cannot be rolled back. The core's
`Inspector::execute("wait_for", ...)` is a single nonblocking probe; a future bridge can schedule
its own polling with the same semantics.

Limits: 1 MiB incoming messages, 16 transport workers, 64 HTTP sessions, 10,000 traversed nodes,
64 tree levels, 60-second condition waits, and 16-megapixel screenshots at scales 0.1–4. These
limits and argument schemas are checked before mutation. Prefer a scoped node ID for large trees.

## Browser bridge assessment

The browser page must not host a TCP/stdin server. `ferroui-mcp` is a native-host dependency and
is not added to `ControlCatalog.Browser`. The transport-independent inspector can be called on
the browser's UI thread by a future Emscripten/page adapter: send a JSON tool request through the
page, execute it with the same permissions and weak IDs, and return JSON or a PNG payload. A host
MCP/CDP bridge can then own authentication, session management, timeouts and polling. This avoids
introducing a second native inspection protocol. Implementing that page/CDP bridge belongs to
the separate CDP work; this change evaluates the boundary and provides the shared core.

## Verification

```sh
cargo test -p ferroui-inspect -p ferroui-mcp --lib
cargo build -p control-catalog-desktop --features mcp
python3 scripts/test-mcp.py
scripts/release/readme-crates-table.sh --check
```

The core tests drive a live headless window with the Simple theme and Skia; the protocol tests
cover dispatch, cancellation/timeouts, invalid requests, authentication, Origin, sessions and
resources. The smoke script launches ControlCatalog over each transport, drives SearchBox,
reads all three trees, changes a property, validates PNG output and checks read-only mode.

Protocol reference: [MCP transports](https://modelcontextprotocol.io/specification/2025-11-25/basic/transports)
and [MCP tools](https://modelcontextprotocol.io/specification/2025-11-25/server/tools).
Client setup: [Claude Code MCP documentation](https://code.claude.com/docs/en/mcp).
