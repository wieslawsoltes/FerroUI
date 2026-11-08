# Quartz Metal / Skia contract audit

Status: identified contract gaps implemented and validated on the available Mac; hardware limits below. Baseline: `141b9551763306f593c48ed5b9d1e460a6352931`.
This audit compares the Ferro backend contracts on macOS, not every API exposed by Skia itself.
An implemented method is not sufficient evidence of matching behavior.

## Acceptance policy

Quartz/Core Text native text is the visual quality target. Preserve its native rasterization,
font metrics and grayscale coverage. Validate UTF-16 clusters, contextual shaping, fallback,
OpenType features, layout, caret/selection and editing independently of Skia's pixels. Skia
supplies a functional comparison and non-text pixel references, not a required text appearance.

Normal drawing, effects and video/camera composition must remain on the GPU. Explicit image
input, native glyph masks, bitmap locks and screenshot/encoding exports may transfer pixels;
media must not acquire an implicit CPU transfer path. Keep optional runtime shaders separate
from the smallest HelloWorld configuration. Hardware-unverified is distinct from unsupported.

## Contract inventory and findings

| Contract / operations | Disposition | Work / evidence |
|---|---|---|
| Render interface: backend creation, surface selection, readiness, resource limits | Implemented for native system-default Metal device | Metal context/generation tests; platform limits below |
| Render targets: drawing, retention, disposal, offscreen layers and shared snapshots | Implemented | Metal ownership/generation tests cover retained snapshots and stale objects; physical device removal is hardware-unverified |
| Drawing primitives: lines, geometry, ellipses, rounded rectangles, regions | Fixed in first audit pass | Proportional neighbor-based radius normalization; asymmetric-corner pixel fixture |
| State: transform, clear, clips (rect/rounded/region/geometry), opacity, masks | Fixed | Parent clips apply once when compositing isolated groups; stroke-only geometry does not create an implicit fill clip; existing singular-transform/state restoration fixtures |
| Compositing layers and bitmap blend modes | Implemented; bitmap option leakage fixed | Source inspection covers every BitmapBlendingMode mapping; GPU regression separates vector SourceOver from bitmap Destination; effect blends use shared numerical references |
| Brushes: solid, gradients, image and scene tiles | Implemented | Transformed/repeat/reflect/mirrored scene and image tile fixtures; fill and stroke bounds both contribute to brush coordinates |
| Render/text options and bitmap interpolation | Fixed bitmap edge coverage | Fractional and rotated bitmap quads match vector coverage; existing merged state, DPI hinted text, mipmap/cubic tests |
| Effect interfaces: blur, drop shadow, luminance, matrix, multiply, convolution, lighting, pixel folds | Implemented | Shared CPU references; parent/output clip regressions |
| Custom graphs and optional runtime shader effects | Implemented with optional compiler | Nine library shaders; image-child GPU test; omitted from minimal size graph |
| Acrylic material | Implemented | GPU material fixture and composed catalog comparison |
| Geometry: construction, groups, booleans and transformed geometry | Fixed in first audit pass | Reject foreign geometry; preserve separate fill/stroke bounds |
| Geometry: bounds, widened strokes, fill/stroke containment and intersection classification | Fixed in first audit pass | Fill-only curve bounds, exact boolean containment classification and emptiness without signed-area cancellation |
| Geometry: stream figures, non-stroked edges, arcs, closing and builder lifetime | Fixed in first audit pass | Preserve figure start across non-stroked edges; one-shot builder lifetime |
| Geometry: contour measurement, point/tangent, trim and exact path export | Implemented by shared contract inspection | Shared kurbo helper follows existing first-contour Skia behavior; exact path export retains separate fill/stroke paths |
| Bitmap: creation, native pixel-format/alpha capability and locks | Fixed in first audit pass | Native RGB565/BGRA/RGBA with declared alpha and exact CPU lock bytes; unchanged locks avoid upload/version changes |
| Bitmap: raw imports, strides, size validation and versions | Fixed | RGB565, unpadded final row, positive stride validation, unchanged locks retain version; invalid decode dimensions return InvalidInput |
| Bitmap: decode, resize and PNG/JPEG encode options | Fixed | JPEG quality validates before readback; all four PNG compression options use system zlib; integer aspect-ratio decode sizing and invalid dimensions have regressions |
| Font manager: default/system/embedded, family traits, locale fallback, stream access | Fixed native metadata | Loaded faces report actual weight/stretch/style; embedded matching ranks these traits; existing system-family, stream, variation and locale fallback checks |
| Text shaping/glyphs: clusters, offsets, ink bounds, geometry and underline intersections | Fixed OpenType settings | Four-character feature tags, UTF-16 owner ranges and upright native glyph images; Core Text context/RTL/color/weight/variation/hint fixtures and catalog RTL selection, replacement, undo and hit tests |
| Regions: decomposition, bounds, intersects/contains, reset and disposal | Compatible by source inspection | Same integer truncation/ceil policy as Skia, including negative fractional inputs; canonical band decomposition is shared with the existing native region implementation |
| External objects: imported frames/shared images, timeline snapshots and completion retirement | Implemented for Metal contracts | Native texture validation, producer overwrite/snapshot and GPU media smoke tests |
| Optional backend-specific leases and rendering fragments | Deliberate API difference/shared default | Skia API lease is backend-specific. Both current backends use the default unsupported fragment interface; not a Quartz-only rendering gap |

