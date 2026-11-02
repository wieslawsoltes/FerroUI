# Compiled XAML themes

XamlX resolves types, overloads, namespaces and includes at build time. The Rust backend used to emit a separate function for each deferred alias or brush, even when only its key, color and source position differed. The WinUI baseline at `df8b92d9` contains 14,421,918 bytes of generated Rust.

## Shared styles, scalar tables and local factory imports

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

## Initial generated Rust in the built-in themes

These counts are bytes of the full native output of the model-host compiler. All five theme crates passed `cargo check`. Each row compares the same theme on both revisions.

| Theme | Baseline Rust bytes | Compact Rust bytes | Reduction |
| --- | ---: | ---: | ---: |
| WinUI | 14,421,918 | 9,234,385 | 36.0% |
| macOS | 13,033,763 | 8,320,752 | 36.2% |
| Fluent | 5,670,771 | 4,242,612 | 25.2% |
| Simple | 3,219,915 | 3,088,372 | 4.1% |
| Material | 15,910,704 | 10,353,001 | 34.9% |

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
