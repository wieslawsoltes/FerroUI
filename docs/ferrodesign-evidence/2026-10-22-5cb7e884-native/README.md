# Native feedback at `5cb7e884`

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

## Boundary

This native result does not resolve the earlier browser SVG/PNG mismatch, establish numerical equality to Figma, or validate later gradient/padding work. The [separate full browser workflow](https://github.com/wieslawsoltes/FerroUI/actions/runs/39862047469) was still running at the observation above. Its bounded alpha diagnostics and unchanged silhouette assertion require their own completed browser evidence. Both requested browser surfaces use the same CPU offscreen export renderer.

The saved files are the connector's decoded UTF-8 job log and structured API response. The log's original BOM and final newline are retained. No official artifact ZIP, raw browser report or screenshot bytes were retrieved for this record.