## Validation and size tracking

Baseline HelloWorld (`dist`, same XAML/theme, stripped fat LTO, unwinding):

| Configuration | Bytes | gzip bytes |
|---|---:|---:|
| Skia | 28,282,256 | 10,801,627 |
| Quartz Metal | 23,569,440 | 8,626,238 |
| Quartz Metal + runtime shaders | 24,992,992 | 9,270,066 |

Baseline tests: 42 Metal, 12 CPU reference, 10 context manager, 13 native-device tests.
Baseline catalog coverage: 15 pages, immediate/composed captures at matching 2× scaling.
New measurements and validation must identify the tested source; baseline artifacts are in
`target/quartz-parity/baseline.json`. The baseline is not a measurement of uncommitted audit fixes.

## Platform and hardware limits

The backend is macOS-only and uses the system-default Metal device with read/write RGBA8 tier 2,
shared events and MPS. OpenGL/Vulkan-specific leases, keyed mutexes and binary semaphores are
not Metal contracts. Wide-gamut/HDR presentation is not currently an established Ferro backend
contract; compare the exposed pixel-format APIs separately from claims about display fidelity.
Physical webcam acquisition, Intel/eGPU behavior and physical GPU removal cannot be verified
on this machine. The synthetic camera proves the GPU presentation path, not hardware capture.

## First implementation checkpoint (8 November 2026)

Eight new regressions reproduced failures before their corresponding fixes. The Metal suite
now passes 50 tests. The writable-input test covers all three native formats and all three
alpha requests, including low-alpha straight RGB that must survive CPU lock round trips exactly.
The GPU texture uses premultiplied pixels independently of that canonical writable memory.

The first size checkpoint saves 16.49% for minimal Quartz Metal and 11.45% with runtime shaders:

| Configuration | Bytes | Change from baseline |
|---|---:|---:|
| Skia | 28,282,256 | 0 |
| Quartz Metal | 23,619,088 | +49,648 |
| Quartz Metal + runtime shaders | 25,042,640 | +49,648 |

Reports (including gzip sizes, executable/source hashes and dependency checks) are in
`target/quartz-parity/phase-one/sizes.json` and
`target/quartz-parity/phase-one-runtime-shaders/sizes.json`. These are checkpoint measurements,
not final sizes for the active goal.

## Second implementation pass

The combined suite passes **58 Metal tests** (including runtime shaders and synthetic media
fixtures) and **24 CPU tests**. Sixteen contract regressions were added in
`src/Quartz/FerroUI.Quartz/parity_tests.rs`. Existing text fixtures remain independent of
Skia rasterization. Commands:

```sh
cargo test -p ferroui-quartz --features metal,runtime-shaders,media-test-fixtures -j 1 --lib -- --test-threads=1
cargo test -p ferroui-quartz -j 1 --lib -- --test-threads=1
```

Additional corrections after the first checkpoint:

- Honor PNG compression options through macOS system `libz`. ImageIO still produces the PNG
  and its metadata; only explicit PNG export recompresses its lossless image-data stream.
  This adds an OS dynamic-library dependency, not a bundled codec or a rendering readback.
