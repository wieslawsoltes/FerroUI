# 2026-10-22 `f2bb1d61` browser and native evidence

This directory preserves the **successful complete CI workflow at exact source `f2bb1d61`**, verified from GitHub API status and the decoded browser job log: **141 Software2D / 138 WebGL2 checks**, with all **28 scenarios in each mode**, plus the successful native gate. It also retains two failed local full attempts, the intervening focused authoring diagnostic and a failed WebGL2-only attempt against the recovered `3a0bf1f1` distribution. Those failures are not erased by the independent CI pass. The later `f8e0471e` matching correction passed its **25 Software2D / 22 WebGL2** focused checks; its additional 11-scenario attempt was blocked before startup. [Validation and measurements](../../VALIDATION.md) separates the tested sources, runtimes, outcomes and limits.

## Exact-source complete browser CI

[FerroDesign workflow 39799180117](https://github.com/wieslawsoltes/FerroUI/actions/runs/39799180117) was observed **completed/success at 16:50:41 UTC on 22 October 2026**. Its [browser job 119894677332](https://github.com/wieslawsoltes/FerroUI/actions/runs/39799180117/job/119894677332) completed successfully at **16:43:12 UTC**. The decoded log verifies checkout `f2bb1d618843bf777b76859d2e06bf361bef009b`, the successful Vello WASM build and the complete editor result. This run built its own runtime; it did not use the recovered `3a0bf1f1` distribution described below.

| Verified browser observation | Software2D | WebGL2 |
| --- | --- | --- |
| Status | **Passed** | **Passed** |
| Passed checks | **141** | **138** |
| Feature scenarios | All **28** | All **28** |
| Actual reported surface | Software2D | WebGL2, ANGLE/Vulkan/SwiftShader |

The printed JSON reports `passed: true`, `scope.full: true`, both modes and `scenarios: "all"`, using Chromium **156.0.8078.4**. The log independently names all 28 scenario starts per mode. It also records **two** guarded screenshot retries, for `WebGL2-native-local-styles` and `WebGL2-native-grid-style`, before the complete suite passed. No all-first-attempt capture claim is made.

The [successful CI summary](successful-ci-browser-summary.json) records the source, exact checkout/build evidence, counts, retry messages and limitations. [successful-ci-browser-log.zip](successful-ci-browser-log.zip) is a locally created, CRC-checked archive containing the full connector-decoded job log, the exact printed JSON from log lines 1243–1554 and member checksums. The extraction removes only the timestamp prefix and preserves the payload's indentation, content and final newline. The 44,244-byte JSON has SHA-256 `f1a6260b96c3b50b69e58d5954715842fe2a911be0cb371811e9a3f9849b6c40`.

This derived evidence ZIP is **not** the official validation artifact. [Canonical API/artifact metadata](../../../../../docs/ferrodesign-evidence/2026-10-22-f2bb1d61/successful-ci-artifact-metadata.json) records validation artifact `12441488836` and build artifact `12441488818`, including their API-reported sizes and digests. Ordinary retrieval did not succeed; the [build-download status](successful-ci-build-download-status.json) records the runtime attempt. No network workaround or escalation was used. The official ZIP bytes/CRCs, raw `report.json`, capture sidecars, total capture counts, detailed page/console error arrays and runtime-member hashes were not independently read or verified. Accordingly, no zero-console/page-error claim or binary equality with the recovered local runtime is asserted for this successful run.

The same job separately passed all [**five actual Chromium IndexedDB checks**](successful-ci-persistence-summary.json), including the **12,000,042-character** document reload. This is distinct from the **19 native persistence-host tests** and from the full FerroUI editor suite. The native job's **394 core + 155 application tests** and five active renderer-graph audits remain recorded in [the native section](#exact-source-native-ci).

### Production inputs and later harness

[successful-ci-input-equivalence.json](successful-ci-input-equivalence.json) records the GitHub comparison from tested `f2bb1d61` through source-delivery checkpoint `e0d3cee3`: four commits and 84 changed paths, consisting of README/evidence documentation and one matching browser-test module. No application, framework, XAML, Cargo manifest/lockfile, asset, browser-host or build-script input changed. Thus the complete CI result covers the current production inputs with the earlier `f2bb1d61` harness. It does not validate the later matching helper as part of a complete suite or establish matching runtime bytes. The later helper has its separately preserved focused pass. At **16:50:41 UTC**, [exact-`e0d3cee3` workflow 39814628155](https://github.com/wieslawsoltes/FerroUI/actions/runs/39814628155) was in progress.

### Browser text-panning measurements

The job subsequently ran [the 150-label text-panning fixture](../../../tests/text-render-benchmark.mjs), with **12 warmup motions** and **36 measured observations per mode**. The timer covers host pointer dispatch until the expected camera position and a newer native frame are observed; two later animation callbacks form a separate settling observation.

| Mode | Printed median | Printed p95 |
| --- | ---: | ---: |
| Software2D | **14.28 ms** | **20.62 ms** |
| WebGL2, SwiftShader requested | **20.30 ms** | **1,366.94 ms** |

The [derived text-panning summary](successful-ci-text-pan-summary.json) preserves log lines 1567–1568 and the exact-source method. The source requests ANGLE/SwiftShader, but the benchmark's original device fingerprint was not retrieved. These are rounded printed aggregates; the official artifact's per-sample data was not retrieved. They include automation and scheduling, are separate from the native engine benchmark, and establish neither FPS nor isolated GPU time. The high WebGL2 tail remains visible. SwiftShader is a software Vulkan device, so no physical-GPU performance claim follows.

## Source and runtime identity

The following identity table applies to the preserved **local recovered-runtime attempts**, not the successful CI build above.

| Input | Exact identity |
| --- | --- |
| Original browser harness | [`f2bb1d618843bf777b76859d2e06bf361bef009b`](https://github.com/wieslawsoltes/FerroUI/commit/f2bb1d618843bf777b76859d2e06bf361bef009b), tree `054e0d03d4d49f327e30e62f22ef78609618a5d2` |
| Recovered CI distribution | Built at [`3a0bf1f13c6ede9abe5360bc3691085a3bc981f0`](https://github.com/wieslawsoltes/FerroUI/commit/3a0bf1f13c6ede9abe5360bc3691085a3bc981f0), from GitHub artifact `12427798904` |
| Actual WASM payload | **51,205,801 bytes**, SHA-256 `7880a628e7117ab88f896f69baa9c9ccedcdc33792302a01f653b4465e1499b9` |
| Documentation checkpoint | [`177f92cdb743d1897aa65a8c8c92e5b4b844e521`](https://github.com/wieslawsoltes/FerroUI/commit/177f92cdb743d1897aa65a8c8c92e5b4b844e521), containing documentation and evidence changes only |
| Later matching-test correction | [`f8e0471eb4301a9b893e8185810550ebba5ddafc`](https://github.com/wieslawsoltes/FerroUI/commit/f8e0471eb4301a9b893e8185810550ebba5ddafc); one test file changes, with the same application/runtime inputs |
| Browser tools | Chromium **156.0.8078.4**, Playwright **1.64.1**, Node **24.19.0** |
| Rendering | FerroUI Vello; requested Software2D and WebGL2 surfaces. WebGL2 reports ANGLE/Vulkan/SwiftShader |

[validation-source.json](validation-source.json) records the distribution, lockfile, dependency graph, module checks and individual harness/runtime hashes. The recovered artifact ZIP matched GitHub's recorded size and SHA-256 and passed its CRC check. This runtime differs from the original local binary used by the [preceding continuation attempts](../2026-10-22-f36dabf7/README.md); those runtime identities are not interchangeable.

[input-equivalence.json](input-equivalence.json) records the five browser-test files changed from build source `3a0bf1f1` to harness `f2bb1d61`, with no application or build-input changes. It also verifies 31 harness/package files against both `f2bb1d61` and documentation checkpoint `177f92cd`. The documentation commit did not replace the frozen test inputs or rebuild the application.

## Original full browser attempt

The original, uninstrumented harness requested both rendering modes and all scenarios (`full: true`). Its overall result was **failed**.

| Mode | Observed result | Shared native captures |
| --- | --- | --- |
| Software2D | **141 checks passed** | All **46** captures passed on their first attempt |
| WebGL2 | **21 checks passed**, then native variable authoring timed out while waiting for alias 102 to disappear | All **6** captures reached before the failure passed on their first attempt |

The [failed-run summary](failed-full-summary.json) indexes the unchanged report. [failed-full-browser-report.zip](failed-full-browser-report.zip) preserves the original report, browser log, input/source manifests, failure document, selected trace JPEGs and diagnostic records, with a per-member SHA-256 manifest. It is a selected evidence archive: the original large Playwright trace ZIP is not included. The extracted JPEGs and derived timing records identify their original trace events. The separate 15-second failure screenshot also timed out, so there is no failure PNG to present as a successful capture.

Neither mode reported a page exception. The Software2D report retains one resource-404 console message; the WebGL2 report has no console errors. A completed Software2D mode within this failed run is not presented as a successful full two-mode gate.

### What the alias-deletion trace establishes

The failing assertion was `!doc.variables[102]` in `authoring-browser.mjs`, after one native **Delete variable** click. Immediately before the click, the dialog showed **Native alias**, its error text was hidden, and the enabled, arranged Delete button contained the actual pointer coordinates. Its native scene had already advanced after selecting that variable.

The [diagnosis](failed-authoring-diagnosis.json) retains these observations:

- The two animation-frame callbacks following the click took **2.127 seconds**.
- The configured **10-second** document predicate wait returned its timeout after **34.294 seconds** of trace time.
- A trace image received about **391 ms after the wait began** already showed two remaining variables, **Accent** and **Alias**, with the dialog editing Accent. Its mapped swap timestamp preceded the wait's start. The native handler commits deletion before rebuilding that dialog, so this rendered state supports successful deletion before the nominal deadline.
- The subsequent failure-state evaluation took **118.002 seconds**. Its exported document confirmed alias 102 absent, the selected layer's binding cleared, revision **17 → 18**, and history **9 → 10**.

The original trace did not record the predicate's invocation times or returned values. These observations establish substantial scheduling or protocol delays, but do not identify the responsible process or prove a specific application, browser or transport cause. Playwright invokes the first predicate immediately when its evaluation reaches the page, before scheduling later animation-frame polls. Changing only the polling mode was therefore not treated as a demonstrated correction.

## Focused diagnostic with read-only instrumentation

The subsequent diagnostic ran **only WebGL2 native-authoring plus the harness's common gates**, with `full: false`. It passed **21 checks**, including both native-authoring cases, in approximately **175 seconds**. All **3** shared native captures passed on their first attempt.

This run used a separate copy of the `f2bb1d61` harness. Only that copy of `tests/authoring-browser.mjs` gained timestamp and result logging around the existing document predicates. The application, original harness, single-click input path, **10-second deadline**, default animation-frame polling, document assertions, history assertions and pixel assertions were unchanged. The diagnostic source manifest records the original and instrumented file hashes and confirms every other test hash remained equal.

[focused-authoring-summary.json](focused-authoring-summary.json) records all nine document waits, and the [terminal summary](focused-authoring-terminal-summary.json) records the successful exit and exact diagnostic scope. Each predicate ran once and returned true on its first evaluation. For the alias deletion, the page evaluated the predicate at **15:24:44.418–.419 UTC**; the complete Node-side wait took **6 ms**. The intended deletion, binding removal and exact undo/redo assertions all passed. The report retains one resource-404 console message and no page exceptions.

[focused-authoring-diagnostic.zip](focused-authoring-diagnostic.zip) contains the raw report, browser log, predicate timestamps, resource samples, source manifest, instrumentation diff, runner/terminal records, capture diagnostics and the actual variable-dialog PNG. Its per-member manifest identifies the archived bytes. The original large focused trace ZIP is not included. The added logging can itself affect scheduling. This successful diagnostic supports the tested behavior under those conditions; it does not explain the earlier stall or establish complete browser success. No application or published test change was made in response to this failure.

## Resource observations and their limits

[resource-pressure-observations.json](resource-pressure-observations.json) records cgroup snapshots during cleanup after the failed full attempt. The shared memory limit was **8,589,934,592 bytes**. Between **15:16:41 and 15:17:47 UTC**, cumulative `oom` increased from **16 to 18** and `oom_kill` from **11 to 12**. That delta establishes new events in that interval, but identifies neither the killed process nor the cause of the earlier predicate timeout.

The focused diagnostic collected 37 resource samples. Its observed cgroup memory peak was **8,437,272,576 bytes**; peak summed RSS for PID/path-matched processes was **1,556,766,720 bytes**. The cgroup `max`, `oom` and `oom_kill` counters did not change during that diagnostic. Cgroup memory includes cache and other processes, while summed process RSS can count shared pages more than once. These figures are not a measurement of the editor's exclusive heap or a demonstrated explanation for the failed full run.

[resource-sampler-audit.json](resource-sampler-audit.json) examines the second full attempt's saved sampler and browser lifecycle. It matches the launched Node PID or the recovered Chromium directory in visible process command lines; it does not traverse descendants or establish exclusive ownership. Matching reparented helpers are included. Nonmatching, unreadable or namespace-invisible processes may be missed, and another invocation using the same directory may be included. No complete process inventory, PSS or per-PID cgroup breakdown was recorded; skipped reads were not counted. The two modes share one browser/GPU process while creating and closing separate contexts, and the saved renderer PIDs change between modes. The final sample has no matched processes but still reports substantial cgroup anonymous memory. Neither this discrepancy nor later cleanup identifies an owner, an editor leak or the timeout cause.

## Exact-source native CI

The native job at source `f2bb1d61` completed successfully at **15:05:07 UTC** in [FerroDesign run 39799180117 / job 119894677519](https://github.com/wieslawsoltes/FerroUI/actions/runs/39799180117/job/119894677519).

| Native gate | Completed result |
| --- | --- |
| Document core | **394** passed; no failed or ignored tests |
| Native application | **155** passed; no failed or ignored tests |
| Combined core/application count | **549** Rust tests passed |
| Persistence host | **19** passed; no failures, cancellations or skipped tests |
| Active renderer graphs | Vello present and Skia absent for Emscripten, Linux x64, Windows x64, macOS ARM64 and macOS x64 |

[native-ci-summary.json](native-ci-summary.json) records checkout identity, gate excerpts, job status and log digest. [native-ci-log.zip](native-ci-log.zip) preserves the decoded native job log and its summary. This ZIP packages connector-returned log text; it is not the original uploaded workflow artifact. The job also completed the public text-input, compositing, geometry and outline API gates recorded in that log.

These native results belong to the exact `f2bb1d61` checkout. They do not certify a complete browser run or native FerroDesign GUI execution on every dependency-audited platform. Earlier 14-job framework successes retain their own source identities in the [preceding evidence index](../2026-10-22-f36dabf7/README.md).

### Native release measurements

The same successful native job produced the current release benchmark: [schema-5 JSON](native-performance/performance-extracted-log.json), [timestamped supporting step](native-performance/performance-step.log), [extraction provenance](native-performance/provenance.json) and [full measurement summary](native-performance/SUMMARY.md). These files preserve decoded-job-log evidence; they do not claim byte identity with a separately downloaded benchmark artifact.

On 10,000 flat filled rectangles with one selected node, scoped nudge measured **0.019659 ms p50**, compared with **17.284794 ms p50** for the explicit full-document transaction reference, with 48 samples per path in the same release process. The scoped gesture-update distribution measured **0.000751 ms p50** across 288 updates. Document/history, serialization and component ID/history equivalence checks passed. These costs exclude browser input, native control layout, scene drawing, GPU execution and durable-storage completion. Component edits retain full-document history, and the detailed summary reports those production costs separately from isolated no-history operations. The earlier `f36dabf7` benchmark remains preserved under its original source and runner scope.

## Second full browser attempt

The unchanged unfiltered rerun ran from **15:27:24 to 15:39:17 UTC**, using the original, uninstrumented `f2bb1d61` harness and the same recovered `3a0bf1f1` CI runtime. It requested both Software2D and WebGL2 with all scenarios, the original assertions and the existing capture policy. Its overall result was **failed**.

| Mode | Observed result | Shared native captures |
| --- | --- | --- |
| Software2D | **141 checks and all 28 scenarios passed** | All **46** passed on their first attempt |
| WebGL2 | **27 checks passed**; vector-network, native-authoring and typography completed, then component authoring failed at the `ComponentValue2` readiness check | All **7** reached before the failure passed on their first attempt |

[failed-second-full-summary.json](failed-second-full-summary.json) preserves the terminal result and integrity/resource observations. [failed-second-full-browser-report.zip](failed-second-full-browser-report.zip) preserves the report and supporting evidence. All **53** shared native captures passed on their first attempt. Neither mode reported a page error; Software2D retained one resource-404 console message, and WebGL2 had no console errors. Post-run hashes confirm that all **31 source files and 16 runtime files** remained unchanged. The second failure's cause is under investigation; no application or test correction is attributed to this result.

Resource sampling was separate from the original test inputs. Across 143 samples at five-second intervals, the shared cgroup reached its **8,589,934,592-byte** memory limit. `memory.events.max` increased by **1,631,582**, while `oom` and `oom_kill` had zero delta. Peak summed RSS for PID/path-matched processes was **2,373,701,632 bytes**. These shared-cgroup and summed-RSS observations have the attribution limits described above and do not establish the cause of the readiness failure.

The independent exact-`f2bb1d61` CI browser job later completed successfully with its own build, as recorded in [the complete CI section](#exact-source-complete-browser-ci). This local failure remains preserved under its recovered-runtime identity. The complete later-harness result is still separate; the exact-`e0d3cee3` workflow was in progress at **16:50:41 UTC**.

## WebGL2-only attempt with all scenarios

The original, uninstrumented `f2bb1d61` harness then ran in a fresh Chromium process with **WebGL2 only and all 28 scenarios requested**. Because Software2D was not requested, its `scope.full` is **false**. The run **failed after 82 checks and 16 completed scenarios**; matching was the 17th scenario entered. The first matching dialog and Same fill case succeeded, but the second dialog did not open after one `SelectMatchingButton` click, so `SelectSameStroke` never became ready. No stroke-row click was reached.

All **29** shared native captures passed on their first attempt. The report has no page errors and one resource-404 console message. All **31 source files and 16 runtime files** remained unchanged. [failed-webgl2-all-scenarios-summary.json](failed-webgl2-all-scenarios-summary.json) records this terminal result. [failed-webgl2-all-scenarios-report.zip](failed-webgl2-all-scenarios-report.zip) preserves the raw report/log, original matching test, failure document/PNG, manifests, resource samples, five selected trace JPEGs and derived timing/resource records. Its per-member manifest identifies the bytes. This selected archive omits the large original Playwright trace ZIP.

### Matching publication evidence and its limits

[failed-matching-diagnosis.json](failed-matching-diagnosis.json) records the exact event and image observations. After Escape, live state reported empty selection at frame/scene **6/8** without a separate native publication barrier. The next canvas click changed live state to selection `[2]`, frame/scene **7/9** and valid visible matching-button bounds; both existing waits returned immediately.

| Trace event | Trace monotonic time | Observed content |
| --- | ---: | --- |
| Source canvas click | 1,264,401.413–1,264,415.912 ms | One real canvas click |
| Presented image swap | 1,264,418.613 ms | Empty selection and page inspector |
| Matching-opener click | 1,264,521.535–1,264,525.986 ms | One click inside the live button bounds at `(1373.5, 188)` |
| Later presented image swap | 1,264,935.260 ms | Selected source and Rectangle inspector, with Select same visible |

The modal remained hidden. The configured 10-second row-readiness wait timed out after **10.005 seconds**; surrounding state evaluations took **8–12 ms**, and the final failure PNG succeeded. Cgroup memory near this event was **4.18–4.27 GB**, with stable `max`, `oom` and `oom_kill` counters. The larger whole-run memory peak does not explain this failure.

The trace supports an unseparated Escape publication transition that can satisfy the next selection's frame/scene floors. The scalar scene notification is coalesced and does not identify a control's layout generation. Presentation timing corroborates the boundary but does not expose the actual native hit-test target, capture owner, button Click event or callback. This evidence does not prove a particular application bug or the complete event-consumption path.

## Corrected matching test: focused validation passed

Published checkpoint `f8e0471e` changes only `tests/matching-browser.mjs`. For a nonempty source selection, the test snapshots state before Escape, requires exact empty selection plus a newer native frame and scene, and checks the page/layer/action visibility before the existing canvas click. It omits a redundant Escape when selection is already empty. After one matching-opener click, the test explicitly requires a visible modal before the unchanged named-row readiness and input. It retains exact selection, dialog closure, authored-document, history, redo and revision assertions, with the same **10-second deadlines**, no action retry and no additional click or runtime change.

A separate **both-mode matching-plus-common-gates** run **passed**, using the corrected published harness and the same recovered CI runtime. Its [terminal summary](../2026-10-22-f8e0471e/matching-focused-summary.json) records **25 Software2D / 22 WebGL2 checks**, all three matching cases in each mode, and **180.13 seconds** total duration. All **four** shared native captures passed on their first attempt. Both modes reported no page errors; Software2D retained one resource-404 console message and WebGL2 had no console errors. The run verified all **31 source files and 16 runtime files unchanged**. Its [raw evidence archive](../2026-10-22-f8e0471e/matching-focused-report.zip) belongs to the corrected harness and does not overwrite the original failed evidence here.

This is `scope.full: false` and validates only its requested matching/common gates. The [corrected-source evidence index](../2026-10-22-f8e0471e/README.md) preserves this pass separately. The later attempt covering the 11 remaining WebGL2 feature scenarios was blocked before Chromium: its local HTTP server listen on `127.0.0.1` returned `EPERM`. The [launch summary](../2026-10-22-f8e0471e/remaining-webgl2-launch-summary.json) records zero checks, zero entered scenarios and no browser report, with all 31 source and 16 runtime files unchanged; the [launch evidence ZIP](../2026-10-22-f8e0471e/remaining-webgl2-launch-block.zip) preserves its raw records. No escalation or further attempt followed that denial. The eleven scenarios remain outstanding for that `f8e0471e` local attempt; the independent complete `f2bb1d61` CI run passed them with the earlier harness. This environment block is not an application-test failure. Neither the focused pass nor a combination of separately executed subsets constitutes a complete two-mode suite pass.

## Source-delivery audit

[source-delivery-audit.json](source-delivery-audit.json) records a read-only inspection at source `b83deee6`. All **31 manifests and 151 path-dependency edges** in its conservative all-target/optional/development closure were tracked and inside the repository. Required XAML, font/theme assets, browser bridge/loader inputs and lockfiles were present; no gitlinks, LFS pointers or archive-exclusion rules were found, and all 96 symlinks resolved. This inventory is separate from the active renderer-graph audit.

The full repository layout is required; a sample-only archive is insufficient. Inspected build scripts do not require FerroUI's `.git` directory, though setup needs Git for the external Emscripten clone and still downloads toolchains and third-party packages. The macOS native build falls back to the tracked native source directory if an extractor flattens its symlink. The pinned public ZIP URL returned HTTP 200, but no ZIP body, CRC, extraction or build from that archive was verified. Existing CI build results retain their own provenance.

## Captures and interpretation

The standalone [Software2D initial shell](Software2D-initial.png), [WebGL2 initial shell](WebGL2-initial.png), [Software2D stacked guides](Software2D-stacked-layout-guides.png), [Software2D matching controls](Software2D-native-matching-fill.png) and [Software2D bucket preview](Software2D-bucket-hover-face.png) come from the original full attempt recorded above. They use the recovered CI runtime and are not labeled as images from the second attempt.

These captures show this application's rendered UI. No matched Figma document/reference screenshot was available for a numerical visual-parity comparison. SwiftShader execution is not physical-GPU throughput evidence, and the diagnostic's elapsed times are not frame-rate measurements.
