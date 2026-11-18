# Typed XAML construction programs

Compiled XAML has two representations. The native one, the default, is generated Rust: typed
factories, populate functions and resource tables. The typed one encodes the operations the XAML
compiler has already resolved as compact instruction streams, and the generated Rust keeps only
the typed imports those streams call. This page records the design, how to select it, what it
covers and what it measured. The design notes of each step are in
[`scripts/themes/full-opcode-spike/typed-xaml.md`](../../scripts/themes/full-opcode-spike/typed-xaml.md);
the earlier pure-token and AST experiments in
[`scripts/themes/full-opcode-spike/README.md`](../../scripts/themes/full-opcode-spike/README.md).

## 1. Selecting the representation

| Value | What is lowered |
| --- | --- |
| `native` (default) | Nothing: typed Rust factories and resource tables, as before |
| `theme-bytecode` | Resource runs and recognised control styles; templates and pages stay native |
| `bytecode` | Also construction, resolved calls and compiled binding paths in pages and templates |

A build script selects it with `Build::xaml_backend(XamlBackend::...)` (or
`Build::typed_xaml_bytecode(true)`); `Build::from_env` reads `FERROUI_XAML_BACKEND` for a
whole-build comparison. A shape the encoder does not recognise keeps its native output, so every
value builds every document.

## 2. Design

- **Where the code is.** The operand and opcode modules are in `src/Markup/FerroUI.Markup`
  (`construction_opcodes`, `resource_opcodes`, `style_opcodes`, `binding_opcodes`,
  `path_opcodes`, `theme_run_opcodes`, `compiled_operands`). The encoders are in
  `src/Markup/FerroUI.Markup.Xaml.Loader/rust_emitter/` (`construction`, `theme_row_program`,
  `theme_run_program`, `theme_style_program`, `selector_visitor`, `setter_choice`,
  `binding_program`). The interpreters are in `src/Markup/FerroUI.Markup.Xaml/xaml_il/runtime/`.
- **The streams are static data of the generated Rust.** A Cargo build packs the streams of a
  generated module into one byte image in `OUT_DIR` (`<module>.fxc`), and the generated source
  embeds it with `include_bytes!` into a `const`; each program is a constant array copied out of
  it at compile time (`rt::program_bytes`). An emitter used in memory writes byte literals
  instead. Nothing is read from a file at run time, and there is no file format beside the
  generated Rust.
- **The linker graph stays the same.** The generated statics and the per-factory or per-unit
  imports are what the linker reaches; a theme run and its imports are reached only through the
  theme unit table, so theme trimming (section 3 of [`docs/xaml-themes.md`](../xaml-themes.md))
  works unchanged.
- **Extracted theme runs.** With `bytecode`, each extracted theme unit run is one
  `apply_theme_run` call over its program and imports; the emitter checks the identity and kind
  of each row table and keeps the original run body when a call does not resolve.
- **Staged setters** of ordinary pages have separate creation and attachment instructions: a
  parented setter stays on the context stack while its value is evaluated, and the attachment
  uses the same setter instance.
- **Deferred content** (templates, deferred resources) runs its program when it is requested.
- **Font literals** lower `context.base_uri()` to the current execution context.

## 3. Coverage

In the integrated build of the desktop ControlCatalog, 173 of its 203 class documents emit a
single document program. Of the 268 distinct deferred factory sites, 216 contain only
construction regions and glue (171) or delegate to a typed style helper (45). The first deferred
fallback is the `ColorPickerFluent` StyleInclude at `App.xaml(38,8)`, whose factory calls an
external builder directly. Per-document and per-factory inventory:
[`windows-catalog-factories.md`](../../scripts/themes/full-opcode-spike/records/windows-catalog-factories.md).

All 347 extracted WinUI runs and all 86 Fluent runs encode in the two themes. The decoded streams
account for 887 and 357 row calls, 559 and 208 deferred registrations, and 50 and 21 immediate
inserts:
[`theme-schedule-windows.json`](../../scripts/themes/full-opcode-spike/records/theme-schedule-windows.json),
[`theme-schedule-status.md`](../../scripts/themes/full-opcode-spike/records/theme-schedule-status.md).

## 4. Measurements recorded

Numbers are bytes of the normalised (stripped, signed the same way) executable unless stated.
Each row links the record it comes from.

