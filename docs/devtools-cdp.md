# Chrome DevTools Protocol

FerroUI can expose its top-level windows and native popup windows as local CDP targets.
This is an opt-in subset of protocol version **1.3**, implemented in `ferroui-devtools` over
`ferroui-inspect`. It inspects native controls; it does not embed a browser or JavaScript engine.
The existing F12 developer tools remain available independently.

## Start the ControlCatalog

```sh
FERROUI_CDP_PORT=9222 cargo run -p control-catalog-desktop --features cdp
```

The feature alone does not open a listener. `FERROUI_CDP_PORT` opts in at runtime; `0` selects
a free port and prints its address. `FERROUI_CDP_TRACE=1` prints connection errors for debugging.
The listener is always on `127.0.0.1`. It accepts at most 16 clients, 16 KiB HTTP headers and
1 MiB incoming WebSocket messages. It rejects other Host headers and ordinary web-page Origins.
Native clients without an Origin and the Chrome DevTools frontend are allowed. Local clients
can inspect and operate the application; only start the endpoint in applications you intend to debug.

For another application, enable the facade feature `devtools` or depend on
`ferroui-devtools` with its `server` feature. After platform setup, call
`DevToolsServer::start(port)` on the UI thread and retain the returned handle until shutdown.
Dropping it closes the listener and clients. It discovers windows from the desktop lifetime
and their native popups. For a custom lifetime, register each top-level explicitly:

```rust,ignore
let server = ferroui_devtools::DevToolsServer::start(9222)?;
server.register(&top_level);
// Keep `server` alive while the application's dispatcher is running.
```

The transport threads post each JSON request to the UI dispatcher; the UI thread does no work
while no client sends one. Framework objects, tree walking, input, highlighting and rendering
stay on the UI thread. Requests that wait longer than five seconds expire before execution. The
endpoint is built on the shared inspector of `ferroui-inspect`, which the MCP server uses too:
node IDs are the same in both, weak, and never assigned to a different control. A control that is
removed from the window is rejected until it is attached again, when it has its old ID. Each
client session has its own tree mode, domain subscriptions and highlight, and the highlights of
all sessions are left out of every tree. Framework logs from Information up are forwarded to the
previous log sink and held in the bounded ring of 1024 entries that the MCP server shares.

## Chrome DevTools

1. Open `chrome://inspect/#devices` in Chrome.
2. Choose **Configure**, add `127.0.0.1:9222`, and enable discovery of network targets.
3. Click **inspect** beside the FerroUI window.

