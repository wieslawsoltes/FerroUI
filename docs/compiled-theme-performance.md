# Compiled XAML themes

The native Rust backend uses XamlX to resolve types, overloads, namespaces and includes at build time. The Rust backend used to emit a separate function for each deferred alias or brush, even when only its key, color and source position differed. The WinUI baseline at `df8b92d9` contains 14,421,918 bytes of generated Rust.

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
