# Selected browser validation of runtime 08e29e4f

This completed run **failed overall**. It exercised the actual FerroUI WebAssembly application in Software2D and WebGL2 with 12 selected feature scenarios and the harness's standard common checks. Software2D completed 9 of 12 scenarios; WebGL2 completed 8 of 12. The results preserve every failure and the checks completed before each failure.

The distribution came from [browser build run 39945441130](https://github.com/wieslawsoltes/FerroUI/actions/runs/39945441130), artifact `12499117711`, built from commit `08e29e4f530c0ae65f6e2ac072f381605e23ed0c`. All 16 payload files matched the embedded source manifest (17 files including the manifest); archive SHA-256, ZIP CRCs, WebAssembly header, pinned tools, and the active FerroUI Vello dependency graph were verified. The graph contains no Skia package. The 52,003,090-byte WASM file has SHA-256 `ae7f7a1c5642e99786d94ff7ef272c97c5e48a90415654e335e337ebe49d7958`.

The browser harness was commit `0f0732784bd6fdf1cb2477a61dd332d9a007fec2`. Its 55 recorded file hashes matched before and after execution. Runtime production inputs match that harness commit, but this run predates the later upstream FerroUI 0.4.0 integration at `1c7151f5`. Chromium was `156.0.8078.4`, Playwright `1.64.1`, and Node `24.19.0`. The observed WebGL2 device was SwiftShader.

## Completed results

| Scenario | Software2D | WebGL2 |
| --- | --- | --- |
| Smoothed corners | Passed: 2 check groups | Passed: 2 |
| Polygon corners | Failed after 1 | Failed after 1 |
| Radius handles | Passed: 7 | Passed: 7 |
| Width profiles | Passed: 5 | Passed: 5 |
| Width handles | Passed: 4 | Passed: 4 |
| Gradient handles | Passed: 17 | Failed during entry, 0 completed |
| Paragraphs | Failed after 4 | Failed after 4 |
| Independent layout padding | Passed: 6 | Passed: 6 |
| Layout spacing variables | Passed: 5 | Passed: 5 |
| Selection and clipping | Passed: 9 | Passed: 9 |
| Guide bindings | Failed after 3 | Failed after 3 |
| Prototype motion | Passed: 3 | Passed: 3 |

Including common checks, the harness recorded 88 completed check groups for Software2D and 68 for WebGL2. These counts include progress within failed scenarios. `scope.full` remains `false`.

The spacing-variable cases include native binding controls, mode changes, literal overrides, exact history, prototype `SetVariable` reflow, a real native Save download during preview, and authored-state restoration. The Software2D gradient cases include the new radial and angular editing interactions. Both corner runs retained the same strict thresholds: 16×/32× reference convergence had maximum alpha difference 1.96875 against a limit of 4, and native PNG versus the 32× reference had maximum alpha difference 10 with zero pixels above 12.

## Preserved failures

Polygon export attempted the XAML button while the Design inspector was active. The real button belongs to Inspect, so its captured bounds were hidden and zero-sized. The later correction must navigate the actual tab and use a runtime that exposes its diagnostic bounds.

Paragraph authoring and the first four checks completed, then the repeated Unicode paragraph case timed out after `keyboard.insertText` at caret zero. The failure document, native state and exact trace are preserved. The reviewed multiline clipboard correction belongs to subsequent validation.

Guide variable authoring, modes and stack reordering completed, then the child-constraint test stopped because `HorizontalConstraint` lacked ComboBox choice diagnostics. The subsequent runtime adds those existing controls to the read-only projection.

WebGL2 gradient setup clicked the native paint-card control but did not satisfy the editor-entry predicate; its failure capture still showed the paint modal. It reached no gradient assertions. Software2D completed all 17 gradient checks. A later focused replay is required to validate a correction.

Neither mode reported a page exception. Software2D recorded a 404 for `/favicon.ico`; the common harness retains that console record. This missing asset also matters to the separate strict text benchmark. Shared-host OOM counters did not increase during this run.

## Evidence

- [summary.json](summary.json) contains every check description, selected scenario result and failure stack.
- [report.json.gz](report.json.gz) is the complete original 1,611,112-byte JSON report, compressed without altering its contents.
- [provenance.json](provenance.json), [build-provenance.json](build-provenance.json), and [harness-inputs.json](harness-inputs.json) record exact source, runtime and file hashes.

Both complete Playwright trace archives and every screenshot/export remain in the local validation report. Their hashes and sizes are recorded in the provenance; the 101 MB of trace archives are not committed to git. The report makes no claim of full Figma feature coverage, Figma screenshot equivalence, or hardware GPU throughput.
