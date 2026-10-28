# Build performance

FerroUI keeps stock Cargo and rustc. This work measures cold compilation, build-script
execution, compiler-cache replay, JavaScript validation, compression reporting and test
execution separately. A speedup in one row is not a speedup of the entire workspace or CI.

## Evidence and decision

The starting point is main commit `98a56f09459c8ae02f07a2bcd235c0e87b4d44b3`.
The [historical CI audit](build-performance-ci-audit.md) and
[machine-readable baseline](build-performance-baseline.json) identify the actual runs,
job steps and compilation messages. The successful October 25 nightly CI run took about
52 minutes. Dependency caches were already hitting. Windows compiled Base and Controls
in six command/feature contexts; macOS spent substantial time executing tests after
compilation. The browser jobs also repeated shared JavaScript work and expensive exact
compression measurements.

The [research review](build-performance-research.md) covers Cargo, rustc, compiler
caches, alternative backends/linkers/build systems, feature unification, parallelism,
relink-without-rebuild and build-system research, with primary-source references.
The [Kache source audit](build-performance-kache-review.md) examines linked-executable
caching and the input-tracking limits that need validation before enabling it.
A Cargo-only fork is not justified by these measurements: compiler frontend/codegen,
native dependencies, build scripts, tests and reporting account for the expensive work.
Sound relink-without-rebuild also requires compiler metadata support. A signature-only
heuristic that ignores dependency changes would be incorrect.
The [architecture review](build-performance-architecture.md) identifies a concrete
follow-up boundary for removing Base and Markup runtime libraries from the model
compiler's host prerequisites, including the API and metadata compatibility work it
requires. That refactor is proposed, not implemented or benchmarked here.

The completed [level-2 CI results and raw-evidence audit](build-performance-follow-up.md)
exceed the **10×** target for two populated-local-compiler-cache rebuild workloads:
12.116× for development builds with optimized scanners and 10.876× for release.
The same audit records the opt-in scanner configuration's 4.4% first-build penalty
for the full WinUI theme graph. These are separate outcomes; there is no claim of
10× faster cold whole-workspace compilation. The [earlier level-3 results](build-performance-results.md)
remain attributed to their measured revision. Experiments that failed to improve
reliably were removed; see [the experimental record](build-performance-experiments.md).
Successful platform job conclusions also retain the workflow's existing optional
Windows automation failures; the follow-up audit compares all 224 case outcomes
with main and finds no changed results.

## Implemented build paths

### Keep unchanged sample builds fresh

`SampleBuild` previously emitted `rerun-if-changed` for an optional `excluded.txt`
even when the file did not exist. Cargo consequently reran the build script and
recompiled its consumer on unchanged invocations. This affected twenty of the
twenty-one samples using the helper, including the catalog's TreeDataGrid dependency.

The fix watches an existing exclusion file explicitly and retains the existing
recursive source-directory watch, which also detects creation and deletion. Cargo
still decides freshness normally. An actual SampleBuild fixture checks generated
tables and executes a dependent program after exclusion-file creation, editing and
deletion, Rust source edits and asset edits. Each change must rebuild correctly;
each unchanged candidate repeat must be fully fresh.

The performance lane measures three alternating baseline/candidate repeats of the
same `treedatagrid-demo --lib --no-default-features` graph after separately recorded
first builds. This feature selection is identical in both measured variants and
does not change production commands or coverage. `--catalog` measures the complete
desktop catalog instead when its platform dependencies are available. An unchanged
build becoming fast is distinct from faster compilation of changed source.

### Keep the catalog's generated placeholders out of its input watches

The catalog has a separate first-repeat defect. Its build script transforms XAML
using placeholder geometry, writes the result under `OUT_DIR/placeholder`, then
watches that generated file as an input. Cargo can therefore rebuild the catalog
immediately after a successful build even when no source changed. The platform
audit observed the same desktop command taking 177 seconds and then 127 seconds;
these observations motivated a controlled comparison, rather than establishing
a speedup by themselves.

The fix retains watches on the original XAML and geometry sources and embeds the
transformed UTF-8 bytes as a Rust string literal in the generated asset table.
The placeholder file remains available to the markup compiler during the build
script. It is no longer a build-script input or a rustc `include_bytes!` input.
Ordinary assets still use their existing file-backed embedding. Default branding,
runtime markup, separate assets, and document-test table placement are preserved.

The regression fixture executes the actual source functions and asset-emission
loop in small Cargo projects. It checks source and geometry edits, file creation
and deletion, ordinary binary assets, escaping and Unicode, feature modes, and
deleted generated intermediates. In particular it forces rustc to recompile a
consumer while the build script stays fresh and its intermediate remains absent;
the resulting program must still return the exact asset bytes.

