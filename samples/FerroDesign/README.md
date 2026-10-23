# FerroDesign

A design editor built with **FerroUI, Rust, compiled XAML and the FerroUI Vello backend**. The same editor, controls, document model and scene renderer run in a browser and in the native launcher.

The editor includes pages and layers, an infinite canvas, stacked layout guides, property-based selection, editable vector networks with a paint bucket, rich text, image cropping, native effect/mask controls, continuous box corners, rounded polygons/stars, variable-width strokes, independent paragraph formatting, cross-frame auto-layout dragging, component variants and typed properties, visual local variables, animated prototypes, comments, undo/redo and portable export. The compiled XAML shell follows the navigation rail, contextual left sidebar, canvas, property sidebar and floating toolbar arrangement of Figma Design. [Feature coverage](docs/FEATURES.md) records the implementation and its limits; this repository does not claim complete Figma compatibility or verified pixel parity.

The implementation guides describe [layout guides](docs/LAYOUT_GUIDES.md), [four-side layout padding](docs/LAYOUT_PADDING.md), [spacing variable bindings](docs/LAYOUT_VARIABLES.md), [matching selection](docs/SELECTION.md), [angular paints](docs/ANGULAR_PAINTS.md), [linear, radial and angular gradient canvas handles](docs/GRADIENT_HANDLES.md), [vector faces and bucket painting](docs/VECTOR_FACES.md), [continuous corners](docs/CORNERS.md), [polygon/star rounding](docs/POLYGON_CORNERS.md), [stroke profiles](docs/STROKE_PROFILES.md), [width handles](docs/STROKE_WIDTH_HANDLES.md) and [native paragraph/text editing](docs/NATIVE_TEXT.md). Their contracts and limits describe the source; completed validation belongs to the exact checkpoints recorded below.

The editor is real FerroUI UI. `Editor.xaml` compiles into native FerroUI controls; `DesignCanvas` derives from `Control`; artwork is drawn through the public `DrawingContext`, geometry, brush, pen, text and imaging APIs. The HTML host only starts WebAssembly and provides browser storage, clipboard and downloads.

## Get the complete source

This sample is an independent Cargo workspace inside the FerroUI repository. Keep the repository around it: its path dependencies refer to FerroUI's actual source, not to a separate graphics implementation.

```sh
git clone --depth 1 --branch feat/ferrodesign-editor --single-branch https://github.com/wieslawsoltes/FerroUI.git
cd FerroUI/samples/FerroDesign
```

