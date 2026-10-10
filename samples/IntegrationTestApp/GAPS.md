# IntegrationTestApp: gaps

What the port of the sample found the framework to lack or to do differently, and what of the upstream sample is not ported. Every entry below was found by reading the sources of the sample against the sources of the framework; none has been confirmed by a build or a test yet. The status of each is therefore **open: found by reading, to be confirmed by a test in the build phase**. A confirmed gap of the framework gets a minimal reproduction in `tests/gaps.rs`, ignored with its identifier, which passes once the gap is closed.

The 24 documents of the sample are converted from the upstream documents by `scripts/convert_catalog_xaml.py` (names only) and are not edited by hand. A document the compiler of the build refuses fails the build of the crate with its diagnostic: the entries marked "document" name what a document needs.

## Gaps of the framework

| Gap | What the sample does | The upstream statement | What the framework lacks or does differently |
|---|---|---|---|
| I001 (document) | The hide button of the secondary window is bound to a method of its window. | `ShowWindowTest.xaml`: `<Button Name="HideButton" Command="{Binding $parent[Window].Hide}">` | `Window` and `WindowBase` declare no method to markup (`src/FerroUI.Controls/markup_types/classes.rs`: properties, fields and events only), so the path `$parent[Window].Hide` has no member `Hide` to resolve and no delegate to convert to a command. `WindowBase` needs `Hide` (and, as the public methods of the managed original, `Show` and `Activate`; `Window` `Close`) in its markup metadata. |
| I002 (document) | The application, the secondary window and the list box page are their own data contexts, and their documents bind to their members. | `App.xaml.cs`, `ShowWindowTest.xaml.cs`, `Pages/ListBoxPage.xaml.cs`: `DataContext = this;` | An object that is its own data context would hold itself. The port sets an element reference of the object (`ElementRef::of(&this)`, with `ValueTypes::register_element_ref`), as the buttons page of the catalog does. To confirm: that an element reference of a class of an application (not a styled element) is accepted; that the bindings of `App.xaml`, which are bindings of native menu items and of a tray icon, read the application through it; and that the bindings of `ShowWindowTest.xaml`, whose data type is the base class (`x:DataType="Window"`), read the window through an element reference of the derived class. |
| I003 | The screens page shows the text of the platform handle of a screen. | `Pages/ScreensPage.xaml.cs`: `ScreenHandle.Text = screen?.TryGetPlatformHandle()?.ToString();` | The contract of a platform handle (`IPlatformHandle`) has no text form; only the class `PlatformHandle` has one (`Display`). The page writes the text of the class from the descriptor and the value of the handle (`PlatformHandle { NSScreen = 1 }`); a handle of another class of the managed original that overrides its text would differ. |
| I004 | The pointer page shows the control that holds the capture of the pointer. | `Pages/PointerPage.xaml.cs`: `PointerCaptureStatus.Text = captured?.ToString() ?? "None";` | A control has no text form (`Display`). The page writes the full name of the class of the control, which is what the managed original writes for a control (its `ToString()` is the one of `object`). |
| I005 | The list box page exposes its items as a list of texts and binds its list box to it. | `Pages/ListBoxPage.xaml.cs`: `public List<string> ListBoxItems { get; }` | A list a document binds an items source to is declared to markup as a typed list of its crate (`ferro_markup_list!`); the framework declares no list of texts, so the sample declares one (`ListBoxItemList`). To confirm: that no crate the sample is built on declares the same list type (two declarations of ``FerroList`1`` of `String` would be registered twice). |
| I006 | The embedding page hosts a native text view of the platform and opens a native modal window. | `Embedding/MacOSTextBoxFactory.cs`, `Embedding/MacHelper.cs`, `Pages/EmbeddingPage.xaml.cs` (classes of an AppKit binding package: a subclass of the text view, the window, the application object) | The port has no binding of AppKit for applications: `ferroui-native` talks to its own native library and exposes handles (`IPlatformHandle`, `MacOSTopLevelHandle`), not the classes of AppKit. The sample makes the Objective-C runtime calls it needs itself (`Embedding/objc.rs`: a class declared at run time for the text view and one for the delegate of the modal window), as the accessibility test of `ferroui-native` does. The native control host of the port takes the view as a handle with the descriptor `NSView`. To confirm on a desktop: the text view is shown and edited, its mouse methods are called, and the modal session runs and ends. |
| I007 | The headless tests of the sample cannot click into a popup. | not a statement of the sample: `tests/shell.rs` | The windows of the headless shell of the samples (`sample_testing::Shell`) are the windows the application creates through the windowing platform of the test; the popup of a combo box, of a menu or of a `Popup` is a popup of its parent window (`MockWindowImpl::popup`), which the shell neither renders nor sends input to. The tests therefore open a drop down with the mouse but select in it through the control. |

## Differences from the upstream sample

| Where | The upstream statement | The port |
|---|---|---|
| `lib.rs` | `Program.OverlayPopups` (a static property of the entry point, which the main window reads) | The entry point is another target of the crate (`program.rs`), so the value is kept by the library (`overlay_popups()`, `set_overlay_popups()`). |
| `Pages/embedding_page.rs` | `x:Class="IntegrationTestApp.EmbeddingPage"` (the only page whose class is of the root namespace) | The module of the class is declared by the root of the crate, so that the class has the namespace the document names. |
| `Embedding/native_text_box.rs` | `Text = "<the upstream name> ToolTip"` of the tool tip of the native text box | `FerroUI ToolTip`: the name of the upstream project does not appear in the sample. An automation test that reads the text of the tool tip reads this text. |
| `Pages/drag_drop_page.rs` | `{position.X:F0}` (a number with no decimals) | `{:.0}`, which rounds a half to the even neighbour where the managed original rounds it away from zero. |
| `Pages/window_page.rs` | `(WindowStartupLocation)ShowWindowLocation.SelectedIndex` and the two other casts of a selected index | An index that is no member of the enumeration (no item selected) leaves the property as it is, where the managed original stores the number. |
| `Pages/screens_page.rs` | `private Window? _lastWindow;` | The page is in the tree of the window: it holds the window weakly. |
| `Pages/embedding_page.rs` | The native window and its `WillClose` handler are left to the collector. | The page releases the native window and the object that listens to it when the modal session has ended. On a platform other than macOS the handler of the button does nothing. |
| `show_window_test.rs`, `topmost_window_test.rs` | `MacOSIntegration.GetOrderedIndex(this)` every 250 ms on macOS | The same, and it panics for a window without a native window (a null reference in the managed original): a window of a headless test that stays open for 250 ms of dispatcher time would end its test that way. The generated tests close their window at once. |

## Not ported

| Upstream file | Why |
|---|---|
| `Embedding/Win32TextBoxFactory.cs`, `Embedding/Win32WindowControlHandle.cs`, `Embedding/WinApi.cs` | The native text box of the Windows platform, which is not a platform of the port. |
| `Program.cs`: the options of the Windows platform and the developer tools of a debug build | The platform and the tools do not exist in the port. |
| `bundle.sh`, `app.manifest`, the bundle properties of the project file (the bundle name, the bundle identifier, the version) | Packaging: the application bundle the automation driver of macOS launches by its bundle identifier, and the manifest of the Windows executable. The port has no bundling step for a sample yet. |
| The Inter font package the project file references | The entry point of the upstream sample does not use it. |

## Assets

`Assets/icon.ico`, the icon of the tray icon of `App.xaml`, is the artwork of this project (`samples/ControlCatalog/PlaceholderAssets/icon.ico`): the upstream sample links the icon of the upstream project, which carries its brand.

## State after the first build

The table above was written by reading; this is what the build and the tests found.

- I001 is fixed in the framework: the public methods of `WindowBase` without parameters
  (`Activate`, `Hide`, `Show`) are declared for markup (`scripts/markup_types_overrides.py`,
  generated into `src/FerroUI.Controls/markup_types/classes.rs`), so a binding path that ends in
  one is a command, as the compiler of the managed original finds the method by reflection. The
  document compiles and compares with the run-time loader (`compiled_show_window_test`).
- I008, found by the tests and fixed in the framework: the list of pages lost its selection as
  soon as a page was selected. The list box is bound to a list of handles of the pages and its
  selected item to a property of the view model; the binding hands the selected page back as the
  object itself, and the list (`IItemsList::index_of` of a `FerroList<T>`) only found an item in
  the form it boxes its own items in, where upstream's `IList.IndexOf(object)` finds the
  reference. `src/FerroUI.Controls/items_source.rs`:
  `reference_item_tests::a_shared_object_is_found_in_a_list_of_its_handles`; in the sample
  `the_list_of_pages_navigates_with_the_mouse`.
- I007 is narrower now: the popups of the windows of the headless shell render through the
  compositor of the test, so a control that opens one works; the shell still sends no input to a
  popup.
- The native text box of the embedding page and the ordered index of a window talk to the
  Objective-C runtime directly (`Embedding/objc.rs`, `mac_os_integration.rs`): `unsafe` code in
  a sample, which the rules of the port keep to the platform crates. It is kept apart in those
  files and waits for a binding of the platform crate a sample may use (I006).
- Tests: 87 pass (24 documents load and construct, 24 classes compare, the shell selects every
  page and finds the controls the automation tests look for, with the accepted binding report of
  the button page).
