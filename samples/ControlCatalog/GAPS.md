# ControlCatalog: open gaps

The documents of the sample that do not load yet, grouped by what they wait for. `excluded.txt` is the machine-readable list (the generated tests of a listed document are ignored with its reason); this page explains each open entry. Every gap of the framework has a minimal reproduction in `tests/gaps*.rs`, ignored with the same identifier, which passes once the gap is closed:

```sh
cargo test -p control-catalog -- --ignored gap_
```

Status: 219 documents, 207 load and show their class; 12 are listed below (one of them, `App.xaml`, loads in the application and in `tests/gaps.rs`).

## Gaps of the framework

| Gap | What is missing | Reproduction | Documents |
|---|---|---|---|
| C209 | The methods `TextBox.Cut`, `Copy`, `Paste` and `Clear` are not declared for markup, so they cannot be bound as commands (`{Binding $parent[TextBox].Cut}`). | `gaps_b::gap_c209_text_box_methods_as_commands` | `Pages/ContextFlyoutPage.xaml` |
| C310 | `AutoCompleteBox.MinimumPopulateDelay` (`TimeSpan`) is not converted from text (`"00:00:01"`). | `gaps_c::gap_c310_time_span_property_from_text` | `Pages/AutoCompleteBoxPage.xaml` (which also waits for its code-behind) |
| C313 | The item type of a collection of a view model is not known to markup: a view model declares a collection as `ItemsSource`, which has no element type, so a compiled binding of an item template or of `DisplayMemberBinding` without `x:DataType` is resolved against no type ("Unable to resolve property or method of name 'Name'"). Upstream takes the type from the collection the items source is bound to (`InheritDataTypeFromItems`). Found when gaps C201 and C208 were closed. | `gaps_b::gap_c313_item_type_of_a_collection_of_a_view_model` | `Pages/ComboBoxPage.xaml` |

One gap of the framework blocks no document of the list: C101, a reflection binding (`x:CompileBindings="False"`) cannot resolve `$parent[prefix:Type]` for a type of a `using:` namespace (`gaps_a::gap_c101_reflection_binding_parent_of_prefixed_type`). The theme of `SampleGalleryPage` uses such paths with compiled bindings, which resolve them.

## Code-behind that needs framework API the port does not have

| Document | Missing API |
|---|---|
| `Pages/AutoCompleteBoxPage.xaml` | its code-behind (`LogicalExtensions.GetLogicalDescendants`, which it uses, is ported); the document also needs gap C310. |
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
