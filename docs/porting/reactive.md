# Reactive foundation: System.Reactive, DynamicData and ReactiveUI

The plan for porting three .NET libraries to Rust:
- Rx.NET (System.Reactive and Microsoft.Reactive.Testing);
- DynamicData;
- ReactiveUI with the parts of Splat it needs.

It covers the reactive and BCL-style code that FerroUI already has: that code moves into reusable `ferroui-*` crates that do not depend on the UI. It also covers porting the libraries' unit tests, an integration of ReactiveUI with the FerroUI controls, and the canonical ReactiveUI sample. Together these are the foundation for writing MVVM applications on FerroUI.

Status: plan (2026-10-18). Nothing in this document is built yet. Section 9 lists the decisions the plan takes by default, which the owner can change.

## 1. References

All references are shallow checkouts in `/Volumes/1TB-macOS/reactive-ref/`, read-only, at the latest `main` on 2026-10-18.

| Reference | Repository | Commit | Version | Port |
|---|---|---|---|---|
| Rx.NET | `dotnet/reactive` (`Rx.NET/Source`) | `89e05b8` (2026-09-24) | 7.1.0-preview | System.Reactive, Microsoft.Reactive.Testing, Observable.Aliases |
| DynamicData | `reactivemarbles/DynamicData` | `65016e5` (2026-10-13) | 10.0-preview (stable 9.x) | DynamicData |
| ReactiveUI | `reactiveui/ReactiveUI` | `721f544` (2026-10-18) | 26.0.1 | ReactiveUI.Core, ReactiveUI.Shared in the `ReactiveUI.Reactive` flavour (section 3.1) |
| ReactiveUI.Binding | `reactiveui/ReactiveUI.Binding.SourceGenerators` | `d1e1b65` (2026-10-13) | 9.1.0 | The run-time library `ReactiveUI.Binding.Shared` in the `Reactive` flavour; the source generators become Rust macros |
| ReactiveUI.Primitives | `reactiveui/Primitives` | `18e8a85` (2026-10-04) | 9.0.0 | Only `ReactiveUI.Primitives.Reactive` and `Primitives.Platform.Reactive.Shared`: the System.Reactive-based extras the `Reactive` flavour uses |
| ReactiveUI for the reference UI framework | `reactiveui/ReactiveUI.Avalonia` | `26f23b5` (2026-10-04) | 12.1.6 | The model for the FerroUI integration crate, and the second sample |
| Splat | `reactiveui/splat` | `39466de` (2026-10-05) | 21.0.0 | The service locator, logging, mode detection and app builder that ReactiveUI uses |
| Samples | `reactiveui/ReactiveUI.Samples` (archived) | `8e7ca27` | — | The getting-started `ReactiveDemo` |

The reference for the UI-framework integration is a third-party repository built on the upstream of FerroUI. As everywhere in this repository, its name is not written in code (`docs/porting/PORTING-GUIDE.md`).

## 2. What exists today

### 2.1 Reactive code

`src/FerroUI.Base/reactive/` has 18 files and 2,260 lines. It is a port of the reactive helpers of the upstream base library:
- `IObserver<T>`, `IObservable<T>`, and `ObservableError = Rc<dyn Error>`;
- `IDisposable`, `Disposable`, `SerialDisposable`, `CompositeDisposable` and `DisposableMixin`;
- `AnonymousObserver`;
- `LightweightSubject` with its observable bases;
- `CombinedSubject` and `WeakObserverSubscription`;
- seven operators in `ObservableExt`: `subscribe_fn`, `select`, `where_`, `start_with`, `combine_latest`, `skip` and `take`;
- an `operators/` folder with a Rx-style `Sink` and the `CombineLatest` and `Switch` sinks.

The whole module is single-threaded and reference counted, has no schedulers, and is used everywhere. `dyn IDisposable` appears in 501 files and `IObservable<` in 109, across about 45 crates. It depends on nothing in Base except `ferro_thread_local!`.

Test stand-ins for Rx subjects exist in five places:
- `src/FerroUI.Base/tests/mod.rs`;
- `tests/FerroUI.LeakTests/subject.rs`;
- the XAML unit tests;
- the TreeDataGrid test support;
- the Appium infrastructure.

The real subjects replace them.

### 2.2 BCL-style code that is not UI-specific

These types live in `ferroui-base` today, but nothing about them is specific to the UI:

