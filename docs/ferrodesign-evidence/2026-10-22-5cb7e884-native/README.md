# Native validation at `5cb7e884`

Exact source **`5cb7e8847746a2bfae45bf46cafed40889f1affb`** completed the [native feedback workflow](https://github.com/wieslawsoltes/FerroUI/actions/runs/39862047515/job/120114911173) successfully. The terminal job status was observed at **2026-10-22 22:16:04 UTC**. [The decoded job log](native-job.log) and [terminal jobs API response](jobs-api.json) preserve the evidence; [summary.json](summary.json) records counts, extraction lines and hashes.

| Gate | Result | Decoded log line |
| --- | --- | ---: |
| Public unhinted Vello glyph geometry/pixels | 8 passed; 0 failed; 0.34 s | 698 |
| Base paragraph layout | 15 passed; 0 failed; 0.03 s | 1596 |
| Controls paragraph layout/editing | 16 passed; 0 failed; 0.17 s | 1894 |
| Complete configured TextBox filter | 174 passed; 0 failed; 2.70 s | 2348 |
| Native rich-history filter | 5 passed; 0 failed; 0.07 s | 2633 |
| Complete application library suite | **217 passed; 0 failed; 0 ignored; 0 filtered out; 25.49 s** | 3296 |

Checkout line 129 identifies the exact source. The paragraph, TextBox and rich-history filters overlap and must not be added as unique test counts. Test-suite durations measure these test executions, not editor frame rate or user interaction latency.

## Radius handles and independent coverage

All eight new native radius-handle tests passed at lines 3078–3085. They exercise captured fractional gestures, exactly-once history, no-op metadata/redo restoration, Escape/Undo/capture-loss ownership, save lifecycle finalization, affine/reflected coordinates, distinct maximum-radius corner markers, native adorner pixels and selection/resize precedence.

The new `native_smoothed_export_matches_independent_dense_curve_coverage_after_history` test passed at line 3148. It uses the browser comparison's 200 × 140, radius-40, smoothing-0.6 fixture. It verifies exact native PNG alpha round-tripping and fresh-versus-retained render equality through changes, Undo and Redo. Its independent reference samples the cubic Bernstein equations and clips their polygon to pixel areas; it does not use the renderer's curve flattening. The 256-to-512-samples-per-cubic convergence check and the existing alpha-difference criterion both passed.

The explicit `Vec<Handle>` type at this source resolves the preceding E0282 compilation failure: the complete application suite now compiles and executes. The text-history and template-lifecycle corrections also pass all configured native filters in the same successful job.

## Second native pass in the full workflow

At **2026-10-22 22:28:16 UTC**, the separate full workflow's [Native validation job 120117259866](https://github.com/wieslawsoltes/FerroUI/actions/runs/39862047469/job/120117259866) was **completed / success** at the same exact `5cb7e8847746a2bfae45bf46cafed40889f1affb` checkout. The post-checkout commit is printed at line **130** of [full-native-job.log](full-native-job.log); [full-jobs-api.json](full-jobs-api.json) preserves its terminal step statuses and the browser sibling's separate state.

| Full-workflow native gate | Result | Decoded full-native log line |
| --- | --- | ---: |
| Complete document/editor core suite | **498 passed; 0 failed; 0 ignored; 0 filtered out; 2.10 s** | 1029 |
| Core documentation tests | 0 tests; successful | 1035 |
| Persistence host tests | 19 passed; 0 failed | 395–402 |
| Input-observation pure tests | 4 passed; 0 failed | 429–436 |
| Trace analysis and mock teardown tests | 13 passed; 0 failed | 517–524 |
| Controls `preedit` filter | 15 passed; 0 failed; 0.13 s | 1816 |
| Controls `rich_ranges` filter | 2 passed; 0 failed; 0.01 s | 2098 |
| Range-style/font-feature layout invalidation | 1 passed; 0 failed; 0.01 s | 2379 |
| Base compositing-layer recording | 7 passed; 0 failed; 0.00 s | 3260 |
| Vello compositing-layer pixels | 8 passed; 0 failed; 0.10 s | 3632 |
| Public Vello geometry path data | 2 passed; 0 failed; 0.00 s | 4004 |
| Public text-layout outline geometry | 3 passed; 0 failed; 0.69 s | 4363 |
| Complete glyph geometry | 2 passed; 0 failed; 0.02 s | 4721 |
| Unhinted Vello glyph geometry/pixels | 8 passed; 0 failed; 0.28 s | 5085 |
| Complete application library suite | **217 passed; 0 failed; 0 ignored; 0 filtered out; 20.21 s** | 6343 |

The native launcher/shared UI compilation gate also completed successfully. The test-name-filtered framework rows are the configured subsets, not the full framework suite; overlapping filters and repeated application runs must not be added as unique coverage. The host, input-observation and mocked trace tests do not execute Chromium, IndexedDB in Chromium or actual CDP tracing.

Lines **1048–1052** verify Vello with no Skia in all five resolved application dependency graphs: `wasm32-unknown-emscripten`, `x86_64-unknown-linux-gnu`, `x86_64-pc-windows-msvc`, `aarch64-apple-darwin` and `x86_64-apple-darwin`. These are Cargo normal/build dependency-graph checks, not five platform executions. The native 10,000-node release performance example also completed; this record makes no browser, frame-rate, GPU or speedup inference from it.

At that same observation, browser job **120117259515** was still **in_progress**, with **Build actual FerroUI Vello WebAssembly application** active and the Chromium test steps pending. The successful native sibling is **not an overall full-workflow or browser pass**.

| Additional preserved file | UTF-8 bytes | SHA-256 |
| --- | ---: | --- |
| [full-native-job.log](full-native-job.log) | 588,129 | `593573b673306af10cc4f80dc34f5f67200c405d44435f69b1879035b58b98da` |
| [full-jobs-api.json](full-jobs-api.json) | 7,399 | `cb1c664f06a6ae35235e3519d43bdea1ddd74e715b03c074eefcc2f8553b8b40` |

## Boundary

This native result does not resolve the earlier browser SVG/PNG mismatch, establish numerical equality to Figma, or validate later gradient/padding work. The [separate full workflow](https://github.com/wieslawsoltes/FerroUI/actions/runs/39862047469) had no completed browser result in either preserved observation. Its bounded alpha diagnostics and unchanged silhouette assertion require their own completed browser evidence. Both requested browser surfaces use the same CPU offscreen export renderer.

The saved files are the connector's decoded UTF-8 job log and structured API response. The log's original BOM and final newline are retained. No official artifact ZIP, raw browser report or screenshot bytes were retrieved for this record.
