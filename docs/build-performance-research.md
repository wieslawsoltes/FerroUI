# Build and compilation performance: research and decisions

Research date: **2026-10-25**, alternatives rechecked **2026-10-26**. This document separates published results,
architectural proposals, and hypotheses that must be measured on FerroUI. It is a
survey of relevant primary sources, not a claim to have exhaustively reviewed
every compiler paper. Actual FerroUI benchmark results belong beside the command,
commit, toolchain, machine, and cache state that produced them.

## Decision

Keep upstream Cargo and rustc as the production build path. First remove measured
redundant work in FerroUI's build scripts, generated metadata, and CI invocation
graph. Evaluate caching and experimental toolchain changes independently.

The requested **10x** has since been exceeded for the two
[audited populated-local-cache rebuild workloads](build-performance-follow-up.md).
A result for a scanner, cache-hit rebuild, or a selected edit/test cycle must be
reported with that scope. It does not establish a 10x improvement in compiling
the complete workspace from an empty artifact cache; the cold WinUI comparison
showed no such improvement.

A Cargo fork is justified only by a measured Cargo bottleneck and a change that
cannot reasonably be implemented through supported Cargo interfaces or submitted
upstream. It is not a prerequisite for eliminating repeated scanning or retaining
valid compiler artifacts. The official [2026 Fast Builds roadmap][fast-builds]
distinguishes clean compilation from iterative development and calls for changes
across rustc, linkers, incremental state, and coordination.

For a measured fraction `p` of end-to-end time improved by a factor `s`, the
upper-bound speedup is `1 / ((1 - p) + p / s)`. Even eliminating all Cargo
coordination would only yield about 1.05x if that coordination accounts for 5%
of elapsed time. Conversely, finding that 95% is unnecessary repeated work makes
a large gain credible. The measurement determines which situation applies.

## Toolchain and repository baseline

At the research date, the latest official stable release is [Rust 1.99.1,
released 2026-10-15][rust-1991]. FerroUI declares Rust **1.89** as its minimum
supported version. Preserve that compatibility unless a separate change
explicitly updates it. A performance comparison must record `rustc -Vv` and
`cargo -Vv`; a moving `stable` label alone is insufficient for reproduction.

The workspace's existing configurations are material to the comparison:

| Configuration | Existing choice | Consequence for experiments |
| --- | --- | --- |
| Dependency resolver | Cargo resolver 2 | Package selection can produce different feature unions. |
| Release | Thin LTO | Do not compare a new non-LTO profile against this and attribute the difference solely to caching. |
| Distribution | Fat LTO, one codegen unit | Whole-program optimization intentionally costs more; preserve shipping behavior. |
| Browser | Size optimization, thin LTO, one codegen unit, selective package overrides | Runtime/size constraints differ from native development. |
| Emscripten | `em++` and target-specific linker flags | Native Linux linker results do not apply to browser builds. |
| Build-time metadata | Many crates run `ferroui-build-scan` through `build.rs` | Scanner cost and invalidation can be optimized without changing Rust language features. |

These facts are directly inspectable in [Cargo.toml](../Cargo.toml),
[.cargo/config.toml](../.cargo/config.toml), and the build-script crates. The
comments in the existing profiles explain the runtime and size reasons for their
choices; these are not disposable settings to improve a benchmark number.

## Measurement architecture

Cargo's stable [`--timings`][cargo-timings] reports compiler units, build-script
execution, concurrency, and dependency unblocking. A unit is an invocation, not
necessarily a unique package. Durations can overlap, so adding them does not
produce wall time. Use the critical path and the active/waiting/inactive curves
to distinguish a single expensive prerequisite from insufficient parallel work.

For a compiler-dominated unit, use the [rustc profiling workflow][rustc-profile]
in a separate diagnostic experiment. Self-profiling may require nightly and adds
overhead; it should not silently change the production performance baseline.

Measure these scenarios separately:

