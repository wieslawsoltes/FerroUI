# Compiled XAML themes

The native Rust backend uses XamlX to resolve types, overloads, namespaces and includes at build time. The Rust backend used to emit a separate function for each deferred alias or brush, even when only its key, color and source position differed. The WinUI baseline at `df8b92d9` contains 14,421,918 bytes of generated Rust.

## Broad Rust source refactor

The continuation of PR #496 targets inherited virtual slots, generated resource operations, XamlX ownership, decoder errors, nullable and reference registration, typed binding setup, and property and reflection installation. The public generic APIs remain the entry points. The [Rust architecture report](compiled-theme-rust-architecture.md) records the current source candidate, full executable size, source and profile identities, API review, correctness checks, and runtime comparisons. Its [raw evidence](benchmarks/compiled-theme-rust-idioms-linux-x86_64.json) keeps each rejected and repaired build separate.

The measurements below retain their original revisions and build policies. In particular, the compact-unwind reference is the 28,087,224-byte executable used to assess the source refactor; the earlier 28,324,712-byte census does not describe the new candidate.

## Compact Linux x86-64 reference before the source refactor

The full editable WinUI consumer measures **28,087,224 bytes** with `--use-opt-level z --compact-unwind`, down 237,488 bytes (0.84%) from the previous best. The optional link reduces unwind metadata while preserving the reference addresses and bytes of all executable code and application data. The original ordinary release was 54,577,848 bytes. The measured build is 48.54% smaller. [Reproduction, performance and archive verification](compiled-theme-size-architecture.md#smaller-unwind-records-with-the-original-code-layout)

## Prior source trials against the 28,324,712-byte reference

The baseline for this round is the **28,324,712-byte** fully editable Linux x86-64 release. Another 30% requires a file of at most **19,827,298 bytes**. The final rebuild reproduced the original release byte for byte (SHA-256 `459ea34851eb89e3556a9539da29b3bcbc7fa5a8af34ee2c2291f393dae85f21`). Three independently linked, full-featured candidates missed the combined size and runtime-performance goal.

| Candidate | Stripped executable | Saving from the 28,324,712-byte reference | Performance result against the reference |
|---|---:|---:|---|
| Shared observer and compiled binding lifecycle | 27,534,728 B | 789,984 B (2.79%) | Boxed style delivery and read took 39.8% longer |
| Typed delivery restored, shared installation and allocation identity | 27,751,768 B | 572,944 B (2.02%) | Boxed style delivery and read took 36.7% longer |
| Batched required-type lookup only | 28,187,888 B | 136,824 B (0.48%) | The full-interpreter fallback took 6.8% longer in a 19-pair CPU-7 repeat. An eight-pair CPU-6 run was inconclusive |

The first two builds passed 338 property and binding cases. The isolated XAML trial reproduced the original Base library and all 45 transitive library or host-macro inputs byte for byte. Eight archive checks and panic cleanup passed on all three full builds. The experiments remain available at their pinned commits. [Architecture analysis, comparison and reproduction](compiled-theme-size-architecture.md) · [Full application measurements](benchmarks/compiled-theme-architecture-comparison-linux-x86_64.json)

## Whole-graph profile-guided release

The size target for this full editable WinUI application is **27,288,924 bytes**, half the 54,577,848-byte original shipping main at `df3e12ad`. All candidate builds use the same runtime source at `ff5ea13a`, all 169 WinUI documents, the external FXP loader and the interpreter for structural edits.

| Final release on x86-64 GNU/Linux | Stripped bytes | Reduction from original shipping main | Bytes over target |
|---|---:|---:|---:|
| Original main, compact FatLTO | 38,600,504 | 29.27% | 11,311,580 |
| Editable, compact FatLTO | 29,194,408 | 46.51% | 1,905,484 |
| Editable, compact FatLTO with PGO, `opt-level=s` | 30,152,576 | 44.75% | 2,863,652 |
| Editable, compact FatLTO with PGO, `opt-level=z` | **28,324,712** | **48.10%** | **1,035,788** |

With profile-guided optimization (PGO) at `s`, machine code shrank by 214,720 bytes, while unwind and exception tables grew by 1,139,024 bytes. The executable gained 958,168 bytes overall. Switching the whole target graph to `z` produced the smallest executable, 869,696 bytes below the previous compact FatLTO build and 26.62% smaller than compact FatLTO main. The `z` binary contains 18,592,421 bytes of machine code and 6,060,176 bytes of unwind and exception data.

Each timing trial rotated the PGO candidate and both unprofiled controls through 81 fresh processes per build. Times end at the first typed Button lookup, before window creation. Each cell gives the median / p95. Compare builds within a row.

| Trial | PGO | Same source, compact FatLTO | Original main, compact FatLTO |
|---|---:|---:|---:|
| `opt-level=s` | 8.86 / 10.07 ms | 10.60 / 12.02 ms | 15.99 / 16.93 ms |
| `opt-level=z` | 7.20 / 10.97 ms | 8.26 / 10.70 ms | 11.82 / 16.12 ms |

After executable-page eviction, the `z` trial medians were 16.30 ms with PGO, 17.62 ms on the same source without PGO and 24.87 ms on compact FatLTO main (31 processes each).

All eleven property workloads were faster at `s` than on unprofiled compact FatLTO main. At `z`, ten were faster, including style install/read/dispose (18.5% less thread CPU time), style binding emission/read (30.3% less) and mixed reads/writes (18.8% less). Scoped metadata lookup measured 2.5% slower, with a paired 95% interval from 1.5% faster to 7.0% slower. Against the original compact ThinLTO property fixture, ten improved and the interval for scoped metadata also included equality. Against the already-optimized unprofiled candidate, `z` took 13.4% longer for scoped metadata lookup and 7.6% longer for cached override lookup. All workload checksums matched in 72 paired blocks per trial.

Both PGO builds passed all 299 public property API tests, an unwinding test covering the panic payload, nested destructor order and store reuse, and all eight archive checks including externally edited FXP v1/v2/v3. In the `z` trial the median scalar edit, structural edit and full fallback took 9.54 ms, 8.18 ms and 1.52 s, compared with 10.38 ms, 9.16 ms and 1.70 s on the unprofiled candidate.

Training ran the embedded startup probe, Button, NavigationView and ScrollViewer template activation, eleven property workloads, a scalar FXP edit and a changed archive requiring the interpreter. The merged profile contained executed definitions from Base (10,665), Controls (3,794), XAML markup (381), the XAML loader (890) and WinUI (215). The shipping builds restore the original application entry point and apply that profile to the target dependencies and final FatLTO application. Reusing the `s` training profile at `z` produced 3,791 control-flow hash mismatch diagnostics. LLVM discarded the corresponding counts. Inspection confirmed applied hot and cold counts in Base, Controls, the XAML loader and WinUI at both settings. Both builds retain panic unwinding, PIE, full RELRO, eager binding and packed relocations.

To rebuild from the repository root, use Python 3.11+, native x86-64 GNU/Linux, LLD, a RELR-capable loader (glibc 2.36+) and a matching `llvm-profdata` (`rustup component add llvm-tools-preview`). The driver records coverage and writes a stripped executable for each optimization level.

```sh
work=$(mktemp -d)
python3 scripts/themes/bench/measure.py --source "$PWD" --work-dir "$work/reference" \
  --solid --compact-app --fat-app-link --linux-safe-icf --linux-packed-relocations --rebuilds 0
python3 scripts/themes/bench/profile_winui.py --source "$PWD" --consumer "$work/reference/consumer" \
  --work-dir "$work" --use-opt-level s --use-opt-level z
```

[PGO inputs, ELF sections, compatibility checks, startup and property samples](benchmarks/compiled-theme-pgo-linux-x86_64.json)

## Whole-program release after syncing main

At `ff5ea13a`, the complete editable WinUI executable on x86-64 GNU/Linux is **29,194,408 bytes** under compact FatLTO. The comparison keeps the original shipping main at `df3e12ad` as the fixed 54,577,848-byte reference. The candidate is **46.51% smaller** and is **1,905,484 bytes above** the 27,288,924-byte target. With compact FatLTO on both revisions, the reduction is **24.37%**. The PR includes the runtime changes from main through `8aa7ca2b`.

| Final compiler and linker policy | Original main native theme (bytes) | Full editable candidate (bytes) |
|---|---:|---:|
| Ordinary ThinLTO | 54,577,848 | 43,376,296 |
| Compact ThinLTO | 44,441,808 | 34,965,128 |
| Ordinary FatLTO | — | 34,295,672 |
| Compact FatLTO | 38,600,504 | 29,194,408 |

The compact FatLTO binary retains 20,307,877 bytes of machine code, 5,256,232 bytes of unwind and exception data, 74,323 frame descriptions and 29,040 bytes of dynamic relocation tables. Recompiling the WinUI Rust unit took 25.78 s in this dependency-warm run. The final whole-program compile and compact link took approximately 316 s. All 894 FXB blocks, four generated Rust files and the FXP archive matched the preceding PR build byte for byte. Each of the four executables passed eight checked cases covering embedded and independently supplied resources, including three archive versions and structural edits.

| First typed Button lookup, compact FatLTO | Median | p95 | Median with executable pages evicted |
|---|---:|---:|---:|
| Original main | 14.22 ms | 19.29 ms | 31.00 ms |
| Pre-sync editable checkpoint | 8.60 ms | 11.98 ms | 21.06 ms |
| Synced editable candidate | 9.85 ms | 14.34 ms | 21.21 ms |

These are 81 rotating fresh processes per configuration with 31 additional verified executable-page evictions. Startup includes theme construction and lookup and ends before window creation. The candidate was faster than original main and slower than the pre-sync editable checkpoint. A scalar edit took 12.93 ms, a structural edit to an independently replaceable dictionary took 10.85 ms, and an edit requiring the full interpreter took 1.86 s.

The separate property fixture passed 299 public API tests. In a 72-block four-way trial, all eleven FatLTO candidate workloads were faster than original shipping ThinLTO. At equal FatLTO, eight improved and the intervals for mixed reads/writes and style-binding emission included parity. Style install/read/dispose took **7.4% longer** (paired 95% interval 6.3–9.2%). The property fixture itself shrank from 11,366,632 to 8,852,056 bytes under equal FatLTO.

A separate headless ThinLTO probe created controls, applied their real styles and templates, traversed the visual tree and released it. Across 61 paired processes, median first activation fell from 663 to 443 µs for Button, 758 to 563 µs for ScrollViewer, 1,690 to 1,480 µs for NavigationView and 680 to 568 µs for a held-out Slider. Visual counts were 2, 8, 26 and 2 on both revisions and on an externally edited FXP. Repeated Button activation's paired interval included equality. [Activation workload and samples](benchmarks/compiled-theme-whole-program-attribution-linux-x86_64.json)

An application can request the same final compiler policy from the repository root. The selected target keeps the distribution linker flags off host-side build tools. The compact configuration uses an ELF loader with RELR support, such as glibc 2.36 or newer.

```sh
cargo --config .cargo/compact-linux-x86_64.toml rustc \
  --profile compact-release --target x86_64-unknown-linux-gnu \
  -p hello-world --bin hello-world -- -Clto=fat -Ccodegen-units=1
```

[ELF hashes and sections, complete archive checks, compile times, startup distributions and paired property measurements](benchmarks/compiled-theme-whole-program-linux-x86_64.json)

## Complete editable WinUI after sharing property dispatch

The full editable Linux x86-64 consumer at `e2e681c1` includes the complete typed theme, external FXP loading and the structural-edit fallback. The original ordinary main executable at `df3e12ad` was 54,577,848 bytes, giving a 50% budget of **27,288,924 bytes**.

| Full-theme consumer | Ordinary ThinLTO (bytes) | Compact ThinLTO (bytes) | Compact final FatLTO (bytes) |
|---|---:|---:|---:|
| Original main | 54,577,848 | 44,441,808 | — |
| Previous editable, `71d44ae8` | 45,249,656 | 36,361,272 | — |
| Editable, `e2e681c1` | 43,163,008 | 34,759,256 | 29,116,824 |

With the same compact ThinLTO settings, the editable consumer is **1,602,016 bytes smaller** than the preceding PR checkpoint and **21.8% smaller** than original main. Retaining the same dependency bitcode and using one final FatLTO unit saves a further 5,642,432 bytes. That experiment is 46.65% smaller than ordinary main and exceeds the absolute budget by 1,827,900 bytes. It preserves panic unwinding, PIE and eager binding. The unwind, search-index and landing-pad tables together occupy 6,954,348 bytes under compact ThinLTO and 5,233,256 under FatLTO.

| Fresh process to first typed Button lookup | Median | p95 | Median after executable pages were evicted |
|---|---:|---:|---:|
| Original main, compact ThinLTO | 17.65 ms | 23.70 ms | 33.39 ms |
| Editable, compact ThinLTO | 12.40 ms | 15.35 ms | 24.21 ms |
| Editable, compact final FatLTO | 11.10 ms | 13.53 ms | 19.41 ms |

Each configuration ran in 81 rotating fresh processes, with 31 additional launches after executable-page eviction. Readiness includes theme construction and the first deferred Button resource lookup. The probe ends before creating a window. The FatLTO final compile and link took 329 seconds on this machine.

At registration, bindings now retain the observer adapter for their actual source representation. Partial virtual overrides copy the parent table once, then replace the slots a class implements. Property metadata retains its typed values and shares a non-generic owner-to-slot lookup across all value types. A 96-type metadata fixture fell from 2,700,464 to 2,390,936 bytes. An independent inheritance model matched 589,824 queries during registration and removal. All 894 FXB blocks, the 296,347-byte FXP and the 4,921,299-byte generated typed theme source matched the preceding checkpoint byte for byte.

The tests covered 299 public property, conversion and binding cases and 35 dispatch and registration cases. The compatibility probe resolved typed resources in seven cases spanning the embedded theme, three FXP versions, scalar edits, a structural edit and the full fallback. A paired public API benchmark uncovered a remaining regression. In 102 blocks, style-binding emission followed by a read took **3.7% longer** with editable ThinLTO than on original main (95% interval 2.5–5.6%). Compared with editable ThinLTO, FatLTO improved mixed property access from 30.04 to 26.41 ns and binding emission from 158.65 to 139.47 ns. Plain reads increased from 7.77 to 8.40 ns and effective-local reads from 7.39 to 8.14 ns. FatLTO remains an explicit benchmark option.

The following command measures the complete editable build with the experimental final link and checks the absolute size budget. Omit `--fat-app-link` for the compact ThinLTO measurement. The byte gate is expected to fail for both recorded builds.

```sh
python3 scripts/themes/bench/measure.py --work-dir /tmp/ferroui-winui-fat \
  --solid --compact-app --linux-safe-icf --linux-packed-relocations \
  --fat-app-link --rebuilds 0 --launches 81 --max-executable-bytes 27288924
```

[ELF sections, hashes, archive checks, startup samples and property workloads](benchmarks/compiled-theme-binding-dispatch-linux-x86_64.json)

## Complete WinUI after sharing property, reflection and template code

The measured production revision is `71d44ae8`. The editable executable includes every WinUI theme resource, the typed compiled path, the external FXP reader and the fallback for structural edits. Process readiness means completion of the first deferred Button theme lookup in a fresh process. These are stripped ELF files for x86-64 GNU/Linux.

| Full-theme configuration | Ordinary ELF bytes | Compact ELF bytes | Compact startup median / p95 (ms) | Generated Rust bytes | XAML build script (s) | Theme rustc unit (s) |
|---|---:|---:|---:|---:|---:|---:|
| Main `df3e12ad` | 54,577,848 | 44,441,808 | 15.10 / 21.37 | 14,421,918 | 29.20 | 76.37 |
| Previous editable `58b32389` | 47,817,336 | 37,920,960 | 11.30 / 16.77 | 5,074,619 | 13.97 | 26.60 |
| Native Rust | 42,660,296 | 33,930,112 | 9.53 / 12.99 | 5,534,757 | 16.80 | 33.92 |
| Typed FXB | 42,286,936 | 33,838,616 | 11.07 / 14.24 | 4,921,299 | 10.16 | 21.27 |
| Editable FXP | 45,249,656 | 36,361,272 | 11.55 / 15.53 | 4,921,299 | 12.45 | 28.69 |
| Editable with FXU enabled | 45,297,760 | 36,405,016 | 11.14 / 15.62 | 4,921,299 | 13.14 | 22.07 |

The compact link uses address-significant safe identical-code folding and RELR. It retains PIE, unwinding and ordinary Rust panic cleanup. Its distribution configuration targets glibc 2.36 or newer. The ordinary link uses the same application objects and supports the earlier loader baseline.

| Editable ELF component | Previous compact | Current compact | Reduction |
|---|---:|---:|---:|
| Entire executable | 37,920,960 | 36,361,272 | 1,559,688 |
| Machine code | 25,852,453 | 24,790,325 | 1,062,128 |
| Read-only constants | 1,724,595 | 1,724,771 | -176 |
| Relocatable read-only data | 2,088,520 | 1,881,784 | 206,736 |
| Global offset table | 544,688 | 512,064 | 32,624 |
| Frame descriptions | 5,617,076 | 5,409,724 | 207,352 |
| Frame search index | 948,044 | 907,468 | 40,576 |
| Exception landing-pad tables | 893,696 | 887,196 | 6,500 |
| Combined unwind and exception data | 7,458,816 | 7,204,388 | 254,428 |
| RELA and RELR dynamic relocations | 44,312 | 40,512 | 3,800 |
| Frame description count | 118,504 | 113,432 | 5,072 |

All section sizes in the table are file bytes. At the ordinary link, the previous PR baseline at `0257f6b6` used 5,796,456 bytes for dynamic relocations. The current source brings the ordinary table down to 5,208,144 bytes. The compact linker policy brings that to 40,512 bytes.

The compact image contains 190,891 relative pointer fixups, down from 210,952. RELR encodes these using 4,758 eight-byte words. The dynamic relocation byte totals exclude a separate 48-byte PLT table. Counts by source section and pointed-to function are included in the raw report.

Two further compiler invocations per revision, pinned to the same two CPUs in main/candidate/candidate/main order, took 75.37 s on main and 27.89 s on the current editable theme. This isolates compiling generated WinUI Rust with prebuilt dependencies. The table gives the separately observed build script and compiler units. Neither measurement includes a cold workspace or native application link.

The startup table uses 41 rotating launches per configuration. With executable pages evicted and residency checked, the editable compact median was 32.60 ms versus 39.27 ms on main. The preceding compact build reached Button in 11.30 ms warm and 29.07 ms with pages evicted, compared with 11.55 and 32.60 ms on the current build. Warm p95 improved from 16.77 to 15.53 ms. A leaf FXP edit reached Button in 16.36 ms. A 751-byte FXU changed merged and compact-density values in 11.86 ms. Delivering edits to a merged dictionary through a complete replacement FXP used the fallback and took 1.78 s. Headless readiness excludes window creation and first paint.

All 894 FXB images are byte-identical to the preceding generator and total 180,091 bytes. The complete reloadable FXP is 296,347 bytes. The compatibility suite checked existing version 1, 2 and 3 FXP images, a structural edit, a matching archive, and a revision-bound FXU. The native and binary inventory covered 7,599 entries. [Machine-code and unwind attribution](ported-runtime-code-size.md) · [Raw samples, ELF sections, toolchain, allocations and checks](benchmarks/compiled-theme-property-reflection-linux-x86_64.json)

### Explicit resource roots

A separate typed-FXB application requested Button plus the compact-density styles its constructor requires. The same public constructor resolved the deferred Button control theme and verified its target type. Under compact linking the executable fell from 33,838,616 to 31,067,568 bytes. Ordinary links were 42,286,936 and 39,340,704 bytes. Generated Rust fell from 4,921,299 to 701,051 bytes, referenced FXB blocks from 894 to 137, and embedded FXB bytes from 180,091 to 25,912. The theme compiler unit took 21.27 and 4.03 s. Three unrelated theme style keys were absent from the rooted executable. Across 41 interleaved launches after builds completed, compact median / p95 fell from 10.68 / 13.21 ms to 6.42 / 8.92 ms. This mode serves the declared resource graph and its dependencies. Applications that accept arbitrary runtime resource names can use the full theme figures above.

### Compiler outlining experiment

The same final application and dependency bitcode were recompiled with LLVM 23 machine outlining enabled at two benefit thresholds. These trial executables also used safe ICF and RELR and passed embedded, external leaf and modified merged-resource checks.

| Final code generation | ELF bytes | Machine code bytes | Combined unwind bytes | Frame count | Startup median / p95 (ms) |
|---|---:|---:|---:|---:|
| Shipping | 36,361,272 | 24,790,325 | 7,204,388 | 113,432 | 11.55 / 15.53 |
| outline-48 | 34,517,560 | 22,484,453 | 7,623,760 | 116,912 | 11.36 / 15.44 |
| outline-192 | 35,251,520 | 23,323,637 | 7,493,044 | 115,711 | 10.98 / 19.00 |

Threshold 48 removes repeated machine instructions and creates additional callable fragments. Its smaller executable has 419,372 more unwind and exception bytes than the shipping compact image. These LLVM implementation flags are recorded as experiments. The shipping figures above use the checked-in compact profile.

The experiment added the following options to the final application rustc invocation. The recorded compiler was rustc 1.99.0 with LLVM 23.1.1.

```text
-Cllvm-args=--enable-machine-outliner=always
-Cllvm-args=--outliner-benefit-threshold=48
```

## Earlier whole-executable measurement at `58b32389`

The complete editable WinUI consumer embeds all 169 XAML documents and retains the external FXP loader. It constructs the theme in a fresh process and resolves a deferred, typed Button control theme. These measurements include Base, Controls, Dialogs, XAML runtime, interpreter and theme. Window creation, rendering and first paint are outside the probe.

| Final link on x86-64 Linux | Previous editable PR, bytes | Current editable PR, bytes | Source savings at the same link policy |
|---|---:|---:|---:|
| Ordinary | 49,664,944 | 47,817,336 | 1,847,608 |
| Safe identical-code folding | 44,695,408 | 42,941,800 | 1,753,608 |
| Packed relative relocations | 43,919,480 | 42,151,984 | 1,767,496 |
| Safe folding and packed relocations | 39,600,120 | 37,920,960 | 1,679,160 |


For each revision, LLD linked the same ThinLTO-generated objects four ways under `opt-level=s`, 16 codegen units and Rust panic unwinding. These ordinary ELFs were stripped at link time. The earlier 49,664,936-byte result for the previous PR used GNU `strip`.

| ELF data | Previous ordinary | Current ordinary | Current safe folding and RELR |
|---|---:|---:|---:|
| Machine code, `.text` | 28,530,117 | 27,089,685 | 25,852,453 |
| Frame descriptions, `.eh_frame` | 6,376,268 | 6,247,084 | 5,617,076 |
| Frame lookup index, `.eh_frame_hdr` | 1,150,348 | 1,134,836 | 948,044 |
| Language-specific exception data, `.gcc_except_table` | 2,444,252 | 2,312,252 | 893,696 |
| Per-frame description entries, count | 143,792 | 141,853 | 118,504 |
| `.rela.dyn` and `.relr.dyn` | 5,796,456 | 5,715,888 | 44,312 |

The modern link retains 2,448 bytes of ordinary dynamic relocations and represents relative fixups in a 41,864-byte RELR table. A separate `.rela.plt` occupies 48 bytes. Source sharing and safe folding remove functions and their associated unwind records. The remaining frame records describe how to restore registers and the stack for live machine code. A compact-profile smoke executable verified a caught panic payload, six RAII guards unwinding in order, and distinct addresses for equal-valued static objects.

### Readiness and build cost

Main was measured at `df3e12ad`, the previous PR at `0257f6b6`, and the candidate runtime at `58b32389`. All fast lanes had 41 rotating fresh-process launches on one CPU. Every process resolved Button before emitting its readiness marker. The separate file-eviction experiment used 17 launches per lane and checked executable and supplied archive page residency with `mincore`. Shared libraries and device caches stayed warm.

| Build | Stripped ordinary bytes | Modern Linux bytes | Median / p95, ordinary (ms) | Median / p95, modern (ms) | Generated Rust bytes | XAML build script (s) | WinUI rustc unit (s) |
|---|---:|---:|---:|---:|---:|---:|---:|
| Current main, native | 54,577,848 | 44,441,808 | 16.45 / 21.73 | 15.79 / 22.32 | 14,421,918 | 29.20 | 76.37 |
| Native Rust | 45,231,872 | 35,491,312 | 11.39 / 17.21 | 10.20 / 12.82 | 5,688,077 | 10.60 | 26.91 |
| Typed FXB | 44,858,192 | 35,400,200 | 11.65 / 17.74 | 10.89 / 13.84 | 5,074,619 | 11.70 | 24.43 |
| Full editable FXP with typed execution | 47,817,336 | 37,920,960 | 12.61 / 15.51 | 10.94 / 14.01 | 5,074,619 | 13.97 | 26.60 |
| Full editable with FXU enabled | 47,862,632 | 37,962,896 | 12.48 / 15.90 | 11.22 / 16.34 | 5,074,619 | 14.28 | 28.75 |

The build-script and rustc columns are individual Cargo unit observations. The final application ThinLTO phase is separate. Two additional matched rustc invocations per revision on the same two CPUs took 74.52 s on main and 38.93 s on the editable PR, a 47.8% reduction. These observations exclude a cold build of all workspace dependencies.

With executable pages evicted, the ordinary editable build reached Button in 31.85 ms versus 44.20 ms on main. On the modern link those medians were 28.84 and 38.02 ms. A modified independent FXP dictionary reached Button in 14.98 ms with a warm file cache. A 751-byte FXU changed a merged value and a compact-density value in 11.92 ms. A complete FXP editing merged resources still required an interpreter rebuild, measured at 1788 ms median.

The FXB encoder references both ends of an adjacent resource name within its own block. It selects v3 only when this reduces total bytes. All 7,354 WinUI records matched after decoding, including import slots, numeric bits and four source positions. Of 894 independent blocks, 532 use the new spelling. Total FXB size fell from 211,373 to 180,091 bytes. In 300 rotating codec samples the old reader and images took 0.678 ms median for the complete corpus, the new reader on old images took 0.565 ms, and the new reader on new images took 0.619 ms. Old images retained the same 702,966 bytes of decoded buffer capacity. The new blocks retained 709,692 bytes. V3 blocks require the updated runtime.

A headless feature comparison matched 7,599 resource entries across native and FXB, including variants, keys, concrete types and representative values. Both instantiated the Button template. Direct exported builders were also checked with no enclosing service provider and with nested scopes. The corrected metadata design keeps cache hits typed. In a distinct 96-block balanced CPU benchmark, overridden reads across multiple host classes cost 14.02 to 14.18 ns per operation. The 75% read / 25% write workload cost 27.14 to 28.77 ns. These are measurable steady-state tradeoffs. [Machine-code families and next experiments](ported-runtime-code-size.md) · [Raw ELF, build, startup and property measurements](benchmarks/compiled-theme-runtime-sharing-linux-x86_64.json)

## Building a smaller Linux application

For an x86-64 GNU/Linux distribution targeting glibc 2.36 or newer, use the compact Cargo configuration with the shipping profile. Run this command from the repository root.

```sh
cargo --config .cargo/compact-linux-x86_64.toml build \
  --profile compact-release --target x86_64-unknown-linux-gnu \
  -p hello-world --bin hello-world
```

The application still uses Rust panic unwinding. LLVM emits an address-significance table for the linker. LLD's safe identical-code folding can then share byte-equivalent functions and exception data while honoring significant addresses. The ELF loader reads a bitmap for relative relocations. The explicit target keeps build-time XAML tools on the host configuration. [LLVM address-significance tables](https://llvm.org/docs/Extensions.html#sht-llvm-addrsig-section-address-significance-table) · [glibc 2.36 release notes](https://lists.gnu.org/archive/html/info-gnu/2022-08/msg00000.html)

For older GNU/Linux loaders, use safe folding with ordinary relocations. With ThinLTO, the backend that emits the final Rust code runs during the application link. This invocation reuses the dependency build.

```sh
cargo rustc --profile compact-release -p hello-world --bin hello-world -- \
  -Cllvm-args=--addrsig -Clink-arg=-fuse-ld=lld -Clink-arg=-Wl,--icf=safe
```

The WinUI benchmark accepts `--compact-app --linux-safe-icf` and independently `--linux-packed-relocations`. Its `--max-executable-bytes` option checks the stripped application including its full runtime. Linux reports include the allocated ELF sections and their file sizes.

## Full WinUI on Linux x86-64, 2 November 2026

The startup marker is flushed after the first freshly constructed theme returns the correct typed Button `ControlTheme`. The release consumer uses size optimization, ThinLTO, 16 codegen units and normal unwinding. The ordinary editable consumer links the external interpreter. A separate run enables compiler-produced operand updates while retaining that interpreter. This opt-in costs 46,152 additional executable bytes with the ordinary ELF link. Both executable size and startup below include the framework. These measurements cover theme readiness before rendering a window.

| Consumer | Stripped bytes | Median spawn to Button (ms) | p95 (ms) |
| --- | ---: | ---: | ---: |
| Main native, `b7537398` | 54,511,936 | 16.21 | 22.68 |
| Previous editable PR, `74a4b1ee` | 49,962,376 | 13.09 | 17.93 |
| Native Rust | 47,043,592 | 11.58 | 18.37 |
| Typed FXB | 46,702,520 | 12.43 | 16.81 |
| Portable solid FXP, interpreter only | 45,478,776 | 1670.78 | 1823.91 |
| Shipping editable FXP | 49,664,936 | 11.37 | 16.42 |
| Shipping plus FXU | 49,711,088 | 12.27 | 17.53 |
| Shipping editable FXP, Button root | 46,155,320 | 6.81 | 9.40 |
| Shipping editable FXP, Linux RELR | 43,919,472 | 12.21 | 15.81 |
| Main native, Linux RELR | 48,755,952 | 16.39 | 28.23 |
| Shipping plus FXU, Linux RELR | 43,964,616 | 12.30 | 17.27 |

| First load of an external revision | Median spawn to Button (ms) | p95 (ms) |
| --- | ---: | ---: |
| Same installed v3 FXP read from disk | 11.86 | 15.99 |
| Edited compact dictionary, v3 FXP | 15.84 | 21.64 |
| Structural replacement of compact dictionary, v3 FXP | 14.23 | 19.43 |
| Edited compact dictionary, indexed v1 FXP | 33.74 | 45.42 |
| Edited compact dictionary, monolithic v2 FXP | 39.36 | 54.13 |
| Edited merged and compact dictionaries, v3 FXP | 1632.55 | 1790.20 |
| Same merged and compact scalar edits, 837-byte FXU | 11.69 | 16.41 |

The primary table uses fresh processes on warm filesystem pages, with 61 rotating launches per fast case and nine per interpreter or mixed-archive fallback. For the compact scalar edit, the median was 0.38 ms below main. Its paired 95% bootstrap ratio interval was 0.921–1.045. That case passed the median gate and the samples do not establish a speedup. The FXU mixed edit had a median ratio of 0.721 with interval 0.703–0.789.

In 31 additional launches per case, Linux reported zero pages of the tested executable and external archive resident before each launch. Main then measured 44.46 ms, installed shipping 36.74 ms, the external v3 scalar edit 43.61 ms, and the mixed FXU edit 33.68 ms. Shared libraries and device caches remained warm.

The report records every sample, rotation, ELF section, artifact hash and compiler observation. The Linux RELR variant changes only the final link and requires a compatible dynamic loader. [Raw measurements and confidence intervals](benchmarks/compiled-theme-incremental-linux-x86_64.json)

The generator now shares more deferred factory bodies and passes type and child-builder operands through the individual resource thunks. It also shares name registration, virtual `EndInit` and nullable binding operations. Reachable call sites determine which type and factory pointers survive linking. The full native Rust output is 5,710,160 bytes and the full editable configuration emits 5,096,702 bytes. In the ordinary no-LTO generator isolation, linked WinUI fell from 10,118,051 to 9,578,256 bytes. The XAML build script took 30.76 s on main, 10.94 s for native Rust, 12.17 s for typed FXB, 14.03 s for editable shipping and 1.88 s for portable FXP (one observation per configuration). Two matched theme-only rustc replays per revision averaged 79.70 s on main and 36.98 s on the PR. These unit timings exclude XAML generation and the final application link. [Generator isolation, decode checks and execution-only FXU results](benchmarks/compiled-theme-codegen-and-operands-linux-x86_64.json)

The full theme has 894 linked FXB leaves, totaling 211,373 bytes. Predictable source coordinates use FXB v2. The prior build emitted 236,219 bytes, with identical keys, payloads, factory slots and locations across all 7,354 rows. The distribution-sized portable FXP is 245,385 bytes. The editable v3 FXP spends 50,962 more bytes on independently compressed pages. An explicit `FERROUI_WINUI_THEME_ROOTS=Button` build emits 718,804 bytes of Rust, 137 FXB leaves totaling 28,671 bytes, and a 102,173-byte FXP.

The build currently certifies one separate native include in full WinUI, `DensityStyles/Compact.xaml`. Its FXP document can be structurally replaced. Inlined resources take full-group transformation when supplied in a complete FXP. For caller-compatible edits, the editor can instead generate an FXU update from two compiler emissions. The measured 837-byte file changes a merged resource in `Resources/Generic.xaml` and a private compact resource. The validation runs checked both values (19 and 17) and the defaults of a fresh theme (14 and 14). Changes to the compiled caller require a rebuild or full FXP. [FXP page layout and incremental execution](porting/xaml-compiler/fxp-solid.md) · [FXB/FXU formats and authoring](porting/xaml-compiler/fxb.md)

To reproduce the warm-cache gate with two worktrees and independent Cargo target directories, run from the candidate checkout. Use the exact installed and edited manifests to produce `density.fxu`. Rebuilding the edited theme with `cargo check --release` runs its build script without linking the application.

```sh
python3 scripts/themes/bench/measure.py --source "$MAIN_CHECKOUT" \
  --work-dir "$BENCH/main" --compact-app --rebuilds 0 --launches 61
python3 scripts/themes/bench/measure.py --solid --compact-app --compiled-updates \
  --work-dir "$BENCH/edited" --rebuilds 0 --launches 61 \
  --startup-baseline "$BENCH/main/stripped-probe" \
  --require-archive-startup "$EDITED_COMPACT_FXP" \
  --require-archive-startup "$MERGED_OPERAND_FXU"
```

## Earlier shipping comparison against main at c6279e98

On Linux x86-64, the full editable WinUI consumer reached its first typed Button theme in **13.05 ms** versus **16.16 ms** on main at `c6279e98`. Both used size optimization, ThinLTO and ordinary unwinding. The shipping consumer also links the runtime that handles independently edited archives. Both executables used the same probe source, a warm filesystem cache, and the complete theme. The marker follows the typed lookup and precedes process teardown. First-frame rendering is outside this headless benchmark.

| Full theme | Generated Rust | XAML build script | Theme rustc | Stripped executable | Spawn to Button p50 | p95 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Main native AOT | 14,421,918 B | 25.56 s | 63.44 s | 54,512,584 B | 16.16 ms | 22.73 ms |
| Precompiled solid with external FXP loading | 5,325,703 B | 14.36 s | 32.68 s | **49,962,376 B** | **13.05 ms** | 15.65 ms |
| Same executable, matching full FXP read from disk | — | — | — | 49,962,376 B | 13.34 ms | 17.60 ms |
| Same executable, edited full FXP read from disk | — | — | — | 49,962,376 B | 1841.92 ms | 1947.81 ms |

The installed startup time fell 19.2% and the stripped executable lost 4,550,208 bytes (8.3%). Fresh processes rotated for 31 rounds per fast case on one CPU after compilation had finished. The changed archive used seven separate launches. The initial Cargo-driven gate also passed, at 14.18 ms against 17.65 ms. In both series startup uses every resource root and the linked external loader can execute a different FXP.

Build-script times are single Cargo unit observations. Theme rustc times are means of two matched invocations per revision in reversed order on the generated output, using the build's exact dependency metadata and optimization settings. They exclude the final ThinLTO link. The machine restarted between Cargo units, so there is no uninterrupted whole-graph cold build comparison. The 49.96 MB executable contains 28.78 MB of machine code, 10.04 MB of exception and unwind data, and 5.80 MB of dynamic relocations. [Raw process samples, compiler times, hashes and ELF sections](benchmarks/compiled-theme-shipping-linux-x86_64.json)

## Shipping WinUI with startup parity

Use `ferroui-themes-winui/solid-theme` for an installed full theme with a reloadable FXP revision. The build resolves XAML types, overloads, includes, property getters and deferred builders. The constructor executes that typed program with FXB resource operands. The application can also install independently supplied FXP files. An exact revision can dispatch to the installed program after checking the bytes, URI, assembly, compiler settings and active overrides. A changed revision can use the incremental paths described above or be resolved through XamlX. The `portable-solid-theme` feature offers a separate, interpreter-only configuration with a smaller WinUI contribution.

The original no-LTO consumer comparison used all 169 WinUI documents, open-ended resource lookup, Linux x86-64, and a warm filesystem cache. Fast lanes rotated in 31 fresh processes each. The two slow lanes used seven launches each. The source-only framework comparison below is a different experiment.

| No-LTO consumer and startup path | Stripped executable | Median spawn to Button |
| --- | ---: | ---: |
| Native AOT on the original main baseline `df8b92d9` | 71,124,168 B | 19.83 ms |
| Prior PR native AOT | 67,059,216 B | 19.68 ms |
| Prior PR solid FXP interpreted at startup | 59,498,592 B | 2,086.97 ms |
| Precompiled solid theme with external FXP loading linked | 65,720,728 B | 13.94 ms |
| Same executable, matching FXP read from disk | 65,720,728 B | 13.76 ms |
| Same executable, edited full FXP read from disk | 65,720,728 B | 2,019.82 ms |

The editable precompiled executable retains 10,118,051 bytes attributed to WinUI, including its typed code, FXB blocks and canonical FXP. The isolated release theme rustc invocation took 70.07 s and the probe compilation and link took 2.17 s. The comparison reused the original release dependency graph with updated framework and compiler components. The independently edited file changed compact font size from 14 to 17, and the runtime returned the edited value. [All launches, linked sections, build inputs and hashes](benchmarks/precompiled-theme-full-release-linux-x86_64.json)

Full WinUI generates 5,939,605 bytes of native Rust or 5,325,703 bytes with FXB, against 14,421,918 bytes on the original main baseline. The compiler shares 186 property getter pairs across 1,272 template-binding assignments. A Button-rooted build references 37 pairs and 137 FXB blocks. Its realized Button template responded to live padding changes and the DatePicker style was absent. The shipping comparison uses the full dictionary. [Generated outputs, trim checks and template verification](benchmarks/compiled-theme-binding-rows-linux-x86_64.json)

Workspace applications can select this profile with `cargo build --profile compact-release`. Cargo reads profiles from the application workspace. Downstream consumers can copy the `[profile.compact-release]` section of FerroUI's root manifest and enable `solid-theme` on the WinUI dependency.

To check startup on another machine, build both checkouts with the same optimization settings. The comparison exits with a failure if the candidate's median time through the verified Button lookup exceeds the reference median. `--compact-app` applies size optimization and ThinLTO to the whole downstream executable. Both programs use normal panic unwinding.

```sh
python3 scripts/themes/bench/measure.py --source ../FerroUI-main \
    --compact-app --work-dir /tmp/main-winui --rebuilds 0
python3 scripts/themes/bench/measure.py --solid --compact-app \
    --work-dir /tmp/solid-winui --rebuilds 0 --launches 31 \
    --startup-baseline /tmp/main-winui/stripped-probe
```

## What the 59.50 MB WinUI executable contains

The stripped Linux x86-64 consumer at `9c484e44` embeds a 245,385-byte FXP containing the complete WinUI theme. Its LLD map accounts for the following allocated input sections. Code instantiated in another crate is charged to the crate that supplied the object file.

| Input archive | Allocated MB |
| --- | ---: |
| FerroUI.Controls | 27.51 |
| FerroUI.Base | 17.54 |
| Markup.Xaml.Loader | 2.13 |
| Markup.Xaml | 1.83 |
| Dialogs | 1.69 |
| XamlX | 0.63 |
| WinUI including the archive | 0.59 |

In the same executable, machine code occupies 38.88 MB, exception and unwind tables 11.21 MB, and dynamic relocations 4.70 MB. These ELF section totals cut across the crates in the table. [ELF and archive breakdown](benchmarks/solid-theme-link-allocations-linux-x86_64.json)

Sharing inherited virtual-table construction, cast adapters and the slow path for markup argument coercion removed **4,471,800 bytes** from that executable. Rebuilding with the same archive, release settings and open reflection registrations produced a 55,026,792-byte binary. Inherited virtual-table builders fell from 1,236,349 to 223,872 bytes, registered cast routines from 709,884 to 232,491 bytes, and specialized markup argument readers from 623,216 to 320,687 bytes. Interpreting the syntax archive still took approximately 1.8 seconds through first Button. [Isolated source comparison, linked sections and launches](benchmarks/solid-theme-framework-erasure-linux-x86_64.json)

## Earlier 600 KB full-theme result

The complete WinUI crate occupies **585,562 allocated bytes** with the solid FXP backend on Linux x86-64. This includes the 245,385-byte archive of all 169 WinUI documents, native code, metadata, writable data, TLS and unwind tables. The size profile sets only the WinUI crate to `opt-level="z"` and `codegen-units=1`. The remaining dependencies use release optimization for speed. The resulting application occupies 59,498,592 bytes after stripping.

| Full WinUI configuration | Generated theme Rust B | Embedded FXB/FXP B | LLD WinUI input sections B | Stripped application B | WinUI rustc s | Spawn to first Button ms |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Previous native AOT | 7,903,409 | 0 | — | 68,233,648 | 168.68 | 19.03 |
| Native AOT | 6,085,047 | 0 | 10,306,856 | 67,059,216 | 163.74 | 18.02 |
| Previous AOT with FXB blocks | 7,289,507 | 236,219 | — | 67,923,400 | 160.16 | 20.11 |
| AOT with FXB blocks | 5,471,145 | 236,219 | 10,272,682 | 66,749,008 | 145.88 | 19.14 |
| Native AOT, size profile | 6,085,047 | 0 | 7,229,830 | 63,949,880 | 76.77 | 19.65 |
| Independent FXP | 0 | 432,438 | 884,983 | 59,764,712 | 9.09 | 2,152.60 |
| Solid FXP, size profile | 0 | 245,385 | 585,562 | 59,498,592 | 4.46 | 1,907.43 |

All rows retain open-ended resource lookup. Native measurements used 25 interleaved fresh processes per executable. The indexed and solid medians come from separate seven-launch rotating runs on the same shared host. The solid table row uses the final class-metadata consumer. Its compilation figures come from the preceding build with identical WinUI settings and archive. Each run warmed the filesystem cache and timed through the ready marker after the first Button lookup in a headless application. Cargo timings measure only the theme crate's rustc invocation and are single samples. Compiling the runtime loader in the solid consumer took 42.54 s with the other dependencies warm. Its theme build script took 1.90 s.

The native emitter shares typed deferred factory bodies, repeats property getters only at their original execution sites, and interns the construction and attachment of matching child styles. Large modules use private path aliases for the symbols their generated code references. No import table enumerates unrelated controls. Full native generated Rust fell 23.0% against the previous PR release probe. The native symbols in `ferroui_themes_winui::compiled_xaml` fell from 4,657,569 to 3,574,920 bytes (23.2%). The full FXB application saved 1,174,392 linked bytes while retaining every one of its 894 resource blocks byte for byte. Native and FXB still link several megabytes of typed builders.

The solid image stores the parsed graphs with shared strings, per-opcode streams and original diagnostic coordinates. WinUI expands that image once. XamlX still resolves types and runs semantic transforms on first use. Both indexed and solid FXP support transactional whole-archive reload. A candidate must pass validation, including its unreferenced members, before it replaces the live contribution.

Bringing packed startup near the 18 ms native result requires moving the remaining semantic work to the compiler. The measured solid consumer spent about 1.37 ms resolving Button after the constructor returned. An execution-ready representation would store the resolved constructor, property, overload and deferred-template plans with document-local typed imports. The runtime could then execute the root plan immediately and open a template plan on first use. A reload would validate the import signatures before publishing a replacement. The current solid format retains portable pre-transform graphs.

LLD assigns each input section to its supplying crate. Shared framework code, linker padding and synthesized dynamic relocations are outside the WinUI input-section figure and inside the total executable. The older release probes have no saved linker maps. Their executable sizes and generated text-symbol totals are available for comparison. Closing the dictionary over explicit roots prunes theme declarations, while the framework's open reflection and conversion registries can still retain control implementations.

```sh
python3 scripts/themes/bench/measure.py --work-dir /tmp/winui-native --lld-link-map
python3 scripts/themes/bench/measure.py --work-dir /tmp/winui-fxb --binary --lld-link-map
python3 scripts/themes/bench/measure.py --work-dir /tmp/winui-solid \
    --portable-solid --size-optimize-theme --lld-link-map --max-theme-bytes 599999
```

The size setting belongs in the root manifest of a downstream Cargo consumer.

```toml
[dependencies.ferroui-themes-winui]
path = "FerroUI/src/FerroUI.Themes.WinUI"
features = ["portable-solid-theme"]

[profile.release.package.ferroui-themes-winui]
opt-level = "z"
codegen-units = 1
```

[Native compilation, linker sections and startup samples](benchmarks/compiled-theme-full-600k-linux-x86_64.json) · [Packed compilation, linker sections and startup samples](benchmarks/packed-theme-linux-x86_64.json) · [Paired compiler-host measurements](benchmarks/compiled-theme-generator-600k.json) · [Solid FXP format](porting/xaml-compiler/fxp-solid.md)

## Earlier comparison of shared styles, scalar tables and local factory imports

The following comparison starts at PR #496’s previous head, with the first four resource factories already batched. The release consumer resolves Button from WinUI. It uses one Cargo job, rustc 1.99.0, opt-level 3, no cross-crate LTO and no incremental compilation. The full and candidate rooted configurations were rebuilt. For the two prior rooted configurations, the existing release probes and original Cargo timings were retained after verifying their generated Rust and embedded leaves byte for byte against a fresh generation at the starting head. All eight startup medians come from the same 25 interleaved fresh-process launches per mode with a warm page cache.

| Dictionary/backend | Generated Rust B | FXB B | Stripped app B | Theme text B | XAML build script s | WinUI rustc s | Spawn to Button ms |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Prior full native | 9,234,385 | 0 | 69,286,928 | 5,918,159 | 22.50 | 179.89 | 16.281 |
| Current full native | 7,903,409 | 0 | 68,233,648 | 4,657,569 | 25.22 | 168.68 | 15.518 |
| Prior full FXB | 8,753,658 | 175,024 | 69,004,736 | 5,938,160 | 23.37 | 171.69 | 15.729 |
| Current full FXB | 7,289,507 | 236,219 | 67,923,400 | 4,691,965 | 25.34 | 160.16 | 14.910 |
| Prior Button roots, native | 1,223,669 | 0 | 60,142,304 | 807,470 | 3.47 | 50.95 | 8.553 |
| Current Button roots, native | 1,007,677 | 0 | 59,983,472 | 596,956 | 4.25 | 51.23 | 7.200 |
| Prior Button roots, FXB | 1,165,828 | 22,362 | 60,118,328 | 810,131 | 4.24 | 54.23 | 8.063 |
| Current Button roots, FXB | 933,574 | 31,016 | 59,959,912 | 603,835 | 4.60 | 50.68 | 7.501 |

Full native output lost 1,330,976 bytes of Rust and 1,053,280 bytes from the linked application. The text symbols attributed to the generated module fell 21.3%. The full FXB application lost 1,081,336 bytes while gaining 1,969 wire records. Those records represent 411 deferred builder references, 470 colors, 489 doubles, 565 thicknesses and 34 corner radii. Framework code and data outside the generated module account for most of the linked application. Single-sample Cargo unit times include other work on the host. [Raw measurements and per-launch data](benchmarks/compiled-theme-dedup-linux-x86_64.json)

Both full builds contained a dictionary registration for each of `DefaultButtonStyle` and `DefaultDatePickerStyle`. Both Button-rooted builds retained one Button registration and zero DatePicker registrations. We counted the native builder rows and decoded the key fields of every FXB leaf the compiled module referenced. The stripped executables also contained the literal DatePicker key only in the full builds. Button resolved by type in every launch. The framework’s open-ended reflection registry can still retain the DatePicker type independently of its theme resource.

The build-script numbers in the table precede a later XamlX traversal optimization. Against identical inputs, four reversed-order, standalone build-script runs measured 30.91 s before and 18.35 s after on average (40.6% less wall time). Both the generated Rust and the exported type metadata had identical SHA-256 hashes. [Host measurement and input fingerprints](benchmarks/compiled-theme-host-namespace.json)

A separate host-only comparison includes the later traversal optimization and measures all four WinUI modes from the previous PR head. It ran each configuration twice in reverse order with fresh output directories. Generated Rust matched the measured release consumers in every case. [Per-run times, model hashes and sidecar fingerprints](benchmarks/compiled-theme-host-matrix.json)

| XAML generation only | Previous PR head s | Optimized host s |
| --- | ---: | ---: |
| Full native | 27.87 | 14.30 |
| Full FXB | 26.84 | 13.62 |
| Button roots, native | 4.28 | 2.67 |
| Button roots, FXB | 4.11 | 2.67 |

On an identical 456-leaf input, alternating 501 in-memory validation, expansion and drop passes per decoder measured 509 µs for the starting decoder and 349 µs for the candidate (median). The current full FXB resource set has 894 leaves and 7,354 records. A complete decode and drop of that larger set took 684 µs median over 101 passes.

## Runtime XML and portable FXD

These standalone consumers embed one production dictionary and pass its bytes to `FerroRuntimeXamlLoader::load_stream`. Both verify that the selected resource is still deferred, trigger its first lookup and assert the result type. The Fluent case also checks the brush color. The XML binary uses the default runtime loader. The FXD binary adds the `portable-xaml` feature. Each startup value is the median of 25 interleaved fresh processes with a warm file cache, measured through the flushed success marker.

| Runtime document | Embedded bytes | Stripped app B | Warm app build s | Dictionary load ms | First lookup µs | Spawn to resource ms |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| WinUI ThemeShadows XML | 2,648 | 56,781,736 | 0.78 | 5.070 | 16.87 | 9.399 |
| WinUI ThemeShadows FXD | 732 | 56,853,272 | 0.95 | 4.522 | 12.03 | 8.434 |
| Fluent BaseResources XML | 35,412 | 56,815,640 | 0.75 | 31.328 | 67.36 | 36.734 |
| Fluent BaseResources FXD | 15,760 | 56,869,448 | 0.81 | 27.645 | 71.66 | 32.163 |

Both representations run the same XamlX type resolution, transformations and interpreter on first load. For these one-document consumers, handling FXD added 71,536 and 53,808 bytes respectively to the stripped executable, including the smaller embedded payload. The warm app build cleans only the consumer, then compiles and links with its dependencies cached. The separately measured runtime loader rustc units took 34.59 s with XML and 36.24 s with portable loading enabled. The standalone release encoder read, parsed, compressed and atomically published each document in 1.426 ms (WinUI) and 4.930 ms (Fluent), including process startup and exit. [Raw observations and toolchain](benchmarks/portable-theme-runtime-linux-x86_64.json)

```sh
python3 scripts/themes/bench/measure_document.py --work-dir /tmp/doc-winui --fixture winui
python3 scripts/themes/bench/measure_document.py --work-dir /tmp/doc-fluent --fixture fluent
```

## Initial WinUI release measurements

The same consumer constructed WinUI and resolved its first deferred Button theme in each configuration. This Linux x86-64 run used rustc 1.99.0, opt-level 3, one Cargo job, no incremental compilation and no cross-crate LTO. Startup is the median of 21 fresh processes per mode, interleaved with a warm OS page cache. MB uses decimal units.

| Build | Generated Rust bytes | Embedded FXB bytes | WinUI rlib MB | Stripped app MB | XAML build script s | WinUI rustc s | Spawn to Button ms |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Baseline | 14,421,918 | 0 | 52.97 | 71.12 | 21.92 | 197.77 | 15.206 |
| Full native tables | 9,234,385 | 0 | 46.43 | 69.29 | 26.68 | 173.33 | 14.552 |
| Full FXB | 8,753,658 | 175,024 | 45.52 | 69.00 | 26.20 | 161.36 | 16.205 |
| Button roots, native tables | 1,223,669 | 0 | 19.66 | 60.14 | 3.47 | 50.95 | 6.916 |
| Button roots, FXB | 1,165,828 | 22,362 | 19.55 | 60.12 | 4.24 | 54.23 | 7.924 |

With full runtime lookup available, FXB saved 2,119,432 bytes from the baseline executable and 36.41 s of theme rustc time. The full FXB consumer took 1.65 ms longer to reach Button than the full native-table consumer. A Button-rooted native build reduced theme rustc time by 74.2%, WinUI rlib size by 62.9%, and whole-executable size by 15.4%. Its generated theme text symbols occupied 807,470 bytes, down 89.7% from baseline. Both rooted executables omitted the unrelated `DefaultDatePickerStyle` string.

Cargo timings measure the build script and theme rustc unit separately. The full-theme build script took longer in these samples. Cross-revision Cargo wall times also include different dependency rebuilds, so the table reports the directly comparable theme units. The results apply to this headless downstream release profile. The repo's LTO-enabled distribution profile and time to first rendered frame were outside this run.

A follow-up removed a second parse for class modifiers and stopped cloning unchanged Rust statements. A host-only check ran the baseline, compact emitter and final compact emitter twice in reversed order against the same WinUI sources and type models, writing to fresh output directories. Mean build-script wall times were 26.12 s, 26.85 s and 26.86 s. The last two produced identical Rust and exported document metadata. The measured full-theme gains come from compiling the smaller Rust program. Selecting roots also reduces the amount of XAML to transform.

The baseline was `df8b92d9`, the measured full representations were `9e4065b9`, and the rooted measurements used `4c6b82c7`, including the sparse theme-dictionary merge fix. Exact byte counts, section sizes, toolchain and all 105 measurements are in the [raw benchmark report](benchmarks/compiled-theme-linux-x86_64.json).

## Choosing the representation

| Consumer | Configuration | Representation |
| --- | --- | --- |
| Installed full WinUI with an editable archive | Enable `solid-theme` | Typed execution with FXB and a matching solid FXP |
| Portable solid WinUI image | Enable `portable-solid-theme` | Complete syntax archive resolved at first use |
| Open-ended resource dictionary | Default | Typed Rust calls with static operand tables for repeated factories |
| Open-ended dictionary with binary resources | Enable `binary-resources` on the theme crate | Typed Rust calls with independently embedded FXB blocks for eligible runs |
| Application with known WinUI keys and controls | Set `FERROUI_WINUI_THEME_ROOTS` at build time | Compile the selected keys and their transitive XAML dependencies, using either backend |
| Library that owns its build script | Call `XamlGroup::resource_roots(&[...])` | Apply the same closed-world pruning to that group |

For example, the following builds a small WinUI consumer with an accent button and a text block. Cargo tracks changes to the environment variable. Unset the variable to build the full theme.

```sh
FERROUI_WINUI_THEME_ROOTS=Button,AccentButtonStyle,TextBlock cargo build --release
```

The build script retains the compact-density dictionary that the WinUI constructor requests from Rust. Supply keys your own Rust code looks up, controls created at runtime, and names computed by plugins or custom markup extensions. All clients of the same WinUI crate instance share the selected roots in a Cargo build.

The closed-world pass runs before merged dictionaries expand. It indexes simple keyed entries and explicit type keys, follows references across the group, and keeps every declaration of a live name across scopes and theme variants. A local type name shared by several namespaces conservatively retains them all. It preserves unknown key expressions, unkeyed entries and whole retained templates. It also preserves named objects observable in the outer namescope. Removed spans retain line breaks and UTF-16 columns for XamlX diagnostics. An unknown requested root fails the build when the group can be analyzed. DTD-bearing markup takes the full compilation path so XamlX can apply its own XML rules.

The compiler recognizes deferred aliases, literal brushes, dynamic-resource brushes and static-resource brushes. It also batches eager colors, doubles, thicknesses and corner radii. Other string-keyed deferred XAML bodies can use run-local references to their typed builders. A singleton imported builder retains its direct call. The caller names exactly the builders in its run. It never enumerates an assembly to resolve an import at run time.

The native configuration leaves small functions with fewer than eight eligible additions on their original typed path. Both representations preserve dictionary insertion order. Changes of dictionary, incompatible operations and executable statements end a run. Source positions travel with the operands. FXB uses versioned records for the supported operations. [FXB specification and reload API](porting/xaml-compiler/fxb.md)

Generated nested styles share linear selector builders and call-site property getter thunks. Each selector application creates a new instance. Property getters execute as each setter is added, preserving the timing of metadata registration. In the full WinUI module, 1,623 calls use 27 selector shapes and 838 style attachments use the shared dynamic-setter loop. Checked template-priority assignments share an out-of-line runtime implementation.

Both backends name the required factory directly. The closed-world pass removes unused theme factories before rustc sees them. WinUI construction also calls the markup and dialogs registration APIs. The controls registration path calls `TypeInfo::register_all(TYPES)`, and runtime casts use registered nullable forms, handles and interface conversions. Those existing registries can retain control and dialog implementations independently of whether a theme factory survives. Applications which use open-ended resource lookup should use the full dictionary.

## Generated Rust in the built-in themes

The model host generated the full native theme in each configuration. Cargo checked all five crates and the generated Rust matched the four tracked reference files byte for byte. Percentages compare the current generator with the original main baseline.

| Theme | Original Rust bytes | Previous PR head bytes | Current Rust bytes | Total reduction |
| --- | ---: | ---: | ---: | ---: |
| WinUI | 14,421,918 | 9,234,385 | 7,903,409 | 45.2% |
| macOS | 13,033,763 | 8,320,752 | 7,123,236 | 45.3% |
| Fluent | 5,670,771 | 4,242,612 | 3,809,295 | 32.8% |
| Simple | 3,219,915 | 3,088,372 | 2,855,051 | 11.3% |
| Material | 15,910,704 | 10,353,001 | 9,207,472 | 42.1% |

The initial native implementation folded 3,598 static aliases, 738 literal brushes, 711 dynamic-color brushes and 338 static-color brushes. The remaining 792 deferred factories use direct typed construction. The FXB build of the same dictionary emitted 456 blocks with 5,385 records. Validating, expanding and dropping all 175,024 encoded bytes took 415 µs median and 538 µs at p95 across 101 passes over the Cargo-produced blocks with their bytes resident in memory.

## Machine code attributable to the full WinUI theme

On the matched Linux release consumer, the linked text symbols in `ferroui_themes_winui::compiled_xaml` occupied 7,833,137 bytes at baseline, 5,918,159 bytes with native tables and 5,938,160 bytes with FXB. These figures sum distinct address/size pairs for `t` and `T` symbols in `nm -S -C --defined-only` on unstripped copies. They exclude tables, unwind data and functions whose symbol names belong to another module.

The whole native executable lost 1,873,296 bytes in `.text`, while `.rela.dyn` grew by 244,056 bytes and `.data.rel.ro` grew by 254,120 bytes. Native tables contain Rust string references. FXB stores block-relative string indices and inline integers. Moving the full dictionary from native tables to FXB removed 240,288 bytes from `.rela.dyn` and 252,360 from `.data.rel.ro` while adding 180,064 bytes to `.rodata`. The resulting executable was 282,192 bytes smaller.

## Measuring a consumer

`scripts/themes/bench/measure.py` builds a headless, release-mode consumer through the normal WinUI constructor. It first resolves the Button control theme, which forces an actual deferred lookup. Each startup sample runs in a new process with the executable already in the OS page cache. There is no window system, renderer or first-frame wait in this measurement.

```sh
git worktree add --detach ../FerroUI-baseline df8b92d9
python3 scripts/themes/bench/measure.py --source ../FerroUI-baseline --work-dir /tmp/theme-before
python3 scripts/themes/bench/measure.py --work-dir /tmp/theme-native --compare-with /tmp/theme-before/stripped-probe
python3 scripts/themes/bench/measure.py --work-dir /tmp/theme-fxb --binary --compare-with /tmp/theme-before/stripped-probe
FERROUI_WINUI_THEME_ROOTS=Button python3 scripts/themes/bench/measure.py --work-dir /tmp/theme-button
FERROUI_WINUI_THEME_ROOTS=Button python3 scripts/themes/bench/measure.py --work-dir /tmp/theme-button-fxb --binary
```

The harness reports generated Rust and referenced FXB bytes separately, strips a copy of the whole executable, and measures a clean theme rebuild with its dependencies warm. It also records fresh-process startup, theme construction, first Button lookup, toolchain, job count and revision in each run's JSON report.
