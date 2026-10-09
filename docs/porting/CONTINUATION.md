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

## In flight on 2026-10-08

Nothing runs in the cloud; the four cloud workers of the morning delivered and were the last ones. In flight locally, each on its own branch, written by a sub-agent without a compiler and validated in the main checkout: `b2-1-render-worker-clear`, `b2-4-strict-confinement`, `markup-gaps-types`, `markup-gaps-events`. Open pull requests: 91 (the two-mode frame benchmark) and 92 (the OpenGL page of the catalog). See "Critical now" below for what each is.

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
- **Stale tracking pages:** regenerate `Avalonia.Markup.Xaml.md` and `Avalonia.Markup.Xaml.Loader.md`. They are stale since the emitter work of #39 and would gain `Statement` and `DocumentInfo`.

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

**Done: step 3, build integration over the run-time type system (branch `xaml-e5-build-integration`, not validated by a build yet).** `xaml.md` 9.6.8 has the design as built and the table of what departs from 9.6.
- `ferroui-build` (`src/FerroUI.Build.Tasks`): `Build::from_env().assembly(..).embed_assets(..).compile_xaml().run()` for a crate's `build.rs`. It writes the generated modules, the asset table, the loader table and `register()` (R8) to `$OUT_DIR/xaml/`, writes `$OUT_DIR/<crate>.xamlmeta`, and prints the `cargo::` lines. `ferroui_markup_xaml::include_compiled_xaml!()` includes the result.
- The `.xamlmeta` files travel through Cargo `links` metadata (`DEP_<CRATE>_XAML_XAMLMETA`). The themes export their checked-in file that way from their existing `build.rs`.
- The constructor of a class is picked by the compiler (`generate_class_file(class, None, ..)`), as upstream picks it. The themes still state theirs, because their markup metadata declares a `new()` upstream's class does not have.
- The include fixture is converted: its generated files are no longer checked in, except the document of the class `StyleWithServiceProvider`.
- The limit: the compiler reads types from the registries of the process, so the build script links the crates its documents name, and it cannot link its own crate. A document that names a type of its own crate (every `x:Class` document) cannot be compiled by a build script. That is why the themes are not converted and why `LocaleCollection` moved from the fixture application to the fixture library.

Remaining, in order. #43's "Continuation" section has the upstream files for each step.

1. **The build-time type system** (`xaml.md` 9.5: the source scanner, `MarkupModel`, `ModelTypeSystem<EmitBacking>`, `export_metadata()`).
   - It removes the limit above: the build script compiles `x:Class` documents without linking anything.
   - Then: `include_xaml!` per class, the generated `register_types` of 9.7.4, the themes on the build script, diagnostics with codes and the cache (9.6.4, 9.6.5).
   - Until then a crate with `x:Class` documents uses the checked-in path (`generate_class_file` in a test). The two `XamlIlTests` classes with compiled markup (`XamlIlClassWithPrecompiledXaml`, `XamlIlClassWithCustomProperty`) can be compiled that way; today their constructors populate through the run-time loader; the two tests are ported and not ignored.
   - Small and independent: remove `new:` from the markup metadata of the two themes so that the compiler picks their constructor too (check with a build that `<FluentTheme/>` in markup goes through `FerroXamlIlConstructorServiceProviderTransformer`).
2. **The catalog compiled.**
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

## Critical now: the render thread (owner, 2026-10-08)

The owner's order on 2026-10-08: finish the work in flight, then the render thread, then the scheduled work. Design, stages and what each step found: `docs/porting/render-thread.md`; the browser stage B2: `docs/porting/browser-render-worker.md`.

State on 2026-10-08, late:

- **Desktop: done and the default.** Stages R1 to R5.7 are on `main` (thread-safe render resources, `Send` jobs, the render-thread mode, the synchronous wait, the compositor lock, render surfaces shared between the threads, atomic native reference counts, the Metal audit, GPU interop objects confined to the lock, the shared image import). `Compositor::new` chooses the render-thread mode wherever the render loop runs in the background; `FERROUI_RENDER_THREAD=0` is the way back (pull request 89).
- **Desktop, still open:** interaction by hand and a stress run in the render-thread mode; the measurements repeated on a quiet machine (the two-mode frame benchmark is pull request 91); the two latent items of the audit are closed as written, to be validated (`render-thread.md`, R5.8: the platform graphics are `Arc<dyn IPlatformGraphics>` with `Send + Sync`, and `try_get_render_interface_feature` hands out a handle bound to the compositor lock).
- **Browser B1 is on `main`** (pull request 88): `scripts/build-browser.sh <example> --threads`, the isolation service worker, the example `thread_spawn`. The threaded link passes `--no-check-features` because the prebuilt Skia bindings shim has no atomics; the proper fix is to compile the shim with `-pthread`.
- **Browser B2 is designed and started.** Owner decision: the UI stays on the browser's main thread and only rendering moves to a worker, with strict confinement of the render target to that worker. Steps B2.1 to B2.8 are the table of section 6 of the design. In flight: B2.1 (`render_worker_clear`, branch `b2-1-render-worker-clear`) and B2.4 (the base library's strict mode, branch `b2-4-strict-confinement`), each written by a sub-agent and validated in the main checkout. Then B2.2, B2.3, B2.5 to B2.8 in order, then the measurements of B3.
- **Scheduled work running beside it:** the markup gaps that block catalog pages, in two groups (`markup-gaps-types`: C305, C309, C201, C306, C208, C102; `markup-gaps-events`: C301, C203, C312, C207, C202); the OpenGL page of the catalog is pull request 92.
- After those: the build integration of the XAML compiler (task 2, remaining step 2), design 09 of `performance/`, the review of waivers and doubly mapped files with one regeneration of the tracking pages, the Metal external objects feature, the EGL seams, Ganesh GL on the desktop.

## Core first (owner, 2026-10-08): done

The owner's order of 2026-10-08 was to finish the core port first, with all four local sub-agents on it. The whole queue (the base library by area, `ferroui-controls`, the markup crates and the run-time loader, the Skia and macOS backends, the headless platform) was written without a compiler, integrated and validated in one build, and merged as pull request 66, with the follow-ups in pull requests 65 (the Metal external objects contracts) and 68. Most of what the tracking pages listed as missing was a false negative of their name matching; what was absent is ported, and the rest is recorded in `docs/porting/data/path-overrides.toml` and `member-waivers.toml`.

Left from it: regenerate the tracking pages once (`scripts/port-status/run.sh`) in a pull request of their own, and take the remaining gaps from them.

## How to run the next period

- All work is done in the local session (owner, 2026-10-08). No new cloud sessions are started; `CLOUD-WORKERS.md` and `cloud-tasks/` stay as the record of how the cloud workers were briefed, and their rules for an exact port, for tests and for delivery apply to local work unchanged.
- Independent tasks may be split between local sub-agents, at most four at a time and never two on the same files. Each task gets its own branch from the current `origin/main` and its own pull request.
- Sub-agents write and commit without compiling (owner, 2026-10-08): four debug builds of the workspace at once filled the disk. A sub-agent checks every name it uses against the source, commits on its branch, and reports what it doubts will compile. The main session then builds and tests one branch at a time in the main checkout (`git checkout --detach <head>`), where the build cache already is, sends the exact compiler and test output back for fixes, and opens the pull request when the branch passes.
- A pull request is merged with a rebase merge when its three checks are green (`Source conventions`, `Build and test (macOS)`, `Check (browser, wasm32-unknown-emscripten)`). The macOS job takes about 40 minutes.
- When `ci.yml` does not start for a pull request, dispatch it by hand on the branch.
- After a toolchain change on `main`, re-run `scripts/browser/setup.sh` before any browser build. The toolchain is installed under `.tools/`; `source .tools/env.sh` before `scripts/build-browser.sh`.
- The generated tracking pages (`TRACKING.md`, `tracking/*.md`) are regenerated with `scripts/port-status/run.sh` in a pull request of their own when no other pull request is open, since every regeneration touches the same files.
