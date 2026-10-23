# Native feedback at `40dafdab`

Source: [`40dafdabc6ba939e34389bc6a03c5fd404833362`](https://github.com/wieslawsoltes/FerroUI/commit/40dafdabc6ba939e34389bc6a03c5fd404833362). [Run 39851783134](https://github.com/wieslawsoltes/FerroUI/actions/runs/39851783134), native job [120079595803](https://github.com/wieslawsoltes/FerroUI/actions/runs/39851783134/job/120079595803), completed with **failure** on 22 October 2026.

## Exact executed scope

| Suite | Result |
| --- | --- |
| Public unhinted Vello glyph geometry/pixels | 8 passed, 0 failed |
| Base paragraph layout | 15 passed, 0 failed |
| Controls paragraph editing | 11 passed, 2 failed |
| Complete application suite | 206 passed, 1 failed; 14.82 seconds |

The two Controls failures are the same late-template preedit subscription failures preserved at [the preceding checkpoint](../2026-10-22-ee1999b9-ci/README.md). This run predates the production subscription correction in [`c88ad28f`](https://github.com/wieslawsoltes/FerroUI/commit/c88ad28f4fad60e6739fafbb18355c659a0a417b). The subsequent TextBox and native-history commands were not reached because this workflow version stopped the combined editing step at its first failing command. Its independent application step still executed and failed separately.

The new application failure was `shell::fields::tests::native_rejected_radius_enter_keeps_focus_and_escape_restores_without_canvas_selection_loss` at `fields_tests.rs:143`. Escape restored `Some("12.50")`; the fixture expected `Some("12.5")`. The established inspector `number()` formatter emits two decimal places for fractional values. The failure therefore concerns the fixture's expected display string, not the numeric document value. The focused control, selected layer and history assertions before this comparison had passed; later assertions in the failed test were not reached. The other equal-value-radius test passed, but its literal `"12.5"` differed from the initial displayed bytes and did not exercise the exact unchanged-draft branch intended by its description.

[`54742a5e`](https://github.com/wieslawsoltes/FerroUI/commit/54742a5ef53fbcdde7e9e5f48ccbda8325ee6b7f) corrects the expected native Escape text to `"12.50"` and strengthens the bound-radius test to assert and submit exactly those displayed bytes. The same correction uses `"18.50"` for the polygon browser fixture's Escape display and `"12.50"` for its equal-value binding/no-op submissions. Document, selection, focus, history and export assertions remain. This evidence records the failing source and the later correction; it does not turn the failed run into a pass.

## Evidence and retrieval limits

[native-feedback-job.log](native-feedback-job.log) is the ordinary UTF-8 decoded job log returned by the GitHub connector, saved with a final newline. Its saved-file length is **247,194 bytes** and SHA-256 is **`7fccb72b7057e7938d74e37c679dad478f351294f82456041d47d55c6f62ad98`**. This checksum identifies the saved text, not GitHub's original transport bytes. No official artifact ZIP or browser output was retrieved for this record. Browser coverage belongs to the separate full-workflow job and is not inferred from these native tests.

