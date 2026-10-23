# Rounded-polygon SVG coverage diagnosis

The frozen `c549f6bc` browser run successfully exported the rounded triangle as
native PNG, SVG and XAML. Both rendering modes then failed the unchanged
silhouette assertion: 212 pixels differed by more than 12 alpha units, against
an allowance of 85. The original failure and exports are retained in
[browser-runtime-c549](../browser-runtime-c549/README.md).

This separate Chromium 156.0.8078.4 diagnostic used the saved Software2D PNG and
SVG. It did not run the application or change the document. It reproduced the
212 differences with ordinary 1× SVG decoding, then applied the same independent
16×/32× area-coverage reference already used by the box-corner browser test.
Only the fixed fixture dimensions changed from 200×140 to 180×160. The original
reference-convergence and candidate-image thresholds stayed unchanged.

| Comparison | Pixels differing by more than 12 alpha units | Maximum alpha difference | Original 85-pixel gate |
| --- | ---: | ---: | --- |
| Native PNG versus ordinary 1× SVG decoding | 212 | 83 | Failed |
| Native PNG versus converged 32× SVG area coverage | 0 | 12 | Passed |
| Deliberately shifted SVG, one pixel horizontally | 468 | 235 | Failed |

All 212 original differences involve partial edge coverage; none is an opposite
opaque/transparent pixel. The independently decoded 16× and 32× references
differ by at most **3.30078125 alpha units**, below the existing limit of four,
with no opposite opaque/transparent pixels. The shifted negative control also
converges and still fails the candidate-image gate. It confirms that this
reference does not erase an actual one-pixel placement error.

The evidence supports correcting the raster comparison reference. No polygon
geometry, application rendering, color, export dimension, picking, history or
numeric tolerance was changed. It does not establish Figma pixel parity or an
absolute error bound for every independent rasterizer.

## Harness correction

The box and polygon tests now share `svg-alpha-reference.mjs`. It keeps the
exported viewBox and path data intact, changes only the declared SVG viewport,
draws at intrinsic resolution with integer translations, and reads sixteen
output rows at a time. Only averaged alpha values cross the browser protocol.
The maximum readback remains about 13 MB, including the existing larger-width
box fixture.

The pure helper's bounded output domain grows from 28,000 to 28,800 pixels to
include the polygon fixture. The new boundary test accepts that exact size and
rejects one additional convergence pixel; malformed buffers and oversized
averaging remain rejected. All seven pure coverage tests and all three touched
browser-module syntax checks passed. The box test's interaction/assertion body
and the scalar comparison function remain byte-identical after extraction.

The polygon test retains its raw 1× comparison in the report, requires 16×/32×
convergence within four alpha units, and then applies the original 12-alpha /
85-pixel gate. Native interior/tip pixels, six-cubic SVG/XAML geometry, exact
document state and subsequent star/history checks remain required. An actual
editor rerun of that corrected harness is a separate gate.

## Preserved records

- [diagnostic-report.json](diagnostic-report.json): original and converged
  comparisons, negative control, input hashes and clean browser shutdown.
- [diagnostic-executed.mjs](diagnostic-executed.mjs): exact scratch script as
  executed, including its original absolute input and tool paths.
- [provenance.json](provenance.json): source/runtime scope, script/report hashes,
  helper checks and unchanged thresholds.

The exact executed script records the historical scratch environment. The
normal repository polygon browser scenario is the portable reproduction path
for the shared reference helper.