| Scenario | Initial state and operation | What it establishes |
| --- | --- | --- |
| Cold artifacts | Dependencies downloaded; no target artifacts or compiler-cache entries | Compilation and build-script cost, without network variance. |
| Unchanged warm build | Repeat exactly the same command and source | Cargo freshness and other fixed overhead. |
| Leaf edit | Change a real leaf implementation after warming | Typical local iteration. |
| Shared-core edit | Change a dependency used by many crates | Downstream invalidation and critical-path effects. |
| XAML/resource edit | Change an actual code-generation input | Generated-output freshness and usable iteration speed. |
| Fresh checkout, populated cache | Recreate the checkout/target directory while preserving the explicitly declared cache | CI artifact reuse; not cold compilation. |
| Test execution | Run identical compiled tests with matching features/configuration | Runtime correctness and runner overhead. |

Use the same runner, toolchain, lockfile, package/target/feature selection,
optimization, debug information, concurrency, and network policy on both sides.
Interleave baseline and candidate runs where practical. Publish raw samples and
medians, failures, and cache statistics. Include installation, restoration, and
upload time in an additional end-to-end CI metric when judging CI usefulness.

Do not count a failed or skipped build as fast. Do not present `cargo check` as a
replacement measurement for executable compilation. Do not present a no-change
cache hit as evidence about genuinely changed code. Keep correctness jobs and
benchmark jobs independently visible.

## Stable improvements that preserve Cargo

### Make generated metadata incremental at its actual input boundary

Cargo's [build-script contract][build-scripts] supports input and environment
tracking. With no `rerun-if-*` instructions, package-wide changes can rerun a
script. Watching a directory requires scanning it for modifications. This
suggests two distinct opportunities: avoid unnecessary reruns, and make necessary
reruns cheaper.

The audit found an instance of the first problem in `samples/SampleBuild/lib.rs`:
an unconditional watch on an absent optional `excluded.txt`. Cargo's
[freshness FAQ][cargo-faq] identifies missing watched files as an always-rebuild
cause. Guarding that explicit file watch while retaining the recursive directory
watch allows unchanged sample builds to stay fresh; creation, edits and deletion
remain observable. This fixes declared inputs instead of bypassing Cargo's checks.

