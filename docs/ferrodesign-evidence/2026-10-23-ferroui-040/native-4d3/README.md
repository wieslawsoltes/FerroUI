# Native validation of the first-preedit patch at 4d3

**The native feedback workflow failed on three new regression tests.** Compilation succeeded, all 13 newly added regression functions executed, and the main styled Unicode first-clause/cancel/commit/history regression passed. The independent GPU and application stages continued after the text-stage failure and passed separately. These results belong to source commit [`4d3c796c7d5b76eb1e9d7f4f24b523841adbd994`](https://github.com/wieslawsoltes/FerroUI/commit/4d3c796c7d5b76eb1e9d7f4f24b523841adbd994), tree `f5870c7d41ef51600b068829a3c40838e701a8ef`.

The source-reviewed correction is a distinct commit, [`dc36199693472ac8297bafdaad6d4a0701acddf0`](https://github.com/wieslawsoltes/FerroUI/commit/dc36199693472ac8297bafdaad6d4a0701acddf0). This folder contains no passing CI or browser claim for that correction. The [matching 4d3 browser build evidence](../browser-build-4d3/) documents compilation and payload verification only; that local browser build was not launched.

## Executed native suites

[Independent native run 39972889213](https://github.com/wieslawsoltes/FerroUI/actions/runs/39972889213), job `120479903485`, attempt 1, checked out the exact 4d3 SHA. It completed with conclusion `failure` after preserving the partial build cache. Pinned Rust was `1.99.0`, tests ran with one test thread, and the required GPU stage had `FERROUI_VELLO_REQUIRE_GPU=1`.

| Executed suite/filter | Passed | Failed | Ignored | Test duration |
| --- | ---: | ---: | ---: | ---: |
| Vello public `unhinted_` glyph checks | 8 | 0 | 0 | 0.33 s |
| Base `paragraph` | 17 | 0 | 0 | 0.04 s |
| Base `text_line_bounds` | 8 | 0 | 0 | 0.02 s |
| Controls `paragraph` | 17 | 0 | 0 | 0.18 s |
| Controls `text_box_tests::` substring filter | 184 | 3 | 0 | 2.75 s |
| Controls `text_box_history::` | 5 | 0 | 0 | 0.06 s |
| Vello `gpu::fragment_tests::` with GPU required | 7 | 0 | 0 | 9.95 s |
| FerroDesign application library | 239 | 0 | 0 | 42.94 s |

The TextBox substring filter selected **132 TextBox cases and 55 masked TextBox cases**, because the latter module name also contains `text_box_tests::`. All masked cases passed; 129 TextBox cases passed and three failed. The parser accounts for all 187 case lines, including four `should panic` cases. Filtered suites overlap: the main new paragraph/preedit regression appears both in the paragraph filter and in the complete TextBox filter. The table does not represent a count of distinct tests across all rows.

All 13 new function names appear in the actual libtest log. Ten distinct new names passed and three failed. [analysis.json](analysis.json) records each occurrence, outcome, and source-log line; [full-log-analysis.json.gz](full-log-analysis.json.gz) retains every parsed case in all eight suites.

## The three failures and their correction scope

| New regression | Observed failure at the 4d3 source location | Supported diagnosis and followup |
| --- | --- | --- |
| `first_preedit_accepts_native_caret_coercion_and_reversed_selection` | `text_box_tests.rs:3365`: presenter preedit was `None`, expected `Some("first")`. | Source review found that TextBox coerced its caret while the TwoWay-bound presenter still contained the old Unicode text. A position valid in provisional `az` could be normalized against the old surrogate pair and echoed back into TextBox. The test's caret and first-clause expectations remain unchanged. |
| `first_preedit_respects_reentrant_host_selection_before_publication_and_clear` | `text_box_tests.rs:3268`: caret/selection was `(1, 1, 1)`, expected `(2, 2, 2)`. | The same stale presenter text could normalize a host callback's chosen caret 2 to 1. The correction synchronizes the connected presenter Text before provisional caret coercion or TextChanging, only while first-clause preparation owns the update. It guards the preparation token, parent, presenter, and text generation against reentry. |
| `first_preedit_respects_reentrant_template_replacement_before_publication` | `text_box_tests.rs:3051`: visual descendant lookup returned `None`. | Applying the outer TextBox template installs the named native presenter immediately, but its nested ScrollViewer content joins the visual tree during layout. The fixture now asserts restored text/selection and the new native owner synchronously through `text_view_visual`, then checks that visual lookup matches that same owner after a normal root layout pass. No production template-lifecycle change was made. |

The assertion values above are directly recorded in the retained logs. The notification-order explanation comes from the exact source and the supported TwoWay test template; the failed run did not record a per-property event trace. The old coercion assertion also lacked an iteration label. The followup preserves its original expected values and adds that diagnostic label.

The dc361 followup adds `first_preedit_text_synchronization_respects_reentrant_owner_changes`, with five cases for cancellation, host Text replacement, host caret selection, template replacement, and a newer clause arriving during the earlier presenter Text notification. This brings the newly introduced regression functions to 14. All three corrected files passed rustfmt syntax parsing and `git diff --check` locally; no local native compilation or browser execution was performed. The exact corrected file hashes are recorded in `analysis.json` so source review cannot be confused with execution of the older 4d3 files.

## Independent corroborating jobs

The [full workflow's native job](https://github.com/wieslawsoltes/FerroUI/actions/runs/39972889324/job/120481660281) compiled the same source and ran the `preedit` filter: **25 passed, 3 failed, 0 ignored**. Its failures were exactly the same three cases. That job also recorded 533 core tests passing, then stopped after the failed preedit stage. Its log is retained separately; the application and GPU successes in the table come from the independent native feedback job, which continued those stages.

The [generated-reference run 39972889281](https://github.com/wieslawsoltes/FerroUI/actions/runs/39972889281), job `120479904215`, completed successfully at the same SHA. It regenerated registered Rust paths, Fluent and Simple compiled XAML/runtime metadata, and checked the runtime emitter and build-script references. This source-compatibility result is separate from the failed native test conclusion.

## Retained evidence and reproduction

[metadata.json](metadata.json) contains final run/job identities, source SHA, timing, and every step outcome. [source-provenance.json](source-provenance.json) is the unchanged initial source inventory: its `pending` field describes that earlier capture, not the final result. It records the exact Git blob hashes and original 13 test names. [compressed-provenance.json](compressed-provenance.json) gives compressed and uncompressed byte counts and SHA-256 values for each losslessly compressed log or analysis file.

The primary raw native job log contains 677,027 bytes, SHA-256 `68b6152de0729797d508cd5a5a3d85daf70d94d3fd5bd0e4e232ccd4b34514ef`. Original GitHub timestamps and ANSI escapes are retained. The other full native and generated-reference logs are compressed independently. No screenshots, traces, native binaries, or build caches are included.

To reproduce the complete case inventory in a temporary directory from this folder:

```bash
evidence_work=$(mktemp -d)
gzip -dc native-job.log.gz > "$evidence_work/job.log"
cp source-provenance.json analyze_log.py "$evidence_work/"
python3 "$evidence_work/analyze_log.py"
```

The generated `log-analysis.json` accounts for all announced cases and their result counts. It distinguishes a completed failing suite from compilation failure, and preserves both appearances of the paragraph/preedit regression. [manifest.json](manifest.json) hashes every retained file in this folder except the manifest itself.
