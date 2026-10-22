# Native lifecycle and generated-reference feedback at `c88ad28f`

Source: [`c88ad28f4fad60e6739fafbb18355c659a0a417b`](https://github.com/wieslawsoltes/FerroUI/commit/c88ad28f4fad60e6739fafbb18355c659a0a417b).

## Native execution

[Native run 39852882829](https://github.com/wieslawsoltes/FerroUI/actions/runs/39852882829), job [120084104777](https://github.com/wieslawsoltes/FerroUI/actions/runs/39852882829/job/120084104777), completed with **failure**. Unlike earlier feedback versions, it executes every configured editing command before returning a failing step, so later suites are no longer hidden behind an earlier failure.

| Suite | Exact result |
| --- | --- |
| Public unhinted Vello geometry/pixels | 8 passed, 0 failed; 0.33 seconds |
| Base paragraph layout | 15 passed, 0 failed; 0.03 seconds |
| Controls paragraph editing | **14 passed, 0 failed**; 0.14 seconds |
| Complete `text_box_tests::` filter | **171 passed, 0 failed**; 2.67 seconds |
| Native rich-history filter | 4 passed, 1 failed; 0.06 seconds |
| Complete application suite | 207 passed, 1 failed; 25.24 seconds |

The Controls paragraph pass includes both previously failing late-template composition fixtures, with their existing assertions retained, and the new presenter lifecycle regression. The latter exercises late template creation, live presenter replacement, detached old-presenter isolation and actual-focus reattachment. These results validate the subscription correction at this source; they do not validate the later null-template-removal changes.

The history failure is `native_rich_history_restores_both_styles_and_application_state_before_text_changing`, an `Option::unwrap()` at `text_box_history_tests.rs:144`. The fixture had not hosted and laid out its TextBox before searching for its presenter. Its preceding history, application payload, callback-order and text assertions had passed; the final presenter assertion was not reached. [`a867795a`](https://github.com/wieslawsoltes/FerroUI/commit/a867795a8f03780f295c3edd2164b112443583d3) hosts this fixture in a real TestRoot and performs initial layout before editing, asserts initial and restored character/paragraph binding identities, and retains every original assertion. No production history change is inferred from this fixture failure.

The application failure is the canonical fractional display mismatch already preserved at [`40dafdab`](../2026-10-22-40dafdab-native/README.md): restored `"12.50"` versus expected `"12.5"`. [`54742a5e`](https://github.com/wieslawsoltes/FerroUI/commit/54742a5ef53fbcdde7e9e5f48ccbda8325ee6b7f) corrects that fixture and strengthens the exact unchanged-value submission path. The new smoothing `60` → rejected `101` → Escape regression passed in this run. The failed overall application suite is not described as a complete pass.

A later production correction, [`cdee5f26`](https://github.com/wieslawsoltes/FerroUI/commit/cdee5f26916c7bc61305b99540c203e469cdfc21), disconnects the old presenter and IME ownership when an attached TextBox removes its template. Its new regressions cover confirmed Unicode/style/selection restoration, existing Redo, stale presenter/client input, later template installation and reentrant template installation during restoration. Those changes require their own exact-source execution; they are not included in these passing filters.

## Official generated references

[Generated-reference run 39852882875](https://github.com/wieslawsoltes/FerroUI/actions/runs/39852882875), job [120083421644](https://github.com/wieslawsoltes/FerroUI/actions/runs/39852882875/job/120083421644), completed with **success**. All three generators and all three drift/recompile gates passed. The five generated files had identical source and regenerated hashes, matching the earlier [`ed4f03dd` provenance](../2026-10-22-ed4f03dd-generated/provenance.json); the printed generated patch was empty.

[generated-provenance.json](generated-provenance.json) is extracted from lines **2968–3016** of the saved [generator-job.log](generator-job.log), after removing each GitHub timestamp prefix. It identifies source commit/tree, exact toolchain, Cargo.lock hash, all six step outcomes and all five source/output hashes. Each theme check group intentionally ignores its writer test after the separate explicit regeneration command has run. This is a pass of the configured generated-reference gates, not a claim that every repository test ran.

## Saved text and scope

| File | Saved UTF-8 bytes | SHA-256 |
| --- | ---: | --- |
| [native-feedback-job.log](native-feedback-job.log) | 315,641 | `d90eeac19c210b3516523d979b27d1c2ee7a912f698516da3837eb0aeebeadaa` |
| [generator-job.log](generator-job.log) | 264,115 | `58fb57a7d09dffc75026aff5ac70fbb00d23a69d4ff055e55a009c533fa3921f` |
| [generated-provenance.json](generated-provenance.json) | 2,370 | `5953e062451fdca066682a433b0782984c0977e2b9e4e88a90397bb3c72521b8` |

Logs are ordinary connector-decoded job text saved with a final newline. These hashes identify the saved files, not original transport bytes. No official artifact ZIP was retrieved, so no raw capture, CRC or downloaded-runtime equivalence is claimed. The `c88ad28f` full browser workflow was superseded while pending and has no completed browser job. Native/generator results cannot establish browser or Figma visual parity.

