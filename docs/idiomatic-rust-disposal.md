# Disposal: from `IDisposable` handles to `Drop` guards (C3b of #423)

The plan for the second half of phase C3 of [idiomatic-rust.md](idiomatic-rust.md): the resources
and subscriptions that are released with an explicit `dispose()` call on an `Rc<dyn IDisposable>`
handle are released by `Drop`. Section 2.1 of that page decided the shape: a guard that is
`#[must_use]`, and keeping a subscription for the life of its source as an explicit call
(`detach()`); each converted API states which it does. This page is the order of the work, measured
on main at 0.5.0-preview.1 (2026-11-01).

## 1. What there is

| Measure (`src/`) | Count |
|---|---:|
| `impl IDisposable for` (`scripts/idioms/inventory.py`) | 371, in 38 crates |
| uses of `IDisposable` | 4520, in 71 crates |
| public functions that return `Rc<dyn IDisposable>` | 333 |
| calls of `.subscribe(` | 665 |
| ... of which drop the returned handle on the spot | about 400 |
| types with both `impl Drop` and `impl IDisposable` | 36 files |

The trait (`ferroui-system`, `i_disposable.rs`) documents the current contract: disposal is
explicit and idempotent, and **dropping a handle keeps the subscription alive**. About 400 call
sites rely on that: they subscribe for the life of the source and drop the handle. A guard that
disposes on drop ends each of those subscriptions at once, without a compile error. The order
below therefore never changes what dropping an existing handle does; it adds the guard, and moves
each API to it only together with every caller of that API, which the compiler then lists
(`#[must_use]` on the guard, an unused guard a warning).

The handles fall into three kinds, which move differently:

- **Resources** (render targets, surfaces, GPU contexts, locks of framebuffers, platform handles,
  file and socket handles): the owner of the handle owns the resource, and releasing it when the
  owner goes away is what `Drop` is for. 36 of these already implement `Drop` as well.
- **Subscriptions of the framework** (event handlers, property observers, the subscriptions of
  bindings, the registrations of input and the dispatcher): some callers keep the handle to
  unsubscribe, many subscribe for the life of the source.
- **Subscriptions of the reactive libraries** (`ferroui-reactive`, `ferroui-dynamic-data`,
  `ferroui-reactiveui` and the crates on them): the Rx contract, where `Subscribe` returns an
  `IDisposable` and the operators compose disposables (`CompositeDisposable`,
  `SerialDisposable`, `RefCountDisposable`). Their public names stay (.NET-named API), and the
  operators need the explicit, shared, idempotent handle.

## 2. The guard

`ferroui-system` gains one type, beside the trait:

```rust
/// A subscription or resource released when the guard is dropped.
#[must_use = "dropping the guard releases it; call `detach()` to keep it for the life of its source"]
pub struct Subscription(Option<Rc<dyn IDisposable>>);

impl Subscription {
    pub fn new(handle: Rc<dyn IDisposable>) -> Self;
    /// Releases now (the same as dropping the guard).
    pub fn dispose(self);
    /// Keeps it for the life of its source; returns the handle for code that still needs one.
    pub fn detach(self) -> Rc<dyn IDisposable>;
}
impl Drop for Subscription { /* disposes the handle, once */ }
impl From<Rc<dyn IDisposable>> for Subscription { ... }
```

The trait stays: it is the handle the guard holds, the interface the reactive operators compose,
and the name of the .NET API. What changes is what the APIs return and what owners hold.

For the reactive libraries, whose `Subscribe` keeps returning the handle (step D5), `ferroui-system`
adds the extension trait `SubscribeGuard`, implemented for every `IObservable<T>` (sized or not, and
so for `Rc<dyn IObservable<T>>` through deref) and re-exported by `ferroui_base::reactive` and
`ferroui_reactive`:

```rust
pub trait SubscribeGuard<T> {
    /// Subscribes `observer`; dropping the returned guard unsubscribes it.
    fn subscribe_guard(&self, observer: Rc<dyn IObserver<T>>) -> Subscription;
}
```

The closure helpers have guard siblings: `ObservableExt::subscribe_fn_guard` of
`ferroui_base::reactive`, and `subscribe_next_guard`, `subscribe_next_error_guard`,
`subscribe_next_completed_guard` and `subscribe_all_guard` of `ferroui_reactive::ObservableExt`.

## 3. Steps

Each step is one pull request from main, with the callers it moves.