The real catalog benchmark uses three independent population/immediate-repeat
pairs of `control-catalog --lib` with default features. It cleans only that
package between pairs and reports population and cleanup costs separately. This
avoids timing a third baseline build after a first-repeat defect has settled.
Every candidate repeat must have all Cargo artifacts fresh. Complete generated
Rust/metadata and first-party outputs are compared; the intentional `assets.rs`
representation difference is checked by compiling and executing a separate Rust
probe for all four asset/document tables, including test-only document bytes.
Against the PR base, the result includes both this catalog fix and its
TreeDataGrid dependency's SampleBuild fix. It is not a cold compilation result.

### Optimize the source scanner's implementation

```sh
cargo --config .cargo/build-tools.toml build --locked -p control-catalog-desktop
```

The explicitly selected config uses `opt-level=2` for four build-tool packages:
`ferroui-build-scan`, `syn`, `proc-macro2` and `quote`. It preserves all selected features, debug assertions,
overflow checks and normal Cargo dependency tracking. Package overrides apply wherever
those packages are compiled, including as build dependencies. Cargo does not load this
file automatically, and ordinary development/release/distribution/browser settings remain
available unchanged.

The initial level-3 experiment made repeated scans about 4.3× faster, with all 20
complete model, diagnostic, source-inventory, function-inventory and statistic
outputs identical. The [subsequent profile comparison](build-performance-profile-tuning.md)
selected level 2 to reduce tool compilation cost while retaining nearly the same
scan throughput. Compiling optimized tools still costs more initially than the
default unoptimized tools. The full cold-build CI comparison measures that
tradeoff; use the option where it pays back, particularly with reused tools or many
scans. Scanner execution and rustc compilation are separate measurements.

### Reuse identical Rust compilation through sccache

Install an official sccache release, then opt in with:

```sh
cargo --config .cargo/compiler-cache.toml build --locked -p ferroui-controls
cargo --config .cargo/compiler-cache.toml build --locked --release -p ferroui-controls
sccache --show-stats
```

The two optional configurations compose for repeated development/CI builds:

```sh
cargo --config .cargo/build-tools.toml --config .cargo/compiler-cache.toml \
  build --locked -p ferroui-controls
```

This also accelerates scanner execution that still happens when a newly empty
target directory requires build scripts to run. The combined CI experiment uses
the build-tool configuration for its uncached, cache-population and cache-replay
legs alike, so its ratio compares identical selected profiles.
That CI lane explicitly requires at least 10× for this populated-local-cache
workload, alongside the generated-output, invalidation and test gates. The
threshold is an acceptance check; a passing measurement must still be reported
separately from first compilation and complete CI wall time.

The native core graph is the initial audited/benchmarked scope. Stock Cargo still resolves
features, tracks inputs, runs build scripts and invokes rustc. The wrapper can reuse
eligible library compilations; linked executables and unsupported invocations still run
normally. The optional config disables rustc incremental compilation because sccache
cannot cache the same incremental invocation. Ordinary incremental Cargo may be better
for local edits; compare actual workloads. No remote cache or credentials are configured.

The benchmark distinguishes no-wrapper compilation, first cache population and populated
local replay. Installation, network cache transfer and remote upload are not included in
the replay ratio. Rust proc macros with untracked filesystem reads require separate
assessment; the two current workspace macros emit tracked Rust `include!`/`env!` or
transform tokens rather than secretly reading files.

### Build shared browser JavaScript once per batch

```sh
node scripts/browser/build-webapp.mjs build
scripts/build-browser.sh themed_view --webapp-prebuilt
scripts/build-browser.sh control-catalog-browser --webapp-prebuilt
```

The first command always installs locked npm dependencies, type-checks, lints and bundles.
Each subsequent reuse checks hashes of every webapp input, installed dependency and
emitted file, together with the validation recipe and relevant environment. It also
tracks inherited ESLint configuration, configuration discovery boundaries and the
contents of an `ESBUILD_BINARY_PATH` override, including same-path replacements
and symlink targets. A changed, missing or corrupt input/output fails reuse. Failed
validation removes the prior receipt.

Reuse is available only when those external inputs can be bounded. TypeScript
configuration must describe a leaf project with supported path-free options;
transitive `extends`/project references, custom resolver paths and inherited
`@types` packages require full validation. Executable or externally importing
custom lint configuration, external TypeScript projects, and
`NODE_PATH`/`NODE_OPTIONS` also require full validation. They still run the complete
standalone recipe successfully when the underlying tools accept them, but produce
no reusable receipt. An explicit `--webapp-prebuilt` remains strict. Ordinary
`--both` reuses the receipt when available and otherwise validates each child
normally, so custom configuration does not break standalone builds.

