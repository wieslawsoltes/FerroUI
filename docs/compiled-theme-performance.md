# Compiled XAML themes

XamlX resolves types, overloads, namespaces and includes at build time. The Rust backend used to emit a separate function for each deferred alias or brush, even when only its key, color and source position differed. The WinUI baseline at `df8b92d9` contains 14,421,918 bytes of generated Rust.

## WinUI release measurements

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

The native emitter recognizes four deferred operations. They create a static resource alias, a literal-color brush, a brush bound to a dynamic color resource, or a brush initialized from a static color resource. A function body needs at least eight eligible additions to use tables. Within it, a change of dictionary, factory or lexical scope ends the current run. Batching preserves insertion order and source positions. A different property, constructor, sharedness, or unrecognized operation takes its original typed path. The binary backend replaces the operands of those same operations with versioned records. The wire format and reload API are described in the [FXB specification](porting/xaml-compiler/fxb.md).

Both backends name the required factory directly. The closed-world pass removes unused theme factories before rustc sees them. WinUI construction also calls the markup and dialogs registration APIs. The controls registration path calls `TypeInfo::register_all(TYPES)`, and runtime casts use registered nullable forms, handles and interface conversions. Those existing registries can retain control and dialog implementations independently of whether a theme factory survives. Applications which use open-ended resource lookup should use the full dictionary.

## Generated Rust in the built-in themes

These counts are bytes of the full native output of the model-host compiler. All five theme crates passed `cargo check`. Each row compares the same theme on both revisions.

| Theme | Baseline Rust bytes | Compact Rust bytes | Reduction |
| --- | ---: | ---: | ---: |
| WinUI | 14,421,918 | 9,234,385 | 36.0% |
| macOS | 13,033,763 | 8,320,752 | 36.2% |
| Fluent | 5,670,771 | 4,242,612 | 25.2% |
| Simple | 3,219,915 | 3,088,372 | 4.1% |
| Material | 15,910,704 | 10,353,001 | 34.9% |

WinUI folded 3,598 static aliases, 738 literal brushes, 711 dynamic-color brushes and 338 static-color brushes. The remaining 792 deferred factories use direct typed construction. The FXB build of the same dictionary emitted 456 blocks with 5,385 records. Validating, expanding and dropping all 175,024 encoded bytes took 415 µs median and 538 µs at p95 across 101 passes over the Cargo-produced blocks with their bytes resident in memory.

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