| Step | Content | Public API | Risk |
|---|---|---|---|
| D1 | The guard (section 2), with tests: drop disposes once, `detach` keeps, `dispose` after `detach` of a clone, `From` | addition | none |
| D2 | Resources: the types of the first kind release in `Drop` (the 36 that implement both first, then the rest); `dispose()` stays as an early, idempotent release. Owners that called `dispose()` in their own teardown hold the value and let it drop | behaviour: a resource is released when its last owner drops it, where it leaked before | a resource that was dropped while still in use (a clone kept elsewhere is not affected; a value moved out and dropped is) |
| D3 | Internal subscriptions of the framework: fields that hold `Option<Rc<dyn IDisposable>>` to dispose in teardown become `Option<Subscription>`, and the teardown becomes a drop | unchanged | low: the field's owner already disposed at the same point |
| D4 | Public subscription APIs of the framework, one area per pull request (events and handlers, property observation, bindings, input, the dispatcher's timers): they return `Subscription`; every caller in the workspace either keeps the guard or calls `detach()`, which keeps today's behaviour. The old signature stays one release as a deprecated `..._handle` variant where an application may depend on it | 0.5 | medium: each caller is visited (the compiler lists them) |
| D5 | The reactive libraries: `Subscribe` keeps returning `Rc<dyn IDisposable>` (the Rx contract); an extension `subscribe_guard` (`SubscribeGuard`, section 2) returns `Subscription` for RAII callers, and the framework's own uses of the libraries move to it where they kept the handle to dispose | addition | none |
| D6 | Samples, templates and the application repositories (`detach()` where they subscribe for the life of the source); the inventory baseline and the documentation | | |

Status: D1, D2 (the locator scope, `DispatcherProcessingDisabled`, `BatchUpdateOperation`) and D3 (245 fields in `ferroui-base`, `ferroui-controls` and the portable crates; the platform crates and the reactive libraries are open) are merged. D4 began with the element events of `Visual`, `StyledElement` and `Layoutable`, and has done the dispatcher's timers (`DispatcherTimer::run` and `run_once`), the routed and input events (the class handlers and route observables of `RoutedEvent`, `Interactive::add_disposable_handler`, `InputScope::changed`, `KeyboardDevice::property_changed` and `TransformTrackingHelper`), property observation (`PropertyChangedObservable` and `ObjectPropertyChangedObservable`), and the composition, media and rendering events (the `invalidated` and `changed` events of brushes, pens, geometries, transforms and drawings, `MediaCollection`, `MaterialPolicy`, `CompositionBatch::completed`, `Compositor::after_commit`, the system backdrops, `RendererDiagnostics` and `PlatformRenderInterfaceContextManager`); and the events of the controls (part A: the application and its lifetime, the auto-complete and auto-suggest boxes, the automation peers, the calendars and date and time pickers, `ComboBox`, `ContextMenu`, the documents, the flyouts, `Frame`, `InfoBar`, `ItemsControl`, `NativeControlHost`, the native menus and `NavigationView`; part B: `ConnectedAnimation`, `NumberBox`, `NumericUpDown`, the pages, `TextPresenter`, `Popup`, `ScrollBar`, `RatingControl`, the rich-text controls, `Screens`, `SelectionModel`, `TitleBar`, `ToolTip`, `TopLevel`, `TrayIcon`, `VisualStateGroup`, `Window` and `WindowBase`). D4 is complete; trait methods whose implementations live in other crates keep returning `Rc<dyn IDisposable>`. The bindings return their expression (`BindingExpressionBase`) and are not in this step. D5 is done: the extension `SubscribeGuard` (section 2), and 141 fields of the framework that kept an `Rc<dyn IDisposable>` only to dispose it (the subscriptions to observables, and with them the fields D3 left in the platform crates, the collections of handles and the guards the D4 APIs return that were detached into such fields) hold `Subscription` guards, in 32 crates; 15 `Drop` impls that only disposed them are removed. Stayed out, with the handle: the operators of the reactive libraries (the sinks, `SerialDisposable`, `CompositeDisposable`, `RefCountDisposable` and the other disposables of `ferroui-reactive`, the internal operators of `ferroui-dynamic-data`, the sinks of `ferroui-reactiveui` such as `ReactiveCommand`'s execution and `IdleTimeoutObservable`, the work items of `FerroScheduler`), the slots shared with closures (the timer of the legacy dispatcher, the render callback of the repeater, the padding insets of the iOS view), the handles cloned out, passed on or returned (the `Parts` of `NavigationView` and `NavigationViewItem`, which are `Clone`; the dialog and confirmation requests of the dock model; `ViewHostNavigation::release`, which takes the field of the view hosts), the handles disposed in place while their field keeps them (the modal subscription of `Window`, the observable trigger, the pointer release of the dock's window drag helper, the template observables of the document tab strip), the handles replaced or taken without a release on a path the code allows (the resources of the disposing behaviors and triggers, the event handler and loaded subscription of `EventTriggerBase`, the focus tracking of `WindowBaseAutomationPeer`, the client subscriptions of the browser text input, the phased work of the repeater, the platform color subscription of the accent colors), the handles never disposed (kept for the life of their owner: the lights of `Visual`, the context subscriptions of the server compositor, the settings of `MaterialPolicy`, the property subscriptions of `DockTargetBase`, the filters of the tree page of the developer tools, the controller and edge-to-edge subscriptions of the iOS insets), the handles held by thread-local tables (the implicit animations of `ElementCompositionPreview`, the delayed bindings of the markup, the nodes of the Win32 automation, the resolver notification of the Splat locator), the priority-value handles of the behaviors' property stacks and of the command binder, the resources of step D2 (`ScopedGrContext`, the WebGL context restore, the EGL display options), and the handles that already release on drop (`ObservedTranslation` of ProTranslate, the Android dynamic color subscription of the Material theme).

D1 and D3 change no behaviour. D2 is the only step that changes what dropping a value does
without a compiler error, so it moves one area at a time and each pull request names the types and
how their owners hold them. D4 visits every caller of the APIs it changes; the default for a
caller that dropped the handle is `detach()`, so a converted caller behaves as before unless it is
changed on purpose.

## 4. Not in scope

- The names `IDisposable`, `Dispose` in XAML-visible or .NET-named APIs (they stay).
- `CancellationToken` and its registrations (phase C4, [idiomatic-rust-async.md](idiomatic-rust-async.md), section 3.7).
- The column disposal of TreeDataGrid (`IDisposableColumn`): an explicit release by the
  presentation, which is the upstream contract.
