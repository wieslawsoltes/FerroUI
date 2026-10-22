# Browser and text-pan feedback at `40dafdab`

Source: [`40dafdabc6ba939e34389bc6a03c5fd404833362`](https://github.com/wieslawsoltes/FerroUI/commit/40dafdabc6ba939e34389bc6a03c5fd404833362), confirmed by the checkout at [log line 130](browser-job.log#L130). [Run 39851782871](https://github.com/wieslawsoltes/FerroUI/actions/runs/39851782871), browser job [120079908067](https://github.com/wieslawsoltes/FerroUI/actions/runs/39851782871/job/120079908067), completed with **failure**.

## Build and executed editor scope

The job compiled the actual optimized WebAssembly application, verified six required distribution JavaScript modules, and reported **FerroUI Vello: WebGL2 hybrid with CPU fallback** at [lines 989–994](browser-job.log#L989-L994).

The [printed editor summary](printed-summary.json), extracted from [lines 1311–1391](browser-job.log#L1311-L1391), records `passed: false` and `scope: { full: true, modes: ["Software2D", "WebGL2"], scenarios: "all" }`. This identifies an unfiltered invocation; it does not mean all scenarios completed.

| Mode | Actual surface | Completed checks | Scenario progress | Result |
| --- | --- | ---: | --- | --- |
| Software2D | Software2D | 24 | `vector-network`, `native-authoring`, then failure within `corners` | Failed |
| WebGL2 | ANGLE / Vulkan SwiftShader Device (Subzero) | 24 | `vector-network`, `native-authoring`, then failure within `corners` | Failed |

Scenario starts are at [lines 1294–1296](browser-job.log#L1294-L1296) and [1301–1303](browser-job.log#L1301-L1303). The first two scenarios returned before the third began. Later feature scenarios were not reached. The 24 checks include the common startup/input/import/export gates and the first completed smoothing check; they are not 24 completed feature scenarios.

Both modes fail the same independent SVG-decoder versus native FerroUI PNG silhouette assertion, reporting **288** differing alpha pixels at `tests/corners-browser.mjs:77`: [Software2D failure](browser-job.log#L1297-L1300) and [WebGL2 failure](browser-job.log#L1304-L1310). The prior smoothing check completed in both modes, including invalid Enter, Escape restoration, and exact document Undo/Redo. This browser failure therefore occurs after that input path. It is distinct from the native fixture's canonical `"12.50"` versus `"12.5"` display failure recorded in the [separate native evidence](../2026-10-22-40dafdab-native/README.md).

The decoded log establishes the failed comparison and its location. It does not establish its geometry, export, or decoder cause. No correction or successful silhouette comparison is claimed here.

## Real IndexedDB execution

The separate persistence command passed all **five** checks in Chromium **156.0.8078.4**, before editor validation. [indexeddb-summary.json](indexeddb-summary.json) is the unchanged JSON printed at [lines 1252–1279](browser-job.log#L1252-L1279). It covers atomic legacy/current/previous snapshot migration, transaction abort after a successful write request, stale-revision protection across two tabs, a document exceeding 10 MiB surviving reload without localStorage, and preservation of the original document after an aborted migration.

The large-document observation records **12,000,042 characters**, a successful flush, IndexedDB phase `saved`, no dirty or pending serialization flags, and a printed save duration of **41 ms**. This persistence pass does not override the editor suite failure.

## Text-pan measurements

The subsequent text benchmark command completed successfully and printed **36 measured native pan observations per mode** using the `compact-v1` metric bridge. Rounded output appears at [lines 1405–1410](browser-job.log#L1405-L1410).

| Mode | Input dispatch → observed native frame, median / p95 | Dispatch p95 | Additional frame-observation p95 | Input dispatch → native presentation p95 |
| --- | ---: | ---: | ---: | ---: |
| Software2D | 15.61 / 17.43 ms | 13.89 ms | 5.46 ms | 103.07 ms |
| WebGL2 | 16.12 / 1310.39 ms | 1304.93 ms | 9.10 ms | 1397.97 ms |

[text-pan-phases.json](text-pan-phases.json) contains the two complete JSON objects printed at **lines 1407 and 1410**, wrapped in an array. Each phase distribution has 36 samples. The objects retain their full precision, three slowest sample observations per mode, trusted pointer events, long-task observations, native frame/presentation deltas, and preparation counters. They are the printed diagnostic subset, not the unavailable complete benchmark artifact.

For the three printed slow WebGL2 samples, browser event timestamp → capture observation spans approximately **1297.4–1307.8 ms**; capture → bubble spans approximately **0.3–0.4 ms**. Those differences use the same page clock. Their preparation deltas are zero and each sample advances native rendering and presentation. These observations locate recorded delay before event capture in these samples, but do not identify a scheduler, compositor, GPU, or transport cause. Host phase durations use the Node clock and must not be subtracted from page timestamps. The presentation interval includes dispatch; phase percentiles are not additive. The software GPU result is not physical-GPU execution timing, and its high tail does not support a general fast-rendering claim.

## Saved files and limits

| File | Saved UTF-8 bytes | SHA-256 |
| --- | ---: | --- |
| [browser-job.log](browser-job.log) | 147,890 | `5ac294e264d3b0a2c5b385f7349297bf4ca1da29ebb1cff549b3c36f4c71e141` |
| [printed-summary.json](printed-summary.json) | 6,848 | `0bbb12e74e68227eaa67f838455e1929efbc42306e66023140aac6a6149d8665` |
| [indexeddb-summary.json](indexeddb-summary.json) | 871 | `4920eeb5405e78a22166b438e8e607df5944025511f0b8034f5899b7b1f48b03` |
| [text-pan-phases.json](text-pan-phases.json) | 10,497 | `70d908f1c40e2cf38d9eb3b6f83909b0418bade08a42aa6a08dbb06a20a8a34e` |

The log is ordinary connector-decoded job text saved with a final newline. JSON extraction removes only each GitHub timestamp prefix; the phase file additionally supplies array delimiters. No printed values or outcomes were changed. Hashes identify these saved UTF-8 files, not original transport bytes. No official artifact ZIP, raw screenshot/trace contents, artifact CRC, or runtime-byte comparison was retrieved or verified for this record. The printed editor summary does not include complete error arrays or capture/retry metadata, so those are not inferred.

This source predates the canonical-display fixture correction [`54742a5e`](https://github.com/wieslawsoltes/FerroUI/commit/54742a5ef53fbcdde7e9e5f48ccbda8325ee6b7f), the subsequent Controls lifecycle/history corrections, and the native radius handles at [`38713369`](https://github.com/wieslawsoltes/FerroUI/commit/38713369933656306b833f97fc02ff2c9ef01202). It also predates optional Chromium tracing at [`124ffdb7`](https://github.com/wieslawsoltes/FerroUI/commit/124ffdb74e2b39ca09d5a1bcd4052bbb9a61a165). None of those later changes has a browser or trace result in this evidence, and no Figma visual-parity conclusion follows from it.
