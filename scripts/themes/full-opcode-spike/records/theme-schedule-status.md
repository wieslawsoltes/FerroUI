# Extracted theme dictionary schedules: checkpoint

The additive `schedule-wip.patch` contains all ten changed files, including the three new files. It is 25,305 bytes, SHA-256 `56ba6747caa152289e4d16e050b4f94bd778dfceebda0181e2b674d012c96c44`. Its base is main at `933fd4632` plus the page-operands snapshot (`typed-page-operands-snapshot.patch.gz`); the additive patch applies to that pair. No changes to root production files.

## Existing implementation and baseline evidence

The previous theme-unit snapshot under `target/typed-xaml/theme-unit-runs/source.patch` and `scripts/themes/full-opcode-spike/typed-xaml.md` already include the `ApplyThemeUnit` construction opcode. It selects the run from the active theme-parts table. The function invoked still executes a native sequence of dictionary additions. Existing standalone probe binaries exercise that older implementation. No durable previously implemented inner dictionary-run interpreter was found among the inspected snapshots.

Measured from the most recently generated construction-enabled theme crate sources under the root `target/dist/build/ferroui-themes-{theme}-*/out/xaml/compiled_xaml.rs`, restricted to `fn …_unit_N` bodies. These are input counts. No patched build or coverage figures have been obtained.

| Theme | Extracted functions | Row calls | Deferred text-key inserts | Deferred type-key inserts | Immediate inserts |
|---|---:|---:|---:|---:|---:|
| WinUI | 347 | 887 | 455 | 104 | 50 |
| Fluent | 86 | 357 | 125 | 83 | 21 |

## Current changes

The new emitter attempts each complete extracted unit run only for the construction-bytecode backend, after rows have been lowered. It uses the existing packed BytecodeImage, emits a single apply_theme_run call with per-run typed imports and the originating document, and retains the original generated body on a shape mismatch. The linker still reaches this function and its imports exclusively via the original theme-unit run table. Instructions cover all *observed source shapes*: the nine row types, deferred string/type keys, enum constants, integers, bools, parsed FontWeight/BoxShadows, GridLength and duration ticks. The new runtime calls the existing typed row, deferred capture, dictionary insertion and error-position helpers. Type-key deferred resources preserve the separate defer and insertion positions. No stock theme or framework optional import is referenced by the encoder's import tables globally.

## Verification state and remaining work

Both new implementation files were rustfmt-formatted. `git diff --check HEAD` passed, and `git apply --check --reverse schedule-wip.patch` passed on the working tree. Compilation and test results were pending at this checkpoint. The first native library test run has no schedule-specific assertions. The existing native extraction tests were just updated for the additional optional argument. The patch is WIP; there are no linked sizes, timings, trim measurements, semantic results or proof of complete lowering yet.

1. Check the test log and repair any build failures. Add loader tests for successful lowering, a rejected non-resolved/changed call, correct position and escaped-string encoding, and preservation of native and unsupported runs. Add runtime regressions for mixed row/deferred inserts, defer laziness, selected unit import retention, first-error ordering/position and malformed operands.
2. Generate both full themes. Inspect every extracted run, count exactly one schedule call and report any retained native entry bodies. Verify all input counts above against decoded instruction counts. Ensure new generated source compiles, particularly trait object bounds and const-promotion of imported closures. Recheck the exact-forms parser with arbitrary quoted keys and scientific/numeric literals; an unsupported form must preserve the original body.
3. Build native, frozen and patched standalone probes using identical profiles and features, for full and Button-selected WinUI/Fluent. Existing probe harness is in root `scripts/themes/full-opcode-spike/typed_theme_probe.py`; its `prepare.py`, `verify.rs`, `button_units.rs` and `scripts/themes/compact-spike` must also travel. It builds from the repository where the script resides, so copy them into the new checkout if they were not included in the frozen production patch. Compare inventory, keys, deferred resource values, actual Button template builds, allocated bytes, linked units and executable bytes. Measure the Catalog, its screenshots and its first draw on an integrated patch.
4. Avoid declaring pure coverage for other ordinary functions, deferred factories or the main populate methods based on this patch. Scope is extracted theme dictionary run bodies only.

## Reproduce in a clean checkout

From a checkout of this branch, create an isolated worktree. The historical source snapshot and the additive patch are both tracked.

```sh
repo=$(git rev-parse --show-toplevel)
evidence="$repo/scripts/themes/full-opcode-spike"
git -C "$repo" worktree add --detach ../ferro-theme-schedule 933fd4632
cd ../ferro-theme-schedule
gzip -dc "$evidence/typed-page-operands-snapshot.patch.gz" | git apply -
git apply --check "$evidence/records/schedule-wip.patch"
git apply "$evidence/records/schedule-wip.patch"
cargo test --locked -p ferroui-markup-xaml -p ferroui-markup-xaml-loader --features ferroui-markup-xaml-loader/emitter --lib
FERROUI_XAML_BACKEND=bytecode cargo check --locked -p ferroui-themes-winui -p ferroui-themes-fluent
```

Port the additive patch in a fresh worktree of the merged branch once the frozen baseline passes.

**Verification update.** The library test run completed. test result: ok. 282 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.10s test result: ok. 473 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.90s These suites do not exercise the new schedule-specific assertions or the generated release app. Complete the full/trimmed and deferred-resource checks above.

## Windows follow-up

The integrated implementation checks the identity and kind of each generated row table. Each extracted function owns its imports. A call the compiler cannot resolve keeps the entire original run. Both generated theme crates compile on Windows. The bytecode image decoder found all 347 WinUI and 86 Fluent extracted functions encoded, with operation counts matching the input table above. [Decoded counts and image hashes](theme-schedule-windows.json)

Run the following from the repository root. The behavior driver builds each headless program with both backends. It compares sorted string keys and type-key/setter inventories, creates the real Button template, and verifies template-part identity and padding updates. Its debug builds cover resource behavior only. Measure linked bytes and paired timings in separate matched release builds.

```powershell
$env:FERROUI_XAML_BACKEND = "bytecode"
cargo check --locked -p ferroui-themes-winui -p ferroui-themes-fluent
cargo test --locked -p ferroui-markup-xaml -p ferroui-markup-xaml-loader --features ferroui-markup-xaml-loader/emitter --lib
python -X utf8 scripts/themes/full-opcode-spike/prepare.py
python -X utf8 scripts/themes/full-opcode-spike/typed_theme_probe.py behavior
```

Pass the two generated `out/xaml/compiled_xaml.rs` paths to `theme_run_coverage.py --per-run` for every extracted function's decoded instructions and named native fallbacks. Each theme's existing `build_script_output_is_the_checked_in_reference` test compares generated Rust source with its checked-in native reference. Both source assertions passed with the native backend and failed with bytecode. All 553 other executed tests in those two bytecode theme suites passed. Four tests were ignored.

All eight headless processes passed their Button checks. Native and bytecode produced identical inventories in each selection. Full WinUI yielded 104 type keys and 2,984 string keys across 166 selected units. Its 16-unit Button selection yielded 9 and 793. Full Fluent yielded 83 and 1,111 across 75 units. Its two-unit Button selection yielded 9 and 38. [Windows case inventories, selected units, hashes and test results](theme-behavior-windows.json)
