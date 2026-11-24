# SVG backend review and validation

This review covers the SVG backend introduced in [PR #1435](https://github.com/wieslawsoltes/FerroUI/pull/1435),
its Skia recording support, and the compositor integration. Correctness fixes and their regression
tests precede the performance changes so measurements use the corrected implementation as the
baseline.

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

## Reproducing the checks

The correctness checkpoint on Linux x86_64 passed the following checks before optimization:

| Check | Result |
| --- | --- |
| SVG default build | 89 passed |
| SVG `media3d` build | 89 passed |
| Complete Skia library suite with the controlled Noto font setup | 603 passed, 9 existing ignored |
| Native lease regressions, included in the Skia suite | 7 passed |
| Existing native transform regressions, included in the Skia suite | 3 passed |
| Base compositor full-frame regressions | 3 passed |
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

The tests involving a synthetic GPU interface validate resource affinity and lifetime contracts.
They do not establish real GPU import, device-loss or readback behavior. Figma and Illustrator
import remain separate application-specific validation; independent SVG raster comparisons do
not substitute for testing those applications.
