# Debugging and profiling QML

Enable `ferroui-qml/diagnostics` in a diagnostic build. The default runtime compiles out breakpoint checks, clocks and source metadata added by this feature.

## Command-line debugger

Run the headless QtQml fixture from the repository root.

```sh
cargo run -p ferroui-qml-build --features diagnostics --bin ferroui-qml-debug -- \
  --trace --break method:nested --call mutate tests/QmlCompiled/diagnostics/Probe.qml
```

At a stop, enter `args` to see the formal parameters and their current values, or `get count` to inspect a receiver property. Enter `i` to step into the next declaration, `n` to step over nested declarations, or `o` to stop after returning to a shallower declaration. Enter `c` to continue or `a` to abort the current QML transaction.

Use `--step` to stop at the first executed declaration. Member breakpoints accept `method:NAME`, `binding:PROPERTY` and `handler:onSignal`. A byte breakpoint such as `--at 'https://qml.invalid/components/project/Part.qml@36'` stops when a declaration covering that UTF-8 offset executes. Repeated `-I` options add module search paths. Each `--call` invokes a zero-argument root method after initialization.

The runner uses the standard QtQml registry and source evaluator. For a visual application, custom native types, AOT Rust or WebScene JavaScript, attach a debugger to the application’s own runtime.

## Host integration

```rust
use ferroui_qml::diagnostics::{Breakpoint, Debugger, Kind, Resume};

let debugger = Debugger::default();
debugger.set_breakpoints([Breakpoint::Member {
    kind: Kind::Method,
    name: "save".into(),
}]);
debugger.on_pause(|frame| {
    eprintln!("Stopped in {} on {:?}", frame.site.name, frame.receiver);
    for (parameter, value) in frame.parameters.iter().zip(frame.arguments) {
        eprintln!("{} = {:?}", parameter.name, value);
    }
    Resume::StepOver
});
instance.set_debugger(Some(debugger.clone()));
```

`Instance::with_debugger` also observes startup bindings and completion handlers. For independent documents, attach to `engine.instance_mut()` before `load_root`. The debugger remains attached after `Engine::reset`. Use `set_debugger(None)` to detach. A pause hook runs synchronously on the engine’s owner thread. A separate frontend can send resume commands to a host-defined channel while that thread waits. The hook receives a read-only view of the transaction, so pending writes may already be visible. Guest JavaScript and generated Rust step at QML declaration boundaries.

## Measurements

Register `on_trace` to stream paired entry and exit events with the receiver, source, byte span, nesting depth and error outcome. `profile()` snapshots completed calls and `take_profile()` drains them. Identical declarations on multiple objects accumulate into the same record. Inclusive time includes nested bindings and methods, exclusive time subtracts them, and the longest call uses inclusive time. Measurements exclude time spent in trace and breakpoint callbacks and include attempts that subsequently roll back. A synthesized binding may have no source span. Source attribution for a binding identifies the evaluated property declaration.

The command-line runner writes events and interactive prompts to standard error and a tab-separated profile to standard output, sorted by exclusive time.
