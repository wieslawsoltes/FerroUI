# Dynamic QML components

A `Component` holds a deferred object definition and its lexical context. Its `createObject(parent, properties)` method allocates a fresh tree in the current engine. The parent controls ownership. Binding expressions, aliases and JavaScript closures use the definition’s lexical scope.

```qml
import QtQml

QtObject {
    id: scene
    property int offset: 1
    property Component rowFactory: Component {
        QtObject {
            required property int key
            readonly property int score: key + scene.offset
        }
    }
    function append(key: int): QtObject {
        return rowFactory.createObject(scene, { key: key })
    }
}
```

Initial property values and WebScene `Qt.binding()` closures are installed before required-property checks and completion handlers. A newly created subtree shares the owner’s dependency graph and transaction. A failed model or native projection validation rolls back its slot writes, structural changes and object handles. Dynamic IDs are private to each creation.

## Bundled files

Declare selectable filenames when configuring the compiler. Each entry is relative to the main QML document’s directory. The compiler links its dependencies, emits Cargo rebuild inputs and includes its executable definition in the generated module.

```rust
fn main() {
    ferroui_qml_build::Compiler::new()
        .with_javascript()
        .qml_entry("views/Details.qml")
        .compile_file("ui/Main.qml", "main_qml.rs")
        .expect("compile QML");
}
```

The CLI accepts repeatable `--qml-entry` arguments. `Compiler::load` supports the same catalog for source execution. Use `Qt.createComponent("views/Details.qml")` to request a local factory, or `Qt.createComponent(module, typeName)` for a registered native type. Query `status`, `progress` and `errorString()` on the returned object. Asynchronous local requests publish their ready or error status on a subsequent component clock tick. Unavailable URLs produce error-status Components.

QML closures resolve relative component URLs against their declaring QML file. Classic scripts resolve them against the classic script’s directory. ES module code accessing the global Qt helper uses the engine’s project base. The shared fixture uses the same filename in all three locations. [Origin regression fixtures](../tests/QmlCompiled/creation/origin)

## Runtime source and lifetime

`Qt.createQmlObject(source, parent, diagnosticUrl)` parses a new object definition, resolves inline types and embedded project components, and attaches it to the supplied parent. A WebScene engine can also use embedded classic scripts and ES module namespaces from the source. The diagnostic URL determines relative imports and appears in the exception’s `qmlErrors`. Code supplied as text executes through the source evaluator or V8 according to the execution profile.

`destroy(delayMs)` schedules destruction of a dynamically created root and its owned children. Zero delay yields to the next engine turn. `Component.onDestruction` can still read the object. After commit, object properties pointing into the deleted subtree become null, visual children detach, and stale handles cannot address replacement objects. Headless Rust hosts drive scheduled destruction with `Instance::advance_time` or request immediate destruction with `Instance::destroy_object`.

Rust hosts can create objects with `Instance::create_object`, pass guest closure bindings through `Instance::create_object_with_properties`, enumerate live objects, and drain newly committed roots. The shared Qt 6.12 oracle exercises creation, destruction, initialization and resource lookup. [Executable fixtures and reference probe](../tests/QmlCompiled/creation)

## Incubating a component

`Component.incubateObject(parent, properties, mode)` accepts the same initial values and `Qt.binding` closures as `createObject`. Omitting the arguments requests an asynchronous, unparented instance. A pending handle exposes `Component.Loading` and a null `object`. The owner dispatcher prepares its private node storage over successive turns. Headless Rust hosts can drive those turns with `Instance::process_deferred_while`.

```qml
function showCard() {
    const job = card.incubateObject(page, { amount: 7 })
    if (!job) return
    if (job.status === Component.Ready) {
        page.selectCard(job.object)
    } else {
        job.onStatusChanged = function(status) {
            if (status === Component.Ready) page.selectCard(this.object)
        }
    }
}
```

The handle exposes its completed object before delivering the status callback. Failed construction produces `Component.Error`. `forceCompletion()` finishes a pending request immediately and is idempotent after completion. `Qt.Synchronous` normally returns a terminal handle. A synchronous request made inside an asynchronous construction phase may itself be deferred. Completion handlers run after that phase. The requested parent is weak while incubation is pending. If it is destroyed before activation, the result is unparented.

Rust callers can use `Instance::incubate_object` with `IncubationMode` and initial properties. Its `Incubator` exposes `status`, `object`, `force_completion` and explicit `cancel`. Cancellation publishes Error. The record heap and the owner queue manage the handle independently of QML object ownership. Object-valued bindings, final linking, user completion handlers and native mounting run within the final activation transaction.