The subsequent platform audit found the catalog watching a generated placeholder
XAML file under `OUT_DIR` as its own input. The [catalog follow-up](build-performance.md#keep-the-catalogs-generated-placeholders-out-of-its-input-watches)
retains original source watches and embeds transformed text directly in the asset
table. Simply removing the generated-file watch would be insufficient: rustc's
old `include_bytes!` would still require that deletable intermediate. The fixture
therefore checks a real rustc recompilation with the build script fresh and the
intermediate missing. The real-graph comparison separately measures the first
repeat after each population, before the baseline defect can settle.

For FerroUI, retain a complete dependency closure: source modules, included files,
manifests, target/features, relevant environment, and generated inputs. Additions
and removals matter as well as edits. Reuse parsed data only after validating
those inputs, and retain output timestamps when generated bytes are unchanged.
Cache corruption or incompatible schema must fall back to recomputation.

Cargo's own [fingerprint implementation][cargo-fingerprint] explains that hashes
and filesystem timestamps both affect freshness. Bypassing either with an
unverified timestamp rewrite can return stale output. Optimizing the scanner
inside Cargo's existing execution contract avoids that class of substitution.

### Distinguish rustc incremental state from compiler-output caching

[`sccache`'s Rust documentation][sccache-rust] requires rustc incremental
compilation to be disabled for cached invocations. Linked artifacts such as
binaries and proc-macro libraries are not cacheable through that mode. It also
warns about procedural macros performing untracked filesystem reads. Therefore,
enable it explicitly for a measured CI workload; do not replace the ordinary
local incremental-development configuration without comparative measurements.

[Swatinem/rust-cache][rust-cache] normally retains dependencies, removes
incremental output, and disables incremental compilation. Restoring its archive
does not prove that workspace compilation was reused. Workspace artifacts can be
retained through configuration, but fresh-checkout timestamps and generated
inputs still require correct invalidation.

### Avoid feature-set rebuild cascades

[`cargo-hakari`][hakari] maintains a workspace-hack crate that keeps shared
dependency feature unions consistent. Its [published explanation and
benchmarks][hakari-about] report cumulative gains around 1.7x on the evaluated
workspace; individual avoided rebuild commands can improve much more. Those
are different metrics.

For a framework shipping many crates, audit isolated package feature correctness
and publication behavior before introducing a workspace-hack. A larger shared
feature union can mask a missing dependency feature, enlarge first builds, and
affect downstream Git consumers. Consolidating compatible CI invocations is a
lower-complexity first experiment; it must preserve intended feature coverage.

### Preserve development and shipping tradeoffs explicitly

The [Cargo build-performance guide][cargo-guide] recommends reduced debug
information as an option. This can reduce code generation, linker work, and disk
usage, but also reduces debugger information. An optional fast profile is more
transparent than silently changing the meaning of the normal debugging profile.
Keep overflow checking, assertions, panic behavior, and release semantics intact
unless the user deliberately selects a profile with different behavior.

Benchmark codegen-unit counts and concurrency; neither higher nor lower is
universally faster. Oversubscription and memory pressure can negate parallelism.
Cargo's jobserver already coordinates rustc and participating build tools.

### Use the actual linker baseline

[Rust 1.90 made LLD the default][lld-default] for
`x86_64-unknown-linux-gnu`. A current stable Linux benchmark should not assume
GNU BFD is the baseline. Mold and [Wild][wild] are candidates for linker-specific
experiments using unchanged compiler settings.

At this research date Wild documents Linux and WebAssembly support, while
incremental linking, Mach-O, and Windows are future work. Its speed as a full
linker must not be described as an already available incremental-linking feature.
FerroUI needs separate native and Emscripten validation.

## Alternative build systems and infrastructure

| System | Integration model | Decision for FerroUI |
| --- | --- | --- |
| [Buck2][buck2] and [Reindeer][reindeer] | Generate Buck rules for Cargo dependencies; execute a Buck build graph | Useful remote-execution architecture, but an additional authoritative graph and integration burden. |
| [Bazel rules_rust / crate_universe][bazel-rust] | Generate Bazel Rust targets using Cargo dependency information | Preserves Rust compilation, but ordinary Cargo is no longer the sole execution model. |
| [BuildXL][buildxl] | Build engine with caching, sandboxing, and distributed execution; documented integrations include JavaScript, MSBuild, Ninja, and DScript | Valuable architecture reference. No verified ready-made Cargo integration was found in the reviewed official documentation; do not claim drop-in support. |
| [sccache distributed compilation][sccache-dist] | Keep Cargo, distribute supported compiler invocations | Requires infrastructure and artifact/input validation; network overhead and the critical path limit gains. |
| [Kache source audit](build-performance-kache-review.md) | Keep Cargo; cache eligible libraries, linked executables and optionally build-script executions | Additional binary-cache opportunity. The pinned audit documents proc-macro, native-link and build-script replay boundaries. No FerroUI result or production enablement is claimed. |
| [Hurry](https://github.com/attunehq/hurry#limitations) | Cargo-compatible build wrapper and shared cache | The documentation checked on October 26 limits support to Rust 1.92 or earlier because Cargo 1.93 removed `--build-plan`. It does not accelerate first-party crates or `test`/`run`/`install`, and Windows acceleration is unavailable. It cannot provide a replacement for the pinned Rust 1.99.1 experiments. |
| [cargo-nextest][nextest] | Cargo-compatible test execution | Speeds some test workloads, not rustc itself; stable doctests still need `cargo test --doc`. |

Remote execution can reduce elapsed time by adding machines; it does not imply
the compiler does less work. Compare costs, setup time, cache transfer, runner
resources, and reliability as well as the successful compile phase.

## Compiler and build-system research

### Cargo's upstream shared build cache

The accepted [cross-workspace cache goal][cargo-shared-cache] and its
[current design discussion][cargo-shared-design] are directly relevant to repeat
checkouts and CI. Cargo can use dependency-graph knowledge that a compiler wrapper
has to infer. The initial design conservatively targets immutable registry/git
units, excluding build-script executions and proc-macro dependents; broader
coverage and remote integration require more work. The October 2026 discussion
still concerns content-addressed storage and the overall design. This is an
upstream development path, not a verified stable replacement for FerroUI's local,
generated-code-heavy build graph. A private Cargo fork would duplicate that design
and compatibility burden without evidence of a better result.

### Relink Don't Rebuild

The accepted [RDR proposal][rdr] needs rustc to expose a compiler-visible interface
fingerprint. Generic and inlinable bodies can affect downstream compilation even
when signatures are unchanged. The proposal also identifies metadata identity
problems: added/removed items shift `DefId`s and can invalidate references in
reused downstream artifacts.

Its original Zed experiments reported up to roughly 50% faster development builds
and a selected check reducing 4.5 seconds to 0.7 seconds. The later roadmap's
5–10x is a goal for suitable iterative changes. A Cargo-only textual public-API
hash does not implement the necessary compiler contract and is not a sound
replacement.

### Parallel rustc frontend

The [2026 parallel-frontend goal][parallel-goal] and [stabilization
proposal][parallel-mcp] describe continuing work on concurrency, incremental
correctness, diagnostics, and reproducibility. The current official usage guide
still marks `-Zthreads` as nightly-only. Treat it as an opt-in experiment with a
pinned nightly and its own correctness lane. It is most relevant when one large
crate limits otherwise available parallel work.

### Cranelift

The [2026 Cranelift performance goal][cranelift-goal] reports February 2026 Zed
incremental-build gains of 5% without debug information and 12% with line tables;
it describes roughly 10–15% average per-crate gain with line tables in rustc's
benchmark suite. The project's target of a further 2x applies inside its backend.

The [backend repository][cranelift] still lists target and SIMD limitations and
restricted/experimental unwinding. It is not a proven unrestricted replacement
for FerroUI's native, browser, and release compiler matrix.

### TPDE

Schwarz, Kamm, and Engelke's [TPDE paper][tpde-paper] evaluates a compiler backend
for existing SSA intermediate representations. Its reported 8–24x LLVM-IR
compilation improvement over LLVM `-O0` on SPECint 2017 excludes Rust frontend
work and application linking. The [implementation][tpde] focuses on ELF x86-64
and AArch64.

The [Rust integration goal][tpde-goal] is experimental and plans fallback to LLVM
for unsupported cases. Its long-term example discusses around 50% end-to-end
improvement for backend-dominated debug builds. Do not transform a 10–20x backend
claim into a 10x Cargo-build claim.

### Independent Rust frontends

[mrustc][mrustc] is useful for bootstrapping rustc, but its own documentation says
it assumes valid input and is unsuitable for everyday use. It does not establish
equivalent validation of new FerroUI code. [gccrs's August 2026 report][gccrs-report]
still describes milestones around Rust 1.49 and ongoing language/library support.
Neither is a demonstrated drop-in, full-feature alternative for this Rust 2024
workspace. Compiler-backend alternatives retaining rustc's frontend should be
distinguished from replacing its language and correctness implementation.

### Slicing and monomorphization

The current [cargo-slicer research documentation][slicer] reports matched-flag
whole-workspace results of about 1.26–1.55x, including Zed at 1.43x. It explicitly
corrects earlier inflated comparisons with different linker/parallel settings.
One cached Cargo workload regressed because dispatch overhead serialized work.
This is useful experimental evidence, not a general 10x guarantee.

The [polymorphic code-generation proposal][polymorphization] explores sharing
generic machine code using hidden vtable parameters while retaining concrete
data representations. It requires compiler implementation and validation; it is
not a Cargo configuration switch. FerroUI's generic and generated code may make
this research relevant, but applicability must be profiled.

### Incremental systems and dynamic dependencies

[Incremental Systems Rethought][incremental-rethought] proposes reusing compatible
compiler work across check/build/test and other activities. Current modes are
not generally interchangeable caches. The [rustc query model][rustc-incremental]
uses dependency validation and early cutoff when recomputed results are unchanged.

Mokhov, Mitchell, and Peyton Jones's [Build Systems à la Carte][build-systems]
separates scheduling from rebuilding and discusses dynamic dependencies,
verifying/constructive traces, and early cutoff. Its useful implication here is
that cached results need complete validated inputs, and an unchanged generated
result can stop unnecessary propagation. This supports a content-validated
FerroUI metadata cache without claiming compiler-level semantic reuse.

### Content-based Cargo freshness

[Cargo's checksum-freshness tracking issue][checksum-freshness] remained open at
the research date. The September 2026 call for testing requires a recent nightly
and both settings:

```toml
[unstable]
checksum-freshness = true

[build]
fingerprint = "content"
```

The maintainers explicitly state that local build-script inputs are not covered
by this change and that hashing can slow some workloads. Keep this out of the
stable/MSRV baseline. Cargo also documents [intermediate artifact layouts as
internal][cargo-cache]; prefer structured compiler-artifact messages when locating
outputs rather than designing a new tool around incidental directory names.

## FerroUI proc-macro audit and sccache experiment boundary

The workspace has two proc-macro crates in the reviewed source:

| Crate | Compile-time external access found | Assessment |
| --- | --- | --- |
| `ferroui-view-macros` | No direct filesystem/environment reads; emits `env!("CARGO_MANIFEST_DIR")` in consumer Rust | Input transformation; the emitted environment macro is visible to rustc dependency tracking. |
| `ferroui-static-view-locator-macros` | No direct filesystem/environment reads; emits `include!(concat!(env!("OUT_DIR"), ...))` | Generated Rust is included through rustc; generator/build-script freshness still needs validation. |

This source inspection does not certify every transitive third-party proc macro
or every possible feature graph. It establishes that the specific direct
filesystem-read caveat was not found in these two implementations. Their own
proc-macro dynamic libraries remain outside sccache's documented cacheable set.

Recommended staged opt-in workload:

1. `cargo build --locked -p ferroui-build-scan --lib`: small validation graph,
   no own build script, no workspace proc macros, and explicit non-proc-macro
   `syn`/`quote`/`proc-macro2` feature configuration.
2. `cargo build --locked -p ferroui-controls --lib`: meaningful native core
   compilation, including Base and the scanner-driven metadata scripts. Use
   exactly the same default features on both benchmark legs. Build scripts must
   still execute and validate inputs; cache reuse cannot replace their contract.
3. Expand to XAML/static-view-locator consumers only after generated-file changes,
   resource edits, environment changes, and module additions/removals have passed
   invalidation tests. Audit the resolved third-party macro graph before widening
   the scope.

For both cache legs, explicitly set `CARGO_INCREMENTAL=0`, keep the same
`RUSTC_WRAPPER`, compiler and flags, and use a dedicated temporary cache directory.
Measure once with an empty sccache and once with the populated sccache after
removing only the isolated target directory. Record hit/miss/non-cacheable counts
and run matching correctness tests. Include an unwrapped baseline if quantifying
wrapper overhead. Do not disable incremental compilation for normal developers
or change global Cargo configuration as a side effect of the experiment.

## Acceptance criteria and follow-up

An optimization is ready when the measured workload completes successfully,
semantics and feature selection remain intact, relevant invalidation tests pass,
and the reported improvement is reproducible from committed commands. A result
below 10x is reported as measured; it is not hidden by changing the denominator.

For a future Cargo/rustc fork, first collect evidence of scheduler/freshness or
compiler time that repository changes cannot remove. Prototype behind an
explicit flag, keep stock Cargo available, and require differential builds,
compiler test suites, and platform coverage before making compatibility claims.
The RDR and shared-incremental-state research above provides a technically sound
direction; skipping compiler checks or accepting stale artifacts does not.

## Primary sources

[rust-1991]: https://blog.rust-lang.org/2026/10/15/Rust-1.99.1/
[fast-builds]: https://goals.rust-lang.org/2026/roadmap-fast-builds.html
[cargo-timings]: https://doc.rust-lang.org/cargo/reference/timings.html
[rustc-profile]: https://rustc-dev-guide.rust-lang.org/profiling.html
[build-scripts]: https://doc.rust-lang.org/cargo/reference/build-scripts.html
[cargo-fingerprint]: https://doc.rust-lang.org/stable/nightly-rustc/cargo/compiler/fingerprint/index.html
[sccache-rust]: https://github.com/mozilla/sccache/blob/main/docs/Rust.md
[rust-cache]: https://github.com/Swatinem/rust-cache
[hakari]: https://docs.rs/cargo-hakari/latest/cargo_hakari/
[hakari-about]: https://docs.rs/cargo-hakari/latest/cargo_hakari/about/
[cargo-guide]: https://doc.rust-lang.org/cargo/guide/build-performance.html
[lld-default]: https://blog.rust-lang.org/2025/09/01/rust-lld-on-1.90.0-stable/
[wild]: https://github.com/wild-linker/wild
[cargo-faq]: https://doc.rust-lang.org/cargo/faq.html#why-is-cargo-rebuilding-my-code
[cargo-shared-cache]: https://goals.rust-lang.org/2026/cargo-cross-workspace-cache.html
[cargo-shared-design]: https://github.com/rust-lang/cargo/issues/5931
[mrustc]: https://github.com/thepowersgang/mrustc
[gccrs-report]: https://rust-gcc.github.io/2026/09/08/2026-08-monthly-report.html
[buck2]: https://buck2.build/docs/about/bootstrapping/
[reindeer]: https://github.com/facebookincubator/reindeer
[bazel-rust]: https://bazelbuild.github.io/rules_rust/crate_universe_bzlmod.html
[buildxl]: https://github.com/microsoft/BuildXL
[sccache-dist]: https://github.com/mozilla/sccache/blob/main/docs/DistributedQuickstart.md
[nextest]: https://nexte.st/
[rdr]: https://github.com/rust-lang/compiler-team/issues/790
[parallel-goal]: https://goals.rust-lang.org/2026/parallel-front-end.html
[parallel-mcp]: https://github.com/rust-lang/compiler-team/issues/1005
[cranelift-goal]: https://goals.rust-lang.org/2026/improve-cg_clif-performance.html
[cranelift]: https://github.com/rust-lang/rustc_codegen_cranelift
[tpde-paper]: https://arxiv.org/abs/2505.22610
[tpde]: https://github.com/tpde2/tpde
[tpde-goal]: https://goals.rust-lang.org/2026/tpde.html
[slicer]: https://yijunyu.github.io/cargo-slicer/
[polymorphization]: https://goals.rust-lang.org/2026/polymorphization.html
[incremental-rethought]: https://goals.rust-lang.org/2026/incremental-system-rethought.html
[rustc-incremental]: https://rustc-dev-guide.rust-lang.org/queries/incremental-compilation.html
[build-systems]: https://www.microsoft.com/en-us/research/wp-content/uploads/2018/03/build-systems-final.pdf
[checksum-freshness]: https://github.com/rust-lang/cargo/issues/14136
[cargo-cache]: https://doc.rust-lang.org/cargo/reference/build-cache.html
