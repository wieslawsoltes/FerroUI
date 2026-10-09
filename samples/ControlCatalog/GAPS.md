# ControlCatalog: open gaps

The documents of the sample that do not load yet, grouped by what they wait for. `excluded.txt` is the machine-readable list (the generated tests of a listed document are ignored with its reason); this page explains each open entry. Every gap of the framework has a minimal reproduction in `tests/gaps*.rs`, ignored with the same identifier, which passes once the gap is closed:

```sh
cargo test -p control-catalog -- --ignored gap_
```

Status: 219 documents, 218 load and show their class; 1 is listed below.

## Gaps of the framework

No gap of the framework blocks a document of the list. One gap is open: C101, a reflection binding (`x:CompileBindings="False"`) cannot resolve `$parent[prefix:Type]` for a type of a `using:` namespace (`gaps_a::gap_c101_reflection_binding_parent_of_prefixed_type`). The theme of `SampleGalleryPage` uses such paths with compiled bindings, which resolve them.

No other gap blocks or changes a page, and none is open for the memory of a visit, with or without interaction (below).

## Bindings that report an error

A binding that fails says so in the log (the area `Binding`, at the level `Warning`, which the desktop host prints), and its value never arrives. `tests/binding_reports.rs` visits every page as the drawer selects it (the tour of `tests/catalog_tour.rs`: 74 pages under the Fluent theme, with the layout, the transition and rendered frames) and collects the reports by page; a report that is not in its list of accepted ones fails the test, and so does an accepted one that is no longer made:

```sh
cargo test -p control-catalog --lib the_bindings_of_the_catalog
```

The first run reported 32 errors on eight pages, after C401 (each progress bar of the catalog reported two more before it). Twenty-two were defects of the port, closed with the gaps below, each with a reproduction in `tests/gaps_d.rs`:

| Gap | Pages (reports) | The report | The defect and its fix |
|---|---|---|---|
| C401 | every progress bar under the Fluent theme (two each) | `Could not find a matching property accessor for 'TemplateSettings' on 'ProgressBar'` | `ProgressBar.TemplateSettings` is a plain property upstream (`public ProgressBarTemplateSettings TemplateSettings { get; }`), and the class declared it for its own code only. The control themes bind the sizes of the indeterminate indicators and the key frames of their animations to its members (`{Binding $parent[ProgressBar].TemplateSettings.ContainerWidth}`). The class declares the property in its markup metadata (`progress_bar.rs`), so reflection bindings and compiled bindings read it; the indicators have their sizes and move (`src/FerroUI.Themes.Fluent/tests/progress_bar_tests.rs`). |
| C402 | CalendarDatePicker (10) | `TextBlock.Text <- Day: Could not find a matching property accessor for 'Day' on 'System.DateTime'` | The control theme of the calendar date picker writes the day of today into its button (`{Binding Source={x:Static sys:DateTime.Today}, Path=Day}`), and the metadata of `DateTime` had its static members only. It declares the components of a date as `System.DateTime` has them (`Date`, `Day`, `DayOfWeek`, `DayOfYear`, `Hour`, `Kind`, `Millisecond`, `Minute`, `Month`, `Second`, `Ticks`, `TimeOfDay`, `Year`). |
| C403 | Data Validation (2) | `TextBox.ErrorConverter <- Converter: Could not convert 'ErrorConverter' to 'Option<ErrorConverter>'` | `DataValidationErrors.ErrorConverter` is `Func<object, object>?`: the property holds the nullable form of the delegate, which no conversion produced from the delegate a binding delivers. The controls crate registers the nullable form (`markup_types/mod.rs`). |
| C404 | Expander (8) | `Expander.CornerRadius <- CornerRadius: Could not convert 'Rc<dyn AnyValue>' to 'CornerRadius'` | The view model of the page has a property of type `object` (a corner radius, or the unset value). A compiled binding whose path is one plain property is built from the typed accessors of the declaration, and for a property of the untyped value type that element delivered the box of the value as the value. Such a property has no typed element (`metadata/typed_path.rs`): the untyped one delivers the value it holds. |
| C405 | TabControl (1), TreeView (1) | `ComboBox.SelectedIndex <- TabPlacement: Could not convert 'Top' (Dock) to 'i32'` | The pages bind the selected index of a combo box to a property of an enumeration, in both directions. `TypeUtilities.TryConvert` converts an enumeration to a number and a number to an enumeration; `ValueTypes::try_convert` did neither. It does now (`data/core/value_type.rs`, with the numeric value of a value of an enumeration in its metadata, `MarkupType::enum_to_value`). |
| C406 | none of the tour (the items are in context menus, which the tour does not open) | `Could not find a matching property accessor for 'LineDown' on 'ScrollBar'` when the menu opens | The control themes bind the items of the context menu of a scroll bar, the scroll buttons of the menu scroll viewer and the copy item of the selectable text block to public methods of the controls (`Command="{Binding $parent[ScrollBar].LineDown}"`), and the classes declared no method for markup: the items did nothing. Found by the audit of the binding paths of the themes (34 paths). `ScrollBar`, `ScrollViewer` and `SelectableTextBlock` declare their public methods. |

