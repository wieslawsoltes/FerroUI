# FerroDesign

A design editor built with **FerroUI, Rust, compiled XAML and the FerroUI Vello backend**. The same editor, controls, document model and scene renderer run in a browser and in the native launcher.

The editor includes pages and layers, an infinite canvas, stacked layout guides, property-based selection, editable vector networks with a paint bucket, rich text, image cropping, native effect/mask controls, continuous box corners, rounded polygons/stars, variable-width strokes, independent paragraph formatting, cross-frame auto-layout dragging, component variants and typed properties, visual local variables, animated prototypes, comments, undo/redo and portable export. The compiled XAML shell follows the navigation rail, contextual left sidebar, canvas, property sidebar and floating toolbar arrangement of Figma Design. [Feature coverage](docs/FEATURES.md) records the implementation and its limits; this repository does not claim complete Figma compatibility or verified pixel parity.

The [layout guide](docs/LAYOUT_GUIDES.md), [matching selection](docs/SELECTION.md), [angular paint](docs/ANGULAR_PAINTS.md) and [vector face and bucket](docs/VECTOR_FACES.md) guides describe their authoring contracts, performance boundaries and remaining gaps.

The editor is real FerroUI UI. `Editor.xaml` compiles into native FerroUI controls; `DesignCanvas` derives from `Control`; artwork is drawn through the public `DrawingContext`, geometry, brush, pen, text and imaging APIs. The HTML host only starts WebAssembly and provides browser storage, clipboard and downloads.

## Get the complete source

This sample is an independent Cargo workspace inside the FerroUI repository. Keep the repository around it: its path dependencies refer to FerroUI's actual source, not to a separate graphics implementation.

```sh
git clone --depth 1 --branch feat/ferrodesign-editor --single-branch https://github.com/wieslawsoltes/FerroUI.git
cd FerroUI/samples/FerroDesign
```

