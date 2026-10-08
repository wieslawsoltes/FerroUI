# ControlCatalog: open gaps

The documents of the sample that do not load yet, grouped by what they wait for. `excluded.txt` is the machine-readable list (the generated tests of a listed document are ignored with its reason); this page explains each open entry. Every gap of the framework has a minimal reproduction in `tests/gaps*.rs`, ignored with the same identifier, which passes once the gap is closed:

```sh
cargo test -p control-catalog -- --ignored gap_
```

Status: 219 documents, 216 load and show their class; 3 are listed below (one of them, `App.xaml`, loads in the application and in `tests/gaps.rs`).

## Gaps of the framework

No gap of the framework blocks a document of the list. One gap is open: C101, a reflection binding (`x:CompileBindings="False"`) cannot resolve `$parent[prefix:Type]` for a type of a `using:` namespace (`gaps_a::gap_c101_reflection_binding_parent_of_prefixed_type`). The theme of `SampleGalleryPage` uses such paths with compiled bindings, which resolve them.

## Code-behind that needs framework API the port does not have

| Document | Missing API |
|---|---|
| `Pages/OpenGl/OpenGlLeasePage.xaml` | `OpenGlFbo` (`Pages/OpenGl/OpenGlFbo.cs`, not ported): it wraps an OpenGL texture in a Ganesh surface (`GRContext.ResetContext` and `Flush`, `GRBackendTexture`, `GRGlTextureInfo`, `SKSurface.Create` over a backend texture). The Skia backend enables the `gl` feature of `skia-safe`, which has these types, for `target_os = "emscripten"` only (`src/Skia/FerroUI.Skia/Cargo.toml`; the desktop build is Graphite on Metal, and no published Skia binary has Graphite and Ganesh together), and `ISkiaGrContext` does not give the Ganesh context (`skia_safe::gpu::DirectContext`) to a lease. The page needs both, and a desktop platform whose leased graphics context is an `IGlContext`. |

## Pages that load with less than upstream shows

| Document | What differs |
|---|---|
| `Pages/NumericUpDownPage.xaml` | The combo box "CultureInfo" lists the specific cultures of the runtime with one of six names. The base library has the data of the invariant culture only; the cultures of the runtime are the ones the registered culture data provider enumerates (`CultureInfo::get_cultures`, `ICultureDataProvider::get_culture_names`). No platform registers a provider yet, so in the application the list is empty and the values are formatted with the invariant conventions, as upstream's page under invariant globalization (its comment: "Trimmed-mode friendly where we might not have cultures"). With a provider the page lists and uses its cultures (`tests/numeric_up_down_page.rs`). |
| `Pages/ContentPage/ContentPagePerformancePage.xaml`, `Pages/CarouselPage/CarouselPagePerformancePage.xaml`, `Pages/DrawerPage/DrawerPagePerformancePage.xaml`, `Pages/NavigationPage/NavigationPagePerformancePage.xaml`, `Pages/TabbedPage/TabbedPagePerformancePage.xaml` | Upstream's pages measure the managed heap (`GC.GetTotalMemory`, `GC.GetTotalAllocatedBytes`) and force collections (`GC.Collect`). There is no garbage-collected heap, and the sample has no statistics of the allocator. The pages show what can be measured: the pages that are alive (weak references), the pages created, the depth of the stack, the time of the last operation and the log. The heap texts read "not available", the heap delta is empty, the entries of the log have no heap size, and "Force GC" refreshes the figures (a page is freed when the last reference to it is dropped, so there is nothing to collect). The texts the code-behind writes about the garbage collector, and the descriptions of the five gallery entries, say what happens here; the texts of the documents are upstream's and still name the heap and the collector. A heap figure needs a counting global allocator in the hosts (or the statistics of the platform allocator), which is a decision for the application, not for a page. |

## Classes of the sample that are not ported yet

These documents load without their class (the survey test, `cargo test -p control-catalog -- --ignored survey`, with `CATALOG_SURVEY` set); their code-behind is sample work, not a framework gap:


## Test harness

| Document | Why |
|---|---|
| `App.xaml` | The tray icon of the document loads its icon through the icon loader of the platform, which the unit test application does not have. The document loads in the application, with the styles of the colour picker library it includes; `gaps::the_application_document_loads` loads it with a test icon loader. |
| `Pages/SettingsPage.xaml` | `{x:Static local:App.CurrentTheme}` needs the application of the catalog as the current application. The page loads in the application, which offers it; the unit test application is not that application. |
