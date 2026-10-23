# Native lifecycle and generated-reference feedback at `cdee5f26`

Source: [`cdee5f26916c7bc61305b99540c203e469cdfc21`](https://github.com/wieslawsoltes/FerroUI/commit/cdee5f26916c7bc61305b99540c203e469cdfc21), tree `3d329806fba18326e88e32c2e61d011151a65dff`.

## Native execution

[Native run 39854329204](https://github.com/wieslawsoltes/FerroUI/actions/runs/39854329204), job [120091228339](https://github.com/wieslawsoltes/FerroUI/actions/runs/39854329204/job/120091228339), completed with **failure**. The saved [native-feedback-job.log](native-feedback-job.log) contains every configured suite's result, including the later successful application suite.

| Suite | Exact result |
| --- | --- |
| Public unhinted Vello geometry/pixels | 8 passed, 0 failed; 0.20 seconds |
| Base paragraph layout | 15 passed, 0 failed; 0.02 seconds |
| Controls paragraph editing | 15 passed, 1 failed; 0.10 seconds |
| Complete `text_box_tests::` filter | 172 passed, 1 failed; 1.47 seconds |
| Native rich-history filter | 4 passed, 1 failed; 0.03 seconds |
| Complete application suite | **208 passed, 0 failed**; 15.32 seconds |

The two Controls filters overlap and fail on the same regression, `paragraph_preedit_template_removal_restores_confirmation_without_discarding_redo`. Both report the final `!target.can_undo() && target.can_redo()` assertion at `src/FerroUI.Controls/text_box_tests.rs:3022`. Initial template removal, confirmed Unicode text/style/selection restoration, isolation from the old presenter and input client, and an actual Redo followed by Undo all passed before this later assertion failed. The remaining failure follows installation of a new template and cancellation of its new composition. The separate `paragraph_preedit_template_removal_respects_reentrant_template_installation` regression **passed** in both filters. Follow-up [`58deaeee`](https://github.com/wieslawsoltes/FerroUI/commit/58deaeeeae2226ece8220d4abcda9d0a3678ca20) changes same-value TwoWay text-binding coercion; that correction requires its own exact-source result.

The history failure is the previously identified unhosted fixture: `native_rich_history_restores_both_styles_and_application_state_before_text_changing` unwraps a missing presenter at `text_box_history_tests.rs:144`. Its preceding history, rich application payload, callback-order and text assertions passed. The later fixture correction [`a867795a`](https://github.com/wieslawsoltes/FerroUI/commit/a867795a8f03780f295c3edd2164b112443583d3) supplies a real TestRoot and initial layout, retaining the original assertions and adding initial/restored binding-identity checks. That correction is outside this source's execution.

The complete application pass includes the corrected canonical fractional-radius fixtures, unchanged-value binding detachment, rejected Enter followed by Escape, and smoothing-percent restoration. The overall native workflow remains failed because the Controls/history failures above are still present at this source.

## Official generated references

[Generated-reference run 39854329141](https://github.com/wieslawsoltes/FerroUI/actions/runs/39854329141), job [120090095921](https://github.com/wieslawsoltes/FerroUI/actions/runs/39854329141/job/120090095921), completed with **success**. All three generators and all three drift/recompile gates passed. Each of the five generated files has the same source and regenerated SHA-256; the printed generated patch is empty.

[generated-provenance.json](generated-provenance.json) matches the timestamp-stripped JSON printed at lines **2730–2778** of [generator-job.log](generator-job.log). It records the exact source commit/tree, Rust and Cargo versions, Cargo.lock hash, all six step outcomes and all five source/output hashes. Each theme check group intentionally ignores its writer test after the separate explicit regeneration step. This result covers the configured generated-reference gates.

## Saved text and scope

| File | Saved UTF-8 bytes | SHA-256 |
| --- | ---: | --- |
| [native-feedback-job.log](native-feedback-job.log) | 317,041 | `c7ecb1d396bf7c480e31397b5c8f80b92bac936969335d66318cb96ff694fb55` |
| [generator-job.log](generator-job.log) | 247,422 | `e9685654aa0666a41b541aa19c32eaec7f898d31be060904e0d24c0642c47df8` |
| [generated-provenance.json](generated-provenance.json) | 2,369 | `06d50cac50bc7a838c51957a22beac532c7c2427f0c9b0edde5d77487a6c61ad` |

The logs are ordinary connector-decoded job text saved with a final newline. These hashes identify the saved files rather than the original transport bytes. No official artifact ZIP bytes were retrieved, so downloaded capture contents, CRC verification and runtime-binary equivalence remain unverified. The `cdee5f26` full browser workflow was canceled while pending and produced no jobs. These native/generator results apply to the source above; they do not establish browser or Figma visual parity for later source.
