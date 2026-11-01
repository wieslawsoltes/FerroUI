# Loading QML from HTTP and HTTPS

Both `Loader.source` and `Qt.createComponent(url)` can load remote QML. A Rust application enables its transport on the type registry before constructing the instance or mounting a view. Bundled component URLs continue to resolve against the Cargo catalog.

On desktop, enable the `http` Cargo feature on `ferroui-qml` and configure the origins the application uses.

```rust
use ferroui_qml::{network::{HttpComponentTransport, Url}, TypeRegistry};
use std::rc::Rc;

let mut types = TypeRegistry::new();
let origin = Url::parse("https://widgets.example.org")?;
types.set_component_transport(Rc::new(HttpComponentTransport::new(&[origin])?));
```

For browser applications, enable the `browser-fetch` feature and register `BrowserComponentTransport::default()`. The transport reads bounded HTTP response streams from the application origin and wakes the owner when the read finishes.

For native views, configure `ControlRegistry::types_mut()` in the same way. The native transport uses TLS verification, a 15-second timeout and a 16-transfer limit. It rejects redirects. A host with its own networking stack can implement `ComponentTransport::request` and complete the supplied `ComponentReply` with UTF-8 bytes or a transport error. The reply can cross threads. Its `is_pending()` method allows a transport to stop work for a retired factory.

The component enters `Loading` immediately. Transfer begins after the creating QML transaction commits. Source parsing, linking, binding evaluation and status signals run on the owner thread. Imports of same-origin QML directories and sibling composite types resolve relative to the defining file. Each pending component accepts at most 64 files, 2 MiB per file and 8 MiB in total. Directory imports consult the remote `qmldir` listing. For identified modules, call `add_component_import_path` with a same-origin module root. For example, `import Lab.Widgets 1.1` looks up `Lab/Widgets/qmldir` and selects the public component file exported at that version. A downloaded manifest can combine its QML files with a static Rust plugin installed in the host registry. An unregistered required plugin fails module loading. Inline JavaScript in fetched QML uses the configured script engine.

Mounted views wake the dispatcher when a reply arrives. A headless host can register an owner-thread wakeup using `Instance::set_component_waker` and drain replies with `poll_component_loads(limit)`. A failed transfer, parse or link sets `Component.Error` and records a diagnostic in `errorString()`. Replacing a Loader source retires its old factory and discards obsolete replies.

[Qt reference fixture](../tests/QmlCompiled/network/Card.qml) · [Qt QML network transparency](https://doc.qt.io/qt-6/qtqml-documents-networktransparency.html)

A fetched QML document can import same-origin classic scripts and ES modules when the instance uses WebScene and the runtime has the `javascript` feature. The transport fetches static imports, classic `.import` directives and literal dynamic imports under the QML dependency graph limits. A computed import can resolve modules already present in that graph. The JS engine reads only its in-memory catalog. Classic code keeps one scope per consuming component, and `.pragma library` and ES modules share state within the instance. Both Cargo and remote loading use the same SWC parser and System.register compiler. V8 `import.meta.url` and classic `Qt.resolvedUrl()` preserve the source URL.

Each instance retains up to 256 remote script resources and 8 MiB of remote source and generated code. Failed VM installations count against the budget. A successfully installed module URL identifies immutable source for the lifetime of the instance. After a failed installation, a new component request can retry the same URL with corrected source.
