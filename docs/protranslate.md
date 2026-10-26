# ProTranslate localization and globalization

ProTranslate is available as five workspace crates. The core can be used without
FerroUI, a desktop backend, a renderer, or a process-global locale. The native
adapter turns its services into FerroUI bindings and attached properties.

| Crate | Responsibility |
|---|---|
| `ferroui-protranslate-core` | Explicit culture state, ordered providers and fallback, cache policy, diagnostics, observable translations, number/date formatting, region and measurement services |
| `ferroui-protranslate-formats` | Normalized catalogs, 13 import/export formats, validation and diagnostics for data that a target format cannot preserve |
| `ferroui-protranslate-build` | Catalog validation, deterministic static providers, typed accessors, observable accessors and opt-in Rust source diagnostics |
| `ferroui-protranslate` | Native bindings, markup extensions, inherited attached properties and UI target lifetime management |
| `ferroui-protranslate-studio` | Portable catalog editing/review, undo/redo, session/settings/recovery/backup persistence, format helpers and AI translation orchestration |

These packages join the workspace's version and release process. Until this port
is released, use the workspace or the PR branch; the existing `0.4.0` release does
not contain these new package names. The [port mapping](porting/protranslate.md)
records the upstream baseline and differences from .NET.

## Run the sample

```sh
cargo run -p sample-protranslate-demo
```

The sample uses the WinUI theme and compiled XAML. Its English, Polish and Arabic
catalogs are compiled into a generated provider and a notifying strings model.
Actual `CompiledBinding` properties on the view model refresh text, number
formatting, region metadata, measurements and flow direction. Controls also
exercise region/measurement overrides and a dynamic native `Format` multi-binding.

## Generate catalogs at build time

Prefer generated catalogs when resource inputs are known at build time. Runtime
lookup needs no JSON parser, file discovery or reflection.

```toml
[dependencies]
ferroui-protranslate-core = { workspace = true }
ferroui-protranslate = { workspace = true }

[build-dependencies]
ferroui-protranslate-build = { workspace = true }
```

For example, `Strings.en-US.json`:

```json
{
  "Hello": "Hello, {0}!",
  "Orders.Total": "Total: {0:N2}",
  "Empty": ""
}
```

And `Strings.pl-PL.json`:

```json
{
  "Hello": "Witaj, {0}!",
  "Orders.Total": "Razem: {0:N2}",
  "Empty": ""
}
```

Register exact inputs in `build.rs`:

```rust
fn main() {
    ferroui_protranslate_build::Builder::new()
        .catalog("Strings.en-US.json")
        .catalog("Strings.pl-PL.json")
        .generate()
        .expect("translation catalogs must be valid");
    // Run the application's normal FerroUI XAML build step here too.
}
```

The builder writes `OUT_DIR/protranslate.rs` and
`OUT_DIR/protranslate.manifest.json`. It tracks each input with Cargo's
`rerun-if-changed` protocol, sorts output deterministically, escapes Rust literals,
resolves identifier collisions, and avoids rewriting unchanged output.

```rust
use std::rc::Rc;
use ferroui_protranslate::Translation;
use ferroui_protranslate_core::{Culture, CultureService, TranslationService};

mod generated {
    include!(concat!(env!("OUT_DIR"), "/protranslate.rs"));
}

fn configure() -> Result<TranslationService, Box<dyn std::error::Error>> {
    let cultures = CultureService::new(Culture::parse("en-US")?);
    let translations = TranslationService::new(
        Rc::new(generated::GeneratedTranslationProvider),
        cultures,
    );
    Translation::set_service(translations.clone());
    let strings = generated::ProTranslateStrings::new(translations.clone());
    assert_eq!(strings.format_hello(["FerroUI".into()])?, "Hello, FerroUI!");
    translations.cultures().set_culture(Culture::parse("pl-PL")?);
    assert_eq!(strings.format_hello(["FerroUI".into()])?, "Witaj, FerroUI!");
    Ok(translations)
}
```

The generated API includes `ProTranslateKeys::ORDERS_TOTAL`,
`get_orders_total`, `value_orders_total`, `format_orders_total` and
`observe_orders_total`. Format/observe methods take `[FormatArgument; N]`, where
`N` is the maximum referenced placeholder index plus one. Rust checks the number
of arguments at the call site. Placeholder sets must agree across languages.
Key-only `.protranslate.keys.txt` files generate accessors without inventing
translations. Their format/observe methods accept slices/vectors because the
argument count is not known until the application provides a catalog.

Import `generated::ProTranslateAccessors` to use the same generated accessors
directly on `TranslationService`. `ProTranslateProviderManifest` exposes static
keys, cultures, source files, entry placeholder information and key-declaration
provenance alongside the JSON manifest. `GeneratedTranslationProvider::with_name`
creates a provider with an explicit identity. Neutral entries provide a default
for any culture; an exact culture entry takes precedence.

