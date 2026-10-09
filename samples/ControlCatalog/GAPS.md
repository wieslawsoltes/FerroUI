# ControlCatalog: open gaps

The documents of the sample that do not load yet, grouped by what they wait for. `excluded.txt` is the machine-readable list (the generated tests of a listed document are ignored with its reason); this page explains each open entry. Every gap of the framework has a minimal reproduction in `tests/gaps*.rs`, ignored with the same identifier, which passes once the gap is closed:

```sh
cargo test -p control-catalog -- --ignored gap_
```

Status: 219 documents, 218 load and show their class; 1 is listed below.

## Gaps of the framework

No gap of the framework blocks a document of the list. One gap is open: C101, a reflection binding (`x:CompileBindings="False"`) cannot resolve `$parent[prefix:Type]` for a type of a `using:` namespace (`gaps_a::gap_c101_reflection_binding_parent_of_prefixed_type`). The theme of `SampleGalleryPage` uses such paths with compiled bindings, which resolve them.

No other gap blocks or changes a page. One more is open for the memory of a visit (below): a named root and its name scope hold each other (`gaps_d::open_named_root_of_a_document`).

## What a visit to a page retains

There is no collector: a page that left the navigation page is freed when the last reference to it is dropped, and a cycle of references is never freed. `tests/catalog_tour.rs` measures it on the desktop, with the counting allocator of the tests (`--features count-allocations`), one measurement at a time:

```sh
cargo test -p control-catalog --features count-allocations --lib catalog_tour_memory -- --ignored --nocapture --test-threads=1
cargo test -p control-catalog --features count-allocations --lib catalog_revisit_memory -- --ignored --nocapture --test-threads=1
```

The first selects every page of the list (74: all but the settings page, which needs the application of the catalog) with the transition and rendered frames, tour after tour, and prints what is alive at the end of each tour. The second prints, per page, what a second and a third visit add. `CATALOG_TOUR_TRACE` records where the blocks a visit leaves were allocated and prints them by class (with the strong references to each object) and by allocating function; `CATALOG_TOUR_TARGET` prints every block that points into the blocks of one class, which is how the cycles below were found (the module documentation has the variables).

| | first tour | each further tour | objects of the server compositor per tour |
|---|---|---|---|
| before (2026-10-09, base `compositor-object-id-reuse`) | 197.8 MB alive | +143.0 MB (1.9 MB a page, 1 344 000 blocks) | +4391 |
| after C317 to C320 | 125.9 MB alive | +71.0 MB (0.96 MB a page, 667 000 blocks) | +1750 |

Closed, each with a reproduction in `tests/gaps_d.rs` and its entry in `docs/porting/DEVIATIONS.md` (every one is a cycle of references that upstream has too and leaves to its collector):

| Gap | What kept what alive | Fix |
|---|---|---|
| C317 | The deferred content of a template held the root of its document, the resource nodes above the declaration and the name scope: an element with a template declared under it (the home page). | The deferred content holds them weakly. |
| C318 | The node of a binding of `DataContext` held the parent of its element (the sections of the home page). | The node holds the parent weakly. |
| C319 | A binding that locates an element (`#name`, `$parent`, the templated parent) held the element it found, and the tracker of an ancestor held the element it searches from (the presenter of every scroll viewer; every element with a `$parent` binding). | The nodes hold their value weakly, the trackers their element and ancestor. |
| C320 | A binding entry and the observable it was subscribed to held each other after the value store was dropped (the content of every scrolled page: the Calendar page went from 29 MB a visit to 0.9 MB). | The observers hold the entry weakly; an entry dropped while subscribed leaves its source. |

What remains, from the largest (third visit of the revisits, after the fixes): Buttons 7.7 MB, NumericUpDown 6.5 MB, Flex Panel 5.5 MB, CalendarDatePicker 4.3 MB, AutoCompleteBox 3.5 MB, ComboBox 3.0 MB, Date/Time Picker 2.4 MB, ProgressBar 2.3 MB, Accelerator 2.1 MB; the smallest pages leave about 0.1 MB, which is what the round trip through the home page itself leaves. Known and open:

- **A named root.** The name scope of a document holds the elements it names and the root holds the scope, so a root with a name of its own keeps itself alive (`gaps_d::open_named_root_of_a_document`, ignored). Upstream has both references. Holding the names weakly would change what a name scope returns for an element that left the tree; the scope needs to know its owner instead.
- **Pages that are not freed.** The Buttons page object is still alive after the catalog navigated away (the home page, Border, Calendar and TextBlock pages are freed: `catalog_tour::the_catalog_frees_the_page_it_navigated_away_from`). The holder is not identified; the recording of the page shows its whole tree alive.
- **The spine of the home page.** After a round trip the content presenter of the page theme, the panel under it and the banner images are alive with one strong reference to the presenter; the holder is not identified.
- **Server objects.** 1750 objects of the server compositor per tour are the visuals of the elements that are still retained; no table of the compositor was seen to grow on its own, and the ids are reused (the fix of the base branch). To be measured again once the elements are freed.
- **Dropped objects whose memory is held weakly.** The two path icons the navigation page creates for its back button per navigation are dropped, and weak references keep their blocks (about 1.5 KB each).
- **Code of the sample, as upstream has it.** `ScreenPage` subscribes to the position of its window and to the screens and never unsubscribes; upstream's page does the same.

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

No document is listed for the test harness. Two documents need more than the unit test application of the framework gives a test by default, and the harness provides it:

| Document | What its generated tests need | How they get it |
|---|---|---|
| `App.xaml` | The tray icon of the document loads its icon through the icon loader of the platform. | The test services carry an icon loader (`TestServices::with_icon_loader`, `ferroui_controls::testing::TestIconLoader`; upstream's `TestServices` has none), and the test services of the catalog register it (`tests/support.rs`). |
| `Pages/SettingsPage.xaml` | `{x:Static local:App.CurrentTheme}` reads the current application as the application of the catalog (upstream: `((App)Current!)._prevTheme`), and the theme selection of the page sets the themes of that application. | `test_applications.txt` names the application the generated tests of a document start. The page is listed with `catalog`: the application of the catalog (`App`) is started as the application of the test, with the test services in place of the services of a platform (`UnitTestApplication::start_with`, `support::start_catalog_app`); it loads `App.xaml` and applies the Fluent theme. Every other document has the default, the unit test application with the Simple theme and the resources of `CustomThemes.xaml`. |