CI uses this explicit contract and retains all existing browser, pixel, rendering,
platform and Rust tests. The [audited CI record](build-performance-follow-up.md)
identifies its measured revision and preserves all six emitted bundle/source-map
comparisons. A changed validation contract is benchmarked again by the workflow.

### Reuse exact compression measurements

```sh
node scripts/browser/module-sizes.mjs target/browser/control-catalog-browser
node scripts/browser/module-sizes.mjs target/browser/control-catalog-browser --no-cache
```

The cache stores exact gzip level 9 and Brotli quality 11 lengths, keyed by file contents,
algorithm settings, Node/zlib/Brotli versions and platform. Records include an integrity
checksum. Missing, damaged or inaccessible entries are recomputed. Report stdout and
GitHub outputs are identical to `--no-cache`. CI transfers these small measurement records
from component sites to the combined-site report, which previously recompressed the same
modules. The cache is an optional local build artifact under `target/`.

## Reproduce the measurements

The [Build performance workflow](../.github/workflows/build-performance.yml) pins Rust
1.99.1 and sccache 0.19.0. Each benchmark preserves raw logs and individual samples.
The regular CI workflow remains responsible for full platform/browser behavior.
The Python evidence tests require Python 3.11 or newer to verify the selected TOML
configuration against the benchmark's permitted profile difference.

```sh
# Separate clean checkouts: BASELINE and CANDIDATE.
# Fetch is outside the timer; every measured command is --locked --offline.
python3 scripts/build-perf/compare.py \
  --baseline ../FerroUI-baseline --candidate . --output target/build-perf/cold \
  --package ferroui-themes-winui --repetitions 3

# Explicit experiment: only the four disclosed build-tool opt levels may differ.
python3 scripts/build-perf/compare.py \
  --baseline ../FerroUI-baseline --candidate . --output target/build-perf/cold-tools \
  --package ferroui-themes-winui --repetitions 3 --candidate-build-tools

# Repeated real sample build; first compilation and input-change controls are separate.
python3 scripts/build-perf/sample-freshness.py \
  --baseline ../FerroUI-baseline --candidate . --output target/build-perf/freshness \
  --repetitions 3

# Catalog input controls and three independent first-repeat pairs.
# These outputs must be outside both source trees; native Skia dependencies are needed.
python3 scripts/build-perf/catalog-inputs.py \
  --baseline ../FerroUI-baseline --candidate . --output ../catalog-input-evidence
python3 scripts/build-perf/catalog-freshness.py \
  --baseline ../FerroUI-baseline --candidate . --output ../catalog-repeat-evidence \
  --repetitions 3 --jobs 4

# Same driver and actual source tree, with complete semantic-output equality.
python3 scripts/build-perf/scanner.py \
  --baseline ../FerroUI-baseline --candidate . --output target/build-perf/scanner \
  --repetitions 5 --candidate-build-tools

# Identical Cargo commands and profiles; target is empty before every sample.
python3 scripts/build-perf/cache.py --source . --output target/build-perf/cache \
  --package ferroui-controls --profile release --repetitions 3

# Same optimized build tools in every leg, including the uncached baseline.
python3 scripts/build-perf/cache.py --source . --output target/build-perf/cache-tools \
  --package ferroui-controls --profile dev --repetitions 3 \
  --cargo-config .cargo/build-tools.toml

# Actual bundles and the real catalog assets, with exact output checks.
node scripts/build-perf/browser.mjs \
  --output target/build-perf/browser.json --samples 3 --site samples/ControlCatalog/Assets
```

Cold comparisons alternate revision order, force owned target/build directories, reject
compiler wrappers, verify toolchain identity and reject Cargo-fresh units. Public Cargo
JSON supplies units, feature sets and build-script output paths. Generated Rust and
`.xamlmeta` files are compared after normalizing only the checkout/target/OUT_DIR locations
that legitimately differ between copies. Cargo's documented HTML timings remain available
for inspecting the critical path; overlapping unit durations are not added into wall time.

Compiler-cache replay requires actual Rust cache hits and the same units/features/profiles,
not a Cargo no-op. A separate real Cargo fixture changes Rust source, generated input,
build-script environment, compiler-tracked environment and build.rs, executes the resulting
program, rejects an intentional compiler error and verifies restoration. The selected
FerroUI tests run after replay, including release tests in the release lane. Network fetch,
daemon setup/shutdown, target cleanup and cache storage are separately recorded.

A `--minimum-speedup` option makes a performance target explicit; its default records
honest evidence without substituting a chosen target for an observation. Measurement
results and source revisions are recorded with the PR's CI evidence.