Culture inference recognizes `Strings.<culture>` files, Android `values-*`
directories and Apple `<culture>.lproj` directories. `Base.lproj` is neutral.
Use `catalog_for_culture` or `catalog_with_options` when the source layout needs
an explicit culture or format. Declared empty catalogs still participate in
coverage checks.

For a model used by FerroUI compiled bindings, enable
`Builder::notify_properties(true)` and add `ferroui-system` as a dependency.
The generated `ProTranslateStrings` then implements the framework's
`INotifyPropertyChanged` interface and identity equality. It owns one shared,
automatically disconnected service subscription; clones share that identity.
Typed compiled-property accessors can read its generated getters without a
reflection-style binding path. The pure-core output remains the default.

## Native XAML

Register `ferroui_protranslate::register_types()` before runtime loading. Compiled
applications include the adapter's exported type metadata in their normal XAML
build; the sample demonstrates this end to end.

```xml
<StackPanel xmlns="https://github.com/ferroui"
            xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"
            xmlns:pt="https://github.com/protranslate/xaml"
            pt:Translation.AutoFlowDirection="True">
  <TextBlock Text="{pt:Translate Title}" />
  <TextBlock Text="{pt:T Subtitle}" />
  <Slider x:Name="Amount" Value="1234.5" />
  <TextBlock Text="{pt:Format Orders.Total, Value={Binding #Amount.Value}}" />
  <TextBlock pt:Translation.Key="OptionalLabel"
             pt:Translation.FallbackValue="Optional label" />
</StackPanel>
```

`Translate`/`T` return native one-way binding expressions. `Format`/`F` combine the
value binding with a culture revision signal through native `MultiBinding`. A
culture change reformats the value even if the translated format string happens
to be identical in both languages. Attached properties support `Key`,
`FallbackValue`, `StringFormat`, inherited `Culture`, and inherited
`AutoFlowDirection`. `Translation.Error` exposes a failure from attached-property
evaluation. Binding failures use the framework's binding error notifications.

An explicit `TranslationBindingSource` allows a service to be scoped to a control
tree instead of using the UI-thread default. Replacing the service on an existing
source refreshes its consumers. As in the source adapter, setting attached
`Translation.Culture` selects the culture of the effective source.

The optional `dependency-injection` feature provides
`ProTranslateServiceCollectionExtensions::add_protranslate` and
`ProTranslateServiceProviderExtensions::use_protranslate` for the native FerroUI
container. Registration shares culture, provider, translation and globalization
services; the explicit bootstrap call configures the XAML default source.
Registration is lazy and idempotent, preserves custom registrations and resolves
factories through the same service provider. Disabling automatic flow direction
removes its value; an explicit local `FlowDirection` continues to take precedence.

## Culture, providers and fallback

`Culture` accepts a canonical BCP-47 name; the empty string is invariant. ICU4X
supplies likely region, script direction, decimal digits and grouping. Formatting
culture and UI lookup culture can be changed independently through
`CultureService::set_cultures`. Repeating the same pair is a no-op.

Lookup tries the requested culture and its parents, then explicitly configured
fallback cultures and their parents, then the configured default culture and its
parents. Each culture appears once. Parent, fallback and default behavior are
controlled by `TranslationFallbackOptions`. A found empty string is a successful
translation. A missing key returns its key by default, with structured diagnostics.

`InMemoryTranslationProvider` is useful for dynamic application catalogs.
`CompositeTranslationProvider` tries providers in registration order. Applications
can implement the small `TranslationProvider` trait to bridge a database, a
resource service or another localization library. A provider returns
`LocalizedString` with key, value, resolved culture, provider identity, missing
state and diagnostics.

The cache is keyed by culture and key. Mutable built-in providers expose revisions;
the service invalidates stale cached results when their revision changes. Custom
providers can expose revisions or call `TranslationService::refresh()` after
updates. `clear_cache` and `remove_cached_string` support explicit invalidation;
`refresh` additionally notifies observable consumers. A finite
`maximum_entries` bounds the cache; zero retains the source library's unbounded
policy. Missing-result caching and clearing on culture changes are configurable.

## Observable values and ownership

```rust
let value = translations.observe("Hello", vec!["FerroUI".into()]);
let subscription = value.subscribe(|value| {
    println!("translation changed: {value:?}");
});
// Keep both handles while the view model needs updates.
// Dropping `subscription` disconnects its callback.
```

Core services are cloneable `Rc` handles scoped to one application thread.
Callbacks run synchronously after state mutation, outside internal borrows.
Subscriptions disconnect on drop; event sources keep callbacks weakly.
Observable strings hold their service, while the service does not retain its
observable strings. They expose resolved value/culture metadata even when only
the metadata changes, and can disconnect explicitly. Native targets hold their subscriptions and are referenced
weakly by refresh callbacks. Cross-thread producers post work to the UI dispatcher
before changing UI-owned services. No process/thread locale is mutated implicitly.

