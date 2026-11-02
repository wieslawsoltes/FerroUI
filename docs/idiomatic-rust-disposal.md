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

## 3. Steps

Each step is one pull request from main, with the callers it moves.

| Step | Content | Public API | Risk |
|---|---|---|---|
| D1 | The guard (section 2), with tests: drop disposes once, `detach` keeps, `dispose` after `detach` of a clone, `From` | addition | none |
| D2 | Resources: the types of the first kind release in `Drop` (the 36 that implement both first, then the rest); `dispose()` stays as an early, idempotent release. Owners that called `dispose()` in their own teardown hold the value and let it drop | behaviour: a resource is released when its last owner drops it, where it leaked before | a resource that was dropped while still in use (a clone kept elsewhere is not affected; a value moved out and dropped is) |
| D3 | Internal subscriptions of the framework: fields that hold `Option<Rc<dyn IDisposable>>` to dispose in teardown become `Option<Subscription>`, and the teardown becomes a drop | unchanged | low: the field's owner already disposed at the same point |
| D4 | Public subscription APIs of the framework, one area per pull request (events and handlers, property observation, bindings, input, the dispatcher's timers): they return `Subscription`; every caller in the workspace either keeps the guard or calls `detach()`, which keeps today's behaviour. The old signature stays one release as a deprecated `..._handle` variant where an application may depend on it | 0.5 | medium: each caller is visited (the compiler lists them) |
| D5 | The reactive libraries: `Subscribe` keeps returning `Rc<dyn IDisposable>` (the Rx contract); an extension `subscribe_guard` returns `Subscription` for RAII callers, and the framework's own uses of the libraries move to it where they kept the handle to dispose | addition | none |
| D6 | Samples, templates and the application repositories (`detach()` where they subscribe for the life of the source); the inventory baseline and the documentation | | |

Status: D1, D2 (the locator scope, `DispatcherProcessingDisabled`, `BatchUpdateOperation`) and D3 (245 fields in `ferroui-base`, `ferroui-controls` and the portable crates; the platform crates and the reactive libraries are open) are merged. D4 began with the element events of `Visual`, `StyledElement` and `Layoutable`, and has done the routed and input events (the class handlers and route observables of `RoutedEvent`, `Interactive::add_disposable_handler`, `InputScope::changed`, `KeyboardDevice::property_changed` and `TransformTrackingHelper`); the other areas (bindings and property observation, the dispatcher's timers, composition) are open.

D1 and D3 change no behaviour. D2 is the only step that changes what dropping a value does
without a compiler error, so it moves one area at a time and each pull request names the types and
how their owners hold them. D4 visits every caller of the APIs it changes; the default for a
caller that dropped the handle is `detach()`, so a converted caller behaves as before unless it is
changed on purpose.

## 4. Not in scope

- The names `IDisposable`, `Dispose` in XAML-visible or .NET-named APIs (they stay).
- `CancellationToken` and its registrations (phase C4).
- The column disposal of TreeDataGrid (`IDisposableColumn`): an explicit release by the
  presentation, which is the upstream contract.
