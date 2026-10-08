# ControlCatalog: open gaps

The documents of the sample that do not load yet, grouped by what they wait for. `excluded.txt` is the machine-readable list (the generated tests of a listed document are ignored with its reason); this page explains each open entry. Every gap of the framework has a minimal reproduction in `tests/gaps*.rs`, ignored with the same identifier, which passes once the gap is closed:

```sh
cargo test -p control-catalog -- --ignored gap_
```

Status: 219 documents, 207 load and show their class; 12 are listed below (one of them, `App.xaml`, loads in the application and in `tests/gaps.rs`).

## Gaps of the framework

No gap of the framework blocks a document of the list. One gap is open: C101, a reflection binding (`x:CompileBindings="False"`) cannot resolve `$parent[prefix:Type]` for a type of a `using:` namespace (`gaps_a::gap_c101_reflection_binding_parent_of_prefixed_type`). The theme of `SampleGalleryPage` uses such paths with compiled bindings, which resolve them.

## Code-behind that needs framework API the port does not have

| Document | Missing API |
|---|---|
| `Pages/AutoCompleteBoxPage.xaml` | its code-behind, which is not ported yet (`LogicalExtensions.GetLogicalDescendants`, which it uses, is ported). |
| `Pages/NumericUpDownPage.xaml` | `CultureInfo.GetCultures(CultureTypes.SpecificCultures)`. |
| `Pages/OpenGl/OpenGlLeasePage.xaml` | `OpenGlFbo` (`Pages/OpenGl/OpenGlFbo.cs`, not ported): it wraps an OpenGL texture in a Ganesh surface (`GRContext.ResetContext` and `Flush`, `GRBackendTexture`, `GRGlTextureInfo`, `SKSurface.Create` over a backend texture). The Skia backend enables the `gl` feature of `skia-safe`, which has these types, for `target_os = "emscripten"` only (`src/Skia/FerroUI.Skia/Cargo.toml`; the desktop build is Graphite on Metal, and no published Skia binary has Graphite and Ganesh together), and `ISkiaGrContext` does not give the Ganesh context (`skia_safe::gpu::DirectContext`) to a lease. The page needs both, and a desktop platform whose leased graphics context is an `IGlContext`. |
| `Pages/ContentPage/ContentPagePerformancePage.xaml`, `Pages/CarouselPage/CarouselPagePerformancePage.xaml`, `Pages/DrawerPage/DrawerPagePerformancePage.xaml`, `Pages/NavigationPage/NavigationPagePerformancePage.xaml`, `Pages/TabbedPage/TabbedPagePerformancePage.xaml` | `NavigationPerformanceMonitorHelper` measures the managed heap (`GC.GetTotalMemory`, `GC.Collect`), which has no counterpart. |

## Classes of the sample that are not ported yet

These documents load without their class (the survey test, `cargo test -p control-catalog -- --ignored survey`, with `CATALOG_SURVEY` set); their code-behind is sample work, not a framework gap:


## Test harness

| Document | Why |
|---|---|
| `App.xaml` | The tray icon of the document loads its icon through the icon loader of the platform, which the unit test application does not have. The document loads in the application, with the styles of the colour picker library it includes; `gaps::the_application_document_loads` loads it with a test icon loader. |
| `Pages/SettingsPage.xaml` | `{x:Static local:App.CurrentTheme}` needs the application of the catalog as the current application. The page loads in the application, which offers it; the unit test application is not that application. |
