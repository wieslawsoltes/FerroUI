# QML for FerroUI

FerroUI can compile a QML component, instantiate its objects in Rust, and mount its visual objects as native FerroUI controls. The compatibility target is the published Qt QML 6.12.0 language and object model. The implementation was written from the public documentation. Qt is used separately as a behavioral reference in an optional test.

## Run the gallery

```sh
cargo run --locked -p qml-gallery
```

The gallery combines a native FerroUI TextBlock, Qt Quick positioners, a text field and a button. Typing edits the greeting. Clicking Increment changes a QML property and updates the count label. The desktop binary and headless test mount the same compiled component. [QML source](../samples/QmlGallery/ui/Main.qml) · [Rust host](../samples/QmlGallery/lib.rs)

![The Linux desktop gallery after entering Ada and clicking Increment.](images/qml-gallery-linux.png)

## Compile a component in Cargo

Applications inside the workspace can use these dependencies.

```toml
[dependencies]
ferroui-qml.workspace = true
ferroui-qml-controls.workspace = true

[build-dependencies]
ferroui-qml-build.workspace = true
```

Compile the file from the application's build script.

```rust
fn main() {
    ferroui_qml_build::compile_file("ui/Main.qml", "main_qml.rs")
        .expect("compile Main.qml");
}
```

Include the module in the application. Retain the returned view while the controls are mounted. It owns the live component and the subscriptions that connect native events to QML.

```rust
use ferroui_qml::Error;
use ferroui_qml_controls::{ControlRegistry, QmlView};

include!(concat!(env!("OUT_DIR"), "/main_qml.rs"));

fn create_view() -> Result<QmlView, Error> {
    let registry = ControlRegistry::standard()?;
    QmlView::new(&compiled_qml(), &registry, Default::default())
}
```

The optional `qml` feature of the FerroUI facade also exposes the runtime and controls crates as `ferroui::qml` and `ferroui::qml_controls`. Generated modules name the direct dependency `ferroui_qml`.

### Check against a Rust type registry during the build

For model-only QML, add `ferroui-qml` to build dependencies and use the same type registrations used by the host. The structural check resolves constructors, member assignments, aliases, handlers, object-valued properties and required child properties. Bindings and application callbacks execute when an instance is created. Required root properties can be supplied through the initial property map.

```rust
fn main() {
    let types = ferroui_qml::TypeRegistry::new();
    ferroui_qml_build::Compiler::new()
        .import_path("qml-modules")
        .check_with(|doc| types.check(doc))
        .compile_file("ui/Model.qml", "model.rs")
        .expect("compile Model.qml against the application types");
}
```

The command-line compiler accepts import roots with `-I`. Its `--check` mode checks source and module linkage. Native type contracts are supplied by the host through the Rust API.

```sh
cargo run --locked -p ferroui-qml-build --bin ferroui-qmlc -- --check samples/QmlGallery/ui/Main.qml
cargo run --locked -p ferroui-qml-build --bin ferroui-qmlc -- samples/QmlGallery/ui/Main.qml > main_qml.rs
```

## Modern JavaScript with WebScene V8

Enable `javascript` on the build helper and use `ferroui-qml-webscene` in the application. The published WebScene 1.0.37 native package contains V8 15.3.10 and ABI 3. The gallery can run the same native controls with either execution backend. Use Python 3.11 or newer for the package verification step. On Linux and macOS, run the following from the workspace root.

```sh
export FERROUI_WEBSCENE_LIBRARY="$(python3 scripts/qml/webscene-runtime.py target/webscene-runtime)"
cargo build --locked -p ferroui-qml-webscene --bin ferroui-qml-webscene-worker
cargo run --locked -p qml-gallery --features webscene
```

On Windows, run these commands in PowerShell.

```powershell
$env:FERROUI_WEBSCENE_LIBRARY = python scripts/qml/webscene-runtime.py target/webscene-runtime
cargo build --locked -p ferroui-qml-webscene --bin ferroui-qml-webscene-worker
cargo run --locked -p qml-gallery --features webscene
```

The setup helper checks the SHA-256 of the published NuGet package and extracts the library, ICU data, snapshot and licenses together. It supports Linux x64 and ARM64, macOS x64 and ARM64, and Windows x64. Keep the native files together when distributing an application. Put the worker beside the application executable or specify both absolute paths with `WebSceneEngine::new(worker, library)`. `WebSceneEngine::from_env()` also accepts `FERROUI_WEBSCENE_WORKER`.

For a workspace application, keep the regular runtime dependencies above and select the build helper feature.

```toml
[dependencies]
ferroui-qml.workspace = true
ferroui-qml-controls.workspace = true
ferroui-qml-webscene.workspace = true

[build-dependencies]
ferroui-qml-build = { workspace = true, features = ["javascript"] }
```

