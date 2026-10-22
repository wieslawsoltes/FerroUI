# Exact-source CI at `124ffdb7`: core and framework passes, application builds blocked

Source: [`124ffdb74e2b39ca09d5a1bcd4052bbb9a61a165`](https://github.com/wieslawsoltes/FerroUI/commit/124ffdb74e2b39ca09d5a1bcd4052bbb9a61a165).

**The complete workflow and independent native feedback workflow both failed.** The core passed all **498 tests**. The selected framework and host checks below also passed, but both native application compilation and the WebAssembly application build stopped at **E0282**, `canvas/corner_radius.rs:206`. **Chromium did not run.** This evidence does not validate the newly added application features or resolve the earlier smooth-corner export pixel failure.

## Exact workflows

| Workflow / job | Completed result |
| --- | --- |
| [Complete run 39858027509 / native job 120102469680](https://github.com/wieslawsoltes/FerroUI/actions/runs/39858027509/job/120102469680) | Failed during application `cargo check --lib --tests`; preceding gates below passed. The application test step was skipped |
| [Complete run 39858027509 / browser job 120102469519](https://github.com/wieslawsoltes/FerroUI/actions/runs/39858027509/job/120102469519) | Failed during the actual WASM build. Chromium installation, IndexedDB validation, editor scenarios and text-pan measurements were skipped |
| [Native feedback run 39858027415 / job 120106042788](https://github.com/wieslawsoltes/FerroUI/actions/runs/39858027415/job/120106042788) | All configured framework test filters passed; application test compilation failed before the application tests could execute |

[workflow-api.json](workflow-api.json) preserves the retrieved run identity/status fields and complete returned job/step summaries. [summary.json](summary.json) records the exact log-derived counts, line references and scope.

## Completed gates

The following results belong to [native-job.log](native-job.log), not to a successful application run:

| Gate | Result | Result line |
| --- | ---: | ---: |
| Document engine, unfiltered | **498 passed**, 0 failed, 0 filtered; **2.07 s** test execution | 1062 |
| Persistence host tests | 19 passed | 389–392 |
| Input-observation host tests | 4 passed | 423–426 |
| Trace analysis and mocked teardown tests | 13 passed | 511–514 |
| Controls preedit / rich ranges / layout invalidation | 15 / 2 / 1 passed | 2160 / 2442 / 2723 |
| Base / Vello compositing-layer filters | 7 / 8 passed | 3604 / 4051 |
| Vello geometry path / shaped text outline / complete glyph outline | 2 / 3 / 2 passed | 4423 / 4782 / 5140 |
| Vello unhinted glyph geometry and pixels | 8 passed | 5504 |

The source syntax and metadata step completed. All **five active dependency graphs** contained Vello and excluded Skia: Emscripten, Linux x64, Windows x64, macOS ARM64 and macOS x64 (lines 1309, 1310, 1343, 1360 and 1361). These are dependency checks, not execution of every desktop GUI.

The release document-engine performance example completed for **10,000 nodes** and printed its JSON at lines **1394–1707**. That benchmark excludes browser rendering and GPU work; no application-speed or browser-performance conclusion is derived here. The 13 trace-related host tests exercise synthetic analysis and mocked cleanup, not an actual Chromium trace.

The independent [native-feedback-job.log](native-feedback-job.log) additionally records:

| Framework filter | Result | Result line |
| --- | ---: | ---: |
| Vello `unhinted_` | 8 passed, 0.34 s | 696 |
| Base `paragraph` | 15 passed, 0.03 s | 1594 |
| Controls `paragraph` | 16 passed, 0.17 s | 1892 |
| Controls `text_box_tests::` | 174 passed, 2.55 s | 2346 |
| Controls `text_box_history::` | 5 passed, 0.06 s | 2631 |

These filtered suites can overlap; their counts are not a combined unique-test total.

## Build failure and subsequent correction

All three build attempts report the same missing type annotation for the collected `handles` value before `handles.clone()`: full native lines **6162–6178**, browser lines **895–911**, and native feedback lines **3060–3076**. The full native and feedback jobs did not execute the application test suite.

The browser build's preceding Node text-input tests passed **10/10** (browser log lines 474–477). These are host tests, not browser scenarios. The build upload step reported that `dist` contained no files (line 929); its successful step status is not proof of a complete browser artifact. There is no editor report, capture count, renderer result or text-pan measurement for this source.

Published follow-up [`5cb7e884`](https://github.com/wieslawsoltes/FerroUI/commit/5cb7e8847746a2bfae45bf46cafed40889f1affb) adds an explicit handle-vector type and the bounded corner-coverage diagnostics. Its validation is separate; no later pass is claimed by these records.

## Cache investigation and preserved provenance

[NATIVE_CACHE.md](NATIVE_CACHE.md) records the successful cache restore, the source comparison and the limits of the emitted Cargo diagnostics. A separate dependency-free probe confirms that the configured logging target was outdated; the correct pinned-Cargo target is `cargo::compiler::fingerprint=trace`. The CI log does **not** establish the reason for invalidation.

The logs are preserved decoded UTF-8 GitHub job logs, including their original timestamp prefixes. The preserved full-workflow logs were not modified. Their checkout lines are native **130**, browser **130**, and feedback **129**.

| File | Bytes | SHA-256 |
| --- | ---: | --- |
| [native-job.log](native-job.log) | 551,154 | `a3195f0b6c5e06736e07aa627db3578b4d38e2bd009a8b0dc6c4aae30f4ca35f` |
| [browser-job.log](browser-job.log) | 85,862 | `31e8d354db984ab7bf0b2f5da438e4e4dc4396293026c1e66e3b1f7b807b46a1` |
| [native-feedback-job.log](native-feedback-job.log) | 287,830 | `8626a3b7c648baf802998933285a333614317ab08075c53dbe68c0dc991aa617` |

These hashes describe the saved text files. No ZIP, runtime binary or raw browser artifact was retrieved. All application validation results above come from CI; the separate local logging-filter probe contains no FerroUI dependency or application test.
