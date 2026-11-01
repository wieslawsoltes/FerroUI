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
| `TimeSpan` (`ferroui-base`, `animation/time_span.rs`: signed 100 ns ticks) | `std::time::Duration`; a signed duration newtype only where a negative span is part of the contract (animation clocks that step backwards, a negative `Delay` that is rejected when the animation runs) | C2, C5 | Tick rounding of `from_seconds`, `from_milliseconds` and the other `f64` constructors; the text format of `Parse` and `ToString` in markup |
| `DateTime`, `DateTimeOffset` (`ferroui-base`, `utilities`) | `std::time::SystemTime` and one small calendar type, or one optional integration (`time` or `jiff`); decided once, in C2 | C2, C5 | The invariant text format used by markup; `SystemTime::now()` is not available on `wasm32-unknown-unknown`, so the clock stays behind the dispatcher there |
| `CultureInfo`, `NumberFormatInfo`, `DateTimeFormatInfo` (`ferroui-base`, `utilities/culture_info.rs`) | ICU4X: `icu_locale_core::Locale`, with `icu_decimal` and `icu_datetime` for formatting (ICU4X 2.4 is in the workspace) | C2, C5 | The culture-specific formatting the controls show (numbers, dates, the invariant culture of markup) |
| `Uri` (`ferroui-base`, `utilities/uri.rs`) | `url::Url`, or a newtype over it where relative references are part of the contract (the resource URIs of the framework, relative `Source` values in markup) | C2, C5 | Relative and absolute forms, the resource schemes of the framework, `ToString` |
| `Guid` (three definitions: `ferroui-splat`, `ferroui-remote-protocol`, `ferroui-microcom`) | `uuid::Uuid`, or one shared type where a layout is part of a contract (the COM `GUID` layout of `ferroui-microcom`) | C1 | Byte order of the wire and COM layouts; the text format |
| `Dictionary<K, V>` (`ferroui-system`, `collections/generic`: the .NET slot and free-list order) | `std::collections::HashMap`; `indexmap::IndexMap` where a caller enumerates the map and depends on the order; the port of the .NET order only where the exact .NET sequence after removals is observable | C1, C5 | Enumeration order: a caller that enumerates is checked, and the order it needs is tested |
| `Task<T>`, `TaskCompletionSource<T>`, `SynchronizationContext` (`ferroui-system`, `threading`; local copies in several crates) | Rust futures on the local executor of the UI dispatcher; a oneshot channel instead of `TaskCompletionSource` | C4, C5 | The dispatcher's ordering guarantees (priorities, the order of continuations); tests come first |
| `CancellationToken`, `CancellationTokenSource`, `CancellationTokenRegistration` (`ferroui-system`, `threading`) | One small cancellation type of the workspace, or a `tokio_util`-style token; decided once, in C4 | C4, C5 | Registration order and the synchronous run of callbacks on cancel |
| `IDisposable` handles (`Rc<dyn IDisposable>` with an explicit `dispose()`) | `Drop` and RAII guards: a subscription token that unsubscribes when it is dropped; an explicit `close()` only where the order of release matters | C3, C5 | The port keeps a subscription alive when its handle is ignored ([PORTING-GUIDE.md, section Types](porting/PORTING-GUIDE.md#types)). A guard changes that: it is `#[must_use]`, and keeping a subscription for the life of its source is an explicit call (for example `detach()`). Each converted API states which it does |
| `...Exception` types | `...Error` types implementing `std::error::Error` (with `Display` and `source()`), returned in `Result` | C3, C5 | Messages that a user can observe (logs, the error text of markup loading) |
| `Nullable<T>`, `Optional<T>` | `Option<T>` (already the rule) | done | |
| `List<T>`, `IList<T>` as values | `Vec<T>`, slices and iterators (already the rule; `FerroList<T>` stays for observable and shared lists) | done | |

### 2.2 Chosen crates

| Need | Crate | Notes |
|---|---|---|
| URLs and URIs | `url` | One `Url` type at every edge; a newtype only where the framework's relative references need it |
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

The summary counts, per pattern, the declarations or the occurrences, the files with at least one, and the crates. The kept names (section 1) are measured too, so that the size of what stays is known.

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
| C3 | Resources and errors: `IDisposable` to `Drop` and RAII guards, `...Exception` to `...Error` | 0.5 |
| C4 | Async: `Task`, `TaskCompletionSource` and `SynchronizationContext` to futures on the dispatcher's executor; one cancellation type | 0.5 |
| C5 | Public signatures take and return the std and chosen-crate types; the old types remain one release as deprecated aliases or conversions; migration notes | 0.5 |
| C6 | The check is clean, the API gate has a 0.5 baseline, the samples and application repositories are updated | |

### 7.1 Status of C3

- **Errors (C3a): done.** The error types of `src/` that were named after .NET exceptions are `...Error` types that implement `std::error::Error` and `Display` (with `source()` where they wrap an error); their messages, the type names the markup compiler reports and the markup names are unchanged. The old names remain as `#[deprecated(since = "0.5.0")]` aliases (a deprecated constant for a unit struct, a deprecated function for a tuple struct), so the inventory still counts them until they are removed after 0.5. The list is the migration table of `CHANGELOG.md`; the rows are in `docs/porting/data/upstream-names.toml`.
- **Kept, not error types:** the panic payload aliases `ferroui_base::threading::DispatcherException` and the private `Exception` of `ferroui-headless` (`Box<dyn Any + Send>`, which cannot implement `std::error::Error`); the private closure aliases `WriteNoTypeWithException` and `WriteWithTypeAndException` of the `ActionLogger` of `ferroui-splat` (functions that take an error); the factories of the derived exception classes, which are unit structs whose `new` returns the one error type (`XamlBindingsTransformException`, `XamlDataContextException`, `XamlPropertyPathException`, `XamlSelectorsTransformException` and `XamlStyleTransformException` of `ferroui-markup-xaml-loader`, whose type names select the diagnostic codes; `FerroWaylandPollException`, `FerroWaylandNetworkException`, `FerroWaylandFlushException`, `FerroWaylandReadException` and `FerroWaylandProtocolErrorException` of `ferroui-wayland`). They become constructors or kinds of their error type in a later step.
- **Kept, name taken:** `AggregateException` of `ferroui_base::data` (the module has an `AggregateError`, the aggregate of a binding notification) and `DecyclerException` of `ferroui_base::media::fonts::tables` (the module has the enum `DecyclerError`, its kind).
- **Resources (C3b, `IDisposable` to `Drop` and RAII guards):** open.
