# Build-time audit of the existing CI

Observed on 2026-10-25 at main commit
`98a56f09459c8ae02f07a2bcd235c0e87b4d44b3`. The observations come from the CI runs of that commit; the raw data is not kept in the
repository.
These historical runs identify where to measure; they do not establish the speedup
of a later change. Queue time, compilation, test execution and reporting are different
quantities.

## The measured baseline

The [2026-10-25 nightly](https://github.com/wieslawsoltes/FerroUI/actions/runs/40155345452)
has successful conclusions for all 15 jobs. Its existing optional Windows
automation step reports 49 passed, 136 failed and 39 ignored; the workflow already
allows those failures. The [follow-up audit](build-performance-follow-up.md)
compares every named outcome with the candidate run. The five largest relevant
jobs were:

| Job | Job wall time | Sum of Cargo compilation messages | Other time |
|---|---:|---:|---:|
| Windows x64 | 3,103 s (51m 43s) | 2,027.38 s | 1,075.62 s |
| Browser with threads | 2,354 s (39m 14s) | 1,682 s | 672 s |
| Browser without threads | 1,988 s (33m 08s) | 1,516.59 s | 471.41 s |
| macOS workspace | 1,775 s (29m 35s) | 1,035 s | 740 s |
| macOS additional features | 736 s (12m 16s) | Not parsed | Not parsed |

The Cargo figures are the sum of the `Finished` messages in each job's log. A
Cargo command includes its dependency work, build scripts and linking; this is not
a measurement of Cargo's own scheduler or of rustc alone. The messages round some
durations to whole seconds. The remaining job time includes tests, installation,
cache transfer, shell commands, browser automation and artifact handling.

The macOS workspace build took 17m 15s. Of its 189 test-harness result lines, 188
include durations totaling 673.60 seconds; the custom iOS result has no duration.
Compilation improvements alone therefore
cannot reduce that complete job by 10x. Eliminating its compilation time entirely
would cap the speedup at about 2.40x with the other work unchanged. The equivalent
upper bounds are about 2.88x for Windows, 4.22x for the browser without threads and
3.50x for the browser with threads. These are arithmetic bounds for the measured
jobs, not predictions of achievable compiler improvements.

For comparison, the [preceding nightly](https://github.com/wieslawsoltes/FerroUI/actions/runs/40054999113)
had Windows at 2,966 s, macOS at 2,384 s, browser without threads at 1,594 s and browser
with threads at 1,992 s. Its source commit differs. The [main push at the audit commit](https://github.com/wieslawsoltes/FerroUI/actions/runs/40096131912)
had the two browser jobs at 1,959 s and 2,342 s respectively; desktop macOS, Windows
and iOS are deliberately skipped on push and exercised by PR/nightly runs. Do not
compare workflow totals without accounting for these different job sets.

## Dependency caches already hit

The audited nightly restored a 440 MB macOS Rust cache, a 412 MB Windows Rust cache
and a 617 MB non-threaded browser Rust cache. The browser also restored the Emscripten,
wasm-bindgen and npm caches. The measured work is therefore not simply a cold registry
download or a failure to cache external dependencies.

The workflow already disables incremental compilation in CI and sets development
and test debug information to zero in native jobs. Recommending those as new native
CI improvements would count existing work twice. The source comments describe an
earlier cache-eviction problem and intentionally restrict Rust-cache writes to main.
A broad additional multi-gigabyte cache should be evaluated against that storage
budget and its restore/save overhead before deployment.

## Repeated compilations on Windows

In one Windows x64 job, the log records `ferroui-base` and `ferroui-controls` compiling
six times each, and `ferroui-build-scan` four times. Distinct feature graphs and host
build-dependency graphs are plausible causes, but the human-readable log does not
prove which fingerprint changed. A measured reproduction should use Cargo fingerprint
tracing or per-invocation timing data before changing the dependency model.

The long compilation commands in that job were:

| Work | Cargo build time |
|---|---:|
| Backend examples without ANGLE | 179 s |
| Backend examples with ANGLE | 185 s |
| Desktop entry point tests | 159 s |
| Five HelloWorld samples built together | 630 s |
| Win32 integration test binary | 197 s |
| Appium automation test binary | 149 s |
| ControlCatalog for picture capture | 175 s |
| ControlCatalog for accessibility capture | 135 s |
| WinUI samples | 133 s |

The last two catalog builds use the same Cargo command. Follow-up fingerprint
analysis found a concrete freshness defect: `SampleBuild::read_excluded` watched
`excluded.txt` even when that optional file did not exist. Twenty of its twenty-one
sample consumers, including `treedatagrid-demo`, lack that file. Cargo explicitly
reports a missing watched file as dirty, then rebuilds the dependent application.
The historical second invocation starts with precisely that sample dependency,
followed by the catalog and desktop host.

The repair emits the file watch only while the file exists. The existing recursive
crate-directory watch still detects creation, editing and deletion. Actual Cargo
regression controls verify those transitions and the resulting generated values.
The new repeated-build comparison measures the real sample graph separately from
its first compilation. It retains the original Cargo commands and all correctness
checks. See [Cargo's documented missing-input behavior](https://doc.rust-lang.org/cargo/faq.html#why-is-cargo-rebuilding-my-code).

Potential approaches, in order of evidence needed:

1. Keep the software-only backend feature checks, the ANGLE feature checks and all
   runtime smoke tests. Consolidate compatible build invocations only after proving
   the intended feature graphs and generated outputs remain equivalent. Building a
   union of features is not a substitute for feature-isolation coverage.
2. Profile source scanning and XAML compilation inside the existing build scripts.
   Their build dependencies execute on the host; improving those algorithms can
   benefit many targets without changing runtime behavior or introducing another
   build system.
3. Evaluate a Cargo-compatible compiler cache on unchanged workspace crates with
   explicit input-change tests. Preserve build-script rerun rules and verify proc
   macro/include-file inputs before enabling a wrapper globally. A warm-cache rebuild
   result must be labeled as such, separate from clean compilation.
4. Stabilize only demonstrably redundant host dependency feature combinations.
   A blanket feature union or workspace-hack dependency is not automatically safe for
   platform-specific combinations, resolver behavior or publication.

## Browser builds and non-compiler work

The threaded job does not repeatedly compile the core library in its five Cargo
commands: the log shows one Base, one Controls and one Build.Scan compilation. Its
five optimized Cargo invocations took 588, 80, 212, 131 and 671 seconds. Much of the
later work is final-application code generation/linking. Cargo `--timings` plus linker
timings are needed to attribute these intervals. Replacing Cargo's process launcher
would leave the final module's LLVM, Emscripten and wasm optimization work in place.

`scripts/build-browser.sh` currently runs `npm ci`, type checking, linting and bundling
once per site, including three sites in the non-threaded job and five in the threaded
job. A future batch interface can validate and build the shared script module once,
then assemble each site, provided it preserves dependency-install integrity and the
same inputs. This is orchestration savings, not a rustc compile-time speedup.

`scripts/browser/module-sizes.mjs` computes gzip level 9 and Brotli level 11 for each
file. The nightly spends 123 s on sizes of the non-threaded catalog, 120 s on threaded
sizes and a further 105 s on the combined site. The [Pages build at the same commit](https://github.com/wieslawsoltes/FerroUI/actions/runs/40096131906)
spends another 82 + 79 + 85 s reporting the two modules and the combined site. Content
hashing and reuse of compression results could preserve the exact reported sizes
while avoiding repeated Brotli work. Reducing the compression level would change
the report's meaning and should not be counted as an equivalent measurement.

Windows UI automation also has substantial actual execution time: its 654-second
automation step includes about 165 seconds of Cargo work and roughly eight minutes
of test/runtime overhead. Faster compilation cannot remove required UI waits or
state transitions. Test optimizations need their own correctness evidence.

## Controlled validation requirements

- Compare the same immutable source commit, Rust version, Cargo.lock, feature set,
  target, profile, job count and runner class.
- Report clean-target builds, restored-cache builds, no-change builds and source-edit
  rebuilds separately. State whether dependency downloads and cache transfers count.
- Preserve the release/browser optimization settings when comparing equivalent
  output. If a faster development profile is offered, label the runtime/debugging
  tradeoff and retain the existing distribution profile.
- Run the same correctness suite on both variants; archive timing JSON, Cargo logs,
  source identity and generated-output comparisons. Include a deliberate Rust source
  edit, a XAML edit and a metadata-changing edit when validating cached build scripts.
- Use repeated runs and report individual samples as well as the median. Cross-day
  historical logs and runner queues do not establish a causal speedup.

No change to test or platform coverage follows from this audit. A 10x result should
name its workload and cache state; the historical data does not support a general
10x claim for a Cargo fork or for all of FerroUI's CI.