The implementation began from upstream commit `325b3ce4b0fcf154eba79cbd92262536daf77e71` and incorporates upstream `934b9ce16e2ada890ee31534f1a98808f483a14f` (FerroUI 0.4.0), including its Vello performance improvements. The branch contains the complete application, independent browser backend selection, native text-input fixes, public isolated compositing and editable geometry/text-outline APIs. [PR #254](https://github.com/wieslawsoltes/FerroUI/pull/254) collects the branch changes.

The [current source-delivery audit for `c549f6bc`](../../docs/ferrodesign-evidence/2026-10-23-ferroui-040/source-delivery-c549/README.md) verified the required tracked framework path dependencies and assets, six embedded Inter fonts, 116 internal symlinks, no submodules or LFS pointers, and all five active Vello/no-Skia graphs. Inspected build logic does not depend on FerroUI's `.git` directory. Extract the complete repository, including its root `.cargo` configuration. This was source inspection and offline graph validation, not a build from an independently downloaded ZIP; toolchain and third-party dependency downloads are still required.

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

The same dialog edits **Top, Right, Bottom and Left padding**, including on empty frames. Linking the fields preserves their existing values until an explicit edit applies one value to all sides. Independent edits retain the exact stored precision of untouched sides. All four drafts validate atomically; Escape discards the group, and **Save design** uses the normal document-save lifecycle. Nested layout reflow is one complete undo step. The compact inspector displays **Mixed** for unequal sides.

In **Variables**, bind each padding side, the main layout gap or the cross gap to a Number variable. Changing the variable or active mode updates spacing and nested layout in one undo step. Literal spacing edits detach only the changed fields; **Unbind** retains the effective value. Aliases and fallback modes work with the same typed validation. Prototype **Set variable** actions also reflow affected spacing in disposable preview state. The [spacing variable guide](docs/LAYOUT_VARIABLES.md) defines target, mode, lock and component behavior.

For an eligible linear, radial or angular gradient fill, open **Fill paints** and choose **Edit on canvas**. Linear handles move endpoints and translate the axis; radial handles move the center and change its normalized radius; angular handles move the center and rotate the sweep. All three support individual stop movement, insertion and deletion. Tied stops, coincident endpoints and angular seam stops retain separately accessible markers. Press-relative affine movement, exact drag-back, Escape/capture-loss cancellation, keyboard navigation and Save/pagehide use owned native transactions. The [gradient guide](docs/GRADIENT_HANDLES.md) describes the supported box geometries, 256-stop preparation bound and remaining geometry limits.

Select one or several layers and drag between layout frames, ordinary frames or the active page. An outline and insertion line preview the destination in rows, columns, wrapped rows and grids, including reversed flow and transformed frames. The document stays unchanged during a layout insertion preview; release commits reparenting, transformed placement and sibling layout as one undo step. Returning to the original slot, Escape or lost pointer capture cancels without a document edit. Automatic grid insertion follows flow order; use the placement dialog for pinned cells. Frame track lists accept values such as `120, auto, 2fr` for fixed, content-sized and fractional tracks.

**Layout grid** in the inspector, or **Layout grid guides…** in quick actions, opens native guide controls for a frame, component or instance. Stack square, column and row guides; edit size, count or Auto, alignment, gutter and margin/offset; and change visibility, order, duplication or removal. Each guide has one RGBA hex field, including its alpha value. Shared grid styles propagate geometry and color while preserving each linked frame's per-index visibility. Guides participate in move/draw/resize snapping, stretch-guide-relative constraints and typed variable bindings. Guide ink is excluded from artwork exports and presentation; automatic child placement uses the separate layout solver. [Layout guides](docs/LAYOUT_GUIDES.md) specifies the backward-compatible JSON, component behavior, bounded snapping and binding rules.

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

While editing a vector network, **Paint bucket** or `Shift+B` previews the hovered bounded face. Press and drag to apply the complete layer fill stack captured at the start of the gesture; hold Shift before pressing to clear crossed faces instead. A changed stroke commits one undo step. Escape or lost pointer capture rolls it back, and an unchanged stroke adds no history. Face queries include crossings between pointer samples so a fast drag can reach narrow faces. **Fill all closed regions** remains a separate command. Both authoring paths automatically split certified line/cubic intersections on a private prepared network, preserving source curves by exact subdivision and stable edge/region identities. Hover does not change the document. Overlaps, tangencies, numerically ambiguous contacts and exhausted work limits decline atomically. See [vector faces and bucket painting](docs/VECTOR_FACES.md) for explicit region paints, inheritance and geometry limits.

**Corner smoothing %** edits continuous box corners on rectangles and frame-like layers, including independent radii. Polygon and Star layers use **Corner radius** for bounded circular fillets while retaining their procedural shape and authored box. Rendering, picking, masks and supported exports share those outlines. These constructions have explicit geometry contracts; a matched Figma reference capture has not established numerical or pixel parity.

In Select mode, hover a selected supported layer's corner and drag its native radius handle. Hold **Alt/Option when pressing a Rectangle handle** to edit that corner independently. Fractional values follow the layer's rotation, reflection, scale and shear. A press without movement changes nothing; dragging back to the original value restores bindings and redo. Release commits once, Escape/capture loss cancels, and save/page-hide completes a held edit before persistence. [Radius handles](docs/CORNER_RADIUS_HANDLES.md) documents handle placement, tiny-shape behavior and the supported primitive types.

The stroke inspector offers Uniform, Wedge, Taper, Quarter taper, Eye and Mirrored taper profiles. **Custom points…** edits ordered position/width multipliers on any stroke-stack entry; invalid drafts remain visible until corrected or discarded. **Edit width on canvas** provides insertion, movement, symmetric-width and removal handles for the first stroke of one supported layer. A captured drag is one transaction, with Escape/capture-loss rollback and lifecycle-save completion. Profiles retain the editable source path and render as cached filled FerroUI geometry. Nonuniform dashed or branching strokes, independent left/right widths and per-contour profiles remain outside the supported authoring subset.

**Flatten selection** converts supported shapes, text and Boolean results into editable paths using the actual FerroUI geometry. Positioned glyph contours and text decorations follow the same shaped layout as rendering. Single-layer conversion preserves its ID, parent, stacking, transform and appearance; compatible opaque siblings can be combined in one undo step. A selection that cannot retain its appearance is rejected without changing the document or history. This includes multi-layer selections with different paint stacks, independent child opacity/effects, and bitmap or color-font glyphs without a complete monochrome outline. See the [flattening limits](docs/FEATURES.md#rendering-and-authoring-limits) for the supported subset.

Double-click a text layer to edit directly on the canvas with a native FerroUI caret, selection, clipboard and IME. Ctrl/Cmd+Enter or moving focus commits one text transaction; Escape restores the original content. Pending IME preedit stays outside the document draft, so lifecycle saves retain the last confirmed text and formatting. Exact native edit ranges map UTF-16 input positions to UTF-8 document ranges, preserving the correct styles even in repeated text. Native undo/redo retains character styles, paragraph styles and the application text snapshot through the existing TextBox history. Completing the session still creates one document transaction.

**Style selected characters…** opens a native range-selection editor with character styles and a rich preview. Its paragraph panel edits alignment, LTR/RTL/Auto direction, line height, letter spacing, before/after spacing and first-line indent for selected hard paragraphs. Shared FerroUI paragraph layout drives canvas text, the inline editor, caret/selection geometry and preview; validated OpenType features remain available. The editing overlay uses one foreground paint per run. Multiple fill stacks, effects and ancestor masks/blending remain canvas-renderer features outside that overlay. [Native text](docs/NATIVE_TEXT.md) describes Unicode replacement, IME cancellation, rich undo and the supported shaped-text XAML export.

Inter remains the primary font. Public FerroUI `FontManagerOptions`, `FontFallback` and `EmbeddedFontCollection` APIs register the same fallback assets in native and browser builds: static **Noto Sans Hebrew Regular 400 and Bold 700**, plus **Noto Color Emoji COLRv1**. Covered characters do not depend on host-installed fonts. Color glyphs retain native color rendering and remain excluded from monochrome-only outline conversion. The [font asset guide and SIL Open Font Licenses](crates/ferrodesign-app/Assets/Fonts/README.md) and [exact upstream provenance](crates/ferrodesign-app/Assets/Fonts/provenance.json) accompany all three unmodified files, totaling **5,092,300 added uncompressed bytes**. Their coverage does not include every writing system or arbitrary document font; [native text](docs/NATIVE_TEXT.md#embedded-font-coverage) defines the boundary.

Use **Affine transform and skew…** for complete parent-relative matrix values and centered skew without flattening children. The constraint diagram controls edge pins and centering during parent resizing. **Frame presets…** creates editable phone, tablet, desktop, presentation or social frames. Authoring palettes can be dragged by their native headers and keep canvas input available; they follow the current selection and stay within the window.

Use the property sidebar for geometry, fills, strokes, typography and layout. With no layer selected it shows the page background and local styles, variables and components. The paint-stack dialog edits fill order, visibility/opacity, gradients, stops and image source windows. **Crop image…** provides native pan and corner gestures for Fill/Stretch images; Escape cancels the active gesture, and Undo restores the whole gesture. Numeric crop windows are also available for Fit and Tile.

The **Effects** editor provides native controls for drop shadow, inner shadow, layer blur and background blur. Edit blur radius, signed shadow offsets/spread, RGB and separate opacity; show or hide a row, move it up/down, duplicate, remove or clear effects. Enter or leaving a field applies its value, and Escape restores the committed value. Numeric arrow keys step by 1, with Shift for 10 and Alt for 0.1. Multi-selection changes matching effect rows on unlocked layers; missing rows remain unchanged. Each accepted command is one history step, and invalid or unchanged input preserves history and bound styles.

The Appearance blend selector controls isolated or pass-through layer compositing. **Masks and compositing…** enables vector, alpha or luminance masks; **Create mask group** wraps selected siblings into one undoable group. Mask-aware selection skips transparent content and vector holes. Vello renders these operations through the same public FerroUI compositing and imaging APIs. Layer opacity applies once to the completed group. The first listed shadow of each kind paints above later shadows of that kind. Leaf shapes paint inner shadows between fills and strokes; container inner shadows sit above their compound content. Background blur samples earlier document content within the isolated group and caches its filtered result; [documented bounds](docs/FEATURES.md#rendering-and-authoring-limits) keep its allocations predictable. Shapes, ordinary fills, strokes and text continue to use native vector rendering.

**Properties and variants…** creates component sets, assigns variant combinations and exposes text, visibility and nested instance-swap properties. Switching a variant preserves matching layer IDs and explicit overrides. The native variable editor manages collections, modes, typed values, aliases and layer bindings. Invalid authoring changes are rejected atomically.

**Local styles** creates fill, text, effect and grid styles from the selection. **Edit** opens native fields for name, description and the complete supported style definition, including ordered paint/effect stacks, typography and layout guides. Definition changes update linked layers across pages; **Replace from selection**, **Detach selected**, and **Delete style** support the rest of the lifecycle. Detaching or deleting preserves each layer's effective appearance. Explicit application and detachment respect locks and compatible layer types; definition changes also reach locked linked layers. Editing a bound property directly makes that property independent, while moving a layer retains its styles. Each accepted command is one undo step, and invalid or unchanged drafts preserve history.

In the Prototype tab, **Edit interactions…** adds trigger/action rules and transitions. Presentation supports animated transitions, Smart Animate, reversible hover/press interactions, scroll containers, wheel scrolling and ScrollTo. Preview changes operate on disposable state; the authored design is restored when presentation ends and remains the source for autosaves. `Layer data…` remains available for validated structured properties without a dedicated control.

## Files and local persistence

The native editable format is a versioned `.ferrodesign` JSON document. It stores the complete document graph, paints, layout, components, variable collections, comments and prototype links. Edits coalesce before serializing the full document as compact JSON for local autosave; portable downloads use the readable pretty format. Both read the authored snapshot during presentation. The browser queues snapshots in IndexedDB. A completed transaction updates the current and previous versions together; the status changes to saved only after completion. Lifecycle saves finish on-canvas text edits, confirmed inspector drafts, image cropping and owned bucket/width gestures before serializing the authored document. Background autosave defers an active bucket/width gesture until it finishes. Legacy localStorage documents migrate on a successful write. Conflicting saves from another tab are reported instead of overwritten, and a failed save leaves the document available for export. The native launcher atomically replaces `ferrodesign-autosave.json` and flushes committed document changes before closing. Export a document to make a portable copy.

Native inspector fields commit on Enter or focus loss; numeric arrow keys in Effects also commit. Save/export/page-hide/window-close boundaries flush focused confirmed drafts before serialization. Invalid values retain their field, validation and unload guard; Escape restores the committed value. Validation cannot clear a conflicting-tab save error or authorize an overwrite. Pending IME preedit stays outside the saved document.

Undo history belongs to the local editor session and defaults to a maximum of 100 committed entries. Starting the application from an autosaved document creates a fresh editor with that document as its initial state and an empty undo/redo history. The portable document and autosave do not persist the undo list. The browser history regression crosses the cap, then verifies exact undo/redo of the newest edit; it does not traverse to the oldest reachable state. This session history is separate from remote collaboration or cloud version history.

`Open` also imports SVG into a selected editable frame. `Place image` accepts PNG, JPEG, GIF or BMP; FerroUI decodes the image and its data is embedded in the portable document. The native Import dialog accepts JSON, SVG markup or a raster image data URI.

Export options include editable JSON, SVG, PNG rendered through FerroUI's `RenderTargetBitmap`, FerroUI XAML artwork, and CSS inspection. Solid-color text XAML records actual shaped glyphs and decorations; [native text](docs/NATIVE_TEXT.md) describes exact edit ranges, rich undo and export limits. SVG/XAML/CSS output have format-specific limits described in the feature matrix. A JSON round trip is the complete fidelity-preserving format for this editor. Figma's proprietary `.fig` file format is not implemented.

## Scope and remaining gaps

The source does not implement `.fig` or Figma clipboard interoperability, multiplayer/presence, cloud accounts and permissions, remote libraries, collaborative branches, the plugin/widget runtime, full Dev Mode or the separate FigJam/Slides/Sites/Make/Draw products. Prototypes lack a complete timeline, spring/physics and video system. Comments, variables, components and document history are local features.

Artwork also retains documented limits: arbitrary vector-anchor fillets, every stroke/curve arrangement, complete SVG/XAML interchange and full color-management/effects parity are not established. The native text overlay and shaped-text XAML export support defined appearance subsets. No same-document Figma reference-image comparison has established full visual parity. [Feature coverage](docs/FEATURES.md) separates these gaps from implemented editing paths.

## Validation

The FerroUI **0.4.0** integration preserves both upstream ancestry and the
editor's public API additions. The merged source passed **533 core tests in CI**,
all five generated Rust/XAML reference checks, and the five active
**Vello/no-Skia** target graphs locally. Native and browser results remain
attached to their exact sources; the completed pre-integration results below
do not substitute for a current-source complete browser pass.
[Integration details](docs/VALIDATION.md#ferroui-040-integration-2026-10-23)
describe the inherited text-bounds and native GPU fragment corrections.

**The corrected `dc361996` native checkpoint passed all 14 added first-preedit
regressions, the complete 188-case TextBox substring filter, 239 application
tests and seven required GPU fragment tests.** Its paragraph, line-bounds,
rich-history and generated Rust/XAML reference checks also passed. Framework
filter counts overlap and are not additive. The exact browser distribution is
verified, including its 53,795,035-byte WASM, all 17 payloads and copied host
sources. Compilation took 11m29s in Cargo; browser interaction results are a
separate gate.

The exact `dc361996` local browser replay passed **all ten selected scenario
executions**, with **57 Software2D / 54 WebGL2 checks**. Corners, polygon/star
rounding, gradient handles, paragraphs and guide bindings passed with the
ordinary common gates; `scope.full` remains false. Its separate strict text-pan
benchmark completed 36 normal plus six trace observations per mode and closed
the browser gracefully. The WebGL2 p95 native-frame observation was **762.638 ms**
on SwiftShader; this does not establish physical-GPU speed or FPS.

The saved `dc361996` paragraph images still show missing Hebrew and crab glyphs.
They predate the embedded fallback change described above, whose **native and
browser validation is pending**; static asset and source checks are separate
from a rendering pass. The separate full `b5008093` CI run also retains a
paragraph-entry setup failure and
a strict browser-shutdown failure. Its [entry diagnosis and source correction](../../docs/ferrodesign-evidence/2026-10-23-ferroui-040/paragraph-entry-b500/README.md)
preserve the original failure; the correction requires its own replay. Earlier
failed runs remain recorded. A complete current-source browser pass and matched
Figma visual parity are not established.

| Gate | Exact source and completed result |
| --- | --- |
| Corrected first-preedit native execution | [`dc361996`, job 120490704088](https://github.com/wieslawsoltes/FerroUI/actions/runs/39976024451/job/120490704088): **all 14 added regressions**, **188/188 TextBox-filter cases**, **239/239 application tests in 37.96 s** and **7/7 required GPU fragment tests in 6.00 s** passed; all other configured paragraph, bounds, glyph and history filters also passed. Generated references passed separately at the same SHA. [Exact logs, names and overlapping counts](../../docs/ferrodesign-evidence/2026-10-23-ferroui-040/native-dc361/README.md) |
| Corrected first-preedit browser delivery | [`dc361996`, job 120490704316](https://github.com/wieslawsoltes/FerroUI/actions/runs/39976024338/job/120490704316): **53,795,035-byte WASM**, **17 payloads plus provenance**, exact source/tree/run and host resources, archive integrity, seven-module delivery closure and Vello/no-Skia graph verified. Cargo build **11m29s**; all ten host input tests passed. [Complete build and delivery evidence](../../docs/ferrodesign-evidence/2026-10-23-ferroui-040/browser-build-dc361/README.md). This build record does not establish an editor interaction pass |
| Current selected browser execution | Exact `dc361996` runtime and 56-file frozen harness: **57 Software2D / 54 WebGL2 checks**, **10/10 selected scenario executions passed**, no page/console errors and no reported unsupported effects. The five selected scenarios include native paragraph editing and all 17 gradient groups; `scope.full` is false. The actual images retain missing Hebrew/emoji glyphs. [Complete report, input records and screenshots](../../docs/ferrodesign-evidence/2026-10-23-ferroui-040/browser-runtime-dc361/README.md) |
| Current strict text-pan measurement | Exact `dc361996`: **36 normal plus 6 separate trace observations per mode**, all sample/cache/frame invariants and graceful browser shutdown passed. Native-frame median/p95: **13.574/25.093 ms Software2D**, **20.063/762.638 ms WebGL2/SwiftShader**. These host observations are not FPS or GPU/display-completion measurements. [All phases, raw traces and boundaries](../../docs/ferrodesign-evidence/2026-10-23-ferroui-040/browser-text-pan-dc361/README.md) |
| Previous selected browser execution | Exact `c549f6bc`: **50 Software2D / 38 WebGL2 checks**; **1/8** complete scenario executions passed. Polygon coverage, first IME preedit and guide input failed in both modes; WebGL2 also failed later gradient insertion. [Original failure evidence](../../docs/ferrodesign-evidence/2026-10-23-ferroui-040/browser-runtime-c549/README.md) and [separate successful strict benchmark](../../docs/ferrodesign-evidence/2026-10-23-ferroui-040/browser-text-pan-c549/README.md) remain unchanged |
| Full native workflow | [`c7dbf26f`, job 120423823557](https://github.com/wieslawsoltes/FerroUI/actions/runs/39955698269/job/120423823557): **533 core / 239 application tests**, native launcher compilation and configured public input/compositing/text/geometry gates passed. Production is identical to `c549`; this predates the new first-preedit fix. [Exact full native record](../../docs/ferrodesign-evidence/2026-10-23-ferroui-040/full-native-c7/README.md) |
| Sequential full browser workflow | [`c7dbf26f`, job 120423823862](https://github.com/wieslawsoltes/FerroUI/actions/runs/39955698269/job/120423823862): **failed and incomplete**, canceled after the 120-minute budget while continuing WebGL2 guide work; six later scenarios were not reached and no final report was produced. The independent cache bridge worked and the browser build took **9m54s**. [Terminal job, capture timings and lossless log](../../docs/ferrodesign-evidence/2026-10-23-ferroui-040/full-browser-c7/README.md) |
| First-preedit native checkpoint | [`4d3c796c`](../../docs/ferrodesign-evidence/2026-10-23-ferroui-040/native-4d3/README.md) compiled but **failed three new caret/selection/template regressions**; all 13 new functions ran and ten passed. The full TextBox substring filter passed **184/187**. Independent application **239/239** and required GPU **7/7** stages passed. The distinct `dc361996` follow-up preserves expectations, adds a five-case reentry regression and passed the separate native run recorded above |
| First-preedit browser delivery | [`4d3c796c`](../../docs/ferrodesign-evidence/2026-10-23-ferroui-040/browser-build-4d3/README.md): verified **53,794,092-byte WASM**, **17 payloads plus provenance**, exact source/tree/run, archive integrity and Vello/no-Skia graph. Build step **15m24s**, Cargo **15m14s**. No local extraction or browser replay; the native failures required the follow-up |
| Previous native application and framework | [`c549f6bc`, job 120410483091](https://github.com/wieslawsoltes/FerroUI/actions/runs/39952473854/job/120410483091): **239/239 app tests**, **43.60 s**, and all configured framework filters passed, including the new native hover/pressed projection. Mandatory GPU fragments: **7/7**, **9.09 s**, with `FERROUI_VELLO_REQUIRE_GPU=1`; no failed or ignored tests. [Exact native evidence](../../docs/ferrodesign-evidence/2026-10-23-ferroui-040/native-c549/README.md) |
| Previous browser build and delivery | [`c549f6bc`, job 120415321864](https://github.com/wieslawsoltes/FerroUI/actions/runs/39952473917/job/120415321864): build step **10 min 10 s**; verified **53,785,830-byte WASM**, **17 payload files plus the provenance manifest**, Vello/no-Skia graph, local resource/module closure and exact-source relative favicon. [Artifact and cache evidence](../../docs/ferrodesign-evidence/2026-10-23-ferroui-040/browser-build-c549/README.md); runtime checks remain separately labeled |
| Integrated core | [`1c7151f5`, run 39950194410](https://github.com/wieslawsoltes/FerroUI/actions/runs/39950194410): **533 passed**, **2.41 s**, with the FerroUI 0.4.0 lockfile |
| Integrated generated references | [`1c7151f5`, run 39950194402](https://github.com/wieslawsoltes/FerroUI/actions/runs/39950194402): all generators and drift checks passed; **five generated files match their committed SHA-256 hashes**, with an empty patch |
| Integrated browser build and delivery | [`1c7151f5`, job 120403095053](https://github.com/wieslawsoltes/FerroUI/actions/runs/39950194298/job/120403095053): build step **28 min 17 s**; verified **53,785,479-byte WASM**, **16 payload files plus the provenance manifest**, Vello/no-Skia graph and local module/resource closure. [Archive, payload hashes and scope](../../docs/ferrodesign-evidence/2026-10-23-ferroui-040/browser-build-1c/README.md). No editor or benchmark was executed against this runtime |
| Integrated native application and framework | [`2d6284d9`, run 39950436412](https://github.com/wieslawsoltes/FerroUI/actions/runs/39950436412): **239/239 app tests**, **26.03 s**, and all configured framework filters passed. Mandatory native GPU fragments: **7/7**, **7.14 s**; no skipped adapter fallback |
| Feature browser execution | Verified `08e29e4f` WASM with frozen `0f073278` harness: **9/12 Software2D and 8/12 WebGL2 scenarios passed**, **88 / 68 completed check groups**. Both modes failed overall. All five spacing-variable checks passed in both modes; all 17 gradient checks passed in Software2D. [Exact results and failures](../../docs/ferrodesign-evidence/2026-10-23-continuation/browser-runtime08-harness0f/README.md) |
| Current browser helper checks | Published `0aab9d7c` correction tree: **53/53 local cases passed** across coverage, pointer expectations, passive input evidence, isolated scenarios and trace/process ownership. Separate from actual editor execution |
| Pre-integration production core | [`1637a093`, run 39944253726](https://github.com/wieslawsoltes/FerroUI/actions/runs/39944253726): **533 passed**, **2.50 s**, including prototype spacing reflow/rollback and unchanged-binding redo preservation |
| Pre-integration native application | [`1637a093`, run 39944253747](https://github.com/wieslawsoltes/FerroUI/actions/runs/39944253747): **239 passed**, **43.57 s**, including the clipped-selection/radius-control priority regression and native diagnostic controls |
| Feature core | [`dfab0ed9`, run 39942848092](https://github.com/wieslawsoltes/FerroUI/actions/runs/39942848092): **528 passed**, **1.72 s**, including radial/angular authoring and spacing-variable ownership/reflow |
| Feature native application | [`dfab0ed9`, run 39942848091](https://github.com/wieslawsoltes/FerroUI/actions/runs/39942848091): **238 passed**, **28.42 s**, including five new radial/angular native cases and three native spacing-variable UI cases |
| Padding Escape correction | [`3be16cb7`, run 39941952822](https://github.com/wieslawsoltes/FerroUI/actions/runs/39941952822): **230 application tests passed**, including the previously failing grouped-draft Escape case |
| Native framework and renderer | The successful `dfab0ed9` feedback job also passed **8 Vello / 15 Base / 16 Controls paragraph / 174 TextBox / 5 rich-history** tests; overlapping filters are not additive. The [`3be16cb7` full native job](https://github.com/wieslawsoltes/FerroUI/actions/runs/39941952796/job/120375278062) passed all five active **Vello/no-Skia** target graphs, native compilation and configured public geometry/compositing/text gates |
| Prototype spacing integration | Exact published tree of [`1637a093`](https://github.com/wieslawsoltes/FerroUI/commit/1637a093723a4f899b7f4e371fd53f0b91914a98): **533 core tests passed locally**, **5.91 s**, including five preview reflow/rollback cases. **34 pure JavaScript** coverage/input/trace-ownership tests passed separately |
| Latest recovered full browser result | [`b7b0b885`, run 39869066118](https://github.com/wieslawsoltes/FerroUI/actions/runs/39869066118) built actual Vello WASM and passed IndexedDB checks, but **failed overall**: **161 Software2D / 155 WebGL2 completed checks**, with **9 / 10 failed scenarios** among 37 requested per mode. The failures and subsequent corrections are preserved; this is not a current-source pass |
| Independent corner reference | The [fixed 16×/32× Chromium decoder experiment](../../docs/ferrodesign-evidence/2026-10-23-browser-harness/README.md) converged at **1.96875 alpha units** and had **0 native PNG pixels differing by more than 12**, retaining the original limits. This isolated reference check is separate from complete editor execution |

Checkpoint [`c549f6bc`](https://github.com/wieslawsoltes/FerroUI/commit/c549f6bcafe614a6373c743f5efbf2a3cb0eccd2)
uses real native hover/press readiness for gradient entry, one verified multiline
clipboard paste, actual Inspect-tab navigation for XAML export, and a relative
favicon asset. The [input evidence](../../docs/ferrodesign-evidence/2026-10-23-continuation/browser-input-readiness/README.md)
preserves the preceding failures and unchanged assertions. These corrections
require the new browser runtime; no activation retry or tolerance relaxation
turns the earlier run into a pass.

The subsequent [`0aab9d7c`](https://github.com/wieslawsoltes/FerroUI/commit/0aab9d7c4a2a9ce7921cd239a4e1f6412230c911)
harness checkpoint delivers one real no-delay double-click and requires native
click counts `[1, 2]` inside the unchanged 500 ms limit. Guide pointer fixtures
use the native zoom control to enter the existing five-screen-pixel attraction
range. The polygon test shares the independently converged SVG coverage
reference with box corners, retaining its original 12-alpha / 85-pixel gate.
The [static polygon diagnostic](../../docs/ferrodesign-evidence/2026-10-23-ferroui-040/polygon-coverage-c549/README.md)
reproduces the raw 212 differences, finds zero against converged coverage, and
rejects a deliberate one-pixel shift with 468 differences. A separate native
first-preedit visibility defect requires a framework correction and new runtime.
The completed failed replay is preserved unchanged.

[Current evidence](../../docs/ferrodesign-evidence/2026-10-23-continuation/README.md)
records exact commits, CI links, log digests and selected result lines.
[Validation and measurements](docs/VALIDATION.md) preserves earlier successes,
failures and subsequent corrections. The historical complete `76d3744c` run
passed 141 Software2D / 138 WebGL2 checks over 28 scenarios per mode; it predates
the current additions and does not validate them.

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

Feature failures remain failed gates while later independent scenarios may run. Every next scenario must first pass the existing unchanged-draft, durable-storage, native-panic and exact-reload checks; a failed boundary aborts that renderer session. Primary errors are printed before bounded diagnostic capture, and a capture timeout also aborts the session. Reports retain each scenario's status, actual completed-check count and failure stage; any failed scenario fails the overall run. Five pure orchestration regressions pass locally; actual feature execution still requires the browser build.

On a bare Linux validation host, use `npx playwright install --with-deps chromium` to install Chromium's required OS libraries as well as the browser. The shorter install command above assumes those libraries are already available.

Both Chromium suites accept `FERRODESIGN_CHROMIUM=/absolute/path/to/chrome` to use an existing executable. Without this variable, they use Playwright's installed Chromium. The editor suite also exercises native local-style definition editing and text/Boolean flattening, including rejected conversions and exact undo/redo.

The GitHub Actions workflow `.github/workflows/ferrodesign.yml` keeps native
validation independent of browser progress. The native job runs locked core
tests, paired release benchmarks, focused native TextBox preedit tests, public
compositing/geometry/text-outline tests, native compilation and the complete
application suite. A separate `.github/workflows/ferrodesign-native.yml`
supplies focused glyph/paragraph/TextBox/history feedback and application
tests; a failed framework stage still fails that job even when its later
application tests pass.

Browser CI builds one real Vello WebAssembly distribution and hashes every
payload alongside its source, tree and harness identity. Eight jobs consume
that same distribution: four contiguous scenario groups in each of Software2D
and WebGL2. Each runs the original common gates and unchanged selected
scenario assertions, with a 60-minute limit and `fail-fast: false`. A separate
job runs the original IndexedDB transaction/reload checks and the text-pan
benchmark with 36 normal and six separate trace samples per mode.

The always-run aggregate requires all **76 scenario/mode pairs**, successful
job and process outcomes, exact report scopes and matching source/harness/
distribution hashes. Missing, failed or incomplete evidence fails the gate.
Per-group reports, screenshots and traces remain separate artifacts. A later
rerun may reuse a successful sibling's proof only for the identical build;
build and execution attempt numbers remain recorded. The partition contract
and its negative cases can be checked without a browser using
`python3 scripts/test_browser_ci.py` from this sample directory.

This configured coverage spans independent sessions. The earlier sequential
timeout remains a separate performance finding. [Validation and measurements](docs/VALIDATION.md)
records completed executions and their exact source boundaries.

The independent `.github/workflows/ferrodesign-browser-build.yml` publishes a
reviewable browser distribution without waiting for an older complete suite.
It uses the same locked build, Vello/no-Skia graph, host checks and static import
validation, and includes the exact commit/tree, tool versions and SHA-256 of
each payload in `ferrodesign-provenance.json`. This build-only feedback does not
replace persistence, interaction, pixel or benchmark validation.

The independent `c549f6bc` build restored **10,459 matching source timestamps**
and skipped **nine changed files**, then rebuilt the application. Its recorded
10 min 10 s build is evidence of reuse in that independent workflow, not a
controlled speedup or a measurement of the full workflow's new cache path.

The full browser workflow at `c7dbf26f` restored the independent c549 build
cache and **10,464 matching source timestamps**, skipped **five changed
inputs**, and completed its browser build in **9m54s**. Its build artifact and
verified source snapshot/target cache were saved before long interactions.
The subsequent sequential browser job exhausted its 120-minute budget while
continuing WebGL2 work and retained failed/incomplete coverage. The
[complete terminal evidence](../../docs/ferrodesign-evidence/2026-10-23-ferroui-040/full-browser-c7/README.md)
keeps the observed build reuse separate from browser correctness and overall
performance.

## Rendering performance

The [current c549 measurement](../../docs/ferrodesign-evidence/2026-10-23-ferroui-040/browser-text-pan-c549/README.md)
completed 36 normal motions and six separate traced motions per mode, with no
additional scene preparation, rebuilt visuals or layer projections. Median
dispatch-to-observed-frame times were 20.148 ms in Software2D and 17.433 ms in
WebGL2; p95 was 369.758 ms and 1,012.635 ms. The trace preserves substantial
application callback work and late browser display waits. These shared-host
SwiftShader observations do not establish smooth frame delivery or physical-GPU
throughput; all phases and long tails are retained in the linked record.

The [completed feature-runtime text-pan measurement](../../docs/ferrodesign-evidence/2026-10-23-continuation/browser-text-pan-runtime08-harness0f/README.md)
at `08e29e4f` recorded 36 motions per mode. WebGL2's observed dispatch-to-frame
median/p95 was **15.24 / 26.77 ms**, native-presentation p95 **118.86 ms**, and
settling p95 **905.03 ms**. Those distinct phases include automation and shared-host
SwiftShader work; the first counter advance is not display FPS or complete
presentation. The overall benchmark failed because Software2D requested a missing
favicon. Its normal samples, WebGL2's separate six-motion trace, and successful
graceful shutdown are retained. The favicon correction and FerroUI 0.4.0 runtime
need their own execution.

The [matched frame analysis](../../docs/ferrodesign-evidence/2026-10-23-continuation/browser-text-pan-runtime08-harness0f/presentation-analysis/README.md)
locates the dominant recorded late wait at Chromium's display-swap boundary,
while separately retaining substantial FerroUI callback work. It does not
identify the underlying driver/worker/scheduling cause or demonstrate a
current-runtime speedup.

The renderer retains immutable geometry, paints, text, image assets and prepared subtree bounds. Scoped changes refresh affected nodes and ancestors, and the layer sidebar reuses unchanged row projections. Scenes with at least 32 prepared roots also use a balanced bounds hierarchy to find paint candidates for the viewport. Candidates return to document paint order before compositing, so spatial partitioning preserves stacking. Unchanged viewport queries reuse their candidate buffer; scoped root-bound changes refit the affected tree branch. A view containing the entire root bounds uses the retained paint-order list directly.

These mechanisms reduce preparation, bounds testing and control allocation. Root signatures, visible artwork drawing, wide sibling aggregation, structural layout commits and complete serialization still have size-dependent costs. The report distinguishes those operations from device-specific GPU time and display frame rate. See [architecture](docs/ARCHITECTURE.md#retained-scene-and-invalidation) for cache boundaries and conservative fallbacks.

The historical paired release benchmark at `f2bb1d61` measured one-node nudges in a 10,000-node native document at **0.019659 ms p50** on the scoped path and **17.284794 ms p50** on the explicit full-document transaction reference, with 48 samples and exact document/history equivalence. A scoped gesture update measured **0.000751 ms p50** across 288 updates. These are native engine costs in that executable, not a previous-version speedup, browser event latency, GPU time or display FPS. Component edits still retain full-document history, and serialization remains document-sized. [Validation and measurements](docs/VALIDATION.md#paired-release-measurements-at-f2bb1d61) gives the historical per-event comparisons, sample counts and decoded-log provenance. These numbers do not measure the newer geometry, width or text-editing paths.

The historical `f2bb1d61` browser job's separate 150-label pan benchmark recorded **14.28 ms median / 20.62 ms p95** in Software2D and **20.30 ms median / 1,366.94 ms p95** in WebGL2, across 36 observations per mode after 12 warmups. It measured host pointer dispatch until a native frame at the expected camera position was observed, including automation and scheduling. SwiftShader was requested, and the observed WebGL2 tail was high. The [rounded CI log values](docs/evidence/2026-10-22-f2bb1d61/successful-ci-text-pan-summary.json) are not FPS or isolated GPU timings; [the method and evidence limits](docs/VALIDATION.md#ci-text-panning-observations-at-f2bb1d61) remain separate from the native model benchmark. The later complete `76d3744c` run likewise retained a high WebGL2 p95; neither run measures the new glyph-cache implementation.

The phase-separated benchmarks at `ee1999b9` and `40dafdab` completed after their failed editor suites, with **150 labels, 12 warmups and 36 measured motions per mode**. The later [`40dafdab` observations](../../docs/ferrodesign-evidence/2026-10-22-40dafdab-browser/README.md#text-pan-measurements) report dispatch through native-frame observation at **15.61 ms median / 17.43 ms p95** in Software2D and **16.12 / 1,310.39 ms** in WebGL2. Dispatch alone had p95 **13.89 / 1,304.93 ms**; the additional frame-observation wait had p95 **5.46 / 9.10 ms**. Cumulative native-presentation observation had p95 **103.07 / 1,397.97 ms**. The [earlier phase results](docs/VALIDATION.md#text-pan-phase-measurements-at-ee1999b9) remain separately preserved. These observations include automation, input/bridge work and scheduling; a presentation counter is not GPU completion. Phase quantiles are not additive, and these runs establish no cross-version speedup or recovery of the substantial WebGL2 tail. The later optional Chromium tracing code has no executed diagnostic result in these records.

The source adds bounded reuse of transformed unhinted glyph contours. Confirmed inline typing uses scoped node refresh when no auto layout exists; auto-layout documents retain full arrangement. The renderer skips preparation of the exact text leaf painted by the native editor while still refreshing its ancestors, then restores canvas text when editing ends. The application tests at `ee1999b9` and `2b0fe4e1` passed the corresponding 10,000-node suppression/history regressions. These work-count checks do not establish full-frame latency or FPS. [Text rendering](docs/TEXT_RENDERING.md) and [native text](docs/NATIVE_TEXT.md) describe the implementation boundaries.

`style_allocations` compares identical edits on linked and unlinked image-filled layers. It counts allocations and requested bytes, asserts equal overhead in each cohort, and deliberately omits timing. Existing history/cache/metadata copies remain in both measurements; this checks that binding comparisons avoid another payload snapshot, not that moves are allocation-free. Shared-definition propagation is still document-sized.

## Source map

| Path | Responsibility |
| --- | --- |
| `crates/ferrodesign-core/src/model.rs` | Document graph, paints, text, components, variables and validated serialization |
| `crates/ferrodesign-core/src/editor.rs` | Transactions, history and editing operations |
| `crates/ferrodesign-core/src/vector_edit.rs` | Bézier editing, exact curve picking/splitting and affine bounds normalization |
| `crates/ferrodesign-core/src/vector_faces.rs` | Checked planar faces, retained boundary geometry and indexed bucket queries |
| `crates/ferrodesign-core/src/vector_planarize.rs` | Bounded certified intersection splitting before native face authoring |
| `crates/ferrodesign-core/src/stroke_profile.rs` | Width presets, custom interpolation, bounded stroke bands and shared handle sampling |
| `crates/ferrodesign-core/src/corners.rs` / `polygon_corners.rs` | Continuous box corners and signed polygon/star fillets |
| `crates/ferrodesign-core/src/text_edit.rs` | Unicode-safe rich-text range preservation |
| `crates/ferrodesign-core/src/paragraphs.rs` | Hard-paragraph ranges, independent formatting and replacement rebasing |
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
| `crates/ferrodesign-app/src/canvas/vector_width.rs` | Native width-handle picking, captured gestures and transactional previews |
| `crates/ferrodesign-app/src/shell/` | Native authoring dialogs and the cached layer projection |
| `crates/ferrodesign-app/src/prototype.rs` | Presentation snapshots and cancellable FerroUI dispatcher timers |
| `crates/ferrodesign-app/src/renderer.rs` | Cached public-API scene renderer |
| `crates/ferrodesign-app/src/renderer/incremental.rs` | Editor-ledger preparation, ancestor updates and exact inline-text suppression |
| `crates/ferrodesign-app/src/renderer/flatten.rs` | Editable contours from public FerroUI geometry and complete text outlines |
| `crates/ferrodesign-app/src/renderer/text_export.rs` | Positioned glyph/decorative paths from the native text drawing sink |
| `crates/ferrodesign-app/src/renderer/root_index.rs` | Prepared-root bounds hierarchy, refitting and retained ordered viewport candidates |
| `crates/ferrodesign-app/src/platform.rs` | Browser/native storage, clipboard and downloads |
| `crates/ferrodesign-app/src/desktop.rs` | Vello native configuration and headless rendering |
| `web` / `scripts` / `tests` | Browser host, reproducible build and interaction validation |

See [architecture](docs/ARCHITECTURE.md) for rendering, invalidation and transaction details.

## License

New FerroDesign source is MIT-licensed, consistent with the parent repository. Refer to the repository license and individual dependency licenses. The sample uses its own name and sample artwork; it is not an official Figma product.
