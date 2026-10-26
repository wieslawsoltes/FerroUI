# ProTranslate Rust port

## Source and objective

Port the reusable localization and globalization framework from
[wieslawsoltes/ProTranslate](https://github.com/wieslawsoltes/ProTranslate), pinned to
`b088669a22c8979d54dd81098991f8d16f880156`, into idiomatic Rust packages in FerroUI.
The source is MIT licensed. Framework-specific .NET adapters are mapped to one native
FerroUI adapter rather than retained as foreign-runtime dependencies.

## Package boundaries

- `ferroui-protranslate-core`: culture snapshots, providers and ordered fallback,
  caching, diagnostics, observable translations, formatting, region and measurement services.
- `ferroui-protranslate-formats`: normalized catalogs and loss-aware format import/export.
- `ferroui-protranslate`: native FerroUI bindings, attached properties, XAML integration,
  and culture-driven target refresh.
- `ferroui-protranslate-build`: deterministic typed catalog generation and validation.
- A runnable sample, public API documentation, port mapping, and focused CI validation.

## Behavioral requirements

Inputs are explicit cultures, translation keys/catalogs, formatting arguments,
provider configuration and user-selected region or measurement overrides. Outputs
are localized strings, structured diagnostics, refreshed native properties and
reproducible generated Rust. The core must remain UI independent. Provider order,
parent/default culture fallback and failure policies must be deterministic. Runtime
culture changes must refresh text and direction without retaining dead UI targets.

Validation covers culture switching, empty and missing translations, provider errors,
format errors, cache invalidation, measurements, catalog round trips and diagnostics,
generated code, native binding updates, and subscription/target lifetime behavior.

## Source-to-Rust mapping

| Upstream project or type | Rust counterpart | Mapping |
|---|---|---|
| `ProTranslate.Abstractions` | `ferroui-protranslate-core` public contracts | Provider, diagnostic, region, measurement, conversion and formatter traits; enums and immutable value types are colocated with their implementations |
| `CultureService` / `ICultureService` | `Culture`, `CultureService`, `CultureSnapshot` | Explicit formatting/UI cultures, idempotent switches, old/new snapshot events, region and flow-direction lookup |
| `InMemoryTranslationProvider` | Same Rust type | Exact-culture lookup, mutable entries, successful empty values and revision-aware cache invalidation |
| `CompositeTranslationProvider` | Same Rust type | Ordered providers, recoverable failures and structured failure traces |
| `TranslationService` / `ITranslationCacheInvalidator` | `TranslationService` | Parent/explicit/default fallback, missing-key policy, error policies, cache configuration, invalidation and refresh |
| `ObservableLocalizedString` | Same Rust type | Typed values and arguments, culture-driven refresh, RAII subscriptions and weak callback ownership |
| `GlobalizationService` | Same Rust type | Shared culture source; region and measurement overrides; metadata-change notifications; replaceable policy services |
| `DefaultRegionProfileProvider` | Same Rust type and `RegionProfileProvider` | ICU likely regions with pinned CLDR region/currency/native-name data and explicit profile overrides |
| `DefaultMeasurementSystemResolver` | Same Rust type and `MeasurementSystemResolver` | US/LR/MM, GB, metric and custom profiles follow the source policy |
| `DefaultUnitConversionService` | Same Rust type and `UnitConversionService` | All 31 source units, category checks, affine temperature conversion and profile conversion |
| `DefaultLocalizedUnitFormatter` | Same Rust type and `LocalizedUnitFormatter` | Localized numeric text with the source unit-symbol contract |
| .NET `string.Format` boundary | `FormatArgument`, `format_composite`, `LocalizedFormat`, `LocalizedDateTime` | Indexed placeholders, braces, alignment, typed numeric/date values and an explicit custom formatter extension point |
| `ProTranslate.Formats` | `ferroui-protranslate-formats` | All 13 format variants, normalized entry metadata, import/export, validation and loss diagnostics |
| `ProTranslate.SourceGenerator` | `ferroui-protranslate-build` | Explicit Cargo inputs, typed key/accessor generation, static binary-search provider and reproducible manifest |
| Generated `ProTranslateStrings` | Generated Rust `ProTranslateStrings` | Typed get/value/format/observe methods; optional native `INotifyPropertyChanged` surface with shared identity and automatic unsubscribe |
| `ProTranslate.Analyzers` | Build compiler and scoped Rust usage scanner | Missing keys, argument/placeholder mismatches, coverage gaps, dynamic keys, malformed inputs and duplicates |
| `ProTranslate.Avalonia` | `ferroui-protranslate` | Native property/binding implementation of `Translate`/`T`, `Format`/`F`, binding source, attached localization and flow direction |
| `ProTranslate.Wpf`, `.Maui`, `.WinUI`, `.Uno` | One native FerroUI adapter | Their framework-specific dispatchers, dependency objects and CLR metadata are replaced by FerroUI's cross-platform APIs |
| `ProTranslate.ResourceManager` | RESX catalog import/generation and `CallbackTranslationProvider` | Runtime .NET resource-manager/assembly loading is a host bridge; `.resx` data can be compiled into a Rust provider |
| `ProTranslate.MicrosoftExtensions` | Optional adapter `dependency-injection` feature | Native `ServiceCollection`/`ServiceProvider` helpers; `IStringLocalizer` hosts adapt through `TranslationProvider`/callback providers |
| Demonstration applications | `samples/ProTranslateDemo` | Compiled native XAML, generated English/Polish/Arabic catalogs, language switching, dynamic number formatting, RTL and region/measurement displays |

The core depends on ICU4X, fixed decimal and `time`, with no rendering or UI
dependency. The format and build crates remain independent of the desktop stack.
The adapter uses the actual native binding expression, multi-binding, attached
property, dependency-injection and XAML metadata APIs. No special-case change to
FerroUI's XAML loader is required.

## Build and lifetime design

Catalog generation is explicit and deterministic. Every file participates in
Cargo invalidation. Generated runtime tables contain literals and perform exact
culture/key binary searches; the shared service owns fallback. Typed format
accessors use compile-time array lengths when the catalog determines arity. A
key-only declaration retains variable arguments because its arity is unknown.
The generator emits a manifest of keys, cultures, source files and placeholders.

Core services use application-thread `Rc` handles. Culture and metadata changes
publish snapshots after mutation. Event sources retain callbacks weakly; an RAII
subscription keeps the callback alive and disconnects on drop. Generated notifying
models share one service subscription across clones. Native bindings use weak
targets, release subscriptions when disposed, and reconnect when their scoped
source changes. The `Format` binding includes a revision signal so an unchanged
format template still rerenders with the new numeric culture.

## Locale data provenance

ICU4X supplies canonical locale parsing, likely subtags, script directionality,
decimal symbols and grouping. Additional region/currency/native-name/affix data
and Gregorian date patterns/names are generated from the official Unicode CLDR
48.0.0 npm archives. The generated files carry archive SHA-256 values, and the
crate's `NOTICE.md` includes the Unicode license.

Reproduction scripts are shipped with the core:

- `tools/generate_locale_data.py`: consumes `cldr-core`, `cldr-numbers-full` and
  `cldr-localenames-full` archives and writes `locale_data.rs`.
- `tools/generate_datetime_data.py`: consumes the CLDR Gregorian dates and core
  archives (including flexible-day-period rules) and writes `date_data.rs`.

These scripts use explicit archive inputs and never read a machine's installed
locale database. Generated tables do not need to be downloaded or regenerated
during application builds.

## Deliberate differences and format boundaries

| Area | Rust behavior and reason |
|---|---|
| Ambient/thread culture | Services require explicit culture state and never mutate the host locale. The source's optional `Thread.CurrentCulture` mutation has no process-wide Rust counterpart. |
| Thread ownership | Services and UI bindings are application-thread scoped. Cross-thread producers dispatch work to that thread; no UI object is made `Send`/`Sync`. |
| Culture identifiers | Validated BCP-47 text replaces managed `CultureInfo` objects. Attached `Culture` is text so XAML can construct it directly. |
| Invariant and locale data | Invariant behavior is deterministic; regional formatting uses pinned Unicode data and can differ from a particular OS/.NET ICU or NLS data version. |
| Dates/calendars | Built-in dates are Gregorian values with explicit UTC offsets. Non-Gregorian calendars and IANA zone conversion use application policy/custom formatters; they are not silently inferred. |
| Custom values and formats | Rust values implement `LocalizedFormat` explicitly. Unsupported numeric masks return structured format failures; there is no reflection-driven `IFormattable` discovery. |
| Compiler analysis | Generated typed methods are compiler checked. The optional source scanner has explicitly selected receivers and a syntax model, not Roslyn/rustc semantic type information or arbitrary macro expansion. |
| Runtime .NET integrations | `.resources` assembly discovery, `ResourceManager` and `IStringLocalizer` implementations remain outside the Rust process unless a host provides a callback bridge. RESX catalogs are supported directly. |
| ICU/i18next/ARB expressions | Preserved as catalog text. This port does not invent an ICU MessageFormat evaluator or automatic plural-selection runtime; the source service formats .NET-style indexed arguments. |
| Gettext plurals | Formulas and indices are retained without evaluating them. Extra plural indices are kept distinct instead of being collapsed. |
| XLIFF | Inline codes are flattened with diagnostics, and multiple segments get distinct `unit#segment` keys. Advanced XLIFF modules and reconstruction of original inline/segmentation structure are outside the normalized source model. |
| XLIFF version name | The source enum's `Xliff20` export produces XLIFF 2.1; the port preserves that convention. |
| Apple plural catalogs | `.stringsdict` exports plural records; singular records are rejected with diagnostics instead of being silently discarded. |

## Validation

The focused workflow `.github/workflows/protranslate.yml` covers:

1. Core, all format converters and build tooling on Linux, macOS and Windows.
2. Real native property/binding tests, runtime XAML aliases, scoped sources,
   dependency injection, failure diagnostics, and subscription/target lifetimes.
3. The compiled XAML sample and generated notifying model through a typed compiled
   property binding, including live culture changes and final release.
4. The desktop sample entry point.
5. Core and catalog crates compiled for `wasm32-unknown-unknown`.
6. Formatting and strict lint checks for the independent libraries.

Regression cases cover separate UI/format cultures, empty values, deterministic
fallback, failed providers, malformed composite formats, reentrant callbacks,
cache generations, all unit pairs within compatible categories, catalog escaping
and round trips, malformed transactional imports, duplicate catalog members,
generated-code compilation/execution and literal/constant key diagnostics.

Run commands and public usage are in the [guide](../protranslate.md). Existing
workspace packaging validates release metadata, exact internal dependency pins,
license/notice inclusion and the generated README crate table.