## Formatting and globalization

Composite formatting supports indexed arguments, escaped braces, alignment and
numeric format specifiers. `FormatArgument` supports strings, booleans, signed and
unsigned integers, floating-point numbers, null, exact decimals, and application
types implementing `LocalizedFormat`. Original integer widths and `f32`/`f64`
precision are retained. Standard numeric formats and custom masks include
scientific notation, interleaved literals/digits, sections, scaling and exact
binary/decimal rounding. Culture-specific grouping, signs and symbols use the
pinned locale data. The regression suite includes 1,404 cases recorded from
.NET 10.0.14 by the shipped fixture generator.

`LocalizedDateTime` and `time::OffsetDateTime` carry an explicit UTC offset.
`LocalizedNaiveDateTime` and `time::PrimitiveDateTime` retain unspecified date/time
semantics; the wrapper can also represent UTC explicitly. The formatting culture
selects the calendar, and a Unicode `u-ca` extension can override it. ICU calendar
conversion covers Gregorian, Buddhist, Chinese/Dangi, Coptic, Ethiopian, Hebrew,
Indian, several Islamic calendars, Japanese, Persian and ROC calendars. Standard
and custom date tokens preserve the appropriate era/year/month, including leap
months. IANA time-zone conversion remains an explicit host policy. Exact decimals
avoid coercing currency or large integer data through `f64`.

`GlobalizationService` combines the same culture/translation source with
replaceable region, measurement resolver, conversion and unit formatting services.
Region and measurement overrides raise metadata notifications even when the
language is unchanged. The built-in measurement system mapping preserves the
source policy: US/LR/MM use US customary units, GB uses imperial units, and metric
regions use metric units.

There are 31 units across length, temperature, mass, volume and speed. Conversions
include affine Celsius/Fahrenheit/Kelvin conversion and distinct US/imperial
volumes. Incompatible categories and unsupported profile units return errors.

```rust
use ferroui_protranslate_core::{GlobalizationService, MeasurementUnit};

let globalization = GlobalizationService::new(translations.clone());
let region = globalization.region_profile()?;
let distance = globalization.format_measurement_for_profile(
    1000.0, MeasurementUnit::Meter, Some("N1"),
)?;
```

Provider and format failures have configurable policies: report and continue or
return the error. Reports go to both the diagnostic sink and subscribers. Native
formatting delegates to this policy rather than defining another error policy.

## Import and export

Supported formats are ProTranslate JSON, i18next JSON, Flutter ARB, RESX, Android
resources, Apple `.strings`, `.stringsdict`, `.xcstrings`, gettext PO/POT,
XLIFF 1.2, XLIFF 2.0, CSV and TSV.
The `Xliff20` export variant writes version 2.1, preserving the upstream convention.

```rust
use ferroui_protranslate_formats::{
    TranslationCatalogConverter, TranslationFileFormat, TranslationFormatOptions,
};

let options = TranslationFormatOptions {
    culture: Some("pl-PL".into()),
    ..Default::default()
};
let imported = TranslationCatalogConverter::import(
    json, TranslationFileFormat::ProTranslateJson, &options, Some("Strings.pl-PL.json"),
);
if imported.has_errors() {
    // Present diagnostics before changing the application's current catalog.
}
let exported = TranslationCatalogConverter::export(
    &imported.catalog, TranslationFileFormat::Resx, &options,
);
// Check exported.diagnostics before accepting a conversion that loses metadata.
```

The normalized model retains source text, comments, context, state, plural category,
format and extensible metadata. Plurals have stable `key.category` keys; gettext
context uses its standard U+0004 separator. Format-specific rich structures are
preserved where representable, with explicit diagnostics when flattening or a
target format loses information. This layer does not add ICU MessageFormat
evaluation or automatic plural selection to runtime translation lookup.

Same-format round trips retain ARB document attributes, Android resource flags,
gettext headers/previous/obsolete entries, nested Apple plural/device variations
and substitutions, and XLIFF segments, inline codes and notes. Edited XLIFF inline
structure is validated before export. The serialized normalized catalog includes
document metadata, so saving a Studio session does not discard it.

## Portable Translation Studio engine

`ferroui-protranslate-studio` extracts the reusable authoring behavior from the
upstream Translation Studio sample. A host supplies the window/view models,
clipboard/file picker integration, storage location, time and network transport.

