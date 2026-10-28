# Validated build-performance results

## Audited revision and scope

These are completed GitHub Actions measurements of source
`34676fe53c165cd305b7f26f62e7ff1d87427669`, compared where applicable with
`98a56f09459c8ae02f07a2bcd235c0e87b4d44b3`. They were collected on October 25,
2026 and audited on October 26. The raw artifacts, individual observations,
toolchain identities, commands and output checks are available in
[performance run 40192472791](https://github.com/wieslawsoltes/FerroUI/actions/runs/40192472791).
All **10 performance jobs** and all **15 job conclusions in the corresponding
[platform run 40192472470](https://github.com/wieslawsoltes/FerroUI/actions/runs/40192472470)**
succeeded. A successful job conclusion is not a claim that every optional test
passed: the platform workflow contains existing `continue-on-error` checks. The
[later platform audit](build-performance-follow-up.md) documents the Windows
automation failures and their exact agreement with the main baseline.

The individual samples are in the artifacts of the [build-performance runs](https://github.com/wieslawsoltes/FerroUI/actions/workflows/build-performance.yml). All nine downloaded benchmark
archives matched their GitHub artifact digests. This document identifies its
measured source explicitly; a subsequent source change requires its own CI run.
The [level-2 and browser-invalidation follow-up](build-performance-follow-up.md)
records that subsequent run without replacing the historical samples below.

The requested **10× target was exceeded for two populated-local-cache rebuild
workloads**. It was not achieved for cold compilation of the WinUI theme graph,
and no whole-workspace or whole-CI 10× claim follows from these results.

## Measured outcomes

All ratios below divide the baseline median by the candidate/reuse median.
The Rust comparisons alternate independent sample order; browser measurements
perform full work before verifying reuse in each pair.

| Workload | Baseline median | Candidate / reuse median | Ratio | Pairs |
| --- | ---: | ---: | ---: | ---: |
| Controls dev compilation, populated local compiler cache, optimized scanner packages in **both** legs | 91.227 s | 7.004 s | **13.024×** | 3 |
| Controls release compilation, populated local compiler cache | 136.842 s | 12.586 s | **10.872×** | 3 |
| Controls dev compilation, populated local compiler cache, default scanner packages | 84.174 s | 14.365 s | 5.860× | 3 |
| Unchanged `treedatagrid-demo --lib --no-default-features`, optional-input watch fixed | 7.935 s | 0.240 s | **33.018×** | 3 |
| Complete source-scanner process, optimized scanner packages | 10.097 s | 2.586 s | 3.905× | 5 |
| Repeated browser JavaScript installation, validation and bundle reuse | 10.778 s | 0.342 s | 31.514× | 3 |
| Repeated exact ControlCatalog asset compression report | 27.041 s | 0.102 s | 265.072× | 3 |
| Cold WinUI theme graph, unchanged compiler settings | 120.217 s | 120.400 s | 0.998× | 3 |
| Cold WinUI theme graph, opt-in optimized scanner packages | 120.556 s | 133.103 s | **0.906×** | 3 |

The cold WinUI graph contains 54 Cargo units and 12 generated Rust/metadata
outputs. The optional scanner configuration made its first build **10.4% slower**
in this experiment. Keep that configuration explicit and measure its payback for
the intended workflow; it is not a default cold-build optimization.

The source-only scanner control was also unchanged: 9.119 s versus 9.134 s,
0.998×. Scanner source rewrites that did not improve reliably were removed, as
recorded in [the experiments](build-performance-experiments.md).

### What remains expensive in a cold build

The preserved Cargo timing page for `baseline-1` in
[`cold-build-evidence-optimized-build-tools`](https://github.com/wieslawsoltes/FerroUI/actions/runs/40192472791/artifacts/12599430699)
shows two distinct Base feature contexts and substantial rustc work:

| Compilation unit | Features | Start | Unit duration | Frontend portion | Codegen portion |
| --- | --- | ---: | ---: | ---: | ---: |
| Base | `markup-functions` | 14.83 s | 67.61 s | 34.00 s | 33.61 s |
| Base | none | 14.91 s | 55.12 s | 30.27 s | 24.85 s |
| Controls | none | 51.51 s | 56.66 s | 36.56 s | 20.10 s |
| WinUI theme | none | 96.01 s | 24.65 s | 17.26 s | 7.39 s |

These are rounded observations from one diagnostic timing page, not independent
benchmarks or additive wall-time costs. The compiler units overlap through Cargo
pipelining. The theme's build-script execution also occupies 18.11 s in that
sample. This evidence directs cold-build work toward the compiler dependency
graph, repeated host/target compilation contexts and XAML generation, rather
than assuming Cargo process scheduling is the dominant cost.

## What the 13.024× cache result proves

The measured command selects `ferroui-controls --lib`, Rust/Cargo 1.99.1,
sccache 0.19.0, four build jobs and the checked-in lockfile. The development
benchmark sets `CARGO_PROFILE_DEV_DEBUG=0`, `CARGO_PROFILE_TEST_DEBUG=0` and
`CARGO_INCREMENTAL=0` consistently. Its optional scanner configuration is
identical in uncached, population and replay legs. It changes neither the
selected features nor the framework's runtime optimization level between legs.

Each observation starts with an empty owned target/build directory. This is
compiler-cache reuse, not an already-fresh Cargo invocation:

- Every sample contains the same **31 non-fresh Cargo compilation units**.
- Every replay records **22 Rust cache hits**, **zero misses** and **zero cache
  errors**, including read/write errors.
- Commands, units, features, profiles and both generated metadata files agree.
- Source and configuration fingerprints are unchanged across the experiment.
- The dev optimized-scanner lane explicitly enforces a **10×** minimum.

| Leg | Sample 1 | Sample 2 | Sample 3 |
| --- | ---: | ---: | ---: |
| Uncached, optimized scanners | 91.227 s | 90.136 s | 92.269 s |
| Populated local cache replay | 7.004 s | 6.972 s | 7.159 s |

Initial cache population cost **97.008 s**. The default-dev and release cache
population costs were **89.185 s** and **141.367 s**, respectively. These costs
are reported separately; they are not omitted cold builds disguised as fast
first builds.

The cache ratio excludes tool installation, network restore/upload, daemon
startup/shutdown and target cleanup. The benchmark records its applicable local
overheads and cache size separately. A remote CI cache must be measured with its
actual transfer costs before applying this ratio to job wall time. Ordinary
Cargo incremental compilation remains available without the optional cache
configuration and may be preferable for real source edits.

## Correctness evidence

The dedicated native correctness job passed **9,889 tests**, with **three
existing ignored tests**. Each of the three compiler-cache configurations also
ran and passed all **4,457 selected Controls tests**, including the release lane.
The platform workflow retained its existing macOS, Windows, Linux, Android, iOS,
browser, rendering, packaging and convention checks.

The cache's independent real Cargo fixture executed nine states: population,
replay, Rust source change, generated input change, build-script environment
change, compiler-tracked environment change, build-script source change,
intentional compiler error, and restoration of valid source. It checked the
resulting executable's values, required the intentional diagnostic and Cargo
failure, and verified successful restoration. This validates those input classes;
it is not a proof about arbitrary proc macros or untracked external build tools.

The sample helper fixture executed **24 phases**, twelve for each revision. It
covered absent-file repetition, creation/edit/deletion of `excluded.txt`, Rust
source edits and asset edits, and repeated each state. All generated/runtime
outputs agreed. All six unchanged candidate repeats were Cargo-fresh, and every
real input transition rebuilt. The timed sample uses the same narrowly specified
feature selection on both revisions; it is not a timing of the entire catalog.

The scanner comparison checks **20 complete semantic outputs**: models,
diagnostics, file inventories, inherent functions and statistics across four
framework crates. Browser measurements compare all six emitted bundle/source-map
files and the complete compression report bytes.

The browser receipt audit subsequently identified external ESLint configuration
and esbuild override inputs outside the original webapp hash. Those inputs were
addressed and revalidated in the [subsequent CI run](build-performance-follow-up.md).
The historical browser timings above remain attributable to the stated source
revision.

## How to use the measured paths

To reproduce the exact observations above, use the source commit named at the top
of this report; its scanner configuration selects level 3. The current optional
configuration is retuned to level 2 after the
[separate profile experiment](build-performance-profile-tuning.md). Its CI
measurements belong to that subsequent revision, rather than replacing these
historical samples. The commands below use the configuration in your checkout.

Use normal Cargo for an already-populated target directory. The sample freshness
fix applies automatically. For rebuilding eligible libraries into an empty target
with an already-populated local sccache, install sccache explicitly and select:

```sh
cargo --config .cargo/build-tools.toml --config .cargo/compiler-cache.toml \
  build --locked -p ferroui-controls
sccache --show-stats
```

Release cache replay uses the existing release profile:

```sh
cargo --config .cargo/compiler-cache.toml \
  build --locked --release -p ferroui-controls
```

These commands do not install or configure a remote cache. The full reproduction
commands and state controls are in [the build-performance guide](build-performance.md).
The ordinary debugging, distribution and browser profiles retain their existing
behavior. Cargo/rustc remains the compiler path; no alternative backend, missing
feature or skipped correctness suite accounts for the reported ratios.

## Remaining architectural work

These measurements support eliminating repeated work through Cargo's supported
interfaces. They do not identify a Cargo-scheduler bottleneck that warrants a
private Cargo fork. A useful cold-build direction is the
[proposed model-compiler dependency split](build-performance-architecture.md):
extract shared parsing and metadata so the build-time XAML compiler does not
depend on the UI runtime merely to parse values. That work must preserve API
identity, parser behavior, generated metadata and feature combinations, and needs
its own controlled cold-build comparison. It is not implemented by this PR.

The prior nightly duration and this platform run's elapsed wall time are not a
matched A/B experiment. Runner queues, job graphs and test contexts differ.
Whole-CI acceleration remains an independently measured outcome.
