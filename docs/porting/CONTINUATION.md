# Continuation

This page is the hand-over point for the next working period: what was finished, what is in flight, the tasks that come next in order, and how to run them. Read it after `CLOUD-WORKERS.md`, which stays the standing brief of every worker. Update it at the end of each period.

## State at the hand-over of 2026-10-07

Merged this period, in order: #26, #34, #33, #31, #27, #32, #29, #30, #36, #35, #28, #37, #38, #40, #41, #39, #42, #43.

- Browser: Rust 1.99.0 with WebAssembly exceptions (#30). The ControlCatalog site loads its pictures and fonts on demand, per page (#31), as plain files copied byte for byte. Since #42 there are no packed bundles: 94 files under `assets/ControlCatalog/`, listed per page in `assets/ControlCatalog.json`, each registered on its own with `registerAsset`. The theme documents are out of the catalog's module (#35). Catalog module 35.28 MB raw, 10.62 MB gzip -9; first frame about 1.2 s unthrottled.
- TableView and Buttons: the per-event cost was the JavaScript exception trampolines (removed by #30) and copies and allocations the port made and upstream does not (#29, recorded under "Corrected divergences" in `DEVIATIONS.md`). Measured after both: TableView 8.4 to 9.0 ms per scroll event (budget 8.3 ms), Buttons 2.4 to 3.1 ms. What remains in TableView is upstream's own cell rebuild on recycled rows.
- XAML compiler: shared runtime helpers and split functions in generated code (#28, #39). The themed browser module is 1.9 % smaller raw. The generated code is deterministic: #39 fixed a hash-order bug, and the drift tests compare against committed output.
- Deviation register: `DEVIATIONS.md` (#33). Every deliberate difference from upstream gets a row there, or in the area's own page, plus a comment at its site.
- XAML compiler stage E5, step 1 (#43): the generated code answers a load by URI of every public compiled document, as upstream's `!XamlLoader` does. The themes' compiled documents are out of the embedded assets for every application.
- Core port: the ContentPresenter suites as exact ports of upstream's four files (#40). The ComplexControl and Foundation automation peer suites complete (#41).

## Since the hand-over (2026-10-07 and 2026-10-08)

- **History.** The history of `main` was rewritten on 2026-10-07 so that every commit has the owner as its only author. Every hash from the 41st commit onward changed; hashes quoted in older documents and pull requests name commits that are no longer on `main`. Branch only from the current `origin/main`, and add no co-author or session lines to commits or pull requests.
- **Release profile (#45).** `release` is thin LTO over the default code generation units, for build time. The former profile (fat LTO, one unit) is the profile `dist`: `cargo build --profile dist`, `scripts/perf-report.py --profile dist`.
- **Artwork (#46).** The ControlCatalog shows the project's own mark, word mark and banner on the desktop too; the feature `placeholder-branding` of the sample is a default feature.
- **Browser opt-level (#47).** `ferroui-base` and `ferroui-controls` are built at "s" in the `browser` profile: scrolling the TableView page takes about 20 % less processor time for 0.76 MB more with gzip. `scripts/browser/scroll-profile.mjs` measures it. This settles the owner decision on the browser opt-level.
- **Performance designs (#48).** `docs/porting/performance/` holds the findings of a scrolling trace and nine improvement designs. The owner's rule for them: ported logic does not diverge from upstream.

## Since then (2026-10-08 and 2026-10-09)

Merged, by area (pull requests 89 to 112). The sections below have the state of each.

- **Render thread, desktop.** The default wherever the render loop runs in the background (#89); the image of a shared GPU context shared between the two threads (R5.7, #90); the two-mode frame benchmark (#91); a test of unpaced changes of the UI thread while the render thread renders (#94); the features of the render interface and the platform graphics shared between the threads by their types (R5.8, #109).
- **Browser, stage B2.** B2.1 (#97), B2.4 (#95), B2.2 (#100), B2.3 (#101), B2.5 (#102), B2.6 (#106): the UI on the main thread of the page and the compositor on a render worker, in a module built with threads.
- **ControlCatalog.** The OpenGL page with its scene and the numerics it needs (#92); eleven markup gaps closed (#96), then C313, C209 and C310 (#99); AutoCompleteBox, NumericUpDown and the five performance monitor pages (#103); `App.xaml` and `SettingsPage` under the generated tests, numeric conversions, validation errors shown, pages freed when they leave a navigation page or a tabbed page (#105, #107). 218 of 219 documents load.
- **XAML compiler.** Build integration for documents that name types of other crates (#104); the source scanner and the build-time type model (#110); dependency models and the build-time type system over the scanned model, with its drift test (#112).
- **Performance.** Design 09: the native recycling benchmark, allocation counting and the counters behind the feature `perf-counters` (#108). Designs 04 and 03: four changes of the port's machinery, behaviour unchanged (#111).
- **Tracking.** The mapping data reviewed and the pages regenerated (branch `tracking-and-continuation`), then the entries that need no code written and the pages regenerated again (branch `tracking-false-negatives`); see "What the tracking pages show (2026-10-09)".

## In flight on 2026-10-09

Nothing runs in the cloud. In flight locally, each on its own branch, written by a sub-agent without a compiler and validated in the main checkout:

| Branch | What | State |
|---|---|---|
| `b2-7-catalog-on-worker-v` | Browser B2.7: the ControlCatalog on the render worker, disposal of a view, a panic of a frame reaches the page | pull request 113, open; being validated |
| `b2-8-site-and-ci-v` | Browser B2.8: one site with both modules, and CI for the threaded one | written; being validated; after B2.7 |
| `skia-shim-threads` | The Skia bindings shim built for threads, so that the threaded link needs no `--no-check-features` | written; waits for the owner's approval of its downloads |
| `xaml-type-system-drift` | XAML compiler: the drift test of the build-time type system driven to zero | in work |
| `perf-02-property-changes` | Performance designs 02 and 01 | in work |
| `perf-05-bindings-text` | Performance designs 05 and 06 | in work |

## In flight at the hand-over of 2026-10-07

Each task was asked to stop at a clean, tested state and to give its pull request a "Continuation" section. Those sections are folded into the tasks below. The pull requests have the full detail: #42 for task 1, #43 for task 2.

| Branch | Task | Brief |
|---|---|---|
| none | Nothing is in flight. Every task stopped at a clean state and its work is merged. The core port's `core-port-11` was never pushed; its next batches are listed in task 3. | Tasks 1 to 3 below |

## Next tasks, in order

### 1. Catalog assets as plain files: follow-ups (optional)

The change itself is done (#42). Sizes did not change. Start-up makes 20 requests before the first frame instead of 11. The first frame came 30 to 100 ms later in every measured round. That is within the spread of single loads, but it went the same way every time. Left open:

- **Navigation cost:** measure navigation to DrawerPage (18 files, previously 5 bundles) or CarouselPage at `--throttle 50,40`, to put a number on the per-file request cost.
- **First-frame difference:** look into the 30 to 100 ms, for example with a CPU profile comparing the 10 `registerAsset` calls with the earlier single bundle unpack.
- **Stale tracking pages:** done. Every page was regenerated on 2026-10-09; see "What the tracking pages show (2026-10-09)".

Do not reintroduce packing.

### 2. XAML compiler stage E5

The stage-table row "E5 Includes and assets" in `xaml.md` (9.10.1), and items 3 and 4 of `browser-platform.md` section 20.

**Done: step 1, the loader table (#43).**
- The generated `compiled_xaml.rs` ends with `try_load`, the counterpart of upstream's `!XamlLoader.TryLoad`. It has one entry per public document, and `x:ClassModifier` is read as upstream reads it.
- The themes register it, and their compiled documents are out of the embedded assets for every application.
- The `remove-compiled-documents` feature is gone. Its deviation row moved to "Corrected divergences".
- One seam is recorded in `xaml.md` 9.10.1: the caller of `generate_class_file` states the class constructor. Upstream picks it from the assembly; the port has no build-time type system of the crate yet.

**Done: step 2, includes across crates (#53).**
- A crate with compiled markup writes `compiled_xaml.xamlmeta` (`rust_emitter::XamlMetadata`, the `documents[]` of `xaml.md` 9.5.1) next to its generated file; both themes do.
- The emitter's transform puts `CompiledFerroXaml.!FerroResources` with `Build:<path>` methods on the assembly of each dependency (`CompiledMarkupTypeSystem`), so upstream's include transformer links an include of another crate's document to its build function, or to its class. A merge include of another crate's document is upstream's error.
- The include group transformers were compared with upstream; the emitter's transform has no run-time include fallback.
- `generate_file` now links includes within its group, and the emitter writes source information (`CreateSourceInfo`).
- Fixture: `tests/XamlIncludeFixture` (library `xaml-include-fixture-theme`, application `xaml-include-fixture-application`), running upstream's `ResourceIncludeTests`, `StyleIncludeTests` and `MergeResourceIncludeTests` against compiled documents.

**Done: step 3, build integration over the run-time type system (#104).** `xaml.md` 9.6.8 has the design as built and the table of what departs from 9.6.
- `ferroui-build` (`src/FerroUI.Build.Tasks`): `Build::from_env().assembly(..).embed_assets(..).compile_xaml().run()` for a crate's `build.rs`. It writes the generated modules, the asset table, the loader table and `register()` (R8) to `$OUT_DIR/xaml/`, writes `$OUT_DIR/<crate>.xamlmeta`, and prints the `cargo::` lines. `ferroui_markup_xaml::include_compiled_xaml!()` includes the result.
- The `.xamlmeta` files travel through Cargo `links` metadata (`DEP_<CRATE>_XAML_XAMLMETA`). The themes export their checked-in file that way from their existing `build.rs`.
- The constructor of a class is picked by the compiler (`generate_class_file(class, None, ..)`), as upstream picks it. The themes still state theirs, because their markup metadata declares a `new()` upstream's class does not have.
- The include fixture is converted: its generated files are no longer checked in, except the document of the class `StyleWithServiceProvider`.
- The limit: the compiler reads types from the registries of the process, so the build script links the crates its documents name, and it cannot link its own crate. A document that names a type of its own crate (every `x:Class` document) cannot be compiled by a build script. That is why the themes are not converted and why `LocaleCollection` moved from the fixture application to the fixture library.

**Done: the build-time type system, first stage (#110).** `xaml.md` 9.5.6 has it as built.
- `ferroui-build` has the type model of a crate (`model.rs`: `AssemblyModel`, `TypeModel`, `MemberModel`, `RegisteredModel`) and its file: `.xamlmeta` of format 2, which keeps the documents of format 1 where the compiler reads them.
- The source scanner (`scanner/`, `scan_crate`) fills the model from the declaration macros of a crate's sources, with `syn` on file level, linking nothing. What it does not read is a diagnostic with file and line (`FRN9xxx`).
- A fixture source tree and a scan of the base and the controls crates as files test it. The second prints the coverage: `cargo test -p ferroui-build --lib real_crates -- --nocapture`.
- `Build` and the emitter are unchanged; `HANDOVER.md` section 11 has the validation.

**Done: the build-time type system, second stage (#112).** `xaml.md` 9.5.7 has it as built.
- Dependency models: the export table of a crate in its model (every public path of a type, with its declaring module), the models of several crates read as a set (`model_set.rs`, `ModelSet`), the scan with the models of the dependencies, and `Build::export_metadata()`.
- The closed table of runtime library types is data (`core_table.rs` of the loader), and both type systems define their types from it.
- `ModelTypeSystem` (`type_system/`): the type system of the compiler over the type models.
- The drift test compares it with the run-time type system of the linked crates, type by type and member by member, for the base and the controls crates. Every difference has a kind; the test is set aside as the work list of the next stage, and 9.5.7 has its first result.

Remaining, in order. #43's "Continuation" section has the upstream files for each step.

1. **The rest of the build-time type system** (`xaml.md` 9.10.1, "Remaining for E5"), in this order: the drift test to zero (branch `xaml-type-system-drift`), with `markup-xaml` added to it once the two crates agree; the call forms and the emitter on the model; `compile_xaml()` on the model.
   - It removes the limit above: the build script compiles `x:Class` documents without linking anything.
   - Then: `include_xaml!` per class, the generated `register_types` of 9.7.4, the themes on the build script, diagnostics with codes and the cache (9.6.4, 9.6.5). The diagnostics bring the two files of `Avalonia.Build.Tasks` that are still to be ported (`XamlCompilerDiagnosticsFilter.cs`, `DeterministicIdGenerator.cs`; row 4 of the tracking table below).
   - Until then a crate with `x:Class` documents uses the checked-in path (`generate_class_file` in a test). The two `XamlIlTests` classes with compiled markup (`XamlIlClassWithPrecompiledXaml`, `XamlIlClassWithCustomProperty`) can be compiled that way; today their constructors populate through the run-time loader; the two tests are ported and not ignored.
   - Small and independent: remove `new:` from the markup metadata of the two themes so that the compiler picks their constructor too (check with a build that `<FluentTheme/>` in markup goes through `FerroXamlIlConstructorServiceProviderTransformer`).
2. **The themes and the catalog compiled.**
   - Its 219 documents and the `x:Class` documents of `src/FerroUI.Dialogs`, so that neither links the run-time loader.
   - The estimate is +6 to +10 MB raw and +0.5 to +1.2 MB gzip on the catalog module, against 0.24 s less CPU before the first frame.
   - Measure a few pages first. If the growth exceeds the estimate, the owner decides on the merge.
3. **Measure #43.** Measure `themed_view` and `control-catalog-browser` for #43's change (not measured yet) and add the table to section 20. `themed_view` is expected to lose the 168 theme documents, as the catalog did in #35.

Regenerating output:
- Corpus: `cargo test -p ferroui-markup-xaml-tests --lib emitter::differential_tests::regenerate_emitter_output -- --ignored --exact`.
- Themes: `cargo test -p ferroui-themes-<fluent|simple> --lib tests::compiled_xaml_tests::regenerate_compiled_xaml -- --ignored`.
- Include fixture: the build script generates everything except the document of the library's class. That one: `cargo test -p xaml-include-fixture-theme --lib tests::compiled_xaml_tests::regenerate_compiled_xaml -- --ignored`. The application has no regeneration test; `cargo build -p xaml-include-fixture-application` regenerates its output, and `build_script_output_is_the_emitters` of each crate compares it with the emitter run in the tests.
- Regenerate twice and diff before every push.

### 3. Core port: the remaining upstream suites

Each suite is an exact port under upstream's names, with framework fixes where a test exposes a difference.

**Done in #51 (`core-port-12`):**
- `Media/ColorTests.cs`: 65 of 66 in `media/color_tests.rs` (the null-argument test has no counterpart); `Color`, `HslColor` and `HsvColor` take upstream's nullable format and ignored format provider.
- `Input/InputElement_Focus.cs`: 46 of 46 in `input_element_focus_tests.rs` of `ferroui-controls`; the controls' test root is a focus scope, as upstream's.
- `Media/GlyphTypefaceTests.cs`: 41 of 41 on font files (`src/FerroUI.Base/test_assets/fonts/`, licences in `src/FerroUI.Base/NOTICE.md`); two of upstream's fonts are replaced by fonts derived from WenQuanYi Micro Hei (`DEVIATIONS.md`, Tests).
- The duplicated ContentPresenter text tests are gone; `FuncDataTemplate` matches as `TypeUtilities.CanCast<T>`; `LayoutManager` verifies access to the UI thread.
- `Data/Core/Parsers/BindingExpressionVisitorTests.cs`: not ported; design note in `xaml.md` 3.6.1, decision below.

**Count on 2026-10-08.** Upstream test methods (`[Fact]`/`[Theory]`) of `tests/Avalonia.Base.UnitTests` and `tests/Avalonia.Controls.UnitTests` at the tracked commit, matched by name (snake case, underscores and case ignored, "Avalonia" read as "Ferro") against every function under `src/`. A test renamed in the port counts as missing, a name found anywhere counts as present.

- `Avalonia.Base.UnitTests`: 2,357 tests, 394 missing in 92 files.
- `Avalonia.Controls.UnitTests`: 3,060 tests, 105 missing in 29 files.

**Test gaps by area** (missing / tests of the file). The order of work is the owner's queue in "Core first" below; each list here is the test side of the batch with the same area (font tables and `Media` go with batch 5, input and styling with batch 6, controls with batch 7).

1. **Font tables** (Base, about 90). The real font files of #51 make them portable now. `Media/Fonts/Tables/HeadTableTests.cs` 13/13, `MaxpTableTests.cs` 11/11, `OS2TableTests.cs` 10/10, `LocaTableTests.cs` 6/8, `GlyfTableTests.cs` 1/13, `DecyclerTests.cs` 1/17; `Media/GlyphDrawingOptionsTests.cs` 14/14; `Media/FontManagerTryGetFontCollectionTests.cs` 11/17; `Media/Fonts/TestInfrastructure/SyntheticFontTests.cs` 5/11; `Media/Fonts/UnmanagedFontMemoryTests.cs` 3/4, `Bcp47ScriptResolverTests.cs` 3/4, `FontCollectionKeyTests.cs` 2/8, `FontFamilyLoaderTests.cs` 2/5, `FamilyNameCollectionTests.cs` 1/2, `FontFallbackScriptHintsTests.cs` 1/10, `FontFamilyKeyTests.cs` 1/3; `Media/GlyphRunTests.cs` 5/7; `Media/NormalizedVariationPositionTests.cs` 6/30; `Media/FontVariationSettingsTests.cs` 2/11; `Media/FontManagerTests.cs` 1/6. Upstream loads more font files from `tests/Avalonia.RenderTests/Assets`; check each licence before adding one, as #51 did.
2. **Property system** (Base, about 60). `AvaloniaObjectTests_Binding.cs` 13/75, `_Threading.cs` 10/10, `_SetCurrentValue.cs` 5/23, `_Metadata.cs` 4/4, `_SetValue.cs` 4/28, `_MultiBinding.cs` 3/7, `_Coercion.cs` 1/19, `_Direct.cs` 1/42; `AvaloniaPropertyTests.cs` 1/14; `Utilities/AvaloniaPropertyDictionaryTests.cs` 19/19.
3. **Styling** (Base, about 50). `Styling/SelectorTests_Multiple.cs` 9/11, `_Template.cs` 6/8, `_Or.cs` 4/9, `_Name.cs` 3/5, `_PropertyEquals.cs` 3/6, `_Class.cs` 2/9, `_NthChild.cs` 2/11, `_NthLastChild.cs` 2/11, `_OfType.cs` 2/4, `_Child.cs` 1/5, `_Descendent.cs` 1/6, `_Not.cs` 1/8; `StyleTests.cs` 6/37, `StylesTests.cs` 5/7, `SetterTests.cs` 2/18, `ContainerTests.cs` 1/5, `ResourceDictionaryTests.cs` 1/16; `ClassBindingManagerTests.cs` 2/2.
4. **Input** (Base, about 45). `Input/AccessKeyHandlerTests.cs` 7/14, `GesturesTests.cs` 7/22, `KeyboardNavigationTests_XY.cs` 5/13, `PointerOverTests.cs` 5/12, `SwipeGestureRecognizerTests.cs` 5/6, `DataFormatTests.cs` 4/12, `KeyboardNavigationTests_Tab.cs` 3/36, `TouchDeviceTests.cs` 3/7, `MouseDeviceTests.cs` 2/8, `PlatformDataTransferItemTests.cs` 2/4; `Interactivity/RoutedEventRegistryTests.cs` 3/4; `FlowDirectionTests.cs` 3/3.
5. **Utilities, collections, data, animation** (Base, about 70). `Utilities/ObjectPoolTests.cs` 13/13, `SafeEnumerableAvaloniaListTests.cs` 10/10, `SingleOrQueueTests.cs` 4/4, `StringSplitterTests.cs` 4/4, `InlineDictionaryTests.cs` 3/3, `AvaloniaResourcesIndexTests.cs` 2/2; `Collections/AvaloniaListTests.cs` 4/31, `AvaloniaDictionaryTests.cs` 1/13; `Data/Core/BindingExpressionTests.DataValidation.cs` 3/22, `.Property.cs` 2/14, `.SetValue.cs` 1/12; `Data/DefaultValueConverterTests.cs` 2/16, `ReflectionClrPropertyInfoTests.cs` 1/1; `Animation/` (`BrushTransitionTests.cs` 2/2, `KeySplineTests.cs` 2/10, `SpringTests.cs` 2/4, `StyleAnimationTests.cs` 2/2, `AnimationIterationTests.cs` 1/27); `DispatcherTests.cs` 2/21, `Logging/LoggingTests.cs` 2/2 and the files with one or two missing tests (`AssetLoaderTests.cs`, `Media/PathMarkupParserTests.cs`, `Media/PenTests.cs`, `Media/RelativeTransformBrushTests.cs` 2/8, `Media/TextFormatting/FormattingBufferHelperTests.cs` 2/8, `Platform/SlicedStreamTests.cs`, `Rendering/DrawingImagePropagationTests.cs`). `Media/ColorTests.cs` counts 1 missing: the null-argument test, which has no counterpart.
6. **Controls** (about 100). `GridTests.cs` 32/81 (the shared-size tests), `DesignTests.cs` 9/9, `NavigationEventArgsTests.cs` 9/10, `HeadlessProbeTests.cs` 7/7, `Utils/SafeEnumerableHashSetTests.cs` 5/5, `NavigationPageTests.cs` 4/161, `ConnectedAnimationTests.cs` 3/57, and one or two in each of `BorderTests.cs`, `GridLengthTests.cs`, `GridSplitterTests.cs`, `Mixins/SelectableMixinTests.cs`, `Primitives/TemplatedControlTests.cs`, `Primitives/UniformGridTests.cs`, `TextBlockTests.cs`, `TextBoxTests.cs`, `ViewboxTests.cs`, `ContentControlTests.cs`, `InputElementGestureTests.cs`, `ItemsSourceViewTests.cs`, `ListBoxTests.cs`, `LoadedTests.cs`, `PanelTests.cs`, `Platform/ScreensTests.cs`, `ScrollViewerTests.cs`, `Selection/InternalSelectionModelTests.cs`, `TabControlTests.cs`, `Utils/AncestorFinderTests.cs`, `Utils/BindingEvaluatorTests.cs`.

Not in the batches: `SourceGenerators/CrossThreadProxyGeneratorTests.cs` (12, a C# source generator), `Media/TextFormatting/HarfbuzzTextShaperTests.cs` (6, belongs to the HarfBuzz crate), `Data/Core/Plugins/DataAnnotationsValidationPluginTests.cs` (5, `System.ComponentModel.DataAnnotations` has no counterpart), `BindingExpressionVisitorTests.cs` (36, decision below) and `Automation/ColorSpectrumAutomationPeerTests.cs` (6, done: ported with the crate `ferroui-controls-color-picker` in `automation/peers/color_spectrum_automation_peer_tests.rs`). Some of the counted files may hold tests that the port has under other names; check each file before porting.

Also: the layout clock is in milliseconds (`DEVIATIONS.md`, Layout). Found while porting the colour tests: the composite formatting of bindings (`data/converters/composite_format.rs`) does not format `Color`, `HslColor` and `HsvColor` with their format strings, as `string.Format` does for an `IFormattable` upstream (`{Binding Color, StringFormat={}{0:X}}`).

### 4. Remaining compositor pieces

`cloud-tasks/composition-animations.md`, and row 12 of `CRITICAL-PATH.md`.

### 5. Browser follow-ups

- `src/Skia/FerroUI.Skia/emscripten/emscripten_sjlj.cpp` bridges Skia's prebuilt sjlj objects to WebAssembly exceptions. Remove it once rust-skia publishes Emscripten binaries built with `-sSUPPORT_LONGJMP=wasm` (recorded in `browser-platform.md` section 14).
- TableView is still slightly over its 8.3 ms budget per event. The rest is upstream's own work. Further gains would come from the build (see owner decisions), not from changing ported logic.

## Decisions the owner made on 2026-10-08

- **Converter culture.** The owner left the design to the port. Decision: follow upstream's contract. `IValueConverter::convert` and `convert_back` and `IMultiValueConverter::convert` gain the culture as their last argument, passed as `&CultureInfo` (`utilities/culture_info.rs`); the bindings and binding expressions gain `ConverterCulture` (`CompiledBinding`, `ReflectionBinding`, `MultiBinding`, `TemplateBinding`, `BindingExpression`, `MultiBindingExpression`), with upstream's default (the current culture when none is set), and `CultureInfoIetfLanguageTagConverter` is ported. A context struct in place of the argument was considered and rejected: it would differ from upstream at every one of the 34 converter implementations for no gain, since the argument is a reference. The change is mechanical and crosses every crate, so it is done in the validation phase, with the compiler finding every implementation and caller, as a pull request of its own.
- **Direct properties and untyped values.** Follow upstream: the untyped route of a direct property converts a value as `AvaloniaProperty<T>.TryConvert` does (the implicit conversions of `TypeUtilities.TryConvertImplicit`), written with the registered value conversions of `ValueTypes` rather than reflection. The ported test `set_value_does_not_convert_to_nullable`, which records the old behaviour, is checked against upstream's test of that name and corrected if the port's version asserts something upstream does not. Done in the validation phase, in the pull request of the property store batch.
- **Build location.** Local builds and validation use `CARGO_TARGET_DIR=/Volumes/1TB-macOS/ferroui-target` (the internal disk is nearly full); build output that is no longer needed is deleted when a phase ends.
- Render thread, R5: the server compositor is confined to a lock, as upstream on macOS (both threads render under it), not to the render thread. An `unsafe impl Send` under the server side is accepted for it.
- Render thread: it is the default on the desktop from 2026-10-08 (`Compositor::new` follows the render loop; `FERROUI_RENDER_THREAD=0` is the way back).
- Browser, stage B2: the UI thread stays on the browser's main thread and only rendering moves to a worker (`browser-render-worker.md`).

## Decisions waiting for the owner

- **Bindings from expression trees.** `CompiledBinding.Create<TIn, TOut>(Expression)` and `BindingExpressionVisitor` (36 tests) have no counterpart in the port. `xaml.md` 3.6.1 compares three options: no counterpart (recommended; the builder chain stands for the expression tree), a `binding_path!` macro, or a run-time expression model. The note maps each of the 36 tests to its counterpart: 27 have one in `CompiledBindingPathBuilder`.
- **CI speed-up.** The earlier proposal for a faster CI run is still open.
- **Stale branches.** Branches of merged pull requests can be deleted by hand.
- **The downloads of the Skia bindings shim (2026-10-09).** The shim built for threads (branch `skia-shim-threads`) is written and needs downloads the owner has to approve before it is built and validated. Until then the threaded link passes `--no-check-features` while two threads call Skia (`browser-render-worker.md`, result of B2.6).
- **The two new CI checks of B2.8 (2026-10-09).** B2.8 adds two checks to CI. Whether they become required for a merge, next to the three of today ("How to run the next period"), is the owner's decision.
- **The order of inherited change notifications across properties (2026-10-09).** When the inheritance parent changes and more than one ancestor store contributes values, the port raises the changes in the order the pairs were found and upstream in the order of its property IDs; property IDs follow the order of registration, which is not upstream's either, so neither order reproduces upstream's sequence. Design 03 recorded it and did not change it (`performance/designs/03-inheritance-parent-change.md`, "A difference found"; `DEVIATIONS.md`, Property system). To decide: sort the pairs by property ID, or keep the order and the deviation row.

## Critical now: the render thread (owner, 2026-10-08)

The owner's order on 2026-10-08: finish the work in flight, then the render thread, then the scheduled work. Design, stages and what each step found: `docs/porting/render-thread.md`; the browser stage B2: `docs/porting/browser-render-worker.md`.

State on 2026-10-09:

- **Desktop: done and the default.** Stages R1 to R5.8 are on `main` (thread-safe render resources, `Send` jobs, the render-thread mode, the synchronous wait, the compositor lock, render surfaces shared between the threads, atomic native reference counts, the Metal audit, GPU interop objects confined to the lock, the shared image import, and R5.8: a feature of the render interface is handed out as a handle bound to the compositor lock, and the platform graphics are `Arc<dyn IPlatformGraphics>` with `Send + Sync`, shared by their types). `Compositor::new` chooses the render-thread mode wherever the render loop runs in the background; `FERROUI_RENDER_THREAD=0` is the way back (pull request 89). A stress test (unpaced changes of the UI thread while the render thread renders, pull request 94) and the two-mode frame benchmark (pull request 91) are on `main`.
- **Desktop, still open:** the measurements repeated on an idle machine, and interaction by hand in the render-thread mode (live resize, scrolling, popups, closing a window while it renders).
- **Browser B1 is on `main`** (pull request 88): `scripts/build-browser.sh <example> --threads`, the isolation service worker, the example `thread_spawn`. The threaded link passes `--no-check-features` because the prebuilt Skia bindings shim has no atomics; the shim built for threads is written (branch `skia-shim-threads`) and waits for the owner's approval of its downloads.
- **Browser B2.1 to B2.6 are on `main`.** The UI stays on the main thread of the page and the compositor runs on a render worker, in a module built with threads, with strict confinement of the render target to that worker; `BrowserPlatformOptions::render_thread` keeps such a module on one thread. Validated with `themed_view` in headless Chrome: no sampled pixel differs between the render thread and one thread, and no call is proxied to the main thread during a frame (`browser-render-worker.md`, result of B2.6).
- **Browser B2.7 and B2.8 are written and being validated.** B2.7: the ControlCatalog on the worker, with the disposal of a view (pull request 113). B2.8: one site with both modules, the host page choosing between them, and CI. Then the measurements of B3.
- **ControlCatalog.** 218 of 219 documents load. The one left is the OpenGL lease page, which waits for Ganesh on OpenGL in the desktop build of the Skia backend. No gap of the framework blocks a document; `samples/ControlCatalog/GAPS.md` has the gaps closed since (C102 to C316).
- **XAML compiler.** Task 2 above: build integration for documents that name types of other crates, the source scanner, dependency models and the build-time type system with its drift test are on `main`. Next: the drift to zero, the call forms and the emitter, `compile_xaml()` on the model, then the themes and the catalog compiled.
- **Performance.** Design 09 is implemented, with its first figures. Designs 03 and 04 are done as far as the owner's rule allows (ported logic does not diverge from upstream). Designs 02 and 01, and 05 and 06, are in work. `performance/README.md` has the status.
- **Tracking.** The review of the waivers and of the doubly mapped files, the entries of the work list that need no code and the regeneration of the pages are done; "What the tracking pages show (2026-10-09)" below has the gaps that are left.
- After those: the Metal external objects feature, the EGL seams, Ganesh GL on the desktop.

## Core first (owner, 2026-10-08): done

The owner's order of 2026-10-08 was to finish the core port first, with all four local sub-agents on it. The whole queue (the base library by area, `ferroui-controls`, the markup crates and the run-time loader, the Skia and macOS backends, the headless platform) was written without a compiler, integrated and validated in one build, and merged as pull request 66, with the follow-ups in pull requests 65 (the Metal external objects contracts) and 68. Most of what the tracking pages listed as missing was a false negative of their name matching; what was absent is ported, and the rest is recorded in `docs/porting/data/path-overrides.toml` and `member-waivers.toml`.

Left from it, and done on 2026-10-09: the tracking pages regenerated in a pull request of their own, and the false negatives of their name matching recorded in a second one. The remaining gaps are taken from them in the next section.

## What the tracking pages show (2026-10-09)

What changed on the branch `tracking-false-negatives` (no code of the port is changed by the tracking part; nothing was built):

- **The pages report the gaps.** Members reported as missing: 908 before, 452 now. Of the 456 that left the list, 123 are ported code the name matching did not find and are now recorded with the Rust member that ports each (type tables, aliases, waivers; `Avalonia.Metal` is tracked in the `metal` directory of the Skia backend); 333 have no counterpart by design or by scope and are recorded file by file with the design section (the source generators, the build tasks, the Skia GPU on Vulkan).
- **Not found, and left reported:** four members (`SkiaContext.PublicFeatures`, `BrowserTopLevelImpl.FrameSize`, `BrowserWindowingPlatform.EventGrouperDispatchQueue`, `HeadlessGlyphRunStub.GlyphTypeface`). The first review counted them among the false negatives; they are not in the Rust code.
- **Read against the code, and different from what the first review said:** the two obsolete `SaveImage` overloads and the legacy `CreateDrawingContext(bool)` of the Skia backend are not ported (their waivers say so); `GpuHandleWrapFeature.cs` of the macOS backend comes with the external objects on Metal, not with Ganesh; the output of the name generator (the parts struct of a class with markup) is designed and not written.
- **The judgements** of the first review that were data matters are resolved; the list at the end of the section has what is left, with 83 members of three files that are not applicable "for now" and are therefore not counted.
- **The type tables (sources, to be validated by a build).** The five classes the drift test of the build-time type system listed as not registered are in the `TYPES` lists: `GlyphRunDrawing` (base), `DecorationsOverlaysAutomationPeer`, `UnrealizedSelectionPeer`, `NativeMenuItemPresenter` and `TopLevelHostAutomationPeer` (controls). Three of them were private to their files and are now visible in the crate, like the other internal classes the list names; that is a second commit, which can be dropped on its own. The recorded Rust paths of the base crate were regenerated (`scripts/generate_markup_types.py --target rust-paths`) and `emitter/rust_paths_check.rs` of the markup tests has the one new path, written by hand in the form its test generates. `GlyphRunDrawing` is not declared as the other registered classes are: its two properties are bare `ferro_property!` accessors outside a `ferro_properties!` block (not registered until an accessor is called, and without a recorded accessor), and it has no `ferro_class_info!` with a constructor. Until `media/glyph_run_drawing.rs` is brought to the form of `geometry_drawing.rs`, the drift test may report its two properties as members only the model has.

The mapping data was reviewed and the pages regenerated twice on 2026-10-09, against the upstream commit the port tracks (`17350180`): first on the branch `tracking-and-continuation`, which left a work list of entries that need no code, then on the branch `tracking-false-negatives`, which wrote those entries. The reference checkout is ahead of the tracked commit, so `scripts/port-status/run.sh` would run the extractor again and move the baseline; the pages were regenerated with `python3 scripts/port-status/port_status.py`, which is the second step of `run.sh` alone. A second run changes nothing.

| Projects in scope | Pages before the core port | First review | Now |
|---|---:|---:|---:|
| C# files, ported of total | 2045 of 2363 | 2221 of 2298 | 2230 of 2262 |
| Files not applicable | 51 | 116 | 152 |
| Types, ported of total (waived) | 2636 of 3276 (0) | 2821 of 3190 (203) | 2842 of 3132 (203) |
| Members, ported of total (waived) | 19466 of 23849 (44) | 20610 of 23406 (1888) | 20669 of 23122 (2001) |
| Members missing | 4339 | 908 | 452 |
| Member coverage | 81.8 % | 95.8 % | 97.9 % |
| Rust-only files without a recorded reason | 306 of 327 | 0 of 324 | 0 of 322 |

No project lost a file or a member in either regeneration. Read the member coverage with its waivers: 2001 members are counted out because `member-waivers.toml` says where each one is under another name or why it has no counterpart, 1153 of them in the base library. The scanner matches names and nothing else (`scripts/port-status/README.md`, Limits).

What is still missing, by project: `Avalonia.Remote.Protocol` 201 members in 14 files, `Avalonia.DesignerSupport` 176 in 9, `Avalonia.Skia` 43 (3 files missing, 4 with gaps), `Avalonia.Controls` 17 in 4, `Avalonia.Native` 8 in one file that exists, `Avalonia.Build.Tasks` 4 in 2, `Avalonia.Browser` 2, `Avalonia.Headless` 1. Every other project in scope has no missing file and no missing member, the base library and `Avalonia.Metal` among them; `Avalonia.Generators` has no file left to port.

The gaps, by missing members. The rows keep the numbers of the table of the first review, which other pages quote. **gap** is work to port.

| # | Upstream | Size | What it is | Judgement | Where it goes |
|---:|---|---|---|---|---|
| 2 | `src/Avalonia.Remote.Protocol` (14 files) | 201 members, 2403 lines | The protocol of the previewer: `MetsysBson.cs` (a BSON reader and writer, 1658 lines, 105 members), the message classes (`ViewportMessages.cs`, `InputMessages.cs`, `DesignMessages.cs`, 60 members), the transports (`BsonStreamTransport.cs`, `BsonTcpTransport.cs`, `TcpTransportBase.cs`, `TransportConnectionWrapper.cs`) | **ported 2026-10-09, written without a build** (the numbers of this row and of the paragraph above are from before it; the pages are not regenerated yet). All 14 files: the 17 message classes with their identifiers, the resolver, the BSON serializer and the transports. What differs is in `DEVIATIONS.md`, "Remote protocol": declarations instead of reflection (`bson_class!`, `bson_enum!`, `ferro_remote_message_guid!`, the table `ASSEMBLY`), and threads with blocking calls instead of tasks (the threading contract is at the top of `i_transport.rs`). Left to do: build it and run its tests (`cargo test -p ferroui-remote-protocol`: the expected bytes of the byte-level tests were computed by hand), regenerate the pages and settle what the scanner does not match (`member-waivers.toml` has the waivers and aliases written from the member list, not from a run) | The crate `ferroui-remote-protocol` in `src/FerroUI.Remote.Protocol/`, one file per upstream file (`metsys_bson.rs`, `viewport_messages.rs`, ...), with `guid.rs`, `error.rs`, `task.rs` and `assembly.rs` for what the runtime library gives upstream. Rows 3 and 10 build on it: a message class of another crate is declared with the same macros and listed in an `Assembly` of that crate |
| 3 | `src/Avalonia.DesignerSupport` (9 files) | 176 members, 1834 lines | The previewer host: `Remote/Stubs.cs` (the windowing stubs, 81 members), `Remote/PreviewerWindowImpl.cs` (40), `Remote/HtmlTransport/SimpleWebSocketHttpServer.cs` (21) and `HtmlTransport.cs`, `Remote/RemoteDesignerEntryPoint.cs`, `DesignWindowLoader.cs` | gap, phase 4, P3; after row 2 | A new crate `ferroui-designer-support` in `src/FerroUI.DesignerSupport/` (`remote/stubs.rs`, `remote/previewer_window_impl.rs`, ...). `DesignWindowLoader.cs` needs the run-time loader |
| 6 | `Skia/Avalonia.Skia/Gpu/OpenGl/GlSkiaExternalObjectsFeature.cs`, `GlSkiaSharedTextureForComposition.cs`, and `GlSkiaGpu.CreateSharedTextureForComposition` | 30 members, 346 lines | Import of external images and semaphores into the Ganesh context on OpenGL, and the texture a second context shares with the compositor | gap. The contracts are ported in `ferroui-opengl` (`i_gl_context_external_objects_feature.rs`, `composition/`), the Skia side is not | `src/Skia/FerroUI.Skia/gpu/open_gl/gl_skia_external_objects_feature.rs` and `gl_skia_shared_texture_for_composition.rs`; the method in `gl_skia_gpu.rs`. It goes with Ganesh on the desktop, and `AvaloniaNativeGlPlatformGraphics.cs` of the macOS backend with it (see the list below) |
| 9 | `Skia/Avalonia.Skia/Gpu/Metal/SkiaMetalExternalObjectsFeature.cs` and `MetalExternalObjectsFeature` of `Avalonia.Native/Metal.cs` | 17 members, 76 lines and a class | The external objects feature on Metal: import of an external image and of a shared event, wait and signal | gap (row 12 of `CRITICAL-PATH.md`: no backend implements the feature, so a drawing surface has no GPU image to import) | `src/Skia/FerroUI.Skia/gpu/metal/skia_metal_external_objects_feature.rs`; the class in `src/FerroUI.Native/metal.rs`, and `GpuHandleWrapFeature.cs` of the macOS backend with it (see the list below). R5.8 settled how a feature leaves the compositor lock, which this waited for |
| 10 | `Avalonia.Controls/Remote/` (`RemoteServer.cs`, `RemoteWidget.cs`, `Server/RemoteServerTopLevelImpl.cs` and `.Framebuffer.cs`) | 17 members, 593 lines | The control that shows a remote top-level and the top-level implementation that serves one over the protocol | gap, with rows 2 and 3 | `src/FerroUI.Controls/remote/` under the same names |
| 4 | `Avalonia.Build.Tasks/XamlCompilerDiagnosticsFilter.cs`, `DeterministicIdGenerator.cs` | 4 members, 84 lines | The severity of a diagnostic of the compiler from the configuration of the project; the generator of identifier parts of a compilation | gap (`xaml.md` 6.3: both port) | `src/FerroUI.Build.Tasks/xaml_compiler_diagnostics_filter.rs` and `deterministic_id_generator.rs`, with the diagnostics of `compile_xaml()` (task 2) |
| 15 | `Skia/Avalonia.Skia/Helpers/DrawingContextHelper.cs`, `Helpers/ImageSavingHelper.cs` | 3 members | `DrawingContextHelper.RenderAsync` (two overloads, the public way to draw a visual onto a Skia canvas of the application) and `ImageSavingHelper.SavePicture` | gap | The two files exist: `src/Skia/FerroUI.Skia/helpers/drawing_context_helper.rs` and `image_saving_helper.rs` |

Four more members were looked for in the Rust code and not found. They are left reported as missing, since a member that is not there is not a false negative:

- `SkiaContext.PublicFeatures` (`Skia/Avalonia.Skia/SkiaBackendContext.cs`): upstream publishes the texture sharing and the external objects features of the GPU there; `SkiaContext` of `skia_backend_context.rs` does not override `public_features`, whose default is empty. It comes with rows 6 and 9.
- `BrowserTopLevelImpl.FrameSize` (`Avalonia.Browser`): a property that returns null and belongs to no contract the class implements.
- `BrowserWindowingPlatform.EventGrouperDispatchQueue` (`Avalonia.Browser`): the queue of the raw event grouper, which upstream creates when the runtime has threads and the dispatcher is the managed one. `browser_input_handler.rs` dispatches input directly and says so: the UI thread of the port is the thread of the page, with and without the render worker.
- `HeadlessGlyphRunStub.GlyphTypeface` (`Avalonia.Headless`): the stub of the port is created without the typeface (`HeadlessGlyphRunStub::new`), which nothing reads.

Read together, the 452 members the pages report are:

- 394 in the tooling of phase 4 (rows 2, 3 and 10): real gaps, not scheduled.
- 55 in real gaps that are scheduled work: the external objects of the Skia backend on OpenGL and on Metal with the features the backend context publishes (rows 6 and 9 and `PublicFeatures`, 48 members), three helpers of the Skia backend (row 15) and two small files of the build crate (row 4, 4 members).
- 3 that are not in the Rust code and are too small to schedule (the two of the browser and the one of the headless platform above).

What the first review listed as needing no code, and what was recorded for each row (`docs/porting/data/path-overrides.toml` and `member-waivers.toml`, both under headings dated 2026-10-09; every entry names the Rust member or the design section):

| # | Upstream | Recorded |
|---:|---|---|
| 1 | `src/tools/Avalonia.Generators` (30 files, 262 members) | `not-applicable` entries in nine groups that name their files one by one, with `xaml.md` 3.8 and the section of the counterpart: the host side of a Roslyn generator, the value-equal list, the glob patterns, the name generator and its writers, its resolvers, its diagnostics, its reduced compiler, `RoslynTypeSystem.cs` (the source scanner and `ModelTypeSystem` of the build crate), the property generator (`ferro_property!`). The project has no crate, so no entry can name a Rust file |
| 4 | `src/Avalonia.Build.Tasks` (40 of 44 members) | `replaced` by `Build` of `src/FerroUI.Build.Tasks/lib.rs`: `CompileAvaloniaXamlTask.cs`, `XamlCompilerTaskExecutor.cs` and `.Helpers.cs`, `GenerateAvaloniaResourcesTask.cs`. `not-applicable`: `XamlFileInfo.cs` (serves the index `xaml.md` 6.3 drops), `ComInteropHelper.cs`, `Extensions.cs`. The two files of row 4 above have no entry |
| 5 | `Skia/Avalonia.Skia/Gpu/Vulkan/` (3 files, 31 members) | One `not-applicable` entry per file: out of scope while `Avalonia.Vulkan` is (`scripts/api-extract/projects.json`); removed when a platform with Vulkan is ported |
| 7 | `src/Avalonia.Metal` (21 members) | Tracked in `src/Skia/FerroUI.Skia/metal`, as `scripts/api-extract/projects.json` has said since the contracts were ported there: the two keys of the project (`rust`, `crate`) were added to its record in `upstream-api.json` by hand, exactly as the extractor writes them, because the extractor cannot be run without moving the baseline. Every type and member is found by name |
| 8 | `SecurityScopedStream.cs` (18) | Waivers per group of overrides: `Read`, `Write` and `Seek` of the type and their provided methods; no counterpart for the capability flags, the asynchronous forms and `Begin`/`End` |
| 11 | The storage items and providers (16) | Aliases to the function of the base struct each asynchronous member forwards to (`get_basic_properties`, `get_parent`, `delete`, `move_to`, `save_bookmark`, `release_bookmark`); waivers that name the macro for the rest and the blanket implementation for the two pickers |
| 12 | `SkiaSharpExtensions.cs` (14) | A waiver per conversion with the free function it is (an alias cannot name a free function); `Clone(SKPath)` has no counterpart |
| 13 | The four Skia caches (11 members, 4 types) | A `types` table per file; one waiver for `Shared` (the thread-local of each cache) |
| 14 | The geometries of the Skia backend (10) | Waivers: `fill` of `GeometryImpl`, `bounds` written by `impl_geometry_impl!`, `GeometryImplBase::invalidate_caches` |
| 15 | The two Skia helpers (4 of 7) | Waivers: `wrap_skia_surface`, `save_image_to_file`, `save_image`. The two obsolete `SaveImage` overloads that ignore their quality are not ported and say so |
| 16 | `CompositionInterop.cs` (6) | Waivers that name the member of the server half |
| 17 | The storage and launcher extensions (6 members, 2 types) | A `types` table for `LauncherExtensions` and `StorageProviderExtensions` (the trait objects); waivers for `try_get_local_path` and `file_system_info_full_name` |
| 18 | `Avalonia.Browser` (4 of 6) | An alias (`RenderWorker::start`), waivers for the thread id and for the graphics context of the two render targets; two members not found, above |
| 19 | The character maps and the two glyphs (5) | Waivers: `get_glyph`; vectors the glyph owns |
| 20 | Three files of the markup crates | `Parsers/PropertyParser.cs` mapped to the re-export in `parsers/mod.rs` of its crate, the entry naming the port in `ferroui-markup`; `XamlDocumentTypeBuilderProvider.cs` `replaced` by the trait of `xaml_document_resource.rs`; `AvaloniaResourceXamlInfo.cs` `not-applicable` (`xaml.md` 6.3) |
| below 20 | Ten members | Waivers for the constructors of `HitTestVisitor` and `FuncFramebufferRenderTarget` (with a `types` entry for its delegate), the two members of `HeadlessWindowImpl` in `headless_window_surface.rs`, `ImmutableBitmap::from_pixels`, the second `CreateDrawingContext`, `with_sk_canvas`; two members not found, above |

The order for the next batches: row 9 (Metal external objects, with `GpuHandleWrapFeature.cs` and `PublicFeatures`), then row 6 with Ganesh on the desktop; rows 4 and 15 with the work that touches their files; rows 2, 3 and 10 when the previewer is scheduled.

Resolved from the list the first review kept for a decision:

- `SwapchainBase.DisposeAsync`: the member exists as `SwapchainBase::dispose`, synchronous, and the waiver stays; its reason now says what holds since the render thread. Nothing implements `ISwapchainImage` outside the tests of the file, so no image is released late today, and the first swapchain built on the class has to make the disposal return the task of the render thread.
- `IL/RuntimeContext.cs` of XamlX is a `replaced` entry of its own: the interface is the trait of `emit/xaml_language_emit_mappings.rs` (its seven members are found by name), the generated class is `XamlIlContext` of the runtime library.
- The entries that are not applicable "for now" state the condition that makes them applicable. The two files of `Avalonia.Native` had an entry each, since their conditions differ: `GpuHandleWrapFeature.cs` (3 members) is created by the Metal device of upstream and came with row 9, not with Ganesh; it is ported (`src/FerroUI.Native/gpu_handle_wrap_feature.rs`, branch `small-gaps`) and its entry is gone, the default rule maps the file. `AvaloniaNativeGlPlatformGraphics.cs` (70 members) comes with row 6. `HeadlessUnitTestSession.cs` (10 members) waits for a decision on tests that share one application. These 80 members are work the pages do not count while the entries stand.
- The two waivers of `EglDisplay` name the condition too: an EGL platform that renders on the render thread.
- `Avalonia.Metal` is tracked where it is ported (row 7 of the second table).

Still kept for a decision (each is a judgement on sources or structure, not on data):

- `IPlatformHandle` and `PlatformHandle` are declared twice, in `src/FerroUI.Base/platform/` (where upstream has them) and in `src/FerroUI.Controls/platform/`. One of the two should go.
- `Avalonia.Metal` is a module of the Skia backend where upstream has a project. The tracking follows the module; a crate `ferroui-metal` of its own would mirror the solution layout and is the owner's to decide.
- The output of the name generator is not written: the parts struct of a class with markup and `initialize_component` (`xaml.md` 9.4.3) come with the step `include_xaml!` per class of stage E5. The generator's files are not applicable because that output is new code in the emitter, not a port of them; the entries say so.
- The heading of the waivers of `Rendering/Composition` in `member-waivers.toml` (triage of 2026-10-08) still says that the server compositor runs on the thread of its compositor. The one waiver that gave it as its reason is rewritten; the others under the heading were not read again against the render thread.
- `Rendering/WebRenderTarget.cs` of the browser has a `renamed` entry to the path the default rule gives; it is kept for its reason.

## How to run the next period

- All work is done in the local session (owner, 2026-10-08). No new cloud sessions are started; `CLOUD-WORKERS.md` and `cloud-tasks/` stay as the record of how the cloud workers were briefed, and their rules for an exact port, for tests and for delivery apply to local work unchanged.
- Independent tasks may be split between local sub-agents, at most four at a time and never two on the same files. Each task gets its own branch from the current `origin/main` and its own pull request.
- Sub-agents write and commit without compiling (owner, 2026-10-08): four debug builds of the workspace at once filled the disk. A sub-agent checks every name it uses against the source, commits on its branch, and reports what it doubts will compile. The main session then builds and tests one branch at a time in the main checkout (`git checkout --detach <head>`), where the build cache already is, sends the exact compiler and test output back for fixes, and opens the pull request when the branch passes.
- A pull request is merged with a rebase merge when its three checks are green (`Source conventions`, `Build and test (macOS)`, `Check (browser, wasm32-unknown-emscripten)`). The macOS job takes about 40 minutes.
- When `ci.yml` does not start for a pull request, dispatch it by hand on the branch.
- After a toolchain change on `main`, re-run `scripts/browser/setup.sh` before any browser build. The toolchain is installed under `.tools/`; `source .tools/env.sh` before `scripts/build-browser.sh`.
- The generated tracking pages (`TRACKING.md`, `tracking/*.md`) are regenerated with `scripts/port-status/run.sh` in a pull request of their own when no other pull request is open, since every regeneration touches the same files. `run.sh` runs the extractor again when the reference checkout is not at the tracked commit, which moves the baseline: to regenerate against the tracked commit, run `python3 scripts/port-status/port_status.py` alone. Run it twice; the second run must change nothing.
