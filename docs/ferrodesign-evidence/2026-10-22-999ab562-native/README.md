# Native validation and cache diagnostics at `999ab562`

The completed [native feedback run 39863039315, job 120120323807](https://github.com/wieslawsoltes/FerroUI/actions/runs/39863039315/job/120120323807) **passed** at exact source `999ab562276fc45c1757aee304330f9571e021f3`. Checkout is printed in [the decoded log](native-feedback-job.log) at lines **36, 108–129**; the job API reports every step successful in [jobs-api.json](jobs-api.json). The log spans **22:16:23–22:29:02 UTC on 2026-10-22**. This source predates the gradient-handle and padding feature work.

This directory preserves the ordinary GitHub connector's completed, decoded UTF-8 job log, including its original BOM and ANSI sequences. It does not contain an independently downloaded log archive, cache archive, browser artifact or browser result. [summary.json](summary.json) is a bounded extraction with exact source lines and cache reasons; the raw log remains authoritative.

## Test results

| Native command scope | Passed | Failed / ignored | Test time | Raw log line |
| --- | ---: | --- | ---: | ---: |
| Vello `unhinted_` geometry and pixels | 8 | 0 / 0 | 0.34 s | [1411](native-feedback-job.log#L1411) |
| Base `paragraph` | 15 | 0 / 0 | 0.03 s | [2309](native-feedback-job.log#L2309) |
| Controls `paragraph` | 16 | 0 / 0 | 0.17 s | [2607](native-feedback-job.log#L2607) |
| Controls `text_box_tests::` | 174 | 0 / 0 | 2.70 s | [3061](native-feedback-job.log#L3061) |
| Controls `text_box_history::` | 5 | 0 / 0 | 0.06 s | [3346](native-feedback-job.log#L3346) |
| FerroDesign application complete library suite | 217 | 0 / 0 | 25.84 s | [4009](native-feedback-job.log#L4009) |

Framework filters overlap, so these counts must not be summed as unique tests. The application result has zero filtered tests. The previously failing template-removal/redo regression passes at **2605** and **2995**, the hosted rich-history regression passes at **3341**, and all eight native corner-radius cases pass at **3791–3798**. This job did not run the complete framework suite, a separate core suite, browser builds or browser interactions.

## Proven cache invalidation cause

The workflow restored **913,900,044 bytes** from the cache key ending in `5cb7e8847746a2bfae45bf46cafed40889f1affb`; the fallback hit and successful extraction are recorded at **165–177**. It later saved the `999ab562` cache at **4036**. The requested and restored keys have the same dependency-hash prefix.

GitHub's [exact commit comparison](cache-compare-api.json) reports one intervening commit and 27 changed paths, all documentation/evidence or the native workflow. No production Rust source, Cargo manifest, lockfile or toolchain file changed. [workflow-source-api.json](workflow-source-api.json) retains both exact workflow versions and Git blob IDs: the workflow difference changes only the fingerprint logging target/level and its comment. The commands, profiles, cache paths and pinned Rust version are unchanged.

The corrected `CARGO_LOG=cargo::compiler::fingerprint=trace` appears at **320** and produces actual freshness reasons. Three directly reported workspace inputs have new checkout modification times later than the restored dependency records:

| Unchanged source input | Cached dep-info mtime, UTC | Checked-out input mtime, UTC | Input newer by | Reason line |
| --- | --- | --- | ---: | ---: |
| `src/FerroUI.Build.Scan/scanner/constants.rs` | 22:04:41.305163439 | 22:16:29.360386485 | 708.055 s | [880](native-feedback-job.log#L880) |
| `src/FerroUI.System/component_model/backing_field.rs` | 22:04:55.055210599 | 22:16:29.502799947 | 694.448 s | [891](native-feedback-job.log#L891) |
| `src/FerroUI.Remote.Protocol/default_message_type_resolver.rs` | 22:05:45.565398073 | 22:16:29.497423801 | 643.932 s | [932](native-feedback-job.log#L932) |

All timestamps in this table are from **2026-10-22**. Cargo explicitly labels them `FsStatusOutdated(StaleItem(ChangedFile ...))`. They coincide with the new checkout at **108–129**. The source comparison establishes unchanged content; the trace establishes the modification-time comparison that made those units dirty.

The first invocation reports **15 dirty units across nine workspace packages**: three direct source-mtime reasons and 12 `StaleDepFingerprint` reasons. Examples of propagated invalidation are Vello at **867**, Base at **870**, Controls at **923**, and Fonts.Inter/HarfBuzz/OpenGL at **935, 938, 948**. Nine workspace packages then compile at **1022–1346**. Registry dependencies remain usable in concrete examples: `quote` at **346** and `syn` at **361** report `filesystem up-to-date`.

These are reported triggering inputs, not an exhaustive list of all files with new mtimes; Cargo can stop at an already stale input or dependency. The evidence resolves the earlier uncertainty in [the `124ffdb7` cache investigation](../2026-10-22-124ffdb7-ci/NATIVE_CACHE.md). It does not show cache corruption, a toolchain mismatch or a correctness defect in Cargo.

## Limits and a bounded optimization candidate

Only the first Vello invocation is fingerprint-traced. Its build takes **2m 09s** at **1398**, before its 0.34-second test run. Later Base, Controls and application commands report different build contexts and times; their repeated package names cannot be assigned the same cause from this trace alone. Subsequent Controls filters reuse their test executable with **0.11-second** build phases at **2882** and **3336**. No exact avoidable-build-time estimate or native/browser runtime improvement has been measured.

A concrete candidate for a separate, fail-closed experiment is a versioned per-file manifest saved **inside the same cached target**: tracked regular-file path, SHA-256, size and actual `mtime_ns`. Restoration would change an input mtime only after verifying the current bytes and size exactly match that manifest. New, changed, untracked, symlinked, unsafe-path and unknown inputs would retain their current timestamps. Cargo would still perform its normal dependency checks; no output, dep-info, fingerprint or Cargo environment override is required.

This candidate is **not integrated or validated by this run**. Before relying on it, tests and an unchanged native CI gate must show that changed/new/deleted files, build-script inputs, generated tracked source, corrupt or mismatched manifests, and input mutation during snapshot/build cannot become falsely fresh. Snapshot identity must stay coupled to the archived target and actual input times. Blindly assigning Git commit times, backdating every source file, or touching compiled outputs is not supported by this evidence. No production or workflow file was modified during this audit.

## File integrity

| File | Bytes | SHA-256 |
| --- | ---: | --- |
| [native-feedback-job.log](native-feedback-job.log) | 618,597 | `f3b288ccf9e2daad288f31c60cdbd61862adc91b9b3409ee073fa1498c26b4b5` |
| [jobs-api.json](jobs-api.json) | 2,514 | `844440c167a3afef98af0178b707209e865c8b421a6cadc31db5379bc9fb523f` |
| [cache-compare-api.json](cache-compare-api.json) | 6,949 | `a7e2971a8d6d5d1ac2cb01259ec905878662747490af46e7af2bff0f8c64a451` |
| [workflow-source-api.json](workflow-source-api.json) | 10,533 | `7e167eb044aa0f2e99d18f6aa7ffa1b8f446afbd785d2045b508c4323dbad815` |
| [summary.json](summary.json) | 14,425 | `90666e93bb026749e1af5c910758a72eafbca4db90d00beaea657be03737f763` |

Hashes cover the saved UTF-8 files, not an unavailable binary archive. The raw log contains **4,052 lines**. The summaries preserve its test deadlines and outcomes without interpreting this native success as a later-source or browser pass.