| Area | Types | Size |
|---|---|---|
| Events | `HandlerList`, `Event<A>`, `EventArgs`, `CancelEventArgs` | about 210 lines |
| Change notification | `INotifyPropertyChanged`, `INotifyCollectionChanged` with `CollectionChange`, `NotifyCollectionChangedEventArgs`/`Action` | about 140 lines plus the arguments in `ferro_list.rs` |
| Weak events | `WeakEventHandlerManager`, `WeakHashList`, `IWeakEventSubscriber`, and `WeakEvent` (which needs the dispatcher for compaction) | about 1,350 lines |
| Time | `TimeSpan`, `DateTime`, `DateTimeOffset`, `TimeZoneInfo`, `DayOfWeek`, `DateTimeKind`, `DateTimeStyles`, and date formatting with its data | about 7,900 lines |
| Globalization | `CultureInfo`, `NumberFormatInfo`, `DateTimeFormatInfo`, `CompareInfo`, `TextInfo`, `GregorianCalendar`, `ICultureDataProvider`, number formatting, `NumberStyles` | about 3,900 lines |
| Numbers | `Decimal` (over `rust_decimal`) | about 1,600 lines |
| Threading | `CancellationTokenSource`/`CancellationToken`, `ThreadSafeObjectPool` | about 260 lines |
| Other | `Uri`, `ObjectPool`, `ThreadBound`, `SynchronousCompletionAsyncResult`, `ferro_thread_local!` | about 1,500 lines |

Three things couple these types to the rest of Base:
- **The thread-local macro.** Almost all of them use `ferro_thread_local!`.
- **The service locator.** `CultureInfo`, `CompareInfo`, `DateTime` and number formatting find their data and the current time through `FerroLocator` and `IRuntimePlatform`.
- **Error and value types.** `TimeSpan` and the formatters take `FormatError` from `data/converters/composite_format.rs`, and `Decimal` registers itself with the markup through `ferro_markup_type!`.

None of this is hard to cut. Each coupling becomes a small seam in the new crate that Base fills in.

Two rules came out of the first move (phase 1a):

- **Registered markup types.** A type that is registered with the markup (`markup_types/`) cannot leave Base yet. `ferro_markup_type!` writes inherent associated functions into the registered type, which the code generated by the markup compiler calls, and Rust allows inherent `impl` blocks only in the crate that defines the type. So `EventArgs`, `CancelEventArgs`, `TimeSpan`, `Uri`, `DateTime`, `DateTimeOffset`, `CultureInfo`, `Decimal` and `NumberStyles` wait for a form of the macro, and of the emitter, for types of other crates. That form is the first task of phase 1b.
- **Upstream utilities stay.** Only the .NET API moves. Utilities that are UI-independent but port types of the upstream framework itself (`ObjectPool`, `ThreadSafeObjectPool`, `WeakHashList`, `SynchronousCompletionAsyncResult`, `ThreadBound`) stay in Base, where the tracking of the upstream port maps them. The exception is `HandlerList`, because `Event` is built on it. The upstream file it replaces (`SafeEnumerableHashSet`) is tracked as replaced by it.

These stay in Base because they belong to the UI framework:
- `FerroList`/`FerroDictionary`, the upstream framework's own collections;
- `ICommand`, `IValueConverter` and the binding layer;
- the composite formatter over boxed binding values;
- the markup registrations in `markup_types/`;
- `FerroLocator`, the upstream framework's locator;
- logging areas that name UI subsystems;
- `Dispatcher`, `DispatcherTimer` and `FerroSynchronizationContext`.

### 2.3 MVVM code

Three pieces of MVVM code exist today:
- **`samples/MiniMvvm`**, a sample library of the upstream framework that stays a sample: `ViewModelBase`, `MiniCommand`, and a string-based `when_any_value` built on `combine_latest`.
- **Six copies of `raise_and_set_if_changed`** across the dialogs, the TreeDataGrid core and the samples.
- **The binding engine**, which already binds to observables: the `^` stream operator and `ObservableStreamPlugin`, plus `FerroObject::bind` with `Rc<dyn IObservable<T>>`.

The TreeDataGrid demo's reference uses `ReactiveObject`, and the port uses a plain base class instead.

## 3. Findings that shape the plan

### 3.1 ReactiveUI comes in two flavours; we port the System.Reactive one

