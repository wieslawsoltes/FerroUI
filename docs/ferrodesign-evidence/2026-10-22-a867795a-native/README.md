# Native rich-history feedback at `a867795a`

Source: [`a867795a8f03780f295c3edd2164b112443583d3`](https://github.com/wieslawsoltes/FerroUI/commit/a867795a8f03780f295c3edd2164b112443583d3), identified by the checkout at [log line 129](native-feedback-job.log#L129).

[Native run 39855458955](https://github.com/wieslawsoltes/FerroUI/actions/runs/39855458955), job [120095169276](https://github.com/wieslawsoltes/FerroUI/actions/runs/39855458955/job/120095169276), completed with **failure**. The corrected hosted rich-history fixture and the complete application suite passed. The remaining failure is the previously recorded template-replacement Redo regression; the two failing Controls filters overlap.

## Executed results

| Suite | Exact result | Saved log |
| --- | --- | --- |
| Public unhinted Vello geometry/pixels | 8 passed, 0 failed; 0.33 seconds | [Line 689](native-feedback-job.log#L689) |
| Base paragraph layout | 15 passed, 0 failed; 0.03 seconds | [Line 1587](native-feedback-job.log#L1587) |
| Controls paragraph editing | 15 passed, 1 failed; 0.16 seconds | [Line 1897](native-feedback-job.log#L1897) |
| Complete `text_box_tests::` filter | 172 passed, 1 failed; 2.51 seconds | [Line 2362](native-feedback-job.log#L2362) |
| Native rich-history filter | **5 passed, 0 failed**; 0.06 seconds | [Lines 2642–2648](native-feedback-job.log#L2642-L2648) |
| Complete application suite | **208 passed, 0 failed**; 25.11 seconds | [Line 3303](native-feedback-job.log#L3303) |

Every configured suite ran. The editing step returned exit code 1 at [line 2650](native-feedback-job.log#L2650); the separately collected application suite subsequently passed. These results are not a successful overall native gate.

## Rich-history correction and remaining failure

The hosted regression, `native_rich_history_restores_both_styles_and_application_state_before_text_changing`, passes at [line 2643](native-feedback-job.log#L2643). It creates and lays out a real TestRoot before editing, then executes native Undo and Redo. Assertions retain exact Unicode text, character-style and paragraph-style collection identities, application snapshot identities, restoration-before-`TextChanging` callback ordering, and presenter bindings. The other four history regressions also pass, covering updates without extra history or Redo replacement, synchronous host callback replacements, caret restoration, and payload release at the history limit and clear. This resolves the unhosted-presenter fixture failure recorded in the [preceding `cdee5f26` evidence](../2026-10-22-cdee5f26-ci/README.md#native-execution).

Both Controls failures are `paragraph_preedit_template_removal_restores_confirmation_without_discarding_redo`, at the final `!target.can_undo() && target.can_redo()` assertion in `text_box_tests.rs:3022`: [lines 1887–1897](native-feedback-job.log#L1887-L1897) and [2353–2362](native-feedback-job.log#L2353-L2362). The earlier confirmation/style/selection restoration and actual Redo/Undo assertions passed; the failure follows installation of a new template and cancellation of its subsequent composition. The separate reentrant template-installation regression passed at [lines 1882](native-feedback-job.log#L1882) and [2285](native-feedback-job.log#L2285).

The same-value TwoWay text-coercion correction [`58deaeee`](https://github.com/wieslawsoltes/FerroUI/commit/58deaeeeae2226ece8220d4abcda9d0a3678ca20) was published later and requires its own execution. This source also predates the native corner-radius additions at [`38713369`](https://github.com/wieslawsoltes/FerroUI/commit/38713369933656306b833f97fc02ff2c9ef01202); the 208-test application pass does not validate those new interactions. The [separate corner core result](../2026-10-22-38713369-core/README.md) has its own source and scope.

## Saved text and scope

| File | Saved UTF-8 bytes | SHA-256 |
| --- | ---: | --- |
| [native-feedback-job.log](native-feedback-job.log) | 315,718 | `d6028ca6114ad3f54759bc84ec24e145cbc2b7256bc0ffb235ebf0530335a35d` |

The log is ordinary connector-decoded job text saved with a final newline. Its hash identifies the saved UTF-8 file, not original transport bytes. No official artifact ZIP was downloaded for this evidence; no artifact CRC, screenshot contents or downloaded-runtime equivalence is claimed. This native result supplies no browser or optional Chromium-trace result for later checkpoints, including [`124ffdb7`](https://github.com/wieslawsoltes/FerroUI/commit/124ffdb74e2b39ca09d5a1bcd4052bbb9a61a165), and does not establish Figma visual parity.
