# Microsoft.Extensions.DependencyInjection

Issue [#251](https://github.com/wieslawsoltes/FerroUI/issues/251). This is independent of the
reactive foundation and UI. One reusable crate, `ferroui-dependency-injection`, contains the
abstractions, provider, activation helpers and an optional `splat` adapter. Its only required
workspace dependency is `ferroui-system` for disposal and local tasks.

## Reference and tracking

Work began 2026-10-22. GitHub's latest stable `dotnet/runtime` release was **v10.0.13**,
published 2026-10-13, commit `6aa72daa24f66c01c3d7b6a32519e9d21396deec`.
The two DI packages and their unit/specification tests were read at that commit.
The optional adapter follows `reactiveui/splat` commit `39466de3667c460fbcd6361d88258780aeb41a33`.
Licenses and paths are in the crate's `NOTICE.md`.

[Generated tracking](tracking/dependency-injection/TRACKING.md) maps the upstream packages to
Rust files. `scripts/port-status/references.toml` names the `dependency-injection` reference.
Set `DI_UPSTREAM` to a runtime checkout at the pinned commit to regenerate; a check only needs
the committed API inventory. The test-gap reports inventory the full upstream suites, including
reflection, open generics, CLR diagnostics and compiler-engine tests. All applicable cases are
covered: the 107 remaining unit cases now have separate Rust tests under
`tests/FerroUI.DependencyInjection.Tests/tests/upstream`. CLR-only exclusions retain their
existing explicit waivers. Tests of internal call sites assert descriptor metadata and observable
cache identity; generic constraints use closed Rust instantiations, and abstract implementations
and null explicit arguments are checked with compile-failure tests.

## Registration and resolution

```rust
use ferroui_dependency_injection::*;
use std::rc::Rc;

trait Clock { fn hour(&self) -> u8; }
struct FixedClock;
impl Clock for FixedClock { fn hour(&self) -> u8 { 12 } }
struct Greeting { clock: Rc<dyn Clock> }

let mut services = ServiceCollection::new();
services.add_singleton::<dyn Clock, _>(|_| Ok(Rc::new(FixedClock)));
services.add(ServiceDescriptor::from_factory(
    ServiceLifetime::Scoped,
    service_factory!(Greeting, |clock: required(dyn Clock)| Rc::new(Greeting { clock })),
));
let provider = services.build_service_provider(ServiceProviderOptions {
    validate_scopes: true,
    validate_on_build: true,
})?;
let scope = provider.create_scope()?;
let greeting = scope.service_provider().get_required_service::<Greeting>()?;
assert_eq!(greeting.clock.hour(), 12);
scope.dispose()?;
provider.dispose()?;
# Ok::<(), ServiceError>(())
```

- Service identities are `TypeId`s of sized types or `dyn Trait` types. `Rc<S>` is stored
  inside `Rc<dyn Any>` to preserve trait-object metadata safely. There is no `unsafe` code.
- A provider snapshots an ordered collection. Last registration wins a single lookup; all
  registrations are returned in order by `get_services`. Each descriptor has its own cache slot.
- Singleton caches belong to the root. Singleton factories receive the root provider, even
  when first requested from a child scope. Scoped caches belong to each scope; scopes are flat.
  Transients are created on every request. Root scoped resolution is allowed when validation
  is off, and retained until root disposal, matching upstream.
- `TryAdd`, `TryAddEnumerable`, replacement and removal operate on descriptors. The enumerable
  form requires `with_implementation::<I>()` to distinguish implementations without reflection,
  and rejects the service type itself or `dyn Any` as indistinguishable implementation metadata.
  `add_sequence` appends descriptors in order.
- `ServiceDescriptor::enumerable_instance::<S>(values)` registers an explicit `Vec<Rc<S>>`
  that overrides synthesized `get_services::<S>()` results, including an empty collection.
  It also resolves directly as `Vec<Rc<S>>`; `.with_key(...)` selects a keyed override.
- `IServiceProvider`, `IKeyedServiceProvider`, `IServiceScopeFactory`, and the two service-probe
  interfaces are built in. Probes do not instantiate services. The scope factory is rooted.
  Alternate providers expose `required_service_provider()` to opt into `ISupportRequiredService`;
  both typed and erased required-service resolution honor that capability.
- `DefaultServiceProviderFactory` implements `IServiceProviderFactory<ServiceCollection>`.
  Collections can be made read-only explicitly; building does not freeze the source collection.

## Constructor injection and validation

Factories return `ServiceResult<Rc<S>>`. `service_factory!` produces `ObjectFactory<S>` plus
metadata from its parameter list. Supported parameters:

| Form | Value given to constructor |
|---|---|
| `required(T)` | `Rc<T>` or missing-service error |
| `optional(T)` | `Option<Rc<T>>`; apply a default in the constructor body |
| `all(T)` | `Vec<Rc<T>>` in registration order, possibly empty |
| `keyed(T, key)` | service under an explicit value key; the key expression runs once |
| `inherited(T)` | service under the current registration's requested key (.NET 10 inheritance) |
| `key()` | current `Option<ServiceKey>`, the equivalent of `ServiceKeyAttribute` |
| `typed_key(T)` | current key cloned as `T`, or `TypeMismatch` at activation; `T: Clone` |

The body returns `Rc<S>`; it can capture explicit constructor arguments. `ObjectFactory::new`
accepts a fallible constructor and a dependency list for more elaborate signatures, including
optional/enumerable keyed parameters. `ActivatorUtilities` creates caller-owned instances using
the same factory mechanism with any `IServiceProvider`. `get_service_or_create_instance` first tries the provider.

`ValidateOnBuild` walks declared dependencies without invoking factories and aggregates errors
for every descriptor, including shadowed registrations. `ValidateScopes` rejects scoped
services from the root and scoped dependencies captured by singletons, including through
transients and enumerations. Circular errors include the dependency path. Opaque closures are
not inspected at build time; runtime resolution still detects cycles and invalid scope use.
Factory errors and unwinding panics do not poison the resolution stack or caches.

## Keys

`None` means unkeyed. `ServiceKey::from("name")` normalizes string keys to owned strings;
`ServiceKey::new(value)` supports arbitrary `Eq + Hash + Debug + 'static` keys. Equality includes
the key's Rust type. Empty string and no key are distinct.

`ServiceKey::ANY` is a catch-all registration for single keyed resolution. Exact registrations
win. A catch-all singleton has a separate cache slot for each requested key, and its factory
receives that key. An AnyKey enumeration returns explicitly keyed registrations in registration
order, excluding catch-all registrations; these share identity with exact-key lookup. AnyKey
single resolution is an error. Enumerations for an unknown concrete key are empty, even when a
catch-all exists, matching upstream's enumerable call sites.

## Ownership, threads and disposal

The provider, scopes and services are **per thread** (`Rc`, neither `Send` nor `Sync`). No global
provider or thread pool is introduced; there is no Arc variant. This matches the FerroUI object
model and Splat/ReactiveUI. Independent threads can build independent providers.

Services registered as external instances remain caller-owned. Factory-created disposables
must opt in with `with_disposal::<S>()`, `with_async_disposal::<S>()`, or `on_dispose` /
`on_dispose_async` callbacks. This avoids relying on trait reflection or dropping the last Rc.
Disposal callbacks run even if clients still hold service handles. All owned disposables,
including transients, are captured in creation order and disposed in reverse order. A scope
owns its scoped/transient services; the root owns singletons and root-created services.
Root disposal invalidates child resolution; child scopes still own their disposal lists and
must be disposed separately.

`AsyncServiceScope::new(Rc<dyn IServiceScope>)` wraps a custom scope and preserves its provider.
Sync disposal delegates to the wrapped scope; async disposal uses its `async_disposable()`
capability when present, otherwise falls back to sync disposal. Built-in scopes expose both modes.
Both providers and `IServiceScopeFactory` can create async scopes.

`dispose_async()` returns `ferroui_system::threading::tasks::Task<()>`, immediately starts local
execution, awaits each async service before proceeding, and prefers async when both modes
exist. Failures fault the task. Synchronous disposal prefers the sync callback. For an async-only
service it returns `AsyncDisposalRequired` **before changing state**, allowing retry through
`dispose_async`. This is an intentional improvement over upstream's partially disposed error
state: synchronously blocking on tasks would deadlock the local executor.

Dispose explicitly. Dropping handles releases Rust memory but does not call `IDisposable` or
start asynchronous disposal. Explicit disposal also releases cached provider/service reference
cycles. A factory that closes its own scope cannot publish a newly created service; any owned
resource it returns is disposed immediately (async-only cleanup is started on the local executor).

## Splat and ReactiveUI

Enable `features = ["splat"]`, then install the adapter **before** initializing ReactiveUI:

```rust
let resolver = ferroui_dependency_injection::splat::MicrosoftDependencyResolver::use_splat(services);
// ReactiveUI registration can now target this resolver:
let registrar = ferroui_reactiveui::DependencyResolverRegistrar::new(resolver.clone());
ferroui_reactiveui::IWantsToRegisterStuff::register(&ferroui_reactiveui::Registrations, &registrar);
```

The adapter implements Splat's `IDependencyResolver`. Contracts become string keys. Registration
mutations dispose the previous built provider and lazily rebuild from the collection, as upstream
does. `from_provider` / `update_provider` select immutable mode. Finish application registration
before resolving owned async-only services; synchronous mutation cannot dispose such a provider.
Null-returning Splat factories resolve to None and are omitted from enumerations, matching the
existing Rust Splat convention. Registration callbacks are unsupported by the upstream Microsoft
adapter and explicitly fail here as well. Splat's explicit constant-disposal callbacks run once
when the adapter closes. A test registers actual ReactiveUI view/activation services and verifies
identity through both APIs.

## Rust-specific exclusions

Rust has no CLR constructors, reflection `Type`, runtime generic instantiation, assembly scanning,
attributes, expression trees, IL generation, runtime JIT engine switching, or EventSource. Factories
and macros replace constructor selection and attributes; closed generic Rust types can be
registered normally, but open-generic registration is not offered. `Vec<Rc<S>>` resolution is an
explicit API instead of synthesizing reflected `IEnumerable<T>` types. No thread-pool stack-guard
fallback is implemented. Keyed/unkeyed descriptor metadata shares accessors. CLR-specific API and
test exclusions are recorded in the tracking inputs and `DEVIATIONS.md`.

Hosting, Configuration and Options are outside this issue.

## Validation

```sh
cargo test -p ferroui-dependency-injection -p ferroui-dependency-injection-tests --all-features
cargo check -p ferroui-dependency-injection --no-default-features
cargo clippy -p ferroui-dependency-injection -p ferroui-dependency-injection-tests --all-targets --all-features --no-deps -- -D warnings
cargo fmt -p ferroui-dependency-injection -p ferroui-dependency-injection-tests -- --check
PYTHON=python3.11 scripts/port-status/run.sh --reference dependency-injection --check
```