Alternatively open the `devtoolsFrontendUrl` returned by
[the local target list](http://127.0.0.1:9222/json/list). The HTTP endpoints are `/json/version`,
`/json/list` (also `/json`) and `/json/protocol`. Discovery includes a browser WebSocket for
multiplexed sessions and a page WebSocket per top-level. A popup hosted within an existing
window remains part of that window's tree.

Elements shows the visual tree with framework type names. Styles shows effective property
values and active value frames (including themes and local setters). The highlight and inspect
picker use the adorner layer and framework hit testing. Console displays framework log messages;
Accessibility reads automation peers. Browser-only panels may report unsupported commands.

## Playwright

Use `chromium.connectOverCDP`, then a **CDP session** to query and drive controls. Playwright's
HTML locators, `page.evaluate`, navigation, script injection and browser contexts are not supported:
those APIs depend on browser DOM and JavaScript semantics. Empty script registration and inert
browser setup commands are accepted so clients can attach; nonempty scripts and evaluation fail
explicitly. The server never evaluates client JavaScript.

```js
import { chromium } from 'playwright';
const browser = await chromium.connectOverCDP('http://127.0.0.1:9222');
const page = browser.contexts()[0].pages()[0];
const cdp = await page.context().newCDPSession(page);
const { root } = await cdp.send('DOM.getDocument', { depth: 1 });
const { nodeId } = await cdp.send('DOM.querySelector', {
  nodeId: root.nodeId, selector: '#SearchBox'
});
await cdp.send('DOM.focus', { nodeId });
await cdp.send('Input.insertText', { text: 'Button' });
const { data } = await cdp.send('Page.captureScreenshot', { format: 'png' });
// Buffer.from(data, 'base64') is a PNG of the native window.
await cdp.detach();
await browser.close(); // Disconnects; leaves the native application running.
```

The runnable ControlCatalog smoke test finds its search box, clicks it through raw input,
types, checks the resulting text and computed properties, reads accessibility, captures a PNG,
and restores the search text:

```sh
npm install --prefix /tmp/ferroui-cdp-client --no-save playwright
PLAYWRIGHT_MODULE=/tmp/ferroui-cdp-client/node_modules/playwright/index.mjs \
  node scripts/cdp/smoke.mjs http://127.0.0.1:9222 target/cdp-smoke.png
```

Other CDP clients can connect directly to a page's `webSocketDebuggerUrl`. Multiplexed clients
use `Target.attachToTarget` with `flatten: true` and include its `sessionId` on subsequent requests.
Closing a target detaches its sessions. Unknown methods return `-32601`; invalid arguments return
`-32602`; framework operation failures return `-32000`; malformed JSON returns `-32700`.

## Supported surface and differences

| Domain | Commands and behavior |
|---|---|
| DOM | `getDocument`, `requestChildNodes`, `describeNode`, `getAttributes`, `pushNodesByBackendIdsToFrontend`, `pushNodeByPathToFrontend`, `querySelector`, `querySelectorAll`, `setAttributeValue`, `getBoxModel`, `getContentQuads`, `getNodeForLocation`, `focus` |
| CSS | `getComputedStyleForNode`, `getMatchedStylesForNode`, `getInlineStylesForNode`; property names and values use FerroUI semantics, not browser CSS. Styles are read-only; use `DOM.setAttributeValue` to set a local property value. |
| Overlay | `highlightNode`, `hideHighlight`, `setInspectMode` (`none` or `searchForNode`); nodes need an adorner layer for highlights. |
| Page | `captureScreenshot` (full-window PNG, 96 DPI, no clip), `getLayoutMetrics`, `getFrameTree`, `getResourceTree`; one frame per target. |
| Input | `dispatchMouseEvent` (move, left/right/middle press/release, wheel), `dispatchKeyEvent`, `insertText`, `dispatchTouchEvent`; window-relative DIPs. Wheel pixels are converted to platform notches at 120 pixels per notch. |
| Accessibility | `getFullAXTree`, `queryAXTree`; names, roles, state and relationships from control automation peers. |
| Log / Runtime | `Log.entryAdded` and `Runtime.consoleAPICalled` for information, warning and error logs. A virtual console context is announced; it has no scripting engine. |
| Target / Browser | Discovery, flattened attachment/detachment, browser-session attachment and auto-attachment to top-levels; no target creation, navigation or browser-context creation. |

`DOM.getDocument` has a FerroUI extension, `tree: "visual" | "logical"` (default `visual`).
Queries use the framework selector parser, including type, `.class`, `#Name`, `:pseudo`, and `>`;
its identifiers and ancestry follow FerroUI rules (combinators use logical parents). Attributes
include `id`/`Name`, classes including pseudo-classes, Text and visibility/enabled state when
registered. Names and pseudo-classes are read-only; class edits preserve pseudo-classes.
Tree structure changes issue `DOM.documentUpdated`; clients should reacquire the tree. Attribute changes issue `DOM.attributeModified`/`DOM.attributeRemoved`, preserving the selected element during hover and focus changes.
Depth `-1` is bounded to 256 levels. Box quads are axis-aligned bounds in target coordinates.

Performance/Tracing, a script language, arbitrary browser CSS editing, DOM mutation beyond local
property/class values, and JavaScript-based Playwright locators are outside this implementation.
The exact advertised command schema ships in `src/FerroUI.DevTools/protocol.json`.

## Browser platform bridge

The browser platform cannot listen on a TCP port. Build `control-catalog-browser` with
`scripts/build-browser.sh control-catalog-browser --features cdp`, or depend on
`ferroui-devtools` with `default-features = false, features = ["browser"]`. On the UI thread,
retain `BrowserBridge::install(&top_level)` after the browser top-level exists. This installs:

```js
window.ferroui.devtools.handle({ id: 1, method: 'Target.getTargets' });
// An array containing the response and any immediately generated events.
window.ferroui.devtools.poll(); // Asynchronous tree, picker, target and log events.
```

Use the returned target ID with `Target.attachToTarget`, then send its `sessionId` on commands.
Use one polling client at a time for a bridge. The client helper detaches its sessions on `await bridge.close()`. Dropping the Rust bridge removes it and releases all sessions. The bridge is explicit opt-in and
is accessible to scripts in the hosting page.

`scripts/cdp/browser-bridge.mjs` exports `connectBridge(realChromePageCDPSession)`. It relays
commands and polls events through Chrome's actual `Runtime.evaluate`; the inner target uses
exactly the same Rust protocol implementation as the native endpoint:

```js
const bridge = connectBridge(await page.context().newCDPSession(page));
const { targetInfos } = await bridge.send('Target.getTargets');
const { sessionId } = await bridge.send('Target.attachToTarget', {
  targetId: targetInfos[0].targetId, flatten: true
});
const tree = await bridge.send('DOM.getDocument', {}, sessionId);
await bridge.close();
```

## Checks

```sh
node --test scripts/cdp/browser-bridge.test.mjs
cargo test -p ferroui-devtools --lib
cargo check -p ferroui --no-default-features --features devtools
cargo check -p ferroui-devtools --no-default-features --features browser \
  --target wasm32-unknown-emscripten
```

The native smoke test above verifies the transport and real ControlCatalog input/rendering.
The WebAssembly check verifies the bridge without requiring a browser canvas or native server.

The implementation was verified on macOS with Playwright 1.64.1 and Chrome 155.0.8059.85:
CDP discovery and attachment, raw mouse/text/key input, effective and matched properties,
automation peers and PNG capture against the ControlCatalog; the Chrome Elements tree expands
and the Styles panel renders native properties. The browser bridge compiles for
`wasm32-unknown-emscripten`; its JavaScript transport has separate unit tests. A full browser
ControlCatalog deployment was not part of this verification.

The frontend check is reproducible against a running catalog:

```sh
# Set CHROME to an installed Chrome executable, or install Playwright's Chromium first.
PLAYWRIGHT_MODULE=/tmp/ferroui-cdp-client/node_modules/playwright/index.mjs \
  node scripts/cdp/devtools-smoke.mjs http://127.0.0.1:9222
```

Chrome may mark framework property names as unknown CSS. Their original names and effective
values remain readable; changes go through `DOM.setAttributeValue`, not the CSS editor.
