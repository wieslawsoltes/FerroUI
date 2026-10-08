# Deviations from upstream

The port is exact: the same architecture, contracts, members and behaviour as upstream, file by file. This page is the register of every place where the Rust code nonetheless differs from the upstream code it ports. Each entry says what differs, why, and whether a user of the framework can observe it. A deviation that is not here is a bug to fix or to record.

Other pages hold the entries of their own area, and this page does not repeat them:

- `browser-platform.md` section 14: the browser backend and the browser hosts of the samples (host-level differences, upstream quirks not copied).
- `xaml.md`: the markup pipeline and the ahead-of-time compiler.
- `PORTING-GUIDE.md`: the systematic mappings that apply everywhere (class model, nullability, events, generics, naming). An application of those rules is not a deviation and is not listed here.

## How to record a deviation

- Add a row to the table of the area, in the same pull request as the code. The pull request lists the deviation as well, as `CLOUD-WORKERS.md` asks.
- Put a comment at the site in the code, naming the upstream member and the difference, for example: `// Deviation (DEVIATIONS.md, Layout): upstream uses Stopwatch; ...`.
- Classify the entry:
  - **Representation**: Rust needs a different form (ownership, no `unsafe`, no array pools, unsigned lengths) and nothing observable changes.
  - **Behaviour**: something a user of the framework can observe differs: a value, an order, an exception, a log message or a timing.
  - **Missing**: an upstream step the port leaves out. It is either waiting for a member that is not ported yet (a seam) or deliberately left out.
  - **Test**: only a test or its support code differs.
- When a later change removes the difference, move the row to "Corrected divergences" with the pull request that removed it.

## Register

### Layout (`src/FerroUI.Base/layout/`)

| Upstream | Port | Kind | Why | Since |
|---|---|---|---|---|
| `LayoutManager.GetTimestamp` uses `Stopwatch.GetTimestamp()` (sub-millisecond ticks). | `get_timestamp` in `layout_manager.rs` reads `Dispatcher::current_dispatcher().now()`, which is in milliseconds. | Behaviour | The port times layout with the dispatcher's clock. Durations below 1 ms read as 0 in `LayoutPassTimed` and the layout time graph. | #25 |

### Colors (`src/FerroUI.Base/media/color.rs`, `hsl_color.rs`, `hsv_color.rs`)

| Upstream | Port | Kind | Why | Since |
|---|---|---|---|---|
| `HslColor.ToRgb`, `HslColor.ToHsv`, `HsvColor.ToRgb` and `HsvColor.ToHsl` bring the hue into range with `while (hue >= 360.0) hue -= 360.0;` and `while (hue < 0.0) hue += 360.0;`, which never end for an infinite hue or one so large that subtracting 360 does not change it. | `wrap_hue` stops when a step does not change the hue and goes on with the hue as it is. | Behaviour | A hang on a non-finite or huge hue becomes a result. Finite hues of ordinary magnitude convert as upstream. | before #51 |
| `Color.Parse(string)` throws `ArgumentNullException` for null, and `TryParse(string?)` returns false for null. | `parse` and `try_parse` take `&str`, which cannot be null. | Representation | The non-nullable form of the string argument; `Parse_Throws_ArgumentNullException_For_Null_Input` and the null row of `TryParse_Returns_False_For_Invalid_Input` have no counterpart (`color_tests.rs`). | before #51 |

### Render data (`src/FerroUI.Base/rendering/composition/drawing/`)

| Upstream | Port | Kind | Why | Since |
|---|---|---|---|---|
| `RenderDataWriter` blits unmanaged payload structs into the stream. | `render_data_writer.rs` and `render_data_reader.rs` encode payloads with `BatchValue`, the little-endian encoding of the batch transport. | Representation | A raw struct copy needs `unsafe`; the stream's format, opcodes and order are upstream's. | #26 |
| `RenderDataWriter` rents its buffer from `ArrayPool<byte>` and uses `int` lengths. | A `Vec<u8>` (initial capacity 256, amortised doubling) and `usize` lengths. | Representation | Rust's standard library has no shared array pool. | #26 |
| `RenderDataStream.Visit` has no default case: an invalid opcode loops forever. | `Visit` panics on `RenderDataOpcode::Invalid`. | Behaviour | A panic stops at the fault instead of hanging. Only reachable with a corrupt stream. | #26 |
| Each payload has a static `Opcode` field. | The associated const `IRenderDataPayload::OPCODE`. | Representation | The Rust form of a per-type constant. The tracking scanner does not match it, so `RenderDataPayloads.cs` shows 35/50 members. | #26 |
| The batch stream is made of 64-byte pooled segments. | No pooled segments. | Representation | The port's batch stream never had them, so `Round_Trip_Spanning_Multiple_Stream_Segments` cannot cross a segment boundary (see the test's header). | #26 |

