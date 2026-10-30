# Scanner profile tuning

## Decision

The optional [build-tool configuration](../.cargo/build-tools.toml) now uses
**optimization level 2** for `ferroui-build-scan`, `syn`, `proc-macro2` and `quote`.
The October 26 local experiment reduced the cost of compiling those tools by
**22.24%** compared with level 3, while median scanner execution differed by
**0.72%**. Every complete scanner output remained identical.

These are explicitly selected Cargo package overrides. The default workspace
profiles, selected Rust features, assertions, overflow checks, panic strategy
and ordinary incremental workflow are unchanged. Optimized tools still cost more
to compile than level 0; select the option for a workload that reuses them or
does enough scanning to repay that cost.

This follow-up responds to the initial CI result in
[the audited evidence](build-performance-results.md): level 3 accelerated scans
but increased the first cold WinUI build by 10.4%. The new profile passed the
same CI semantic and correctness checks and the composed cache-replay **10×**
gate in the [subsequent measured run](build-performance-follow-up.md), which
records 12.116× populated-cache replay and a 4.4% cold-build penalty. The
historical 13.024× observation belongs to the earlier level-3 revision; it is not
attributed to this configuration.

## Source and method

The measurements used source commit `34676fe53c165cd305b7f26f62e7ff1d87427669` and
Rust/Cargo 1.99.1, with the lockfile/manifests/configuration/driver hashes, execution
order and equivalence checks recorded. These are **local Linux
container measurements**, with nine visible processors and four Cargo jobs,
rather than GitHub Actions results. No result from a different runner is used as
the denominator of a local ratio.

All package versions and source files are fixed. Compilation uses `--locked
--offline`, no compiler wrapper, no incremental artifacts, debug information
disabled and an empty owned target directory for every sample. The scanner
driver is the same for every level and reads a frozen copy of the same framework
sources. Tool compilation and scanner execution are timed separately.

The four package overrides are the only permitted profile differences. Every
other recorded profile field, feature selection, target kind and compilation
unit remains the same. Cargo still reads the full dependency graph and executes
its normal build scripts. The complete WinUI comparison also verifies all units
are non-fresh and compares generated Rust and metadata after only the existing
source/target/OUT_DIR location normalization.

## Scanner results

There are three independent tool compilations and five independent scanner
processes for each level. Execution order varies to reduce ordering bias.

| Optimization level | Tool compilation median | Complete scanner process median | Pure scanning median |
| --- | ---: | ---: | ---: |
| 0 | 3.582 s | 10.001 s | 9.798 s |
| 1 | 9.940 s | 2.947 s | 2.812 s |
| **2** | **11.672 s** | **2.396 s** | **2.259 s** |
| 3 | 15.009 s | 2.379 s | 2.237 s |

Level 2 costs 22.24% less to compile than level 3. Its 0.017 s median scanner
process difference is small relative to the local variation. Level 1 further
reduces tool creation cost but increases scanner execution materially, so it is
not selected for the option intended to accelerate repeated scans.

All **20 semantic files** agree across **all 20 scanner executions**: type models,
diagnostics, source inventories, inherent functions and statistics for Base,
Controls, Markup and Markup.Xaml. All twelve scanner-library builds have the same
unit kinds, selected features and other profile fields. The report retains the
individual timings, including compilation of the common driver outside scanner
execution timing.

## Complete cold WinUI build

The full `ferroui-themes-winui` graph was built six times in three pairs:

| Pair | Order | Level 3 | Level 2 | Level 3 / level 2 |
| --- | --- | ---: | ---: | ---: |
| 1 | 3, then 2 | 133.374 s | 114.512 s | 1.165× |
| 2 | 2, then 3 | 118.244 s | 117.688 s | 1.005× |
| 3 | 3, then 2 | 118.706 s | 108.152 s | 1.098× |
| **Median** | | **118.706 s** | **114.512 s** | **1.0366×** |

The median is 3.53% less elapsed time. There is substantial pair-to-pair
variation, especially the first level-3 observation. Every sample remains in the
report. The earlier two-pair ratio would have overstated the apparent improvement;
the third pair was added specifically to investigate that drift.

All **54 compilation units** were cold in every build. Features and all other
profile fields match, and all **12 generated Rust/metadata outputs** agree across
all six builds. No compilation failed and the source remained unchanged.

This is favorable evidence for the option, not a precise universal cold-build
speedup. The more repeatable benefit is reduced tool-compilation cost with nearly
unchanged scanner throughput. Whole-workspace cold compilation, source-edit
iteration and CI job wall time remain separate workloads.

## CI acceptance

The existing [performance workflow](../.github/workflows/build-performance.yml)
continues to compare ordinary cold builds, explicitly optimized cold builds,
complete scanner outputs, sample freshness and compiler-cache replay. Uncached,
population and replay legs of the composed cache experiment all use the same
selected level-2 configuration.

The evidence guard permits **only levels 0 and 2 for the four named packages** in
the explicit source-versus-optimized experiment. All other package/profile
differences fail equivalence. Its regression tests also parse the complete TOML
configuration and require exactly those four overrides, rejecting unrelated
profile settings that Cargo's JSON might not expose. The actual cache comparison
requires exact profile equality and has no profile-difference exception.

Use the current option with stock Cargo:

```sh
cargo --config .cargo/build-tools.toml build --locked -p ferroui-themes-winui

# With an explicitly installed and populated compiler cache:
cargo --config .cargo/build-tools.toml --config .cargo/compiler-cache.toml \
  build --locked -p ferroui-controls
```

Reproduction commands for the controlled CI drivers are in
[the build-performance guide](build-performance.md#reproduce-the-measurements).
To reproduce the older level-3 versus level-2 local comparison, use the recorded
source commit and the exact four package overrides and commands preserved in the
JSON report; do not compare it with a moving source revision or a different
runtime/profile configuration.
