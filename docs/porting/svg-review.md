# SVG backend review and validation

This review covers the SVG backend introduced in [PR #1435](https://github.com/wieslawsoltes/FerroUI/pull/1435),
its Skia recording support, and the compositor integration. Correctness fixes and their regression
tests precede the performance changes so measurements use the corrected implementation as the
baseline.

## UI semantic extension

The subsequent [UI semantic extension](svg-semantics.md) adds an explicitly captured, immutable
control graph alongside each selected rendered layer. Review kept the source graph separate from
Skia's composed artwork and prepared-fragment cache. It corrected logical/template ancestry
cycles, bounded collections containing empty values, made Grid placement part of the layout
projection, and resolved relationships against final application-customized IDs.

The completed extension passes **133 SVG tests with default features and 133 with `media3d`**,
including 21 new regressions for capture, exact immediate-renderer transforms, content selection,
inert XML escaping, namespace/reference collisions, hidden subtrees, invalid/bounded snapshots,
layer filtering/removal, raster/inspection output and cache reuse. Strict all-target/all-feature
Clippy, formatting, API-model, private-reference and idiom checks pass. Its standard-library
semantic reader passes 12 focused Python tests; the three existing renderer-validator tests also
pass. The unrelated full Skia/Base suites recorded below were not repeated for this extension.

The [real-control example](../../samples/SvgExport/semantics/README.md) records 20 UI nodes and 15
draw calls in one vector layer. Semantic and plain exports produce **identical 800 × 540 RGBA
pixels** through Inkscape 1.4. Their metadata differs while the painted scene does not; a rich
export after the plain export reports a prepared-vector cache hit. SVGZ decompresses to the exact
semantic SVG. The example includes a JSON extraction, preview and file/pixel fingerprints. Direct
Figma/Illustrator retention of the foreign metadata remains untested.

## Correctness findings and fixes

| Area | Failure found | Resolution and regression coverage |
| --- | --- | --- |
| Native layer blits | A Skia layer recovered its concrete destination through a downcast before the SVG adapter could validate the source. A GPU-affine image could enter a CPU recording and disappear on replay. | Added an optional destination `try_blit_layer` hook. The SVG implementation checks CPU readability, counts the operation, records explicit fallback, and delegates the original device-space blit. Tests cover retained CPU snapshots and an unreadable GPU-affine source without requiring hardware. |
| Interrupted capture | Dropping a balanced but incomplete recording during producer panic could publish successful partial artwork. A live native lease could cause a second panic during destruction. | Completion records an aborted-capture error, preserves the original producer panic, and publishes failure through the existing surface result slot. A readiness guard survives failed native finalization. Tests retain a previous successful frame, fail a new frame with a live lease, verify replacement by failure, and render a subsequent frame successfully. |
| Completion callbacks | A failed finalization could be retried by `Drop`, or leave a target permanently recording. | Finalization becomes terminal before native code and owns its cleanup guards locally. Public `on_finished` callbacks retain their finished-picture guarantee; internal completion also reports failed finalization. Secondary callback panics are contained only during an existing unwind. |
| Retained native features | A recording feature captured transform and opacity when requested, so a later lease could use stale state. | A lazily allocated state object tracks live context transform, DPI and opacity. Lease acquisition snapshots current state. Pixel tests exercise nonuniform DPI, clipping restores and opacity/compositing transitions. Normal recording does not gain eager canvas transform calls. |
| Native resource lifetime | Features and disposed leases could continue drawing after their context ended. Late disposal could restore an old transform onto a surface already used by a later frame. | Features and leases hold weak context-state references. Disposed or orphaned leases reject use; orphan cleanup cannot mutate a newer frame's canvas. Tests cover disposed/dropped contexts and shared-surface reuse. |
| Nested platform leases | Disposing a parent could unlock drawing while platform graphics remained leased. Repeated disposal of an old nested lease could release a newer lease. | Parent disposal requires the nested lease to end first, and platform-lease disposal is idempotent. A synthetic GPU interface tests exclusivity with raster targets. |
| Resolved brush coordinates | Finite raw radial/conic, stroke or tile-brush values could become nonfinite after resolving relative coordinates or stroke-expanded bounds. | Validate resolved gradient and tile coordinates before native recording, using the native intermediate DPI for image source rectangles. Tests cover fill/stroke paths, invalid viewports, valid HiDPI limits and empty tiles. |
| Raster device mapping | A small permitted raster allocation could still require an infinite native scale or translation. | Validate the composed native mapping before allocation and replay, including subnormal logical dimensions and extreme origins. |
| Wireframe image instances | Flattening `<use>` into a rectangle changed source/instance positions, transforms and width/height semantics. | Retain `<use>` and redirect it to a separate bounds resource. Preserve both transforms and original image resources. Tests validate the resulting resource graph and instance attributes. |
| X-ray image opacity | Multiplying a presentation attribute ignored inline CSS opacity, percentages and `!important`. | A compositing wrapper applies the inspection multiplier while preserving the original CSS and attributes. |
| Numeric precision | Adjacent tokens could merge after rounding: `1+2` could become `12`, or `1-0.001` could become `10`. Foreign metadata was also rounded by attribute name. | Preserve numeric token boundaries, including scientific notation, and limit SVG numeric normalization to SVG elements. Paths and matrices remain exact. |
| CSS resource parsing | URL-like strings and comments were treated as references; quoted parentheses were mishandled; unquoted rewritten semantic IDs could become invalid CSS. | Use CSS-aware token boundaries, comments and strings; preserve required quoting; reject malformed strings containing raw line breaks. Tests include repeat-optimization stability. |
| Inspection CSS | Comments around inline paint property names could prevent inspection overrides. | Parse declaration names through comments while preserving unrelated declarations. |
| Native image opacity | The pinned Skia SVG writer put image paint alpha in `fill-opacity` on an image `<use>`, which does not modulate SVG image pixels. A cropped bitmap recorded at 0.85 opacity became opaque. | A producer-specific normalization converts that attribute to element opacity only for verified references to image resources. It preserves the embedded PNG, intrinsic pixel alpha, crop and transforms. Generic SVG processing retains SVG's ordinary semantics. Native-recording tests cover opaque and translucent pixels at three draw opacities. |
| Independent coverage comparison | Comparing only RGB composited on white could miss opacity loss, for example an opaque white image against transparent white pixels. | The independent validator now reports alpha MAE/RMSE and changed coverage in addition to composited RGB. An optional alpha gate and dependency-free PNG tests exercise the previously invisible failure. |
| Outlined text versus device text | Small outlined glyphs differed from native raster text; a pixel comparison alone could not distinguish displacement from font hinting. | A regression compares actual exported contours with the recorded glyph IDs, fractional advances, offsets and baseline. Strong/aligned and unhinted/unaligned recordings produce identical vector paths. Documentation distinguishes portable outlines from device-specific hinting and baseline snapping. |
| Explicit image sampling and edge policy | The pinned SVG writer ignores native image sampling and primitive antialias flags, so explicit nearest/mipmap/cubic sampling or aliased geometry could silently change appearance. | Read the native context's effective render options at actual draws. Classify unsupported bitmap sampling and explicit aliased non-text primitives as composed fallback. Tests cover all sampling modes, inherited and restored options, preconfigured contexts, unused sampling scopes, a nearest-neighbor pixel edge and an aliased ellipse's binary coverage. |

The changes preserve explicit errors for unsupported resources and the existing whole-layer
compositing fallback. They do not replace missing GPU resources with transparent pixels or relax
strict-vector fidelity checks.

## Integration coverage

The new integration fixture runs a real window visual tree through layout, `CompositingRenderer`,
the manual render loop, the Skia platform adapter and `SvgSurface`. It checks:

1. A complete two-sibling frame remains vector in strict mode at 2× DPI.
2. A later frame includes an unchanged sibling when the other sibling changes.
3. Actual visual opacity produces composed raster fallback, with the expected device-pixel size
   and sampled colors.

This complements the lower-level render-target and drawing-context tests. It does not rely on
manually calling only the target's drawing methods.

## Independent rendering and text diagnostics

The benchmark fixtures exercise geometry, repeated definitions, HarfBuzz-shaped Inter text,
cropped translucent bitmaps and composed raster layers. Each workload has normal, x-ray,
wireframe, cutaway and exploded views, plus a Skia raster reference. The validator checks the
resource graph before rendering with Inkscape, then compares normal output against that
reference. Timing excludes fixture generation, rendering, parsing checks and pixel comparisons.

The bitmap fixture exposed the native image-opacity defect above. With the correction, mean
absolute RGB error decreased from **13.8267 to 0.2705 / 255**, and the alpha channels agreed at
every pixel (**0.0000 / 255** mean and maximum error). It retained eight vector layers and 128
editable embedded image elements; the correction does not rasterize the scene. Residual RGB
differences occur at independently sampled image edges.

The dense 14-pixel Inter text fixture has **6.5707 / 255** mean RGB error at one pixel per logical
pixel, primarily from the different hinted raster and unhinted outline representations. At four
pixels per logical pixel this decreases to **1.2355 / 255**; using unhinted, unaligned source text
in that diagnostic gives **0.8428 / 255**. The vector SVG is byte-identical across those text-option
and reference-density variations. The fractional-position regression establishes geometry
preservation independently of raster coverage. These figures describe that fixture, not a
universal tolerance for text. The selected raster fidelity policy is the route to the recorded
Skia pixel appearance at a specified density.

## Reproducing the checks

The correctness checkpoint on Linux x86_64 passed the following checks before optimization:

| Check | Result |
| --- | --- |
| SVG default build | 99 passed |
| SVG `media3d` build | 99 passed |
| Complete Skia library suite with the controlled Noto font setup | 603 passed, 9 existing ignored |
| Native lease regressions, included in the Skia suite | 7 passed |
| Existing native transform regressions, included in the Skia suite | 3 passed |
| Base compositor full-frame regressions | 3 passed |
| Independent validator alpha/PNG regressions | 3 passed |
| Strict SVG Clippy, all targets and all features | Passed with `-D warnings` |
| SVG formatting and `git diff --check` | Passed |
| Generated API model, private-reference and idiom conventions | Passed |

The default and feature-enabled SVG runs both include the new compositor, interruption,
resource-affinity and tile-coordinate tests. The `media3d` run also verifies rendered scene pixels.

```sh
cargo test --locked -p ferroui-svg --lib
cargo test --locked -p ferroui-svg --features media3d --lib
cargo test --locked -p ferroui-skia --lib
cargo test --locked -p ferroui-base --lib full_frame_render_tests
cargo clippy --locked -p ferroui-svg --all-targets --all-features --no-deps -- -D warnings
cargo fmt -p ferroui-svg --check
python3 -m unittest discover -s src/Svg/FerroUI.Svg/examples -p test_validate_exports.py
python3 scripts/generate_api_model.py --check
python3 scripts/conventions/private-references.py
python3 scripts/idioms/inventory.py --check
```

Font-manager tests depend on the host's Fontconfig policy as well as installed files. A strong
system-wide fallback alias makes otherwise absent families resolve, while a monochrome emoji
font can satisfy character fallback without selecting a family named `Emoji`. For the full Skia
run, use a controlled Noto Sans/Serif/Mono/CJK/Color Emoji font set and stock weak fallback rules.
This changes the test process's font environment, not the framework or its assertions. The SVG
benchmark prepares its embedded Inter text independently and records the font environment used
for both measurements.

The nine pre-existing ignored Skia tests comprise seven installed/Windows-font-dependent cases,
one profiling-only wrapping test and one platform-dependent glyph-overhang expectation. This
review did not add ignores or weaken those assertions.

## Optimization regression coverage

The performance round adds nine prepared-cache regressions and four XML regressions. Both the
default and `media3d` SVG configurations pass **112 tests** after these changes. The cache tests
compare warmed output against fresh cache-disabled output while varying inspection parameters,
native writer flags, per-layer selection, resource-prefix shifts, cache controls and outer document
options. They also verify supported signed-zero opacity values, bounded view switching, and
continued validation of invalid options and raster budgets. Zero stroke widths remain invalid.

The XML regressions check borrowed versus rewritten CSS tokens, pre-rename rejection of malformed
references, empty-definition pruning with stable IDs, and exact multilingual escaping. The
independent XML harness passes all **35 tests**. The [release performance report](svg-performance.md)
records the before/after measurements and the complete fixture invariance checks.

The tests involving a synthetic GPU interface validate resource affinity and lifetime contracts.
They do not establish real GPU import, device-loss or readback behavior. Figma and Illustrator
import remain separate application-specific validation; independent SVG raster comparisons do
not substitute for testing those applications.