### Bindings (`src/FerroUI.Base/data/`)

| Upstream | Port | Kind | Why | Since |
|---|---|---|---|---|
| `CompiledBinding.Create<TIn, TOut>(Expression<Func<TIn, TOut>>, ...)` builds a compiled binding path from a LINQ expression tree through `BindingExpressionVisitor<TIn>`. | Not ported; a path is built with `CompiledBindingPathBuilder`. `BindingExpressionVisitorTests.cs` is not ported. | Missing | Rust has no expression trees. Design note and options in `xaml.md` 3.6.1; the decision is with the owner (`CONTINUATION.md`). | #51 |

### Visual tree (`src/FerroUI.Base/visual_tree/`)

| Upstream | Port | Kind | Why | Since |
|---|---|---|---|---|
| `VisualLocator.Track(Visual, int ancestorLevel, Type?)`: a negative level makes `ElementAtOrDefault` return null. | `VisualLocator::track` takes `ancestor_level: usize`. | Representation | An unsigned level, as `VisualAncestorElementNode` already took; a negative level cannot be passed. | #27 |

### Logical tree (`src/FerroUI.Base/logical_tree/`)

| Upstream | Port | Kind | Why | Since |
|---|---|---|---|---|
| `LogicalExtensions.GetLogicalChildren` returns the `LogicalChildren` collection itself; `GetLogicalSiblings` enumerates the parent's collection lazily. | `get_logical_children` and `get_logical_siblings` return a snapshot (`Rc<Vec<..>>`) of the collection. | Behaviour | As `get_visual_children` does. A change to the collection after the call is not seen through the result. | #32 |
| `LogicalExtensions.IsLogicalAncestorOf(this ILogical? logical, ILogical? target)` accepts a null receiver and returns false. | `StyledElement::is_logical_ancestor_of(&self, target: Option<&StyledElement>)`. | Representation | The receiver of a Rust method cannot be null; the target stays optional. | #32 |
| `ControlLocator.Track(ILogical, int ancestorLevel, Type?)`: a negative level makes `ElementAtOrDefault` return null. | `ControlLocator::track` takes `ancestor_level: usize`. | Representation | As `VisualLocator::track`; a negative level cannot be passed. | #36 |

### Property system (`src/FerroUI.Base/property_store/`, `src/FerroUI.Base/ferro_object.rs`)

| Upstream | Port | Kind | Why | Since |
|---|---|---|---|---|
| `ValueStore` propagates inherited values over `GetInheritanceChildren()` by index up to the count at the start; an index past the end throws. | `FerroObject::for_each_inheritance_child` stops at an index past the end, and skips a child that was dropped without being detached. | Behaviour | The port's list holds weak references, and `inheritance_children` prunes dropped children, which can shorten the list during the loop; upstream's list of strong references cannot shrink that way. | #29 |

### ControlCatalog sample (`samples/ControlCatalog/`)

| Upstream | Port | Kind | Why | Since |
|---|---|---|---|---|
| `CompositionPage.ButtonThreadSleep` calls `Thread.Sleep(5000)`. | `button_thread_sleep` in `Pages/composition_page.rs` calls `std::thread::sleep` for 5 s. | Behaviour | Ported as upstream has it: the button demonstrates a blocked UI thread. Recorded because the port otherwise rules out blocking waits on the UI thread. | #34 |
| The messages of `CompositionPage.CustomVisualHandler` are four `static readonly object` instances compared by reference. | Four thread-local `Rc<dyn Any>` objects compared with `Rc::ptr_eq`. | Representation | `Rc` is not `Sync`, so it cannot be a `static`; one instance per thread compares the same way on the UI thread. | #34 |
| `CareCompanionAppPage.MakePipsPager` creates the two-way binding of `PipsPager.SelectedPageIndex` with `CompiledBinding.Create<TIn, TOut>(c => c.SelectedIndex, carousel, mode: BindingMode.TwoWay)`, which derives the path from an expression tree. | `make_pips_pager` in `Pages/CarouselPage/care_companion_app_page.rs` writes the path out with `CompiledBindingPathBuilder` (one element, the property `SelectingMultiPage.SelectedIndex`) and sets the source and the mode on the `CompiledBinding`. | Representation | Rust has no expression trees, so `CompiledBinding.Create` has no counterpart; the binding has the same source, path and mode. | #60 (no pull request yet) |
| `DialogsPage.GetStorageProvider` returns a `ManagedStorageProvider` when "force managed" is checked, on every platform. | `try_get_storage_provider` in `Pages/dialogs_page.rs` does so everywhere but in the browser, where it returns the storage provider of the top-level. | Missing | `ferroui-dialogs` does not build the managed storage provider for WebAssembly: it works on the local file system, which the browser does not have. | #59 |