ReactiveUI 26's default packages run on ReactiveUI's own Rx library, ReactiveUI.Primitives (`RxVoid`, `ISequencer`, `Signal<T>`). The `*.Reactive` packages build the same shared sources against System.Reactive. `src/ReactiveShim.props` defines `REACTIVE_SHIM` and rebinds the names:

| Name | Default flavour | `Reactive` flavour |
|---|---|---|
| `RxVoid` | ReactiveUI.Primitives type | `Unit` |
| `ISequencer` | ReactiveUI.Primitives type | `IScheduler` |
| `TaskPoolSequencer` | ReactiveUI.Primitives type | `TaskPoolScheduler` |
| Activation disposables | `MultipleDisposable` | `CompositeDisposable` |

The extra operators that both flavours use come from `ReactiveUI.Primitives.Reactive`, about 1,000 lines of System.Reactive extensions. As the owner asked, the port is the `Reactive` flavour: `ReactiveUI.Core` + `ReactiveUI.Shared` compiled with `REACTIVE_SHIM`, on top of the System.Reactive port, plus the `Reactive` parts of Primitives. ReactiveUI.Primitives itself, the default engine, is not ported.

### 3.2 Much of the ReactiveUI API now lives in ReactiveUI.Binding

These all moved out of the ReactiveUI repository into ReactiveUI.Binding:
- the property observers: `WhenAny`, `WhenAnyValue` and `WhenAnyObservable`;
- `ToProperty` and `ObservableAsPropertyHelper`;
- the bindings: `Bind`, `OneWayBind`, `BindTo`, `BindCommand` and `BindInteraction`;
- `Interaction<TIn, TOut>`;
- the view-side interfaces `IViewFor` and `IActivatableView`;
- view location: `IViewLocator` and `DefaultViewLocator`;
- the binding extension points `ICreatesObservableForProperty`, `IPropertyBindingHook` and `IBindingTypeConverter`, with about 60 converters.

The run-time library of that repository has 250 files and 39,664 lines. A C# source generator reads each lambda at build time and writes direct property reads and subscriptions. Fallbacks marked `Unsafe` use expression trees and reflection. The port takes the run-time library. In place of the generator, it uses Rust macros that expand a property path into a getter and a property name at compile time, which is the same idea. There is no reflection fallback (section 5.5).

### 3.3 ReactiveUI no longer uses DynamicData; DynamicData is ported on its own

ReactiveUI 26 replaced DynamicData with an in-repository change-set layer (`ReactiveUI.Core/ChangeSets`, `ChangeSetBinder`). DynamicData is still the standard library for reactive collections in .NET MVVM applications. The owner asked for it, so it is ported as its own crate that depends only on the Rx port. The ReactiveUI port keeps ReactiveUI's own small change-set layer as upstream has it.

### 3.4 The canonical sample does not use DynamicData

The sample the ReactiveUI documentation has always used is `ReactiveDemo` (`ReactiveUI.Samples/wpf/getting-started/ReactiveDemo`, 9 files, 329 lines). It is a NuGet package search that uses:
- `WhenAnyValue` with `Throttle(800 ms)`, `DistinctUntilChanged`, `Where`, an asynchronous `SelectMany` and `ObserveOn(main thread)`;
- `ToProperty`, `ReactiveCommand` and `WhenActivated`;
- `Bind`, `OneWayBind` with a converter, and `BindCommand`;
- `ReactiveWindow`/`ReactiveUserControl`, and view location for the items of a list.

It calls the NuGet search API, and no current sample of the ReactiveUI repositories uses DynamicData. So the plan ports two samples and gives DynamicData a demo of its own (section 7).

## 4. Crates

Every new crate is published, and none of them depends on the UI except the integration crate. The names keep the `ferroui-` prefix, as the owner asked, and the API keeps the .NET names in Rust case: `Select` becomes `select`, `WhenAnyValue` becomes `when_any_value`, `SourceCache` stays `SourceCache`.

