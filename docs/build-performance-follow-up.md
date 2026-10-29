# Build-performance follow-up: level-2 tools and browser invalidation

## Measured revision

This report records **`ea26d5682540df159e42a65b78454cdd1be3997f`**, measured on
October 26, 2026, **before the subsequent ControlCatalog placeholder-input fix**.
The source comparisons use baseline
`98a56f09459c8ae02f07a2bcd235c0e87b4d44b3`. All ten jobs in
[performance run 40332610724](https://github.com/wieslawsoltes/FerroUI/actions/runs/40332610724)
and all fifteen job conclusions in
[platform run 40332610557](https://github.com/wieslawsoltes/FerroUI/actions/runs/40332610557)
are successful, subject to the existing optional-test qualifications below.
The individual samples, medians, source/toolchain identities and correctness controls are in
the artifacts of those runs; a new [build-performance run](https://github.com/wieslawsoltes/FerroUI/actions/workflows/build-performance.yml) reproduces them.

This revision follows the [historical level-3 results](build-performance-results.md)
and the [controlled level-3 versus level-2 experiment](build-performance-profile-tuning.md).
The opt-in configuration now selects level 2 for `ferroui-build-scan`, `syn`,
`proc-macro2` and `quote`. The browser receipt implementation also includes the
external-configuration and tool-input invalidation fixes. Measurements from
different jobs or revisions must not be subtracted to estimate the effect of
either change; each ratio below uses its own paired experiment.

## Outcomes

Ratios divide the baseline median by the candidate/reuse median. Each row has
three pairs, except the two scanner rows, which have five.

| Workload | Baseline median | Candidate / reuse median | Ratio |
| --- | ---: | ---: | ---: |
| [Controls dev, populated local compiler cache, level-2 tools in both legs](https://github.com/wieslawsoltes/FerroUI/actions/runs/40332610724/artifacts/12655616550) | 72.926 s | 6.019 s | **12.116×** |
| [Controls release, populated local compiler cache](https://github.com/wieslawsoltes/FerroUI/actions/runs/40332610724/artifacts/12657056695) | 138.893 s | 12.771 s | **10.876×** |
| [Controls dev, populated local compiler cache, default tools](https://github.com/wieslawsoltes/FerroUI/actions/runs/40332610724/artifacts/12656051265) | 82.972 s | 14.343 s | 5.785× |
| [Unchanged `treedatagrid-demo --lib --no-default-features`](https://github.com/wieslawsoltes/FerroUI/actions/runs/40332610724/artifacts/12655681440) | 8.184 s | 0.244 s | 33.487× |
| [Complete scanner process, level-2 tools](https://github.com/wieslawsoltes/FerroUI/actions/runs/40332610724/artifacts/12654999938) | 6.239 s | 1.635 s | 3.816× |
| [Scanner source-only control, default tools](https://github.com/wieslawsoltes/FerroUI/actions/runs/40332610724/artifacts/12655745570) | 6.129 s | 6.080 s | 1.008× |
| [Repeated browser JavaScript validation and bundling](https://github.com/wieslawsoltes/FerroUI/actions/runs/40332610724/artifacts/12654604956) | 10.313 s | 0.347 s | 29.720× |
| [Repeated exact asset-compression reporting](https://github.com/wieslawsoltes/FerroUI/actions/runs/40332610724/artifacts/12654604956) | 26.869 s | 0.096 s | 280.663× |
| [Cold WinUI graph, unchanged compiler settings](https://github.com/wieslawsoltes/FerroUI/actions/runs/40332610724/artifacts/12655877148) | 122.098 s | 123.775 s | 0.986× |
| [Cold WinUI graph, opt-in level-2 tools](https://github.com/wieslawsoltes/FerroUI/actions/runs/40332610724/artifacts/12656406041) | 95.081 s | 99.274 s | **0.958×** |

The composed dev cache lane passes its explicit **10× acceptance gate**. Cold
compilation does not: the optimized-tools candidate takes **4.4% longer** than its
paired baseline. These results establish neither 10× cold workspace compilation
nor 10× whole-CI acceleration.

The compiler-cache experiments use Rust/Cargo 1.99.1, sccache 0.19.0, four jobs,
the same lockfile and profiles within each lane, and an empty target directory
for every observation. Every replay has **31 non-fresh Cargo units, 22 Rust cache
hits, zero misses and zero cache errors**. Initial cache population costs are
76.986 s for dev with level-2 tools, 87.358 s for default dev, and 142.093 s for
release. Installation, remote cache transfer and upload are outside the replay
ratio; daemon and cleanup overheads are recorded separately. The JavaScript
pairs include the full locked install/typecheck/lint/bundle recipe before reuse;
the npm download cache is not reset.

## Correctness and platform qualifications

The [native correctness job](https://github.com/wieslawsoltes/FerroUI/actions/runs/40332610724/job/121605664692)
passes **9,889 tests**, with zero failures and three existing ignores. Each of
the three cache lanes separately passes all **4,457 selected Controls tests**.
Each also passes nine executable invalidation states, including an intentional
compile error with exit 101 and successful restoration. The sample-helper
fixture passes all 24 phases: real input transitions rebuild, and all six
unchanged candidate repeats are fresh. The comparisons preserve 20 scanner
semantic outputs, 12 cold-build Rust/metadata outputs, all six browser bundle
files and complete compression-report bytes. All **60 browser tooling tests**
pass in both the performance and source-conventions jobs, covering the revised
receipt contract.

Successful platform job conclusions do **not** mean every platform test passes.
The existing optional Windows **“Automation tests of the IntegrationTestApp
sample”** step reports **49 passed, 136 failed and 39 ignored**. All **224 named
case outcomes match the main-branch baseline exactly**, with zero changed
results. Both runs first report failure to create a native child window, followed
by a missing `NativeTextBox` element. The workflow already marks this step
`continue-on-error: true`; compare the
[current Windows log](https://github.com/wieslawsoltes/FerroUI/actions/runs/40332610557/job/121605666693)
with the [main-baseline log](https://github.com/wieslawsoltes/FerroUI/actions/runs/40155345452/job/121055454739).

Existing platform exclusions and capability limits remain visible. The macOS
workspace job reports 27,114 passes, zero failures and 105 ignores across 188
Rust-format summaries. Counts overlap other jobs and are not unique-test totals.
Windows x64 and ARM64 WGL checks fall back successfully to software when no
OpenGL 4.0/3.2 context is available. The informative WinUI comparison completes
64 comparisons with zero tool failures, while pixel differences range from
0.24% to 13.16%; it does not establish visual identity. The JSON retains each
job's individual counts, ignored-test reasons and informative status.

## The separate catalog follow-up

The same Windows job still runs `cargo build --locked -p control-catalog-desktop`
twice: Cargo reports **177 s**, then **127 s**. The repeat recompiles
`control-catalog` and `control-catalog-desktop`; it is not fresh and these two
observations are not a controlled speedup measurement. Follow-up inspection of
the [measured build script](https://github.com/wieslawsoltes/FerroUI/blob/ea26d5682540df159e42a65b78454cdd1be3997f/samples/ControlCatalog/build.rs)
identified another input-tracking defect: placeholder substitution writes
`$OUT_DIR/placeholder/App.xaml`, then the asset loop watches that generated file
as an input. The first repeat can therefore rerun the build script that created
it.

The subsequent fix embeds transformed UTF-8 text directly in the generated Rust
asset table, preserves the original XAML/geometry input watches, and keeps the
placeholder file as build-script work data. Removing only its watch would leave
rustc dependent on a deletable intermediate through `include_bytes!`. This fix
and its independent population/first-repeat measurements require their **own
source revision and CI evidence**. No catalog speedup is attributed to the
earlier runs documented here. A subsequent comparison against main includes
both the earlier `SampleBuild` fix and the catalog fix; a comparison against
`ea26d568` isolates the latter.
