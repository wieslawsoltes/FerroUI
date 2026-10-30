# Build performance experiments

These are **local measurements from October 25–26, 2026**, not GitHub Actions results. They
motivate the opt-in configuration and the controlled CI experiments; they do not
establish a whole-project compilation speedup.

The production Cargo profiles and scanner implementation are unchanged by these
experiments. Only the optional `.cargo/build-tools.toml` configuration and the
reusable measurement driver are retained. Two speculative scanner changes were
removed after measuring them.

## Source, compiler and workload

- Baseline source: `98a56f09459c8ae02f07a2bcd235c0e87b4d44b3`.
- Compiler: `rustc 1.99.1 (58cf8f9c3 2026-10-13)`, full commit
  `58cf8f9c35b355199727627d07ae34a4e2846d67`, LLVM `23.1.1`.
- Host: `x86_64-unknown-linux-gnu`, shared Linux container, 9 visible processors.
  Other workloads were active. The range of the samples below is material.
- Cargo: stable Cargo with the checked-in lockfile, no compiler cache wrapper.
  Library compilation used `CARGO_INCREMENTAL=0`, `CARGO_PROFILE_DEV_DEBUG=0`
  and `CARGO_PROFILE_TEST_DEBUG=0`.
- Driver: [`scripts/build-perf/scan.rs`](../scripts/build-perf/scan.rs), compiled
  at `opt-level=0`, `debuginfo=0` against each scanner library. Its measured
  version has SHA-256
  `f11e3ca13cbe7731d0e257a78e75de21ea03f3f873b5fe8cbfc9605ae400a669`.
- Both binaries read the **same source directory**, with the same transitive
  dependency models. Each observation starts a new process. The order alternates
  baseline/candidate, candidate/baseline, baseline/candidate.

The driver scans actual framework source files and explicitly supplies the
dependency models listed below:

| Crate | Source files read | Types in the model | Dependency models supplied |
|---|---:|---:|---|
| `ferroui-base` | 1,452 | 446 | None |
| `ferroui-controls` | 622 | 412 | Base |
| `ferroui-markup` | 13 | 0 | Base |
| `ferroui-markup-xaml` | 73 | 51 | Base, Controls, Markup |
| Total | 2,160 | 909 | |

This is a source-scanning workload, not a replay of Cargo build-script scheduling.
In particular, Markup is included as an additional source audit: it has no model
export build script in the ordinary Cargo build. Its model is explicitly supplied
to the final scan here. The separate whole-Cargo benchmark measures the actual
build graph and build-script inputs.

For every sample, the driver writes five complete semantic records per crate:
`.xamlmeta`, diagnostics, source-file inventory with invocation counts, inherent
functions with locations, and scanner statistics. All **20 records matched byte
for byte** between baseline/candidate and across repeats in each experiment.
There is no comparison of just type counts or a success exit code.

The reported *scan seconds* sum `scan_crate` time across the four crates. Dependency
model cloning, serialization, filesystem output, process startup and shutdown are
outside those timers. *Process seconds* measure the complete driver invocation,
including those costs. Rust compilation of the scanner and its dependencies is a
separate cost.

## Retained as an option: optimize the build-tool packages

The experiment uses `opt-level=3` for exactly four packages:
`ferroui-build-scan`, `syn`, `proc-macro2` and `quote`. Every other profile setting
is unchanged. These are Cargo package overrides, so they apply to a named package
wherever it is used, including as a build dependency. They do not change the Rust
language, dependency selection, build-script tracking, debug-assertion setting or
overflow-check setting.

This initial experiment was implemented in
[`.cargo/build-tools.toml`](../.cargo/build-tools.toml). A subsequent
[October 26 profile comparison](build-performance-profile-tuning.md) retunes that
optional configuration to level 2; the level-3 observations here remain historical:

```sh
cargo --config .cargo/build-tools.toml build --locked -p control-catalog-desktop
```

Cargo does not load this file unless the caller requests it. Test builds inherit
the development settings through Cargo's usual profile rules; release,
distribution and browser profiles are unaffected.

| Pair | Execution order | Baseline scan, s | Optimized scan, s | Baseline process, s | Optimized process, s |
|---|---|---:|---:|---:|---:|
| 1 | Baseline, optimized | 12.666890 | 2.406327 | 12.922588 | 2.574816 |
| 2 | Optimized, baseline | 10.249822 | 2.538876 | 10.458037 | 2.697660 |
| 3 | Baseline, optimized | 9.304424 | 2.090483 | 9.523214 | 2.228245 |
| Median | | **10.249822** | **2.406327** | **10.458037** | **2.574816** |

The ratio of medians is **4.260× for scanning** and **4.062× for the complete
scanner process**. These are neither a 10× result nor a measurement of Cargo
compiling all FerroUI crates. The explicit cold-build CI comparison is needed to
measure the net effect on an application build.

### Compilation cost

Cargo spent **18.67 seconds** compiling the optimized graph after an unoptimized
build had already been made. It rebuilt `proc-macro2`, `quote`, `syn` and the
scanner; `unicode-ident` was already available. Registry access was offline.
This measures the cost of creating those optimized artifacts in that environment,
not a fully empty toolchain/registry or all of FerroUI.

For context, an ordinary scanner-library build with its unoptimized dependencies
already present took **2.23 seconds**. Those cache preconditions differ, so
subtracting these two numbers would not be a controlled clean-build comparison.
The optimized artifacts can be reused on subsequent builds, but a short-lived CI
job must pay for producing them unless its cache supplies them. This is why the
option is not enabled in the default profiles.

## Rejected: memoize scanner path resolution

