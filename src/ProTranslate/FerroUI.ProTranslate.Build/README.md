# ProTranslate build integration

`ferroui-protranslate-build` generates Rust catalog providers, typed keys and
accessors, a provider manifest, and localization diagnostics from explicit input
files. The generated runtime provider depends only on `ferroui-protranslate-core`.
Native property notifications are an explicit opt-in and additionally require
`ferroui-system` in the consuming crate.

```rust,ignore
// build.rs
fn main() {
    ferroui_protranslate_build::Builder::new()
        .catalog("Strings.en-US.json")
        .catalog("Strings.pl-PL.json")
        .catalog("app.protranslate.keys.txt")
        .notify_properties(true)
        .generate()
        .expect("valid translation catalogs");
}
```

```rust,ignore
// Application module
mod generated {
    include!(concat!(env!("OUT_DIR"), "/protranslate.rs"));
}
```

For a key `Shell.Title`, generated members include
`ProTranslateKeys::SHELL_TITLE`, `get_shell_title`, `value_shell_title`,
`format_shell_title`, and `observe_shell_title`. Key names are escaped as Rust
string literals; valid identifiers are assigned deterministically, with collision
suffixes. `ProTranslateStrings::new(service)` stores an application-scoped
`TranslationService`, and `culture()` / `ui_culture()` expose its current cultures.
Value accessors return `Result`, preserving the configured provider failure policy.

The generated `ProTranslateAccessors` trait exposes the same get/value/format/observe
methods directly on `TranslationService`. Import the trait for extension-style
calls without allocating a strings model. `ProTranslateProviderManifest` exposes
static `KEYS`, `CULTURES`, `SOURCE_FILES` and `ENTRIES`, including source filenames,
placeholder indices and whether each format contract was validated. These tables
are available at runtime without parsing the JSON manifest.

Direct generated-provider lookups validate keys. Neutral catalog entries are
defaults for every requested culture, as in the source generator; a culture-specific
entry takes precedence. `GeneratedTranslationProvider::with_name` supplies a
validated custom provider identity while the default unit provider remains usable.

For catalog values with a known composite format, formatting and observation
methods accept `[FormatArgument; N]`, where `N` is the largest distinct placeholder
index plus one. Repeated placeholders do not increase the argument count. Key-only
files provide no format contract, so their formatting methods accept slices and
observation methods accept vectors rather than inventing a zero-argument limit.
Malformed composite text likewise retains an unknown argument contract and its
diagnostic, rather than generating a misleading fixed zero-argument accessor.

## Native typed binding notifications

With `.notify_properties(true)`, generated `ProTranslateStrings` implements
`ferroui_system::component_model::INotifyPropertyChanged`. Each instance owns one
core service subscription. Its `Clone` handles share the same state and notification
event; `PartialEq` and `Eq` compare that identity. The service callback holds only a
weak reference, and dropping the last handle removes the subscription.

Culture switches and `TranslationService::refresh()` raise an empty property name,
which is the framework's contract for invalidating every typed property. The
generated `refresh()` method can also raise that notification directly. Native
compiled property paths can read `value_shell_title()` through a typed getter
closure and choose an explicit application policy for errors. No reflected resource
discovery or string-path getter registry is generated.

## Inputs and diagnostics

The compiler accepts all thirteen formats supported by
`ferroui-protranslate-formats`, plus line-oriented `.protranslate.keys.txt` files.
`Strings.<culture>.json` and compound catalog suffixes infer culture from the
filename. Invalid tags in that explicit convention produce errors, including for
empty catalogs. `CatalogInput::with_culture` supplies an explicit culture for other
naming conventions. Catalog formats with native locale metadata use their imported
record cultures. Duplicate canonical `(culture, key)` records are errors.

Android `values-pl-rPL` / `values-b+zh+Hant+TW` directories and Apple `.lproj`
directories infer their locale, including Windows paths. `Base.lproj` is neutral.
Declared cultures remain in the manifest and coverage checks even when their
catalog is empty. The file builder accepts `catalog_for_culture` and
`catalog_with_options(CatalogFileOptions)` for explicit culture/format overrides.
Dependency tracking rejects non-UTF-8 or multiline paths rather than registering
a different filename through lossy conversion.

Generated output and manifests are ordered deterministically. The manifest includes
keys, cultures, catalog filenames, placeholder indices, and notification mode.
Build scripts emit `rerun-if-changed` for every supplied input and write outputs
only when their bytes change. Errors prevent output generation; `.deny_warnings(true)`
can enforce a stricter project policy.

Diagnostics retain the source project's identifiers where applicable:

| Code | Meaning |
| --- | --- |
| `PTA001` | Missing static key or generated key constant |
| `PTA002` | Placeholder-set or argument-count mismatch |
| `PTA003` | Missing culture/key coverage |
| `PTA004` | Dynamic key that cannot be validated statically |
| `PTA005` | Invalid culture or Rust source input |
| `PTSG002` | Duplicate translation or key declaration |
| `PTSG003` | Catalog text is not a valid composite format |
| `PTF...` | Catalog import diagnostics, retaining key and culture context |

## Optional Rust usage analysis

```rust,ignore
let options = ferroui_protranslate_build::SourceAnalysisOptions::default()
    .receiver("translations")
    .receiver("self.translations");
let result = ferroui_protranslate_build::Builder::new()
    .catalog("Strings.en-US.json")
    .source("src/view_model.rs", options)
    .generate();
```

The scanner inspects only explicitly named receiver expressions. It recognizes
literal keys, unambiguous string constants, generated constants, and array or
`vec!` argument counts for lookup/format/observe calls, including `format_for`.
Function parameters, closures, loop and match bindings, conditional patterns, and
local variables conservatively shadow file-level constants.

This is syntax analysis, not rustc type analysis: macros remain opaque except for
argument `vec!` syntax, and cross-module/type resolution is not claimed. Tools with
semantic information can submit `KeyUsage` records directly. `CatalogCompiler`
returns generated text, manifest JSON, and diagnostics without writing files.

## Validation

The test suite covers deterministic generation, identity normalization, malformed
inputs, scoped source analysis and actual generated Rust compilation/execution.
The runtime validation exercises typed accessors, escaping, culture fallback,
observable translations, notification delivery, clone identity, and subscription
release against the real core and system crates.