Ten reports are accepted: the upstream sample makes them too. Its documents are the same, and a null in the middle of a path is an error of the binding upstream (`ExpressionNode.ValidateNonNullSource` reports "Value is null." through `BindingExpression.OnNodeError`, which logs it):

| Page | Reports | Why |
|---|---|---|
| ComboBox | `TextBlock.Text <- SelectedItem.Name at SelectedItem: Value is null.` (1) | `Pages/ComboBoxPage.xaml` shows the name of the selected item of its view model (`{Binding SelectedItem.Name, StringFormat=Selected Item: {0}}`), and `ComboBoxPageViewModel.SelectedItem` is null until an item is selected. |
| Flex Panel | `... <- SelectedItem.IsVisible`, `AlignSelfItem`, `Order`, `Shrink`, `Grow`, `BasisKind`, `BasisValue`, `HorizontalAlignment`, `VerticalAlignment` `at SelectedItem: Value is null.` (9, one each) | `Pages/FlexPage.xaml` edits the selected item of the panel with nine bindings to members of `SelectedItem`, and `FlexViewModel.SelectedItem` is null until an item of the panel is pressed. |

No report of the tour is a binding evaluated before its data context arrives: a binding whose data context is null reports nothing, as upstream. The settings page is not part of the tour (it needs the application of the catalog).

The same class of defect as C401 and C402 (a path of a theme that names a member its type does not declare) is audited where it can be found without running a page: `theme_binding_paths` of the XAML test crate resolves every binding path of the documents of the two themes, of the colour picker and of the dialogs against the declared members (`docs/porting/xaml.md`, 3.7.1).

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

What these measurements do not exercise: a tour selects the pages and renders them; it does not move the pointer or the focus, open a popup, a flyout or a tooltip, or scroll. The tours with interaction do (the next section).

## What interaction retains

A tour selects the pages and renders them. `tests/interaction_tour.rs` adds the input: every page a tour shows is driven through the raw input of the window and of its popups, as a platform backend delivers it, in a fixed order and by the manual clock, and what is alive is read as the tours read it. `tests/interaction_controls.rs` does the same control by control, independent of the catalog.

```sh
cargo test -p control-catalog --features count-allocations --lib catalog_interaction_tour_memory -- --ignored --nocapture --test-threads=1
cargo test -p control-catalog --features count-allocations --lib catalog_interaction_revisit_memory -- --ignored --nocapture --test-threads=1
cargo test -p control-catalog --features count-allocations --lib catalog_interaction_survivors -- --ignored --nocapture --test-threads=1
cargo test -p control-catalog --features count-allocations --lib interaction_control_survivors -- --ignored --nocapture --test-threads=1
cargo test -p control-catalog --features count-allocations --lib interaction_repeat_memory -- --ignored --nocapture --test-threads=1
```

The first two are the tours and the revisits with interaction; the third prints the elements of a page that are alive after the catalog left it, with the pointer and the focus where the visit left them; the fourth and the fifth are the measurements of the controls (below). The variables of the tours apply; `CATALOG_TOUR_POPUPS=overlay` hosts the popups in the overlay layer of the window, as a platform without popup windows does, and `CATALOG_TOUR_VERBOSE` prints what was driven on each page.

### What a visit drives