The prototype memoized complete `(module, path)` lookups while constructing one
model from an already finalized module tree. It kept recursive import-skip and
cycle handling intact, cached unresolved names separately from absent entries,
and had no cache across scans. A new regression test covered shadowed names,
failed lookups and cyclic glob imports; all 45 tests passed.

| Pair | Baseline scan, s | Cached scan, s | Baseline process, s | Cached process, s |
|---|---:|---:|---:|---:|
| 1 | 9.051924 | 9.291728 | 9.250418 | 9.513109 |
| 2 | 9.359945 | 9.144789 | 9.583808 | 9.356784 |
| 3 | 9.182225 | 9.317442 | 9.403554 | 9.542795 |
| Median | **9.182225** | **9.291728** | **9.403554** | **9.513109** |

The scan ratio was **0.988×**, approximately 1.2% slower. Semantic equality is
necessary but does not justify extra complexity without a useful speed gain.
The cache and its prototype-specific test were removed.

## Not retained: reuse lexer tokens and avoid temporary visitor vectors

The second prototype reused lexical tokens between `syn`'s syntax parser and the
independent `ValueTypes` occurrence counter, rather than tokenizing regular source
twice. Files with a BOM, a `#!` prefix or a lexical error retained the original
`syn::parse_file` path. It also replaced two temporary collections of all path
segments with reverse iteration over their last two segments.

In Base and Controls, 132 physical source files containing `ValueTypes` total
2,892,225 bytes; this is the potential token-reuse input, not a claim that all of
those files are part of every target build. A regression test compared parse
results, shebangs, token output, source positions and error messages for ordinary
source, BOMs, shebangs, inner attributes, Unicode identifiers and invalid input.
All 45 scanner tests passed, and the 20 complete semantic records were identical.

| Pair | Baseline scan, s | Prototype scan, s | Baseline process, s | Prototype process, s |
|---|---:|---:|---:|---:|
| 1 | 9.531027 | 9.671668 | 9.787626 | 9.884720 |
| 2 | 10.138090 | 10.081924 | 10.357074 | 10.306260 |
| 3 | 10.007286 | 9.178631 | 10.221577 | 9.390783 |
| Median | **10.007286** | **9.671668** | **10.221577** | **9.884720** |

The median scan ratio was **1.035×**, but the paired ratios ranged from a
regression to about 9% improvement in a variable shared environment. This does
not demonstrate a robust improvement worth retaining as a performance change.
The source edits and their prototype-specific test were removed. The stable
source-only CI comparison is therefore intentionally a control measurement.

## Not retained: align the native build tool's Base feature context

An October 26 follow-up tested whether enabling Base's existing `markup-functions`
feature on the build tool's dependency edge would let stock Cargo share native
host and target units. The experiment recorded the exact one-line manifest patch, command, compiler/profile identities,
unit differences, generated-output hashes and selected Cargo timing observations (raw data not
kept in the repository).
Its source is `34676fe53c165cd305b7f26f62e7ff1d87427669`, with the same explicit
level-2 tool configuration on both sides, four jobs, no incremental/compiler cache,
and new empty targets. This is a different experiment from the scanner rows above.

| One completed pair | Baseline | Feature alignment |
| --- | ---: | ---: |
| Cold WinUI build | 139.382 s | 141.266 s |
| Cold Cargo compilation units | 54 | 51 |
| Generated Rust/metadata files | 12 | 11 |

Cargo removed the featureless Base build-script/library units and one duplicate
Markup unit. Normal runtime dependency features and all remaining unit profiles
matched. Unique generated outputs matched; the only removed metadata file was
an identical duplicate Base model. The WinUI build-script execution finished
earlier, but Controls metadata and downstream libraries still determined when the
final theme library could start: 115.08 s baseline versus 115.11 s candidate.

The wall-time ratio was 0.987×. One pair does not establish a statistical regression,
but it demonstrates no elapsed-time gain that justifies retaining the change.
A second pair was interrupted after that decision and contributes no observation.
An optional-feature variant was checked for default dependency-tree parity on
native, AArch64 and Wasm graphs, without cross-target compilation or performance
claims. Neither feature change is included in the production manifests. This
result also shows why fewer Cargo units alone are insufficient evidence for a
faster build or a private Cargo fork.

## Structural findings for subsequent work

The source audit found existing optimizations that must not be counted as new
work:

- [The export code](../src/FerroUI.Build.Scan/export.rs) already writes metadata
  only when its content changes and explicitly tracks the files it reads.
- [The XAML emitter](../src/Markup/FerroUI.Markup.Xaml.Loader/rust_emitter/emitter.rs)
  already splits bodies exceeding 1,024 top-level statements into parts of at most
  256 statements. Historical reports of one unsplit Fluent-theme function are not
  the current implementation.
- [The workspace profiles](../Cargo.toml) already separate thin-LTO `release`,
  fat-LTO single-codegen-unit `dist`, and the browser profile.
- [Core CI](../.github/workflows/ci.yml) already disables incremental compilation,
  generally disables development/test debug information, and avoids a redundant
  whole-workspace `cargo check` before the macOS tests.

Substantial work remains below Cargo's scheduling layer. The build tool
[depends on the complete Base crate](../src/FerroUI.Build.Tasks/Cargo.toml), which
can require separate host/target or feature variants before markup generation.
The checked-in reference outputs of the WinUI, Fluent and Simple themes are
12,230,147, 5,518,305 and 3,201,549 bytes of Rust source respectively. Their
actual generated counterparts still require parsing, type checking,
monomorphization and code generation. These observations identify hypotheses for
compiler timing and architectural work; they do not establish that changing
Cargo's scheduler can remove those costs.
