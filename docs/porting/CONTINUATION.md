# Continuation

This page is the hand-over point for the next working period: what was finished, what is in flight, the tasks that come next in order, and how to run them. Read it after `CLOUD-WORKERS.md`, which stays the standing brief of every worker. Update it at the end of each period.

## State at the hand-over of 2026-10-07

Merged this period, in order: #26, #34, #33, #31, #27, #32, #29, #30, #36, #35, #28, #37, #38, #40, #41, #39.

- Browser: Rust 1.99.0 with WebAssembly exceptions (#30). The ControlCatalog site loads its pictures and fonts on demand, per page (#31). The theme documents are out of the catalog's module (#35). Catalog module 35.28 MB raw, 10.62 MB gzip -9; first frame about 1.2 s unthrottled.
- TableView and Buttons: the per-event cost was the JavaScript exception trampolines (removed by #30) and copies and allocations the port made and upstream does not (#29, recorded under "Corrected divergences" in `DEVIATIONS.md`). Measured after both: TableView 8.4 to 9.0 ms per scroll event (budget 8.3 ms), Buttons 2.4 to 3.1 ms. What remains in TableView is upstream's own cell rebuild on recycled rows.
- XAML compiler: shared runtime helpers and split functions in generated code (#28, #39). The themed browser module is 1.9 % smaller raw. The generated code is deterministic: #39 fixed a hash-order bug, and the drift tests compare against committed output.
- Deviation register: `DEVIATIONS.md` (#33). Every deliberate difference from upstream gets a row there, or in the area's own page, plus a comment at its site.
- Core port: the ContentPresenter suites as exact ports of upstream's four files (#40). The ComplexControl and Foundation automation peer suites complete (#41).

## In flight at the hand-over

Each was asked to stop at a clean, tested state, open its pull request with a "Continuation" section, and mark it ready when CI is green. Read that section before continuing a task. A draft pull request means the work did not reach a clean state.

| Branch | Task | Brief |
|---|---|---|
| `browser-plain-assets` | The catalog's pictures and fonts as plain files in the site, fetched and registered one by one, instead of packed `.assets` bundles (owner's request). On-demand loading per page stays. | Task 1 below |
| `xaml-e5-loader-table` | XAML compiler stage E5: loader table, includes across crates, then the catalog compiled | Task 2 below |
| `core-port-11` (if it exists) | Next batch of upstream test suites | Task 3 below |

## Next tasks, in order

### 1. Catalog assets as plain files

Finish what the `browser-plain-assets` pull request leaves, using its "Continuation" section. The goal:
- The original files are copied byte for byte into the site under paths that mirror their asset paths.
- A page's files are fetched before the catalog creates that page, and the rest are prefetched while idle.
- The asset loader stays synchronous. A single "register one asset" entry point replaces the bundle format.
- `register_asset_bundle`, `asset_bundle.rs` and the packing step are gone.

Measure the site size, the first frame (unthrottled and `--throttle 50,40`) and the number of start-up requests. Record the numbers in `browser-platform.md`. Do not reintroduce packing.

### 2. XAML compiler stage E5

The stage-table row "E5 Includes and assets" in `xaml.md`, and items 3 and 4 of `browser-platform.md` section 20. In order:

1. **Loader table.** The generated code answers a load by URI of every compiled document, as upstream's `!XamlLoader` does. Compiled documents then leave the embedded assets for every application, not only the catalog's browser host. This removes the `remove-compiled-documents` feature of the theme crates and its deviation row in `browser-platform.md` section 14.
2. **Includes across crates.** `StyleInclude`, `ResourceInclude` and `MergeResourceInclude` resolve the compiled documents of another crate at build time. The test is a fixture of two crates: a theme library and an application.
3. **The catalog compiled.** Its 219 documents and the `x:Class` documents of the dialogs, so that neither links the run-time loader. The estimate is +6 to +10 MB raw and +0.5 to +1.2 MB gzip on the catalog module, against 0.24 s less CPU before the first frame. Measure a few pages first. The owner decides on the merge if the growth exceeds the estimate.

Regenerating output:
- Corpus: `cargo test -p ferroui-markup-xaml-tests --lib emitter::differential_tests::regenerate_emitter_output -- --ignored --exact`.
- Themes: `cargo test -p ferroui-themes-<fluent|simple> --lib tests::compiled_xaml_tests::regenerate_compiled_xaml -- --ignored`.
- Regenerate twice and diff before every push.

### 3. Core port: the remaining upstream suites

Continue with the suites the core-port pull requests list as next. Each suite is an exact port under upstream's names, with framework fixes where a test exposes a difference. Known items:

- **Blocked:** `ColorSpectrumAutomationPeerTests.cs` (6 tests). `ColorSpectrum` and its peer are not ported, because the color picker library has no crate yet.
- `deferred_text_tests.rs` and `content_presenter_text_tests.rs` duplicate some ContentPresenter tests. Fold them into the exact ports of #40.
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

- **Browser opt-level.** A faster opt-level for the hot crates of the browser build would buy frame rate at the cost of module size. The size and first-frame figures for opt-level 3, "s" and "z" are in `browser-platform.md` section 18. The `browser` profile uses "z" outside the XAML pipeline.
- **CI speed-up.** The earlier proposal for a faster CI run is still open.
- **Stale branches.** Branches of merged pull requests can be deleted by hand.

## How to run the next period

- One cloud session per task, created from `main`, with the task's brief as its first message and `CLOUD-WORKERS.md` as the standing brief. Sessions in one environment do not share a target directory, so they do not contaminate each other's builds.
- Each session opens a draft pull request early, pushes often, rebases onto `main` before its final push, and marks the pull request ready when CI is green. Pull requests are merged with rebase merges.
- When `ci.yml` does not start for a pull request, dispatch it by hand on the branch.
- After a toolchain change on `main`, re-run `scripts/browser/setup.sh` before any browser build.