```rust
// build.rs
fn main() {
    ferroui_qml_build::Compiler::new()
        .with_javascript()
        .compile_file("ui/Main.qml", "main_qml.rs")
        .expect("compile QML and its JavaScript imports");
}
```

```rust
use ferroui_qml::Error;
use ferroui_qml_controls::{ControlRegistry, QmlView};
use ferroui_qml_webscene::WebSceneEngine;
use std::{rc::Rc, time::Duration};

include!(concat!(env!("OUT_DIR"), "/main_qml.rs"));

fn create_view() -> Result<QmlView, Error> {
    let engine = WebSceneEngine::from_env()?.with_timeout(Duration::from_secs(2));
    QmlView::with_script_engine(
        &compiled_qml(), &ControlRegistry::standard()?, Default::default(), Rc::new(engine)
    )
}
```

The compiler parses JavaScript with SWC, resolves relative files, watches the transitive dependency graph for Cargo and embeds executable scripts in the generated Rust. V8 compiles the JavaScript functions in the worker and caches their factories. For a binding such as `parent.width`, the runtime checks the actual receiver, declaration scope and property type. A proven read of a scalar QML slot stays on the UI thread and participates in normal dependency tracking. Method references, JavaScript-object members and general expressions retain guest execution. Static module initialization runs before this optimization takes effect. The default build mode continues to generate native Rust for the built-in language profile. To load a file at runtime, use `Compiler::new().with_javascript().load(path)?.document` and pass the linked document to `Instance::with_engine` or `QmlView::with_script_engine`.

### Scopes, modules and host objects

- Classic `.js` code-behind has state per importing component. Local `.import` aliases have their own scope, and `.pragma library` state is shared inside one instance's JavaScript realm.
- ES module exports, re-exports, cycles, top-level await, `import.meta.url` and literal dynamic imports run in V8. The released WebScene ABI lacks a native module-import callback. The build helper lowers ESM to `System.register` with SWC and an in-memory SystemJS linker executes the resulting module graph. Deferred modules execute when imported. Static resources and dynamic imports must resolve to local files at build time.
- A property read through a JavaScript proxy goes through the active QML transaction, so a binding subscribes to the branch it actually executes. Proxy writes and nested calls use the same transaction and notification order. Guest functions and objects keep identity across synchronous Rust calls in the same instance.
- The worker uses WebScene’s ABI 3 synchronous resource callback and its pinned JavaScript text-loader hook to exchange in-memory requests with the QML thread. No server or port is used. One backend can serve several QML instances. Each instance owns a separate worker and V8 realm. A timed-out worker is terminated. Committed QML slots survive the reset, and references into the old JavaScript heap become stale. Rollback applies to QML-managed changes. Arbitrary mutations of V8-owned closure state and module globals follow JavaScript semantics.
- With WebScene, a signal's `connect(fn)` registers a JavaScript function, a bound QML method, or another signal. The optional form `connect(receiver, fn)` supplies the callback's `this`. `disconnect` removes all matching registrations. A callback can disconnect a later listener during delivery. Registrations added during delivery take effect on the next emission. These callbacks also observe `Instance::emit` and property writes from Rust. QML transaction rollback restores changed registrations. After a worker restart, reconnect callbacks in the new realm. [Signal fixture](../tests/QmlCompiled/webscene/Signals.qml)
- Rust can inspect a retained Promise with `Instance::promise_state`, wait for it with a time budget via `settle_promise`, or call and await a method as one QML transaction via `call_and_await`. A rejection rolls back QML writes made by that atomic call. V8 microtasks can compute and import modules. A continuation that attempts QML host access after the synchronous invocation ends receives an error. Model the later UI update as an explicit host call handling the settled result.