### Templates (`src/FerroUI.Controls/templates/`)

| Upstream | Port | Kind | Why | Since |
|---|---|---|---|---|
| `FuncDataTemplate<T>` matches with `TypeUtilities.CanCast<T>`: null data matches a reference type `T` (`string`, a control type) and is passed to the build function as null. | `FuncDataTemplate::for_type` and `for_type_with_match` match null only when `T` is a nullable form (`Option<String>`, `Option<Ref<Control>>`, `Option<i32>`) (`func_data_template.rs`). | Representation | A Rust type has no null of its own: the C# `string` that accepts null is `Option<String>` here, and a template for it matches null. A template for `String` or `Ref<Control>` is the non-nullable form and is not chosen for null content. | #40 |

### Skia backend (`src/Skia/FerroUI.Skia/`)

| Upstream | Port | Kind | Why | Since |
|---|---|---|---|---|
| The backend recovers its typeface with the type test `PlatformTypeface is SkiaTypeface`. | `IPlatformTypeface::as_any`, and `SkiaTypeface::try_get` downcasts through it. | Representation | The Rust form of a runtime type test; it replaces the thread-local registry of live typefaces by address that stood in for it. | #52 |

### Tests and test support

| Upstream | Port | Kind | Why | Since |
|---|---|---|---|---|
| Upstream's test setup supplies a font manager to `constraint_and_negative_margin`. | The `MockPlatformRenderInterface` of `ferroui-base` has none; the test adds `TextTestScope` to supply one. | Test | Not recorded in #25. | #25 |
| The clipboard tests of the text controls flush only posted sync-context callbacks. | They run every dispatcher job, including the queued layout pass, and their log recorder ignores `Layout`-area messages. | Test | Running the layout pass logs the new layout timing messages, which the recorder would otherwise count. | #25 |
| The `TestRoot` of `Avalonia.UnitTests` has a settable `StylingParent` (the GlobalStyles and application-resource tests of `StyledElementTests` set it). | The `TestRoot` of `ferroui-controls` has none; `StylingRoot` in `styled_element_tests.rs`, a test root with a settable styling parent, takes its place. | Test | No reason recorded for the missing setter; the tests need only the setter. | #37 |
| `StyledElementTests.Resources_Owner_Is_Set` verifies `AddOwner` on a `Mock<IResourceDictionary>`. | `Resources` takes a concrete `ResourceDictionary`, so the test uses `RecordingResourceDictionary`, a derived dictionary that records its owners from the `on_add_owner` hook. | Test | A mock of the interface cannot be assigned to the property. | #37 |
| `ContentPresenterTests_Standalone` sets a `Mock<Control>` that also implements `IContentPresenterHost`, `IPresentationSource` and `ILogicalRoot` as the logical parent. | `MockHostParent`, a logical-root control registered as a content presenter host whose `register_content_presenter` returns false, and `MockParent`, a logical-root control (`content_presenter_tests_standalone.rs`). | Test | Moq has no counterpart; the tests do not use `IPresentationSource`. | #40 |
| `ResourceIncludeTests`, `StyleIncludeTests` and `MergeResourceIncludeTests` load each test's documents as a group of their own with `AvaloniaRuntimeXamlLoader.LoadGroup`, some into an existing root instance. | Against compiled documents (`tests/XamlIncludeFixture`), every document is a document of one of two crates: a URI two tests give different documents is under a folder named after the later test, a document without a URI is named after its test, a document of another assembly (`Demo`, the test assembly) is a document of the library, and a document loaded into a root instance is built by its build function. The theories run as `_false` and `_true` tests, and a test that expects the compiler to fail compiles its documents in the test. | Test | A crate holds one document per URI and is compiled once, where upstream compiles a group per test. | #53 |
| `TestServices.RealFocus` supplies a render interface, a font manager and a text shaper. | `TestServices::real_focus` of `ferroui-controls` has none; `Can_Get_Directional_Next_Element_With_Options` and `Can_Get_Directional_Next_Element_With_FocusedElement_Option` take them from a `TextTestScope` (`real_focus_with_text_services` in `input_element_focus_tests.rs`). | Test | The layout pass of those tests lays out the text of the buttons. No reason was recorded for the missing services in the preset. | #51 |
| `GlyphTypefaceTests` loads `MiSans-Normal.ttf` (a CJK font with vertical metrics) and `NISC18030.ttf` (a bitmap font without a `head` table) from the test assembly. | `glyph_typeface_tests.rs` loads `WenQuanYiMicroHei-Subset.ttf` and `WenQuanYiMicroHei-NoHead.ttf`, generated from WenQuanYi Micro Hei by `scripts/generate_glyph_typeface_test_fonts.py` with the table shapes the tests rely on (`vhea` and `vmtx`; no `head`, which is renamed `bhed`). | Test | Neither upstream file states a licence that allows redistributing it (MiSans: "All Rights Reserved"; NISC18030: a copyright line only), and together they are 15 MB. The substitutes are Apache 2.0 and 5 KB (`src/FerroUI.Base/NOTICE.md`). | #51 |
| `CustomFontManagerImpl` of the Skia unit tests passes the culture's `ThreeLetterISOLanguageName` and `TwoLetterISOLanguageName` to Skia's character matching. | Only the two letter name (`unit_tests/media/custom_font_manager_impl.rs`). | Missing | Seam: the port's `CultureInfo` has no `ThreeLetterISOLanguageName`. | #52 |
| `CustomFontManagerImpl.Dispose` disposes its system font collection. | It also releases the collection. | Test | The collection holds the font manager it was created with; releasing it breaks the reference cycle that the garbage collector collects upstream. | #52 |
| `TestServices.MockPlatformRenderInterface` has the headless render interface, and `TestFontManager` creates `HeadlessPlatformTypeface`s. | The mock render interface of the base crate's render test doubles, and the base crate's `TestPlatformTypeface` (`unit_tests/mod.rs`, `unit_tests/test_font_manager.rs`). | Test | The port has no headless platform; the stand-ins do the same (record nothing; the font file in memory with its identity read from its tables). | #52 |
| `UnitTestApplication.Start` disposes the `InputManager` of the scope when the application ends. | Not disposed. | Missing | The locator gives the input manager as `dyn IInputManager`, which has no `dispose` and no downcast; the font manager, which upstream disposes in the same place, is disposed. | #52 |
| `BitmapSaveTests.Save_With_Null_Options_Throws` and `Save_With_Invalid_Png_CompressionLevel_Throws`. | Not ported (header of `unit_tests/media/bitmap_save_tests.rs`). | Test | The options are a reference that cannot be null, and `CompressionLevel` is a closed enum that cannot hold 42. | #52 |
| `TextFormatterTests.Wrap_With_Ltr_Text_Does_Not_Touch_Trailing_Whitespace_BiDi` formats a text that names the upstream project. | The text names the port. | Test | The naming rule; the test compares the text with itself. | #52 |
| `CustomFontCollectionTests` load the font folder next to the test assembly (`AppContext.BaseDirectory`). | `file://` URIs of the crate's `test_assets` folder. | Test | The port's `Uri` has no implicit file URIs, and the test fonts are not copied next to the test binary. | #52 |

