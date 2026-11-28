# Full-theme Button template construction

Resolving the first setter at compile time for a fresh, default `TemplateBinding` reduced the cost of a repeated Button template build by **263 ns in WinUI and 203 ns in Fluent**. It removed 7 and 6 heap allocations per build and reduced both full-theme executable sizes. A separate change that put one construction result on the stack saved 114 ns and 72 ns, with a larger executable for both themes. The two changes were measured independently.

| Full theme | Backend | Signed bytes | Median ns/build | Paired difference from stock, ns (95% interval) | Allocations/build | Requested bytes/build |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| WinUI | Native | 18,578,288 | 2,543 | −461 (−504 to −434) | 76 | 7,860 |
| WinUI | Stock bytecode | 18,384,640 | 2,999 | 0 | 83 | 8,268 |
| WinUI | Stack register | 18,417,616 | 2,887 | −114 (−144 to −104) | 80 | 7,980 |
| WinUI | Proven binding choice | 18,335,008 | 2,739 | −263 (−306 to −218) | 76 | 7,844 |
| Fluent | Native | 15,567,952 | 1,695 | −250 (−273 to −235) | 54 | 6,134 |
| Fluent | Stock bytecode | 15,653,472 | 1,947 | 0 | 57 | 6,334 |
| Fluent | Stack register | 15,669,952 | 1,868 | −72 (−85 to −65) | 55 | 6,142 |
| Fluent | Proven binding choice | 15,620,464 | 1,761 | −203 (−217 to −191) | 51 | 6,006 |

Each mode also made two reallocations per build. Frees matched allocations in each case. Requested bytes include the new requested size of each reallocation. Each paired difference is the median of differences within the 24 blocks. [Raw times, exact allocation counts, sizes and executable hashes](timings.json)

## Where the extra work came from

In both default Button templates, `ContentPresenter.Content="{TemplateBinding Content}"` crossed a runtime setter choice. The stock code created a standalone five-byte construction program for the fresh binding, boxed its output, called `ProvideValue`, boxed the value for the choice, checked for non-null, and called the binding setter. Crossing this boundary also materialized results from adjacent construction regions.

For this exact shape, the framework's `TemplateBinding::provide_value` always returns a clone of its own non-null binding reference. The patch requires a compiler-resolved binding setter as the **first** ordered choice, its exact non-null predicate, and the two emitted statements that construct and provide a default binding. The priority expression is still evaluated once before the value. The binding installs with the binding's own priority. Source property lookup precedes target property lookup. The original source position stays on the instruction. Configured bindings and other choices keep their existing dispatch. The runtime test checks property evaluation order, updates, a local override, removing the override, and resetting the source to null. [Independent default-binding patch](template-binding-choice.patch)

The compiler proved 141 such choices across WinUI and 88 across Fluent. Whole-theme construction regions fell from 1,683 to 1,403 and from 933 to 760. The actual Button factory fell from five regions to three in WinUI and from three to one in Fluent. All modes produced identical per-theme inventories including setter counts, with 104 type keys and 2,984 string keys in WinUI and 83 type keys and 1,111 string keys in Fluent. [Generated code and inventory census](coverage.json)

The separate stack-register change specializes regions with exactly one constructed register while preserving its type, lifetime, and reverse-order cleanup on errors. It covers 585 WinUI and 268 Fluent regions, including three and two real Button regions respectively. An exact-capacity vector would still allocate on their first result. Both the stock and stack-register versions made two reallocations in the complete Button build. [Independent stack-register patch](single-register.patch)

Diagnostic profiles also found time inside the interpreter after excluding calls it makes. In stock WinUI, 133 of 1,813 in-build samples (7.3%) were executing in the construction module itself, versus 107 of 1,723 (6.2%) with the proven binding choice. In Fluent those counts were 129 of 1,756 (7.3%) and 93 of 1,770 (5.3%). The allocator itself accounted for 33.6% and 38.0% of stock in-build samples. Binding installation and its callees accounted for another overlapping 18.1% and 20.8%. These single-profile percentages describe the hot paths. The paired elapsed-time runs above measure the effects. [Profile counts](profiles.json) · [Eight raw Apple sample profiles](profiles.tar.gz)

## Measurement and reproduction

Measurements used macOS arm64 and rustc 1.99.1. Every variant links the complete corresponding theme into a headless application with the project's fat-LTO dist settings. Each binary was built with plain `cargo build --offline --profile dist`, stripped with Apple's `strip`, ad-hoc signed and verified. Other developer builds ran on this machine during parts of the measurements.

Each observation is a fresh process. It looks up the actual Button ControlTheme, follows its based-on chain to the template, warms 512 times, then builds, verifies the named ContentPresenter, black-boxes and drops the result 65,536 times. Twenty-four blocks use a balanced four-mode order and alternate the order of the themes. The intervals bootstrap the paired median with a fixed seed and 10,000 resamples. Allocation counting runs in separate processes (three checks of 2,048 builds per variant) with a thread-local macOS interposer. Timed processes run without it. Every process subsequently verifies live padding changes and full resource checks. Profiles sample a separate 3-million-build run on the pre-strip executable for three seconds, reporting only samples inside `ControlTemplate::build`.

Use three fresh worktrees of `933fd46325772fd1cdec81add9d4912dda2fe931`. Copy the investigation's `scripts/themes/compact-spike` and `scripts/themes/full-opcode-spike` into each. In all three, apply the frozen five-crate baseline with the first command below. Its source digest is `1ad66d7e2956c471271ff4dd095310d4ee6891d37fcb9b25632b72d21521ffe1`. Apply only `single-register.patch` in the second worktree, and only `template-binding-choice.patch` in the third. Both patches were verified against the frozen index and their resulting sources were compared byte-for-byte with those used to build. The tests passed for both candidates (274 runtime tests each, 467 stack-register or 468 binding-choice loader tests, one existing loader test ignored).

```sh
gzip -dc scripts/themes/full-opcode-spike/typed-selector-snapshot.patch.gz | git apply -
cargo test --offline -p ferroui-markup-xaml -p ferroui-markup-xaml-loader --lib

# Use an absolute shared output directory and a separate absolute build cache per worktree.
# Run native and stock in the baseline, single in the register checkout, and fused in the binding checkout.
sh scripts/themes/full-opcode-spike/template-runtime/build.sh "$variant" "$out" "$cache"

# Once all eight binaries are present, from any of the three worktrees
clang -O2 -dynamiclib scripts/themes/full-opcode-spike/template-runtime/allocation_probe.c -o "$out/allocation_probe.dylib"
python3 scripts/themes/full-opcode-spike/template-runtime/measure.py "$out"
```

For each optional profile, start the corresponding `$out/$theme-$variant-unstripped` with `TEMPLATE_ITERATIONS=3000000`. After 1.4 seconds run `sample "$pid" 3 1 -file "$out/$theme-$variant-profile.txt"` and allow the process to finish. Running `python3 scripts/themes/full-opcode-spike/template-runtime/profile.py "$out"` aggregates the eight captures.