On every page, in this order: the pointer over a grid of the window 40 pixels apart (640 moves); the pointer resting on the elements with a tooltip, with the timers of the dispatcher fired until the tooltip opens and after the pointer left; Tab 24 times and Shift+Tab 4 times; the wheel over the scroll viewers, 30 notches down and 30 up; a press with a small drag on the focusable elements that are visible, enabled and hit at their centre (at most 48: a click on a button, a drag on a slider, an open drop-down on a combo box, a picker, a flyout button or a menu item), with typing, Backspace and undo in a text box; a popup that opened has the pointer moved over it and its middle pressed (an item of a drop-down is selected, a sub menu opens) and is closed with Escape, or by a press in a corner of the window when Escape did not close it; the right button on the elements with a context menu or a context flyout; Escape, and the timers once more. A window a page opens is closed again. The card of a gallery that a press activates pushes or shows its sample, which is then driven in place of the cards. One tour is 47 360 pointer moves, 2 887 keys, 10 080 wheel notches, 658 presses (64 with typing), 99 context menus, 173 popups opened and one window.

What it does not drive: touch and pen on the pages (a control-level scenario has touch), gestures of more than one pointer, drag and drop between elements, the input method client (the platform of the tests has none), real time (an animation runs by the frames of the manual clock, a timer fires when the visit fires it), more than the first sample of a gallery, more than the 48 first focusable elements of a page and nothing outside its viewport, and the button "Select Random" of the TreeView page (its command indexes ten children of a node, as upstream's does, and panics after "Remove" was pressed; upstream throws there). The generators the view models create without a seed are seeded in the measurement, so that a visit is the same every time.

### The tours with interaction

| | first tour | each further tour | objects of the server compositor per tour |
|---|---|---|---|
| without interaction (after C321 to C330, above) | 55.6 MB alive | +56 KB (439 blocks) | +8 |
| with interaction, at the start (2026-10-09, base `catalog-revisit-memory-2`) | not reached: the first press on an item of a drop-down panics in the tooltip service, and a press on the SplitView page overflows the stack | | |
| with the two fixed, the keyboard navigation of the application missing from the tours | 92.8 MB alive | +83 to +86 KB (673 blocks) | +8 |
| with the keyboard navigation, before C331 | 93.0 MB alive | +2.25 to +2.36 MB (2 998 blocks) | +8 |
| after C331 | 93.0 MB alive | +83 to +86 KB (673 blocks) | +8 |

The second tour adds 2.2 MB, what the first one did not fill; six tours measured, with the popups as popups of the platform. The revisits, in both hostings of the popups: every page adds nothing on its third visit but the four below, and the elements of every page are freed when the catalog leaves it (`catalog_interaction_survivors`: nothing alive but the presenter of the Screens page; `the_catalog_frees_the_page_it_interacted_with` asserts it for fifteen pages).

What an interactive tour still adds is the 673 blocks of four pages, each the code of the sample as upstream has it:

| Page | A visit leaves | Why |
|---|---|---|
| Composition | 27 KB, 202 to 208 blocks | The buttons "Create and animate" start key frame animations that run forever on the gradient stops and the brushes they create. Upstream's server compositor holds a running animation in its clock (`ServerCompositorAnimations._clockItems`), a brush and a gradient stop are activated when they are created (`CompositionBrush.InitializeDefaultsExtra`, `Server.Activate()`) and nothing deactivates them: the animation of a brush nothing draws any more runs on, in both implementations. Without a press the page leaves nothing. |
| Notifications | 23.5 KB, 213 blocks, 4 objects of the server compositor | As without interaction: the `WindowNotificationManager` of every attachment. |
| Clipboard | 22.3 KB, 193 blocks, 4 objects of the server compositor | The same. |
| Screens | 10.8 KB, 59 blocks | As without interaction: the subscriptions of the page to its window. |

### What the tours with interaction found

| | What happened | Fix |
|---|---|---|
| Tooltip service | A press that closes the window it was delivered to (an item of a drop-down, whose popup closes) is still handed to the tooltip service with that window as its root; the service read the root element of the closed root and panicked. Upstream reads a null `RootElement` there. | `IInputRoot::try_root_element`, `None` once the root has closed; the service compares that (`tool_tip_tests`). |
| Property values that are not a number | The value store and the transitions compared with `==`, to which NaN differs from NaN; upstream compares with `EqualityComparer<T>.Default` and `object.Equals`, to which they are equal. A width that is not set was a new base value every time it was set again: the pane of the split view, whose width has a transition, answered a width that was unset while the transition ran with one transition after the other until the stack overflowed. | `value_equals`: `==`, and NaN with NaN (`animatable_tests`). |
| C331 | The server visual of an adorner subscribes to the transforms of the visual it adorns; the entry of a disposed adorner stayed in the list of the adorned visual, with the memory of the adorner's server visual. A control kept one for every focus adorner it ever had: 0.9 KB each time the keyboard moved the focus to it, for as long as it lived (fifty moves over four buttons: +45 KB). | The disposal of a server visual takes its entry out (`interaction_controls::gap_c331_focus_adorners_of_a_control_the_keyboard_focuses_again_and_again`). |

C331 is in `docs/porting/DEVIATIONS.md` (Server composition visuals), the two others under Input and Property system; a page of the catalog is freed with its controls, so the tours show C331 only through the controls of the main view, which stay.

### Control by control

`tests/interaction_controls.rs` shows a control in the window of the tours, drives it, takes it out of the tree and drops it; nothing of it may be alive afterwards: its template, the containers it realized, what its popups showed. Twenty-one scenarios, each with the popups as popups of the platform and in the overlay layer (42 tests that run by default): a combo box (opened, an item pressed, Escape; removed while open; 2 000 items scrolled by the wheel), the calendar date picker, the date and the time picker, flyouts and a menu flyout, a menu with a sub menu, a context menu and a context flyout, a tooltip (and one removed while open), a list box and an items control with 10 000 items scrolled from end to end by the offset, the wheel and the keys (at most 40 containers at any moment), a text box with 300 characters typed, undo, redo and a selection, removed with the focus, a slider removed while dragged, an auto complete box removed with its drop-down open, buttons under Tab and an access key, a repeat button removed while held, transitions removed while they run, a tab control and a tree view, a scroll viewer by wheel, thumb and track, and touch (a tap, a hold that opens a context menu, a pan removed with the finger down). Every control is freed. `interaction_control_survivors` runs each scenario three times in one window: the third run adds no block. `interaction_repeat_memory` repeats fifteen interactions on a control that stays in the tree, ten times and twenty more: nothing grows after C331.

Three holders keep an element for a while after it left the tree, here as upstream, and the scenarios account for them:

| Holder | What it holds | For how long |
|---|---|---|
| The timer of the hold gesture (`Gestures.PointerPressed`, `DispatcherTimer.RunOnce`) | The arguments of the press, with the element that was pressed. | Until the timer fires, the time a hold takes to begin. The scenarios fire the pending timers before they count. |
| The input method manager of the keyboard device (`TextInputMethodManager._focusedElement`) | The item of a drop-down that closed while the focus was moving to it: the box that lost the focus closes the drop-down, the keyboard device clears the focus, and the change that was under way ends with `_textInputManager.SetFocusedElement(element)` for the item. | Until the focus changes again. One element. |
| The context flyout of the text boxes of the Fluent theme | Its presenter, with three menu items: the flyout is one resource of the theme that every text box shows. | As long as the theme. One presenter (`the_context_flyout_of_text_boxes_is_shared`). |

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

## Defects of the sample that no test saw

Not gaps of the framework: code of the sample that was ported wrong, found by looking at what the shell shows (`tests/shell.rs`).

| Where | What was wrong | Why no test saw it |
|---|---|---|
| `Controls/section_control.rs` | The constructor of `SectionControl` did not populate the control from its document (`InitializeComponent()` in upstream's constructor), so the control had no content: the home page showed the titles of the sections with nothing under them, and the page of a section was empty under its header. The pages and the drawer were not affected: the drawer has its own templates in `MainView.xaml`. | The generated tests construct the class and show it, and an empty user control shows; the comparison of the compiled markup with the run-time loader populates an instance the constructor did not run for (`create_uninitialized`), in both paths; the tours count the pages they visit, not what a page shows; and no binding fails in a control without content, so nothing was logged. `tests/shell.rs` now asserts the cards under each section title, on the home page and on the section pages, at 1400x900 and 800x600, and that a pressed card opens its page; `every_class_with_a_document_populates_itself` fails for a class that declares a document and never calls `initialize_component`. |

What `tests/shell.rs` asserts besides: the entries of the drawer and their selection, the section a click opens and the pages it lists, the search box (the query filters the drawer, a result opens its page, clearing it restores the list), the button of the bar (it toggles the drawer on a root page and goes back from a sample a gallery pushed) and the header the bar shows, the hovered and the pressed card, the settings page (the theme variant and the theme of the catalog follow its combo boxes, and the home page shows its cards under each), the compact drawer, and the keyboard (Tab through the entries, Enter and Space). One behaviour is upstream's and is asserted as it is: an entry of the drawer that runs its command from the keyboard loses the focus, because `MiniCommand` cannot execute while it executes and a disabled element gives the focus up.
