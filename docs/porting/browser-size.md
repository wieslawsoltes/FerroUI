# Browser build: size of the WebAssembly module

Status: first size pass, 2026-10-05. Subject: the `themed_view` example of `src/Browser/FerroUI.Browser` (Fluent theme, embedded Inter font, run-time XAML loader), built with `scripts/build-browser.sh`. Every number on this page was measured; how is described in section 6. See `browser-platform.md` for the platform design.

## 1. Result

| Build | `.wasm` raw | gzip -9 | brotli -11 | `.js` glue raw / gzip | First frame, WebGL2 | First frame, Software2D |
|---|---:|---:|---:|---:|---:|---:|
| Before: workspace `release` profile | 48,664,055 | 10,250,979 | 6,266,459 | 193,759 / 38,632 | 6.48 s | 6.29 s |
| After: `browser` profile and build script | 27,114,727 | 7,817,664 | 5,148,143 | 199,714 / 38,018 | 6.36 s | 6.18 s |
| Change | −44.3% | −23.7% | −17.8% | +3.1% / −1.6% | −1.9% | −1.7% |

Bytes. First frame: median of 7 loads of the page from a local server in headless Chromium 141 (section 6), measured back to back in one session. The script module of the platform (`ferroui.js`, 11,884 bytes, 4,801 gzip) and the host page are unchanged. Both rendering modes render pixel-identical to the release build after every step.

The module is still too large to publish comfortably: 7.8 MB with gzip, 5.1 MB with brotli. What is left is framework code, mostly monomorphized generic code of the property system (section 2.4), and the reductions that remain need a decision or other work (section 4).

## 2. Where the bytes are (before)

Measured on the release build of the baseline, linked once more with `--profiling-funcs` (a name section; the same 85,274 functions, code section 43,365,000 bytes against 43,368,975 in the published module) and with the `wasm-ld` map (`-Wl,--Map`). `scripts/browser/wasm-breakdown.py` joins the two: the map gives the input of every function (the crate whose code generation unit holds it, or the archive member of a native library), the module gives its size after Emscripten's `wasm-opt -O3`. A Rust generic function is in the crate that instantiates it. 4,876 functions (578,618 bytes, 1.3% of the code) were renamed or merged by `wasm-opt` and cannot be attributed.

### 2.1 Sections

| Section | Bytes | Share |
|---|---:|---:|
| code | 43,368,975 | 89.1% |
| data | 4,944,933 | 10.2% |
| element (function table) | 256,324 | 0.5% |
| function, type, import, export and the rest | 93,823 | 0.2% |

The published module has no name section, no DWARF and no producers section.

### 2.2 Code by crate or library