## Corrected divergences

Places where the port did work that upstream does not do, or did it differently, and a later change made it match upstream again. They are kept here so a regression can be recognised. Extra allocations and copies on hot paths belong here too: `PORTING-GUIDE.md` rules out allocations on hot paths that upstream avoids.

| Upstream | What the port did | Corrected by |
|---|---|---|
| `RenderDataStream` records opcodes and payloads into a byte stream. | Recorded a `Vec<RenderDataOp>` of an enum. | #26 |
| `VisualAncestorElementNode` subscribes to `VisualLocator.Track`. | Followed the attachment events of the element itself, and panicked with "Cannot find a Visual to get a visual ancestor." where upstream throws "Cannot find an ILogical to get a visual ancestor.". | #27 |
| `GetLayoutRoot` and `GetLayoutManager` are extension methods of `Visual`. | Declared them on `Layoutable` only. | #27 |
| `ServerCompositionVisual.ServerTreeWalker.Walk` reads `Children.List.Count` and allocates nothing. | Allocated an empty `Rc<Vec>` for every visual without a children collection, every frame (`walker.rs`). | #29 |
| `CompositionTarget.HitTestChildren` and `HitTestFirstCore` index the children from `Children.Count - 1` down. | Copied the children of every visual entered into a new `Vec` (`composition_target.rs`). | #29 |
| `CompositionContainerVisual.OnRootChangedCore` enumerates `Children` in place. | Copied the children of every visual of an attached or detached subtree (`visual.rs`). | #29 |
| `ValueStore` (`OnInheritanceAncestorChanged`, `OnInheritedEffectiveValueChanged`, `OnAncestorInheritedValueChanged` and the other inherited-value loops) indexes `GetInheritanceChildren()` in place. | Copied the inheritance children of every object a change passed through (`value_store.rs`). | #29 |
| `ValueStore.ReevaluateEffectiveValue` and `ReevaluateEffectiveValues` index `_frames` from the last frame. | Copied the frame list for every reevaluation (`value_store.rs`). | #29 |
| `IPseudoClasses.Remove` compares the name in place. | `Classes::remove_pseudo` allocated a `String` of the name for every call (`classes.rs`). | #29 |
| `LogicalAncestorElementNode` subscribes to `ControlLocator.Track`, which finds the ancestor when it is subscribed whether or not the element is attached. | Followed the attachment events of the element itself and reported no ancestor while the element was not attached to a logical tree. | #36 |
| `Classes.AddRange` adds every name of the argument that the collection does not hold yet, duplicates within the argument included. | `Classes::add_range` also dropped a name that appeared earlier in the same argument (`classes.rs`). | #38 |
| `XamlCompilerTaskExecutor` removes every compiled document from the assembly (`res.Remove()`) and answers a load by URI of a public one through the generated `!XamlLoader.TryLoad` (browser-platform.md, section 14, formerly the row "`ControlCatalog.Browser` host: the markup documents of the Fluent and Simple themes"). | The theme crates embedded every compiled document as an asset, and the catalog's browser host selected the feature `remove-compiled-documents` to leave them out, with a hand-written loader that answered only the document of the theme class. | #43 |
| `XamlTypeSystem.FindAssembly(name)` (`SreTypeSystem` of the run-time loader, `CecilTypeSystem` of the build) finds the assembly whose name equals `name` (without regard to case in the SRE type system). | `RuntimeTypeSystem::find_assembly` returned the first assembly whose name contained `name`, so the include transformer looked for the compiled documents of `ferres://Tests/..` in any assembly whose name contains `tests` (`runtime_type_system.rs`). | #53 |
| The `TestRoot` of `Avalonia.UnitTests` is a focus scope (`IFocusScope`). | The `TestRoot` of `ferroui-controls` was not one, so the focus of its content was not remembered for the root (`test_support.rs`). | #51 |
| `TypeUtilities.CanCast<T>` is `value is T`: a value of the type a nullable `T` holds (`int` for `int?`) and an object of class `T` behind a handle of a base class match. | `FuncDataTemplate::for_type` matched a value only when its box held exactly `T` (`func_data_template.rs`). | #51 |
| `LayoutManager.InvalidateMeasure`, `InvalidateArrange`, `ExecuteLayoutPass` and `EnqueueBringIntoView` call `Dispatcher.UIThread.VerifyAccess()` (formerly the Layout row "`LayoutManager` calls `Dispatcher.UIThread.VerifyAccess()`"). | Did not call it, so a call from a thread other than the UI thread was not caught (`layout_manager.rs`). | #51 |
| `UnitTestApplication.Start` disposes the `FontManager` of the scope when the application ends. | Left it undisposed, so a font backend that owns its system fonts (which hold the backend) was never released: about 180 MB per Skia unit test. | #52 |
| `TwoLevelCache` takes an `int` secondary size and throws `ArgumentOutOfRangeException` for a negative one. | Took a `usize`. | #52 |
