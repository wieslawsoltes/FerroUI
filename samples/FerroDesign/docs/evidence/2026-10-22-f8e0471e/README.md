# 2026-10-22 `f8e0471e` matching publication validation

The corrected matching regression passed in both requested browser surfaces. This is a **focused run**, with `scope.full: false` and `scenarios: ["matching"]`. It does not replace a successful unfiltered two-mode suite. The [preceding evidence index](../2026-10-22-f2bb1d61/README.md) preserves the failed original runs and their diagnoses without changing their outcomes.

## Source and runtime

| Input | Exact identity |
| --- | --- |
| Published harness | [`f8e0471eb4301a9b893e8185810550ebba5ddafc`](https://github.com/wieslawsoltes/FerroUI/commit/f8e0471eb4301a9b893e8185810550ebba5ddafc) |
| Harness tree | `bd39bd8077fc0c2060164ef5a0c5e572b4c30c6b` |
| Changed input from `f2bb1d61` | Only `tests/matching-browser.mjs`; SHA-256 `a6a20bd9fb0254dbbec64364dc3532eb1bd2d6919ac66e890133d139e6e59eb1` |
| Actual application distribution | Built by CI at `3a0bf1f13c6ede9abe5360bc3691085a3bc981f0`; artifact `12427798904` |
| WASM payload | 51,205,801 bytes; SHA-256 `7880a628e7117ab88f896f69baa9c9ccedcdc33792302a01f653b4465e1499b9` |
| Browser | Chromium 156.0.8078.4, Playwright 1.64.1, Node 24.19.0 |
| Execution | Fresh Chromium; original native input and assertions; no predicate instrumentation |

[validation-source.json](validation-source.json) and [input-equivalence.json](input-equivalence.json) record the exact inputs. All 31 harness/package files match the published commit; only matching differs from the preceding frozen harness. All 16 recovered runtime file hashes match before and after execution. Application/build inputs are unchanged from the distribution's source; six browser-test files differ from that build checkpoint. No new application binary or production behavior is attributed to this test-only commit.

## Why the test changed

The original WebGL2-only attempt completed 16 scenarios and 82 checks, then failed in the second matching-dialog cycle. The native selection model and live control bounds changed before the new inspector was presented. Its retained trace image showed the empty-selection Page inspector after the selecting canvas click, while the first frame showing the selected Rectangle inspector appeared after the attempted inspector-button click. The old frame/scene floors could be satisfied by the pending Escape transition.

The [original diagnosis](../2026-10-22-f2bb1d61/failed-matching-diagnosis.json) contains exact timestamps, states and image references. It supports a missing synchronization boundary; the bridge did not expose the actual native hit-test recipient, so it does not prove which control received the failed click.

The correction serializes the two selection transitions. When a selection exists, the test captures its pre-Escape frame/scene counters, sends Escape once, waits for empty selection and newer native frame/scene notifications, and verifies that the Page inspector is shown while the layer inspector and matching action are hidden. It then uses the existing canvas-selection input and publication gates. A redundant Escape is skipped when selection is already empty. Each matching dialog must become visible after its single opener click before its command row is used.

All original exact selections, authored-document equality, revision, history and redo checks remain. The correction adds no action retry or fixed delay and keeps the 10-second predicate deadlines. Frame/scene counters remain coalesced notifications rather than per-mutation hit-test versions. The successful focused result supports this corrected sequence under the recorded conditions; it does not establish a universal input-timing guarantee.

## Completed focused result

The run started at **16:16:31 UTC** and completed at **16:19:41 UTC**. The measured test-process duration was **180.129 seconds**, with exit code 0. The completion timestamp includes terminal collection after the test process exited.

| Requested mode | Actual surface | Passed checks | Requested matching checks | Shared native captures |
| --- | --- | ---: | ---: | ---: |
| Software2D | Vello Software2D | 25 | 3 | 2, both first attempt |
| WebGL2 | Vello WebGL2 through ANGLE/Vulkan/SwiftShader | 22 | 3 | 2, both first attempt |

The scope includes the harness's common drawing, import/export, persistence and compact-layout gates; Software2D also executes the 10,000-layer workload. The other 27 feature scenarios remain explicitly `not-requested` in each mode. All four native captures passed without a retry. Neither mode reported a page exception. Software2D retained one literal resource-404 console message; WebGL2 reported no console errors. SwiftShader results are not physical-GPU throughput measurements.

[matching-focused-summary.json](matching-focused-summary.json) records the terminal result, scope, resources, captures and post-run input integrity. [matching-focused-report.zip](matching-focused-report.zip) preserves the unchanged 416,968-byte raw report, source/input/post-run manifests, browser log, resource samples, runner, terminal records and both matching PNGs. The raw report SHA-256 is `3e2878479902033c81502eaa45685bb19f0457d323da5399ab8f0dfcc4a0410a`. The ZIP has a per-member hash manifest and passed CRC verification. It is a selected evidence archive; the original large Playwright trace ZIPs are omitted.

### Software2D workload observations

The focused run's 10,000-layer fixture retained 25 realized layer controls before and after scrolling. Import through first-frame/state verification took 1,297 ms; keyboard nudge through preparation verification took 295 ms. Nudge, undo and redo each prepared one node incrementally, rebuilt no immutable visual, and reused the layer projection without rebuilding it. Zooming to the selected node tested 27 bounds and found one candidate among 10,000 roots.

Compact autosave serialization took 29 ms and produced 13,192,068 bytes. Reload through exact restored-document verification took 1,578 ms; the portable export was 20,712,219 bytes. Its exact digest matched the preceding workload. The separately recorded 5 ms flush wait is not end-to-end save latency because pending background work may already have progressed. These are single observations on a shared host, not percentiles or FPS. [The native release benchmark](../2026-10-22-f2bb1d61/native-performance/SUMMARY.md) measures a separate model/transaction workload.

### Resource observations

The external sampler recorded 38 samples at five-second intervals. The shared cgroup reached its 8,589,934,592-byte memory limit; `memory.events.max` increased by 11,069, while `oom` and `oom_kill` had zero delta. Peak summed RSS for PID/path-matched processes was 1,894,105,088 bytes. No matching process remained in the final sample. The [sampler audit](../2026-10-22-f2bb1d61/resource-sampler-audit.json) explains why this is neither exclusive application memory nor a complete process-tree/PSS inventory. These values do not attribute earlier failures to a process or cause.

## Remaining validation

One WebGL2-only run of the remaining 11 feature scenarios was authorized: geometry authoring, Flatten, local styles, angular paints, grid styles, regional paints, network bucket, compact autosave, prototype input, prototype motion and bounded history. A transient sandbox mount-space error paused preparation; ordinary sandbox execution subsequently recovered without deletion or escalation. The launch then passed the disk and memory gates, but the default sandbox denied the harness HTTP server's listen on `127.0.0.1` with `EPERM`. Chromium never started, no browser report was created, and **zero checks or scenarios ran**. This is an environment launch block, not an application scenario failure. No escalation, alternate execution route, browser-security change or retry followed the denial.

[remaining-webgl2-launch-summary.json](remaining-webgl2-launch-summary.json) records the five-second process exit and unchanged 31 source / 16 runtime hashes. [remaining-webgl2-launch-block.zip](remaining-webgl2-launch-block.zip) preserves the raw error log, runner, terminal/resource records, manifests and integrity checks with a per-member hash manifest. The remaining 11 WebGL2 scenarios have no result from this attempt. The preceding original harness had already completed the first 16 WebGL2 feature scenarios; the focused corrected run above covers matching only. These observations do not constitute a complete current two-mode suite pass.

At **16:19 UTC**, the original exact-`f2bb1d61` browser job in [run 39799180117](https://github.com/wieslawsoltes/FerroUI/actions/runs/39799180117) was still in its editor-validation step. The new exact-`f8e0471e` [FerroDesign run 39812414651](https://github.com/wieslawsoltes/FerroUI/actions/runs/39812414651) was pending with no jobs returned; its separate [framework run 39812425327](https://github.com/wieslawsoltes/FerroUI/actions/runs/39812425327) was in progress. No overall CI pass is inferred from these observations. Native/core results remain separately source-tagged in the preceding evidence index.

## Subsequent complete CI result

The independent exact-`f2bb1d61` browser job subsequently **completed successfully at 16:43:12 UTC**, as observed through GitHub at 16:50:41 UTC. Its connector-decoded log contains a parseable result with `passed: true`, `scope.full: true`, both rendering modes and all scenarios requested: **141 Software2D / 138 WebGL2 checks**, with all **28 scenarios** started in each mode. Two WebGL2 native captures used the permitted guarded retry. The separate real-IndexedDB suite also passed. The [complete CI evidence](../2026-10-22-f2bb1d61/README.md#exact-source-complete-browser-ci) preserves the decoded job log, extracted JSON, canonical API metadata and verification limits.

The [committed-input comparison](../2026-10-22-f2bb1d61/successful-ci-input-equivalence.json) confirms that production application/framework/build inputs are unchanged between that successful source and `e0d3cee`; the only subsequent non-documentation change is the matching-test correction recorded above. The complete CI pass therefore validates those production inputs with the original `f2bb1d61` harness. It is separate from this corrected harness's focused pass and the zero-check local launch block; no exact-`f8e0471e` complete-suite pass is inferred. The `f8e0471e` queued workflow was later cancelled without any browser jobs, and the `e0d3cee` workflow was observed in progress at 16:50:41 UTC.

The successful CI artifacts have canonical API IDs, advertised sizes and SHA-256 digests, but their temporary download URLs could not be fetched through the ordinary local network path. The preserved log-derived ZIP was verified locally; the official validation ZIP's raw report, capture sidecars and CRC were not independently inspected. The successful browser build's bytes are likewise not claimed identical to the recovered `3a0bf1f1` distribution used for this focused run. The later success does not change any earlier failed or blocked outcome.

## Native screenshots

The [Software2D initial shell](Software2D-initial.png), [WebGL2 initial shell](WebGL2-initial.png), [Software2D fill matching](Software2D-native-matching-fill.png) and [WebGL2 fill matching](WebGL2-native-matching-fill.png) are actual PNGs from this focused run. They show the editor rendered through FerroUI. No matched Figma reference document was available for a numerical pixel-parity comparison.
