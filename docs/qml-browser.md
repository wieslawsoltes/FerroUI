# Running QML JavaScript in a browser

Run the QML Wasm instance in a dedicated application worker. Each instance executes its JavaScript in a separate script worker with the shared QML scope, module and host-object protocol. Property reads, writes, reentrant calls and bindings run against the current Rust transaction. JavaScript modules and classic scripts come from the compiled resource catalog. A thrown exception rolls back QML writes. The backend supports Promise inspection and bounded settlement, including access to the owner after `await`.

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

## Hosting

The page must be in a secure context with cross-origin isolation enabled. Serve `Cross-Origin-Opener-Policy: same-origin` and `Cross-Origin-Embedder-Policy: require-corp` or compatible isolation headers. The application creates a nested worker from a Blob and compiles packaged JavaScript functions at runtime. The page's Content Security Policy must allow those operations. The script worker runs trusted application code with its origin's worker permissions.

Each transport direction uses a 64 KiB shared slot. Longer UTF-8 messages stream in chunks, up to 32 MiB per frame. The application worker waits for the result with a deadline and services nested QML calls in the meantime. This backend requires blocking `Atomics.wait` support in the application worker. The window thread stays available for painting and input. `QtQml.WorkerScript` currently uses the desktop WebScene backend.

## Chromium verification

The [browser fixture](../tests/QmlBrowser) builds the QML component with Cargo, loads real Wasm and a JavaScript module in headless Chromium, and exercises dependency updates, nested calls, imperative closure bindings, Promise settlement, rollback, multichunk UTF-8 data, instance isolation and worker timeout/recovery. [Conformance workflow](../.github/workflows/qml.yml)