- Reject nonpositive decode dimensions and use floor for the derived aspect-ratio dimension.
  Preserve a minimum of one pixel for thin valid images instead of creating a zero-size target.
- Apply antialiased parent clips once at isolated-layer composition, retaining filter input
  outside the parent clip. Identity opacity/group operations preserve edge coverage.
- Restrict bitmap blend options to image operations; vectors and glyphs retain SourceOver.
- Preserve partial coverage of fractional/rotated bitmap quads when their pixel center lies
  just outside the destination. Glyph masks without quad edges retain their own bounds checks.
- Match Skia's no-additional-clip behavior for geometries with no fill path, including
  when pushed under a singular transform before restoring the transform. Foreign geometry
  passed to factories/drawing fails explicitly rather than silently dropping content.
- Pass OpenType tags as the four-character strings required by Core Text. Apply feature
  settings over UTF-16 ranges of the containing text while preserving contextual shaping.
  A loaded Noto Sans fixture verifies global ligatures off and disabling just one of two
  ligatures in a nonzero-offset text run. Overlapping settings use the last active value.
- Rasterize standalone Core Text glyph images without the drawing-surface Y flip. The extra
  flip inverted hinted letters and color emoji; vector text was unaffected. A semantic
  orientation test checks an upright F and red-above-green in the traffic-light emoji,
  independently of native antialiasing. The mixed-script catalog page exposed the defect.

`Geometry::bounds()` now includes fill-only curves as a deliberate correctness improvement.
Skia's simple geometry currently reports stroke bounds from `bounds()` but unions separate
fill bounds in `get_render_bounds()`. Catalog assertions compare the rendering contract,
not this incidental Skia bounds limitation. Native geometric antialiasing and boolean-path
normalization need not be pixel-identical to Skia.

## Catalog validation

The new **Renderer parity** page exercises asymmetric corners, fill-only curve bounds, broken
stroke closure, native writable formats and mixed-script text. Its **Run text checks** action
asserts Arabic selection, selection hit testing, replacement and undo. The optional
`FERROUI_SMOKE_RENDERER_PARITY=1` hook invokes the same action after layout. The page skips
native formats that another backend does not advertise.

Fresh Quartz and Skia runs each captured **16 pages / 32 images** at the same **2200×1600**
pixel size and 2× scaling. Each page ran in its own process to avoid asynchronous snapshot /
navigation races. The comparison includes Renderer parity, Home, TextBlock, TextBox, Image,
Visuals, Composition brushes, Shadows and lights, Effect brushes, Acrylic brush, Blend and
colour effects, Colour effects, Image effects, Shader effects, Interactive effects and Page
transitions. Both backends pass the parity page's selection/replacement/undo action. All Quartz
capture logs report zero GPU errors; executable hashes and image dimensions were checked.

Manifests and captures are under `target/quartz-parity/final-quartz-catalog/` and
`target/quartz-parity/final-skia-catalog/`. The Skia executable was built at `228d5fe7a`; subsequent
changes affect Quartz-only code and documentation, so its compiled backend and shared catalog
inputs are unchanged. The final Quartz executable includes `d364b405a`. Animated samples are
compared by behavior and visible features, not their animation phase. Native font metrics and
rasterization remain the quality target; no text pixel-equivalence threshold is imposed.

The clean review app also passed real UI interactions: title-bar search, opening Renderer
parity, and clicking Run text checks. Double-clicking an empty caption maximized the window
from 2200×1600 to 3840×1920 and restored it to 2200×1600. The review app contains `18f7d87d7`;
the final no-fill/singular-transform correction does not change those exercised paths.

A fresh media smoke run passes bundled video playback, synthetic camera preview, navigation
retirement and QR decoding. Startup, video and camera checkpoints retain seven readbacks and
fourteen uploads: media adds zero CPU transfers. The later QR image operation adds unrelated
uploads. Video records five GPU imports/conversions; the synthetic 640×480 camera records
advancing GPU imports/conversions at 30 fps. Zero GPU errors are reported. Physical camera
acquisition is unverified because no webcam is available.

## Caption dragging regression