### 4.1 macOS arm64, `dist` profile

| Matched workload | Native | Typed | Observed cost |
| --- | ---: | ---: | --- |
| ControlCatalog with Skia | 65,498,144 | 60,595,952 | First compositor draw 345.0 vs 345.5 ms, 11 process pairs; all 18 screenshot pairs matched |
| Default desktop Hello World | 16,290,272 | 16,250,912 | First draw 184.1 vs 195.6 ms, 11 pairs |
| Full WinUI, 166 units | 18,578,224 | 18,055,232 | Repeated Button construction +183 ns (paired 95% bootstrap interval +145 to +249.5 ns) |
| Full Fluent, 75 units | 15,567,888 | 15,456,192 | +9.5 ns per Button, interval -16 to +42 ns |
| WinUI Button, 16 units | 10,261,696 | 10,222,288 | +70 ns per Button, interval +8 to +126 ns |
| Fluent ambient + Button, 2 units | 9,973,872 | 10,022,896 | File grew 49,024 B; +21.5 ns per Button, interval -37.5 to +74.5 ns |

The Catalog saving is 4,902,192 bytes (7.48%). The compiler emitted 632,129 bytes of instructions
across the application's dependency graph (332,370 for ControlCatalog, 209,427 for WinUI). Both
full theme inventories matched. Timing runs used 32 alternating process pairs and 128 warm
Button constructions per process. Geometry alone saved 114,912 file bytes but added 75, 87 and
384 ns to construct three DrawerPage icons. Records:
[`typed-page-operands-results.json`](../../scripts/themes/full-opcode-spike/typed-page-operands-results.json),
[`path-geometry/results.json`](../../scripts/themes/full-opcode-spike/path-geometry/results.json),
[`typed-xaml.md`](../../scripts/themes/full-opcode-spike/typed-xaml.md#converters-checked-writes-and-list-operands-in-ordinary-pages).
The font literal lowering took the Catalog a further 49,360 bytes down, to 60,546,592 bytes,
with 633,037 bytes of images and 161 of 203 class documents fully encoded at that step:
[`font-operands-results.json`](../../scripts/themes/full-opcode-spike/records/font-operands-results.json).

The scale available from population: the original 65,876,016-byte Catalog (another strip and
sign convention) held 7,997,836 bytes of directly named generated XAML instructions, and
removing every population body from a matched build removed 11,486,416 executable bytes (an
application with empty pages). The rest is the framework, property system, bindings, renderer,
assets and metadata:
[`catalog-allocation.json`](../../scripts/themes/full-opcode-spike/catalog-allocation.json),
[`catalog-omission.md`](../../scripts/themes/full-opcode-spike/catalog-omission.md),
[`original-size-survey/measurements.json`](../../scripts/themes/full-opcode-spike/records/original-size-survey/measurements.json).

### 4.2 Windows x86_64 (`x86_64-pc-windows-msvc`, Rust 1.93.0, Windows SDK 10.0.26100.0, `dist`, default Skia features)

| Shipping executable | Native | Typed, before theme runs and staged setters | Typed, with them | Change of the last step |
| --- | ---: | ---: | ---: | ---: |
| Hello World | 25,799,168 | 25,711,104 | 25,711,104 | 0 |
| ControlCatalog | 77,764,096 | 70,831,104 | 70,606,848 | -224,256 |

With the theme runs and staged setters the typed Catalog is 7,157,248 bytes (9.20%) smaller,
Hello World 88,064 bytes (0.34%). The instruction images of the Catalog's dependency graph grew
from 633,037 to 641,652 bytes; in the linked PE `.text` fell by 282,112 bytes and `.rdata` grew
by 52,736 bytes. Median first compositor draw over 11 alternating pairs, in milliseconds:

| Application | Native, before | Typed, before | Native, with | Typed, with |
| --- | ---: | ---: | ---: | ---: |
| Hello World | 550.7 | 552.2 | 451.8 | 443.0 |
| ControlCatalog | 592.7 | 592.8 | 544.0 | 532.8 |

A first integrated Catalog series measured 646.6 ms native and 655.1 ms typed and the order
reversed on the repeat, so these samples do not establish a startup change. All 18 native/typed
screenshot pairs (Buttons, ListBox, Calendar, TextBlock, TextBox, ColorPicker in three theme
variants) matched pixel for pixel. Full and Button-trimmed WinUI and Fluent gave identical native
and typed inventories in all four comparisons, each headless process building a real Button
template 130 times. Records:
[`windows-desktop-results.json`](../../scripts/themes/full-opcode-spike/records/windows-desktop-results.json),
[`theme-behavior-windows.json`](../../scripts/themes/full-opcode-spike/records/theme-behavior-windows.json);
the PE and image measurement script is
[`windows_desktop_measure.py`](../../scripts/themes/full-opcode-spike/windows_desktop_measure.py).

### 4.3 Framework experiments beside the XAML work

Sharing typed binding operations saved 2,020,464 bytes in the native Catalog; in the warm
propagation probe source-to-target went from 42 to 45 ns and target-to-source from 51 to 57 ns,
with no allocation in either arm. Sharing registered routed-event adapters saved 98,784 bytes
with all 1,138 method identities and signatures kept. Both are standalone patches, not part of
this change:
[`typed-binding-size/README.md`](../../scripts/perf/typed-binding-size/README.md),
[`combined-xaml/README.md`](../../scripts/perf/typed-binding-size/combined-xaml/README.md),
[`registration-review/README.md`](../../scripts/perf/typed-binding-size/registration-review/README.md).

## 5. Theme selection at crate and dictionary boundaries

Three gaps of theme selection reproduce through the build tasks (they are tracked in #846 to
#858):

| Input | Observed compiler output |
| --- | --- |
| Library XAML creates WinUI; consumer XAML creates a ListBox | The library emits `with_parts` with 16/166 units; the consumer exports the ListBox reference but emits an empty parts listing, and the selected table omits `Controls/ListBox.xaml` |
| The same `ListBox::new()` in a package's `src/main.rs` or `src/lib.rs` | The binary source yields 16 units without ListBox; the library source 29 units with it |
| A brush `UsedA` in one optional dictionary references `{DynamicResource BOnly}`, which a second graph defines | A exports an empty reference list; with `UsedA` rooted the selector keeps units `[0, 1]` of A and `[0]` of B, without the provider of `BOnly` |

The reproduction is
[`theme-roots-probe.rs`](../../scripts/themes/full-opcode-spike/records/theme-roots-probe.rs):
build Hello World with the native backend, then compile the probe with `rustc` against the
`ferroui_build` and `ferroui_build_scan` rlibs of that target directory and run it with a fresh
directory and the WinUI `.xamlmeta` of the build.

## 6. Reproducing

```sh
export TYPED_THEME_OUT="$PWD/target/typed-xaml-probes"
python3 scripts/themes/full-opcode-spike/prepare.py
python3 scripts/themes/full-opcode-spike/typed_theme_probe.py
python3 scripts/themes/full-opcode-spike/paired_templates.py "$TYPED_THEME_OUT"
python3 scripts/themes/full-opcode-spike/typed_theme_probe.py catalog
python3 scripts/themes/full-opcode-spike/catalog_startup.py --directory "$TYPED_THEME_OUT/catalog" \
    --candidate bytecode --first-frame --pairs 11 --output first-draw.json
for mode in native bytecode; do
  python3 scripts/themes/full-opcode-spike/build_catalog_candidate.py "target/typed-xaml/$mode" --backend "$mode"
done
FERROUI_XAML_BACKEND=bytecode cargo test --locked -p control-catalog --lib -- --test-threads=1
```

Build each arm in its own target directory and Cargo invocation, with identical executable
names, strip and sign steps. A startup marker, the first compositor draw, page navigation and
steady-state rendering measure different things; keep their samples apart.

## 7. What is left before it can be the default

1. Account separately for executable bytes, instruction bytes, native typed adapters, imported
   constants and strings, and the remaining generated native code; prove complete coverage per
   document, deferred factory and theme run, or name the exceptions.
2. Pass the native-equivalence, loader and Catalog suites with the typed backend, and verify
   rendering, resource keys and values, name scopes, bindings, diagnostics, deferred timing,
   relative URIs, error positions and control behaviour.
3. Measure Hello World, the full and trimmed WinUI and Fluent themes and the desktop
   ControlCatalog on one revision with matched settings, startup through first draw and repeated
   template creation, keeping the regressions (trimmed Fluent grows) in the record.
4. Decide from that evidence whether the file savings justify the runtime costs observed for
   repeated template construction.
