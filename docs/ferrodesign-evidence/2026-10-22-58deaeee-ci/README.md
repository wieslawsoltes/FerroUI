# Native text-history and generated-reference feedback at `58deaeee`

Source: [`58deaeeeae2226ece8220d4abcda9d0a3678ca20`](https://github.com/wieslawsoltes/FerroUI/commit/58deaeeeae2226ece8220d4abcda9d0a3678ca20), tree `f3d8a9ca4826cd165fca916b152a1edc0bb95b78`. The native checkout is printed at [line 129](native-feedback-job.log#L129); the generated-reference provenance independently records the same commit and tree.

## Native execution

[Native run 39857371121](https://github.com/wieslawsoltes/FerroUI/actions/runs/39857371121), job [120100682612](https://github.com/wieslawsoltes/FerroUI/actions/runs/39857371121/job/120100682612), completed with **failure**. All configured framework text/editing filters passed. The application test binary failed to compile, so **no application tests executed** in this job. [jobs-api.json](jobs-api.json) preserves the terminal job and step statuses.

| Suite | Exact result | Saved log |
| --- | --- | --- |
| Public unhinted Vello geometry/pixels | 8 passed, 0 failed; 0.34 seconds | [Line 696](native-feedback-job.log#L696) |
| Base paragraph layout | 15 passed, 0 failed; 0.03 seconds | [Line 1594](native-feedback-job.log#L1594) |
| Controls paragraph editing | **16 passed, 0 failed**; 0.17 seconds | [Line 1892](native-feedback-job.log#L1892) |
| Complete `text_box_tests::` filter | **174 passed, 0 failed**; 2.61 seconds | [Line 2346](native-feedback-job.log#L2346) |
| Native rich-history filter | **5 passed, 0 failed**; 0.06 seconds | [Line 2631](native-feedback-job.log#L2631) |
| Complete application suite | **Not executed: compile failure** | [Lines 3060–3076](native-feedback-job.log#L3060-L3076) |

The same-value text-coercion correction resolves the final Redo assertion previously preserved at [`cdee5f26`](../2026-10-22-cdee5f26-ci/README.md) and [`a867795a`](../2026-10-22-a867795a-native/README.md). `paragraph_preedit_template_removal_restores_confirmation_without_discarding_redo` passes in both overlapping Controls filters, at [lines 1890](native-feedback-job.log#L1890) and [2280](native-feedback-job.log#L2280). Its strengthened assertions include exact confirmation/style/selection and application-payload identities, native Redo/Undo before template replacement, and another actual Redo/Undo after the later canceled composition. The separate direct same-value assignment regression passes at [line 2327](native-feedback-job.log#L2327); different external text retains the reset semantics tested by that case. Reentrant template installation also passes at [lines 1889](native-feedback-job.log#L1889) and [2279](native-feedback-job.log#L2279).

The hosted rich-history regression passes at [line 2626](native-feedback-job.log#L2626), retaining character/paragraph styles, application snapshot identities, callback ordering and presenter bindings. Counts from these overlapping filters must not be added as unique test coverage.

The subsequent application compilation fails with **Rust E0282** at `crates/ferrodesign-app/src/canvas/corner_radius.rs:206`: the collection produced by `let handles = source…collect()` needs an explicit type before `handles.clone()` at line 257. Rust suggests a `Vec<_>` annotation. The job exits with code **101**. This is a compile failure, not a failed radius gesture assertion or a failed application test result. The earlier complete **208-test application pass at `a867795a`** remains valid for its older source and does not validate these new corner-radius paths.

## Official generated references

[Generated-reference run 39857370963](https://github.com/wieslawsoltes/FerroUI/actions/runs/39857370963), job [120098973929](https://github.com/wieslawsoltes/FerroUI/actions/runs/39857370963/job/120098973929), completed with **success**. All three official generators and all three drift/recompile gates passed. The generated patch printed between [lines 2726–2727](generator-job.log#L2726-L2727) is empty, and all five files have identical source and regenerated SHA-256 values.

[generated-provenance.json](generated-provenance.json) is the exact JSON printed at [lines 2728–2776](generator-job.log#L2728-L2776), with only timestamp prefixes removed. It identifies the exact commit/tree, Rust and Cargo versions, Cargo.lock hash, all six step outcomes, and all five source/output hashes. The two theme check groups intentionally ignore their writer tests after explicit regeneration. This successful generator result does not override the native application compile failure.

## Saved text and scope

| File | Saved UTF-8 bytes | SHA-256 |
| --- | ---: | --- |
| [native-feedback-job.log](native-feedback-job.log) | 284,986 | `829334bef4f0a36cb41788ee02e2e2064fe3b1fb39dbbe4c6e638611bdbe2046` |
| [generator-job.log](generator-job.log) | 247,181 | `d746b40e0887e7ce3915fedf9bbfb9bf563c58d909e7d18e5003a164ea6284e7` |
| [generated-provenance.json](generated-provenance.json) | 2,369 | `62e09244c5843808c5ca52542ddb60aa5e90f08f524001360196e3f04cbc07dc` |
| [jobs-api.json](jobs-api.json) | 5,760 | `125e3c7aa72da80706bbf811a3e8c527df67b9f2b71cb8d9c18799731a43d840` |

The logs are ordinary connector-decoded job text, preserving the supplied UTF-8 BOM and final newline. Hashes identify these saved UTF-8 files rather than original transport bytes. The API file groups the connector's unchanged native and generated job-summary payloads. No official artifact ZIP, downloaded runtime or capture/trace bytes were retrieved; no artifact CRC or binary equivalence is claimed. These are exact-source native and generator results, not a browser, optional Chromium-trace, latest-head or Figma visual-parity pass.
