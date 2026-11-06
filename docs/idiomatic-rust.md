# Idiomatic Rust under the .NET-named API

FerroUI ports .NET frameworks (the upstream framework, WinUI 3 and the libraries listed in `NOTICE.md`). To keep the port close to its sources, the first releases also kept many shapes of the .NET base class library: ports of `System.*` types (`TimeSpan`, `Uri`, `Dictionary`, `Task`, `CancellationToken`), `IDisposable` handles, exception-shaped error types, and duplicates of the same type in several crates. From 0.5 the framework keeps the .NET names of its control API and makes everything that API is made of idiomatic Rust and std. This page holds the rules, the replacement of each type, the deprecation policy, the format of the map from upstream names to Rust names, and the inventory that measures the work. The plan and its phases are issue #423.

## 1. What keeps its .NET name

These names stay as upstream has them (in `snake_case` where Rust needs it, as PORTING-GUIDE.md, [section Types](porting/PORTING-GUIDE.md#types) and [section Naming](porting/PORTING-GUIDE.md#naming), describe for the porting side). They are never renamed for idiom:

- control types and the other framework types a user of the framework names in code and markup: `Button`, `TextBox`, `Grid`, `ResourceDictionary`, `Storyboard`;
- their methods, properties and events, including the `get_`/`set_` accessors that mirror a .NET property or method (`TextBox::select`, `Control::get_value`), and the `..._property()` and `..._event()` definitions;
- upstream interface names, kept as traits with their `I` prefix: `ICommand`, `IValueConverter`, `IDataTemplate`, `ILogicalScrollable`;
- `EventArgs` types (`RoutedEventArgs`, `PointerPressedEventArgs`, ...);
- `INotifyPropertyChanged`, `INotifyPropertyChanging`, `INotifyCollectionChanged`, `INotifyDataErrorInfo` and `ObservableCollection`, with their API;
- every name visible to XAML: the markup type and member names (`ferro_markup_type!`, the type names of the XAML namespaces, the WinUI and other upstream-facing names). Markup does not change. Where a Rust name changes, its markup name is written explicitly so that it stays.

A `get_` method that mirrors no .NET member is reviewed like any other helper; it is not kept because of its prefix.

## 2. What becomes idiomatic Rust and std

The types in the signatures of the API, the mechanisms of resource lifetime and errors, and the implementation underneath use std. Where std has nothing, one well-established crate is chosen once for the whole workspace, and no other crate is used for the same need.

### 2.1 Type replacements

| .NET shape (and where the port has it) | Replacement | Phase | Behaviour to keep (a test, or a DEVIATIONS row) |
|---|---|---|---|
| `TimeSpan` (`ferroui-base`, `animation/time_span.rs`: signed 100 ns ticks) | `std::time::Duration`; a signed duration newtype only where a negative span is part of the contract (animation clocks that step backwards, a negative `Delay` that is rejected when the animation runs; [idiomatic-rust-signatures.md](idiomatic-rust-signatures.md), section 3.2). Done in C5: the port type is `SignedDuration` (`TimeSpan` a deprecated alias), the timer properties and functions take `Duration`, and markup and bindings know both | C2, C5 | Tick rounding of `from_seconds`, `from_milliseconds` and the other `f64` constructors; the text format of `Parse` and `ToString` in markup |
| `DateTime`, `DateTimeOffset` (`ferroui-base`, `utilities`) | `std::time::SystemTime` at the edges, with the port types underneath; no new crate (decided in C2: the calendar arithmetic of the port types already comes from `time`, a dependency of `ferroui-base`, and neither `time` nor `jiff` becomes a public type; which signatures take `SystemTime`: [idiomatic-rust-signatures.md](idiomatic-rust-signatures.md), section 3.3). Done in C5: instants are `SystemTime`, and the calendar values keep `DateTime` and `DateTimeOffset`, deliberate declarations of the baseline (O4) | C2, C5 | The invariant text format used by markup; `SystemTime::now()` is not available on `wasm32-unknown-unknown`, so the clock stays behind the dispatcher there, and the conversions never read it |
| `CultureInfo`, `NumberFormatInfo`, `DateTimeFormatInfo` (`ferroui-base`, `utilities/culture_info.rs`) | ICU4X: `icu_locale_core::Locale`, with `icu_decimal` and `icu_datetime` for formatting (ICU4X 2.4 is in the workspace; [idiomatic-rust-signatures.md](idiomatic-rust-signatures.md), section 3.4). Done in C5: the identity of a culture is a `Locale` (L1), and `CultureInfo` stays the formatting parameter, made from and giving a `Locale`, a deliberate declaration of the baseline (O3) | C2, C5 | The culture-specific formatting the controls show (numbers, dates, the invariant culture of markup) |
| `Uri` (`ferroui-base`, `utilities/uri.rs`) | `url::Url`, or a newtype over it where relative references are part of the contract (the resource URIs of the framework, relative `Source` values in markup; [idiomatic-rust-signatures.md](idiomatic-rust-signatures.md), section 3.5). Done in C5: `url::Url` where a URI is absolute and `UriRef` where it can be relative, in markup loading, assets, fonts and the platform edges; the port `Uri` stays the canonical markup form of `System.Uri` | C2, C5 | Relative and absolute forms, the resource schemes of the framework, `ToString` |
| `Guid` (three definitions: `ferroui-splat`, `ferroui-remote-protocol`, `ferroui-microcom`) | `uuid::Uuid`, or one shared type where a layout is part of a contract (the COM `GUID` layout of `ferroui-microcom`; the public signatures: [idiomatic-rust-signatures.md](idiomatic-rust-signatures.md), section 3.6). Done in C5: the public signatures take `uuid::Uuid`, the `Guid` of Splat and of the remote protocol are deprecated, and the microcom `Guid` is a deliberate declaration of the baseline | C1, C5 | Byte order of the wire and COM layouts; the text format |
| `Dictionary<K, V>` (`ferroui-system`, `collections/generic`: the .NET slot and free-list order) | `std::collections::HashMap`; `indexmap::IndexMap` where a caller enumerates the map and depends on the order; the port of the .NET order (`SlotOrderMap` since X1 of C5) only where the exact .NET sequence after removals is observable ([idiomatic-rust-signatures.md](idiomatic-rust-signatures.md), section 3.7). Done in C5: the public signatures take `HashMap` or `IndexMap`, `Dictionary` is a deprecated alias of `SlotOrderMap`, and the BSON `Dictionary` and `DateTime` of the remote protocol keep their names (O6) | C1, C5 | Enumeration order: a caller that enumerates is checked, and the order it needs is tested |
| `Task<T>`, `TaskCompletionSource<T>`, `SynchronizationContext` (`ferroui-system`, `threading`; local copies in several crates) | Rust futures on the executor of `ferroui_system::task`, whose schedule on a UI thread is the dispatcher: `JoinHandle<T>` for a task (`Dispatcher::start_local`, `spawn_local`, `spawn`), `Shared<T>` for the tasks of the reactive libraries, `task::oneshot` instead of `TaskCompletionSource`, the `Schedule` of the thread and `Dispatcher::current_priority` instead of `SynchronizationContext` (done in C4, [idiomatic-rust-async.md](idiomatic-rust-async.md)) | C4, C5 | The dispatcher's ordering guarantees (priorities, the order of continuations); tests come first ([idiomatic-rust-async.md](idiomatic-rust-async.md), section 2) |
| `CancellationToken`, `CancellationTokenSource`, `CancellationTokenRegistration` (`ferroui-system`, `threading`) | One cancellation type, decided in C4: `ferroui_system::threading::CancellationToken` in the shape of `tokio_util` (`new`, `cancel`, `is_cancelled`, `child_token`, `cancelled`, `drop_guard`), implemented in `ferroui-system`; `register` gives a `CancellationGuard`; the error is `task::Canceled` ([idiomatic-rust-async.md](idiomatic-rust-async.md), section 3.7) | C4, C5 | Registration order and the synchronous run of callbacks on cancel |
| `IDisposable` handles (`Rc<dyn IDisposable>` with an explicit `dispose()`) | `Drop` and RAII guards: a subscription token that unsubscribes when it is dropped; an explicit `close()` only where the order of release matters | C3, C5 | The port keeps a subscription alive when its handle is ignored ([PORTING-GUIDE.md, section Types](porting/PORTING-GUIDE.md#types)). A guard changes that: it is `#[must_use]`, and keeping a subscription for the life of its source is an explicit call (for example `detach()`). Each converted API states which it does. The reactive libraries keep `subscribe` returning the handle (the Rx contract); an owner that keeps a subscription to end it later subscribes with `subscribe_guard` (the extension trait `SubscribeGuard`) or the `_guard` siblings of the closure helpers (`subscribe_fn_guard`, `subscribe_next_guard`, ...) |
| `...Exception` types | `...Error` types implementing `std::error::Error` (with `Display` and `source()`), returned in `Result` | C3, C5 | Messages that a user can observe (logs, the error text of markup loading) |
| `Nullable<T>`, `Optional<T>` | `Option<T>` (already the rule) | done | |
| `List<T>`, `IList<T>` as values | `Vec<T>`, slices and iterators (already the rule; `FerroList<T>` stays for observable and shared lists) | done | |

### 2.2 Chosen crates

| Need | Crate | Notes |
|---|---|---|
| URLs and URIs | `url` | One `Url` type at every edge; a newtype only where the framework's relative references need it. A workspace dependency, and a dependency of `ferroui-base` since C2 |
| Locales, number and date formatting, segmentation | ICU4X 2.4 (`icu_locale_core`, `icu_locale`, `icu_decimal`, `icu_datetime`, `icu_calendar`, `icu_segmenter`) | Already in the workspace (#393) |
| GUIDs and UUIDs | `uuid` | Already in `Cargo.lock`; the COM layout of `ferroui-microcom` converts to and from it |
| Ordered maps and sets | `indexmap` | Already in `Cargo.lock`; only where the insertion order is part of the behaviour |
| Hashing on hot paths | `rustc-hash` | Already a workspace dependency; std's hasher everywhere else |

A new crate for any of these needs, or a second crate for one of them, is an owner decision recorded here first.

## 3. Deprecation policy

- The .NET-named infrastructure types are **replaced, not wrapped**: the API takes and returns the std or chosen-crate type, and the implementation uses it. A wrapper that keeps the old type underneath is not a replacement.
- In 0.5 each old type remains **for one release**: as a `#[deprecated(since = "0.5.0", note = "use ...")]` type alias or re-export where the new type has the same shape, or as `From`/`Into` conversions to and from the new type where it does not. The note names the replacement.
- The old types are removed in the release after 0.5 (C6 of #423 checks that nothing in the workspace, the samples or the application repositories still uses them).
- Every change of a public signature has a migration note in `CHANGELOG.md`, under `[Unreleased]`, in a `### Migration` subsection: the old type, the new type, and the conversion.
- 0.4.x stays compatible: the API gate (`scripts/api-gate/semver_checks.py`, docs/release.md) compares with 0.4.0, and C0 and C1 change no public API. Signature changes land on main for 0.5, whose baseline moves when 0.5.0 is released.

## 4. Behaviour

Behaviour stays the same unless the change is recorded with a test. Where a .NET-shaped type encodes behaviour that the controls rely on, its replacement keeps that behaviour with a test, or the pull request adds a row to `docs/porting/DEVIATIONS.md` (kind **Behaviour**) and a test of the new behaviour. The known cases:

- the enumeration order of `Dictionary` (in order of insertion, except that an entry added after a removal takes the slot of the entry removed last);
- the tick rounding of `TimeSpan` and its negative values;
- the culture-specific formatting of `CultureInfo` and the invariant culture of markup;
- the keep-alive of an ignored `IDisposable` handle;
- the ordering of continuations on the dispatcher.

## 5. The map from upstream names to Rust names

Review against the upstream sources has to keep working when a Rust name changes. The port docs therefore keep a map from each upstream name to the Rust name that replaces it, in `docs/porting/data/upstream-names.toml`. Only names whose Rust form differs from the mechanical one (`snake_case` of the upstream name, the same type name) are listed; a control member that keeps its name is not. The file is created with the first entry, and each pull request that replaces a type adds its rows.

Each entry is a TOML table in the array `name`:

```toml
[[name]]
upstream = "System.TimeSpan"            # the upstream name: namespace and type, or type.member
rust = "std::time::Duration"            # the Rust path that replaces it
kind = "type"                           # type | trait | member | mechanism
phase = "C2"                            # the phase of #423 that made the change
pull = 0                                # the pull request
deprecated = "ferroui_base::animation::TimeSpan"   # the old Rust path, while it remains (optional)
behaviour = "test"                      # same | test | deviation: how the behaviour is kept
notes = "Negative spans of the animation clocks use ferroui_base::animation::SignedDuration."
```

- `upstream` uses the upstream spelling, including the namespace for `System.*` types, so that a search for the upstream name finds the row.
- `rust` is a path that compiles in a crate that depends on the owning crate.
- `behaviour = "deviation"` names the DEVIATIONS row in `notes`.

## 6. The inventory and its check

`scripts/idioms/inventory.py` measures the patterns of #423 exactly, per crate. It reads every `.rs` file under `src/`, `samples/` and `tests/`, blanks comments, string literals and char literals, and skips generated files (a `//` comment saying `@generated`, `Generated by` or `Do not edit` in the first lines) and build directories. `ferro_markup_type!(struct X { .. })` describes the markup type of an existing type and is not counted as a declaration.

```sh
python3 scripts/idioms/inventory.py                    # the summary over the crates of src/
python3 scripts/idioms/inventory.py --all --by-crate   # with samples/ and tests/, and per crate
python3 scripts/idioms/inventory.py --json             # everything, for tooling
python3 scripts/idioms/inventory.py --check            # the check of CI
python3 scripts/idioms/inventory.py --update-baseline  # rewrite scripts/idioms/baseline.json
```

The summary counts, per pattern, the declarations or the occurrences, the files with at least one, and the crates. The kept names (section 1) are measured too, so that the size of what stays is known. The direction of a pattern whose types are deprecated since 0.5.0 says so (`Task<`, `TaskCompletionSource`, `SynchronizationContext`, `CancellationTokenSource` and the registrations: "deprecated in 0.5, counted until removed after 0.5"), and `CancellationToken`, the one cancellation type, is counted as the target shape.

The check guards new code. A declaration (`struct`, `enum`, `union`, `trait` or `type`) is .NET-shaped infrastructure when its name ends in `Exception`, or when it is the name of a .NET base class library type that has a std or chosen-crate replacement (the list `SYSTEM_TYPES` of the script: `TimeSpan`, `Uri`, `Guid`, `Dictionary`, `Task`, `IDisposable`, `CancellationToken`, the Rx disposables, and the other ports of `System.*`). `scripts/idioms/baseline.json` holds how many such declarations each crate has. The conventions job of CI runs `--check`, which fails:

- when a crate declares such a name more often than its baseline (a new port, or a new duplicate of an existing one): use the replacement of section 2; an error type is named `...Error` and implements `std::error::Error`;
- when the baseline allows more declarations than the sources have: the baseline only shrinks, so the pull request that removes a declaration runs `--update-baseline` and commits the result.

A deliberate new declaration (a type that has to keep a .NET name because markup or a wire format names it, and that cannot be described with `ferro_markup_type!`) is added with `--update-baseline`, with the reason in the pull request.

## 7. Phases

The phases of #423, each as small pull requests from main:

| Phase | Content | Public API |
|---|---|---|
| C0 | The inventory script and its check, this page | unchanged |
| C1 | Duplicates and internal-only types: one `Guid` (or `uuid`), internal `Dictionary` uses to `HashMap`/`IndexMap`, internal single-implementor traits to concrete types | unchanged |
| C2 | std types at the edges: `From`/`Into` between `TimeSpan` and `Duration`, `Uri` and `url::Url`, `CultureInfo` and the ICU4X `Locale`; new APIs take the std or ICU types | additions only |
| C3 | Resources and errors: `IDisposable` to `Drop` and RAII guards (the steps: [idiomatic-rust-disposal.md](idiomatic-rust-disposal.md)), `...Exception` to `...Error` | 0.5 |
| C4 (done) | Async: `Task`, `TaskCompletionSource` and `SynchronizationContext` to futures on the dispatcher's executor; one cancellation type (the plan, the ordering the dispatcher guarantees and the steps: [idiomatic-rust-async.md](idiomatic-rust-async.md); the status: section 7.3) | 0.5 |
| C5 (done) | Public signatures take and return the std and chosen-crate types; the old types remain one release as deprecated aliases or conversions; migration notes (the measures, the target per type and the steps: [idiomatic-rust-signatures.md](idiomatic-rust-signatures.md); the status: section 7.4) | 0.5 |
| C6 | The check is clean, the API gate has a 0.5 baseline, the samples and application repositories are updated | |

### 7.1 C2: the conversions at the edges

C2 adds conversions to the port types of `ferroui-base` and changes no signature. Each conversion has tests next to the type.

| Port type | From the std or ICU type | To the std or ICU type | Rules |
|---|---|---|---|
| `animation::TimeSpan` | `From<Duration>` (kept) | `TryFrom<TimeSpan> for Duration`, error `NegativeTimeSpanError` | A tick is 100 ns. A `Duration` is truncated to whole ticks (149 ns is one tick) and saturates at `TimeSpan::MAX_VALUE`; a non-negative `TimeSpan` converts exactly (also `MAX_VALUE`); a negative one is an error |
| `utilities::Uri` | `From<url::Url>` | `TryFrom<&Uri> for url::Url`, error `UriToUrlError` | The `Url` text becomes the original string of an absolute `Uri`. The canonical text of an absolute `Uri` (its `ToString`) is parsed; a relative reference is an error (`Relative`), and so is a text the URL parser rejects (`Invalid`). The schemes of the framework (`ferres://`, `fonts:`, `resm:`) and `file://` keep their text; the URL standard normalizes the special schemes (`http://host` is `http://host/`, default ports and dot segments go) |
| `utilities::CultureInfo` | `From<&Locale> for CultureInfo` | `TryFrom<&CultureInfo> for Locale`, error `icu_locale_core::ParseError` | By name. The invariant culture is the unknown locale `und` both ways. A culture name that is not a BCP-47 tag (`es-ES_tradnl`, `x-custom`) is an error, which is why this direction is `TryFrom`: `CultureInfo::get_culture_info` takes any name |
| `utilities::DateTime` | `TryFrom<SystemTime>`, error `SystemTimeRangeError` | `TryFrom<DateTime> for SystemTime` | A system time is a UTC instant and gives a value of kind `Utc`, rounded down to a whole tick; outside of the years 1 to 9999 it is an error. A `Utc` value converts as it is; a `Local` or `Unspecified` value is local time and is converted with the offset of `TimeZoneInfo` (as .NET's `ToUniversalTime` and `new DateTimeOffset(DateTime)` do), without clamping. Ticks are whole nanoseconds, so this direction is exact; it fails only where the platform's `SystemTime` cannot hold the instant (before 1970 on `wasm32-unknown-unknown`) |
| `utilities::DateTimeOffset` | `TryFrom<SystemTime>`, error `SystemTimeRangeError` | `TryFrom<DateTimeOffset> for SystemTime` | The instant is kept and the offset is not: a value made from a system time has the offset zero |

The decision for `DateTime` and `DateTimeOffset` (section 2.1): no new crate. `std::time::SystemTime` is the type of the edges, and the port types stay underneath for the calendar, the kinds and the .NET text formats; C5 decides which public signatures take `SystemTime`. None of the conversions reads the clock.

### 7.2 Status of C3

- **Errors (C3a): done.** The error types of `src/` that were named after .NET exceptions are `...Error` types that implement `std::error::Error` and `Display` (with `source()` where they wrap an error); their messages, the type names the markup compiler reports and the markup names are unchanged. The old names remain as `#[deprecated(since = "0.5.0")]` aliases (a deprecated constant for a unit struct, a deprecated function for a tuple struct), so the inventory still counts them until they are removed after 0.5. The list is the migration table of `CHANGELOG.md`; the rows are in `docs/porting/data/upstream-names.toml`.
- **Kept, not error types:** the panic payload aliases `ferroui_base::threading::DispatcherException` and the private `Exception` of `ferroui-headless` (`Box<dyn Any + Send>`, which cannot implement `std::error::Error`); the private closure aliases `WriteNoTypeWithException` and `WriteWithTypeAndException` of the `ActionLogger` of `ferroui-splat` (functions that take an error); the factories of the derived exception classes, which are unit structs whose `new` returns the one error type (`XamlBindingsTransformException`, `XamlDataContextException`, `XamlPropertyPathException`, `XamlSelectorsTransformException` and `XamlStyleTransformException` of `ferroui-markup-xaml-loader`, whose type names select the diagnostic codes; `FerroWaylandPollException`, `FerroWaylandNetworkException`, `FerroWaylandFlushException`, `FerroWaylandReadException` and `FerroWaylandProtocolErrorException` of `ferroui-wayland`). They become constructors or kinds of their error type in a later step.
- **Kept, name taken:** `AggregateException` of `ferroui_base::data` (the module has an `AggregateError`, the aggregate of a binding notification) and `DecyclerException` of `ferroui_base::media::fonts::tables` (the module has the enum `DecyclerError`, its kind).
- **Resources (C3b, `IDisposable` to `Drop` and RAII guards):** done; the steps are in [idiomatic-rust-disposal.md](idiomatic-rust-disposal.md). The guard (`Subscription`, D1), the resources (D2), the internal subscriptions (D3), the public subscription APIs (D4) and the subscriptions to observables with `subscribe_guard` (D5) are done, and so is D6 (the samples, the application repositories and the inventory; the templates take it with their bump to 0.5).

### 7.3 Status of C4

C4 is done (steps A0 to A9, [idiomatic-rust-async.md](idiomatic-rust-async.md), section 5). Every asynchronous step of the user interface runs on the executor of `ferroui_system::task`, whose schedule on a user-interface thread is the dispatcher at a priority, and the ordering the dispatcher guarantees is unchanged and tested (G1 to G20). Tasks are `JoinHandle<T>` (`Dispatcher::start_local`, `spawn_local`, `spawn`) and, in the reactive libraries, `Shared<Result<T, ObservableError>>`; a completion source is `task::oneshot`; the `SynchronizationContext` is the `Schedule` of the thread with `Dispatcher::current_priority`; the cancellation type is `CancellationToken` with `CancellationGuard`, and the cancellation error `task::Canceled`. `use_future` of `ferroui-view` runs on the dispatcher and aborts a replaced load (A8). The old types (`Task`, `TaskCompletionSource`, the executor functions of `threading::tasks`, the `SynchronizationContext` trait, `FerroSynchronizationContext`, `CancellationTokenSource` and the registrations, `OperationCanceledError`, `TaskCanceledError`, `DispatcherTask`, `DispatcherTaskScheduler`, `invoke_async_task*`, `to_task_scheduler*` and the local types listed there) are deprecated since 0.5.0 with their behaviour and `From` conversions, and are removed in the release after 0.5 (C6); the inventory counts them until then. The migration is the "Async migration" table of `CHANGELOG.md`, and the rows of `docs/porting/data/upstream-names.toml` map the upstream names.

### 7.4 Status of C5

C5 is done (steps S0 to Z, [idiomatic-rust-signatures.md](idiomatic-rust-signatures.md), section 4). The public signatures take and return the std and chosen-crate types: `std::time::Duration` for the timer intervals and delays and `SignedDuration` (the port, renamed from `TimeSpan`) where a span can be negative; `url::Url` for an absolute URI and `UriRef` for a reference that can be relative; `SystemTime` for an instant; `icu_locale_core::Locale` for the identity of a culture; `uuid::Uuid` for an identifier; `HashMap` or `IndexMap` for a map. Markup is unchanged: its names (`System.TimeSpan`, `System.Uri`, `System.DateTime`, `System.Globalization.CultureInfo`) and its text formats load as before, and the new types are further Rust forms of them in the run-time loader and the compiler. What stays by decision (section 3.9 of the plan) is in the inventory baseline as deliberate declarations: `CultureInfo` (the formatting parameter, O3), `DateTime` and `DateTimeOffset` (the calendar values, O4), the BSON `DateTime` and `Dictionary` of the remote protocol (wire types, O6) and the `Guid` of `ferroui-microcom` (the COM layout). The old forms (`TimeSpan`, `NegativeTimeSpanError`, `Dictionary` of `ferroui-system`, the `Guid` of Splat and of the remote protocol, the methods of `ICultureDataProvider` keyed by name) are deprecated since 0.5.0 and removed in the release after 0.5 (C6). A trait method has one signature, so the traits that changed (`IAssetLoader`, `ILauncher`, `IStorageItem`, `IStorageProvider`, `IUriContext`, `IRuntimePlatform::get_utc_now`, `IMessageTypeResolver`, the font backends) have no deprecated form. The migration is the "Signatures migration" table of `CHANGELOG.md`, and the rows of `docs/porting/data/upstream-names.toml` map the upstream names.
