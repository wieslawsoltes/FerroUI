# Native application pass with a failed framework gate: `2b0fe4e1`

Source: [`2b0fe4e1642a9a4e989021c7249c8d751975b094`](https://github.com/wieslawsoltes/FerroUI/commit/2b0fe4e1642a9a4e989021c7249c8d751975b094).
Run/job: [39846931439 / 120064759737](https://github.com/wieslawsoltes/FerroUI/actions/runs/39846931439/job/120064759737).

The native feedback job **failed overall**, while the independent complete application suite **passed all 205 tests** in **24.35 seconds**, with zero failed, ignored or filtered application tests. The suite duration is not an interactive performance measurement.

| Gate | Result |
| --- | --- |
| Vello unhinted glyph geometry/pixels | **8 passed**, zero failed |
| FerroUI.Base paragraph filter | **15 passed**, zero failed |
| FerroUI.Controls paragraph filter | **11 passed, 2 failed** |
| Complete `ferrodesign-app --lib` suite | **205 passed**, zero failed |

The application step runs independently after a preceding framework-test failure; that failure still fails the job. The failed shared-paragraph shell step did not reach its later standalone TextBox/history commands.

## Added regression and remaining failure

This checkpoint fixes generic XAML background selection to choose an active fill (`visible && opacity > 0`) rather than an inactive earlier entry. The new `shell::text_export::tests::text_xaml_ancestor_background_ignores_visible_zero_opacity_fills` regression passed at log line **2538**. The complete suite also passed the native text export, rich-text application history, width/corner geometry and suppressed-text incremental tests present at this source. These application results do not clear the independent framework failures.

The two Controls failures were the same post-preedit text assertions recorded at [`ee1999b9`](../2026-10-22-ee1999b9-ci/README.md):

- `paragraph_preedit_replacement_uses_confirmed_ranges_and_blur_restores_the_collection`: actual `A🦀\nB\nC`, expected `A\nC`.
- `repeated_paragraph_ime_replacement_uses_owned_selection_and_restores_on_blur`: actual `A\nA\nA\n`, expected `A\n`.

The presenter was found; the TextBox retained the original text after preedit instead of the expected provisional selection removal. A subsequent investigation identified a production subscription gap for a presenter created after TextBox attachment. Published correction [`c88ad28f`](https://github.com/wieslawsoltes/FerroUI/commit/c88ad28f4fad60e6739fafbb18355c659a0a417b) handles late template application and presenter replacement and adds detach/reattach coverage while retaining both failed paragraph assertions. Its exact smoothing invalid-Enter/Escape native regression and expanded execution of all configured text-editing suites also require a separate run. No rerun pass for that source is recorded here. This job contains no browser execution and does not validate the later `40dafdab` inspector correction or a complete latest-head workflow.

## Saved evidence

[native-feedback-job.log](native-feedback-job.log) is the ordinary connector-decoded UTF-8 job log, **245,657 bytes**, SHA-256 **`6f817cf64fd2b8cd61624e9b814b692ba155ed7e059c574845b6316b66e4493b`**. The exact checkout hash appears at line **129**. Test results appear at lines **690** (Vello), **1584** (Base), **1901** (Controls) and **2554** (application); the failed assertions are at lines **1881–1899**.

The file hash identifies this saved decoded log, not a downloaded official artifact. No artifact ZIP or browser report was retrieved for this record. See [VALIDATION.md](../../../samples/FerroDesign/docs/VALIDATION.md) for the current validation boundary and historical complete passes.
