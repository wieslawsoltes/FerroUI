# Native target-cache investigation at `124ffdb7`

Scope: completed [native feedback run 39858027415, job 120106042788](https://github.com/wieslawsoltes/FerroUI/actions/runs/39858027415/job/120106042788), exact source `124ffdb74e2b39ca09d5a1bcd4052bbb9a61a165`. Evidence is [native-feedback-job.log](native-feedback-job.log), [workflow-api.json](workflow-api.json), and [cache-compare-api.json](cache-compare-api.json). The subsequent logging-filter probe below is separate from this CI run. No Cargo freshness, timestamp, cache-policy or production behavior change is proposed here.

## What the completed job proves

- Checkout completed at log lines **108–129**. Cache restoration then succeeded at **165–180**, restoring **913,854,929 bytes** from `ferrodesign-native-feedback-Linux-c853afe99434eacd4dcfefdcac84ff2b9147cf9ac3c4d464925cb9033cb78b1c-58deaeeeae2226ece8220d4abcda9d0a3678ca20`.
- The requested key has the same dependency-hash prefix and the new commit suffix. The restore paths include the registry, Git sources and `samples/FerroDesign/target` (**143–149**). This was a successful fallback-key restore, not a cache miss.
- GitHub's exact comparison from that cached commit to the checked-out commit reports **one commit and seven changed files**. They are the two workflows, text-trace JavaScript and text-rendering documentation. **No Rust source, Cargo manifest or lockfile changed.** The native workflow's only change added the fingerprint logging environment variable. The full filename list and exact workflow patch are preserved in [cache-compare-api.json](cache-compare-api.json).
- The job uses pinned **Rust 1.99.0** (line **199**), the same target path and test settings `DEBUG=0`, `CODEGEN_UNITS=16` (**318–320**). The source comparison shows no toolchain/profile-setting change between the cached and checked-out commits.
- Nevertheless, the first glyph command recompiles repository crates: Build.Scan, Base, Controls and System at **325–328**, then Remote.Protocol, OpenGL, HarfBuzz, Fonts.Inter and Vello at **516, 585–586, 631–632**. Cargo reports **2m 05s** for this build at **683**.
- Later invocations use distinct package/test contexts and can require different units; their repeated package names alone do not prove a cache defect. Repeating Controls filters reuses the same test executable in **0.11 s** and **0.10 s** build time (**2167–2168**, **2621–2622**), so some reuse works within the job.

## What the diagnostic did not establish

Line **323** confirms `CARGO_LOG=cargo::core::compiler::fingerprint=info` was passed to the first command. The complete decoded log contains **no emitted Cargo fingerprint reason records**. It contains ordinary compilation messages, not a report that identifies a newer source timestamp, missing dep-info, changed feature set, environment change or other freshness cause.

Fresh checkout timestamps are consistent with the unchanged-source/rebuilt-path-crate pattern, but this run does not prove that explanation. Build-script inputs and the completeness of restored per-unit outputs also remain uninspected. A successful archive restore does not itself prove that every needed fingerprint and output was present. No exact speedup or avoidable-build-time estimate follows.

## Confirmed logging-filter defect and narrow correction

The pinned Cargo **1.99.0**, commit `5f94df4789f005f9a352888e8355ffc645b7ed0e`, uses the logging target **`cargo::compiler::fingerprint`**. The workflow requested the older **`cargo::core::compiler::fingerprint`** target. Changing only its level would leave the target wrong. Current [official Cargo fingerprint documentation](https://doc.rust-lang.org/stable/nightly-rustc/cargo/compiler/fingerprint/index.html), viewed as Cargo 1.99.1, explicitly documents `CARGO_LOG=cargo::compiler::fingerprint=trace`.

Local validation verified the target with the pinned **1.99.0** Cargo on one isolated dependency-free library, using `cargo check --offline` and two fresh target directories. Both commands exited **0**. The old target at `trace` emitted **0 fingerprint records**; the corrected target at `trace` emitted **4**. [fingerprint-probe/result.json](fingerprint-probe/result.json) preserves the Cargo version, counts and log hashes; [old.log](fingerprint-probe/old.log) and [current.log](fingerprint-probe/current.log) preserve the text outputs. The exact [manifest](fingerprint-probe/Cargo.toml), [lockfile](fingerprint-probe/Cargo.lock) and [source](fingerprint-probe/src/lib.rs) are also retained. The successful current log identifies a missing fingerprint in its fresh target, as expected; it does not diagnose the CI cache.

The minimal correction is **only** `CARGO_LOG=cargo::compiler::fingerprint=trace` on the existing first glyph command. No `-vv`, test-command, profile, toolchain, cache-key or freshness change is needed to validate this logging correction. A later CI log must establish the actual dirty-unit reasons.

This is logging-filter validation, not a FerroUI build, a cache-freshness experiment or a performance measurement. Do not introduce timestamp rewriting or content-manifest mtime restoration on the strength of the current log. If a later trace identifies newer checkout inputs, any proposed restoration would need content equality, safe path handling, complete tracked build inputs and a demonstrated rejection of changed files before it could be trusted.

The failed job's application error is separately explained by **E0282 at `canvas/corner_radius.rs:206`**. It is not evidence that a cache caused incorrect compilation, and the later published type annotation is not a cache-performance result.
