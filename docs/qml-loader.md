# Loading QML subtrees

`QtQuick.Loader` creates its item in the surrounding FerroUI QML instance. The item has its own component ID scope. Its ownership parent is the Loader, so a native visual root enters the Loader's `data` collection and participates in the normal retained control tree. The Loader can also own a nonvisual `QtObject`.

## Component definitions and files

```qml
import QtQuick

Item {
    id: page
    property int amount: 7
    Component {
        id: summary
        Rectangle { width: 120; height: 32; color: "blue" }
    }
    Loader {
        id: content
        sourceComponent: summary
    }
    function showDetails() {
        content.setSource("Details.qml", {
            amount: Qt.binding(function() { return page.amount })
        })
    }
}
```

A Component-valued property also accepts an inline object definition, such as `sourceComponent: Rectangle { width: 120; height: 32 }`. The runtime allocates that subtree when a component instance is requested. Its internal IDs remain private to that instance.

The file compiler discovers an existing project-local QML file assigned as a string literal to `Loader.source`, bundles the linked definition, and tracks it as a Cargo dependency. Declare files selected by methods or runtime expressions in the build configuration.

```rust
ferroui_qml_build::Compiler::new()
    .with_javascript()
    .qml_entry("Details.qml")
    .compile_file("ui/Main.qml", "main_qml.rs")?;
```

Each component entry is relative to the input QML file's directory. Native expressions and WebScene use the same catalog, object allocator and destruction path. WebScene additionally accepts a `Qt.binding` closure in the initial property map. Initial values satisfy required properties and are available during `Component.onCompleted`.

## Lifecycle and layout

- A successful load publishes `item`, sets `status` to `Loader.Ready`, sets `progress` to 1 and emits `loaded()`. `sourceComponent` exposes the file's component factory when loading by URL.
- Setting `active` to false releases the item and changes the status to `Loader.Null`. The Loader remembers the selected source and the map supplied to `setSource()`. Reactivating creates a fresh object. Calling `setSource()` for the same URL applies the new map to a fresh instance.
- Selecting another source retires the previous subtree and disconnects its native input subscriptions. An unavailable catalog entry or invalid required-property initializer produces `Loader.Error`. An exception from a running QML handler aborts its enclosing transaction and restores the previous selection and subtree.
- With `asynchronous: true`, an active Loader publishes `Loader.Loading`. Across owner turns, it privately prepares native member metadata, declared property storage, method bodies and signal signatures for the static child tree. These allocations move into the live instance when construction completes. Changing the source, deactivating or destroying the Loader discards pending work. Setting `asynchronous` to false while loading completes the current selection synchronously.
- During preparation, `item` stays null. Binding evaluation, ID and alias linking, object-valued property construction, completion handlers and native validation run in the final transaction. An asynchronous completion error rolls back the new subtree, sets `Loader.Error` and surfaces the exception to the host.
- Width and height are independent. An unassigned axis follows the loaded visual item's QML size. An explicit Loader axis applies to the item when it loads or when that Loader dimension changes. Assigning `undefined` restores automatic sizing on that axis. The last size remains available while the Loader is inactive.

## Driving incubation

Native views run deferred work at background priority with a four-millisecond cooperative budget. A partially prepared tree resumes on a timer targeting the next 16 ms interval, leaving the dispatcher available for input and rendering. A single native type preparation, a callback, or the final activation and native mount may exceed that budget.

Headless hosts can call `Instance::process_deferred(limit)` once per owner turn or supply their own clock through `Instance::process_deferred_while(limit, has_time)`. The predicate is checked between jobs and between preparation steps. Newly queued continuations wait until the next call. While `has_deferred_work()` is true, the host must schedule another turn.

The shared fixture covers replacing an inline item with a file, caching initial properties while inactive, reloading, error status and unloading. It runs through source-loaded and Cargo-generated components with both execution engines. The offscreen Qt 6.12.0 reference uses the same QML file. [Fixture](../tests/QmlCompiled/loader/Main.qml) · [Reference runner](../tests/QmlCompiled/loader/qt_reference.py) · [Qt Loader API](https://doc.qt.io/qt-6/qml-qtquick-loader.html)