| Crate | Directory | Counterpart | Contents | Depends on | Group |
|---|---|---|---|---|---|
| `ferroui-system` | `src/FerroUI.System` | The parts of the .NET base class library that FerroUI ports | The code of section 2.2 that moves out of Base, plus what the libraries need from the BCL: `ObservableCollection<T>`, `ReadOnlyObservableCollection<T>`, `PropertyChangingEventArgs`/`INotifyPropertyChanging`, `Lazy`, `IComparer`/`IEqualityComparer` as traits and closures, `TimeProvider`/stopwatch seams | `time`, `rust_decimal`, `rustc-hash` | core |
| `ferroui-reactive` | `src/FerroUI.Reactive` | System.Reactive, System.Reactive.Observable.Aliases | Disposables, schedulers, subjects, the operators of `Observable`, `Notification`, `Timestamped`, `TimeInterval`, `EventPattern`; the existing `reactive` module moves here | `ferroui-system` | core |
| `ferroui-reactive-testing` | `src/FerroUI.Reactive.Testing` | Microsoft.Reactive.Testing | `TestScheduler`, `ReactiveTest`, `ReactiveAssert`, hot and cold observables, `Recorded`, `Subscription` | `ferroui-reactive` | core |
| `ferroui-dynamic-data` | `src/FerroUI.DynamicData` | DynamicData | Change sets, `SourceCache`, `SourceList`, change-aware collections, cache and list operators, binding to observable collections, aggregation, and the test helpers that DynamicData ships in its library (`ChangeSetAggregator`) | `ferroui-reactive` | core |
| `ferroui-splat` | `src/FerroUI.Splat` | Splat (Splat.Core, Splat.Logging, Splat.Builder, mode detection) | `AppLocator` and the resolvers, logging (`IEnableLogger`, `IFullLogger`, `LogHost`), `ModeDetector`, `AppBuilder`, `IModule` | `ferroui-system` | core |
| `ferroui-reactiveui` | `src/FerroUI.ReactiveUI` | ReactiveUI.Core + ReactiveUI.Shared (`Reactive` flavour), ReactiveUI.Primitives.Reactive extras, ReactiveUI.Binding (run time) | `ReactiveObject`, `ReactiveRecord`, `ReactiveProperty`, `ReactiveCommand`, activation, routing, message bus, suspension, the builder, `RxSchedulers`; `WhenAny*`, `ObservableAsPropertyHelper`, bindings, interactions, `IViewFor`, view location, converters | `ferroui-reactive`, `ferroui-splat` | core |
| `ferroui-reactiveui-testing` | `src/FerroUI.ReactiveUI.Testing` | ReactiveUI.Testing (`Reactive` flavour) | `TestSequencer` equivalents on `TestScheduler`, `RxTest` helpers, the app-builder test base | `ferroui-reactiveui`, `ferroui-reactive-testing` | core |
| `ferroui-reactiveui-controls` | `src/FerroUI.ReactiveUI.Controls` | The integration of ReactiveUI with the reference UI framework, with the matching parts of ReactiveUI.Binding and Primitives | The dispatcher scheduler, the activation fetcher, `ReactiveWindow`, `ReactiveUserControl`, `RoutedViewHost`, `ViewModelViewHost`, the data-template binding hook, property observation and command binding for FerroUI objects, `AutoSuspendHelper`, `use_reactive_ui` on the app builder | `ferroui-reactiveui`, `ferroui-controls` | controls |

Two kinds of crate are not planned:
- **A procedural-macro crate.** Section 5.5 explains why `macro_rules!` is enough to begin with.
- **Ports of the frameworks that are not FerroUI.** These include WPF, WinForms, MAUI, WinUI, Blazor, AndroidX, Uno and the dependency-injection adapters of Splat.

Base keeps its public paths: `ferroui_base::reactive`, `ferroui_base::utilities::{TimeSpan, CultureInfo, ...}` and the rest become re-exports of the new crates. So no crate of the workspace and no application has to change its imports. 0.3 may change the public API, but this move does not need to.

## 5. Design decisions

### 5.1 Threading: single-threaded observables, with entries from other threads

FerroUI's object model is `Rc`/`RefCell` and thread-bound, and the existing `IObservable` is single-threaded. The port keeps this model.

**Observables and observers are not `Send`.** The schedulers that need no thread port as they are:
- `ImmediateScheduler`;
- `CurrentThreadScheduler`, which is the trampoline that makes recursion and re-entrancy safe without locks (a thread-local queue);
- `VirtualTimeScheduler`, `HistoricalScheduler` and `TestScheduler`.

**The main-thread scheduler.** `DispatcherScheduler` is built on FerroUI's `Dispatcher` and `DispatcherTimer`, and plays the role of `RxSchedulers.MainThreadScheduler`. It lives in `ferroui-reactiveui-controls`, and its scheduling seam lives in `ferroui-reactive`. A generic `SynchronizationContextScheduler` on `FerroSynchronizationContext` sits behind the same seam.

