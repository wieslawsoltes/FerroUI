# Native validation of the first-preedit correction at dc361

**The exact corrected source passed the independent native feedback workflow.** All 14 added first-preedit regression functions appear explicitly with passing outcomes in the closed libtest log. The full TextBox filter, native rich history, shared paragraph layout, required GPU fragment tests, and all 239 FerroDesign application tests passed. These results belong to source commit [`dc36199693472ac8297bafdaad6d4a0701acddf0`](https://github.com/wieslawsoltes/FerroUI/commit/dc36199693472ac8297bafdaad6d4a0701acddf0), tree `57c2625a948b9401b9517b2724b2715d8e7ca92b`.

The earlier [4d3 native failure](../native-4d3/) remains separately preserved. This folder records execution of its correction; it contains no browser pass, whole-workflow pass, performance comparison, or OS IME candidate-window claim.

## Executed native suites

[Native run 39976024451](https://github.com/wieslawsoltes/FerroUI/actions/runs/39976024451), job `120490704088`, attempt 1, checked out the exact dc361 SHA. Both the final run and job APIs report `success`. The job ran from 15:10:47 to 15:25:30 UTC on 2026-10-23. Pinned Rust was `1.99.0`; Cargo used one build job and the tests used one test thread. Every recorded job step completed successfully, including source timestamp checks and the final cache snapshot/save.

| Executed suite/filter | Passed | Failed | Ignored | Test duration |
| --- | ---: | ---: | ---: | ---: |
| Vello public `unhinted_` glyph checks | 8 | 0 | 0 | 0.26 s |
| Base `paragraph` | 17 | 0 | 0 | 0.04 s |
| Base `text_line_bounds` | 8 | 0 | 0 | 0.02 s |
| Controls `paragraph` | 17 | 0 | 0 | 0.19 s |
| Controls `text_box_tests::` substring filter | 188 | 0 | 0 | 3.10 s |
| Controls `text_box_history::` | 5 | 0 | 0 | 0.06 s |
| Vello `gpu::fragment_tests::` with GPU required | 7 | 0 | 0 | 6.00 s |
| FerroDesign application library | 239 | 0 | 0 | 37.96 s |

These are **overlapping filter counts**, not an additive count of distinct tests. The TextBox substring filter contains 133 `text_box_tests` cases and 55 `masked_text_box_tests` cases, including four expected-panic cases. The Controls paragraph filter shares seven TextBox cases with that filter. The Base paragraph and line-bounds filters share `full_is_the_default_of_paragraph_properties`. All announced cases and all result counts match the parsed log inventory exactly.

The required GPU stage explicitly recorded `FERROUI_VELLO_REQUIRE_GPU=1`; its seven cases executed and none were ignored. The raw log does not identify a physical GPU model, and these results make no hardware-performance claim. Durations in the table are libtest suite durations, excluding compilation and setup.

## All 14 added regressions executed

Every function below passed in `text_box_tests::`. The styled paragraph/history function also passed in the earlier Controls paragraph filter, giving 15 execution occurrences for 14 distinct added functions. [analysis.json](analysis.json) retains each exact log line, outcome, and one-based line number; the complete case inventory is in [full-log-analysis.json.gz](full-log-analysis.json.gz).

```text
paragraph_first_preedit_preserves_unicode_styles_cancel_commit_and_native_history
first_preedit_respects_reentrant_cancellation_before_publication
first_preedit_respects_reentrant_detach_before_publication
first_preedit_respects_reentrant_template_replacement_before_publication
first_preedit_respects_reentrant_host_text_before_publication
first_preedit_keeps_reentrant_host_text_that_returns_to_the_provisional_value
preedit_clear_queues_a_new_clause_until_confirmed_selection_is_restored
first_preedit_coalesces_newer_clauses_during_provisional_text_changing
first_preedit_respects_reentrant_host_selection_before_publication_and_clear
first_preedit_checks_selection_notification_before_publication
first_preedit_accepts_native_caret_coercion_and_reversed_selection
first_preedit_does_not_overwrite_a_newer_clause_or_its_cursor
first_preedit_still_clears_on_external_text_or_caret_changes
first_preedit_text_synchronization_respects_reentrant_owner_changes
```

The final synchronization function exercises five callback cases inside one named test: cancellation, host Text replacement, host caret/selection, template replacement, and a newer clause. It is counted once. Existing first-clause, rich blur, native history and paragraph regressions also executed through the complete filters.

The three failures preserved at 4d3 now pass. The production correction synchronizes the connected presenter Text before provisional caret coercion while first-clause preparation owns the update, then checks its preparation token, parent, presenter and text generation against reentry. This prevents a TwoWay caret binding from resolving the new position against an old Unicode cluster. Original caret and first-clause expectations remain intact. The template-replacement fixture checks the installed native owner synchronously, then confirms that the same presenter becomes a visual descendant after normal layout; no production template-lifecycle change was introduced. The preceding failure folder retains the detailed source diagnosis and exact failed assertions.

## Generated references passed separately

[Generated-reference run 39976024371](https://github.com/wieslawsoltes/FerroUI/actions/runs/39976024371), job `120490704061`, also completed successfully at the exact dc361 SHA. It regenerated registered Rust paths, Fluent and Simple compiled XAML/runtime metadata, and verified the corresponding runtime emitters and build-script references. Its final API metadata and complete job log are retained here. Its generated-reference upload is recorded by the job, but the output artifact itself is outside this evidence folder.

This source-compatibility result is distinct from native test execution and from browser runtime validation. No local native compilation or browser execution was used to produce this evidence.

## Retained evidence and reproduction

[metadata.json](metadata.json) contains the final run/job identities, exact source SHA, timing, and every step outcome. [source-provenance.json](source-provenance.json) preserves the unchanged initial source inventory: its `pending` field describes that initial capture, not the final result. Its six exact Git-object byte hashes, including the three Controls files and native workflow, were checked again during packaging. Final execution status is recorded in `metadata.json` and `analysis.json`.

The primary native job log is preserved losslessly as [native-job.log.gz](native-job.log.gz): **731,629 uncompressed bytes**, SHA-256 `da4f4e961d93edd28eb06e0c6bd2cf2534821affe7b6f9af1f5aa9a5767e5f47`. The generated-reference job log is **241,832 uncompressed bytes**, SHA-256 `1f560910bfdb54320ec85554e6a56a2c5ccbc391d862647af948246fe1dddcc1`. Original GitHub timestamps and ANSI escapes remain present. [compressed-provenance.json](compressed-provenance.json) records both compressed and uncompressed hashes and byte counts for all three compressed files.

To reproduce the complete case inventory from this folder in a temporary directory:

```bash
evidence_work=$(mktemp -d)
gzip -dc native-job.log.gz > "$evidence_work/job.log"
cp source-provenance.json analyze_log.py "$evidence_work/"
python3 "$evidence_work/analyze_log.py"
gzip -dc full-log-analysis.json.gz > "$evidence_work/expected-analysis.json"
cmp "$evidence_work/log-analysis.json" "$evidence_work/expected-analysis.json"
```

The parser strips timestamps and ANSI escapes only for matching; it leaves the retained raw log unchanged. It checks announced/result counts, records all actual case outcomes, and handles expected-panic and split `--nocapture` lines. The reproduced analysis matched the retained analysis byte-for-byte during packaging. [manifest.json](manifest.json) hashes every retained file in this folder except the manifest itself.
