# ControlCatalog: open gaps

The documents of the sample that do not load yet, grouped by what they wait for. `excluded.txt` is the machine-readable list (the generated tests of a listed document are ignored with its reason); this page explains each open entry. Every gap of the framework has a minimal reproduction in `tests/gaps*.rs`, ignored with the same identifier, which passes once the gap is closed:

```sh
cargo test -p control-catalog -- --ignored gap_
```

Status: 219 documents, 218 load and show their class; 1 is listed below.

## Gaps of the framework

No gap of the framework blocks a document of the list. One gap is open: C101, a reflection binding (`x:CompileBindings="False"`) cannot resolve `$parent[prefix:Type]` for a type of a `using:` namespace (`gaps_a::gap_c101_reflection_binding_parent_of_prefixed_type`). The theme of `SampleGalleryPage` uses such paths with compiled bindings, which resolve them.

No other gap blocks or changes a page, and none is open for the memory of a visit (below).

## What a visit to a page retains

There is no collector: a page that left the navigation page is freed when the last reference to it is dropped, and a cycle of references is never freed. `tests/catalog_tour.rs` measures it on the desktop, with the counting allocator of the tests (`--features count-allocations`), one measurement at a time:

```sh
cargo test -p control-catalog --features count-allocations --lib catalog_tour_memory -- --ignored --nocapture --test-threads=1
cargo test -p control-catalog --features count-allocations --lib catalog_revisit_memory -- --ignored --nocapture --test-threads=1
cargo test -p control-catalog --features count-allocations --lib catalog_markup_survivors -- --ignored --nocapture --test-threads=1
```

The first selects every page of the list (74: all but the settings page, which needs the application of the catalog) with the transition and rendered frames, tour after tour, and prints what is alive at the end of each tour. The second prints, per page, what a second and a third visit add. The third shows the markup of `CATALOG_TOUR_XAML` in the window of the catalog, takes it out and prints the elements that are still alive: a page is narrowed down to the control and the shape that stays. `CATALOG_TOUR_TRACE` records where the blocks a visit leaves were allocated and prints them by class (with the strong references to each object) and by allocating function; `CATALOG_TOUR_TARGET` prints every block that points into the blocks of one class, with how many of the pointers are weak references of the object model (the feature marks them, `ferroui-base/tagged-weak-references`), the holders with other pointers first, which is how the causes below were found (the module documentation has the variables).

| | first tour | each further tour | objects of the server compositor per tour |
|---|---|---|---|
| before (2026-10-09, base `compositor-object-id-reuse`) | 197.8 MB alive | +143.0 MB (1.9 MB a page, 1 344 000 blocks) | +4391 |
| after C317 to C320 | 125.9 MB alive | +71.0 MB (0.96 MB a page, 667 000 blocks) | +1750 |
| after C321 to C330 | 55.6 MB alive | +56 KB (0.8 KB a page, 439 blocks) | +8 |

The last row is measured with the weak references marked (a weak reference is one word larger); the second tour adds 248 KB, the rest of what the first one did not fill, and the tours after it 55 to 66 KB each, six tours measured. The revisits: every page adds nothing on its third visit but the three below.

Closed, each with a reproduction (`tests/gaps_d.rs`; C323 in `tests/catalog_tour.rs`, it needs a compiled template) and its entry in `docs/porting/DEVIATIONS.md`. Every one but C330 is a cycle of references that upstream has too and leaves to its collector; C330 is a list upstream bounds:

| Gap | What kept what alive | Fix |
|---|---|---|
| C317 | The deferred content of a template held the root of its document, the resource nodes above the declaration and the name scope: an element with a template declared under it (the home page). | The deferred content holds them weakly. |
| C318 | The node of a binding of `DataContext` held the parent of its element (the sections of the home page). | The node holds the parent weakly. |
| C319 | A binding that locates an element (`#name`, `$parent`, the templated parent) held the element it found, and the tracker of an ancestor held the element it searches from (the presenter of every scroll viewer; every element with a `$parent` binding). | The nodes hold their value weakly, the trackers their element and ancestor. |
| C320 | A binding entry and the observable it was subscribed to held each other after the value store was dropped (the content of every scrolled page: the Calendar page went from 29 MB a visit to 0.9 MB). | The observers hold the entry weakly; an entry dropped while subscribed leaves its source. |
| C321 | The root of a document holds its name scope and the scope the elements it names: a root with a name of its own. | The scope holds the element it is attached to weakly, every other name as before. |
| C322 | `DataValidationErrors.Owner` held the control the errors are shown in, and the themes make the owner the `DataContext` of a part of the errors template: every control with validation in its template (TextBox, ComboBox, NumericUpDown, CalendarDatePicker, AutoCompleteBox, Slider, the pickers: NumericUpDown 6.5 MB a visit, Flex Panel 5.5, CalendarDatePicker 4.3, AutoCompleteBox 3.5, ComboBox 3.0). | The owner is an element reference (`ElementRef<Control>`), and an element reference that is a value is the element to a binding. |
| C323 | The type resolver of a reflection binding held the context of the build, with the root object, the parents and the name scope; the instance of a multi binding holds its bindings: the tree of a compiled template with such a binding (the text box of the Fluent theme). | The resolver holds the type resolver service of the document. |
| C324 | The disposable of a routed event handler held the element the handler was added to: an element that keeps the disposable of its own handler (every slider). | The disposable holds the element weakly. |
| C325 | A dynamic resource extension held its anchor, the element the style it is a setter value of is declared under (the text box of the CalendarDatePicker of the Fluent theme). | The extension keeps its anchor as its expressions do: an element or a host weakly. |
| C326 | The Buttons page of the sample is its own data context (`DataContext = this`): 7.7 MB a visit. | The page sets an element reference to itself; the bindings of its document read the page through it. |
| C327 | The observer of a local value binding and its source held each other after the object was dropped, with the memory of the object: every text block of the default data template (Calendar 866 KB a visit, TableView 243 KB). | The observers the source holds hold the binding weakly; a binding dropped with its value store leaves its source. |
| C328 | A view model of the sample keeps an observable of its own properties (`this.WhenAnyValue(..)`), which held the model: the list box page with its 10 000 items (343 KB a visit). | The observable of MiniMvvm holds the model weakly. |
| C329 | The path of a compiled binding to a named element held the name scope: the binding of a setter of a style under a named element (the Focus page, 751 KB a visit). | The path holds the scope weakly, as the node it creates always has. |
| C330 | A visual subscribes to the values of its render-affecting properties, and a value many visuals share kept an entry, and the memory of the visual, for every visual that ever drew it: the geometry of the back button of the navigation page (6 KB for every navigation), the icons of spinners and combo boxes. Upstream subscribes with a weak event whose list is compacted. | A visual that is dropped takes its handlers out. |

What the earlier list had open is closed with these: the named root (C321); the Buttons page (C326); the spine of the home page, which the round trip through the home page no longer leaves (every page but the three below adds nothing on its third visit); the dropped path icons whose memory weak references held (C330). The objects of the server compositor were the visuals of the elements that stayed alive, as supposed: with the elements freed a tour adds 8, the visuals of the two notification managers below, and no table of the compositor grows on its own.

What a tour still adds is the code of three pages of the sample, which upstream has the same way; the window holds what they leave for as long as it lives, in both implementations, so nothing is changed (`docs/porting/DEVIATIONS.md`, ControlCatalog sample):

| Page | A visit leaves | Why |
|---|---|---|
| Notifications | 22 KB, 191 blocks, 4 objects of the server compositor | `OnAttachedToVisualTree` creates a `WindowNotificationManager` for the top level, which installs itself in the adorner layer of the window and is never removed. |
| Clipboard | 22 KB, 191 blocks, 4 objects of the server compositor | The same. |
| Screens | 11 KB, 57 blocks | `OnAttachedToVisualTree` subscribes to the position of the window and to the changes of its screens, with handlers that hold the presenter of the page, and never unsubscribes. |

Caches that the first visit to a page fills and no later visit grows are not in these figures: the themes and control themes instantiated on first use, the parsed documents, fonts and glyphs (the first tour).

What the measurements do not exercise: a tour selects the pages and renders them; it does not move the pointer or the focus, open a popup, a flyout or a tooltip, or scroll. What those leave is not measured here (the session over the built site, `scripts/browser/catalog-memory.mjs`, scrolls and opens the demos).

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