**Work on other threads.** The thread-pool schedulers (`ThreadPoolScheduler`, `TaskPoolScheduler`, `NewThreadScheduler`, `EventLoopScheduler`) cannot run single-threaded observables on other threads. Background work gets two forms instead:
- **Futures.** `Observable::from_async`, `start_async` and `to_future` run a future and deliver its result on the thread that subscribed. This is what an asynchronous `SelectMany` or `ReactiveCommand::create_from_task` needs.
- **`TaskPoolScheduler`.** It runs a `Send` closure on a worker thread and posts its result back to the scheduler of the subscriber. This is the `RxSchedulers.TaskpoolScheduler` role.

This is recorded as a deviation. The tests that need real concurrency (about 11 % of the Rx tests and 5.5 % of the DynamicData tests) are ported where they keep their meaning on one thread, and waived with a reason where they test locks.

**The browser target is single-threaded.** Everything above except the worker thread works there unchanged.

### 5.2 Errors

The error type stays `ObservableError = Rc<dyn Error>`, which `Replay`, `Catch` and `Retry` can clone. Each place where .NET would let an exception from a user selector escape into `OnError` is handled in one of two ways:
- **A fallible variant.** `select` takes `Fn(T) -> U`, and a `try_select` takes `Fn(T) -> Result<U, ObservableError>`. This is the form the operators use when an error has to reach `OnError`.
- **A panic.** A panic propagates as a panic, which is Rust's form of an unhandled exception.

The rules of Rx's `ExceptionHelper` are recorded: an `OnError` that has no handler panics.

### 5.3 Shape of the API

**`Observable<T>`.** The handle is `Rc<dyn IObservable<T>>`, wrapped in a cloneable newtype `Observable<T>`. The operators are inherent methods with Rx's names in snake case (`select`, `select_many`, `where_`, `distinct_until_changed`, `throttle`, `observe_on`, `to_property`). `ObservableExt` and the existing calls keep working.

**Overloads.** The 654 overloads of `Observable` reduce to fewer methods:
- **Generic methods.** The numeric families (`sum`, `min`, `max`, `average`) become one generic method each.
- **Macros.** The arities 2 to 16 of `combine_latest`, `zip` and `when` come from macros, as the T4 templates do upstream. `combine_latest` has 18 overloads and `zip` has 19.
- **Separate names.** Each overload with an index or a scheduler becomes its own method (`select_with_index`, `throttle_on`).

The upstream checklist of the surface is `System.Reactive.MakeRefAssemblies/.PublicApi/PublicAPI.Shipped.txt`, which has 2,062 lines. Every entry is ported, waived (blocking, Qbservable, Remoting, reflection-based events, ...) or merged into another, as in `docs/porting/data/member-waivers.toml`.

**Out of scope.** The parts with no meaning in this model are recorded as waived in that checklist:
- IQbservable and the expression-tree query providers (about 28,000 lines);
- `FromEventPattern` by event name (reflection);
- the blocking operators, which would deadlock the only thread: `Wait`, `First`, `Last`, `Single`, `ForEach`, `GetEnumerator`, `Latest`, `Next`, `ToEnumerable` (the pull adapters that never wait, `Collect`, `Chunkify` and `MostRecent`, are ported);
- Remoting.

The `*Async` forms of the terminal operators port as futures.

### 5.4 DynamicData's shape

**Keys.** `TKey : notnull` becomes `K: Eq + Hash + Clone`.

**Equality and ordering.** Each comparison becomes a trait or a closure:
- `IEqualityComparer<T>` becomes a trait with a default based on `PartialEq`, plus overloads that take a closure.
- `IComparer<T>` becomes `Rc<dyn Fn(&T, &T) -> Ordering>`.
- `SortExpressionComparer` becomes a builder of comparers over getter closures.

**`Optional<T>`.** It becomes `Option<T>`.

**Change sets.** `IChangeSet` and its variants (sorted, paged, virtual, grouped, contextual) become concrete structs with traits where upstream branches on the interface.

**Locks.** DynamicData's `DeliveryQueue`/`SynchronizeSafe` serialize edits under a lock and deliver notifications outside it. Without threads, this becomes a re-entrancy-safe queue: an edit made during a notification is queued and delivered after the current one. That keeps the order the upstream tests check.

