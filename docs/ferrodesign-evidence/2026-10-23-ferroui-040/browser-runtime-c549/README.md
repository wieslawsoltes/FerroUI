# Exact c549 browser interaction replay

**This selected-scenario replay failed.** Both the verified application runtime
and the immutable browser harness came from
`c549f6bcafe614a6373c743f5efbf2a3cb0eccd2`, tree
`8c5bbcc8ed5e5a61d287708cf4e2c0e7c912259d`. Chromium 156.0.8078.4 and
Playwright 1.64.1 exercised Software2D and WebGL2 sequentially, with the normal
common checks and four requested scenarios. The process exited with code 1.
One of eight complete scenario executions passed; seven failed. This is not a
full-suite, complete feature-parity or visual-parity result.

The [build record](../browser-build-c549/README.md) verifies the complete
distribution and exact source. All 55 frozen harness/package files still matched
their original hashes and c549 Git objects after the replay. No source input,
assertion, timeout or threshold was changed during execution.

## Completed checks and failures

| Scenario | Software2D | WebGL2 / SwiftShader |
| --- | --- | --- |
| Polygon corners | Failed after 1 group: 212 alpha differences above 12; allowance 85 | Same failure after 1 group |
| Gradient handles | **All 17 groups passed**, 46.490 s | Failed after 8 groups at native double-click stop insertion |
| Paragraphs | Failed after 6 groups at first composition preedit visibility | Same failure after 6 groups |
| Guide bindings | Failed after 4 groups at the fractional guide move expectation | Same failure after 4 groups |

Software2D completed 50 named checks, including 22 common checks; WebGL2
completed 38, including 19 common checks. The harness intentionally runs the
additional 10,000-node common fixture only in Software2D. Both modes completed
normal native text/IME entry, paint editing, compact-toolbar and durable reload
checks. Console and page error lists were empty, and both final states reported
`FerroUI Vello` with zero unsupported effects. These successful checks do not
override the scenario failures.

The Software2D gradient result includes affine endpoint and stop edits, exact
history and cancellation, held-drag Save/pagehide serialization, radial ellipse
and angular sweep controls, and independent exported-pixel checks. The current
native paint-card entry also succeeded in WebGL2; its later failure occurred
while waiting for a newly inserted stop after a real double-click. The saved
failure state still selected the axis and retained the two original stops.
The [input diagnosis](gradient-insertion-diagnosis.json) compares both original
traces: Software2D receives native click counts [1, 2] with presses 175 ms apart,
whereas WebGL2 receives [1, 1] with presses 994 ms apart. The latter interval
exceeds the unchanged 500 ms native double-click interval, so no insertion is
expected. The delayed Playwright double-click call takes 2,658.384 ms in that
case. Original DOM timestamps were not captured at this insertion, and the
precise blocking CDP phase is unknown. The native event counts and unchanged
document are preserved; a future harness correction must deliver a real timely
double-click and verify its native recognition without weakening insertion or
history assertions.

The polygon scenario now reaches native XAML export through the actual Inspect
tab, then returns to Design. Native PNG dimensions, transparent tip, interior
color and matching six-cubic SVG/XAML geometry passed before the direct 1× SVG
alpha comparison failed. The exported
[PNG](Software2D-polygon-corners-triangle.png),
[SVG](Software2D-polygon-corners-triangle.svg) and
[XAML](Software2D-polygon-corners-triangle.xaml) are retained with the original
failure. Further reference-rasterization analysis is a separate diagnostic and
does not retroactively change this result.

The paragraph clipboard correction reaches real repeated-Unicode paste,
replacement/deletion, rich-range preservation and native undo/redo checks.
The newly reached failure is the first composition update over a selected range:
the native control reports composition active, but its presenter preedit is
null. The confirmed document and history remain unchanged. The independent
[preedit diagnosis](paragraph-preedit-diagnosis.json) preserves exact before and
failure trace states and identifies a source-level ordering defect: selection
deletion synchronizes presenter text/caret after the first preedit is published.
A framework correction and a new runtime regression are required.

The guide failure has a specific input-fixture explanation. The imported native
view fits at zoom 2.953333333333333. The production snap radius remains five
screen pixels, or 1.693002257 world units. The test requests local x=47 and
expects attraction to x=51, which requires 11.813 screen pixels. The actual
result is [47, 51], with unchanged dimensions; the vertical coordinate already
lies on its guide. The [input diagnosis](guide-snap-input-diagnosis.json) records
both modes, source hashes and the exact geometry. Future pointer positions must
respect the existing screen-space radius; this is not a reason to enlarge that
radius or loosen the final geometry assertions.

## Performance scope and resources

The Software2D common fixture rendered 10,000 native layers while realizing 25
layer rows. Its nudge/undo/redo checks prepared one leaf, rebuilt no immutable
visuals and reused the layer projection; spatial-query and exact durable-reload
checks also passed. These are correctness and bounded-work observations on a
shared host, not a frame-rate promise.

The independent [strict text-pan benchmark](../browser-text-pan-c549/README.md)
completed both normal and trace cohorts on the same exact runtime/harness before
this replay. It is a separate successful result and retains the slow timing
tails and display-completion limitations.

Both owned browser and Node process IDs were absent after this replay. Shared
OOM counters stayed at 74 events / 6 kills from its pre-run baseline. Free disk
space fluctuated sharply because of concurrent host activity, but the complete
report, both trace ZIPs and failure exports were written successfully. No disk
or resource error is substituted for the concrete assertions above.

## Retained evidence

- [summary.json](summary.json) records every completed check, exact scenario
  status, duration and error, plus the Software2D 10,000-node observations.
- [report.json.gz](report.json.gz) contains the complete unabridged report with
  native failure states; [browser.log.gz](browser.log.gz) retains terminal output.
- [provenance.json](provenance.json) records all raw report/trace hashes,
  compressed-file hashes, source identity, retained exports and resource history.
- [source-provenance.json](source-provenance.json) records all 55 input hashes
  and the verified runtime artifact/WASM identity.
- The original Software2D and WebGL2 trace ZIPs remain available locally under
  the recorded report directory. Their exact hashes are retained here; these
  larger trace archives are not duplicated in Git.
- Current-source [initial canvas](Software2D-initial.png),
  [radial handles](Software2D-gradient-handles-radial.png) and
  [angular handles](Software2D-gradient-handles-angular.png) show the actual
  FerroUI application. Failure PNGs and complete exported documents are retained
  alongside the report, rather than replaced with successful later captures.

Every committed compressed record was decompressed and compared byte-for-byte
with its original. The earlier
[08 runtime / 0f harness replay](../../2026-10-23-continuation/browser-runtime08-harness0f/README.md)
remains a separate historical result and does not establish the outcome of this
FerroUI 0.4.0 runtime.
