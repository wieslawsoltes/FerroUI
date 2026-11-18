# Complete WinUI theme execution on macOS arm64

All 175 XAML documents from commit `933fd46325772fd1cdec81add9d4912dda2fe931` form one WinUI theme (2,132,586 source bytes). PR [496](https://github.com/wieslawsoltes/FerroUI/pull/496), at `8678e15b8ae738931237c0e70ef66f61a6ecbfd2`, supplies the FXD/FXP wire codec. The offline compiler uses XamlX to parse every document, writes the solid FXP and verifies each decoded document by re-encoding the original and decoded syntax trees and comparing bytes. `prepare.py` in the sibling semantic-spike directory performs this step.

The native executable uses the current checkout’s complete generated WinUI theme. The FXP executable includes the 251,770-byte archive, the decoder and the runtime compiler/interpreter. Its constructor decodes the documents, resolves types and properties, runs the ordinary document and group transforms and populates the class root. The XML reference embeds 487,507 bytes of tokenized XML, reconstructs the text and runs the unchanged group loader. That loader separately builds each document in addition to populating the class root. The cold timing and memory difference between the two interpreted executables therefore includes this extra work. The native and FXP arms build the same class root. PR 496 also contains a different prelinked execution backend. This experiment measures its portable document representation with current FerroUI runtime transforms.

## Complete executable measurements

The probes use macOS arm64 with rustc 1.99.1, LLVM 23.1.1, release optimizations, fat LTO, one codegen unit, abort-on-panic, size optimization for the WinUI crate, symbol stripping and identical ad-hoc signing. The in-process timers start at theme construction in a fresh process. Each process builds 128 additional Button templates. The table shows medians of 11 fresh processes per backend with rotating order and warm file caches. Other builds were running during the measurements.

| WinUI backend | Executable bytes | Theme construction | First Button template lookup | First template build | Repeated template build |
| --- | ---: | ---: | ---: | ---: | ---: |
| Current generated native | 18,578,224 | 3.595 ms | 104.2 µs | 47.1 µs | 2.69 µs |
| Solid FXP | 24,208,208 | 1,142.563 ms | 364.0 µs | 88.5 µs | 29.12 µs |
| Reconstructed XML | 24,355,760 | 1,574.227 ms | 408.0 µs | 89.8 µs | 28.99 µs |

FXP added 5,629,984 bytes to the stripped native probe. Its executable contains 20,262,892 bytes of Mach-O instructions, compared with 15,052,688 for native. The isolated verifier’s peak resident sets in one additional run each were 24,100,864 B native, 109,690,880 B FXP and 269,041,664 B XML. These high-water marks include verification after the timing stages.

Every executable produced the same 103,727-byte sorted inventory of 104 resolved type-keyed ControlThemes and their direct setter counts plus 2,984 distinct string keys gathered across dictionaries. SHA-256 is `98ea312d67134d745601d581923778739b5eae140b94ba2db69b0bdac126be8c`. Each actually built a Button’s named ContentPresenter, verified that padding tracked two changes to the templated parent, verified identity of the ButtonBackground brush in Light, Dark and HighContrast and checked the 32-to-24 compact-density height change. The exhaustive inventory counts string keys without evaluating every string-keyed value. Raw timings are in [results.json](results.json).

## Desktop ControlCatalog

The complete default Skia Catalog shrank from 65,493,168 to 65,261,072 bytes with FXP, a saving of 232,096 bytes (0.35%). Its Mach-O instruction section grew by 389,484 bytes, from 51,347,192 to 51,736,676. The constant-data section `__DATA_CONST,__const` fell by 775,344 bytes. Summing the named file-backed sections gives a net reduction of 169,380 bytes; the rest of the file difference includes layout and link-edit data.

Seven alternating fresh launches per mode measured process spawn through the Catalog’s own after-setup Light theme selection message. Median time rose from 116.771 ms native to 1,214.557 ms FXP, an increase of 1,097.786 ms. This milestone precedes the first rendered frame.

All nine 1900×1360 framework frames matched pixel-for-pixel: Buttons, ListBox and Calendar in Light, Dark and HighContrast. Both windows used an explicit desktop position of (20, 40) and client size of 950×680. An initial unpinned comparison showed different scrollbar and ListBox hover pixels; repositioning the native app alone also changed the hovered row. The recorded parity comes from the matched-position recapture.

## Reproduction

From the checkout root on macOS with the PR commit present locally and Cargo dependencies fetched, run the commands below. Set `SPIKE_TARGET` to an absolute path to an isolated prewarmed Cargo target if desired. For a local Skia build, use the same SDK and Ninja settings as an ordinary desktop Catalog build.

```sh
python3 -B scripts/themes/full-opcode-spike/run.py
python3 -B scripts/themes/full-opcode-spike/run.py catalog
```

The second command builds the stock desktop app from the root workspace, then a private copy of the Catalog, diagnostics and inking manifests that routes their shared WinUI dependency to FXP. All Catalog pages and auxiliary themes keep their normal compilation. The FXP theme crate still runs its normal build script to export type metadata for the Catalog’s offline compiler. Its linked module supplies the FXP entry point and empty native unit descriptors to meet the Catalog’s generated interface. Stock Catalog already retains all 166 WinUI units. Both modes use the default Skia desktop feature set, and the differing runtime-loader, XamlX, FXP decoder and full-themes/runtime-populate dependencies are counted in the FXP executable.

For the recorded native Catalog measurement we reused the stock build from the compact-theme experiment, built with `cargo build --locked --profile dist -p control-catalog-desktop --bin control-catalog-desktop`. Its previously stripped, linker-signed file measured 65,876,072 bytes. An additional Apple strip produced 65,876,048 bytes. Ad-hoc re-signing changed the signature page size from 4 KiB to 16 KiB and yielded 65,493,168 bytes. We apply the same final strip and ad-hoc re-sign to every executable in this report. Cargo’s profile-level strip figure and a post-sign figure should be compared within their respective conventions.

## Resolved resource bytecode

The production resource-row experiment freezes just the compiler and runtime for nine resolved resource families. Both builds use the same host, source and profile. Ordinary theme construction, selectors, templates and Catalog pages use their normal native code. In bytecode mode 897 independently linkable programs encode 7,070 WinUI resources in 53,315 instruction bytes plus string and namespace imports. These figures cover all 175 WinUI documents.

| Normalized executable or median duration | Native | Resource bytecode |
|---|---:|---:|
| Stripped, re-signed full WinUI probe | 18,578,256 B | 18,370,176 B |
| Stripped, re-signed default desktop Catalog | 65,498,144 B | 65,255,904 B |
| Probe theme construction | 3.637 ms | 3.749 ms |
| Probe first Button lookup | 102.959 µs | 103.208 µs |
| Probe first Button template build | 46.083 µs | 45.625 µs |
| Probe repeat Button template build | 2.689 µs | 2.634 µs |
| Catalog spawn to after-setup theme message | 129.212 ms | 124.449 ms |

Both Catalog baselines were independently rebuilt for this comparison. The 13 probe pairs and seven Catalog pairs alternate launch order. Timings include an initial cold file-cache outlier. The resource inventory and 104 control theme keys and setter counts matched, as did the live Button template binding. All nine Buttons/ListBox/Calendar framework screenshot pairs matched pixel-for-pixel across Light, Dark and HighContrast at 2200×1524. The raw samples, signatures, segment sizes and resource counts are in [typed-resource-results.json](typed-resource-results.json). Cargo's strip helper aborted on this host so both final sizes use the same Apple `strip` followed by `codesign -f -s -`. The unnormalized Cargo output sizes are retained in the results.

To reproduce this particular stage after other bytecode families have been implemented, create a separate checkout at the base commit below and apply the saved source patch there. Run the probe preparation from the new checkout using the investigation scripts copied from this directory and its sibling compact-spike directory.

```sh
git worktree add --detach ../ferro-resource-repro 933fd46325772fd1cdec81add9d4912dda2fe931
mkdir -p ../ferro-resource-repro/scripts/themes
cp -R scripts/themes/full-opcode-spike scripts/themes/compact-spike ../ferro-resource-repro/scripts/themes/
cd ../ferro-resource-repro
gzip -dc scripts/themes/full-opcode-spike/typed-resource-snapshot.patch.gz | git apply -
python3 -B scripts/themes/full-opcode-spike/prepare.py
export CARGO_TARGET_DIR="$PWD/target/resource-repro"
for mode in native bytecode; do
  export FERROUI_XAML_BACKEND="$mode"
  cargo build --profile dist --manifest-path target/full-opcode-spike/native/Cargo.toml
  cargo build --locked --profile dist -p control-catalog-desktop --bin control-catalog-desktop
  for exe in full-opcode-probe control-catalog-desktop; do
    cp "$CARGO_TARGET_DIR/dist/$exe" "$CARGO_TARGET_DIR/$exe-$mode"
    strip "$CARGO_TARGET_DIR/$exe-$mode"
    codesign -f -s - "$CARGO_TARGET_DIR/$exe-$mode"
    codesign --verify "$CARGO_TARGET_DIR/$exe-$mode"
  done
  FULL_SPIKE_KEYS="$CARGO_TARGET_DIR/$mode-keys.txt" "$CARGO_TARGET_DIR/full-opcode-probe-$mode"
done
cmp "$CARGO_TARGET_DIR/native-keys.txt" "$CARGO_TARGET_DIR/bytecode-keys.txt"
```

## Resolved resource, style and construction bytecode

A second frozen production snapshot adds per-factory ControlTheme programs, inline selector steps, deferred template descriptors and construction bytecode, including typed calls that return values. These measurements include the linked interpreters and imports. All times are medians across 11 alternating pairs of fresh processes. The sample columns are theme construction, first Button theme lookup, first Button template build and the mean of 128 subsequent template builds. Other Cargo jobs used the same machine during parts of the run, so small timing differences should be interpreted with the raw samples.

| Probe | Linked units | Linked style factories | Native → bytecode, stripped and signed | Construction | First lookup | First build | Repeat build |
|---|---:|---:|---:|---:|---:|---:|---:|
| Full WinUI | 166/166 | 277/277 | 18,578,256 → 18,700,768 B | 3.832 → 3.964 ms | 106.667 → 115.667 µs | 46.333 → 51.875 µs | 2.810 → 2.908 µs |
| Full Fluent | 75/75 | 117/117 | 15,567,920 → 15,778,640 B | 1.191 → 1.249 ms | 247.208 → 238.000 µs | 20.458 → 21.292 µs | 1.750 → 1.918 µs |
| Fluent ambient + Button | 2/75 | 10/117 | 9,973,904 → 10,023,616 B | 0.325 → 0.338 ms | 242.709 → 243.833 µs | 19.667 → 19.791 µs | 1.843 → 1.983 µs |

Apple link maps identify exactly the same units and style factories in each pair. The two-unit Fluent app references only the ambient and Button units, builds a genuine Button template, observes padding updates through TemplateBinding and verifies a Calendar type lookup returns no theme. Both full probes compare every top-level control theme key and setter count, and all string keys found recursively in their dictionaries. Full Fluent resolves 83 control themes and 1,111 distinct string keys. Full WinUI resolves 104 control themes and 2,984 distinct string keys.

WinUI's 277 style programs occupy 36,956 instruction bytes, covering 3,423 setters, 1,525 nested styles and 175 control template descriptors. They reference 440 native expression and 177 computed-value entries. Fluent's 117 style programs use 15,670 instruction bytes for 1,592 setters, 625 nested styles and 107 template descriptors, with 196 native expressions and 109 computed values. Fluent also uses 13,906 bytes for 372 resource programs containing 1,824 entries. Instruction-byte totals sum the logical programs before any merging by the linker. The executable sizes count import arrays and shared machine code. Three WinUI factories and five Fluent factories contain constructs outside the ControlThemeTable grammar. The general emitter handles their remaining statements.

[typed-style-results.json](typed-style-results.json) contains the linked unit names, raw samples, hashes, operation counts and verified inventory counts. Reproduce the frozen snapshot in another clean checkout at the same base commit as the resource experiment. Copy the two script directories there, apply `typed-style-snapshot.patch.gz` in place of the resource patch, prepare the native probe, and run the commands below. The additional isolation patch disables construction lowering, keeps the resource bytecode common to both modes, and leaves the backend switch controlling just the style representation. Apply that patch only when reproducing the style contribution separately.

```sh
gzip -dc scripts/themes/full-opcode-spike/typed-style-snapshot.patch.gz | git apply -
python3 -B scripts/themes/full-opcode-spike/prepare.py
python3 -B scripts/themes/full-opcode-spike/typed_theme_probe.py
```

For a matched full WinUI and desktop comparison of resource programs with or without style programs, use the second patch and a fresh output directory. The `native` selection in this isolated compiler keeps the resource programs and uses native style tables.

```sh
git apply scripts/themes/full-opcode-spike/typed-style-isolation.patch
export TYPED_THEME_OUT="$PWD/target/style-isolation"
python3 -B scripts/themes/full-opcode-spike/typed_theme_probe.py build native winui
python3 -B scripts/themes/full-opcode-spike/typed_theme_probe.py build bytecode winui
python3 -B scripts/themes/full-opcode-spike/typed_theme_probe.py compare winui
python3 -B scripts/themes/full-opcode-spike/typed_theme_probe.py catalog
```

| Isolated style comparison | Resource bytecode and native style tables | Resource and style bytecode |
|---|---:|---:|
| Full WinUI executable | 18,370,176 B | 18,137,792 B |
| Desktop Catalog executable | 65,255,888 B | 64,969,664 B |
| Full WinUI theme construction | 4.217 ms | 4.440 ms |
| First Button lookup | 113.167 µs | 116.250 µs |
| First Button build | 48.959 µs | 50.459 µs |
| Repeat Button build | 2.729 µs | 2.732 µs |
| Catalog spawn to after-setup | 183.914 ms | 183.971 ms |

Switching the style representation alone saved 232,384 bytes in the WinUI probe and 286,224 bytes in Desktop Catalog. The headless medians use 31 alternating process pairs. The Catalog medians use 21 pairs, following a seven-pair pass with a larger timing spread. Other builds were active during this experiment. Both pairs used the same Apple strip and re-sign normalization. The two independently built desktop executables rendered identical pixels on all nine 1900×1360 Buttons, ListBox and Calendar captures in Light, Dark and HighContrast. The JSON includes the initial and repeated timing samples.

## Integrated binary images and trimming

The next snapshot writes one packed `.fxc` per generated Rust module. Rust evaluates fixed-size slices of that file at compile time. Per-unit style programs, resource programs, construction functions and native imports remain eligible for linker trimming. The following measurements use pairs of independently rebuilt executables from identical manifests. The single build switch selects native or bytecode emission. Sizes include the complete linked runtime and the normalized code signature.

| Probe | Linked units | Linked style factories | Native → bytecode | Difference | Theme construction | First Button lookup | First build | Repeat build |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| Full WinUI | 166/166 | 277/277 | 18,578,256 → 18,553,568 B | −24,688 B | 3.754 → 4.046 ms | 108.625 → 124.375 µs | 51.083 → 48.875 µs | 2.747 → 3.257 µs |
| WinUI Button closure | 16/166 | 32/277 | 10,278,144 → 10,271,952 B | −6,192 B | 1.370 → 1.487 ms | 106.541 → 105.500 µs | 39.000 → 43.125 µs | 2.649 → 2.891 µs |
| Full Fluent | 75/75 | 117/117 | 15,567,920 → 15,719,824 B | +151,904 B | 1.276 → 1.318 ms | 258.084 → 273.375 µs | 20.792 → 23.041 µs | 1.952 → 1.961 µs |
| Fluent ambient + Button | 2/75 | 10/117 | 9,973,904 → 10,039,488 B | +65,584 B | 0.368 → 0.384 ms | 279.875 → 259.042 µs | 22.500 → 21.583 µs | 1.842 → 2.139 µs |

The WinUI build script reads the theme's exported metadata and runs the production `kept_units` closure with one explicit Button root. Its 16 units cover resources and controls reached through the ambient dictionaries and Button. A hand-selected three-unit experiment failed because the ambient content needed `PathIconUnscaledTemplate`. Only the production closure is included in the measured pair. The Fluent selection explicitly includes its ambient and Button units. Every probe realizes a real Button template and checks two live padding updates. The selected variants also verify Calendar is absent. Native and bytecode full-theme resource inventories match byte-for-byte. Timings use 11 alternating fresh-process pairs and include host activity.

| Complete theme module | Packed file | Construction programs | Binding paths | Resource programs | Style programs | Distinct ≥32-byte program patterns physically found in full → selected binary |
|---|---:|---:|---:|---:|---:|---:|
| WinUI | 168,176 B | 2,420 | 194 | 897 | 277 | 1,171 → 134 |
| Fluent | 68,942 B | 1,207 | 139 | 372 | 117 | 576 → 33 |

For each theme the full and selected builds have the same packed image hash. None of the four linked bytecode executables contains the whole module image contiguously. The last column checks exact byte sequences from the source image in the normalized executable. Substrings can overlap or be shared, so the results measure whether patterns remain; they cannot assign file bytes uniquely to each program. Link maps independently confirm the selected unit and style-factory names. The generated Rust references slices covering all but 9,066 bytes of the WinUI image and 986 bytes of the Fluent image. Those are build-artifact byte counts.

[typed-image-results.json](typed-image-results.json) contains the raw samples, source snapshot hash, signed executable hashes, exact linked unit names, per-family byte counts, binary pattern results and Mach-O segment sizes. For reproduction, start a fresh checkout at the same base commit, copy the investigation script directories there, and run the commands below with an absolute `CARGO_TARGET_DIR` if reusing a build cache.

```sh
gzip -dc scripts/themes/full-opcode-spike/typed-image-snapshot.patch.gz | git apply -
python3 -B scripts/themes/full-opcode-spike/prepare.py
export TYPED_THEME_OUT="$PWD/target/image-repro"
python3 -B scripts/themes/full-opcode-spike/typed_theme_probe.py
```

## Standalone selector bytecode and construction result extraction

This source snapshot adds typed selector programs for ordinary pages and deferred content and uses the iterator-based construction result API. Both modes were independently rebuilt with the same fat-LTO distribution profile, Apple strip and ad-hoc signature. Times are medians of eleven alternating pairs of fresh processes. Each repeated-build observation averages 128 builds. Other compiler processes were active during parts of the run.

| Case | Linked units | Native → bytecode bytes | Change | Theme construction (ms) | First Button lookup (µs) | First template (µs) | Repeated template (µs) |
|---|---:|---:|---:|---:|---:|---:|---:|
| Full WinUI | 166/166 | 18,578,256 → 18,384,592 | -193,664 | 3.671 → 3.836 | 102.792 → 105.208 | 45.250 → 44.708 | 2.723 → 3.144 |
| WinUI Button closure | 16/166 | 10,278,144 → 10,271,728 | -6,416 | 1.369 → 1.428 | 103.541 → 105.958 | 40.000 → 42.084 | 2.676 → 3.156 |
| Full Fluent | 75/75 | 15,567,920 → 15,653,440 | +85,520 | 1.227 → 1.284 | 244.792 → 248.459 | 20.584 → 22.666 | 1.773 → 2.020 |
| Fluent ambient and Button | 2/75 | 9,973,904 → 10,039,440 | +65,536 | 0.333 → 0.346 | 244.834 → 253.084 | 21.291 → 22.000 | 1.805 → 2.095 |

WinUI has 34 separately named selector programs containing 270 logical instruction bytes. Its full probe retains 34 named selectors and 277 whole-theme programs. Its Button probe retains one named selector and 32 whole-theme programs. The generated image occupies 188,339 bytes, including 15,547 bytes outside final referenced instruction spans. Fluent has 68 separately named selector programs containing 667 logical instruction bytes. Its full probe retains 68 named selectors and 117 whole-theme programs. Its Button probe retains zero named selectors and 10 whole-theme programs. The generated image occupies 77,678 bytes, including 3,792 bytes outside final referenced instruction spans. All four executables embed individual instruction streams. The complete theme image does not appear contiguously in any of them. Both full probes compared all discovered resource keys and theme setter counts. Each Button probe built its real template, observed live padding changes through TemplateBinding, and checked that the Calendar theme was absent.

The generated Catalog sources previously contained 404 direct selector-builder calls. In the bytecode build, 48 modules use 118 standalone selector programs (936 logical bytes), covering all thirteen original selector shapes. WinUI previously contained 181 such calls and Fluent 329. All three regenerated corpora contain zero direct selector-builder calls. Their complete ControlTheme program counts stayed at 54, 277 and 117 respectively. A separate native/bytecode executable verified a scoped, fallible RenderTransform property conversion and an attached Grid.Row selector. The accessor type witness for that out-of-corpus conversion was added after this size snapshot.

[Raw observations and source hashes](typed-selector-results.json) include per-process samples, link-map names, per-family byte counts, image coverage and corpus measurements. To reproduce, follow the preceding packed-image instructions with `typed-selector-snapshot.patch.gz` as the source patch. The four probes use the same `typed_theme_probe.py` commands.

## Final full-theme comparison

The current construction VM adds ordered setter choice, separated string and operation import pools, grouped register lifetimes and the default TemplateBinding choice proved in the [allocation and timing investigation](template-runtime/README.md). The source digest is `90c8d10584ba72b83b861b2d5dccd4f2cd637b27db52ae5eca0c05a4fdc03d9d`. Each pair uses identical project, theme selection, compiler, linker and runtime settings. Time columns show medians from eleven pairs of fresh processes. The last column averages 128 real Button template builds in each process. Other development builds ran concurrently.

| Case | Linked units | Native → typed bytes | Change | Construction (ms) | First Button lookup (µs) | First template (µs) | Repeated template (µs) |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Full WinUI | 166/166 | 18,578,256 → 18,121,696 | -456,560 | 3.763 → 4.192 | 105.291 → 112.375 | 48.375 → 46.459 | 2.705 → 2.855 |
| WinUI Button closure | 16/166 | 10,278,144 → 10,271,792 | -6,352 | 1.438 → 1.458 | 111.375 → 111.000 | 41.833 → 41.917 | 2.682 → 3.273 |
| Full Fluent | 75/75 | 15,567,920 → 15,505,952 | -61,968 | 1.295 → 1.464 | 253.083 → 291.709 | 22.250 → 20.584 | 1.979 → 1.826 |
| Fluent ambient and Button | 2/75 | 9,973,904 → 10,022,992 | +49,088 | 0.330 → 0.339 | 236.542 → 247.666 | 20.042 → 18.625 | 1.826 → 1.862 |

WinUI emitted a 191,711-byte theme instruction image. It contains 2,612 referenced programs, including 1,179 construction programs, 31 ordered setter choices and 277 ControlTheme programs. The full and Button builds retained 277 and 32 named style factories. Out of 1,029 distinct instruction sequences of at least 32 bytes, 1,029 occur in the full binary and 131 in the Button binary.

Fluent emitted an 80,445-byte theme instruction image. It contains 1,340 referenced programs, including 593 construction programs, 51 ordered setter choices and 117 ControlTheme programs. The full and Button builds retained 117 and 10 named style factories. Out of 479 distinct instruction sequences of at least 32 bytes, 479 occur in the full binary and 27 in the Button binary.

A second timing pass rotated all four cases across 32 more alternating pairs with the same eight executables. Full WinUI measured 2.697 → 2.853 µs per repeated build, a paired median increase of 144 ns (95% bootstrap interval 127 to 183). The trimmed WinUI case measured 2.622 → 2.832 µs, a paired increase of 198 ns (159 to 245). Full Fluent measured 1.830 → 1.832 µs, a paired difference of +8 ns (−28 to +28). Trimmed Fluent measured 1.840 → 1.849 µs, a paired difference of +20 ns (−1 to +48). The longer comparison supports a repeatable WinUI runtime cost. Both Fluent intervals include zero. Reproduce this pass using `python3 scripts/themes/full-opcode-spike/paired_templates.py "$TYPED_THEME_OUT"`.

Both theme class roots use native theme registration. Counting generated `fn build_*` bodies after excluding comments and typed construction import declarations, 375 of 1,455 WinUI functions and 245 of 854 Fluent functions still contain direct native `__markup_` calls. These include 254 and 216 direct `ProvideValue` sites respectively. The construction, resource, style, selector and setter-choice byte streams therefore coexist with native factory bodies and typed host imports.

Both full builds matched the native key/type/setter inventories byte-for-byte. The Button builds retained exactly the expected units and exercised the named presenter, live padding updates and the expected Calendar availability. Each executable stores individual reachable programs. A bytewise search found no complete contiguous theme image in the four typed executables.

The benchmark predates a compiler guard that treats literal `true` and `false` as values during lowering. A separate run of the updated WinUI and Fluent build scripts regenerated both full-theme Rust sources and both FXC images, with identical SHA-256 values to the measured outputs.

[Per-process measurements, inventory and image hashes, exact retained units and symbol counts](typed-final-results.json) include the details. Use [the final compiler and runtime snapshot](typed-final-snapshot.patch.gz) with the packed-image reproduction commands above.

## Theme-unit instructions and imports

The next snapshot encodes theme-unit application and lexical namespace operations and transfers last-use values from construction registers. Its full WinUI instruction file was 196,511 B and its full Fluent file was 82,010 B. The source corpora contain 175 and 86 XAML documents respectively. Typed native imports and some generated Rust remain in both executables. Each full probe retained every native theme unit and matched its native key/type/setter inventory.

| Application workload | Units retained | Native executable | Typed executable | File change | Paired change per warm Button build, median and 95% bootstrap interval |
| --- | ---: | ---: | ---: | ---: | ---: |
| Full WinUI | 166 / 166 | 18,578,224 B | 18,104,992 B | −473,232 B | +162 ns (+129 to +188) |
| WinUI Button | 16 / 166 | 10,261,696 B | 10,255,248 B | −6,448 B | +142.5 ns (+109 to +173) |
| Full Fluent | 75 / 75 | 15,567,888 B | 15,489,280 B | −78,608 B | −40 ns (−53 to −23) |
| Fluent ambient + Button | 2 / 75 | 9,973,872 B | 10,039,392 B | +65,520 B | −9.5 ns (−28 to +17) |

Each timing uses 32 alternating process pairs and 128 real Button builds per process, after the concurrent application benchmarks had finished. Median repeated construction on full WinUI moved from 2.690 to 2.846 µs. Full Fluent moved from 1.830 to 1.793 µs. The same snapshot's default desktop Hello World saved 6,384 B. Its two application pages produced 82 instruction bytes. The matching Catalog saved 4,473,952 B. Seven alternating navigation pairs recorded 2,268 native and 2,266 typed draws with median frame times of 7.483 and 7.470 ms respectively (medians of per-process medians). [Source snapshot](typed-theme-unit-snapshot.patch.gz) · [Theme links, inventories and timings](typed-theme-unit-results.json) · [Desktop application measurements](typed-application-results.json)

### Shared property-write decoder

The latest source resolves more static and converted values and shares the decoding of register-to-property writes across typed member adapters. It emits 198,490 instruction bytes for WinUI and 82,796 for Fluent. Both full probes matched their native inventories. The trimmed probes retained the same units as native. The warm timing column reports paired differences across 32 alternating process pairs after the Catalog navigation tests finished. The raw data also include an earlier pass under build load.

| Workload | Native executable | Typed executable | File change | Paired change per warm Button build, median and 95% bootstrap interval |
| --- | ---: | ---: | ---: | ---: |
| Full WinUI, 166 units | 18,578,224 B | 18,088,080 B | −490,144 B | +478.5 ns (+127.5 to +696.5) |
| WinUI Button, 16 units | 10,261,696 B | 10,255,024 B | −6,672 B | +252 ns (−80 to +417) |
| Full Fluent, 75 units | 15,567,888 B | 15,489,168 B | −78,720 B | −63 ns (−234 to +657.5) |
| Fluent ambient + Button, 2 units | 9,973,872 B | 10,022,912 B | +49,040 B | +56 ns (−159.5 to +406) |

The full WinUI Button cost remained measurable. Its magnitude varied between passes and compiler snapshots. The complete Fluent executable saved bytes while the smallest Fluent case grew. The paired intervals on this host leave the Fluent timing differences unresolved. [Source snapshot](typed-shared-setter-snapshot.patch.gz) · [Theme measurements and program reachability](typed-shared-setter-theme-results.json)