**Binding targets.** `ObservableCollectionExtended` and the adaptors bind to `ferroui-system`'s `ObservableCollection<T>`, which raises the same `INotifyCollectionChanged` that FerroUI's items controls already read. A `BindingList` target is not needed (WinForms).

### 5.5 Property paths without reflection: macros

`WhenAnyValue(x => x.Search.Text)`, `ToProperty(this, x => x.Results)`, `Bind(vm, x => x.Name, v => v.NameBox.Text)` and DynamicData's `WhenValueChanged`, `AutoRefresh` and `SortExpressionComparer` all take expressions upstream. In the port, each takes a path macro that expands at compile time into a getter closure and the property names along the path:

```rust
let results = vm
    .when_any_value(prop!(AppViewModel => search_term))
    .throttle(Duration::from_millis(800), RxSchedulers::task_pool_scheduler())
    .select(|term| term.trim().to_owned())
    .distinct_until_changed()
    .where_(|term| !term.is_empty())
    .select_many(move |term| search_nuget_packages(term))
    .observe_on(RxSchedulers::main_thread_scheduler())
    .to_property(&vm, prop!(AppViewModel => search_results));
```

**What `prop!` does.** It produces a typed `PropertyPath<Source, Value>`, holding the getter and the name of each step. `ICreatesObservableForProperty` subscribes to each step:
- through `INotifyPropertyChanged` for a model;
- through the property system for a FerroUI object, using the registered property of that name.

A `#[reactive]`-style macro for the properties of a view model, the counterpart of ReactiveUI.SourceGenerators' `[Reactive]`, writes the field, the getter and a setter that calls `raise_and_set_if_changed`.

**Why not a procedural macro yet.** `macro_rules!` covers both macros, so the plan starts without a procedural-macro crate. If the ergonomics need one, it is added later.

**Tests.** The 1,233 tests of the source generators test generated C#, so they are replaced by tests of the macros. The 203 tests of the generated code port as tests of what the macros produce.

### 5.6 Time

Each time type maps to a Rust or `ferroui-system` type:
- `TimeSpan` becomes `std::time::Duration` in the schedulers and operators. `ferroui-system`'s `TimeSpan` converts from and to it and stays where FerroUI uses it.
- `DateTimeOffset` becomes the scheduler's `now()`, with `ferroui-system`'s `DateTimeOffset`.
- The stopwatch and the clock are a `TimeProvider`-style seam, as in .NET 8, so that virtual time and real time share the code of the operators.

### 5.7 Tests

**Mapping.** Each test of the references becomes a Rust `#[test]` in the crate's `tests/` folder, one file per upstream test file, under the same name in snake case.

**Test frameworks.** MSTest (Rx), xUnit with AwesomeAssertions (DynamicData) and TUnit (ReactiveUI, ReactiveUI.Binding, Splat, the integration) all become plain `#[test]` with `assert_eq!`. DynamicData's `.Should()` assertions become a small assertion helper in the test support of `ferroui-dynamic-data`.

**Random test data.** DynamicData's tests draw it from Bogus. The port uses a seeded generator in the test support that produces the same kinds of people and data. The tests check properties of the result rather than particular values, so they keep their meaning.

**Accounting.** Each test is ported or waived with a reason, as `docs/porting/TESTS.md` describes. The common reasons are:
- argument-null validation, which Rust's types make impossible (about 350 Rx tests);
- real-thread concurrency (section 5.1);
- Qbservable and reflection;
- platforms other than FerroUI.

Tracking pages per reference are generated like those of the upstream port (section 8).

## 6. Phases

Each phase is one or more pull requests that build, pass the existing tests and the ported ones, and are merged when CI is green. Sub-agents write and commit; the builds and the validation run one branch at a time in the main checkout, as the other ports do. The sizes are the upstream sizes the phase ports.

