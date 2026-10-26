# ProTranslate demo

Run the native desktop sample:

```sh
cargo run -p sample-protranslate-demo
```

The application uses compiled FerroUI XAML and generated Rust catalogs, with no
runtime JSON discovery. Its English, Polish and Arabic catalogs are validated by
`ferroui-protranslate-build` in `build.rs` and compiled into
`GeneratedTranslationProvider`.

The normal title, subtitle, region details and measurement displays use
`{CompiledBinding ...}` with `x:DataType="local:MainWindowViewModel"`.
The registered viewmodel delegates to generated `ProTranslateStrings` accessors,
forwards their native property notifications, and exposes fallible getters so
evaluation errors remain visible to the binding system. Globalization notifications
also refresh the region and measurement properties when an override changes without
changing the active culture.

The language buttons update existing native bindings immediately. The amount
slider exercises a bound argument to `{pt:F Price, Value={Binding #Amount.Value}}`:
both value changes and culture changes update the formatted text. Arabic also
changes inherited flow direction. The region panel uses the same culture service
for CLDR region metadata and profile-based distance and temperature conversion.

## Integration exercised

| XAML/API | Behavior |
| --- | --- |
| `{CompiledBinding Title}` / `{CompiledBinding Subtitle}` | Typed generated accessors and native change notifications |
| `{CompiledBinding RegionDetails}` / `{CompiledBinding Measurements}` | Culture and override notifications through a typed viewmodel |
| `{pt:T AmountLabel}` | Concise native markup-extension translation |
| `{pt:F Price, Value={Binding #Amount.Value}}` | Native multi-binding with locale-aware numeric formatting |
| `pt:Translation.Key` | Translation on a text/content target |
| `pt:Translation.AutoFlowDirection` | Inherited, culture-driven RTL/LTR |
| `pt:Translation.FallbackValue` | Explicit fallback for a missing catalog key |
| `GlobalizationService` | Region and measurement preferences |
| `TranslationBindingSource` | Live replacement, cache refresh and consumer lifetime |

The sample tests compare compiled and runtime-loaded documents, exercise live
culture/value and region/measurement-override changes, and confirm closing the
window releases its viewmodel and translation consumers:

```sh
cargo test -p sample-protranslate-demo --lib
cargo test -p ferroui-protranslate --lib
```

The owning UI thread creates and mutates the services. A worker that loads a
catalog should post its service update to FerroUI's dispatcher before changing
UI-facing culture state.