The implementation began from upstream commit `325b3ce4b0fcf154eba79cbd92262536daf77e71` and incorporates upstream `ab7a15b8641a54c8964b6ec10a3efe9af38d29b5` (FerroUI 0.4.0-preview.1), including its Vello performance improvements. The branch contains the complete application, independent browser backend selection, native text-input fixes, public isolated compositing and editable geometry/text-outline APIs. [PR #254](https://github.com/wieslawsoltes/FerroUI/pull/254) collects the branch changes.

The [source-delivery audit at `b83deee6`](docs/evidence/2026-10-22-f2bb1d61/source-delivery-audit.json) found the required framework path dependencies and assets in the tracked repository, with no submodules, LFS pointers or build-script dependence on FerroUI's `.git` directory. Extract the complete repository, including its root `.cargo` configuration. This was a source-tree audit, not a build from an independently downloaded ZIP; toolchain and third-party dependency downloads are still required.

## Build and run in a browser

Required host tools: a C/C++ build toolchain, `git`, `curl`, Python 3, and Node.js 22/npm (the CI configuration), plus Bash, `tar`, `install` and either `sha256sum` or `shasum`. The setup script installs its own pinned Rust and Emscripten tools without editing shell profiles.

```sh
./scripts/setup.sh
source .tools/env.sh
./scripts/build-browser.sh
python3 -m http.server 8080 --directory dist
```

Open <http://localhost:8080>. The standard browser configuration requests **FerroUI Vello hybrid on WebGL2** and falls back to **Vello CPU with Software2D presentation**. To request an exact surface, open <http://localhost:8080/?RenderingMode=Software2D> or <http://localhost:8080/?RenderingMode=WebGL2>. An explicit mode does not fall back to a different surface. WebGL1 is rejected because Vello does not implement that surface. The renderer itself is Vello in this distribution.

The generated `dist` directory is a static distribution containing the application JavaScript, WebAssembly, FerroUI browser bridge, resources and dependency manifest. Serve it over HTTP; opening `index.html` through `file://` does not provide a usable WebAssembly application origin. This build does not enable the FerroUI service worker.

The pinned browser tools are Rust **1.99.0**, Emscripten **6.0.10** and `wasm-bindgen-cli` **0.2.129**. `Cargo.lock` and the browser validation package lock are committed. The build script requires the **active target dependency graph** to contain FerroUI Vello and rejects `ferroui-skia`, `skia-safe` or `skia-bindings`. The first build needs access to the Rust package registry.

Setup prefers official, pinned SHA-256-verified wasm-bindgen binaries on Linux and macOS for x86_64 and aarch64. Unsupported targets or unavailable downloads retain the locked Cargo installation; a checksum mismatch stops setup before extraction or execution.

The first build is substantial because it compiles FerroUI itself. The browser profile builds one job at a time, splits code generation and avoids LTO to control peak memory. Vello and its rasterization dependencies retain optimization level 3. The repository's Emscripten configuration links with `em++`. The build retains unresolved-symbol diagnostics. Do not increase parallelism on a memory-constrained runner before measuring peak memory.

Useful build options:

```sh
./scripts/build-browser.sh --out /tmp/ferrodesign-dist
./scripts/build-browser.sh --skip-webapp  # when the unchanged FerroUI JS bridge is already built
./scripts/build-browser.sh --debug
./scripts/check-renderer.sh             # audit all five supported target graphs
```

## Native launcher

Use Rust **1.99.0**, as required by this sample's Cargo manifest, and the platform prerequisites documented by the parent FerroUI repository. The setup environment above supplies the pinned toolchain for native builds as well. Linux native linking needs a C++ toolchain, `pkg-config`, and the Fontconfig and FreeType development packages (`libfontconfig1-dev` and `libfreetype6-dev` on Ubuntu). Windows uses the system OpenGL provider with a CPU framebuffer fallback, without the optional ANGLE feature. macOS uses the Apple SDK and Metal backend.

```sh
cargo run --locked -p ferrodesign-app
cargo run --locked -p ferrodesign-app -- --renderer-info
cargo run --locked -p ferrodesign-app -- --software
cargo run --locked -p ferrodesign-app -- --render-smoke /tmp/ferrodesign.png
```

The native configuration uses Vello hybrid with Metal on macOS, GLX on Linux/X11, and WGL on Windows. Each platform retains a Vello CPU framebuffer fallback. `--software` requests the CPU configuration. The headless smoke command draws the compiled-XAML application through a real Vello render target and writes PNG. These are configured platform paths; the source-tagged validation section records which builds and executions have actually completed.

Vello is selected through FerroUI's public `BrowserPlatformOptions` and `VelloApplicationExtensions`. Document drawing continues through FerroUI `DrawingContext`, retained geometry/brushes, `TextLayout`, compositing scopes and `RenderTargetBitmap`. The application does not issue backend-specific drawing calls. The retained scene, viewport bounds hierarchy, editing transactions and compiled XAML controls are shared across GPU and raster presentation.

## Editing

The initial document contains editable Orbit mobile banking screens, a component page, styles, variables, comments and a linked prototype. Click a layer on the canvas or in the left sidebar; the right sidebar edits the selected layer. Empty-canvas selection exposes the page background.

Select an auto-layout frame or one of its children, then choose **Layout and grid placement…** in the inspector. Native controls edit child sizing, min/max limits, growth, absolute positioning, one-based grid cells and row/column spans. **Move before/after** follows the visual flow, including reversed layouts; pinned grid children keep their cells.

Select one or several layers and drag between layout frames, ordinary frames or the active page. An outline and insertion line preview the destination in rows, columns, wrapped rows and grids, including reversed flow and transformed frames. The document stays unchanged during a layout insertion preview; release commits reparenting, transformed placement and sibling layout as one undo step. Returning to the original slot, Escape or lost pointer capture cancels without a document edit. Automatic grid insertion follows flow order; use the placement dialog for pinned cells. Frame track lists accept values such as `120, auto, 2fr` for fixed, content-sized and fractional tracks.

**Layout grid** in the inspector, or **Layout grid guides…** in quick actions, opens native guide controls for a frame, component or instance. Stack square, column and row guides; edit size, count or Auto, alignment, gutter and margin/offset; and change visibility, order, duplication or removal. Each guide has one RGBA hex field, including its alpha value. Shared grid styles propagate geometry and color while preserving each linked frame's per-index visibility. These are visual guides: child placement uses the separate layout solver, and guide ink is excluded from artwork exports and presentation. [Layout guides](docs/LAYOUT_GUIDES.md) specifies the backward-compatible JSON, component behavior, drawing limits and snapping, guide-relative constraints and typed variable bindings.

**Select same…** in the inspector and **Select same properties…** in the layer menu or quick actions select matching appearance, fill, stroke, effect, text, font or component-instance properties. Multiple selected values form a union across eligible layers on the current page; hidden or locked ancestors exclude their descendants. The matching command changes selection without changing the document, revision, history or redo. **Select inverse** selects the other eligible page-level roots, also excluding the page-level ancestor of an eligible selected descendant. The [selection contract](docs/SELECTION.md) defines the exact comparison rules, rich-text resolution and hierarchy behavior.

| Action | Shortcut |
| --- | --- |
| Select / frame / rectangle / ellipse / line | `V` / `F` / `R` / `O` / `L` |
| Pen / freehand pencil / text | `P` / `Shift+P` / `T` |
| Paint bucket while editing a vector network | `Shift+B` |
| Hand / comment / section | `H` / `C` / `Shift+S` |
| Auto layout / component / place image | `Shift+A` / `Ctrl`/`Cmd+Alt+K` / `Ctrl`/`Cmd+Shift+K` |
| Pan | Hold `Space` and drag, middle-button drag, or scroll |
| Zoom around the pointer | `Ctrl`/`Cmd` + scroll |
| Fit page / fit selection | `1` / `2` |
| Undo / redo | `Ctrl`/`Cmd+Z` / `Ctrl`/`Cmd+Shift+Z` |
| Copy / cut / paste / duplicate | `Ctrl`/`Cmd+C` / `X` / `V` / `D` |
| Group / ungroup | `Ctrl`/`Cmd+G` / `Ctrl`/`Cmd+Shift+G` |
| Move selection | Arrow keys; hold `Shift` for 10 units |
| Parent / children / siblings | `Shift+Enter` / `Enter` / `Tab` and `Shift+Tab` |
| Select inverse | `Ctrl`/`Cmd+Shift+A` |
| Cycle overlapping layers | `Alt`+click / `Shift+Alt`+click, or `Alt+[` / `Alt+]` |
| Deep selection with Select groups enabled | `Ctrl`/`Cmd`+click; double-click descends one group |
| Quick actions | `K` or `Ctrl`/`Cmd+K` |
| Save / open | `Ctrl`/`Cmd+S` / `Ctrl`/`Cmd+O` |
| Cancel gesture / leave presentation | `Escape` |

In presentation, an authored Escape key rule takes priority; otherwise Escape returns to the editor.

Shortcuts are handled when the canvas has focus; text fields retain their normal text-editing keys. Shift constrains drawing and resizing, locks a move to an axis, and snaps rotation to 15-degree increments. Dragging snaps to pixels and nearby object edges/centers; quick actions can toggle snapping. The pen creates editable vector paths: click for corners, drag to create Bézier tangents, and finish with Enter or by clicking the first point. Select a vector and press Enter or double-click it to edit anchors. Drag anchors or control handles, double-click a curve to split it, and use Corner/Smooth/Mirror in the sidebar. An entire drag is one undo step.

The vector sidebar also offers Bend and Erase tools. Smooth and mirrored tangents remain constrained during later edits. Convert a path to a vector network to connect branches, shift-select vertices in boundary order, fill a region, join or weld vertices. Split edges retain stable identities and preserve their curves; the complete operation participates in document history.

While editing a vector network, **Paint bucket** or `Shift+B` previews the hovered bounded face. Press and drag to apply the complete layer fill stack captured at the start of the gesture; hold Shift before pressing to clear crossed faces instead. A changed stroke commits one undo step. Escape or lost pointer capture rolls it back, and an unchanged stroke adds no history. Face queries include crossings between pointer samples so a fast drag can reach narrow faces. **Fill all closed regions** remains a separate command. Checked planar topology preserves original lines, Bézier curves and stable identities; unsupported unsplit crossings produce an error. See [vector faces and bucket painting](docs/VECTOR_FACES.md) for explicit region paints, inheritance and geometry limits.

**Flatten selection** converts supported shapes, text and Boolean results into editable paths using the actual FerroUI geometry. Positioned glyph contours and text decorations follow the same shaped layout as rendering. Single-layer conversion preserves its ID, parent, stacking, transform and appearance; compatible opaque siblings can be combined in one undo step. A selection that cannot retain its appearance is rejected without changing the document or history. This includes multi-layer selections with different paint stacks, independent child opacity/effects, and bitmap or color-font glyphs without a complete monochrome outline. See the [flattening limits](docs/FEATURES.md#rendering-and-authoring-limits) for the supported subset.

Double-click a text layer to edit directly on the canvas with a native FerroUI caret, selection, clipboard and IME. Ctrl/Cmd+Enter or moving focus commits one text transaction; Escape restores the original content. Pending IME preedit stays outside the document draft, so lifecycle saves retain the last confirmed text and formatting. Rich-text ranges remain valid UTF-8 byte ranges across edits. **Style selected characters…** opens a native range-selection editor with character styles and a rich preview. The same rich fonts, sizes, weights, colors and decorations now appear during native on-canvas editing. The rich-text editor also provides LTR/RTL/first-strong direction and validated OpenType feature tags such as `liga=0, ss01=1`.

The active native text editor uses the first visible foreground paint for each run. Multiple fill stacks, effects and ancestor masks/blending remain canvas-renderer features outside the editing overlay. Automatic direction uses the first strong character of the layer; independent paragraph direction and paragraph-specific range layout remain unimplemented.

Use **Affine transform and skew…** for complete parent-relative matrix values and centered skew without flattening children. The constraint diagram controls edge pins and centering during parent resizing. **Frame presets…** creates editable phone, tablet, desktop, presentation or social frames. Authoring palettes can be dragged by their native headers and keep canvas input available; they follow the current selection and stay within the window.

Use the property sidebar for geometry, fills, strokes, typography and layout. With no layer selected it shows the page background and local styles, variables and components. The paint-stack dialog edits fill order, visibility/opacity, gradients, stops and image source windows. **Crop image…** provides native pan and corner gestures for Fill/Stretch images; Escape cancels the active gesture, and Undo restores the whole gesture. Numeric crop windows are also available for Fit and Tile.

The **Effects** editor provides native controls for drop shadow, inner shadow, layer blur and background blur. Edit blur radius, signed shadow offsets/spread, RGB and separate opacity; show or hide a row, move it up/down, duplicate, remove or clear effects. Enter or leaving a field applies its value, and Escape restores the committed value. Numeric arrow keys step by 1, with Shift for 10 and Alt for 0.1. Multi-selection changes matching effect rows on unlocked layers; missing rows remain unchanged. Each accepted command is one history step, and invalid or unchanged input preserves history and bound styles.

The Appearance blend selector controls isolated or pass-through layer compositing. **Masks and compositing…** enables vector, alpha or luminance masks; **Create mask group** wraps selected siblings into one undoable group. Mask-aware selection skips transparent content and vector holes. Vello renders these operations through the same public FerroUI compositing and imaging APIs. Layer opacity applies once to the completed group. The first listed shadow of each kind paints above later shadows of that kind. Leaf shapes paint inner shadows between fills and strokes; container inner shadows sit above their compound content. Background blur samples earlier document content within the isolated group and caches its filtered result; [documented bounds](docs/FEATURES.md#rendering-and-authoring-limits) keep its allocations predictable. Shapes, ordinary fills, strokes and text continue to use native vector rendering.

**Properties and variants…** creates component sets, assigns variant combinations and exposes text, visibility and nested instance-swap properties. Switching a variant preserves matching layer IDs and explicit overrides. The native variable editor manages collections, modes, typed values, aliases and layer bindings. Invalid authoring changes are rejected atomically.

**Local styles** creates fill, text, effect and grid styles from the selection. **Edit** opens native fields for name, description and the complete supported style definition, including ordered paint/effect stacks, typography and layout guides. Definition changes update linked layers across pages; **Replace from selection**, **Detach selected**, and **Delete style** support the rest of the lifecycle. Detaching or deleting preserves each layer's effective appearance. Explicit application and detachment respect locks and compatible layer types; definition changes also reach locked linked layers. Editing a bound property directly makes that property independent, while moving a layer retains its styles. Each accepted command is one undo step, and invalid or unchanged drafts preserve history.

In the Prototype tab, **Edit interactions…** adds trigger/action rules and transitions. Presentation supports animated transitions, Smart Animate, reversible hover/press interactions, scroll containers, wheel scrolling and ScrollTo. Preview changes operate on disposable state; the authored design is restored when presentation ends and remains the source for autosaves. `Layer data…` remains available for validated structured properties without a dedicated control.

## Files and local persistence

The native editable format is a versioned `.ferrodesign` JSON document. It stores the complete document graph, paints, layout, components, variable collections, comments and prototype links. Edits coalesce before serializing the full document as compact JSON for local autosave; portable downloads use the readable pretty format. Both read the authored snapshot during presentation. The browser queues snapshots in IndexedDB. A completed transaction updates the current and previous versions together; the status changes to saved only after completion. Lifecycle saves finish on-canvas text edits, confirmed inspector drafts and image cropping, then serialize the authored document. Legacy localStorage documents migrate on a successful write. Conflicting saves from another tab are reported instead of overwritten, and a failed save leaves the document available for export. The native launcher atomically replaces `ferrodesign-autosave.json` and flushes committed document changes before closing. Export a document to make a portable copy.

Native inspector fields commit on Enter or focus loss; numeric arrow keys in Effects also commit. Save/export/page-hide/window-close boundaries flush focused confirmed drafts before serialization. Invalid values retain their field, validation and unload guard; Escape restores the committed value. Validation cannot clear a conflicting-tab save error or authorize an overwrite. Pending IME preedit stays outside the saved document.

Undo history belongs to the local editor session and defaults to a maximum of 100 committed entries. Starting the application from an autosaved document creates a fresh editor with that document as its initial state and an empty undo/redo history. The portable document and autosave do not persist the undo list. The browser history regression crosses the cap, then verifies exact undo/redo of the newest edit; it does not traverse to the oldest reachable state. This session history is separate from remote collaboration or cloud version history.

`Open` also imports SVG into a selected editable frame. `Place image` accepts PNG, JPEG, GIF or BMP; FerroUI decodes the image and its data is embedded in the portable document. The native Import dialog accepts JSON, SVG markup or a raster image data URI.

Export options include editable JSON, SVG, PNG rendered through FerroUI's `RenderTargetBitmap`, FerroUI XAML artwork, and CSS inspection. Solid-color text XAML records actual shaped glyphs and decorations; [native text](docs/NATIVE_TEXT.md) describes exact edit ranges, rich undo and export limits. SVG/XAML/CSS output have format-specific limits described in the feature matrix. A JSON round trip is the complete fidelity-preserving format for this editor. Figma's proprietary `.fig` file format is not implemented.

## Validation

The fresh native CI job at [`f2bb1d61`](https://github.com/wieslawsoltes/FerroUI/commit/f2bb1d618843bf777b76859d2e06bf361bef009b) passed **394 core tests and 155 native application tests**, including Vello pixel assertions, plus **19 persistence-host tests**. Its locked dependency audits verified Vello and no active Skia packages on all five supported target graphs. [Exact-source native evidence](docs/evidence/2026-10-22-f2bb1d61/README.md#exact-source-native-ci) records the successful job and its decoded log; this native result is separate from the browser gate.

The independent exact-`f2bb1d61` [FerroDesign CI workflow](https://github.com/wieslawsoltes/FerroUI/actions/runs/39799180117) **completed successfully**. Its full browser suite passed **141 Software2D / 138 WebGL2 checks**, with all **28 scenarios in each mode**; the browser job completed at **16:43:12 UTC on 22 October 2026**. The [preserved decoded job log and printed result](docs/evidence/2026-10-22-f2bb1d61/README.md#exact-source-complete-browser-ci) establish the full result, including two guarded WebGL2 screenshot retries. The [source comparison through `e0d3cee3`](docs/evidence/2026-10-22-f2bb1d61/successful-ci-input-equivalence.json) confirms unchanged application/framework/build inputs: the later non-documentation change is the matching browser test. This complete CI pass therefore covers the current production inputs with the earlier `f2bb1d61` harness; the latest harness's complete run remains separate. The workflow also passed [five actual Chromium IndexedDB checks](docs/evidence/2026-10-22-f2bb1d61/successful-ci-persistence-summary.json), distinct from the 19 host tests.

The local browser evidence below uses the recovered `3a0bf1f1` CI distribution. The [evidence index](docs/evidence/2026-10-22-f2bb1d61/README.md) and [validation history](docs/VALIDATION.md) preserve the source identities, reports and diagnostic limits separately:

| Browser attempt | Completed observation | Result |
| --- | --- | --- |
| First full run, original `f2bb1d61` harness | Software2D **141** checks; WebGL2 **21**, then an alias-deletion predicate deadline | Failed |
| Focused authoring diagnostic, instrumented copy | WebGL2 **21** checks with the original inputs, deadlines and assertions | Passed only its selected scope |
| Second unchanged full run | Software2D **141** checks and all **28** scenarios; WebGL2 **27**, then component-control readiness failed | Failed |
| Fresh WebGL2-only run, all 28 scenarios requested | **82** checks and **16** scenarios completed; the 17th, matching selection, failed to reopen its dialog | Failed; `full: false` |
| Corrected `f8e0471e` matching test, both modes | **25 Software2D / 22 WebGL2** checks, including all three matching cases in each mode | Passed only its selected scope; `full: false` |
| Additional `f8e0471e` WebGL2 coverage | Local HTTP listen denied before Chromium; **zero checks or scenarios ran** | Environment blocked; eleven scenarios outstanding for this `f8e0471e` local attempt |

All **29** shared captures in the failed WebGL2-only attempt passed on their first attempt. Its trace shows the page inspector still presented before the matching-opener click, with the selected-layer inspector first presented afterward, despite live state already reporting selection and valid button bounds. The test-only [`f8e0471e` correction](https://github.com/wieslawsoltes/FerroUI/commit/f8e0471eb4301a9b893e8185810550ebba5ddafc) separates Escape publication from the next selection and requires the dialog-open outcome after one click. Its [focused both-mode pass](docs/evidence/2026-10-22-f8e0471e/matching-focused-summary.json) retains all assertions and deadlines; all four shared captures passed on their first attempt. This supports the corrected test boundary without proving the original native event target or an application defect. The application and runtime are unchanged.

The [remaining-scenario launch record](docs/evidence/2026-10-22-f8e0471e/remaining-webgl2-launch-summary.json) reports `EPERM` while listening on `127.0.0.1`, before Chromium startup. No browser report was produced; all 31 source and 16 runtime files remained unchanged. This is an environment block, not an application-test failure. The [corrected-source evidence index](docs/evidence/2026-10-22-f8e0471e/README.md) keeps it separate from the focused pass.

The successful CI summary is extracted from the decoded job log. The official validation/build ZIPs could not be retrieved locally, so detailed capture totals, page/console error arrays, archive CRCs and binary equality with the recovered `3a0bf1f1` runtime are not claimed. At **16:50:41 UTC**, the [`e0d3cee3` workflow](https://github.com/wieslawsoltes/FerroUI/actions/runs/39814628155) was in progress; no complete latest-harness result is inferred. No same-document Figma pixel comparison is claimed. Earlier `f36dabf7` and `3a0bf1f1` native/framework successes and browser failures remain in the [continuation evidence](docs/evidence/2026-10-22-f36dabf7/README.md). The older `945283ef` implementation's complete **62 Software2D / 59 WebGL2** pass remains in its [historical evidence](docs/evidence/2026-10-19-945283ef/README.md), with its original source and smaller test scope.

```sh
cargo test --locked -p ferrodesign-core
./scripts/check-renderer.sh
cargo check --locked -p ferrodesign-app
cargo test --locked -p ferrodesign-app --lib -- --test-threads=1
cargo run --release -p ferrodesign-core --example performance -- 10000
cargo run --locked -p ferrodesign-core --example style_allocations
npm ci
npx playwright install chromium
node --test tests/document-store.test.mjs
node tests/document-store-browser.mjs
npm run test:browser
```

`tests/browser.mjs` serves the actual `dist` build and exercises both requested browser rendering modes. It checks native pixels, pointer drawing, vector-point and pen-handle gestures, bucket strokes, on-canvas Unicode text editing, paint/effect controls, stacked guides, matching selection, auto-layout drag previews and drops, prototype pointer/key/delay interactions, keyboard history, document import/export, durable reload, and a 10,000-layer document larger than the usual localStorage allowance. ComboBox checks verify the actual native popup item focus. Canvas selection, filtered action palettes and the tested keyboard-to-inspector transitions wait for the relevant native frame and input scene. The suite also checks startup exceptions and render-thread panics, and writes PNG screenshots, traces and JSON/HTML reports to `browser-report`. The separate IndexedDB test verifies real transaction rollback, migration and conflicting tab writes.

On a bare Linux validation host, use `npx playwright install --with-deps chromium` to install Chromium's required OS libraries as well as the browser. The shorter install command above assumes those libraries are already available.

Both Chromium suites accept `FERRODESIGN_CHROMIUM=/absolute/path/to/chrome` to use an existing executable. Without this variable, they use Playwright's installed Chromium. The editor suite also exercises native local-style definition editing and text/Boolean flattening, including rejected conversions and exact undo/redo.

The GitHub Actions workflow `.github/workflows/ferrodesign.yml` has independent native and browser jobs. The native job runs locked core tests, paired release benchmarks, focused native TextBox preedit tests, public compositing/geometry/text-outline tests, native compilation and the complete native application suite. The browser job builds the real WebAssembly application and runs persistence and editor interaction validation. The native suite contains pixel comparisons for incremental preparation, spatial root culling, guide clipping and shadow order, plus bounded background blur, gesture ownership and authoring parser regressions. Browser checks also cover networks, components, variables, rich text, crop gestures, page controls, grid spans/reorder, effect transactions and 10,000-layer preparation/root-candidate counts. Build and validation artifacts are attached to workflow runs. See [validation and measurements](docs/VALIDATION.md) for observed results; source coverage and added assertions do not establish a passing run on their own.

## Rendering performance

The renderer retains immutable geometry, paints, text, image assets and prepared subtree bounds. Scoped changes refresh affected nodes and ancestors, and the layer sidebar reuses unchanged row projections. Scenes with at least 32 prepared roots also use a balanced bounds hierarchy to find paint candidates for the viewport. Candidates return to document paint order before compositing, so spatial partitioning preserves stacking. Unchanged viewport queries reuse their candidate buffer; scoped root-bound changes refit the affected tree branch. A view containing the entire root bounds uses the retained paint-order list directly.

These mechanisms reduce preparation, bounds testing and control allocation. Root signatures, visible artwork drawing, wide sibling aggregation, structural layout commits and complete serialization still have size-dependent costs. The report distinguishes those operations from device-specific GPU time and display frame rate. See [architecture](docs/ARCHITECTURE.md#retained-scene-and-invalidation) for cache boundaries and conservative fallbacks.

The latest paired release benchmark at `f2bb1d61` measured one-node nudges in a 10,000-node native document at **0.019659 ms p50** on the scoped path and **17.284794 ms p50** on the explicit full-document transaction reference, with 48 samples and exact document/history equivalence. A scoped gesture update measured **0.000751 ms p50** across 288 updates. These are native engine costs in the same executable, not a previous-version speedup, browser event latency, GPU time or display FPS. Component edits still retain full-document history, and serialization remains document-sized. [Validation and measurements](docs/VALIDATION.md#paired-release-measurements-at-f2bb1d61) gives the current per-event comparisons, sample counts, size-dependent costs and decoded-log provenance while preserving the earlier benchmark separately.

The same CI job's separate 150-label browser pan benchmark recorded **14.28 ms median / 20.62 ms p95** in Software2D and **20.30 ms median / 1,366.94 ms p95** in WebGL2, across 36 observations per mode after 12 warmups. This measures host pointer dispatch until a native frame at the expected camera position is observed; it includes automation and scheduling. The benchmark requests SwiftShader, and the observed WebGL2 tail was high. The [rounded CI log values](docs/evidence/2026-10-22-f2bb1d61/successful-ci-text-pan-summary.json) are not FPS or isolated GPU timings; [the method and evidence limits](docs/VALIDATION.md#ci-text-panning-observations-at-f2bb1d61) are recorded separately from the native model benchmark.

`style_allocations` compares identical edits on linked and unlinked image-filled layers. It counts allocations and requested bytes, asserts equal overhead in each cohort, and deliberately omits timing. Existing history/cache/metadata copies remain in both measurements; this checks that binding comparisons avoid another payload snapshot, not that moves are allocation-free. Shared-definition propagation is still document-sized.

## Source map

| Path | Responsibility |
| --- | --- |
| `crates/ferrodesign-core/src/model.rs` | Document graph, paints, text, components, variables and validated serialization |
| `crates/ferrodesign-core/src/editor.rs` | Transactions, history and editing operations |
| `crates/ferrodesign-core/src/vector_edit.rs` | Bézier editing, exact curve picking/splitting and affine bounds normalization |
| `crates/ferrodesign-core/src/vector_faces.rs` | Checked planar faces, retained boundary geometry and indexed bucket queries |
| `crates/ferrodesign-core/src/text_edit.rs` | Unicode-safe rich-text range preservation |
| `crates/ferrodesign-core/src/prototype.rs` | Trigger rules, activation tokens, overlays and variable overrides |
| `crates/ferrodesign-core/src/geometry.rs` | Affine geometry, vector paths and viewport math |
| `crates/ferrodesign-core/src/layout.rs` | Constraints and row/column/wrapped/grid auto layout |
| `crates/ferrodesign-core/src/layout_guides.rs` | Square/row/column guide geometry, ordered stack edits and scoped history |
| `crates/ferrodesign-core/src/layout_drag.rs` | Immutable sibling geometry, indexed insertion targets and validated reorder commits |
| `crates/ferrodesign-core/src/effects_edit.rs` | Typed effect commands, atomic batch validation and scoped history |
| `crates/ferrodesign-core/src/style_edit.rs` | Validated local style definitions, propagation, detachment and history |
| `crates/ferrodesign-core/src/flatten.rs` | Conservative flatten planning, reference checks and atomic replacement |
| `crates/ferrodesign-core/src/spatial.rs` | Spatial index, clipping and geometry-aware hit testing |
| `crates/ferrodesign-core/src/selection/matching.rs` | Property-based matching, hierarchy eligibility and inverse selection |
| `crates/ferrodesign-app/ui/Editor.xaml` | Compiled FerroUI editor shell and control styling |
| `crates/ferrodesign-app/src/shell.rs` | Control events, property editing and document commands |
| `crates/ferrodesign-app/src/canvas.rs` | FerroUI canvas control, input gestures and selection overlays |
| `crates/ferrodesign-app/src/canvas/grids.rs` | Bounded guide drawing and transformed clipping through public FerroUI APIs |
| `crates/ferrodesign-app/src/canvas/network_bucket.rs` | Hover preview, captured bucket strokes, commit and cancellation |
| `crates/ferrodesign-app/src/shell/` | Native authoring dialogs and the cached layer projection |
| `crates/ferrodesign-app/src/prototype.rs` | Presentation snapshots and cancellable FerroUI dispatcher timers |
| `crates/ferrodesign-app/src/renderer.rs` | Cached public-API scene renderer |
| `crates/ferrodesign-app/src/renderer/flatten.rs` | Editable contours from public FerroUI geometry and complete text outlines |
| `crates/ferrodesign-app/src/renderer/root_index.rs` | Prepared-root bounds hierarchy, refitting and retained ordered viewport candidates |
| `crates/ferrodesign-app/src/platform.rs` | Browser/native storage, clipboard and downloads |
| `crates/ferrodesign-app/src/desktop.rs` | Vello native configuration and headless rendering |
| `web` / `scripts` / `tests` | Browser host, reproducible build and interaction validation |

See [architecture](docs/ARCHITECTURE.md) for rendering, invalidation and transaction details.

## License

New FerroDesign source is MIT-licensed, consistent with the parent repository. Refer to the repository license and individual dependency licenses. The sample uses its own name and sample artwork; it is not an official Figma product.