| # | Phase | Ports | Upstream tests | Effort |
|---|---|---|---|---|
| 1 | `ferroui-system` | 1a: the thread-local macro, events, property and collection change notification, cancellation. 1b: the markup registration of types of other crates, then time, globalization, `Decimal`, `Uri` and the event arguments. Move section 2.2 out of Base. Seams for the culture data, the clock and `FormatError`. Re-exports in Base. Update the thread-local check script, the port-status path mappings and the markup registrations (`Decimal` and `NumberStyles` move their inline registrations to `markup_types`). | The existing Base tests of those types move with them | 1 week |
| 2 | Rx core | Disposables (1,800 lines), `IObserver`/`IObservable`, the Producer/Sink pattern, `SafeObserver`/`AutoDetachObserver`, `Notification`, `Unit`, subjects (2,500 lines: `Subject`, `BehaviorSubject`, `ReplaySubject`, `AsyncSubject`, `ConnectableObservable`). Schedulers: Immediate, CurrentThread, VirtualTime, Historical. `ferroui-reactive-testing`. The existing `reactive` module moves in. | Disposables 58, subjects 105, concurrency (single-threaded part of 216), internal 14, root 263, Testing | 1.5 weeks |
| 3 | Rx operators | The 113 operator implementations of `Linq/Observable` (24,000 lines) by family: creation, single-sequence, multiple, time, aggregates, multicast, joins, imperative. Observable.Aliases. | About 3,050 operator tests (`Tests/Linq/Observable`, one file per operator), most of them on `TestScheduler` | 3–4 weeks |
| 4 | Rx async and threads | `from_async`, `start_async`, `to_future`, `for_each_async`, the scheduler seam for the dispatcher and the synchronization context, `TaskPoolScheduler` (section 5.1). | The async operator tests that keep their meaning | 1 week |
| 5 | DynamicData | Kernel, change sets, `SourceCache`/`SourceList`, `ChangeAwareCache`/`List`, then the cache operators (314 overloads, 108 names), the list operators (124, 63), binding, aggregation and the test helpers. Upstream's partial files map one-to-one to modules. | About 1,958 (Cache 969, List 557, Binding 157, Internal 109, Aggregation 96, Kernel 54) | 4–5 weeks |
| 6 | Splat | `AppLocator` with the resolvers, logging, `ModeDetector`, `AppBuilder`, `IModule`. The generated typed overloads of logging (about 5,000 lines) shrink to generic methods. | The tests of those parts: a subset of Splat.Tests (574), Splat.Logging.Tests (471) and Splat.Builder.Tests (18) | 1 week |
| 7 | ReactiveUI | `ReactiveObject`, `ReactiveRecord`, `ReactiveProperty`, `ReactiveCommand` (2,800 lines), activation, routing, message bus, suspension and auto-persist without reflection, the builder, `RxSchedulers`, the change-set layer, ReactiveUI.Primitives.Reactive's extras. Then ReactiveUI.Binding's run time: `WhenAny*`, OAPH, bindings, interactions, view location, converters. The `prop!` and property macros. `ferroui-reactiveui-testing`. | ReactiveUI.Tests 880, ReactiveUI.Builder.Tests 184, ReactiveUI.Testing.Tests 41, ReactiveUI.Splat.Tests 6, ReactiveUI.Binding.Tests 1,855, GeneratedCode.Tests 203 (as macro tests) | 5–6 weeks |
| 8 | Integration with FerroUI | `ferroui-reactiveui-controls`: the dispatcher scheduler, the activation fetcher (`Loaded`/`Unloaded`, attached to the visual tree), `ReactiveWindow`, `ReactiveUserControl`, `RoutedViewHost`, `ViewModelViewHost`, the data-template hook, property observation and command binding for FerroUI objects, `AutoSuspendHelper`, the app-builder extension. | The integration's tests (175 + 47) and those of the binding integration (42), on FerroUI's headless platform | 1.5 weeks |
| 9 | Samples | Section 7. | The example's tests (33) | 1.5 weeks |
| 10 | Release | Documentation (a guide to MVVM on FerroUI), the README, the changelog, the crate table, `0.3.0-preview.N` with the new crates. | — | 0.5 week |

**Totals.**
- **Upstream sources.** About 140,000 lines across the references, without Qbservable and the platforms that are not FerroUI.
- **Upstream tests.** About 10,000 to port or waive.
- **Effort.** About 20–25 weeks of work at the pace of the earlier ports.

**Order.** Phases 1 to 4 come first and in order: everything else depends on them. Phase 5 (DynamicData) and phase 6 (Splat) are independent of each other and can run beside phase 7's first part. Phase 8 needs phase 7. Each phase ends with its tracking page and its entries in `DEVIATIONS.md`.

## 7. Samples

