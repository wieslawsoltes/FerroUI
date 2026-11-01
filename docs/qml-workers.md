# WorkerScript and native dispatch

Import `QtQml.WorkerScript` to run a local JavaScript script or ECMAScript module in an independent WebScene V8 process. Each WorkerScript owns its global object, loaded modules and FIFO input queue. Assign a source and send copied values from a method or signal handler.

```qml
import QtQml
import QtQml.WorkerScript

QtObject {
    id: app
    property string answer
    property WorkerScript calculator: WorkerScript {
        source: "sum.mjs"
        onMessage: function(result) { app.answer = result.label }
    }
    function calculate() { calculator.sendMessage({values: [2, 3]}) }
}
```

```js
// sum.mjs
WorkerScript.onMessage = ({values}) => {
    const total = values.reduce((value, next) => value + next, 0)
    WorkerScript.sendMessage({label: String(total)})
}
```

Use the build helper's JavaScript profile and select `WebSceneEngine` when constructing the component. Literal sources and their static dependencies are embedded and watched by Cargo. For a runtime-selected source, add `Compiler::worker_script_entry("tasks/alternate.js")` or pass `--worker-entry tasks/alternate.js` to `ferroui-qmlc`. Runtime lookup is relative to the QML file that declares the worker and is confined to the linked catalog. A plain `.js` entry runs as a classic script and a `.mjs` entry runs as an ECMAScript module.

Messages sent before `ready` are queued. QML publishes source changes and outgoing messages when the originating transaction commits. Plain objects, arrays, numbers, booleans, strings, null and undefined are copied across the process boundary. Sending a ListModel grants access to a private model snapshot with a revision. In the worker, change roles using the usual ListModel methods and call `sync()` to submit the edits to the owner thread. A single transaction applies the batch. A conflicting owner revision rejects `sync()` and the application can send a fresh snapshot to retry. A dropped or timed-out waiter revokes queued edits that have not started.

A native `QmlView` posts delivery work to its FerroUI dispatcher. A Rust host can use `Instance::set_worker_waker` to signal its own event loop and `poll_worker_scripts(limit)` on the owner thread to deliver queued events. Each delivery gets its own QML transaction. Keep dispatching while a worker is synchronizing a model.

The process supervisor bounds every JavaScript invocation using the timeout configured on the WebScene adapter. The per-worker input queue permits 256 messages. The owner inbox permits 1,024 events across the instance. Both directions enforce a 32 MiB queue budget. Source replacement and destruction cancel the old process and revoke its pending deliveries. Core QML remains headless-capable and uses a script engine that implements the worker API for these elements.

[Shared fixture and Qt reference](../tests/QmlCompiled/workers/) · [Rust integration tests](../tests/QmlCompiled/workers.rs)