The native window path formerly inspected the immediate hit visual's chrome role. Unmarked
Grid/TextBlock template descendants therefore hid an ancestor TitleBar. Commit `09e030e1c`
uses the shared ancestor-aware chrome resolver already used on other platforms and consumes
handled caption presses so native tracking cannot forward a late client mouse-down.
Five shared chrome tests (including title-bar ancestry and interactive-child precedence) and
one native event-swallowing regression pass. On 8 November 2026, a physical mouse drag was
checked by hand: it moves the corrected Quartz Metal catalog window. This closes the
remaining caption-drag verification gate.

The UI automation's synthetic events reach AppKit with `pressedMouseButtons == 0`, so AppKit
declines to start a native move for those events. The physical test establishes the fix;
the synthetic-input limitation does not require replacing OS snapping/Spaces with a manual
move loop.

## Deliberate differences and remaining verification limits

This is parity against Ferro's exposed rendering contracts, not every Skia codec, platform
API or native Skia lease. Default Skia and the optional native graph remain separate. Core Text
is the text-quality benchmark; different native font metrics, rasterization and fallback
choices are expected. Font family enumeration describes installed families, as in the current
Skia font-manager implementation; loading a stream does not install that family system-wide.

GPU media import/composition is supported; physical camera capture remains unverified because
this Mac has no webcam. Synthetic capture validates texture interop only. Intel/eGPU devices,
physical device removal, HDR/wide-gamut fidelity and other operating systems require their
own hardware/platform evidence. These are not silently promoted to validated capabilities.
The audit closes the concrete functional defects identified above; it does not establish
exhaustive visual equivalence across every catalog interaction or every possible input.

## Final size checkpoint (8 November 2026)

Both measurements use clean source commit `d364b405a39d6f476a475c60b854a7d5b00552eb`,
the same HelloWorld UI, theme and `dist` profile. The final figures include the caption and
no-fill/singular-transform corrections. [Recorded results](quartz-parity-results.json) preserve
full executable/source hashes, commands, OS/compiler details, dependency checks and validation.

| Configuration | Bytes | gzip bytes | Saving against current Skia |
|---|---:|---:|---:|
| Skia | 28,282,240 | 10,800,990 | — |
| Quartz Metal | 23,619,184 | 8,655,412 | 16.49% |
| Quartz Metal + runtime shaders | 25,042,736 | 9,297,767 | 11.45% |

Minimal Quartz Metal grows by 49,744 bytes from the initial audit baseline while retaining a
16.49% saving against the current Skia executable. The optional runtime compiler costs
1,423,552 additional bytes; its configuration saves 11.45%. The current Skia build differs
from the baseline by 16 bytes after the common native caption change. No theme, asset,
unwinding or optimization setting was reduced to obtain these figures.

Both native dependency graphs exclude Skia, HarfBuzz, wgpu, Vello and ANGLE. The minimal graph
also excludes Naga; the runtime-shader graph includes it deliberately. PNG export now links
macOS `/usr/lib/libz.1.dylib`; no bundled zlib or image codec was added. Release HelloWorld
smoke runs complete with zero GPU errors and zero readbacks for both native configurations.

```sh
python3 scripts/measure-quartz-prototype.py --native-backend quartz-metal --output target/quartz-parity/final-size
python3 scripts/measure-quartz-prototype.py --native-backend quartz-runtime-shaders --output target/quartz-parity/final-runtime-size
FERROUI_SMOKE_RENDERER_PARITY=1 python3 scripts/capture-quartz-comparison.py \
  --binary target/quartz-parity/control-catalog-quartz-metal \
  --output target/quartz-parity/final-quartz-catalog --macos-bundle \
  --window-position 100,100 --expected-pixel-size 2200x1600 \
  --pages 'Renderer parity,Home,TextBlock,TextBox,Image,Visuals,Composition brushes,Shadows and lights,Effect brushes,Acrylic brush,Blend and colour effects,Colour effects,Image effects,Shader effects,Interactive effects,Page transitions'
```

Use the Skia catalog executable and a separate output directory for the comparison run.
Choose a window position and expected pixel size appropriate to the test display. Raw
screenshots are under the ignored `target/quartz-parity/` artifact directory, not shipped in
HelloWorld or committed as catalog assets.
