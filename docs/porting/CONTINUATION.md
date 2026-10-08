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

One cloud worker per row, started from `main` with the brief named.

| Branch | Task | Brief |
|---|---|---|
| `xaml-e5-includes` | XAML compiler stage E5, includes across crates (task 2, remaining step 1) | `cloud-tasks/xaml-e5-includes.md` |
| `color-picker` | The colour picker library, its themes, tests and catalog page | `cloud-tasks/color-picker.md` |
| `core-port-12` | Color, focus and glyph typeface suites and the clean-ups of task 3 | `cloud-tasks/core-port-suites.md` |
| `skia-test-suites` | The remaining suites of the Skia backend and the typeface hook (row 16) | `cloud-tasks/skia-test-suites.md` |

Merged on 2026-10-08, done in the local session because they are macOS only: the native storage provider with file and URL activation and files on the clipboard (#50), and the native control host (#54). Both advance row 18 of `CRITICAL-PATH.md`; neither was exercised against AppKit beyond start-up (the pickers need a user, and nothing in the port hosts a native view yet).

The four workers above are the last cloud workers: the owner decided on 2026-10-08 that all work is done locally once they have finished. See "How to run the next period".

Next in line, locally, when a worker has finished: build integration of the XAML compiler (task 2, remaining step 2), the documents still on `samples/ControlCatalog/excluded.txt`, and design 09 of `performance/` (benchmarks and counters).

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

Remaining, in order. #43's "Continuation" section has the upstream files for each step.

1. **Build integration.**
   - `compile_xaml()` / `embed_assets()` (`xaml.md` 9.6) replace the checked-in `compiled_xaml.rs`, with `register()` (R8) and the generated `register_types` of 9.7.4.
   - The constructor choice of the seam moves into the compiler.
   - The `.xamlmeta` files travel through Cargo `links` metadata (`DEP_<CRATE>_XAML_XAMLMETA`, `xaml.md` 9.6.3); today each generator names the checked-in files of its dependencies (`DEPENDENCIES` in the fixture application's `tests/compiled_xaml_tests.rs`).
   - The `XamlIlTests` that load compiled documents of the test assembly can then be ported.
2. **The catalog compiled.**
   - Its 219 documents and the `x:Class` documents of `src/FerroUI.Dialogs`, so that neither links the run-time loader.
   - The estimate is +6 to +10 MB raw and +0.5 to +1.2 MB gzip on the catalog module, against 0.24 s less CPU before the first frame.
   - Measure a few pages first. If the growth exceeds the estimate, the owner decides on the merge.
3. **Measure #43.** Measure `themed_view` and `control-catalog-browser` for #43's change (not measured yet) and add the table to section 20. `themed_view` is expected to lose the 168 theme documents, as the catalog did in #35.

Regenerating output:
- Corpus: `cargo test -p ferroui-markup-xaml-tests --lib emitter::differential_tests::regenerate_emitter_output -- --ignored --exact`.
- Themes: `cargo test -p ferroui-themes-<fluent|simple> --lib tests::compiled_xaml_tests::regenerate_compiled_xaml -- --ignored`.
- Include fixture (library first, the application reads its `.xamlmeta`): `cargo test -p xaml-include-fixture-<theme|application> --lib tests::compiled_xaml_tests::regenerate_compiled_xaml -- --ignored`.
- Regenerate twice and diff before every push.

### 3. Core port: the remaining upstream suites

Each suite is an exact port under upstream's names, with framework fixes where a test exposes a difference. The counts are the upstream tests missing by name on `a382956`, from the name inventory of the core-port worker. In order:

1. `Media/ColorTests.cs`: 18 of 66 missing.
   - Today the port has an inline `mod tests` in `src/FerroUI.Base/media/color.rs` (64 tests, some under names upstream does not use) and 2 tests each in `hsl_color.rs` and `hsv_color.rs`.
   - Write an exact `media/color_tests.rs` and remove the inline duplicates.
   - Fix any Color, HSL or HSV divergence the new tests expose.
   - No blocker is known.
2. `Input/InputElement_Focus.cs`: 21 of 46 missing. It is large (1,747 lines). Expect gaps in the focus manager.
3. `Media/GlyphTypefaceTests.cs`: 26 of 42 missing. It may need font-table test assets.
4. `Data/Core/Parsers/BindingExpressionVisitorTests.cs`: all 36 missing. Upstream walks LINQ expression trees, so this needs a design decision first: what in the Rust compiled-binding path parser stands for them. Expect a deviation entry, not a straight port.

Also:

- **Blocked:** `Automation/ColorSpectrumAutomationPeerTests.cs` (6 tests). `ColorSpectrum` and its peer are not ported, because the color picker library has no crate yet (`ColorSpectrum.cs` alone is about 1,740 lines).
- `deferred_text_tests.rs` and `presenters/content_presenter_text_tests.rs` duplicate ContentPresenter tests that #40 ported exactly. Remove the duplicates.
- **Templates:** the gaps against `TypeUtilities.CanCast<T>` that #40 recorded in `DEVIATIONS.md` (null data and reference types, `int` against `int?`).
- **Layout** rows of `DEVIATIONS.md`:
  - `LayoutManager` does not call `Dispatcher.VerifyAccess()`. It can be ported now.
  - The layout clock is in milliseconds.

### 4. Remaining compositor pieces

`cloud-tasks/composition-animations.md`, and row 12 of `CRITICAL-PATH.md`.

### 5. Browser follow-ups

- `src/Skia/FerroUI.Skia/emscripten/emscripten_sjlj.cpp` bridges Skia's prebuilt sjlj objects to WebAssembly exceptions. Remove it once rust-skia publishes Emscripten binaries built with `-sSUPPORT_LONGJMP=wasm` (recorded in `browser-platform.md` section 14).
- TableView is still slightly over its 8.3 ms budget per event. The rest is upstream's own work. Further gains would come from the build (see owner decisions), not from changing ported logic.

## Decisions waiting for the owner

- **CI speed-up.** The earlier proposal for a faster CI run is still open.
- **Stale branches.** Branches of merged pull requests can be deleted by hand.

## How to run the next period

- All work is done in the local session (owner, 2026-10-08). No new cloud sessions are started; `CLOUD-WORKERS.md` and `cloud-tasks/` stay as the record of how the cloud workers were briefed, and their rules for an exact port, for tests and for delivery apply to local work unchanged.
- Independent tasks may be split between local sub-agents, at most four at a time and never two on the same files. Each task gets its own branch from the current `origin/main` and its own pull request.
- A pull request is merged with a rebase merge when its three checks are green (`Source conventions`, `Build and test (macOS)`, `Check (browser, wasm32-unknown-emscripten)`). The macOS job takes about 40 minutes.
- When `ci.yml` does not start for a pull request, dispatch it by hand on the branch.
- After a toolchain change on `main`, re-run `scripts/browser/setup.sh` before any browser build. The toolchain is installed under `.tools/`; `source .tools/env.sh` before `scripts/build-browser.sh`.
- The generated tracking pages (`TRACKING.md`, `tracking/*.md`) are regenerated with `scripts/port-status/run.sh` in a pull request of their own when no other pull request is open, since every regeneration touches the same files.