The module tests use a shared QML fixture that also runs on Qt 6.12.0 and a separate async fixture for extended V8 behavior. Direct V8 execution follows ECMAScript semantics where Qt differs. For example, V8 preserves a trailing hole when slicing a sparse array, whereas Qt 6.12 trims it. [Shared fixture](../tests/QmlCompiled/webscene/App.qml) · [Promise fixture](../tests/QmlCompiled/webscene/Async.qml) · [Published Linux runtime](https://www.nuget.org/packages/WebScene.NativeEngine.Runtime.linux-x64/1.0.37)

## Architecture

| Crate | Responsibility |
|---|---|
| `ferroui-qml-compiler` | Tokenizes UTF-8, parses object and script syntax, records source spans, validates declarations and links component definitions. The module resolver is supplied by the caller. |
| `ferroui-qml-build` | Resolves local files and versioned module exports, tracks Cargo inputs and emits readable Rust. Rust compiles the expressions, methods, signal handlers and control flow in the supported language. |
| `ferroui-qml` | Owns object identity, typed slots, component scopes, aliases, reactive dependencies, signal dispatch and native callback registration. Both execution paths share property reads, writes and conversions. |
| `ferroui-qml-webscene` | Provides the WebScene V8 backend, its worker executable, bounded request protocol and embedded JavaScript module loader. |
| `ferroui-qml-controls` | Creates FerroUI controls, validates and applies native property updates, projects child collections and forwards native input. It also supports registration from FerroUI class and property metadata. |

Each instance owns its object arena. Object references carry an instance identity. The linker gives each composite instance a private ID scope and resolves each type using its defining document's imports. Statically referenced module singletons share the same dependency graph as their consuming components. They are allocated eagerly, once per instance arena.

When a binding runs, the runtime tracks the properties it actually reads. A conditional binding can therefore stop listening to the discarded branch. Imperative assignment detaches the existing binding. A failing transaction restores the previous values and dependency edges and discards pending host notifications. Native Rust functions used in bindings must be deterministic with respect to their declared arguments. Application-owned side effects inside a callback are outside the transaction.

## Compatibility with Qt QML 6.12

| Area | Available in this implementation | Remaining coverage |
|---|---|---|
| Components and modules | File and inline components, typed composite properties and lists, private IDs, import aliases, local directory imports, versioned `qmldir` exports and source singletons | Dynamic creation, incubation, destruction and loader lifecycle, multiple independently loaded roots in one engine, complete pragma behavior |
| Object model | Required, readonly and default properties, lexical source property shadowing with virtual, override and final declarations, aliases, typed signals and methods, property change handlers, `Component.onCompleted`, source and registered enums | Complete Qt value-type library, attached properties, interception, aliasing and inheritance edge cases, including overriding properties supplied by native Rust types |
| Built-in Rust scripting | Numeric, boolean and string expressions, object and array literals with identity, indexed reads and writes, function and signal calls, local declarations, assignment, branching, loops and returns | Closures, prototypes, exceptions, Promises and local script modules require an external engine. |
| WebScene scripting | V8 built-ins, closures, classes, prototypes, exceptions, Promises, classic scripts and local ES modules. Both the file loader and Cargo-generated components use the same engine. | Computed import paths, Qt WorkerScript, asynchronous QML host callbacks and browser-side V8 deployment |
| Reactive infrastructure | Dependency tracking, signal connections, conditional Binding overlays with restoration, Timer with a deterministic clock and native dispatcher integration | Delayed and multi-target Binding forms, the complete Qt global object, network and resource loading |
| Models | `QtQml.Models.ListModel` and flat `ListElement` declarations, fixed and dynamic role types, insertion, moves, removal, role edits, row handles and rollback | Nested list roles, function-valued roles, model/view delegates, proxy and worker-thread models |
| Native visuals | Item, Rectangle, Row, Column, Flow, Text, TextInput and TextEdit. The Controls module supplies Label, Button, CheckBox, TextField and TextArea. Parent fill and center anchors participate in native arrangement. | Sibling and edge anchors, complete implicit geometry, visual delegates, animation/state systems and the rest of Qt Quick and Controls |
| FerroUI API | Register a Rust type, properties, signals, enum values and deterministic methods. Register visual factories or expose supported scalar styled properties with `register_ferro_control`. | Property types outside the reflected scalar set require an explicit native adapter |
| Tooling | Cargo build integration, native Rust code generation, command-line checks, source diagnostics, headless tests and a Qt reference harness | Language server, debugger, profiler, designer support and Qt binary plugin/toolchain interoperability |

## Verification

```sh
cargo test --locked -p ferroui-qml-compiler -p ferroui-qml -p ferroui-qml-build -p qml-compiled-tests
cargo test --locked -p ferroui-qml-controls
cargo test --locked -p qml-gallery --no-default-features
```

The compiled fixtures exercise duplicate internal IDs across instances, writes through aliases, typed Rust calls, singleton identity, method control flow and transaction rollback. A separate fixture compares the two FerroUI backends with the published Qt runtime on original inputs. Qt and Python are only needed to repeat the live comparison. [Behavioral reference and reproduction steps](../tests/QmlCompiled/reference/README.md)

The inventory fixture checks a sequence of edits against the same resulting row order and role values observed in Qt 6.12.0. It also tests FerroUI rollback and rejection of stale row handles. [Inventory component](../tests/QmlCompiled/models/Inventory.qml)

## Public references

- [Qt QML 6.12](https://doc.qt.io/qt-6.12/qtqml-index.html)
- [QML language reference](https://doc.qt.io/qt-6.12/qmlreference.html)
- [Object attributes](https://doc.qt.io/qt-6.12/qtqml-syntax-objectattributes.html)
- [Property binding](https://doc.qt.io/qt-6.12/qtqml-syntax-propertybinding.html)
- [Scope and name resolution](https://doc.qt.io/qt-6.12/qtqml-documents-scope.html)
- [Module imports](https://doc.qt.io/qt-6.12/qtqml-syntax-imports.html)
