# 2026-10-22 continuation evidence

This directory preserves the native results, browser failures, focused palette result and source identities for the layout-guide, matching-selection and vector-bucket continuation. The directory name identifies the first browser-test checkpoint; individual records below identify later checkpoints explicitly. It is not a claim that every file was produced by `f36dabf7`, nor a claim of complete Figma parity.

The subsequent recovered-runtime revalidation used harness [`f2bb1d61`](https://github.com/wieslawsoltes/FerroUI/commit/f2bb1d618843bf777b76859d2e06bf361bef009b) against the distribution built by CI at [`3a0bf1f1`](https://github.com/wieslawsoltes/FerroUI/commit/3a0bf1f13c6ede9abe5360bc3691085a3bc981f0). Only five browser-test files changed between those two commits. Its two unfiltered local attempts failed overall, although Software2D passed all 28 scenarios and 141 checks in each. The [separate recovered-runtime evidence index](../2026-10-22-f2bb1d61/README.md) preserves those terminal reports, the focused authoring diagnostic, the later WebGL2-only failure and the exact-source native results. [Validation](../../VALIDATION.md) distinguishes current gates from this historical checkpoint.

## Source and runtime identities

| Record | Source identity | Runtime identity |
| --- | --- | --- |
| First local complete attempt | Harness `f36dabf795f9e7b42ada46e34878b3ae5fd50697`; application `93a896797a37dd965a988c9e80e9187800d803fb` | Original local WASM, 51,209,724 bytes; SHA-256 `ea97294298eb4537155e0f84231387f4119c9a5ec6084d09e6687d630376f29f` |
| Palette diagnostic and second local complete attempt | Harness `3a0bf1f13c6ede9abe5360bc3691085a3bc981f0`; same application | Same original local WASM |
| Fresh complete revalidation | Harness `f2bb1d618843bf777b76859d2e06bf361bef009b`; CI build source `3a0bf1f13c6ede9abe5360bc3691085a3bc981f0` | CI artifact `12427798904`; WASM 51,205,801 bytes; SHA-256 `7880a628e7117ab88f896f69baa9c9ccedcdc33792302a01f653b4465e1499b9` |

The restored CI distribution is a different binary from the original local distribution. Their hashes are not interchangeable. The original local reports retain their original runtime manifests. The fresh run records the recovered CI payload separately. Rust 1.99.0, Emscripten 6.0.10 and wasm-bindgen 0.2.129 are pinned by the source; the browser runs use Chromium 156.0.8078.4. WebGL2 reports ANGLE/Vulkan/SwiftShader, rather than a physical GPU measurement.

## Preserved browser outcomes

| Attempt | Software2D | WebGL2 | Scope and raw evidence |
| --- | --- | --- | --- |
| First local complete attempt | 141 checks passed | 74 checks passed, then failed at the explicit layer-selection menu | `full: true`, overall **failed**; [original report, source manifest and failure image/document](failed-browser-first-run.zip) |
| Corrected palette diagnostic | 38 checks passed | 35 checks passed | `full: false`; only selection, matching and Flatten plus shared gates; [original report](targeted-palette-report.zip) |
| Second local complete attempt | 141 checks passed | 118 checks passed, then failed entering vector editing in the regional-image fixture | `full: true`, overall **failed**; [original report, source manifest and failure image/document](failed-browser-second-run.zip) |
| CI at `f36dabf7` | 88 checks passed, then failed the parent-resize geometry assertion | 95 checks passed, then exhausted the native-style screenshot deadline and its one retry | Overall **failed**; [selected unmodified artifact files and trace extracts](failed-ci-browser-f36dabf7.zip) |
| CI at `3a0bf1f1` | 88 checks passed, then failed the same parent-resize assertion | 118 checks passed, then failed the same regional-image vector-entry transition; no screenshot timeout observed | Overall **failed**; [decoded CI logs and extracted report](ci-3a0bf1f1-logs.zip), [extraction provenance](ci-3a0bf1f1-browser-failure-provenance.json) |

[prior-browser-summary.json](prior-browser-summary.json) is a derived index of the preserved original reports, including console/page errors, scenario status counts and capture attempts. A mode that completed inside an overall failed run is not presented as a passing complete run. The palette diagnostic's skipped scenarios remain skipped.

The first failure led to publication waits after filtering and recentering the native action palette. The command is clicked once and its actual outcome is required: selection actions replace or close the Actions palette; successful Flatten closes its dialog; rejected Flatten must produce a new exact native error toast; matching closes its dialog even when selection was already equal.

The second local failure and both CI geometry/region failures exposed keyboard-selection transitions whose new model state preceded the published native input scene. The `f2bb1d61` tests wait for a subsequent native frame and scene after selecting the exact parent or network root. They assert actual parent dimensions and child constraint geometry, and require vector-edit mode and canvas focus before clicking the bucket control. Unsplit face discovery requires its exact new intersection error. Native pointer/keyboard input, document/history invariants and pixel thresholds remain enforced.

The `f36dabf7` CI screenshot trace showed responsive native state, loaded fonts, an intact rendering context, unchanged document/view state and continuing frame delivery. Some trace frames arrived 27–32 seconds after their recorded swap time. That observation supports extending the existing single guarded retry from 15 to 60 seconds; it does not identify the exact queue responsible or establish that the longer retry recovers the failure. Diagnostic JPEGs in the archive are not substitutes for passing PNG captures.

Two later focused local runs passed before the local workspace was lost: four keyboard/paint scenarios at 46/43 checks and vector-network at 26/23 checks. Their latest raw reports were not recovered, so they are contextual observations rather than downloadable proof in this directory. The later recovered-runtime reports are preserved separately with their actual successes, failures and filtering scopes.

## Native and dependency evidence

- [core-tests.log](core-tests.log): 394 passing core tests, no failures or ignored tests.
- [native-tests.log](native-tests.log): 155 passing native application tests, including actual Vello pixel assertions, no failures or ignored tests.
- [ci-3a0bf1f1-logs.zip](ci-3a0bf1f1-logs.zip): the independently retrieved successful native CI log confirms those 394/155 results, all 19 persistence-host tests and the five target backend checks. [Native-log provenance](ci-3a0bf1f1-native-log-provenance.json) records its exact digest.
- [IndexedDB browser report](indexeddb-ci-3a0bf1f1-extracted-log.json): all five real-Chromium storage checks passed, including exact reload of 12,000,042 characters. [Extraction provenance](host-browser-ci-3a0bf1f1-provenance.json) also records the successful TypeScript check, ten browser input/composition bridge tests and JavaScript bundle in the same CI build. These gates do not imply that the later editor interaction suite passed.
- [framework-ci-summary.json](framework-ci-summary.json) and [framework-ci-3a0bf1f1-summary.json](framework-ci-3a0bf1f1-summary.json): completed framework CI at the two checkpoints, all 14 jobs successful in each run.
- [renderer-graphs](renderer-graphs): locked normal/build dependency graphs for Emscripten, Linux x64, Windows x64, macOS ARM64 and macOS x64. Vello is present and Skia is absent from the active application graphs. This is dependency evidence, not native GUI execution on five systems.

[original-logs.zip](original-logs.zip) keeps the exact original bytes of the local core, native and failed packaging logs. The corresponding readable `.log` files remove trailing whitespace and terminal blank lines only. The earlier [memory-killed build log](browser-build-oom.log) is retained; a later single-worker build succeeded. Its WASM identity is recorded above and in the original local source manifests.

## Native release benchmark

The [extracted performance JSON](native-performance/performance-extracted-log.json), [raw supporting step](native-performance/performance-step.log) and [provenance](native-performance/provenance.json) come from the successful native job in [run 39765407889](https://github.com/wieslawsoltes/FerroUI/actions/runs/39765407889/job/119783951316). This is decoded job-log evidence, not a byte-identity claim for a separately downloaded performance artifact. The performance JSON and raw step were recovered with exact matches to their previously recorded SHA-256 digests. The provenance file was regenerated and says so explicitly. [Complete decoded log and supporting metadata](ci-f36dabf7-logs.zip) are preserved separately.

The paired release measurement uses 10,000 nodes, one selected node and exact document/history equivalence in the same process:

| Operation | Scoped p50 | Explicit full-document reference p50 | Samples |
| --- | ---: | ---: | ---: |
| Nudge | 0.027372 ms | 16.479897 ms | 48 |
| Undo | 0.016031 ms | 3.731835 ms | 48 |
| Redo | 0.014918 ms | 3.527510 ms | 48 |

These engine timings exclude browser events, native control layout, drawing, GPU execution and persistence. They do not benchmark the new guide, matching or bucket paths and do not establish a frame-rate claim. The same report records size-dependent work: compact serialization is 21.233539 ms p50 for 12,802,055 bytes, parse/validate/index is 59.822304 ms p50, and component operations still using full-document history have medians of 21.314896 ms for variant changes, 20.904493 ms for text and 28.715560 ms for visibility.

## Captures and limits

The seven PNGs in this directory are illustrative native-editor captures preserved before workspace recovery: initial Software2D/WebGL2 shells, stacked guides in both modes, matching-fill controls in both modes and Software2D bucket hover. Exact per-file run/binary attribution was not recovered, so these images are not treated as primary source-tagged validation evidence or labeled as captures from the fresh recovered-runtime run. The separate failure archives retain their own original failure images, documents and available source manifests.

No matched Figma document/reference screenshot was available for a numerical visual-parity comparison. The implementation inventory and remaining gaps are in [FEATURES.md](../../FEATURES.md); guide, selection and vector-face contracts are in their respective implementation guides.
