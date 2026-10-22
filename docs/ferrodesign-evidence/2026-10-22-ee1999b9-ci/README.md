# Current-feature CI: partial passes and failed gates at `ee1999b9`

Source: [`ee1999b93e6ebbcd089690e7690e672edf0a902e`](https://github.com/wieslawsoltes/FerroUI/commit/ee1999b93e6ebbcd089690e7690e672edf0a902e).

The complete FerroDesign workflow **failed**, and the independent native feedback job **failed**. The application suite nevertheless passed all **204 tests**, the core passed all **493 tests**, and the browser job built the actual Vello WebAssembly application and passed the real Chromium persistence checks. These are distinct completed gates; they do not constitute a complete current-feature pass.

## Exact runs and results

| Workflow/job | Observed result |
| --- | --- |
| [Native feedback run 39845598687 / job 120059769833](https://github.com/wieslawsoltes/FerroUI/actions/runs/39845598687/job/120059769833) | **Failed** overall. **8/8 Vello glyph** and **15/15 Base paragraph** tests passed; Controls paragraph tests had **11 passes and 2 failures**. The independent application step then passed **204/204 tests** in **18.13 s** |
| [Complete run 39845598638 / native job 120061876291](https://github.com/wieslawsoltes/FerroUI/actions/runs/39845598638/job/120061876291) | **Failed** overall. **493/493 core tests** passed in **2.43 s**, plus **19/19 persistence-host** and **4/4 input-host** tests. All **five** Vello/no-Skia target graphs passed. Native preedit tests had **11 passes and 1 failure**; later framework/application steps in this job were skipped |
| [Complete run 39845598638 / browser job 120061875782](https://github.com/wieslawsoltes/FerroUI/actions/runs/39845598638/job/120061875782) | **Failed** overall. The real WASM build and **5/5 Chromium IndexedDB** checks passed. The editor suite passed **23 Software2D / 23 WebGL2 checks**, then both modes failed at the corner inspector assertion. The independent text-pan measurement step subsequently completed successfully |

The native feedback workflow deliberately runs its application step after a preceding framework-test failure. That earlier failure still fails the job. The application counts are complete, unfiltered `ferrodesign-app --lib` results, not a selected subset or a replacement for the failed Controls tests. The later standalone TextBox/history commands in the failed shared-paragraph shell step were not reached.

The five dependency graphs are Emscripten, Linux x64, Windows x64, macOS ARM64 and macOS x64. They verify active normal/build dependencies containing Vello and excluding Skia packages; they do not execute every desktop GUI.

## Native paragraph failures

Both Controls failures occurred after `set_preedit_text`, where the TextBox still contained the original text instead of the expected provisional selection removal:

| Test in `presenters::text_presenter_paragraph_tests` | Actual text | Expected text |
| --- | --- | --- |
| `paragraph_preedit_replacement_uses_confirmed_ranges_and_blur_restores_the_collection` | `A🦀\nB\nC` | `A\nC` |
| `repeated_paragraph_ime_replacement_uses_owned_selection_and_restores_on_blur` | `A\nA\nA\n` | `A\n` |

The first test also accounts for the full workflow's single preedit failure. See [native feedback log](native-feedback-job.log) lines 1880–1900 and [full native log](full-native-job.log) lines 1724–1736. This is different from the earlier `605f04c3` fixture lookup returning `None`: the presenter was found here and the text assertion failed. A subsequent source investigation identified a production subscription gap when the TextPresenter is created after the TextBox is attached. Published correction [`c88ad28f`](https://github.com/wieslawsoltes/FerroUI/commit/c88ad28f4fad60e6739fafbb18355c659a0a417b) handles late template application and presenter replacement, with detach/reattach regression coverage. Both failed paragraph assertions are retained. No rerun pass for that later source is recorded here; these failed outcomes remain unchanged.

## Browser scope and failure

[browser-summary.json](browser-summary.json) preserves the exact printed summary, parsed from [browser-job.log](browser-job.log) lines **1258–1336** after removing timestamp prefixes. Its content matches the logged JSON. It reports `passed: false`, `scope.full: true`, both requested modes and `scenarios: "all"`. Here `full` describes the requested scope, not completed coverage.

Both modes completed the common gates, vector-network authoring and native-authoring checks, including successful automatic transverse-intersection splitting and atomic overlapping-edge rejection. They then entered `corners` and failed at `tests/corners-browser.mjs:34`: the inspector text was `""` instead of `"60"` after Escape from a rejected `101` draft. The preceding assertions had already checked that the invalid text remained `"101"` and that the document/history were unchanged. The remaining feature scenarios did not execute. Chromium was **156.0.8078.4**; WebGL2 reported **ANGLE/Vulkan/SwiftShader**, not a physical GPU.

The subsequent `40dafdab` inspector correction retains focus after rejected Enter, so Escape can discard that field's draft instead of clearing canvas selection. It also makes explicit equal-value radius entry detach its variable binding while passive blur remains unchanged. The later `c88ad28f` checkpoint adds the exact smoothing invalid-Enter/Escape native regression and ensures the configured text-editing suites all run before CI reports any failure. These source changes are not a successful browser rerun; the original corner failure is retained.

The separate Chromium IndexedDB output at log lines **1196–1223** passed migration/current-and-previous atomicity, abort-after-request-success rollback, stale-tab conflict rejection, a **12,000,042-character** document reload, and aborted-migration preservation. These five real-browser checks are separate from the 19 host tests and the failed editor suite.

## Text-pan measurements

The separate benchmark used **150 visible Inter labels**, **12 warmups** and **36 measured pointer motions per mode**. It used the `compact-v1` read-only frame-state bridge. The overall rounded values come from log lines **1350 and 1353**. [text-pan-phases.json](text-pan-phases.json) contains the two phase records printed at lines **1352 and 1355**, with exact phase aggregates and each mode's three slowest samples; its objects match those logged records.

| Observation | Software2D median / p95 | WebGL2 median / p95 |
| --- | ---: | ---: |
| Dispatch through expected camera/new native frame observation | **15.13 / 21.51 ms** | **15.64 / 1,275.36 ms** |
| Host pointer dispatch | **12.84 / 13.78 ms** | **11.42 / 1,265.21 ms** |
| Additional frame-observation wait after dispatch | **2.23 / 6.27 ms** | **4.08 / 10.15 ms** |
| Cumulative dispatch through native presentation observation | **76.61 / 109.21 ms** | **127.01 / 1,358.52 ms** |

The first duration starts before `page.mouse.move` and ends after the expected camera and newer native frame are observed. The presentation duration is cumulative from that same start; it includes protocol work and observing a newer public presentation counter. It is not an additional phase to add to the first row. Quantiles from different distributions are not additive. Two later animation callbacks are recorded separately as settling observations.

Large WebGL2 delays appear in the dispatch measurement itself. These records do not identify their cause or isolate drawing, glyph preparation or GPU execution. They include automation, bridge serialization, input handling and scheduling. A presentation counter is not a GPU-completion timestamp. The high tail remains visible; no FPS, physical-GPU throughput, Figma parity or speedup claim follows. The earlier benchmark used a different observation method, so comparing its headline numbers does not establish an implementation improvement.

## Preserved files and provenance

The logs are ordinary UTF-8 text returned by the GitHub connector. They record exact checkout hashes at native-feedback line **129**, full-native line **130** and browser line **130**. Local SHA-256 values below describe these saved files, not the official artifact ZIPs.

| File | Bytes | SHA-256 |
| --- | ---: | --- |
| [native-feedback-job.log](native-feedback-job.log) | 245,443 | `8221eb4113fc69c12fb56593b7143c3a81c4794e9a1910ee3b915190d23425cf` |
| [full-native-job.log](full-native-job.log) | 165,529 | `e7bc5f711727a0340c64cb24d348a6be12f67fdf2a0d31aa2b1df0a3ac63c31e` |
| [browser-job.log](browser-job.log) | 141,185 | `d53f9055d43e6df85c7a26ac294c8f6b5cd9b95d88f4130c8117ef0c5c000dd0` |
| [browser-summary.json](browser-summary.json) | 6,555 | `88c5bffbe8138f753a7a9f24b2845b6b460e2b6ffa01c858feda68f152e0ae8a` |
| [text-pan-phases.json](text-pan-phases.json) | 17,772 | `8778814d585d2b7d3328ee2c6608dd94e976325d47fc53510fbde5b2bfaca13b` |
| [jobs-api.json](jobs-api.json) | 7,622 | `3b935d24a42f0deab1c1b9079fc3bf251a76843b5f5ffd19495f03e20c3dd7a3` |
| [artifacts-api.json](artifacts-api.json) | 2,621 | `95956e68e46951aa42f30fa1ab9a4dce10cadf2612c6306389e93e5c8e4e5e87` |

The official build artifact is **12458160518**, editor validation artifact **12457049011**, and native validation artifact **12454809818**. The metadata preserves their published sizes/digests. Their ZIP bytes were not retrieved, so no local CRC check, full raw browser-report inspection, screenshot/capture-total audit, detailed editor page/console-error-array inspection or runtime-byte comparison is claimed. The saved phase record contains only the logged aggregates and selected samples, not the complete benchmark artifact.

See [VALIDATION.md](../../../samples/FerroDesign/docs/VALIDATION.md) for later source corrections, pending gates and the separately preserved complete predecessors.