1. **ReactiveDemo** (`samples/ReactiveDemo`), the canonical getting-started application.
   - **Port.** It is ported file for file: `AppViewModel`, `NugetDetailsViewModel`, `MainWindow`, `NugetDetailsView`, the `App`. The views are in FerroUI markup, with the bindings done in code as upstream (`Bind`, `OneWayBind`, `BindCommand` inside `WhenActivated`).
   - **Search backend.** The upstream sample calls NuGet's search API through NuGet.Protocol. The port calls the same HTTP endpoint (`/v3/query`) behind the feature `online`, as the TreeDataGrid demo does for its online page. By default it uses an in-memory catalogue, as the ReactiveUI documentation's version does, so that it runs and is tested offline.
   - **Opening a project.** The "open in browser" command uses the platform's launcher.
2. **The integration's example** (`samples/ReactiveUIExample`), a port of the example of the ReactiveUI integration for the reference UI framework (35 files, about 1,765 lines, with 33 headless tests). It exercises what ReactiveDemo does not:
   - `RoutedViewHost` and `ViewModelViewHost` with view contracts;
   - interactions;
   - `AutoSuspendHelper`;
   - observing the properties of controls.

   Its tests port to FerroUI's headless platform.
3. **DynamicData.** No current ReactiveUI sample uses it. The plan adds a page to the integration's example that is an explicit addition, not a port:
   - a `SourceCache` of the example's items, filtered by a throttled search text, sorted, transformed into view models;
   - bound with `SortAndBind` to the list;
   - edited with `AddOrUpdate`/`Remove`;
   - using `AutoRefresh`.

   It follows the pattern of the one upstream sample that used DynamicData offline, `ItemsDDViewModel` of the archived WinForms demo. A larger DynamicData showcase, such as the community's "Dynamic Trader", would be a separate decision.

## 8. Tracking, rules and checks

- **Tracking pages.** The port-status tooling (`scripts/port-status/`) gets one source per reference: the Rx.NET sources, DynamicData, ReactiveUI with the `REACTIVE_SHIM` sources, ReactiveUI.Binding, the Primitives extras, and the integration. It also gets pages under `docs/porting/tracking/`, with file, type and member counts as for the upstream port. The public API checklists are the upstream files `PublicAPI.Shipped.txt` (Rx), `ApiApprovalTests.DynamicDataTests.DotNet9_0.verified.txt` (DynamicData), and the integration's `PublicAPI.txt`.
  - **Rx.NET, tracked.** The pages are in `docs/porting/tracking/reactive/`: the index `TRACKING.md` (totals, tests, waivers by reason) and one page per project, `System.Reactive.md` and `Microsoft.Reactive.Testing.md`. The data (the extraction, the path overrides, the waivers of section 5, the test waivers and the test report) is in `docs/porting/data/reactive/`. `scripts/port-status/run.sh --reference reactive` regenerates them, after `python3 scripts/port-status/test_gaps.py --reference reactive --all` when tests were ported (`scripts/port-status/README.md`).
- **Deviations.** Every difference goes to `docs/porting/DEVIATIONS.md`, in new sections per crate.
- **Rules.** The rules of the upstream port apply:
  - no stubs;
  - attribution in `NOTICE.md` for the new references, which are all MIT licensed;
  - `ferro_thread_local!` for thread-locals;
  - the upstream name of the UI framework is not written in code, even where the integration's reference repository carries it in its own name;
  - build output on the external volume.
- **CI.** The new crates join the conventions job, the workspace tests and the packaging check of the published crates without changes to the workflows. The thread-local script is updated in phase 1 to know the new crate that holds the macro.

## 9. Decisions taken by default

Each decision below is what the plan does unless the owner decides otherwise:

1. **One crate for the BCL subset** (`ferroui-system`), not one per .NET namespace. It can be split later if build times call for it.
2. **Single-threaded observables** with futures and a worker-thread scheduler for background work (section 5.1). A `Send` variant of the whole library is not planned.
3. **The `Reactive` flavour of ReactiveUI 26**, with ReactiveUI.Binding's run time folded into `ferroui-reactiveui`. They are separate packages upstream only because of the source generators.
4. **Rx's names in snake case**, plus the aliases (`map`, `filter`, `flat_map`) of Observable.Aliases.
5. **`macro_rules!` for property paths** and view-model properties, with no procedural-macro crate at first.
6. **The two samples of section 7**, plus the DynamicData page as an explicit addition.
7. **Base keeps its paths** as re-exports, so the extraction is not a breaking change.