```rust
use std::rc::Rc;
use ferroui_protranslate_studio::{
    CatalogGateway, EntryFilter, MemoryStudioStorage, SessionService,
    StudioDocument, StudioPaths,
    formats::{TranslationFileFormat, TranslationFormatOptions},
};

let imported = CatalogGateway::import(
    r#"{"Hello":"Witaj"}"#,
    TranslationFileFormat::ProTranslateJson,
    &TranslationFormatOptions { culture: Some("pl-PL".into()), ..Default::default() },
    "Strings.pl-PL.json",
    "pl-PL",
)?;
let mut document = StudioDocument::new("project-polish", imported, 0)?;
let id = document.filtered_ids(&EntryFilter::default())[0];
document.set_target(id, "Witaj, świecie!")?;
document.undo()?;
document.redo()?;

let storage = Rc::new(MemoryStudioStorage::default());
let paths = StudioPaths::new("sessions", "recovery.json", "backups", "settings.json")?;
let sessions = SessionService::new(storage, paths);
let saved = sessions.save_session(&mut document, None, 1)?;
assert!(saved.document_is_clean);
```

The document preserves the full catalog and uses stable entry identities, so
duplicate keys/cultures remain distinct. Text, notes, review status, additions,
deletion and batch edits support bounded undo/redo. Filtering, measured coverage,
placeholder validation, snippet completion and format previews are reusable APIs.
Composite previews call the core formatter with explicit culture/date inputs;
other expression previews are editor illustrations.

Session, recovery, settings and backup services share an explicit `StudioStorage`
contract. Native `FileStudioStorage` uses atomic replacement; backups use atomic
create-if-absent and rotate only after a successful new save. Failed saves retain
the previous file and dirty state. Corrupt records return diagnostics. Storage
namespaces cannot overlap. `AutoSaveScheduler::tick` lets the host schedule
snapshots on its owning thread. The in-memory engine can be used on WebAssembly
with default features disabled.

`AiTranslationManager` provides OpenAI Chat Completions, Anthropic Messages and
Gemini generateContent adapters through an injected `AiTransport`. It supports
explicit connection tests, paginated model discovery, single/batch translation,
progress and cancellation. Hosts supply credentials and select a model returned
by discovery or provide an explicit model ID. No historical model is selected
automatically, and discovery failures remain errors. Tests use fake transports.

Capture a `TranslationPlan` before awaiting a request and turn its result into
suggestions afterward. Suggestions track the source/target text, culture, notes,
review state and entry/document revision. Accepting a stale result fails instead
of overwriting newer work; accepting a batch is one undoable transaction.
Settings omit credentials by default and provide an explicit persistence policy
for hosts that need it. See the [Studio crate guide](../src/ProTranslate/FerroUI.ProTranslate.Studio/README.md)
for provider protocol references and the full host contract.

## Build diagnostics

| Code | Meaning |
|---|---|
| `PTA001` | A statically known usage references a missing key |
| `PTA002` | Argument count or cross-language placeholder set mismatch |
| `PTA003` | Culture catalog coverage gap |
| `PTA004` | A dynamic key cannot be checked at build time |
| `PTA005` | Invalid source or culture input |
| `PTSG002` | Duplicate key declaration or duplicate culture/key translation |
| `PTSG003` | Catalog text is not a supported composite format |
| `PTF...` | Parser, catalog-validation or format-conversion diagnostic |

The build script can inspect selected Rust receivers:

```rust
use ferroui_protranslate_build::{Builder, SourceAnalysisOptions};

Builder::new()
    .catalog("Strings.en-US.json")
    .source("src/view_model.rs", SourceAnalysisOptions::default()
        .receiver("translations").receiver("self.translations"))
    .deny_warnings(true)
    .generate()?;
```

The syntax scanner handles literal keys, supported local string constants,
generated constants and statically sized argument arrays. Selected receiver paths
also support static facade calls; generated accessor scanning is opt-in through
`generated_accessors(true)`. Ambiguous/shadowed/conditional constants remain
unknown. The scanner does not replace rustc's semantic model or inspect arbitrary macro expansion. Consumers
with semantic information can submit resolved `KeyUsage` records directly.
Typed generated methods provide the primary compiler-checked API.

## Validate

```sh
cargo test --locked -p ferroui-protranslate-core -p ferroui-protranslate-formats \
  -p ferroui-protranslate-build -p ferroui-protranslate-studio
cargo test --locked -p ferroui-protranslate -p sample-protranslate-demo \
  --features ferroui-protranslate/dependency-injection --lib
cargo check --locked -p sample-protranslate-demo --bin sample-protranslate-demo
cargo check --locked -p ferroui-protranslate-core -p ferroui-protranslate-formats \
  -p ferroui-protranslate-studio --no-default-features --target wasm32-unknown-unknown
```

The focused workflow runs service tests on Linux, macOS and Windows, native binding
and compiled XAML tests on Linux, and a WebAssembly compilation of the independent
service/catalog/authoring crates. The workspace's package checks include all five
libraries. The [review record](porting/protranslate-review.md) describes the
correctness audit and its regression coverage.
