# Running QML JavaScript in a browser

Run the QML Wasm instance in a dedicated application worker. Each instance executes its JavaScript in a separate script worker with the shared QML scope, module and host-object protocol. Property reads, writes, reentrant calls and bindings run against the current Rust transaction. JavaScript modules and classic scripts come from the compiled resource catalog. A thrown exception rolls back QML writes. The backend supports Promise inspection and bounded settlement, including access to the owner after `await` in compiler-lowered async functions.

Compile the QML entry at build time using `ferroui_qml_build::Compiler::with_javascript()`. In the application worker, start a script worker before constructing the instance so its initial bindings can execute synchronously.

```rust
use ferroui_qml::{ComponentSource, Error, Instance, TypeRegistry};
use ferroui_qml_webscene::WebSceneEngine;
use std::{rc::Rc, time::Duration};

pub async fn load(
    component: &impl ComponentSource,
    types: &TypeRegistry,
) -> Result<Instance, Error> {
    let engine = Rc::new(WebSceneEngine::browser()?.with_timeout(Duration::from_secs(5)));
    engine.prepare_browser_instance().await?;
    Instance::with_engine(component, types, Default::default(), engine)
}
```

Call `prepare_browser_instance().await` once per new scripting instance when sharing an engine. An overdue guest operation terminates the worker. Prewarm a replacement before the next script operation on the affected instance. Rust-managed QML state survives rollback, and JavaScript objects belonging to the terminated worker expire. Dropping the instance terminates its worker. Dropping the engine also terminates unused prewarmed workers.

## Calling QML from a page

The packaged `browser_application.js` module lets window code call a compiled application while the Rust instance and its JavaScript guests run in dedicated workers. Serve the module alongside the application's worker entry. An application that already loads the generated Wasm package can also obtain the module URL from the exported `qmlApplicationModule()` function after initialization.

```js
// page.js
import {openQmlApplication} from "./browser_application.js";

const app = await openQmlApplication(new URL("./owner.js", import.meta.url));
const stop = await app.watch("total", value => { output.textContent = value; });
await app.set("seed", 4);
const result = await app.call("bump", [2]);
const title = await app.get("caption", "nested"); // a QML id
await stop();
app.dispose();
```

The worker entry installs its factory. The handshake completes after the factory returns a `BrowserApplication`.

```js
// owner.js
import {serveQmlApplication} from "./browser_application.js";
import init, {createModel} from "./pkg/my_app.js";

serveQmlApplication(async () => {
    await init();
    return createModel();
});
```

Export the factory from the application's Wasm crate. The `load` function above constructs the compiled QML instance.

```rust
#[wasm_bindgen::prelude::wasm_bindgen(js_name = createModel)]
pub async fn create_model() -> Result<ferroui_qml_webscene::BrowserApplication, wasm_bindgen::JsValue> {
    let model = load(&compiled_qml(), &ferroui_qml::TypeRegistry::new()).await
        .map_err(|err| wasm_bindgen::JsValue::from_str(&err.to_string()))?;
    ferroui_qml_webscene::BrowserApplication::new(model, std::time::Duration::from_secs(5))
        .map_err(|err| wasm_bindgen::JsValue::from_str(&err.to_string()))
}
```

`get`, `set`, `call` and `watch` address the root by default and accept a QML id as their last argument. Values cross the boundary by copy, including plain records, arrays, strings, booleans, numbers, null and undefined. Reading a ListModel yields an array of role records. Watching its property reports committed row edits, including changes to nested models. Other watches report committed property changes. `watch` resolves to an unsubscribe function. Calls await returned Promises. Exceptions, rejection and untransferable results roll back the call's QML writes.

Each client owns one application and executes requests in order. Up to 256 pending requests share a 32 MiB input budget. The default client deadline is 15 seconds per request. Pass `{timeout, signal, onError}` when opening the application to configure the deadline in milliseconds, cancel its lifetime, or receive asynchronous errors. Disposal, cancellation and request timeout terminate the application worker and reject pending requests. The owner delivers timer, deferred, fetch and WorkerScript events between requests. A failed asynchronous QML transaction rolls back and reports through `onError`. Other requests and event sources can continue. A fatal worker or transport failure closes the client. [Application module](../src/Markup/FerroUI.Qml.WebScene/browser_application.js) · [Chromium fixture](../tests/QmlBrowser/index.html)

## Hosting

The page must be in a secure context with cross-origin isolation enabled. Serve `Cross-Origin-Opener-Policy: same-origin` and `Cross-Origin-Embedder-Policy: require-corp` or compatible isolation headers. The application creates a nested worker from a Blob and compiles packaged JavaScript functions at runtime. The page's Content Security Policy must allow those operations. The script worker runs trusted application code with its origin's worker permissions.

Each transport direction uses a 64 KiB shared slot. Longer UTF-8 messages stream in chunks, up to 32 MiB per frame. The application worker waits for the result with a deadline and services nested QML calls in the meantime. This backend requires blocking `Atomics.wait` support in the application worker. The window thread stays available for painting and input.

## WorkerScript

Each `WorkerScript` declaration creates a separate browser worker and resolves its source against the embedded component catalog. The application worker receives replies asynchronously. The packaged page adapter installs the waker and drains replies automatically. A custom host calls `Instance::poll_worker_scripts` when its worker waker fires. A worker calling `ListModel.sync()` waits for that transaction to commit or reject the edits. Replacing the source or destroying the object terminates the guest and cancels queued deliveries. [WorkerScript example](qml-workers.md)

## Fetching QML and ECMAScript modules

An application worker can fetch QML at runtime using `BrowserComponentTransport`. Enable `browser-fetch` on `ferroui-qml` and register the transport before creating the instance. For fetched QML that imports JavaScript, also enable `javascript` so the Wasm application can compile the new source, then construct the instance with the prepared browser script engine. The browser conformance fixture serves both the QML and its imported ES module over HTTP and checks the evaluated result. [Worker fixture](../tests/QmlBrowser/lib.rs) · [Resource loading](qml-network.md)

## Chromium verification

The [browser fixture](../tests/QmlBrowser) builds the QML component with Cargo, loads real Wasm and a JavaScript module in headless Chromium, and exercises dependency updates, nested calls, imperative closure bindings, Promise settlement, rollback, multichunk UTF-8 data, instance isolation and worker timeout/recovery. It also checks WorkerScript FIFO delivery, classic and ECMAScript module entry points, nested model conflicts, source replacement and object destruction. [Conformance workflow](../.github/workflows/qml.yml)