| Crate or library | Functions | Bytes | Share of code |
|---|---:|---:|---:|
| ferroui_controls | 37,275 | 19,539,387 | 45.1% |
| ferroui_base | 25,896 | 13,162,341 | 30.4% |
| Skia (prebuilt archive of rust-skia, Skia's own sources) | 6,839 | 3,339,729 | 7.7% |
| ferroui_markup_xaml_loader (run-time loader) | 2,264 | 2,093,517 | 4.8% |
| ferroui_markup_xaml | 3,452 | 1,722,612 | 4.0% |
| HarfBuzz | 503 | 591,460 | 1.4% |
| not attributed | 4,876 | 578,618 | 1.3% |
| xamlx (XAML front end) | 642 | 533,963 | 1.2% |
| FreeType (in the Skia archive) | 628 | 362,362 | 0.8% |
| libjpeg-turbo (in the Skia archive) | 266 | 283,864 | 0.7% |
| ferroui_skia | 518 | 212,840 | 0.5% |
| ferroui_themes_fluent | 480 | 200,599 | 0.5% |
| ferroui_browser | 294 | 110,536 | 0.3% |
| libc++ (Emscripten) | 403 | 109,228 | 0.3% |
| libpng (in the Skia archive) | 159 | 105,962 | 0.2% |
| roxmltree | 48 | 56,286 | 0.1% |
| core, std, alloc, compiler_builtins, panic_unwind, hashbrown (their own code units; generic code is counted where it is instantiated) | 264 | 108,199 | 0.2% |
| libc, dlmalloc, libc++abi, compiler-rt (Emscripten) | 187 | 66,156 | 0.2% |
| zlib, Wuffs (in the Skia archive) | 51 | 60,252 | 0.1% |
| everything else (ferroui_markup, the example, ferroui_harfbuzz, ferroui_opengl, rust_decimal, skia_safe, ferroui_fonts_inter, wasm_bindgen, rust-skia C++ shims) | 229 | 127,086 | 0.3% |

Native code in total (Skia with its third-party libraries and the rust-skia shims, HarfBuzz, Emscripten's libraries): 4,933,147 bytes, 11.4% of the code. Rust code: 37,853,232 bytes, 87.3%.

### 2.3 Skia

Skia comes as rust-skia's prebuilt binary `wasm32-unknown-emscripten-ganesh-gl-jpegd-jpege-pdf`; the linker keeps what is reachable. No PDF and no SVG code is linked. No ICU is linked (the binary has none; HarfBuzz brings its own Unicode tables, 99,651 bytes of data in total).

| Part of Skia (by source file of the archive member) | Bytes of code |
|---|---:|
| Ganesh: context, resources, effects | 590,388 |
| SkSL compiler and code generators (shaders are generated at run time for WebGL) | 500,022 |
| Ganesh: ops, path renderers, tessellation | 382,933 |
| core: canvas, paths, images, utilities | 360,775 |
| core: raster (scan conversion, blitters, pipeline) | 309,653 |
| image codecs and encoders (Skia side) | 201,784 |
| Ganesh: GL backend | 178,056 |
| path ops | 156,696 |
| image filters | 140,722 |
| skcms colour management | 119,005 |
| effects, gradients, colour and mask filters, shadows | 99,042 |
| core: picture recording and playback | 87,041 |
| FreeType font host and custom font manager | 61,275 |
| core: shaders, blenders, runtime effects | 57,356 |
| GPU shared code (text sub-runs, blending, tessellation) | 46,542 |
| core: text and glyph cache | 41,567 |
| other (logging, files, utilities) | 6,872 |
| Third-party libraries of the archive: FreeType 362,362, libjpeg-turbo 283,864, libpng 105,962, zlib 40,792, Wuffs 19,460 | 812,440 |
| Total | 4,152,169 |

### 2.4 Rust code by family

To see generic arguments and closures, the baseline was built once more with `-Csymbol-mangling-version=v0` (same profile; code section 43,359,187 bytes). Families are by the path of the item a function belongs to: a method of `StyledProperty<T>` counts for the property system whichever crate instantiates it, a closure for the function it is in. 36,059,321 bytes of Rust code were attributed (functions that `wasm-opt` merged are not counted).

| Family | Functions | Bytes | Share of Rust code |
|---|---:|---:|---:|
| property store (`ferroui_base::property_store`: effective values, binding entries, observers) | 10,957 | 6,933,924 | 19.2% |
| property definitions and metadata (`StyledProperty`, `DirectProperty`, `FerroProperty`, metadata tables, registry) | 6,072 | 4,103,777 | 11.4% |
| value type registry (`ferroui_base::data::core::value_type`) | 6,273 | 3,907,139 | 10.8% |
| rest of ferroui_base | 7,127 | 4,329,477 | 12.0% |
| layout, controls and templates (`ferroui_controls`) | 7,300 | 3,615,848 | 10.0% |
| class model (vtables, interface registration, casts) | 6,798 | 1,972,155 | 5.5% |
| std collections, `Rc`, iterators, `Once` and the rest | 8,033 | 1,868,154 | 5.2% |
| std drop glue (`drop_in_place`) | 3,797 | 1,440,536 | 4.0% |
| media, rendering and composition (with ferroui_skia, ferroui_harfbuzz, ferroui_opengl) | 3,520 | 1,424,809 | 4.0% |
| markup metadata tables (`markup_types::register_value_types` and its closures) | 115 | 1,178,142 | 3.3% |
| animation | 1,352 | 1,025,422 | 2.8% |
| XAML front end: transformers and compiler of the loader crate | 703 | 932,481 | 2.6% |
| input and interactivity | 2,252 | 735,174 | 2.0% |
| XAML front end: xamlx | 623 | 467,507 | 1.3% |
| XAML loader: run-time type system | 251 | 411,472 | 1.1% |
| markup extensions and converters (ferroui_markup_xaml) | 858 | 408,320 | 1.1% |
| styling and themes | 708 | 277,083 | 0.8% |
| class registration (`register_types`) | 42 | 192,095 | 0.5% |
| XAML loader: framework nodes | 106 | 187,082 | 0.5% |
| text formatting and Unicode | 147 | 181,511 | 0.5% |
| XAML loader: interpreter | 85 | 159,926 | 0.4% |
| reactive (observables, disposables) | 200 | 115,150 | 0.3% |
| browser platform and the example | 174 | 72,901 | 0.2% |
| roxmltree, ferroui_markup, std formatting and panics, small crates | 169 | 119,236 | 0.3% |

Together the XAML front end, the run-time loader and the interpreter (xamlx, ferroui_markup_xaml_loader, roxmltree) are 2,210,724 bytes; with the markup metadata tables and ferroui_markup_xaml 3,797,186 bytes (10.5%). The property system with the value type registry is 14,944,840 bytes (41.4%).

Monomorphization hot spots, by total size of the instances of one generic item:

| Generic item | Instances | Bytes |
|---|---:|---:|
| `ValueTypes::register_object` | 344 | 1,828,892 |
| `core::ptr::drop_in_place` | 3,792 | 1,440,130 |
| `BindingEntry::set_value` | 216 | 848,918 |
| `StyledProperty::from_untyped` | 216 | 645,380 |
| `StyledProperty::route_bind` | 218 | 556,674 |
| `EffectiveValue::dispose_and_raise_unset` | 216 | 450,613 |
| `EffectiveValue::coerce_value` | 216 | 403,897 |
| `LocalValueBindingObserver::start` | 176 | 368,173 |
| `EffectiveValue::set_and_raise_core` | 256 | 358,871 |
| `LocalValueBindingObserver::next_value` | 216 | 347,203 |
| `MarkupArguments::next` | 256 | 331,088 |
| `ValueTypes::register_conversion` (inner closures) | 734 | 312,362 |
| `BindingEntry::start_core` | 216 | 288,536 |
| `FerroProperty::register_with` | 376 | 261,461 |
| `alloc::rc::Rc::drop_slow` | 760 | 256,743 |

The property store is instantiated for each of about 216 property value types; each instance of the binding and effective-value machinery is 2 to 4 kB. The largest single functions are the markup metadata tables: `ferroui_base::markup_types::register_value_types` (712,807 bytes), `ferroui_controls::markup_types::register_value_types` (406,205), `ferroui_base::data::core::value_type::register_defaults` (260,485) and `ferroui_markup_xaml::register_types::register_value_types` (187,675).

### 2.5 Data

From the link map (sizes before `wasm-opt`, which drops zero bytes and merges segments: 5,174,741 bytes in the map, 4,944,933 in the module).

| Data | Bytes |
|---|---:|
| Embedded fonts: Inter in six weights (`ferroui_fonts_inter`) | 1,878,122 |
| Embedded theme documents: 86 Fluent `.xaml` files (`ferroui_themes_fluent`) | 763,784 |
| ferroui_base: Unicode tries of the text formatter (general data, segmentation, bidi, east-asian width) 289,840; vtables, string literals and other tables 670,066 | 959,906 |
| ferroui_controls: vtables and string literals | 751,391 |
| merged string literals (linker) | 173,212 |
| Skia with its third-party libraries | 203,821 |
| ferroui_markup_xaml, ferroui_markup_xaml_loader, xamlx | 233,737 |
| HarfBuzz (Unicode tables, shapers) | 99,651 |
| the rest | 111,117 |

The fonts compress to 928,325 bytes with gzip -9 (12% of the gzip size after this pass), the theme documents to 73,328.

### 2.6 Where the time to the first frame goes

A CPU profile of the baseline in headless Chromium (`first-frame.mjs --cpu-profile`, named build) up to the first frame: 7.3 s of samples under the profiler. 3.9 s are Emscripten's exception-handling trampolines: every Rust call that can unwind into a frame with cleanup goes through an `invoke_*` function of the script, which looks the target up in the function table and calls back into the module (`invoke_*`, `getWasmTableEntry`, `stackSave`, the wasm-to-js and js-to-wasm transitions). Attributed to the nearest Rust caller, 2.5 s of that is in xamlx and 1.0 s in the loader crate; their own code is another 1.3 s and 0.8 s. The run-time compilation of the theme documents by the XAML loader is therefore about 5.6 s of the first frame; downloading and compiling the module is about 0.3 s from a local server.

This matters for the size settings: smaller code inlines less, makes more calls and therefore more trampoline calls (section 3.1).

## 3. Reductions applied

One commit each, in this order. Sizes are of the module and script as the site serves them; first frame as in section 1.

| Step | `.wasm` raw | gzip -9 | brotli -11 | `.js` raw | First frame WebGL2 / Software2D |
|---|---:|---:|---:|---:|---:|
| Baseline (`release`) | 48,664,055 | 10,250,979 | 6,266,459 | 193,759 | 6.48 / 6.29 s |
| 1. `browser` profile: opt-level z, XAML crates at 3, link at -O2 | 38,501,779 | 8,310,184 | 5,406,940 | 206,348 | 6.72 / 6.76 s |
| 2. fat LTO | 36,416,171 | 8,254,259 | 5,338,998 | 201,361 | 6.64 / 6.50 s |
| 3. one code generation unit | 35,864,231 | 8,086,486 | 5,280,743 | 201,596 | 6.26 / 6.29 s |
| 4. `-sENVIRONMENT=web` | 35,864,231 | 8,086,486 | 5,280,743 | 199,714 | 6.16 / 6.14 s |
| 5. `wasm-opt -Oz` (Binaryen 132.0.0 from npm) | 27,114,727 | 7,817,664 | 5,148,143 | 199,714 | 6.27 / 6.31 s |
| 6. precompressed `.gz` and `.br` next to the files | 27,114,727 | 7,817,664 | 5,148,143 | 199,714 | 6.36 / 6.18 s (6.16 s served with brotli) |

### 3.1 The `browser` profile

`[profile.browser]` in the workspace manifest inherits `release` (which this pass does not change) and sets `opt-level = "z"`, `lto = "fat"` and `codegen-units = 1`. `xamlx`, `ferroui-markup-xaml-loader`, `ferroui-markup-xaml` and `roxmltree` keep `opt-level = 3` through `[profile.browser.package.*]`. `scripts/build-browser.sh` builds this profile by default.

Measured alternatives (thin LTO, 16 units, before `wasm-opt -Oz`):

| Rust opt-level | emcc link level | `.wasm` raw | gzip -9 | First frame |
|---|---|---:|---:|---:|
| 3 (release) | -O3 | 48,664,055 | 10,250,979 | 6.3 s |
| s | -Os | 40,573,734 | 8,783,141 | 9.3 s |
| z | -Oz | 27,479,525 | 7,932,428 | 13.5 s |
| z, XAML crates at 3 | -Oz | 28,477,130 | 8,236,828 | 9.1-9.3 s |
| s, XAML crates at 3 | -Os | 41,107,912 | 8,983,696 | 9.3 s |
| z, XAML crates at 3 | -O3 | 37,110,494 | 8,498,567 | 7.2 s |
| z, XAML crates at 3 | -Os | 36,868,709 | 8,434,901 | 8.9 s |
| z, XAML crates at 3 | -O2 (chosen) | 38,501,779 | 8,310,184 | 6.8-7.7 s |
| z everywhere | -O2 | 37,363,534 | 8,010,381 | 9.9 s |

(First frames of this table are medians of 3 or 5 loads in the sessions where each was built; section 1 has the consistent series.)

Two findings decide the settings:

- **rustc passes its opt-level to the Emscripten linker** (`-Os`, `-Oz`, `-O3`). At `-Os` and `-Oz` Emscripten sets `SHRINK_LEVEL` and leaves out the cache (`wasmTableMirror`) of `getWasmTableEntry` (`src/lib/libcore.js`, `#if SHRINK_LEVEL == 0`). Every `invoke_*` trampoline then calls `wasmTable.get`, which costs 2.8 s of the first frame of this example (CPU profile of the `-Oz` link: 3.2 s in `get` against 0.02 s with the cache). The build script therefore adds `-Clink-arg=-O2` for the `browser` profile (the last `-O` wins), and the size optimisation of Binaryen runs afterwards as its own step (3.5). This link argument lives in the build script because a profile cannot carry link arguments on stable cargo and `.cargo/config.toml` would also apply it to the dev profile.
- **opt-level z everywhere slows the XAML loader**: 9.9 s against 6.8 s for 1.1 MB less. The four crates that compile the theme at start-up stay at opt-level 3.

Fat LTO: −2,085,608 bytes; one code generation unit: −551,940 bytes more. The build of the profile from scratch takes 670 s on the 4-core machine of this pass (the release profile: 470 s).

### 3.2 Panic strategy, strip, debug information

- `panic = "abort"` does not build on the pinned toolchain: `error: the crate core requires panic strategy unwind which is incompatible with this crate's strategy of abort` (Rust 1.90.0, with the profile setting and with `-Cpanic=abort`). Not applied; see section 4.
- `strip` has no effect on this target: rustc's Emscripten linker driver passes only `-g0` (for `debug = 0`) and ignores `strip`. The published module already has no name section, no DWARF and no producers or target-features section. Nothing to drop.
- Dead stripping: `wasm-ld` runs with `--gc-sections` by default; the map shows no PDF or SVG code of the Skia archive in the module.

### 3.3 Linker flags

- `-sENVIRONMENT=web` (in `.cargo/config.toml`): the script leaves out the Node.js and worker branches. Script 201,596 → 199,714 bytes (gzip 38,609 → 38,018); the module is unchanged. The module runs in the main thread of a page (`browser-platform.md`, section 6), so nothing that is supported changes.
- `-sDYNAMIC_EXECUTION=0` was measured and changes nothing: the script has no `eval`. Not added.
- Settings that would change behaviour are in section 4 (`FILESYSTEM=0`, `INCOMING_MODULE_JS_API`, `MALLOC=emmalloc`, Closure).

### 3.4 Debug names

Nothing to do: see 3.2. The build script never adds `--profiling-funcs`; the measurements of section 2 used separate links.

### 3.5 `wasm-opt -Oz`

`scripts/build-browser.sh` runs `wasm-opt` of Binaryen 132.0.0 (`scripts/browser/package.json`, installed with `npm ci`; the Binaryen version of Emscripten 6.0.10) on the module of the site with `-Oz` and the settings and features emcc uses for its own run (`--low-memory-unused --zero-filled-memory --pass-arg=directize-initial-contents-immutable`, MVP plus threads, bulk memory, multivalue, mutable globals, non-trapping float-to-int, reference types, sign extension). 35,864,231 → 27,114,727 bytes. The JavaScript build of `wasm-opt` takes about 7.5 minutes for this module on the machine of this pass (the native binary of the SDK: about 1 minute).

Most of the reduction is Binaryen's `merge-similar-functions`, which `-Oz` runs: on an earlier build (thin LTO), `-Oz` took the module from 38,501,779 to 28,711,504 bytes, and to 37,133,394 bytes with `--skip-pass=merge-similar-functions`. It merges the many near-identical instances of the generic property code (section 2.4). It did not slow the first frame (6.9 s against 7.1 s without the pass, same session).

### 3.6 Precompressed files

For the browser profile the build script writes `<file>.gz` (`gzip -9`) and `<file>.br` (brotli quality 11) next to the `.wasm`, `.js`, `.html`, `.css` and `.map` files (`scripts/browser/compress.mjs`). A server that serves them with `Content-Encoding` transfers 7.8 MB (gzip) or 5.1 MB (brotli) instead of 27.1 MB and does not compress on each request. Headless Chromium accepts both over `http://127.0.0.1`; the page renders the same (`first-frame.mjs --encoding br|gzip`). First frame from the local server: 6.16 s with brotli against 6.36 s uncompressed (WebGL2, median of 7, same session; the local server has no bandwidth limit, so this shows only that decompression costs nothing measurable).

### 3.7 Streaming instantiation

Already in place: the script Emscripten generates fetches the module and calls `WebAssembly.instantiateStreaming`, falling back to `instantiate` only if that fails. `first-frame.mjs` watches the console for the fallback message (`wasm streaming compile failed`); it did not appear in any load of any build. Requirement for hosting: the module must be served as `application/wasm`. The host page (`main.js`) needs no change.

### 3.8 What the module is made of after this pass

The same analysis on the final module (linked once more with names; fat LTO puts all Rust code in one object, so Rust is shown by the crate in the function path, where a generic function counts for the crate that defines it):

| | Before | After |
|---|---:|---:|
| code section | 43,368,975 | 22,268,696 |
| data section | 4,944,933 | 4,513,995 |
| Rust code, ferroui_base path | 23,701,412 | 7,958,306 |
| Rust code, core/alloc/std/hashbrown path and impls on primitive types (closures, drop glue, collections) | 8,213,048 | 3,736,118 |
| Rust code, ferroui_controls path | 3,501,401 | 2,029,836 |
| Rust code, XAML loader, xamlx, roxmltree | 2,147,944 | 1,970,025 |
| Rust code, markup metadata tables (`markup_types::`) | 1,211,339 | 1,157,568 |
| Skia with its third-party libraries | 4,152,169 | 4,046,586 |
| HarfBuzz | 591,460 | 346,234 |
| not attributed (merged by `wasm-opt`) | 578,618 | 1,373,408 |

Native code barely changes (Skia is prebuilt; HarfBuzz is compiled by `cc` at the opt-level of the profile); the Rust code shrinks by more than half. The markup metadata tables, straight-line registration code, do not shrink. Skia, HarfBuzz, the fonts and the theme documents are now about a quarter of the module.

## 4. Reductions that need a decision or other work

Estimates are of the final module of this pass (27.1 MB raw, 7.8 MB gzip, 5.1 MB brotli) unless stated.

| Reduction | Estimated saving | What it takes |
|---|---|---|
| **Type-erased core of the property system.** Move the value-type independent part of `BindingEntry`, `EffectiveValue`, `LocalValueBindingObserver`, `StyledProperty` routes and `ValueTypes::register_*` out of the generic functions into non-generic ones over boxed values, leaving thin typed shims. | Largest single lever: these families are 14.9 MB of 36.1 MB Rust code before, about 7-8 MB of the final code (ferroui_base path). Halving the per-type code would save roughly 3-4 MB raw, 1 MB gzip; not measured. | A change of `ferroui_base` against the porting rule of mirroring upstream's typed generics; needs an owner decision and a design that keeps behaviour and the tests. |
| **Remove the run-time XAML loader once the ahead-of-time compiler exists** (`xaml.md`). | Code of xamlx, the loader crate and roxmltree: 1.97 MB of the final module. If compiled markup does not need the markup metadata tables at run time (`markup_types::register_value_types` and its closures, 1.16 MB of the final module) they go as well: about 3.1 MB raw together, roughly 0.9 MB gzip (not measured). The theme documents (0.76 MB raw, 73 kB gzip) become compiled code instead. The larger gain is start-up: the run-time compilation of the theme is about 5.6 s of the 6.3 s to the first frame (section 2.6). | The ahead-of-time compiler, and a decision whether the browser build keeps the run-time loader as an option (`AvaloniaXamlLoader.Load` of strings at run time). |
| **Lazy loading of theme documents.** Load the control themes of the Fluent theme when a control first needs one instead of compiling all of them at start-up. | Size: little (the documents are 73 kB of the gzip size). Start-up: most of the 5.6 s of run-time compilation is for control themes the example does not show; not measured. | A change of the theme loading against upstream's merged dictionaries; moot once the ahead-of-time compiler exists. |
| **Font subsetting.** | Measured with fontTools 4.60.1 (`pyftsubset`, all layout features, U+0000-024F, U+2000-206F, €, ™, U+FFFD): the six Inter weights go from 1,878,122 to 610,352 bytes raw, 928,325 to 299,382 gzip: −1.27 MB raw, −0.63 MB gzip (8% of the gzip size). The Fluent theme documents ask for the weights Normal, SemiLight, Medium and SemiBold; leaving out Thin and Bold saves 626,616 bytes raw, 308,719 gzip, but applications that ask for those weights would get the nearest weight or a synthesized one. | A decision on the script coverage of the default font (text outside the subset falls back to another family or to missing glyphs), and a pinned subsetting step in the build of `ferroui-fonts-inter`; upstream ships the full fonts. Alternatively fetch the font at run time (`browser-platform.md`, section 9) so that it is cached separately. |
| **A Skia build with fewer features.** | Image codecs and encoders with libjpeg-turbo, libpng, zlib and Wuffs are about 0.7 MB of code; image filters 0.14 MB; picture recording 0.08 MB. A build without JPEG and with only the image formats FerroUI decodes would save about 0.3-0.5 MB raw (0.1-0.2 MB gzip). SkSL (0.5 MB) and Ganesh are needed for WebGL. | rust-skia has no prebuilt binary with fewer features for this target; a custom Skia build (from source, as `browser-platform.md` forbids for CI today) or a custom binary hosted for the build. Owner decision. |
| **Splitting the module** (Emscripten `-sSPLIT_MODULE` / `wasm-split`). | Not measured. The first frame needs the framework, the theme and the renderer; what can be deferred is code no page of the example runs (Software2D or WebGL, unused controls and codecs). | A profiling run to choose the split, loading of the secondary module, and checking that it works with the `wasm-bindgen` integration of Emscripten (not documented). |
| **WebAssembly exception handling** instead of Emscripten's JavaScript trampolines. | Start-up: the trampolines are 3.9 s of the 7.3 s profile. Size: the `invoke_*` call sites and their landing pads; not measured. | Rust's `-Zemscripten-wasm-eh` is unstable; needs a newer or nightly toolchain, or waiting for stabilisation. |
| **`panic = "abort"`.** | Removes landing pads and drop paths of unwinding; not measured (does not build). | Does not build with Rust 1.90.0 on this target (3.2); needs a toolchain where the target supports it, and the `catch_unwind` uses of the core reviewed (`browser-platform.md`, section 12.13). |
| **Brotli on the server.** | 5.1 MB instead of 7.8 MB transferred (−34%). | Hosting that serves `.br` files (section 3.6): a deployment decision, no code. |
| **Smaller Emscripten runtime.** `-sFILESYSTEM=0`; `-sINCOMING_MODULE_JS_API=[]` (or only what the host page uses); `-sMALLOC=emmalloc`; `--closure 1`. | The script is 38 kB gzip, so at most about 20 kB from the first, second and fourth; `emmalloc` saves part of dlmalloc's 10 kB. | `std::fs` is used by `ferroui_base` (storage, font collections, bitmaps): without the file system those calls fail instead of using the in-memory file system. Restricting the module API removes options such as `locateFile` from embedders. `emmalloc` changes allocation performance. Closure needs checking against the `wasm-bindgen` glue. |

## 5. Not changed

- The workspace `[profile.release]` and the Skia feature set of every target.
- The behaviour of the page: every step renders `themed_view` pixel-identical to the release build in both modes (section 6).

## 6. How it was measured

Machine: the Linux container of this pass, 4 cores (Intel Xeon, 2.8 GHz), 15 GB. Toolchain as in CI: Emscripten 6.0.10, Rust 1.90.0, `wasm-bindgen` 0.2.129 (command-line tool and crate), Node 22, Chromium 141.0.7390.37 (Playwright build). Binaryen 132.0.0 from npm for step 5; `wasm-tools` 1.239.0 and `twiggy` 0.7.0 (installed with `cargo install --locked`) for inspection.

Scripts (no dependencies beyond Node, Python 3 and gzip):

```sh
scripts/build-browser.sh themed_view                       # browser profile; --profile release for the baseline
node scripts/browser/size-report.mjs target/browser/themed_view
node scripts/browser/first-frame.mjs target/browser/themed_view --runs 7 [--query '?RenderingMode=Software2D'] \
    [--screenshot ref.png | --compare ref.png] [--expect-mode webgl2|2d] [--encoding br|gzip] [--cpu-profile f.cpuprofile]
python3 scripts/browser/wasm-breakdown.py <module with names>.wasm <link>.map
```

- **Sizes**: `size-report.mjs` prints raw bytes, `gzip -9` (the system command) and brotli quality 11 (Node's zlib, default window).
- **First frame**: `first-frame.mjs` serves the site from a local HTTP server (`application/wasm`, no caching), starts headless Chromium with a fresh profile per load (no code cache), SwiftShader for WebGL and an 800×600 viewport at scale 1, and records from the start of navigation: the end of the module download, the first draw call of the framework (a WebGL draw call or `putImageData`), the first animation frame after it (the frame on screen) and the closing of the splash screen. SwiftShader is a "major performance caveat", so the script lets the WebGL context be created although the page asks for `failIfMajorPerformanceCaveat`. The reported value is the median of the loads. Timings vary by about ±0.5 s between sessions on this machine; the series in section 1 was measured back to back.
- **Rendering check**: after the last load the script waits until two screenshots a second apart are equal and compares the page with a reference screenshot of the release build, pixel by pixel; any difference fails. It also fails when the page reports an error or when the first canvas context is not of the expected type (`webgl2` without a query, `2d` with `?RenderingMode=Software2D`).
- **Attribution**: link the example once more with names and the map, without changing anything else:

  ```sh
  cargo rustc --profile browser --target wasm32-unknown-emscripten -p ferroui-browser --example themed_view -- \
      -Clink-arg=-O2 -Clink-arg=--profiling-funcs -Clink-arg=-Wl,--Map=link.map
  wasm-opt -g -Oz <flags of scripts/build-browser.sh> themed_view.wasm -o named.wasm   # browser profile only
  python3 scripts/browser/wasm-breakdown.py named.wasm link.map
  ```

  For the families of section 2.4 the build additionally used `CARGO_TARGET_WASM32_UNKNOWN_EMSCRIPTEN_RUSTFLAGS=-Csymbol-mangling-version=v0` (in a separate target directory).
