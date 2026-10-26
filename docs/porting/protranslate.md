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

## Work status

This document opens the port before implementation. The PR will be updated in small
commits with completed mappings, exact validation results, and any remaining differences.
