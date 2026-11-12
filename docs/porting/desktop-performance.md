# FerroUI binary size and startup time: measurements (P1), proposals (P2) and results (P3)

Commit measured: `48507636f39523f8fd7dc74f7690ab1f1f9aff9f` (main).
Machine: Apple M3 Pro (11 cores, 18 GB), macOS 26.6 (25G72), rustc 1.90.0, release profile
`lto = "thin"`, `codegen-units = 4`.

The machine was shared with three other workers during the whole session. Every build and every
timing run was made while holding the shared cargo lock, so no other *build* ran concurrently, but
the load average never dropped below 4 (it is printed with each measurement). Timings are medians
of 7 or 9 warm launches; a difference below about 5 % between two configurations is noise.

Everything in this report is **measured** unless a line says *estimated* or *not verified*.
Sections 0 to 5 are the measurement pass on `main` at the commit above; section 6 is the
optimisation pass that followed it, with the before and after numbers.

## 0. Method and tools

All tools are private to the work area (`/private/tmp/claude-501/ferroui-work-perf-bin/tools`) and
nothing was added to the product code for the measurements.

| Tool | Purpose |
|---|---|
| `symsize.py` | Per-symbol size attribution from the symbol table of the unstripped binary (`nm -n`, size = distance to the next symbol in the section; sections from `otool -l`). Rust symbols are attributed to the crate named first in their path; C / C++ / Objective-C symbols to the static library (and, for Skia, the build target of the archive member) that defines them (`nm -A` of `libskia.a`, `libskia-bindings.a`, `libembedded_harfbuzz.a`, `libferro_native_osx.a`). |
| `modsize.py`, `v0size.py` | Per-module breakdown; `v0size.py` works on a second build made with `-C symbol-mangling-version=v0`, whose names keep the generic arguments (demangled with the system `c++filt`), so instantiations can be attributed to the type they are made for and closures to the module that defines them. |
| `ehcat.py` | Attributes the entries of `__eh_frame` (`dwarfdump --eh-frame`) to code families. |
| `perfhook.dylib` | A measurement-only interposer loaded with `DYLD_INSERT_LIBRARIES`. It prints the time since process start (`kinfo_proc.p_starttime`) of: its own constructor (dyld has finished loading; no code of the application has run), `+[NSApplication sharedApplication]`, `MTLCreateSystemDefaultDevice`, `-[NSWindow makeKeyAndOrderFront:]`, `-[NSApplication run]`, `-[NSApplication finishLaunching]`, the first `-[CAMetalLayer nextDrawable]` and the first `-[CAMetalDrawable present]`. |
| `startup.py` | Launches a binary N times with `FERROUI_SMOKE_EXIT_MS`, timestamps the marker line (`Window opened` / `App activated`) on its own clock and collects the hook lines; prints median / min / max and the load average. |
| `susp` + `sample` | `sample -wait` attaches too late to see the start of a process, so `susp` starts the program suspended (`POSIX_SPAWN_START_SUSPENDED`), `sample <pid> 6 1 -mayDie` attaches (1 ms interval), and the process is resumed. `sampleflat.py` reconstructs the per-stack self samples and sums them per phase (root-first) and per kind of work (deepest framework frame). |

Limits of the method:

* `DYLD_PRINT_STATISTICS` prints nothing on this macOS version (dyld 4 dropped it). The time before
  `main` is bounded by the hook constructor instead.
* `xctrace` was not used: `sample` from a suspended launch gave the full call tree at lower cost.
* **Time on glass could not be measured.** The presented handler of the first drawables reported
  `presentedTime == 0` on every run (the session has no visible display), so the last milestone is
  the return of the first `present` call, not the vertical blank at which the frame became visible.
* While `sample` is attached, every `dlopen` blocks in `dyld4::RuntimeState::notifyDebuggerLoad`
  (170 to 190 ms per run in total). Those stacks are excluded from the tables as a sampling
  artifact; absolute times therefore come from the hook runs, the *split* from the sample runs.
* Symbol-table attribution covers code and named data exactly. Private constants of Rust
  (`l_anon...` are not in the symbol table) are merged into the preceding named symbol; the
  "anonymous constant data" row is therefore a pool of string literals, tables and every
  `include_bytes!` asset, and is only split by the known sizes of the embedded files.

## 1. Size

Stripped sizes (`strip`, then ad-hoc re-signed): hello_window **22.77 MB**, themed_window
**48.48 MB**, control-catalog-desktop **85.32 MB**.

### 1.1 By component (MB of the unstripped binary's sections, and share)

| Component | hello_window | themed_window | control-catalog |
|---|---|---|---|
| ferroui-base (object model, property system, bindings, media, compositor) | 8.52 (38.7 %) | 18.11 (38.6 %) | 22.13 (26.5 %) |
| core / alloc / std / hashbrown instantiations (drop glue, closures, collections) | 2.81 (12.8 %) | 7.10 (15.1 %) | 9.36 (11.2 %) |
| ferroui-controls | 1.19 (5.4 %) | 3.43 (7.3 %) | 3.60 (4.3 %) |
| XAML pipeline code: xamlx + loader + markup-xaml + markup + roxmltree | 0 | 1.50 (3.2 %) | 1.51 (1.8 %) |
| themes and application crates (code only) | 0.01 | 0.04 | 1.28 (1.5 %) |
| Unwind tables (`__eh_frame`, `__unwind_info`, `__gcc_except_tab`) | 3.54 (16.1 %) | 8.24 (17.6 %) | 10.46 (12.5 %) |
| Anonymous constant data (embedded assets and markup, string literals, tables) | 0.89 (4.0 %) | 3.41 (7.3 %) | 30.03 (36.0 %) |
| Skia: core + utilities | 2.05 (9.3 %) | 2.09 | 2.11 |
| Skia: Graphite + SkSL + shared GPU code | 0.77 (3.5 %) | 0.77 | 0.77 |
| Skia: image codecs (libjpeg-turbo 0.51, libpng 0.14, wuffs 0.10, zlib 0.05) | 0.82 (3.7 %) | 0.82 | 0.82 |
| Skia: font manager (CoreText) | 0.03 | 0.03 | 0.03 |
| Skia: PDF back end | 0.00 | 0.00 | 0.00 |
| ICU code or data | 0.00 | 0.00 | 0.00 |
| HarfBuzz | 0.74 (3.4 %) | 0.75 | 0.77 |
| Native platform library (Objective-C++) + its Rust interop | 0.48 (2.2 %) | 0.50 | 0.50 |
| ferroui-skia / ferroui-harfbuzz (Rust back ends) | 0.11 | 0.11 | 0.11 |
| std runtime support (backtrace symbolisation, decimal) | 0.06 | 0.07 | 0.07 |
| Sections total | 22.01 | 46.91 | 83.42 |

Observations:

* **Skia is not the problem.** All of Skia is 3.7 MB, constant across the three applications. The
  PDF back end and the JPEG encoder are enabled as crate features but nothing references them, so
  the linker drops them completely (0 bytes of `pdf.*` members survive). ICU is not linked at all
  (`embed-icudtl` only matters with the `textlayout` feature). No Skia font data is embedded; the
  Inter font crate is not linked by any of the three applications.
* **The framework's own generic code is the problem.** ferroui-base plus the standard-library
  instantiations made for its types are 51 to 54 % of hello_window and themed_window.
* **Unwind tables are 16 to 18 % of the code-only binaries.** See 1.4.
* **The catalog is one third assets**: 18.06 MB JPEG, 5.30 MB TrueType (4.3 MB of it one CJK font),
  0.62 MB PNG, 0.37 MB ICO, 1.26 MB XAML of the sample and 1.15 MB XAML of the two themes, stored
  uncompressed. The upstream sample embeds the same files uncompressed.
* The embedded theme markup in themed_window is 1.15 MB (Simple 0.41 MB in 81 documents, Fluent
  0.75 MB in 86 documents; the example links both themes).

### 1.2 Where ferroui-base goes (themed_window, text only, 17.55 MB by first path component)

| Module | MB | Functions |
|---|---|---|
| `property_store` (`EffectiveValue<T>`, `ValueStore` generic methods, `BindingEntry<T>`, `LocalValueBindingObserver<T>`) | 6.14 | 16 491 |
| `data` | 4.53 | 9 479 |
| of which `data::core::value_type::ValueTypes` (conversion registry: registration code and conversion closures) | 2.58 | 5 640 |
| of which `TypedBindingExpression<T>` + `TypedClrPropertyInfo` | 0.97 | 2 549 |
| of which `BindingNotification::to_binding_value<T>` | 0.47 | 306 |
| `styled_property` (`StyledProperty<T>` routes) | 1.20 | 4 329 |
| `media`, `animation`, `ferro_object`, `metadata`, `ferro_property`, `ferro_property_metadata`, `direct_property`, `rendering` | 0.33 to 0.57 each | |

With the generic arguments visible (v0 build, attribution to the type an instantiation is made
for): `property_store` 6.80 MB, `data` 6.06 MB (the value type registry alone 3.67 MB including its
closures and their drop glue), `styled_property` 1.24 MB.

The property system is instantiated for **216 value types** in themed_window (219 in the catalog):
112 structs and enums, 46 `Option<value>`, 24 `Rc<dyn Trait>` / `Option<Rc<dyn Trait>>`,
17 `Option<Ref<class>>`, 11 primitives, 6 `Rc<T>`. Each instantiation of the store machinery costs
roughly 35 kB of code. The managed original shares one instantiation among all reference types;
here about 47 of the 216 are reference-like handles that each get their own copy.

### 1.3 Largest generic families (themed_window; type arguments erased)

| Family | Instances | kB |
|---|---|---|
| `core::ptr::drop_in_place<_>` | 11 926 | 1 936 |
| closures `FnOnce::call_once` (+ vtable shims) | 28 511 | 2 374 |
| `ValueTypes::register_object<T>` | 345 | 1 101 |
| `LocalKey::with` (nearly all of them closures passed to the value type registry) | 3 689 | 689 |
| `BindingEntry<T>::set_value` | 264 | 595 |
| `on_next` of the typed observers | 1 910 | 563 |
| `BindingNotification::to_binding_value<T>` | 306 | 466 |
| `EffectiveValue<T>::coerce_value` / `dispose_and_raise_unset` / `set_and_raise_core` | 264 each | 349 / 336 / 271 |
| `AnyValue::any_value_eq` | 1 991 | 332 |
| `ValueTypes::register_conversion` closures / `register_cast` closures / `register_nullable::unwrap` | 1 053 / 1 009 / 815 | 316 / 254 / 191 |
| `MarkupArguments::next<T>` | 301 | 279 |
| `register_class::init` / `__register_properties` | 343 / 188 | 229 / 210 |
| `into_markup_value<T>` | 307 | 164 |

Closures by defining item: the `MARKUP` metadata tables (property getters, setters, constructors
and methods of the markup type tables) 1.05 MB, the class vtable builders 0.46 MB.

**Markup metadata and untyped value conversion together** (value type registry 3.67 MB, `MARKUP`
closures 1.05 MB, `metadata::markup_type` 0.47 MB, `markup_types` tables 0.37 MB, class
registration 0.44 MB) are about **6 MB of themed_window (12 %)**, four times the code of the whole
XAML compiler and interpreter (1.5 MB). Removing the run-time parse therefore removes little code
by itself; the tables are what a run-time type lookup by name keeps alive (see P2, item 9).

### 1.4 Unwind tables

| Section | hello_window | themed_window | control-catalog |
|---|---|---|---|
| `__eh_frame` | 2.08 | 4.82 | 6.12 |
| `__gcc_except_tab` (landing pad tables) | 1.03 | 2.45 | 3.10 |
| `__unwind_info` (compact unwind) | 0.43 | 0.97 | 1.24 |

`__eh_frame` in hello_window holds 29 399 FDEs: 29 370 for Rust functions (of 49 118), 6 for the
13 000 C, C++ and Objective-C functions. 26 949 of the Rust FDEs describe a standard frame (frame
pointer set up at entry, callee-saved pairs in order) that the compact unwind encoding can express.
They are emitted anyway because rustc requests *asynchronous* unwind tables: the epilogue CFI
(`DW_CFA_remember_state` / `DW_CFA_restore_state`) cannot be encoded compactly, so LLVM falls back
to DWARF for every function that has an epilogue. clang requests synchronous tables on this target,
which is why the C++ code needs none. Verified on a small program: with
`-Z use-sync-unwind=yes` the FDEs of all its own functions disappear. The flag is unstable (see P2,
item 6).

## 2. Startup (warm)

### 2.1 Timeline (ms since process start, median of 9, load average 4.5 to 5.6)

| Milestone | hello_window | themed (Simple) | themed (Fluent) | control-catalog |
|---|---|---|---|---|
| dyld finished, no application code has run | 16 | 18 | 17 | 14 |
| `+[NSApplication sharedApplication]` entered (start of platform init) | 39 | 40 | 40 | 32 |
| `MTLCreateSystemDefaultDevice` entered / returned | 94 / 102 | 95 / 102 | 97 / 104 | 82 / 89 |
| main window ordered front (the application's "window opened") | 178 | 394 | 510 | 733 |
| `-[NSApplication run]` entered | 182 | 396 | 510 | 734 |
| `finishLaunching` returned | 199 | 412 | 526 | 747 |
| first `nextDrawable` returned (first render starts) | 222 | 432 | 539 | 790 |
| **first `present` returned (first frame submitted)** | **235** | **445** | **552** | **806** |
| marker line seen by the harness (`Window opened` / `App activated`) | 182 | 396 | 510 | 844 |

The first frame is submitted about 50 ms after the window-opened event (70 ms in the catalog). The
catalog prints `App activated` 38 ms *after* its first frame, so its 876 ms baseline overstated the
time to first frame by that much.

The Mach-O has one `__mod_init_func` section of six pointers (C++ static constructors of Skia,
HarfBuzz and the native library) and 27 kB of rebase information: static initialisation is not
measurable. The 14 to 18 ms before the first instruction of the application are process creation
and dyld loading 20 system dylibs.

### 2.2 Phases of the main thread (ms of wall time from the sample runs; sampling artifact excluded)

| Phase | hello | themed (Simple) | themed (Fluent) | catalog |
|---|---|---|---|---|
| before `main` (dyld) | 13 | 15 | 9 | 20 |
| platform init: native factory, `NSApplication` (56), Metal device (7 to 19), menu exporters | 92 | 76 | 69 | 75 |
| Skia platform init (13 of it `CTFontManagerCopyAvailableFontFamilyNames` in the font manager constructor) | 13 | 13 | 13 | 14 |
| `Application::initialize`: theme and application markup load | 0 | **189** | **260** | **503** |
| main window construction (`NSWindow` creation 25 to 38, window XAML in the catalog) | 44 | 40 | 35 | 49 |
| `Window::show`: styling, templates, first layout, native show | 24 | 39 | 28 | 44 |
| application builder, other | 11 | 12 | 1 | 18 |
| **sum before the run loop** | **197** | **384** | **415** | **723** |
| AppKit `finishLaunching` | 9 | 9 | 8 | 9 |
| run loop: dispatcher jobs (layout and render passes, timers) until exit | 26 | 25 | 19 | 77 |
| run loop: AppKit event handling and Core Animation commits until exit | 134 | 63 | 67 | 238 |

The sums before the run loop agree with the hook timeline (182 / 396 / 510 / 734). The two run-loop
rows cover the whole 1.2 s life of the process, including the close of the window, so only part of
them is before the first frame. Inside them: `-[NSApplication _reopenWindowsAsNecessary...]` (state
restoration, 17 to 35 ms blocked in `open`), `-[NSTextInputContext activate]` (7 to 54 ms), the
first compositor render on the main thread (16 to 65 ms including 5 to 9 ms of Metal pipeline
creation with a warm shader cache).

Registration is not a cost: `register_types()` of all crates 1 to 10 ms, class registration
(`register_class`, property registration) 7 to 12 ms, the value type registry 2 to 13 ms.

### 2.3 Theme and markup load

| | themed (Simple) | themed (Fluent) | catalog (both themes + App.xaml) |
|---|---|---|---|
| `FerroRuntimeXamlLoader::load_*` inclusive | 182 | 257 | 502 |
| parse (XML and markup extensions) | 6 | 17 | 18 |
| transform (compiler passes over the AST) | 151 | 216 | 385 |
| interpret (`Interpreter::populate`, object construction, property assignment) | 16 | 22 | 85 |
| share of the time to the window-opened event | 46 % | 50 % | 68 % |
| time spent in `malloc` / `free` / `realloc` during the whole run | 49 | 61 | 104 |

The transform passes are 80 % of the load. The catalog instantiates both themes at startup
(`App::initialize` reads the `FluentTheme` and `SimpleTheme` resources, as the upstream sample does:
Fluent 252 ms, Simple 159 ms), plus its own application markup. This whole row is what the
ahead-of-time compiler removes; only the "interpret" part (16 to 85 ms) has a counterpart in
compiled markup.

### 2.4 What the framework itself controls

In hello_window, 252 of the 366 sampled milliseconds have no framework frame below the system
frame doing the work: `NSApplication` initialisation, window server round trips, window creation,
Metal device creation, CoreText, state restoration. The framework's own Rust code (class
registration, property system, layout, compositor, text) is about 60 ms. Without markup, startup is
bounded by AppKit; with markup, it is bounded by the run-time XAML load.

## 3. Cold launch

Experiment (file `p1run.log`): a fresh copy of each stripped binary (new inode, page cache warm
because the copy was just written), launched three times with the hook; then the same file
re-signed in place with a new ad-hoc identity and launched twice.

| | hello (22.8 MB) | themed (48.5 MB) | catalog (85.3 MB) |
|---|---|---|---|
| launch 1 of a fresh copy: hook constructor / marker | 519 / 660 | 762 / 1129 | 1079 / 1930 |
| launch 2: hook constructor / marker | 14 / 167 | 15 / 387 | 16 / 831 |
| launch 1 after re-signing: hook constructor / marker | 445 / 606 | 605 / 972 | 820 / 1672 |
| launch 2 after re-signing: hook constructor / marker | 13 / 169 | 12 / 369 | 16 / 841 |
| cold penalty at the marker: fresh copy / after re-signing | 493 / 437 | 742 / 603 | 1099 / 831 |

* **The whole penalty is paid before the first instruction of the process** (the hook constructor
  moves from 14 ms to 0.5 to 1.1 s; the rest of the timeline is unchanged to within 10 ms).
* It is **not page-in**: the file content was in the page cache in every case, and re-signing the
  same warm file brings the penalty back.
* It is **not the Metal shader cache**: nothing after dyld changes, and pipeline creation is 5 to
  9 ms per launch. (Unbundled executables share one system cache directory; a first launch on a
  machine that never compiled these shaders was not measured.)
* It **scales with the size of the file**: about 0.3 s plus 6 to 10 ms per MB. The system log shows one
  `amfid` validation per cold launch (`... not valid: Error Domain=AppleMobileFileIntegrityError
  Code=-423 "The file is adhoc signed or signed by an unknown certificate"`), followed by
  `syspolicyd` notarization lookups: the kernel has the code signature of a new executable
  validated (every page hash of the file) before it runs, and caches the result per file until the
  file changes.

The 2 to 4 s reported for the first launch after a build is this validation of a just-linked
90 MB to 110 MB unstripped binary, plus disk read when the page cache does not hold the file
(*not measured*: `purge` needs root). Every megabyte removed from the binary removes 6 to 10 ms
from each first launch after an install or update; a real distribution (Developer ID signature,
notarization ticket stapled) pays the same hashing once per install.

## 4. Proposals (P2)

### 4.1 Profile experiments (themed_window unless stated; stripped size; startup = marker Simple / Fluent)

| Configuration | Size MB | Change | Startup ms | Clean build | Peak RSS |
|---|---|---|---|---|---|
| baseline: `lto = "thin"`, `codegen-units = 4`, `opt-level = 3` | 48.48 | | 377 to 396 / 483 to 516 | 515 s (all three) | not recorded |
| `opt-level = "s"` (thin, 4 units) | 44.87 | -7.5 % | 415 / 516 (baseline in the same lock hold: 377 / 483) | | 3.4 GB |
| `codegen-units = 1` (thin) | 44.07 | -9.1 % | 397 / 502 | | 4.2 GB |
| **`lto = "fat"`, `codegen-units = 1`** | **41.71** | **-14.0 %** | **388 / 491** | 874 s (all three) | 5.0 GB |
| fat, 1 unit, `opt-level = "s"` for the XAML pipeline crates only | 40.01 | -17.5 % | 390 / 506 | | 5.3 GB |
| fat, 1 unit, `opt-level = "s"` everywhere | 35.52 | -26.7 % | 382 / 500 | | 5.6 GB |
| thin, 4 units, `-Z use-sync-unwind=yes` (unstable flag, measurement only) | 46.36 | -4.4 % | 392 / 506 | | |

All three applications with fat LTO and one unit: hello_window 22.77 -> 19.85 MB (-12.8 %),
themed_window 48.48 -> 41.71 MB (-14.0 %), control-catalog 85.32 -> 77.07 MB (-9.7 %; -14 % of
its non-asset part). Marker 182 / 388 / 491 / 854 ms against 182 / 396 / 510 / 844 ms: unchanged
within noise. Fat LTO with `opt-level = "s"` gives 69.22 MB for the catalog (-18.9 %).

Run-time throughput proxy (the only one available without adding a benchmark: user CPU seconds of
the catalog selecting its 50 pages at 150 ms each, three runs): baseline 1.92 / 2.31 / 1.97,
fat + 1 unit 1.52 / 1.63 / 1.75, fat + 1 unit + `opt-level = "s"` 1.84 / 2.01 / 1.94. The proxy is
noisy (plus or minus 10 %), but it orders the three: fat LTO is no slower than today and probably
faster; `"s"` gives that gain back.

### 4.2 Ranked proposals

| # | Proposal | Measured effect | Risk / cost | Verdict |
|---|---|---|---|---|
| 1 | Release profile `lto = "fat"`, `codegen-units = 1` | -10 to -14 % size on all three, startup unchanged, CPU proxy not worse | Clean release build 1.7 times longer; link needs 5 to 6 GB. The profile is shared with the browser build (`scripts/build-browser.sh --release`), which was **not verified** with it | Pure win on desktop: implemented (P3 patch 1) |
| 2 | Move the type-independent parts of generic functions into non-generic functions (value type registry insertions, binding notification errors, markup value conversion, property registry) | See P3 | None: same statements in the same order | Pure win: implemented (P3 patches 2 and 3) |
| 3 | `opt-level = "s"` for the whole workspace (on top of 1) | A further -15 % (themed 41.7 -> 35.5 MB, catalog 77.1 -> 69.2 MB); startup unchanged | CPU proxy +19 % against fat LTO at level 3, i.e. back to today's level. No layout / render benchmark exists to quantify hot paths | Not a pure win. Owner decision; needs a render and layout benchmark first |
| 4 | `opt-level = "s"` only for xamlx, roxmltree, the markup crates and the themes (on top of 1) | -4 % themed, -0.4 % catalog; startup unchanged within noise | Touches code that is being replaced by the ahead-of-time compiler | Low value; skip until the compiler lands, then re-measure |
| 5 | Synchronous unwind tables | -4.4 % (2.5 MB of `__eh_frame` in themed_window) | Needs `-Z use-sync-unwind`, unstable; only panics (synchronous) unwind in this code base, so nothing would be lost | Blocked on the toolchain. Track the rustc flag; no action now |
| 6 | `panic = "abort"` | Not measured (estimate: the 2.5 MB of `__gcc_except_tab` plus the cleanup code, 8 to 10 %) | **Ruled out.** The framework's behaviour depends on unwinding: `catch_unwind` in `rendering/render_loop.rs:104`, `threading/dispatcher_main_loop.rs:79` and `:117`, `threading/dispatcher_task.rs:431`, around every native callback (`FerroUI.Native/callback_base.rs:23`) and in `clipboard_impl.rs:28`; the dispatcher re-raises panics of operations in their awaiters; 164 `should_panic` tests | No |
| 7 | Strip symbols in the release profile (`strip = "symbols"`) | The files on disk are 29 to 33 % smaller than cargo's default output (themed 68.3 -> 48.5 MB); every size in this report is already the stripped size | Panic backtraces lose function names. Diagnostics change | Do it at packaging time, keeping the unstripped binary or a dSYM; not in the profile |
| 8 | Drop Skia features (`pdf`, `jpeg`) | 0 bytes: the linker already drops the PDF back end and the JPEG encoder; the JPEG decoder is used | The only prebuilt Skia for aarch64-apple-darwin with Graphite is `graphite-jpegd-jpege-metal-pdf`; every other key returns 404, so any change triggers a Skia source build | No |
| 9 | Registration tables keep unused classes alive | An application that calls `register_types()` (every application that loads markup at run time) links the whole control set, all markup metadata and all conversions: ferroui-controls 1.19 -> 3.43 MB, ferroui-base 8.5 -> 18.1 MB between hello_window and themed_window | Inherent to looking types up by name at run time. Once markup is compiled ahead of time and the run-time loader is not linked, the tables can stay unregistered and the linker can drop what the compiled markup does not reference | For the ahead-of-time compiler work: generated code should reference types directly and must not call the crate-wide `register_types()`. Size effect *not measured* |
| 10 | One shared instantiation of the property store machinery for reference-like value types | *Estimated* 2 MB in themed_window (47 of 216 instantiations at about 35 kB each, plus their share of bindings) | A redesign of `EffectiveValue<T>`, `BindingEntry<T>`, `StyledProperty<T>` internals around an erased handle; touches the hot path with the one audited downcast | Worth a design note by the owner of the property system; not attempted |
| 11 | Further outlining inside the property store (`BindingEntry<T>::set_value` 2.2 kB x 264, `EffectiveValue<T>` methods 1.0 to 1.3 kB x 264, `ValueStore` generic methods) | *Estimated* 1 to 2 MB in themed_window | Hot path, port of generic upstream code; every split must keep borrow and re-entrancy order | Candidate follow-up, one function at a time with the property system tests |
| 12 | Lazy work at startup | Nothing found that upstream defers and FerroUI does eagerly: the Skia font manager is created at platform initialisation in both (13 ms, moves to first text measure if deferred), the catalog instantiates both themes in `App.Initialize` in both | | No change |
| 13 | Compress embedded assets | 25.6 MB of the catalog are assets, of which 18 MB already compressed JPEG | Upstream stores them uncompressed | No |

What dominates each number after these proposals:

* **Warm startup of a themed application** is the run-time markup load (46 to 68 % of the time to
  the first window). Nothing in this list touches it; it is the ahead-of-time compiler's to
  remove. The remainder (about 200 ms) is AppKit, the window server and Metal.
* **Cold launch** is code signature validation, proportional to file size: each MB removed saves
  6 to 10 ms on the first launch of a new binary.
* **Size** is monomorphised framework code. Compiler settings recover 10 to 14 % for free and a
  further 15 % at a CPU cost; structural reductions (items 9 to 11) are where the next 20 to 30 %
  would have to come from.

## 5. Commands

```sh
L="lockf -k /private/tmp/claude-501/ferroui-cargo.lock"
T=/private/tmp/claude-501/ferroui-work-perf-bin/tools
# baseline build of the three applications, stripped copies
$L $T/../build3.sh /private/tmp/claude-501/ferroui-work-perf-orig base
# size attribution
python3 $T/symsize.py base/themed_window --top 60 --members --libs skia=<libskia.a> skia-bindings=<libskia-bindings.a> harfbuzz=<libembedded_harfbuzz.a> native=<libferro_native_osx.a>
python3 $T/sizetable.py base/*.symsize.txt
python3 $T/modsize.py base/themed_window 2 ferroui_base 45
dwarfdump --eh-frame base/hello_window > eh.txt && python3 $T/ehcat.py base/hello_window eh.txt
# generic arguments: second build with v0 mangling
$L $T/xbuild.sh <src> x-v0 tc RUSTFLAGS=-Csymbol-mangling-version=v0 && python3 $T/v0size.py x-v0/themed_window 2 60
# timeline (hook) and split (sample from a suspended launch)
$L $T/m.sh base 9
$L $T/p1sample.sh && python3 $T/sampleflat.py p1/sample-hello.txt --thread Main --buckets $T/buckets-phase.tsv --rootfirst
# cold launch
$L $T/p1run.sh
# profile experiments (environment overrides, guarded against a Skia source build)
$L $T/xone.sh <src> x-fat1 htc CARGO_PROFILE_RELEASE_LTO=fat CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1
$L $T/cpu.sh x-fat1
```

Raw outputs are kept next to the tools: `base/*.symsize.txt`, `base/startup-final.txt`,
`p1/startup-buckets.txt`, `p1/startup-incl.txt`, `p1run.log`, `x-*/sizes.txt`, `x-*/startup.txt`,
`x-v0/*.v0.txt`, `xqueue*.log`.

## 6. Optimisation pass (P3): results

Branch `desktop-perf`, rebased on `main` at `4bd90e8`. Every change below is one commit; the
suites of the standing brief pass on the branch (section 6.5).

### 6.1 Before and after (macOS, workflow `.github/workflows/perf.yml`)

Workflow run 37379497054: `main` at `af7ce03` as the baseline, this branch at `17b8392` (its changes
on top of `af7ce03`; the branch was rebased once more afterwards, without changes of its own) as
the ref, both built in the same job on a GitHub `macos-15` runner
(Apple M1, virtual, 3 CPUs, macOS 15.7.9, rustc 1.99.0). The launches started once the load
average had dropped to 1.43; the two builds of each application were launched alternately, 15
warm launches each. Start-up is the median from process start to the window-opened line
(`Window opened`, in the catalog `App activated`); the range is the fastest and the slowest
launch.

| Application | Stripped size before | after | change | Start-up before (range) | after (range) | change |
|---|---|---|---|---|---|---|
| hello_window | 22.94 MB | 18.30 MB | -20.2 % | 124 ms (90 to 129) | 124 ms (96 to 140) | -0.5 % |
| themed_window, Simple theme | 48.95 MB | 37.74 MB | -22.9 % | 381 ms (359 to 429) | 364 ms (352 to 416) | -4.5 % |
| themed_window, Fluent theme | (same file) | | | 519 ms (481 to 591) | 497 ms (470 to 576) | -4.2 % |
| control-catalog-desktop | 86.54 MB | 72.45 MB | -16.3 % | 1236 ms (1155 to 1379) | 1168 ms (1082 to 1343) | -5.5 % |

Load average before and after the launches: 1.43 / 2.80.

* **Size**: 16 to 23 % smaller. On the report's machine `main` measured 22.77 / 48.48 / 85.32 MB;
  the runner builds with a newer rustc and a newer `main` and gets 22.94 / 48.95 / 86.54 MB.
* **Warm start-up**: unchanged to slightly faster. Every median is equal or lower, by less than
  the spread of the launches. That is what the report predicted: warm start-up is AppKit, Metal
  and the run-time markup load, which this pass does not touch. The small gain is consistent with
  the report's CPU proxy, which found fat LTO no slower and probably faster.
* **First launch** of each new file: one launch per file, taken right after the first launch of
  the other build of the same application, so it does not isolate the cost of validating the code
  signature. The numbers are in the job summary and are not used here.
* Two earlier runs measured the two refs one after the other instead of alternately (runs
  37361719427 against `a321ad9` and 37368390723 against `af7ce03`, 9 launches each). The second gives
  the same sizes as the table; the first, against the older `main`, gives changes of -20.3, -23.1
  and -16.3 %. Their start-up medians moved by -2 to +25 % in no consistent direction: the second build was always launched later, on a busier runner. That is
  why the workflow now alternates the launches.

### 6.2 Effect of each change on size (Linux)

Linux x86_64 build of the same three applications (rustc 1.97.0, 4 CPUs, `strip`), measured with
`python3 scripts/perf-report.py --size-only`. The applications build on Linux but cannot open a
window there (no windowing backend), so these are sizes only. Skia is not linked on Linux (it is
only referenced from the macOS branch of `use_platform_detect`); the rows are the framework's own
code, which is what the changes act on. Each row is measured on top of the previous one.

| Change (commit) | hello_window | themed_window | control-catalog-desktop |
|---|---|---|---|
| `main` (`lto = "thin"`, `codegen-units = 4`) | 17.63 MB | 44.57 MB | 84.22 MB |
| Release profile `lto = "fat"`, `codegen-units = 1` | 15.24 MB (-13.5 %) | 38.66 MB (-13.3 %) | 76.81 MB (-8.8 %) |
| Value type registry: insertions in non-generic functions | 14.11 MB (-7.4 %) | 35.00 MB (-9.5 %) | 72.09 MB (-6.1 %) |
| Type-independent parts of generic helpers out of line | 13.52 MB (-4.2 %) | 33.87 MB (-3.2 %) | 70.82 MB (-1.8 %) |
| Error paths of the typed property routes compiled once | 13.41 MB (-0.8 %) | 33.69 MB (-0.5 %) | 70.62 MB (-0.3 %) |
| **Total** | **-23.9 %** | **-24.4 %** | **-16.1 %** |

The value type registry change is larger than the report expected: besides the insertions it takes
the initialisation check of the per-thread table (`with_registry`: table creation, default
registrations, deferred and process-wide registrations) out of every registration function, which
the compiler had inlined into each of them. `ValueTypes::register_object<T>` alone went from 1.71 MB
to 0.16 MB in themed_window (345 instantiations), and the generated `register_value_types`
functions of the markup metadata from 1.19 MB together to 0.14 MB.

The per-commit effect was not measured on macOS: a workflow run for the error-path commit alone was
cancelled to free the runners for CI. Its effect on macOS is contained in the total of 6.1.

### 6.3 Verification of the four changes taken over from the measurement pass

| Change | Verification | Result |
|---|---|---|
| Release profile `lto = "fat"`, `codegen-units = 1` | Desktop: release builds of the three applications (Linux, macOS workflow), all suites. Browser (the profile is shared): `scripts/build-browser.sh themed_view` with the pinned toolchain (Emscripten 6.0.10, Rust 1.90.0, wasm-bindgen 0.2.129) on Linux, page loaded in headless Chromium (SwiftShader WebGL2): the themed view renders as before (title, button, check box, text box, slider, progress bar, list box) | Kept. Browser WASM at the same commit: 44.35 MB (9.94 MB gzip) with thin LTO and 4 units, 39.65 MB (9.42 MB gzip) with fat LTO; the release build takes 12.0 instead of 6.3 minutes on 4 CPUs. Desktop release build of the three applications: 27.5 instead of about 12 minutes on 4 CPUs; the link of the catalog was seen at 4.2 GB of resident memory |
| Value type registry insertions | Read against the previous code: the same insertions in the same order; the only reordering is that `ValueType::of::<T>()` and `T::TYPE` are evaluated before the table is borrowed, and neither touches the table | Kept |
| Generic helpers out of line (`BindingNotification::to_binding_value`, `FerroPropertyRegistry::register` / `register_attached`, `into_markup_value`, `MarkupArguments::next`) | Read against the previous code: same statements, same order, same messages; `property.as_direct()` is now evaluated before the registry is borrowed, and it does not touch the registry | Kept |
| `scripts/perf-report.py` | Ran on Linux (`--size-only`) and on macOS through the workflow | Repaired: a launch that never exits is now killed after the smoke delay plus 60 s (it hung the script before); everything is built before anything is launched and the launches wait for the load average to drop (the first macOS run measured start-up straight after a 13 minute build at a load average of 10 to 15, and its start-up figures were noise); the warm launches go round the applications; with `--baseline-target` the baseline build is launched in the same invocation, alternating with the measured one in every round (two runs that measured the refs one after the other disagreed by up to 27 points on the same medians); the first launch is measured once per file (themed_window Simple and Fluent share one); the load average before and after is recorded with the numbers. Added `--size-only`, `--source`, `--baseline-target`, `--save-baseline`, `--markdown`, `--title` and `--no-fail` |

### 6.4 The ranked proposals after this pass

| # | Proposal | Status |
|---|---|---|
| 1 | Fat LTO, one code generation unit | Done (6.2) |
| 2 | Type-independent parts of generic functions out of line | Done (6.2), and extended to the error paths of the typed property routes (`StyledProperty<T>::from_untyped`, `route_set_value`, `route_set_current_value`, `DirectPropertyBase<T>`, `EffectiveValue<T>::set_local_value_and_raise_untyped`, `BindingEntry<T>`, `ValueStore::set_value<T>`): the messages and panics move into non-generic `#[cold]` functions of `FerroProperty`; the text of every message is unchanged, panics carry `#[track_caller]` so they still report the route that raised them |
| 5, 6 | Unwind tables, panic strategy | Not changed: the framework depends on unwinding (re-checked: `catch_unwind` in `rendering/render_loop.rs`, `threading/dispatcher_invoke.rs`, `FerroUI.Native/callback_base.rs` around every native callback, `FerroUI.Native/clipboard_impl.rs`; `resume_unwind` in `FerroUI.Native/dispatcher_impl.rs` and `clipboard_impl.rs`; `should_panic` and `catch_unwind` tests across the suites), so `panic = "abort"` stays ruled out. Measured instead: stable rustc accepts `-C force-unwind-tables=no` on `aarch64-apple-darwin`; it is what the report wanted from the unstable `-Z use-sync-unwind`. Functions that can unwind keep their unwind information (LLVM emits it for every function that is not `nounwind` or has a landing pad, whatever the attribute), but without the asynchronous epilogue CFI, so the compact encoding applies; `nounwind` functions without a landing pad get none (checked on the assembly of a small crate for `aarch64-apple-darwin`: the epilogue CFI disappears, a leaf function loses its CFI, a function calling `catch_unwind` keeps it). Panics and `catch_unwind` are therefore unaffected by construction; the suites were not run with the flag. What changes is diagnostics: a backtrace (`RUST_BACKTRACE`, the default panic hook) stops at a `nounwind` frame without unwind information, which can be the caller of a function that catches every panic. Effect on macOS (workflow run 37354274751, the branch before the error-path commit built with and without the flag; sizes only, its start-up figures predate the repaired script): hello_window 18.40 -> 17.66 MB (-4.0 %), themed_window 37.77 -> 36.04 MB (-4.6 %), control-catalog-desktop 72.48 -> 70.34 MB (-3.0 %). Applying it means `[target.aarch64-apple-darwin] rustflags = ["-C", "force-unwind-tables=no"]` in `.cargo/config.toml` (it then also applies to the tests, which exercise every catch site). Left to the owner because of the stated condition and the backtrace change |
| 9 | Registration tables keep code alive | Not changed. In hello_window, which loads no markup, the markup tables of the classes it uses (`__MARKUP` closures, markup argument and value conversion) are 0.33 MB of code (Linux, fat LTO), referenced from the static `TypeInfo` of every class. They are not dead: untyped bindings resolve CLR-style members through them (`data/core/plugins/markup_members.rs`) and the value type registry parses text through a class's markup `parse` (`data/core/value_type.rs`), so registering them lazily from `register_types()` would change what a binding in an application without markup can do. The class and value type registration itself (`register_class::init`, `register_object<T>`) runs when a class is first used and is needed by bindings. What the run-time loader keeps alive in themed applications goes away with the ahead-of-time compiler, which this branch does not touch |
| 10, 11 | Shared instantiation for reference-like value types; outlining inside the hot property store methods | Not attempted. After this pass the largest generic families in themed_window (Linux) are `BindingEntry<T>::set_value` (0.43 MB over 218 instantiations, about 2 kB each), `StyledProperty<T>::route_bind` (0.33 MB), `route_set_value` (0.16 MB) and the `EffectiveValue<T>` methods (0.1 to 0.2 MB each); what remains in them depends on `T` (clone, compare, drop of the value, construction of the typed entries), so a further reduction needs the type-erased design of item 10 rather than more outlining |
| 12 | Lazy start-up work | Not changed; re-checked against upstream. The Skia font manager is created when the platform initialises, as upstream does (`FontManagerImpl` holds `SKFontManager.Default`); the family names are only enumerated when asked for (`get_installed_font_family_names`), as upstream. The catalog instantiates both themes in `App::initialize`, as the upstream sample does. The rest of the time to the first window is AppKit, the window server and Metal, and the run-time markup load, which the ahead-of-time compiler removes |
| 3, 4, 7, 8, 13 | `opt-level = "s"`, symbol stripping in the profile, Skia features, asset compression | Unchanged verdicts of section 4.2 |

### 6.5 Suites

On Linux, on the branch rebased on `main` at `4bd90e8` (the suites were also run on the two
earlier bases, `a321ad9` and `af7ce03`, with the same outcome; the error-path commit was run
through the two property system suites on its own):

```text
cargo check --workspace --all-targets --locked                 ok
cargo test -p ferroui-base --lib                               ok. 3831 passed; 0 failed; 0 ignored
cargo test -p ferroui-controls --lib                           ok. 3919 passed; 0 failed; 0 ignored
cargo test -p ferroui-markup -p ferroui-markup-xaml -p ferroui-markup-xaml-loader -p xamlx
                                                               ok. 255 passed; ok. 185 passed;
                                                               ok. 379 passed, 1 ignored; ok. 99 passed
cargo test -p ferroui-markup-xaml-tests                        ok. 560 passed; 0 failed; 15 ignored
cargo test -p ferroui-themes-fluent -p ferroui-themes-simple   ok. 193 passed, 1 ignored; ok. 196 passed, 2 ignored
cargo test -p control-catalog -p mini-mvvm                     ok. 489 passed, 105 ignored; ok. 8 passed
naming check (grep of the standing brief)                      no output
python3 scripts/generate_markup_types.py --upstream <upstream at 1735018> --check
                                                               generated files are up to date
```

The macOS job of CI runs the whole workspace on every push of the branch.

### 6.6 Commands

```sh
# sizes of the Linux build (no window can open there)
python3 scripts/perf-report.py --size-only
# size and start-up on a desktop session (macOS), compared with a stored baseline
python3 scripts/perf-report.py --save before.json
python3 scripts/perf-report.py --check before.json
# the fair comparison: build a second checkout first, then launch both builds alternately
CARGO_TARGET_DIR=$PWD/../before-target python3 scripts/perf-report.py --source ../before --size-only
python3 scripts/perf-report.py --baseline-target ../before-target
# per-family code size of an unstripped Linux build (the analysis of 6.2 and 6.4)
nm -S --size-sort -C target/release/examples/themed_window
```

On GitHub: Actions, workflow "Performance", "Run workflow" with `ref`, optionally `baseline` (a
second ref built and measured in the same job), `runs` (at least 7) and `rustflags` (applied to
the measured ref only, to measure a flag against the same ref without it). The tables go to the
job summary and the numbers to the `perf-report` artifact.

## 7. Release profile for build time, `dist` profile for size (2026-10-07)

The fat-LTO profile of section 6 makes every executable generate the code of the whole program on
one core, so a release build of several applications costs one such link each. The default release
profile is therefore set up for build time, and the profile of section 6 is kept under the name
`dist`:

| Profile | Settings | Use |
|---|---|---|
| `release` | `lto = "thin"`, sixteen code generation units (the default) | Day-to-day release builds: `cargo build --release` |
| `dist` | `lto = "fat"`, `codegen-units = 1` | The executables that are shipped, and size measurements: `cargo build --profile dist`, `python3 scripts/perf-report.py --profile dist` |

The `browser` profile sets its own `lto` and `codegen-units` and is not affected.

Measured on the Apple M3 Pro (11 cores) at `1ab1127`, rustc 1.90.0, a clean build of the seven
desktop executables (`cargo build --release --locked -p control-catalog-desktop -p ferroui-desktop
-p ferroui-themes-simple -p ferroui-native --examples --bins`); the machine was shared with other
work (load average 4 to 8 during the launches), start-up is the median of nine warm launches:

| | `dist` (fat LTO, one unit) | `release` (thin LTO, sixteen units) | Change |
|---|---:|---:|---:|
| Clean build, wall time | 19 min 17 s | 4 min 52 s | -75 % |
| hello_window, stripped | 38.13 MB | 47.87 MB | +25.5 % |
| themed_window, stripped | 52.95 MB | 67.89 MB | +28.2 % |
| control-catalog-desktop, stripped | 87.86 MB | 105.11 MB | +19.6 % |
| hello_window, start-up | 171 ms | 157 ms | within noise |
| themed_window (Simple / Fluent), start-up | 178 / 179 ms | 174 / 178 ms | within noise |
| control-catalog-desktop, start-up | 331 ms | 330 ms | within noise |

The sizes of the earlier sections are those of the `dist` profile. The workflow
`.github/workflows/perf.yml` builds with the release profile of each ref, so a comparison of a ref
before this change with one after it shows the difference of the profiles, not a regression of the
code.

## 8. The cross-framework gauntlet (2026-11-08)

`samples/GauntletBenchmark` is FerroUI's port of the gauntlet and the still wall of the Cranpose
benchmark ([cranpose-comparison.md](cranpose-comparison.md)): the same screens, per-frame work and
`PERF_*` environment as the benchmark's desktop apps, so its harness (`desktop.py`) measures it
beside the other frameworks. `PERF_REPORT=1` prints how often the app's animation-frame
callback runs (`PERF fps`), for A/B runs of FerroUI builds; a build with the feature `counters`
prints beside it the batches the compositor committed (`PERF batches`), which are the frames
(section 9.46: the two differed by a factor of two until that section):

```sh
PERF_SCENARIO=gauntlet PERF_TIER=16 PERF_REPORT=1 PERF_FONTS=<folder of Roboto-*.ttf> \
  cargo run --release -p gauntlet-benchmark
```

Apple M3 Pro, `release`, Skia, own counter, median of the last 8 of 12 seconds, two alternating
rounds:

| Build | Gauntlet (tier 16) | Still wall (tier 20, pointer outside the window) |
|---|---:|---|
| main (d09b86731) | 15.6, 16.6 | 27.0, 29.4 |
| disposal index and hit-test buckets (section 9.1, 9.2) | 19.3, 19.0 | 33.5, 28.4 |

The run of the benchmark's own harness (external frame counter, every framework) needs the display
at 60 Hz, as the benchmark is calibrated; it is added here once made.

### 8.1 The local performance gate

A change to layout, text, the property system, composition, rendering, input or the TreeDataGrid
is measured on the developer's machine against the commit it is based on before it goes up for
review, and the table of the gate goes into the description of the pull request. The gate is not
a CI job: hosted runners are too noisy and too unlike a desktop for a few-percent threshold, so it
is run locally, on a quiet machine (no other build, no VM or emulator, on mains power).

```sh
python3 scripts/perf/gate.py                                   # base: merge-base with origin/main
python3 scripts/perf/gate.py --base main --markdown target/perf-gate.md
python3 scripts/perf/gate.py --only gauntlet                   # the frame gate alone
python3 scripts/perf/gate.py --only frames,framework           # FerroUI's own regression benchmarks
python3 scripts/perf/gate.py --only window --window-scenes text,wall
python3 scripts/perf/gate.py --only tdg --tdg-filter VirtualizationBenchmarks
python3 scripts/perf/gate.py --only counters                   # the counter checks alone
python3 scripts/perf/gate.py --dry-run                         # the plan and the commands
```

The script checks the base out, detached, in a git worktree of its own under the scratch folder
(`--scratch`, default `ferroui-gate-scratch` beside the checkout, reused between runs and safe to
delete), builds the base and the candidate (this checkout as it is, uncommitted changes included)
with a target folder each, then runs them alternately for `--rounds` rounds (default 3), and
compares the medians of the rounds. The side that runs first changes with the round (base then
candidate in the first, candidate then base in the second): what a run leaves behind, the caches
of the system and the heat of the machine, is then not always the start of the same side. A metric over its tolerance by less than the
spread of its own rounds on either side is not decided yet: one disturbed round can move a median
of three that far (a still wall run beside a busy machine went from 1.7 to 2.2 ms on one side
only). The gate then measures another round of that suite, alternately again, until the change is
larger than the spread or `--max-rounds` (default 9) is reached, and decides on the medians of all
the rounds. Cargo runs through `FERROUI_CARGO` when it is
set (a wrapper that serialises the builds of the machine; the runs go through it too, so no other
build starts during a measurement), else `cargo`. The logs of every build and run are kept under
`<scratch>/logs`. Exit status: 0 when nothing regressed beyond its tolerance, 1 when something
did or a benchmark failed on the candidate, 2 when a build or a run failed.

What it measures:

- **The counter checks** (suite `counters`): unit tests that assert exact numbers of units of
  work, read from the performance counters of `ferroui-base`
  (`diagnostics::perf_counters`). The counters compile to nothing without the feature
  `perf-counters`, so the workspace test run of CI builds these tests with their counts switched
  off (`assert_count` asserts only when the counters are compiled in) and checks the behaviour
  alone; the gate builds and runs them with the feature on. The list is `COUNTER_CHECKS` in
  `gate.py` (package, features, test filter); today the style work tests of the property store
  (`src/FerroUI.Base/tests/property_store/style_work_tests.rs`, section 9.25). Alone:

  ```sh
  cargo test -p ferroui-base --features perf-counters --lib style_work
  ```

  The checks are deterministic: they run once, on the candidate only, before the measured
  suites, need no base and no quiet machine, and have no tolerance. A test that fails, a run that
  fails or a filter that selects no test is a row `FAILED` in the table and exit status 1; a
  build that fails is exit status 2. A change that adds a counter-asserting test adds its filter
  to `COUNTER_CHECKS`.
- **The frame gate** (`samples/GauntletBenchmark/tests/frame_gate.rs`): the gauntlet at tier 16
  and the still wall at tier 20 in a headless window of 1280 x 820 points at a scaling of 2, as
  the desktop program opens them, rendered through the compositor into a raster Skia framebuffer
  in the render-thread mode of the desktop (`Compositor::with_render_thread`), with no pointer
  device. The screens drive their own frames through the top level's animation frame callback,
  which advances the frame index once a frame, so nothing depends on wall time; 60 frames of
  warm-up, then 240 measured (`--warmup`, `--frames`). Per scenario: the UI thread's busy time a
  frame (animation callback, layout, recording, commit), the render thread's time a frame, and
  the wall time a frame. Alone:

  ```sh
  cargo test -p gauntlet-benchmark --release --test frame_gate -- --ignored --nocapture --test-threads=1
  ```

  When the base has no frame gate (a commit before it), the candidate's `samples/GauntletBenchmark`
  is copied into the base worktree, and added to its workspace members when needed, so both sides
  run the same benchmark code; the output says so. `--copy-gate` does the same when the base has
  a gate of its own.
- **The frame benchmarks** (`tests/FerroUI.FrameBenchmarks`, suite `frames`): FerroUI's own
  regression scenes, kept apart from the ports of other frameworks' benchmarks (the gauntlet
  follows the Cranpose benchmark and is synced with it). Each scene does one kind of per-frame work
  of a heavy screen, so that a change of that work shows in its own numbers, and each names the
  sections of chapter 9 it guards: `text` (text that changes every frame), `paragraph` (a long
  paragraph one word of which changes every frame), `list` (a scrolling list of shadowed cards),
  `transforms` (panels moved by their render transform and opacity), `width` (a layout under a
  changing width), `wall` and `wall+pointer` (a still wall), `moves` (tiles of a
  canvas that move), `lines` (charts whose lines change every frame), `items` (boxes drawn as items
  of the composition visual above them, some of which are drawn anew, move, and leave the tree and
  come back every frame), and the per-frame work of the patches of a patchwork screen, each on a
  whole window: `recolor` (a row of the tiles of a wall filled anew), `cards` (cards of tiles that
  move by a transform, one of which leaves the tree, shows another card and comes back when a move
  begins), `reorder` (two children of a wrapping panel of 6,000 that change places), `deep`
  (numbers twelve boxes deep, a row of which changes), `rows` (a list of rows of 150 tiles that
  scrolls a row a frame and recycles none) and `pile` (translucent tiles lying over one another,
  one of which is opaque in turn). The harness is the frame gate's; besides the times a frame,
  every scene reports the time it takes to build (`build_ms`) and the window to show it (`show_ms`: its first
  layout, recording and commit) and their sum (`startup_ms`). Alone:

  ```sh
  cargo test -p ferroui-frame-benchmarks --release --test frames -- --ignored --nocapture --test-threads=1
  ```

  The scenes of the start (`startup/tiles`: 58,696 tiles of six elements on a canvas, 352,187
  elements; `startup/controls`: 1,600 templated controls of the Fluent theme, 24,017 visuals) are
  shown once and measured to their first frame: the build, the show, the time from the show to
  the first frame (`first_frame_ms`) and their sum, and the resident memory each of them added
  (`build_mb`, `show_mb`, `first_frame_mb`, `startup_mb`). `sizes_of_an_element` prints the
  sizes of the objects an element is made of (the element class by class, its composition visual
  and the visual of the render thread); the gate compares each. With the `counters` feature
  (`--features counters`) a start also prints the allocations of each phase and the blocks it
  left alive by size class, and the counters of the framework (section 9.40). The gate runs every
  test of the frame benchmarks in a process of its own: a start is the start of a process that
  has built nothing yet, and a scene leaves the heap and the caches of the process in a state
  that moves what is measured after it. The `text` scene builds in 3.65 ms in a process of the
  frames alone and in 4.2 to 5.0 ms after the two starts; the `list` scene, measured after six
  others in one process, was built in 1.6 ms by one side of a comparison and in 2.7 ms by the
  other, whose scenes before it had left other blocks free, and in 1.2 and 1.1 ms by the two in
  processes of their own (section 9.42). Before the first scene of the frames a
  process measures, the harness shows a small scene made of what the scenes use (text, rounded
  borders, polylines, panels) and runs 30 frames of it, once: what a process does once is then in
  no scene's `build_ms` and `show_ms`. Without it the first scene carried the state the process
  started in: `lines`, the first by its name, was built and shown in 0.9 and 1.4 ms in a process
  started right after another and in 1.3 and 1.9 ms after 20 seconds of rest or 3.0 and 5 to
  6.6 ms after a large process; with it, in 0.8 and 1.3 ms in each case. Before that, and before
  each start, the harness reads one byte of every page of the program's own file
  (`page_in_the_program`): the system reads a page of a program from the disk when the process
  first runs through it, and a build or a show, measured once a process, then timed the disk
  whenever the system did not hold the file, which it does not after it was just written or after
  a large process pushed it out. `list` was built and shown in 1.1 and 1.7 ms with the file in
  memory and in 1.6 and 2.3 ms with about 1,150 pages read inside the two, and of two builds of
  the same scene the larger program paid more of it.

  The UI thread, render thread and wall time the gate compares are those of a frame the surface received
  (the time for a frame of the clock times `frames / rendered` of the `GATE` line, when fewer
  frames were received than the clock had). The loop of the benchmark does not wait for a
  display: a frame of the clock that starts while the render thread still has the batch before
  it commits nothing and costs close to nothing, so a side that presents fewer of its frames
  looked faster by the frames it did not draw (section 9.42: 226 to 228 of 240 frames on one
  side, 237 to 239 on the other, which read as 5 to 7 % of wall time). The UI thread time is
  compared the same way: the work of a frame of the clock that committed nothing is work for
  the picture after it, and a side that ran two frames of the clock for each picture showed
  half its UI thread time for a frame (section 9.46). The frame gate itself fails when the
  surface receives fewer than nine in ten of the frames of a scenario.
- **The tree benchmarks** (`tests/FerroUI.Benchmarks`, category `trees`, suite `framework`): the
  framework's tree mechanics on walls of small tiles, in an application with a desktop's services
  (the Fluent theme, Skia's fonts, HarfBuzz): building, attaching (from the leaves up and from the
  root down), laying out, detaching and clearing; inherited properties, the enabled and visible
  states, the flow direction, the data context, the theme variant and styles passed down an
  attached wall; class styles and resources on it; and the text of many short labels. The suite
  runs `trees::` and `layout::ElementConstructionBenchmark` (`--framework-filter`) with the
  TreeDataGrid suite's iterations. When the base has no `trees` category, the candidate's
  `tests/FerroUI.Benchmarks` is copied into it.
- **The window suite** (`window`): the desktop program of the frame benchmarks (`frame-benchmarks`)
  opened once per scene (`--window-scenes`, default `text,list,transforms,wall`) for
  `--window-seconds` (12): the time from the start of the process to its first animation frame
  (`first_frame_ms`) and to its second (`second_frame_ms`), and its time a frame (`ms_per_frame`,
  from its frame rate after the first three seconds) on the backend of the platform (Skia with
  Graphite on macOS). It measures what the raster frame benchmarks cannot: the GPU backends and the
  start of a real process. The first animation frame comes before the scene has been drawn; the
  first commit of the composition tree, the first drawing and whatever the platform asks to be
  painted when a window is shown keep the thread until the second, which is the time by which the
  scene is on the screen and the application answers. A scene that holds the display's rate shows
  no change of its time a frame; its first frames still do.
- **The TreeDataGrid benchmarks** (`tests/FerroUI.TreeDataGrid.Benchmarks`, `run_benchmarks`): the
  median time of one operation of each benchmark, read from the tables of the harness. The gate
  runs the harness with shorter iterations than its defaults (7 iterations of at least 250 ms
  after 2 of warm-up; `--tdg-iterations`, `--tdg-min-iteration-ms`, `--tdg-warmup`) so that a
  full run of both sides takes minutes rather than an hour; `--tdg-filter` selects benchmarks as
  `FERROUI_BENCH_FILTER` does.

The two suites of the benchmark harness (`framework` and `tdg`) run each class of benchmarks in a
process of its own (`FERROUI_BENCH_CLASS`, one run a class of the names `FERROUI_BENCH_LIST`
prints). A benchmark starts from the heap, the caches and the threads the benchmarks before it in
its process left: `trees::Propagation.font_family[Tiles=2000]` of one build took 1.82 ms in one
process that had run the hundred and more benchmarks before it and 1.40 ms in the next, and is
within a few percent after the benchmarks of its class alone. When the harness of the base is
from before it could run one class, both sides run in one process, and the output says so.

Beside the time, the harness counts the instructions the processor retires for an operation,
where the system counts them for a process (macOS; `harness::processor`), and the table of the
gate has their change in the column `Instructions`, on the line of the time. The count is of the
work alone: it does not move with the frequency of the cores, the caches or where a build put
the code and the data. Of 336 pairs of runs of the same build in two processes the median
difference was 0.01 % and the largest 0.74 %, where the cycles of 49 of the pairs differed by
more than 5 %. The verdicts of a benchmark with the count:

- `REGRESSED`: the time is over its tolerance and so are the instructions. The change does more
  work.
- `REGRESSED (instructions)`: the instructions are over their tolerance and the time is not.
  The change does more work that this machine hides. The instructions are over their tolerance
  when the median of the candidate is 2 % and more than one instruction an operation over that
  of the base, and every round of the candidate counted more than every round of the base: a
  few benchmarks count differently from one process to the next, because the order of a hash
  table follows a seed the process draws (`trees::Propagation.styles_changed` counted 13.70 to
  14.51 million instructions in four runs of one build), and rounds that overlap say nothing.
- `REGRESSED (time alone)`: the time is over its tolerance and the instructions are not. The
  work is the same. What is left is how the processor runs it: the alignment of a loop, the
  cache lines of the data a benchmark walks, a branch the predictor loses, all of which move
  with any change of a build, in code the change does not touch as well
  (`styling_subsystem::SelectorMatch.none`, measured by the gate at +10.7 % for a change that
  touched no styling code, retired 0.95 % fewer instructions).

Each of the three fails the gate. A benchmark that took less time with the instructions of the
base is `faster (time alone)`: a quieter round, or memory and code that lie better, and not
less work.

Tolerances. A metric regresses when the candidate is slower than the base by more than
`--tolerance` (default 5 %) times the factor of the metric, and by more than a floor below which
the measurement resolves nothing (0.05 ms a frame, 2 ns an operation):

| Metric | Factor | Why |
|---|---:|---|
| UI thread and wall time of a frame | 1 | hundreds of frames of a deterministic CPU workload |
| Render thread time of a frame | 1.5 | it runs beside the UI thread and moves with core placement and glyph cache state |
| UI thread, render thread and wall time of a frame benchmark | as above | compared for a frame the surface received, not for a frame of the clock |
| TreeDataGrid and tree benchmark operation | 2 (3 under 100 ns) | short iterations in a new process per run; heap and code layout move a benchmark by several percent, and the shortest operations are near the timer |
| Instructions of such an operation | 2 % whatever `--tolerance` is | a count, the same to a hundredth of a percent between two processes of a build (floor one instruction) |
| Build and show of a scene (`build_ms`, `show_ms`) | 1.5 | measured once a run: one sample a round (floor 0.5 ms) |
| Start of a scene (`startup_ms`) | 1 | the sum of the two (floor 0.5 ms) |
| First frame of a scene of the start (`first_frame_ms`) | 1.5 | measured once a run |
| Memory of a start (`build_mb`, `show_mb`, `first_frame_mb`; `startup_mb`) | 2; 1 | resident memory moves with what the allocator returns to the system; the sum less (floor 1 MB) |
| Size of an object of an element (`sizes/...`) | 0 | exact: any growth is a regression |
| Window: first frame and time a frame | 2 | a real process: the display, the system compositor, the disk and the shader caches (floors 20 ms and 0.5 ms) |

A regression the gate reports after its extra rounds is a real one; a change that is meant to
cost time says so in the pull request, with the table. A benchmark that regressed in time alone
is looked at before the pull request goes up: its counts on the two sides, the sizes of the
objects it uses (`sizes/...`), and a second run of it. When the counts and the sizes are those of
the base and the code the benchmark runs is not what the change touches, the time is that of
where this build put the code and the data; the pull request shows the line with its counts and
says so. The gate is not told to pass it.

## 9. Deviations from upstream for frame cost

Each deviation keeps the observable behaviour of upstream (the same events, the same order, the
same results) and changes only how much work a frame does.

### 9.1 The disposal queue of the compositor is indexed by id

Upstream keeps the server objects to dispose with the next batch in a list and searches it for an
earlier entry of the same object before adding one. A frame in which thousands of visuals record
new draw lists disposes thousands of render data objects, which made that search quadratic: 14 %
of the UI thread in the gauntlet. `Compositor` keeps the position of each id's entry in a table
indexed by the id (ids are dense), so queueing is constant time; the batch writes the same entries
in the same order.

### 9.2 The hit-test tree reads only a bucket's children

The composition hit-test tree splits a collection's children into buckets of 32 in z order and
checks a bucket again when the readback revision changed. Checking a bucket took a snapshot of the
whole collection, so a hit test over a collection of thousands of children was quadratic (97 % of
the UI thread on the still wall with the pointer in the window). A bucket now reads its own 32
children by index.

### 9.3 A store keeps the results of its inherited lookups

A read of an inherited property the object does not hold walks its inheritance ancestors to the
nearest one that holds it, as upstream does. A layout pass reads inherited properties about
400,000 times a frame in the gauntlet (`UseLayoutRounding`, `FlowDirection` and the text
properties, nearly all of them held by no ancestor), each walk two to three ancestors long. A store
now keeps what each lookup found, or that it found nothing, tagged with the thread's inheritance
generation; the generation changes whenever the holder of an inherited value can change (an
inheriting property's effective value added to or removed from any store, an inheritance parent or
ancestor changed). A change of the value itself needs no new lookup: the store keeps the holder's
effective value, which holds the current value. Tests: `ferro_object_tests_inheritance.rs`, the
last two. Local gate (section 8.1), three rounds: gauntlet UI thread 44.90 to 43.46 ms a frame
(-3.2 %), still wall UI thread -5.9 %.

### 9.4 The font manager keeps the glyph typefaces it found

Every text line asks for the glyph typeface of its run's typeface, and a run's properties keep the
answer only for themselves; a text block creates its run properties with each layout, so in the
gauntlet every line of every text built a `FontFamily` from its name and looked it up in the
collections again, every frame. `FontManager::try_get_glyph_typeface` now keeps what it found, by
typeface, until the thread's glyph typeface generation changes: a font collection deriving from
`FontCollectionBase` gaining a glyph typeface, or a collection added, replaced or removed. A
collection that implements `IFontCollection` directly and changes what it returns calls
`FontManager::invalidate_glyph_typefaces`. Local gate, three rounds: gauntlet UI thread 45.09 to
43.15 ms a frame (-4.3 %; the render thread, which the change does not touch, -1.8 % in the same
rounds).

### 9.5 Render transform and opacity changes do not record the drawing again

A visual's render transform and opacity are composition properties: the render thread applies them
to the visual's retained drawing. Upstream nevertheless invalidates the visual when either changes,
so its `render` runs again, a new draw list is recorded and serialized, and the old one is
disposed, for every frame of an animation. FerroUI routes these changes through
`Visual::invalidate_composition_properties` and `IRenderer::add_dirty_properties`: the compositing
renderer synchronizes the visual's composition properties without calling `render`. The default
of `add_dirty_properties` is `add_dirty`, so other renderers behave as before. No `render` of the
framework reads the visual's own transform or opacity (checked over every `render` override).
Changes of `Bounds` still record the drawing: `TextPresenter::render` tracks its position for the
caret of input methods. Tests: `visual_render_tests.rs`. Local gate, three rounds: gauntlet UI
thread -0.5 %, still wall UI thread -5.2 % (its cursor moves by a transform).

### 9.6 The hit-test tree compares readback revisions before reading readbacks

With the pointer in a window, pointer-over and tooltips hit-test the pointer after every frame, and
each hit test checks the children of every collection it passes through: the readback revision
changes with every frame, so every bucket of the collection is checked again. Checking a child read
its whole readback (a lock and a copy, and clones of its root and of the compositor's readback
indices) only to find, for nearly every child, the revision the tree already has. A child is now
checked by the revision of the readback slot it would read, from the slots' atomics, and its
readback is read only when that revision differs; the tree is unchanged for every child whose
revision it already has, as before. Local gate, three rounds, with the frame gate's scenario of the
still wall with the pointer in the window (`wall/20+pointer`): UI thread 3.59 to 1.64 ms a frame
(-54.4 %); gauntlet UI thread -4.1 %.

A first attempt skipped the whole check while no child's readback had been written since the last
one (a revision of the children kept by each server visual); the gate refused it (+17.8 % on
`wall/20+pointer`): the still wall's container holds the changing clock and cursor beside its
static tiles, so the revision of its children changes with every frame and the bookkeeping only
added cost.

### 9.7 A text block keeps a left-aligned layout whose lines fit a narrower width

A text block lays its text out again whenever its constraint changes, except in the cases upstream
and section 9 already keep. In the gauntlet each text block builds two layouts a frame: one for its
measure and one for its arrange, at another width. A layout is now also kept for a new width when
it is left-aligned, its lines all fit the new width, and the new width is not larger than the one
it was created for or it was created without a width. Wrapping and trimming are greedy: each line
ends at the furthest point that fits the larger width, and that point still fits the smaller one,
so the lines are the ones a new layout has; the height rule of the kept layouts is unchanged.
Tests: `a_layout_whose_lines_fit_the_new_width_is_kept_and_has_the_lines_of_a_new_one`, and the
widths of `a_layout_that_depends_on_the_width_is_created_again` now make the lines change.
Counters (`--features counters`): text layouts created 5,844 to 5,143 a frame (-12 %); local gate:
gauntlet UI thread -1.6 % (within noise), no metric regressed.

### 9.8 A text block keeps its run properties between layouts

Each layout of a text block made its default run properties anew: about nine property reads (the
font family, style, weight, stretch and size, the decorations, the foreground and the font
features, an inherited lookup), an allocation, and a glyph typeface the new properties resolved
again for the first line. A text block now keeps them until a property they are made of changes:
the change handler already lays the text out again for each of them (`invalidate_text_layout`:
the font properties, flow direction, letter spacing, text, decorations, font features and
foreground), and now also drops the kept properties. Test:
`a_kept_layouts_run_properties_follow_the_font_size`. Local gate, three rounds: gauntlet UI thread
40.93 to 37.24 ms a frame (-9.0 %), no metric regressed. The paragraph properties (flow direction,
wrapping, line height and spacing, letter spacing, text line bounds and the alignment of the
layout) are kept the same way, by alignment, and dropped by both invalidations of the layout
(-1.5 %, within noise, eight reads and an allocation fewer a layout).

### 9.9 Layoutables keep a copy of their size, margin and visibility

A layout pass reads a few properties of every element many times: in the gauntlet, `IsVisible`
about 93,000 times a frame, `Width`, `MaxWidth`, `MaxHeight`, `MinWidth`, `Height`, `MinHeight`
and `Margin` between 17,000 and 38,000 times each, every read a lookup in the element's value
store. `Visual::is_visible` and the getters of these `Layoutable` properties now keep a copy of
the value (`property_store::HotValue`) with the generations it was read at: the value store's,
which every change of one of its effective values advances (a value set, coerced, added or
removed, before the new value is written), and the metadata generation, which every override or
removal of property metadata advances. A read with both generations unchanged returns the copy;
any other read looks the property up and keeps the new value. Only properties that do not
inherit are kept this way: an inherited value changes with an ancestor's store. Tests:
`layout/hot_value_tests.rs` (local, style, current and animation values, a cleared local value,
margin and visibility). Local gate, three rounds: gauntlet UI thread 37.28 to 36.11 ms a frame
(-3.1 %), still wall with the pointer -6.2 %, no metric regressed.

### 9.10 Targets track the dirty area as several rectangles by default

Upstream keeps one dirty rectangle per target unless region clipping is asked for
(`UseRegionDirtyRectClipping`). One rectangle is the union of everything that changed, so two
small changes far apart repaint the whole area between them: on the still wall the clock line at
the top and the cursor outline, which steps 37 tiles a frame and so moves down the window over
time, made a rectangle that grew from 1522 x 138 to 986 x 882 pixels in 900 frames, and the
render time from 28 to 81 ms a frame. Every backend that renders (Skia, Vello, Quartz) supports
regions, so `CompositionOptions::use_region_dirty_rect_clipping` now means on when it is `None`
and only `Some(false)` keeps the single rectangle. The tracker is upstream's
`MultiDirtyRectTracker` with its defaults (at most 8 rectangles, merge eagerness 1000), and a
target still renders into its retained layer, so the pixels outside the rectangles are the last
frame's. Test: `composition_options.rs` (the default). Headless still wall, 900 frames: 81.4 to
10.8 ms a frame, flat over the run (10.6 ms at 120 frames).
### 9.11 Skia draws box shadows with a blur mask filter

Upstream gives the paint of a box shadow a blur image filter. An image filter makes Skia draw the
shadow's shape into a layer of its own and blur the layer's pixels with a full Gaussian pass, for
every shadow in every frame it is drawn: in the gauntlet, whose cards each have a shadow, that
pass (`GaussianPass::blurSegment`) was 39 % of the render thread. The Skia backend now gives the
paint a blur mask filter of the same sigma (`MaskFilter::blur(Normal, sigma, respect_ctm)`): the
same Gaussian of the shape's coverage, in the shadow's color, under the same clip that keeps it
outside (or inside) the box. No layer is made, and Skia draws a blurred rounded rectangle from a
nine-patch it caches on the CPU and with an analytic shader on the GPU, so equal shadows cost a
lookup. The Vello backend already drew most shadows in closed form, and Quartz with its own GPU
path. Test: `media/box_shadow_tests.rs` `box_shadows_of_every_kind` (outer and inset, rounded and
square, offsets, spreads, blurs from 2 to 40, two shadows on one box), whose expected image is the
image filter's: the two differ by at most 3 of 255 in a channel, in 6 of 120,000 pixels. Local gate,
three rounds: gauntlet render 88.9 to 50.9 ms a frame (-42.8 %), wall time -42.3 %.

### 9.12 A text block keeps a layout over its width hold and for a taller height

Upstream makes a new text layout whenever the constraint of a text block changes; section 9.7
keeps a left-aligned layout whose lines fit a narrower width. In the gauntlet that kept nothing:
every text block is measured twice a frame, by its grid, at an infinite height and then at the
height of its auto row, and the rule kept a layout only for a height not larger than the one it
was made for, so the two measures replaced each other's layout: 4,890 layouts a frame.

- `TextLayout::is_truncated_by_height` tells whether a layout left lines out because they did
  not fit its maximum height. A layout that left nothing out has the lines of a layout made for
  any height its lines fit, so a text block keeps it for a taller height too.
- `TextLayout::width_hold` is the range of maximum widths over which a new layout makes the same
  lines (docs/porting/text-layout-performance.md, step 1): wrapping is greedy, so the lines stay
  while every line fits (from the widest line without its trailing white space, with it for a
  line that ends the paragraph or at a required break) and while no line can take the next
  line's first word (up to a line's width with its trailing white space and that word, read from
  the advances of the shaped run). It is known for left-to-right text that wraps without
  trimming, a line limit, letter spacing, tabs or paragraph styles, whose lines are one shaped
  run each and end at break opportunities, kept 0.01 inside its exact bounds for the tolerance of
  the formatter's comparisons. `TextLayout::holds_width` answers for one width and stops at the
  first line that rules it out. A left-aligned text block keeps its layout for any width the
  layout holds.

Tests: `text_layout_width_hold_tests.rs` (every width a layout holds, in quarter-pixel steps,
gives a new layout with the same lines, over texts with hyphens, dashes, slashes, runs of spaces,
required breaks and words wider than the width), the same check with HarfBuzz and Skia over the
gauntlet's card bodies (`TextPipelineBenchmark` tests), and `text_block_tests.rs`
(`a_wrapped_layout_is_kept_at_a_wider_width_it_holds`,
`a_layout_that_left_no_line_out_is_kept_for_a_taller_height`); the reference test that arranges
a text block wider and expects a new layout centers its text, which the hold does not cover.
Gauntlet, per frame: text layouts 4,890 to 1,765, text lines formatted 5,533 to 2,263.
`TextPipelineBenchmark.text_block_measure_sweep` (a width step of 7 a measure): labels 1.97 to
0.14 ms, card bodies 7.85 to about 5.5 ms, articles unchanged within the noise. Local gate, three
rounds: gauntlet UI thread 39.0 to 33.6 ms a frame (-13.9 %), no metric regressed.
### 9.13 A visual that only moves does not record its drawing again

Upstream's `Bounds` affects render: any change of a visual's bounds, its position included,
records its drawing again. A visual draws in its own coordinates, and its position reaches the
compositor as the offset of its composition visual, so a change of position alone now
synchronizes the composition properties (section 9.5's path) and keeps the drawing; a change of
size records it again. A class whose drawing reads its own position names `Bounds` in its own
`affects_render` (`TextPresenter`, which records its offset in the root while it draws). Every
item of a scrolled list moves without changing size, so scrolling no longer records the drawings
of what it moves. Test: `visual_render_tests.rs`
(`moving_a_visual_invalidates_its_composition_properties_and_resizing_it_its_drawing`). In the
gauntlet, whose content is resized every frame, it saves the recording of about 650 visuals a
frame (8,200 to 7,550). The local gate is unchanged within its tolerance (raster Skia): the gain is in
lists that scroll at a fixed width, where every moved item is no longer recorded.

### 9.14 An unchanged drawing is not sent to the render thread again

A visual recorded again often draws exactly what it drew before: a left-aligned text block
without a background, resized while its layout was kept (section 9.12), draws the same lines at
the same places. `RenderDataDrawingContext::get_render_results_for` compares the new
recording with the render data the visual has: the same operations on the same resources, by
identity, none of them a custom draw operation (which a dropped recording disposes) or a
resource not sent. When they are the same, the new recording is dropped, its references
released, and the render data stays: the render thread neither receives nor deserializes a new
stream, computes its bounds or disposes the old one, and the UI thread creates no server object
for it. Because an invalidation can come from something the drawing does not contain (an opacity
mask, an effect), the visual still has the server draw its area again
(`CompositionDrawListVisual::invalidate_draw_list_content`, a third state of the draw list in the
batch).

The identity of a glyph run in a recording was the `Arc` the recording wraps it in, new for every
recording, so no two recordings of the same text were equal (and a glyph run drawn twice in one
recording took two slots); it is now the glyph run itself.

Tests: `compositor_tests.rs` (`an_unchanged_drawing_is_not_sent_again_and_its_area_is_drawn_again`;
`visual_brush_as_opacity_mask_is_a_server_side_scene_brush`, whose mask changes while the drawing
does not, checks the area is drawn again). A counter, `VisualsRecordedUnchanged`. Gauntlet: 3,820
of 8,177 recordings a frame are unchanged. Local gate against section 9.12, three rounds: render
thread 51.8 to 45.7 ms a frame (-11.7 %), UI thread -8.1 %, wall time -11.7 %, no metric
regressed.

### 9.15 Measure holds: an element is not measured again where its measure cannot change

Upstream measures an element again whenever its available size differs from the last one. When
the width of a window or a column changes, every element below is measured again, although most
of them return the same desired size: in the gauntlet, whose content width changes every frame,
10,765 measures a frame.

An element may now report a measure hold (`ferroui_base::layout::MeasureHold`): the range of
available sizes over which its measure returns the same desired size and leaves its descendants as
they are. `LayoutableImpl::measure_hold_override` gives it in the space of the constraint
`measure_override` received; `Layoutable::measure_core` maps it to the space of the available
size through the margin, the minimum, maximum and explicit sizes, and the clamp of the desired size
to the available size (an element whose desired size was clamped holds nothing). `Layoutable::measure`
returns at once when the measure is valid and the new available size is inside the hold. An
element whose type reports no hold holds at the constraint it got: an axis fixed by an explicit size
(or equal limits) gives `measure_override` that constraint at every available size, so a fixed-size
element holds for every available size it fits, and any other element only at its own available
size, as upstream compares them. The types that report a hold:

- `TextBlock`: the widths of its layout's width hold (section 9.12), any width for left-aligned
  text that neither wraps nor trims, and every height its lines fit when none was left out, with
  its padding; nothing for other alignments or inlines;
- `Border` and `Decorator`: their child's hold moved by the padding and the border; any size
  without a child;
- `StackPanel`: the intersection of its visible children's holds across the stack (they get its
  constraint across it and an infinite size along it), any size along it;
- `WrapPanel`, horizontal with items at the start: its lines are the same from the widest line up
  to the narrowest width at which a line would take the next item, intersected with its
  children's holds where they get its constraint;
- `Grid` without definitions: the intersection of its children's holds; with one star column
  (without limits or a shared size), auto or pixel rows (or any rows at an infinite height, where
  star rows size to their content) and no spans, at a finite width: a cell of the star column gets
  the grid's width less the fixed width of the other columns and the spacing, so its hold moves by
  that amount (less a rounding step on each side when the layout is rounded); the other cells get
  constraints the grid's width does not change. Other grids hold only their own size.

An infinite upper bound includes infinity, so a child measured at an infinite size along a stack
is inside its own hold. A hold relies on everything a measure reads either being the available
size or invalidating the measure when it changes; `UseLayoutRounding`, which upstream does not
mark as affecting the measure (the next measure at another size picks it up), now invalidates it. Tests: `layout/measure_hold.rs` (the mapping through margins, limits and
the desired size), `measure_hold_tests.rs` in the controls (a tree of borders, a stack, a
decorator and wrapped, unwrapped and centered text blocks, measured at a width and then at every
quarter-pixel width 40 around it, has the desired sizes and arranged bounds of a tree laid out at
that width from nothing; the tree has a wrap panel of chips, an `Auto,*` header grid with a fixed
avatar, a `*,Auto` row and, in one variant, a grid of two star columns). Gauntlet, per frame: measures 10,765 to 4,605, arranges 7,600 to 5,630,
drawings recorded 8,177 to 6,099. Local gate against section 9.14, three rounds: UI thread 33.6 to
29.4 ms a frame (-12.6 %), no metric regressed.

### 9.16 A shaped run finds its break opportunities once

Upstream finds the UAX #14 break opportunities of a run again for every line it wraps, from the
start of the line, and again for every layout of the same text: a third of the time of a layout
whose shaping is cached went there (docs/porting/text-layout-performance.md, section 2). A shaped
buffer now keeps the break opportunities of the text it was shaped for in a table found once and
shared with the buffers split from it, as its cluster advances are (`ShapedBuffer::line_breaks`).
A buffer that reaches the end of that text and starts at one of its break opportunities reads the
entries after its start; any other finds them again from its start, as upstream does. A line
starts after the white space of the line before it, at a break opportunity, and no rule of UAX #14
looks back across one, so the entries are the ones finding them again would give.

Test: `text_break_table_tests.rs` (texts with quotes, brackets, numbers, hyphens, dashes, URLs,
CJK, Hangul, emoji sequences and flags, no-break and zero-width spaces, soft hyphens, combining
marks, runs of spaces, required breaks and right-to-left words, at widths every 1.75 from 6 to
700, wrapping and wrapping with overflow: the lines are the ones of layouts that find the breaks
again for every line). `TextPipelineBenchmark`, a layout with shaping cached: card bodies 7.15 to
4.30 ms, articles 2.65 to 1.60 ms, labels 1.44 to 1.17 ms. Local gate, three rounds: gauntlet UI
thread 27.4 to 25.0 ms a frame (-9.1 %), no metric regressed.

### 9.17 The window's Graphite context inserts its recordings in order

Graphite evicts its glyph atlases at the end of every recording unless the context requires
recordings to be inserted in the order they were snapped (`fRequireOrderedRecordings`, off by
default): every frame rasterized and uploaded every glyph it drew again. The window's Graphite
context on Metal now sets the option, as Skia's own Metal window does; FerroUI inserts its
recordings in order already. skia-safe has no setter, so the native options are set through
`skia-bindings` (`ordered_recordings_options` in `skia_metal_gpu.rs`). With the option, a
recording that fails to insert makes the context refuse the recorder's later ones, so the
context then reports itself lost (`GraphiteGrContext::is_lost`) and is made again, as after a
lost device. docs/porting/skia-retained-text.md has what Graphite keeps across frames and what it
does not. Window (gauntlet tier 16, 60 Hz, the two builds alternately): 28.1 / 29.5 / 28.6 to
29.9 / 29.8 / 29.8 frames a second.

### 9.18 Removing a control from the queue of Loaded events takes constant time

A control attached to a tree waits in a queue for its `Loaded` event; one detached before the
dispatcher raised it is removed from the queue. Upstream keeps the queue in a set; FerroUI kept it
in a vector (for the order of the events) beside a set, and removing a control searched the
vector, so detaching a tree of n controls that had not been loaded yet took time in n squared:
detaching 10,000 tiles of the still wall (70,000 elements) took more than a second. The queue now
keeps each control's slot: removing empties it, which releases the control at once and keeps the
order of the others, and the vector drops its empty slots when they outnumber the queued controls.
Tests: `control.rs` (`loaded_queue_tests`), the leak tests of the controls (a removed control is
not held by the queue). `ElementConstructionBenchmark` (the still wall's tiles, created, attached
to a styled root and laid out): attaching and detaching 10,000 tiles 1,375 to 195 ms, with layout
1,517 to 338 ms; 1,000 tiles 26.6 to 12.9 ms. The local gate is unchanged.

### 9.19 The native renderer clips rectangles and rounded rectangles analytically

The native Metal renderer (`ferroui-quartz`) made every clip a mask: a texture the size of the
clip, the clip's path flattened into edges on the processor and a compute pass writing its
coverage, for every clip in every frame. A frame's dirty region (section 9.10) was such a mask
too, so every clip inside it was one as well: the still wall (58,696 tiles, each clipped to its
rounded bounds) made tens of thousands of masks a frame.

A rectangle clip and a rectangle clip with equal elliptical corners, under a transform that keeps
the axes, are now coverage functions of the shaders (`qm_clip_rounded`, `clip_rectangle` and
`clip_rounded` in `shaders.metal`): the parameters of every draw carry the rectangle clip in effect
(the intersection of the rectangle clips, the area of each pixel inside it) and one rounded
rectangle, and the work of a draw is cropped to their pixels. A region of one rectangle (the
dirty area of a frame that changed everywhere) is a rectangle clip. A rounded clip inside a rounded
clip, a clip under a rotation or a skew, a region of several rectangles and any other geometry
clip with a mask as before; the mask is drawn through the analytic clips in effect and takes them
in. Tests: the renderer's tests, its parity tests with Skia included. Window, frames a second:
still wall (tier 20) 0.1 to 60 (the display's rate), gauntlet (tier 16) 2.8 to 4.2.

### 9.20 The native renderer: glyph shapes, boxes, turned clips and repeated runs

Glyph shapes. A glyph run of the native renderer asked Core Text for the ink bounds and the
outline of every glyph it held, each through a copy of the font at the run's size
(`CTFontCreateCopyWithAttributes`), and built the outline of the whole run when it was created.
Text that changes every frame creates its runs every frame (the line metrics of a text layout come
from its runs): on the gauntlet that was 80% of the user interface thread. The font now keeps the
bounds and the outline of each glyph at each size and simulation (`GlyphShapes`, at most 16,384 a
font), and a run builds its outline only when it is asked for one (its geometry, its intersections
with an underline, drawing it as outlines); drawing it through native glyphs never asks.

Boxes. A rounded rectangle, a rectangle under a transform that turns it, and either inside a mask
clip were paths: curves flattened to a quarter of a pixel, edges prepared and a coverage image
looked up or rasterized at every position. Every solid box without a shadow is now a box: the
shaders test the same four samples a pixel against the box and its elliptical corners
(`box_coverage` in `shaders.metal`), in the box's own space when the transform turns it (the
samples mapped through the inverse transform). A sample within the flattening tolerance of a
corner may land on the other side of it than with the path; the box is the exact one. Aliased
rounded rectangles stay paths.

Turned clips. A rectangle or rounded rectangle clip under a transform that turns it was a mask (a
texture, a compute pass, and the end of the render pass around it). When no other analytic clip
with corners is in effect it is now the analytic clip of section 9.19 kept in its own space.

Glyph strikes and repeated runs. Every glyph run made two copies of its font (at its size and its
device size) and looked its caches up by hashing them; the copies are now kept by font, size and
scale and the caches key fonts by identity. Every glyph run with native hinting was also prepared
for reuse (a copy of each of its draws); on the gauntlet not one of a million prepared runs was
reused, as its text changes or moves every frame. A run is now prepared when it repeats: one seen
once is remembered by its hash and drawn directly. Still text is prepared one frame later.

Uploads and batches. The uploads of a command buffer are slices of one pooled arena where each
flush took a buffer of its own; paths and general draws, which share a format, share a batch, and a
draw that samples no image takes the batch's image, so text and the shapes around it make one
batch: half the command buffers of a gauntlet frame.

Tests: the renderer's tests, parity tests of rounded boxes against their paths (clipped, turned and
cut by the canvas), of boxes inside analytic clips, and of analytic clips (axis-aligned and turned)
against mask clips of the same shapes. Window, frames a second, gauntlet (tier 16), interleaved
A/B: 3.7 (section 9.19) to 4.0 (rounded boxes), 5.1 (glyph shapes), 5.7 (glyph strikes and boxes in
mask clips), 6.3 (turned boxes), 6.65 (turned clips), 8.5 (repeated runs). The still wall stays at
the display's rate. The uploads and the batches change the frame rate by less than the noise; the
gauntlet's frame is now held about evenly by the render thread and by the GPU.

### 9.21 Walks of the tree stop where nothing changes; per-element work made on use

Building a large tree from its leaves up and attaching it (the still wall: 58,696 tiles of seven
elements) did work that grows with the number of elements times their depth, or that most elements
never need. Each change keeps upstream's observable behaviour: the same values, events and order
for every element; what differs is the work done where nothing could change.

- **Measure invalidation.** Every element whose visual parent changes invalidates its own measure
  and that of its whole subtree (`LayoutHelper::invalidate_self_and_children_measure`); a tree
  built from the leaves up was walked once for every level above each element. An element whose
  subtree that walk invalidated, and which has not been measured since
  (`Layoutable::subtree_measure_invalid`), ends the walk; the flag is cleared when the element or
  one below it is measured. The flagged elements are closed downwards, so clearing stops at the
  first ancestor without it.
- **Effective enabled state.** `InputElement::update_is_effectively_enabled_from` updated the whole
  subtree of every element whose parent or enabled state changed. A child's value is derived from
  its parent's and every change is passed down as it happens: an unchanged value ends the walk.
- **Inline collections.** Every text block made an inline collection, its logical children and its
  subscription when constructed; most show their `Text`. The collection is made, and its change
  raised, on first use of `inlines()`.
- **Styling passes.** Every styling pass ended with a re-evaluation of every value of the element
  (`ValueStore::end_styling`), even when no style applied. The store remembers whether a
  re-evaluation was put off, or a frame inserted, during the pass, and re-evaluates only then
  (section 9.25 describes the flag and has its tests).
- **Classes.** Every input element removed its focus, pointer-over and disabled pseudoclasses when
  constructed, which made its collection of classes. Removing a pseudoclass from an element without
  classes makes nothing (`StyledElement::set_pseudo_class`).
- **Removing many children.** `FerroList::remove_all` searched the removed items for every item of
  the list, and every child unlinked from its inheritance parent was searched for among its
  siblings: clearing a panel was quadratic. Items that are one run of the list are removed as that
  run (the one notification the search raised for it); the inheritance children are a deque
  searched from both ends.
- **Control themes.** Every styled element looks up the control theme of its type when attached;
  most types have none, and each lookup walked the whole of the theme's styles at the application
  to find nothing. The application keeps the result of each type-key lookup by theme variant and
  forgets them whenever a dictionary or style it owns changes (`notify_hosted_resources_changed`)
  or its dictionary is replaced.
  A memory of the lookup at each element, tried as well, is not kept: it made attaching slower
  (5 to 7 % from the leaves up, 4.7 % from the root down, 5.1 % for a changed resource), since
  every element then holds and clears state that the application's table already answers.
- **Inherited properties no element sets.** A property no store holds (the flow direction of most
  trees) was looked up through every ancestor holding inherited values, and the per-store results
  are forgotten whenever an inheritance changes anywhere. Each inheriting property counts the
  stores holding an effective value of it (`FerroProperty::holders`, shared with the properties
  added to other owners); while none does, a lookup returns the default at once.

Tests: the tests of the base and controls crates, and tests of the inline collection made on use
(`selectable_text_block_tests`), of type-key lookups following the application's resources
(`application_tests`) and of inherited values held through another owner and released with their
holder (`text_block_tests`). Measured with the `trees` benchmarks (2,000 tiles, against the commit
before): attach -33 %, first layout -44 % (part of it the benchmark itself, which freed the
previous wall inside the measurement), detach -25 %, clear -38 %, attach with class styles -19 %, a
style added to the root -20 %. Still wall (tier 20), launch to first frame: 2.26 s to about 1.9 s.

### 9.22 Skia: a glyph run's first draw skips the text blob cache

A text blob is a key of Skia's text blob cache: drawing one hashes it, keeps the GPU blob made of it
and posts a purge message when it is dropped. Text that changes every frame makes runs that are
drawn once, and each paid for the cache for nothing. A run's first draw is of its glyphs alone
(`Canvas::draw_glyphs_at`), which Skia draws without the cache; the blob is made, and cached by
Skia, when the run is drawn again (`GlyphRunImpl::draw`). The pixels are the same: both draw the
same glyphs at the same positions with the same font. Tests: the Skia backend's tests. Gauntlet
(tier 16), Skia window, interleaved A/B: 27.4 to 31.2 frames a second.

### 9.23 macOS: a render target made by a paint is drawn once

The Metal render target of a window is made by its first paint (`TopLevelImpl::CreateMetalRenderTarget`
needs the main thread, and the paint is the first that needs the target). Giving the view its target
(`-[FrnView setRenderTarget:]`) asked for the new layer to be displayed, and so did the target's
`resize:withScale:`; both requests came from inside the paint that was about to draw that layer, so
every window was painted twice when shown, the second time with the same content at the same size
in the same transaction. For a small scene that is a frame; for the still wall it is the whole
first drawing again, about 100 ms on the UI thread.

The view now knows when it is inside the paint of its top level (`updateLayer`). A target set
during a paint is given its size and no display request; the paint that set it draws it. At the end
of that paint the view asks for the display itself if the target has begun no frame on the main
thread since (`mainThreadFrames` of the render target protocol, counted by the Metal target in
`BeginDrawing`): a paint that made a target and drew nothing still gets the display it had before.
The request is the view's (`requestDisplay`: the target's layer and the view), no longer a side
effect of the target's resize. Every other request is unchanged: a change of the frame size or of
the backing properties, also inside a paint (the second display of a cooperative resize), and
`Invalidate` of the top level. The IOSurface target counts no frames, so its windows are asked to
display as before. Upstream's sources are changed by
`native/FerroUI.Native/patches/0005-one-display-for-a-render-target-set-in-a-paint.patch`. Test:
`src/FerroUI.Native/tests/first_paint.rs` counts the paints of a real window (one when shown with
Metal, where it counted two before; another for a changed size; none while nothing changes).

Measured with the window program of the frame benchmarks (the still wall, 60 Hz, six launches each,
alternately): the second animation frame 1.30 to 1.33 s after the start of the process before, 1.11
to 1.18 s after; the frame rate of the first second 28 to 30 against 37 to 39. The first animation
frame is unchanged (0.73 to 0.76 s), which is why the gate's window suite now also measures the
second (`second_frame_ms`).

The comparison found a fault of the build: `ferroui-native` carries its static libraries
(`+whole-archive`), and nothing in the command line of the Rust compiler names their content, so a
compiler cache (sccache) gave the crate compiled before a change of the native sources back after
it, and the program ran the other side's native code. The build script now writes a fingerprint of
what the library is compiled from (the script, the generated header, the headers and the sources)
as a Rust source of the crate (`interop::NATIVE_SOURCES`), which every such cache reads.

### 9.24 A render walk asks once whether a light is registered

The lights of the composition (`CompositionLight` and its targets, the WinUI lighting API) are
applied where a visual is drawn: `LitContent::begin` looked, for every visual of every walk, through
the lights registered with the compositor for those that light the visual (`light_map_of`). An
application that registers no light, which is most, paid for the look at every visual: 2.5 % of the
render thread of the gauntlet. The walk now asks the compositor once, when its context is made
(`ServerCompositor::has_registered_lights`, kept in `RenderContext::lights_registered`), and looks
for the lights of a visual only when one is registered and alive. Lights are registered while
batches are applied, never during a walk, so the answer holds for the whole walk. With a light
registered nothing changes. Tests: the lighting tests of the composition
(`composition_light_tests.rs`, `composition_winui_lighting_tests.rs`). Gauntlet (tier 16), Skia
window, render thread: the light lookups go from about 70 to 0 of about 3,070 samples.

### 9.25 A styling batch that changed nothing does not reevaluate; compiled resource rows share one deferred group

Two changes to what applying styles and loading a compiled dictionary cost.

**Styling batches.** Upstream's `ValueStore.EndStyling` reevaluates every effective value of the
store when the outermost batch ends, whatever the batch did. An element opens a batch when its
styles are applied, when its styles or its control theme are invalidated and when styles are
detached from it, and many of these batches change nothing: no style or theme matches the
element, or it holds no frame of the kind that is removed (an invalidation that recurses over a
tree). The reevaluation of such a batch visits every effective value and every entry of every
frame of the element to arrive at the values it started from.
`ValueStore` now keeps a flag (`reevaluation_pending`, `property_store/value_store.rs`), set
while a batch is open by the three things that can change an effective value during it: a frame
inserted (`insert_frame`), and a reevaluation of one property or of the store put off because the
batch is open (a frame removed, a binding of a frame that produced a value, a style whose
activator changed). `end_styling` of the outermost batch reevaluates when the flag is set, and
clears it. The values and the notifications of a batch that changed something are
upstream's; a batch that changed nothing raises nothing, as before, and now does no work.

Tests: `style_work_tests.rs` (a clean batch completes one pass with no reevaluation and leaves
the generation of the values as it was; a frame added in a nested batch, a frame removed, a
binding update and a style activation during a batch are applied at the end of the outermost
one). The counts are asserted with `perf-counters` on, by the counter checks of the local gate
(section 8.1): `StylingPassesCompleted`, `FullValueReevaluations`, `SingleValueReevaluations`,
`ReevaluatedEffectiveValues`, `ReevaluatedFrameEntries`. `scripts/perf/style-work.py` runs the
probe of the same file (`style_work_probe`) from two builds in alternation.

**Compiled resource rows.** Upstream's generated code gives every deferred resource of a
dictionary a deferred content of its own (`XamlIlRuntimeHelpers.DeferredTransformationFactoryV3`):
an object holding the builder, the resource nodes above the declaration, the root object and the
name scope, added with `ResourceDictionary.AddDeferred`. The rows of a compiled dictionary that
are built on first use (the brushes with a literal, static or dynamic colour and the
`{StaticResource}` aliases of a row table, [xaml-themes.md](../xaml-themes.md), section 2.2) did
the same, one allocation a row at load, for rows of which an application reads a fraction. A
table is now one group (`IDeferredResourceGroup`, `controls/i_deferred_content.rs`) that holds
the static rows and one capture a row in a vector, and each row's dictionary entry is the group
and the index of the row (`ResourceDictionary::add_indexed_deferred`, the item
`IndexedDeferred`), the size of an entry with deferred content. A lookup builds the row through
the group with the capture of that row, keeps the value in the entry as for any deferred
resource, and tells the group to drop the capture (`IDeferredResourceGroup::release`); a build
that fails keeps the entry and the capture, and the next lookup builds again.

What is kept of the per-row form, because the difference would be observable:

- The capture of each row is taken immediately before the row is added, as upstream's is: a
  handler of the dictionary's change notification can change the parents between two rows.
- A row whose capture owns a value (a root object or a parent that is a plain value, which
  `WeakValue` holds by value) is added as ordinary deferred content, so that value is released
  when that resource is built, not when the last row of the group is.
- A development build with hot reload, and a build with source overlays (the designer), keep
  deferred content for every row, with its reload hooks. A reload can also put a saved deferred
  entry back in place of a built value, so the dictionary does not call `release` in a
  development build with hot reload.

Tests: `theme_resources_tests.rs` (the rows are lazy, shared per dictionary and do not keep
their owners alive; each declaration is captured after reentrant notifications; a mixed group
releases an owned root per resource; a realized row releases its capture and the rows around it
keep their indices) and `resource_dictionary.rs` (the group is told of a realized row and not of
a failed build; the destruction of a source can reenter the dictionary). The workspace test run
builds `ferroui-markup-xaml` with hot reload and source overlays on, so CI runs the library
tests of the crate on its own as well. `tests/FerroUI.ResourceBenchmarks` with
`scripts/perf/resource-descriptors.py` measures the load and the allocations of the two forms
from two builds.

### 9.26 A collection of styles offers an element the styles whose selector can match its type

Upstream applies a style to an element by matching the selector of every child style against
it (`StyledElement.ApplyStyle` walks `IStyle.Children`, and `Style.TryAttach` evaluates the
selector). A control theme of a control with a large template has many nested styles, most of
which name a part of another type than the element at hand, so most of these matches fail at
the type test of the selector's last compound, once for each style and each element of the
template.

A collection of styles (`StyleChildren`, `Styles`) can now keep, for a style key, the list of
its entries that can match an element of that key (`styling/selector_candidates.rs`), and
`StyledElement::apply_style` visits the entries of the list in source order.

- **What is left out.** Only a style that is index-enabled (`Style::enable_selector_index`,
  hidden; the compiled control theme tables call it for each nested style they build), has no
  child styles, and whose selector requires a type that rules the key out. The type is read from
  the last compound of the selector, which must start with it (`Selector::type_candidate`):
  exact for a type selector, with the derived types for `:is()`. A selector with an OR, a
  `:not()`, a nesting or a custom node at the start of that compound gives none, and so does a
  style whose parent is a container query. Every other entry is in every list: styles that are
  not index-enabled, styles with children, nested collections, custom `IStyle` implementations
  (which the index does not call). The matcher evaluates the whole selector of every listed
  style, activators included; the index does not replace any part of it.
- **Where the lists live.** A collection and the index-enabled styles added to it share one
  state (`IndexState`: the number of such styles, a generation, the lists); the collection
  allocates it when the first such style is added, and each `Style` holds a weak pointer to the
  state of the collection that counts it. A collection without index-enabled styles has no state:
  `CandidateCache::select` returns the whole snapshot after one check, with no scan and no
  allocation. A collection with fewer than eight entries is traversed whole as well.
- **Invalidation.** The generation of a state advances, and its lists are dropped, when the
  collection changes, when a style it counts gets a selector, is enabled again or gets a parent,
  and when the children of such a style change. No state is shared between collections, so a
  change of one collection or of its styles does not invalidate the lists of another. A list is
  used only for the snapshot of the collection and the generation it was computed for; a
  traversal re-reads the element's style key and the generation before each step and, when
  either changed, goes on from its position through every remaining entry of its snapshot.
- **Memory.** The lists of a collection are kept by style key, up to 1024 keys; a list is a
  shared slice of entry indices (`u32`), and keys with the same candidates share one slice. A
  key beyond the bound is computed at each use and nothing is dropped for it, so a rotation of
  more keys than the bound does not empty the lists of the others.
- **A restriction.** A collection counts a style when the style is added to it while it is
  index-enabled. A style enabled for the first time afterwards is a candidate for every key
  until it is added again; a style in two collections is narrowed by the first one. Generated
  code enables a nested style before it adds it.

The values and the order in which styles apply are upstream's: an entry that is left out is one
the matcher rejects by type. Tests: `styling/selector_candidate_tests.rs` (the lists for exact,
derived and overridden style keys; selectors that give no type; edits of the collection, of a
selector, of the children and of the parent, during a traversal too; a collection without
index-enabled styles is never scanned and allocates nothing; 200 style keys in rotation are
scanned once each; keys beyond the bound; a change of another collection invalidates nothing;
a style the index cannot leave out and one it can apply in source order, with the same values
as without the index), `theme_styles_tests.rs` of `ferroui-markup-xaml` (a generated theme) and
the style reload test of `ferroui-hot-reload` (an indexed collection replaced by one loaded at
run time). `tests/FerroUI.ResourceBenchmarks/selectors.rs` with
`scripts/perf/selector-candidates.py` measures the application of a generated theme from two
builds, for selectors with a type, with a type and a pseudo-class, and without a type.

### 9.27 A text block keeps a layout that trimming and the line limit left whole

At the gauntlet's tier 16 every card is about 60 px wide. A text block is measured at the width
its parent has left, which is not rounded (59.763), and arranged at its layout-rounded width
(60.0), so every text block saw two constraints a pass. The rules that keep a layout over a new
constraint (sections 9.7, 9.12 and 9.15) refused every text of the gauntlet: the paragraphs wrap
with a character ellipsis and a line limit, the subtitles are four inlines on one line with an
ellipsis. Of 1,793 text layouts a pass, 1,048 were of a text that had not changed.

- `TextLayout::width_hold` no longer excludes trimming and a line limit. A line is trimmed in two
  cases only: it does not fit the width (the hold already excludes a layout with a collapsed or
  overflowed line), or lines after it were left out for the maximum height
  (`is_truncated_by_height`, already excluded) or for the line limit. The new
  `TextLayout::is_truncated_by_max_lines` tells the last case, with or without trimming, and
  such a layout has no hold. A layout that left nothing out is the layout of the same text
  without trimming and without a limit, and inside the hold a new layout has the same lines, so
  the same count: nothing is left out there either.
- A text block that does not wrap keeps a left-aligned layout without a collapsed line
  (`TextLayout::has_collapsed_lines`) at any width its lines fit, and reports the measure hold
  from that width upwards: without wrapping the width decides nothing but whether a line is
  trimmed, and a line that fits is not. Lines left out by the line limit do not depend on the
  width without wrapping.
- Inlines without a control among them (`EmbeddedControlRun`) follow the rules of a text: their
  layout depends on the constraint the way a text's does. The text block remembers whether its
  text runs hold a control when it builds them; while the runs are not built, and with a control,
  a layout is not kept, as before.

A measure that holds over more constraints showed a defect of the grid that upstream shares: the
definitions of a grid are made when they are first read, a read invalidates nothing, and the
arrange took a grid with definitions for one measured with them. A cell of a keyboard (a grid
without definitions around an unwrapped text) whose definitions a test helper had read was
arranged at a new size without a measure and failed on definition structures that were never
validated. The grid now arranges, and reports its hold, by the measure that ran
(`Grid::is_measured_with_definitions`; DEVIATIONS.md, "Grid").

The same change set makes three representations cheaper without changing what they do. The sets
of addresses of the compositor (the objects to serialize, the visuals of the renderer, the
resources of a recording) hash with `rustc_hash` instead of SipHash; an address needs no defence
against chosen keys. `RenderDataResources::intern` searches the table of a recording while it has
at most eight resources and builds its map only beyond that: a recording of a control has a brush,
a pen and a glyph run, and paid for a map. `Visual::synchronize_composition_properties` reads the
styled values it follows (opacity, clip to bounds, and whether a clip, an opacity mask, a cache
mode, an effect and a render transform are set) from a snapshot kept while the value store of the
visual is unchanged (`HotValue`, section 9.9); an object that is set is read as before, since it
can change without the store changing.

Tests: `text_layout_width_hold_tests.rs`
(`a_trimmed_limited_layout_makes_the_same_lines_at_every_width_it_holds`: over the texts of
section 9.12 and line limits of 1 to 40, a layout with a hold left nothing out and has no
collapsed line, has the lines of the plain text, and a new layout at every width it holds, in
half-pixel steps, has the same lines, none collapsed, none left out;
`layouts_the_hold_does_not_describe_have_none`), and `text_block_tests.rs`
(`a_trimmed_limited_layout_that_left_nothing_out_is_kept_at_a_width_it_holds`,
`a_layout_that_left_text_out_for_the_line_limit_is_created_again`,
`an_unwrapped_layout_that_was_not_trimmed_is_kept_at_any_width_it_fits`,
`inlines_without_a_control_are_kept_the_way_a_text_is`: the kept layout has the lines and the
desired size of a new text block at the new constraint).

Gauntlet (tier 16), Skia window, per render pass: text layouts created 1,793 to 779 (the rest are
texts that changed), layouts dropped for a new measure constraint 541 to 18 and for a new arrange
constraint 506 to 15, measures executed 4,671 to 3,506. UI thread, from samples: about 23 to about
17 ms a frame. The frame rate does not change (31.5 to 32.3 frames a second before and after):
the render thread is busy for the whole frame and sets it.

### 9.28 The UI thread of a frame: values read in place, events nobody hears, a drawing kept by its render data

After section 9.27 the render thread and the UI thread of the gauntlet cost about the same, and
with the render thread no longer the only limit the UI thread's share of a frame decides the frame
rate as well. Its samples show no single cost; they show the same small work done for every one of
about 13,500 visuals a frame. This section removes that work where it has no effect. None of the
changes alters a value, an event, a drawing or an order.

Values read where they are kept:

- The generation of inherited values and the generation of property metadata are one for the
  process (an atomic) and no longer one a thread. Every read of an inherited value and every
  `HotValue` compares them, and the address of a thread-local value costs more than the load of a
  static. A change on one thread makes the stores of every thread look their values up once more;
  no store misses a change.
- `HotValue` (section 9.9) is public and hidden from the documentation, with `get_for(object,
  read)`, so that the controls crate keeps values the way the base crate does. A layoutable keeps
  its two alignments with its sizes and margin. A border keeps its thickness, corner radius and
  background sizing, a decorator its padding, and a row or column definition its user size,
  minimum and maximum: a grid reads the three of every definition in every pass of its layout.
  Each kept value is read again whenever the value store of its object changes.
- Layout asks for the layout scale of every element it rounds. `IPresentationSource` has
  `layout_root_scaling()` (default: the scaling of its layout root), and
  `Visual::layout_scaling_of_presentation_source` reads it in place, where
  `LayoutHelper::get_layout_scale` handed out the layout root as a counted reference for every
  element.
- An assignment of an animated property returns at once when the object has no animation
  controller, and the transitions of composition facades are asked only of a visual that has
  facades.

Work that has no receiver:

- A routed event counts its instance handlers (`RoutedEvent::has_instance_handlers`): adding a
  handler, removing it and dropping an interactive that holds handlers keep the count. Building a
  route for an event that has neither a class handler nor an instance handler returns the empty
  route without walking the ancestors. A stack panel raises its snap-points events, which bubble,
  in every arrange as upstream does; in the gauntlet that walked the ancestors of every stack
  panel a frame for nobody. Handlers added later are found: the count is read when a route is
  built.
- `Visual::synchronize_composition_child_visuals` answers a visual with no visual child, no child
  composition visual and no facades without building the lists it compares.
- The compositor asks for a commit only when none is pending (`ensure_commit_requested`), where
  every change of every object handed the pending batch out and dropped it.

A drawing kept by its render data:

- A recorder keeps the stream of a recording it discards (nothing drawn, or the drawing the
  visual already has, section 9.14) and records the next visual into the same storage
  (`RenderDataStream::recycle`). Storage beyond 512 bytes is not kept, so that a recording does
  not hold much more than it needs.
- A visual whose drawing changed gives the new recording to the render data it already has
  (`CompositionRenderData::replace_recording`, through
  `RenderDataDrawingContext::get_render_results_for`, which returns a `RecordedDrawing`: the same
  drawing, updated in place, or a new render data). Upstream, and FerroUI until here, creates a
  render data for every changed drawing and disposes the old one: a server object created and
  one disposed, an allocation and a registration for each of the visuals a frame draws. The
  server-side render data already resets itself and reads a new stream whenever it is
  serialized, which is how it received its first recording; the visual tells its server-side
  visual that its content changed (the change code of an unchanged drawing list), so bounds and
  the kept fragment follow as for a new render data. The resources of the old recording are
  released as `dispose` releases them: by the render data when the recording was not sent yet,
  by the server-side render data when it reads the next one. A render data that something else
  than its visual and the renderer holds is not written to: the recording gets its own, as
  before. A visual that stops drawing has no render data, as before.

The Skia backend sets the transform of its canvas when something is drawn with it:

- `DrawingContextImpl` keeps the transform it is given and applies it to the canvas before the
  first operation that depends on it (a draw, a clip, a layer, a save, a lease of the canvas).
  The render walk sets a transform for every visual and sets it back after it, and a visual
  outside the dirty area or without a drawing recorded two matrix operations for nothing. What is
  recorded is what is drawn with, in the same order; Skia's API is used as before, only less
  often. When the canvas can be leased (`ISkiaSharpApiLeaseFeature`) the canvas follows at once.

Tests: `interactive_tests.rs`
(`the_count_of_instance_handlers_follows_added_removed_and_dropped_handlers`,
`a_class_handler_is_reached_when_no_instance_handler_is_attached`), `render_data_tests.rs`
(`a_changed_recording_goes_to_the_render_data_the_visual_has`: the server-side render data keeps
its id, is not disposed, has the bounds and replays the operations of the new recording, also
after two recordings before one batch; `an_equal_recording_leaves_the_render_data_as_it_is`;
`a_shared_render_data_and_an_empty_drawing_are_not_recorded_in_place`;
`the_custom_operations_of_a_replaced_recording_are_released`), `hot_value_tests.rs`
(`the_kept_alignments_follow_their_changes`), `kept_value_tests.rs` of the controls
(`the_kept_values_of_a_border_follow_local_style_and_cleared_values`,
`the_kept_sizes_of_a_definition_follow_its_properties`) and
`drawing_context_transform_tests.rs` of the Skia backend
(`drawing_uses_the_last_transform_set`, `a_transform_that_nothing_is_drawn_with_is_not_recorded`,
`the_canvas_follows_the_transform_at_once_when_the_api_can_be_leased`). Counters: `routed event
routes skipped (nothing listens)`, `visuals recorded into their render data`, and the transforms
applied to the canvas beside the transforms set.

Gauntlet (tier 16, GPU Skia, 60 Hz, this machine, sampled, per frame), before and with the
section:

| Per frame | Before | With |
|---|---|---|
| UI thread | 18.4 ms | 16.7 ms |
| Layout pass | 9.28 ms | 8.59 ms |
| `Compositor::commit` | 7.55 ms | 6.53 ms |
| `ValueStore::get_value` | 1.36 ms | 0.74 ms |
| Routed events | 0.37 ms | 0.14 ms |
| Render thread | 35.3 ms | 33.4 ms |
| Frames per second | 31.8 to 34.2 | 33.6 to 34.3 |

The frame rate hardly moves because the render thread sets it while it draws directly; the UI
thread's share is what a pipelined frame gets back.

### 9.29 A plain text is laid out once for the texts that are the same

After section 9.28 text is a third of the UI thread of a gauntlet frame (about 5.6 of 16.7 ms).
A frame makes 796 layouts, 744 of them because the text of a block changed, and each is made
from nothing: the characters are split into runs, shaped, broken into lines and given their
metrics, and the layout the block had is released. Most of that is work that was done before.
The texts are short (95 % have six bytes or fewer) and they repeat: only 4 % of the layouts a
frame makes are of a text that was never laid out with the same font, size, brush, wrapping and
constraint. The repeats are spread over time, so a layout has to be kept for a while to be
found again:

| Layouts kept while nothing shows them | Layouts found instead of made |
|---|---|
| 1,024 | 31 % |
| 2,048 | 38 % |
| 4,096 | 42 % |
| 8,192 | 47 % |
| 16,384 | 53 % |

Upstream makes a layout for every text block and every change. egui keeps its laid-out texts
(galleys) by their content for the frames that use them, and Cranpose keeps its measured texts
the same way. FerroUI keeps the layout itself, the object the formatter makes, so nothing about
shaping, line breaking or drawing is done another way: a layout that is found is the layout
that would have been made.

What is kept and where:

- The font manager of a thread keeps the layouts (`TextLayoutCache`, a field of `FontManager`,
  `media/text_formatting/text_layout_cache.rs`). `TextLayout::shared_plain` asks it for the
  layout of a plain text and makes the layout with `TextLayout::from_text_source` when there is
  none. The default `TextBlock::create_text_layout` calls it for a text without inlines.
- A layout is found by everything it is made of: the text, the run properties (typeface,
  requested family, size, font scale, baseline alignment, culture), the paragraph properties
  (flow direction, alignment, wrapping, line height, line spacing, indents, letter spacing,
  line bounds and metrics mode), the trimming, and the exact maximum width, maximum height and
  line limit. The foreground and background brush are compared as objects, never by their
  colour: a layout draws with the brush object it was made with, and a brush of the same colour
  that is another object can change later. Shared brushes (a theme's, `Brushes::black()`) are
  what makes layouts of different blocks the same.
- A layout that can change after it was made is not kept and belongs to its block as before: a
  text with text decorations or font features (collections that are observed), a paragraph
  with tab stops, a text longer than 256 bytes, and every text with inlines. A text block that
  makes its own layout (`SelectableTextBlock`, `AccessText`, an override of
  `create_text_layout`) does not ask the cache.
- A kept layout is shared (`TextLayout::is_shared`): `TextLayout::dispose` leaves it as it is,
  because another block may show it, and the cache releases its lines when it lets the layout
  go. A layout something holds is never let go. Of the layouts nothing holds the cache keeps
  the 4,096 used last, as far as their text is within 64 KiB (what a layout keeps grows with
  its text, so long texts go sooner); it looks for the ones to release once in 1,024 new
  layouts or 16 KiB of new text, so keeping them costs a constant amount for a layout made. A
  shared layout does not keep the run cache of the block that made it.
- A cache that is full keeps a text from the second time it is asked for. Full means that the
  last look for layouts to release found 4,096 that nothing holds, or 64 KiB of their text:
  from then on a layout that is added takes the place of another. The layout of a text that is
  asked for once (a clock, a counter, a value that never comes back) is then made for its block
  alone, as a layout that cannot be shared is, and the block releases it. Without that every
  such text pushed out a layout that could have been found again, and was itself released by
  the cache long after its memory was last used: the benchmark with texts that are new every
  time cost 14.0 % more than without a cache, and costs 3.8 % less with this rule (alternating
  runs against the commit before section 9.27; the texts that repeat went from 10.8 % to
  13.6 % less). The cache remembers the
  texts it was asked for and did not keep by sixteen bits of the hash of the request, four to
  a slot of 16,384 (128 KiB, made when the cache is first full); a text that looks like one of
  them is kept at its first request, which is what happens to every text while there is room.
  A program with fewer texts than the cache keeps never sees any of this.
- Every change of the fonts of a thread (a font collection added or removed, glyph typefaces
  invalidated: the generation the glyph typeface lookups already follow) lets all kept layouts
  go, so no layout shaped with a font that is gone is found. A layout that is still shown stays
  whole for the block that shows it and is made again when that block asks for a layout.

A new text keeps what is not made of it: `TextBlock` no longer drops its run properties and
paragraph properties when its `Text` changes (they are made of the font, the brushes and the
paragraph values), only its layout and runs.

What a program can see: two text blocks that show the same text the same way return the same
object from `TextBlock::text_layout`, and `TextLayout::dispose` does nothing to a shared layout.
Measured sizes, lines, hit tests and drawing are the ones of a layout made for the block.

Tests: `text_layout_cache_tests.rs` (`the_same_text_with_the_same_properties_is_one_layout`,
`what_a_layout_is_made_of_tells_layouts_apart`: each part of the key on its own,
`a_layout_is_shared_for_the_same_brush_object_only`,
`a_layout_made_of_what_can_change_is_not_shared`,
`disposing_a_shared_layout_leaves_it_to_its_other_holders`,
`the_layouts_nothing_holds_are_let_go_from_the_oldest`,
`a_full_cache_keeps_a_text_from_its_second_request`,
`long_texts_nothing_holds_are_let_go_by_their_length`,
`a_change_of_the_fonts_lets_every_layout_go`) and `text_block_tests.rs`
(`text_blocks_that_show_the_same_text_the_same_way_share_a_layout`,
`a_text_that_comes_back_has_the_layout_it_had`,
`a_shared_layout_outlives_a_block_that_lets_it_go`,
`a_layout_made_of_more_than_a_plain_text_is_not_shared`). The gate measures it with
`text::TextBlockChangingTextBenchmark` (`change_and_measure`: a text block whose text changes
and is measured, with texts that repeat and with texts that are new every time; the second is
the cost the cache adds to a layout it cannot find).

Counters (`--features counters`): `text layout cache hits`, `text layout cache misses`,
`text layout cache evictions` and `text layout cache: first requests of a text, not kept`,
beside `text layouts created`.

Gauntlet (tier 16), Skia window, per render pass: text layouts created 796 to 450 (346 found,
446 made and kept, 418 let go: the cache is full and turns over, as the table of this section
says it would at 4,096), text runs shaped 768 to 427, text lines formatted 856 to 507. With
the rule for a full cache the same frames find as many layouts (346), keep 242 of the ones
they make and let 244 go; 186 are first requests, whose layout stays with its block.

| Gauntlet, tier 16, direct drawing | Frames a second | UI thread, ms a frame | Render thread, ms a frame |
|---|---|---|---|
| Section 9.28 | 33.9 | 16.7 | 33.4 |
| With the kept layouts | 36.6 | 14.5 | 27.3 |

The thread times are from samples (about a tenth above the real ones). The render thread gains
more than the UI thread: text blocks that show the same text hold one layout, so they draw the
same glyph runs, and the renderer finds what it keeps for a glyph run more often (a reading of
the numbers, not measured apart).

What the layouts that are not found are (every lookup of 20 seconds of the gauntlet written out,
the second half counted: 139,577 lookups, 54,874 different keys):

| Text | Lookups | Different keys | Found with 4,096 kept | Found with every layout kept |
|---|---|---|---|---|
| A price (`611.26`) | 67,901 | 45,538 | 5.0% | 32.9% |
| A change (`+0.94%`) | 49,008 | 2,712 | 94.5% | 94.5% |
| A share (`96%`) | 13,552 | 100 | 99.3% | 99.3% |
| The rest (titles, wrapped paragraphs) | 8,599 | 6,007 | 29.4% | 30.1% |
| All | 139,577 | 54,874 | 41.3% | 60.7% |

Four in five of the layouts that are not found are prices, numbers that were not shown before:
no number of kept layouts finds them (16,384 kept layouts find 51% of all lookups, every layout
kept 61%), so the number stays at 4,096. The wrapped paragraphs are not found because their
width changes with every frame of an animation; they are shaped once all the same, by the text
run cache of their text block (upstream), and only wrapped again. What is left of the cost of a
changing text is the cost of making a layout of a short text, of which shaping is a seventh and
allocation a quarter.

### 9.30 The pushed states of a drawing context and the readback pass

Two costs of a frame that are not work on the scene but the way two small pieces of state are
kept. Both were found by sampling the gauntlet with the Graphite stage moved off the render
thread, where the render thread itself is what limits the rate.

**The pushed states of a drawing context.** A `DrawingContext` remembers what was pushed (a
transform, a clip, an opacity) to pop it in reverse order. Upstream rents that stack from a
pool, and FerroUI took it from a pool of the thread: a lookup of the thread's storage, a
borrow, and a block on the heap of a few bytes for every context that pushes, which is every
visual that draws. The states are now stored in the context itself
(`StateStack = SmallVec<[PushedStateType; 16]>`, `media/drawing_context.rs`); a context with
more than sixteen nested states moves them to the heap as a vector does. Nothing about the
order of pushes and pops changes, and `states` is `None` before the first push and after the
disposal as before.

**The readback pass.** The UI thread reads four values of a server visual back (its matrix to
the root, its target, whether it is visible, and its subtree bounds). Upstream collects the
visuals whose values changed in a queue and visits each one more time after the targets were
rendered (`ServerCompositionVisual.UpdateReadback`). In the gauntlet that is several thousand
visuals a frame, and each visit touches three places that are not in the cache any more: the
visual, the readback it shares with the UI thread, and the lock of that. FerroUI keeps the
pass and its place in the frame, and changes what it has to touch:

- The update walk takes the copy of the four values at the moment it has finalized the bounds
  of a visual (`readback.rs`, `snapshot_for_readback_update`), when the visual is being worked on anyway.
  The queue of the pass holds the copy beside the readback it is written to
  (`ReadbackQueueEntry`, `server/server_compositor.rs`), so the pass writes without visiting
  the visual.
- A change that is announced after the copy was taken (an adorner, a value set by a later
  pass) turns the entry of the visual back into a visit, so the pass writes what upstream's
  pass would have read.
- A visual is in the queue once for a pass by the number of the pass, where upstream sets a
  flag in the visual and the pass clears it in every visual again.
- The two slots of a readback are atomics. The protocol of the slots is upstream's (one
  writer, one reader, the reader never reads the slot that is written), and the lock of the
  readback indices, held for the whole write and taken by the reader to move to a newer
  revision, still orders the write of a revision before every read of it. What went away is a
  lock for each slot that this protocol never contended.

The copy relies on two things the code states where it takes it: the own transform of a visual
is written only through `set_own_transform` and before the update walk, and the values the
copy holds are final once the walk has finalized the bounds of the visual.

What it measures (M3 Pro, gauntlet at tier 16, cycle-aligned means of alternating runs):

| | Before | After |
|---|---|---|
| Readback pass, render thread, Graphite stage on its own threads | about 1.5 ms a frame (7.4 % of the thread) | about 0.75 ms a frame (3.9 %) |
| Frame rate, Graphite stage on the render thread | 34.9 fps | 35.4 fps (inside the noise of the runs) |

With the Graphite stage on the render thread the frame is bound by Skia's own work and neither
change moves the rate. They are kept for the configuration in which the render thread's own
passes are the limit, and because neither adds a code path: the stack is the same stack in
another place and the pass is the same pass with fewer places to touch.

What a program can see: nothing. The values the UI thread reads back and the revision they
belong to are the same, and so is the order in which states pop.

Tests: `drawing_context_tests.rs`
(`more_states_than_the_context_stores_in_itself_pop_in_reverse_order`) and
`server_composition_visual/readback_tests.rs` (`nothing_is_read_before_the_first_write`,
`every_value_survives_a_slot`, `a_reader_keeps_its_revision_while_newer_ones_are_written`,
`two_writes_of_one_revision_leave_the_last`, `the_pass_writes_what_the_update_walk_left`,
`a_change_announced_after_the_copy_is_read_by_the_pass`,
`a_visual_is_in_the_queue_once_per_pass`, `the_visuals_of_a_pass_share_its_revision`).

### 9.31 A render data that is read again, the observers of a resource, the paint of a solid fill

Three costs on the render thread that are storage handed back and fetched again for every
visual whose drawing changed, every frame. They were found by sampling the gauntlet with the
Graphite stage on its own threads: the largest single entry of the render thread's self time
was the allocator's `free`, and the copy and the reset of `SkPaint` followed it.

**A render data that is read again.** Since section 9.28 a visual whose drawing changed keeps
its render data, and the server-side object reads the new recording when the batch arrives
(`ServerCompositionRenderData::deserialize_changes_core`). Upstream's shape of that read is a
reset followed by a new stream, whose resources are walked afterwards to observe them: the
buffer of the operations, the table of the resources and the list of the resources the data
observes are freed and allocated again. Now the object lets go of what it drew and keeps where it was stored
(`release_content`: the subscriptions are removed, the custom draw operations disposed, the
stream emptied by `RenderDataStream::recycle`, which already bounds what a recycled stream may
keep), reads the new recording into that storage, and subscribes to each server-side resource
at the moment the stream resolves it. `reset`, which a disposal uses, releases the storage as
before.

**The observers of a resource.** A render resource tells its observers when it was
invalidated. Upstream walks its dictionary in place; here the list is behind a `RefCell` and
an observer may change it while it is told, so it is copied first. The copy was a vector on the heap for every invalidated resource in every frame; a resource has
one or two observers, so the copy is now a `SmallVec` of four on the stack
(`ServerRenderResourceCore::queued_invalidate`). Who is told is the same: every observer
that is in the list when the invalidation starts, once.

**The paint of a solid fill.** The Skia drawing context keeps one paint for fills and resets
it after each (`PaintWrapper::dispose`), which is a destruction and a construction of a
`SkPaint`. A solid colour fill sets two things on the paint, the anti-aliasing and the colour,
and every fill sets both before anything else. So the paint of a solid fill goes back
unreset (`PaintWrapper::release_fill`) and the next fill starts from it. The wrapper knows
that it is still such a paint because its paint is private: `paint_mut`, the only way to
configure it further, clears the mark, and a fill with a shader, a stroke, a blend mode or
anything else is reset as before. The one other user of the kept paint, the acrylic material
rectangle, resets what it takes. No Skia behaviour is replaced and no Skia call is made that
was not made before; one call is left out where its result is already there.

What it measures (M3 Pro, gauntlet at tier 16, Graphite stage on its own threads, the same
binary without and with the three changes, 51 s each, cycle-aligned means):

| | Before | After |
|---|---|---|
| Frame rate | 52.6 to 52.9 fps | 54.5 fps |
| Render thread, per frame | | 0.6 to 0.8 ms less |

With the Graphite stage on the render thread the frame is bound by Skia's own work and the
rate does not move.

What a program can see: nothing. A render data draws its last recording and follows the
resources of that recording only, the observers of a resource are told as before, and every
fill has the pixels it had.

Tests: `render_resource_tests.rs`
(`a_replaced_recording_observes_the_resources_of_the_new_drawing`),
`render_data_tests.rs` (`the_custom_operations_of_a_replaced_recording_are_released`, which
now runs through the kept storage), `server/server_render_resource.rs`
(`every_observer_is_told_once_however_many_there_are`,
`an_observer_added_twice_is_told_once_and_stays_until_it_is_removed_twice`,
`an_observer_may_remove_itself_and_others_while_it_is_told`,
`an_observer_added_while_the_others_are_told_is_told_the_next_time`,
`a_dropped_observer_is_pruned_and_a_disposed_resource_tells_nobody`) and the Skia render
tests (`a_fill_draws_the_same_after_any_other_fill`, six kinds of fill in all thirty-six
orders compared pixel by pixel with the fill drawn first in a context, and
`consecutive_solid_fills_each_have_their_colour_opacity_and_edge_mode`).

### 9.32 A visual that leaves its tree for a moment or for a pool keeps its composition visual

A virtualizing panel that recycles a row takes the row out of the visual tree
(`VirtualizingStackPanel::recycle_element`, as upstream: the container is cleared, pooled, hidden
and removed) and puts it back when the next item needs a container, in the same layout pass
or in a later one. Leaving the tree ran `Visual::detach_from_compositor` for the row and for every element
under it, which lets go of the composition visual, and entering it again created a new one. A
gauntlet frame recycles rows all the time, so every frame made and released composition
visuals and their server-side objects by the hundred: allocations on the UI thread, a
serialized object each, and on the render thread the same again plus the removal of the old
ones. The new objects are wherever the allocator had room, so the walks of the UI thread and
of the render thread lost their locality as the run went on: the recording of a frame took
5.4 ms at the start of a run and 7.8 ms forty seconds later.

**What is kept for a pass.** A visual that leaves its tree asks its renderer to keep its composition
visual until the renderer's next update pass (`IRenderer::keep_departed_composition_visual`).
The renderer holds the composition visual beside a weak reference to the visual, so it keeps
no visual alive, and it queues an update pass. A visual that is in a tree again before that
pass, with the same compositor, rejoins with the composition visual it had
(`Visual::attach_to_compositor_core`): the object, its server-side object, its render data
and its place under its parent are the ones from before, and the synchronisation that follows
any attachment writes whatever changed meanwhile. A visual that is not back by then is
released in that pass exactly as it was released at once before
(`CompositingRenderer::release_departed`, which also runs when the renderer stops or is
disposed), and so is the composition visual of a visual that was dropped. The update pass
always comes before the commit that would send the batch, so the render thread never sees a
composition visual that waits.

**What is kept for a pool.** One pass is not enough for a list whose rows are of more than
one kind. A recycled row waits in the pool of its recycle key until an item of its kind needs
a container, and that is often a later frame: with only the rule above, a gauntlet frame
still created 35 composition visuals and kept 2.4. So the element that pools says so.
`Visual::leave_tree_keeping_composition_visuals(leave)` runs `leave`, which takes the visual
out of its tree, and hands back what the visual and everything under it would have given to
the renderer for one pass, as a `KeptCompositionVisuals`
(`IRenderer::keep_composition_visuals_departing_in`; a renderer without composition runs
`leave` and keeps nothing). Whoever holds that value decides how long the composition
visuals wait, and releases them by dropping it: what is back in a tree by then is left
alone, and everything else is released exactly as the update pass releases what it kept.
`VirtualizingStackPanel` holds one beside each element of its recycle pool
(`remove_pooled_child`), lets go of it right after the element is a child again
(`get_recycled_element`), and lets go of all of them when the panel itself leaves the visual
tree. What waits is bounded by the pool, which already keeps the far larger elements
themselves: a composition visual and its server-side object for each visual of a pooled
element, with the render data it last had. The same conditions as for one pass decide what
may be kept, the calls nest (a removal inside another collects for itself), and a removal
that unwinds leaves what it collected to the next update pass.

**When it is not kept.** Only a composition visual that nothing but the synchronisation of
the visual tree wrote to is kept, because only then is the kept one the same as a new one
would be after its first synchronisation. It is released at once, as before, when

- it was handed out: `Visual::composition_visual` and the public `attach_to_compositor`
  mark it (`ElementCompositionPreview`, animations, lights, effects and an application ask
  that way); the synchronisation itself reads it with `own_composition_visual`, which marks
  nothing;
- the visual has a child composition visual or composition extras (facades, transitions,
  lights, a theme shadow, a layer, unloading visuals, show or hide requests);
- the composition visual draws acrylic, or has an extension that does not say that it follows
  its control out of the tree
  (`ICompositionDrawListVisualExtension::follows_control_out_of_tree`, `false` unless
  overridden). The border's does: `Border` writes its corner radius to it whether or not the
  border is in a tree;
- the renderer is stopped, disposed or in its update pass (no pass will follow in which to
  release it), or the renderer has none of this (the default of the two new `IRenderer`
  methods is to refuse).

A visual that returns under another renderer rejoins only when the composition visual is not
in the tree of another target (`IRenderer::rejoins_departed_composition_visual`); under
another compositor it never does. A visual that returns under another parent in the same pass
keeps its composition visual and leaves the children of the old composition parent at once,
since a composition visual has one parent.

**While it waits.** A visual out of the tree has no composition visual: `composition_visual`
answers `None` as before, the renderer's dirty lists ignore the visual, and hit testing does
not find it. The last point needs care, because the composition visual is still a child of
its parent until the update pass: a waiting draw list visual reports no draw list to the hit
test and its children are not visited (`DrawListData::departed`,
`CompositionTarget::hit_test_children`, `hit_test_first_core`). What waits for a pool goes
through update passes: in the first of them its topmost composition visual leaves the
children of its composition parent, as that of any removed visual does, and from then on the
kept visuals are a subtree of their own that is in no tree, which the render thread neither
walks nor draws. Where the visual was shown is redrawn in that pass, as for any visual that
leaves.

**An empty record for every visual that left a tree.** `detach_from_compositor` first releases
the theme shadow visual, and that release went through the accessor that creates the
composition extras of a visual when it has none. So every visual that left a tree once had
extras from then on, empty ones, and missed every short path that asks whether a visual has
extras: the child synchronisation of section 9.28 and the transitions check among them. The
release, the end of a show or hide request, the removal of an unloading visual and the change
of the transitions now leave a visual without extras as it is
(`Visual::change_composition_extras`).

**The bounds of a server-side visual that changes its parent.** A server-side visual
remembers the bounds of its subtree in the space of its parent, and hands them to the parent
as an extra dirty rectangle when the parent changes
(`ServerCompositionVisual.OnParentChanging`), so that the place it leaves is redrawn. It
went on remembering them, and the next update under the new parent added the same bounds to
the dirty region again, now read in the space of the new parent: a rectangle that belongs to
no place the visual is or was. Upstream does the same; there it is a redraw of a few pixels
too many after a rare reparenting, here every row that comes back from a pool is reparented.
The bounds are now forgotten when they are handed to the old parent
(`on_parent_changing_core`), and the next update computes them for the place the visual is
in.

What it measures (M3 Pro, gauntlet at tier 16, Graphite stage on its own threads, the build of
section 9.31 against the same with this section, cycle-aligned means):

| Measure | Section 9.31 | With this section |
|---|---|---|
| Frames per second, two runs each | 53.7, 54.0 | 58.5, 59.4 |
| UI thread pass, ms | 17.3 to 17.5 | 15.9 to 16.6 |
| Render tick, ms | 17.0 to 18.0 | 14.4 to 15.1 |
| Picture recording, ms, at the start and at the end of a run | 5.4 and 7.9, 5.6 and 8.1 | 4.9 to 5.5 throughout |
| Composition visuals created per pass, after the first 300 passes | 35.3 | 0.0 |
| Composition visuals kept per pass | 0 | 37.8 to 47.0 |

The recording time that grew over a run no longer grows: the server visuals
that the rows left behind and made again are the same ones now, and the dirty rectangle of a
row that comes back is the rectangle of the place it is in. The display runs at 60 frames per
second, so 60 is the most this measure can show.

What a program can see: a composition visual that the program asked for is never kept, so
whatever a program holds or changed is released when its visual leaves the tree, as before.
The identity of a composition visual that nobody asked for now survives a removal and an
addition within one update pass, which a program can only observe by walking the composition
tree down from a composition visual it did ask for; what it changes on a visual reached that
way stays changed on the kept one, where a new one would have started over. The diagnostic
tools ask for the composition visuals they show, which only means that what they inspect is
not kept. A row that comes out of a recycle pool has the composition visuals it went in
with, under the same rule; a pooled element whose composition visual a program asked for is
not kept, so nothing changes for it.

Tests: `compositor_tests.rs`
(`visual_back_before_the_update_pass_keeps_its_composition_visuals`,
`visual_back_in_the_tree_is_synchronized_with_what_changed_meanwhile`,
`visual_not_back_by_the_update_pass_releases_its_composition_visuals`,
`composition_visuals_kept_for_a_dropped_visual_are_released`,
`composition_visual_handed_out_is_released_when_its_visual_leaves`,
`composition_visual_is_not_kept_with_a_child_visual_or_while_rendering_is_stopped`,
`visual_out_of_the_tree_is_not_hit_through_what_is_kept_for_it`,
`visual_moved_to_another_parent_in_one_pass_keeps_its_composition_visuals`,
`child_removed_while_its_parent_is_out_of_the_tree_releases_its_composition_visual`,
`composition_visual_kept_for_one_compositor_is_not_used_with_another`,
`visual_that_leaves_its_tree_gains_no_composition_extras`; for a pool
`visual_taken_out_for_a_pool_keeps_its_composition_visuals_across_update_passes`,
`composition_visuals_kept_for_a_pool_are_released_by_what_holds_them`,
`composition_visuals_kept_for_a_pool_outlive_no_visual`,
`child_that_left_a_pooled_visual_releases_its_composition_visual_with_the_pool`,
`visuals_taken_out_inside_one_another_are_kept_by_the_innermost`,
`nothing_is_kept_for_a_pool_outside_of_a_tree_or_while_rendering_is_stopped`,
`composition_visuals_kept_for_a_pool_are_not_used_by_a_disposed_renderer`; for the bounds
`visual_moved_to_another_parent_and_place_redraws_where_it_was_and_where_it_is`) and the
counter checks `composition_work_of_a_row_taken_out_of_the_tree_and_put_back`
(`compositor_tests.rs`) and `composition_work_of_rows_that_wait_in_the_recycle_pool`
(`virtualizing_stack_panel_tests.rs`), which the performance gate runs with the counters
`composition visuals created` and `composition visuals kept` (`scripts/perf/gate.py`,
`COUNTER_CHECKS`).

### 9.33 The UI thread starts on its next frame when its batch is taken, and a late frame layer draws while the frame before it is presented

After sections 9.26 to 9.32 a gauntlet frame at tier 16 cost the UI thread about 15 ms and
the render thread about 38 ms, and the two ran one after the other more than they had to: the
UI thread held its next commit back until the render thread had deserialized the batch
before, and the render thread spent most of its tick inside Graphite, which sorts the draws
of a frame, prepares their text and builds its passes on the thread that draws and snaps.
Neither is work that can be made much smaller without repeating what Skia does. What can
change is when it runs and on which thread. This section has three parts: when a commit may
follow the one before it, what happens to a batch that just misses its tick, and a frame
layer of the Skia backend that draws on threads of its own.

**A batch is accepted when the render thread takes it.** `CompositionBatch` has a third
completion, `accepted`, before `processed` and `rendered`: the render thread raises it when it
takes the batch from the queue (`ServerCompositor::apply_pending_batches`), before it reads
the first change. From then on the batch belongs to the frame that thread is starting, and a
batch committed now is applied by the frame after it. `MediaContext` holds the next commit
back until the batch before it is accepted, where upstream waits until it is processed
(`MediaContext::commit_compositor`). The UI thread therefore prepares its next batch while the
render thread applies and draws this one. One batch at most waits in the queue either way,
so nothing piles up, and nothing of an accepted batch can be observed: whoever needs its
effects still waits for `processed` or `rendered`. A batch that is completed without being
applied raises `accepted` too. The compositor holds the request for the next commit back by
the same signal (section 9.46; until that section it waited for `processed`, and every
picture took two render passes).

**A batch that follows an idle tick is drawn in the display frame it was made for.** With a
render thread, a batch that is committed a moment after the tick of its display frame waits
for the next tick, a whole display frame, although the render thread has nothing to do. The
queue remembers a tick that applied no batch and drew no frame (`BatchQueue::note_idle_tick`),
and the first batch enqueued after such a tick asks the render loop for a tick out of turn
(`Compositor::enqueue_batch`, `IRenderLoop::request_frame_out_of_turn`, which
`ThreadProxyRenderTimer::request_tick_out_of_turn` serves on the timer thread). One idle tick
answers one batch, so no display frame gets a second frame, and a compositor that renders on
the thread that commits asks for nothing.

**A frame layer that holds a frame back.** `IDrawingContextLayerImpl` has two new methods,
both with defaults that change nothing: `has_pending_frame` (the frame drawn into the layer
last is not what `blit` shows yet) and `complete_pending_frame`. A layer that says so is
presented once more by its owner when no further frame follows:

- `ServerCompositionTarget::render_frame` presents in a tick that has nothing to draw when
  its layer holds a frame back, after `complete_pending_frame`;
- `ServerCompositor` asks for another tick while a layer of an active target holds a frame
  back (`has_pending_frames`), never while rendering is stopped;
- the batches a tick applied are reported as `rendered` at the end of that tick, unless a
  layer holds the frame of the tick back: then they are reported at the end of the next
  tick, which presents their frame (`ServerCompositor::notify_batches_rendered`). They wait
  one tick and never longer, and a compositor that is dropped answers them;
- a synchronous render (`Compositor::render` on the thread of the compositor, and the wait
  of `MediaContext::immediate_render_requested` for `rendered`) returns with the committed
  frame presented, as before.

**The pipelined frame layer of Graphite on Metal.** The frame layer of a window (the layer a
render target context creates, no other surface) is a `PipelinedLayerSurface`
(`gpu/graphite/pipelined_layer.rs`) unless `SkiaOptions::frame_pipelining` is `Off`. It draws
its frames in one of two ways:

- *directly*, as an ordinary layer: the drawing context draws on the surface of the layer
  with the recorder of the context. Every layer begins like this;
- *pipelined*: the drawing context records the frame as a picture
  (`ISkiaSurface::begin_recorded_frame`). When it is disposed, the picture goes to the band
  threads of the context: the layer is divided into horizontal bands of equal height, two
  by default, and each band has a thread. Each thread owns a Graphite recorder and a surface
  that wraps the texture of the layer, plays the picture under the clip of its band, snaps
  its recorder and keeps the recording. The render thread does not
  wait: it draws the layer onto the target as the frame before left it, flushes, and then
  tells the threads to insert their recordings. The frame is on the screen one frame later
  than it was drawn, and the render thread is free for the next one in the meantime.

Nothing of Skia is repeated: the picture, the clip and the playback are Skia's, and the
bands differ from one playback only in the clip Skia applies. What changed is the thread that
asks.

`SkiaOptions::frame_pipelining` is a `SkiaFramePipelining`: `Off`, `Adaptive` (the default)
or `On` (every frame, for tests and measurements). An adaptive layer pipelines after four
frames in a row that were late (a frame is late when it cost more than half of the time to
the frame after it, and 4 ms at least), draws directly again after ninety light frames in a
row, starts over after a pause of 100 ms, and stays direct for 120 frames after a frame that
only the recorder of the context draws. An application whose frames are light never
pipelines one, has no threads for it, and shows every frame in the tick that drew it, as
upstream does.

`SkiaOptions::frame_pipelining_bands` is the number of bands, and of threads: two by default
(`DEFAULT_FRAME_PIPELINING_BANDS`), and a value outside 1 to 16
(`MAX_FRAME_PIPELINING_BANDS`) is brought into that range
(`SkiaOptions::effective_frame_pipelining_bands`). With one band the whole frame is drawn by
one thread beside the render thread. A band is 64 pixels high at least, so a layer has the
bands it is asked for or as many as fit it (`band_count`): a layer of 200 rows that is asked
for eight has three. A layer that is asked to be divided and has room for one band only, and
a layer lower than 64 pixels, is an ordinary layer. The threads belong to the GPU context
and are made once, by its first layer that pipelines a frame, as many as that layer has
bands; a layer of the same context that asks for more draws in as many bands as there are
threads. What the number of bands buys and costs is measured below.

**Frames only the recorder of the context draws.** A pipelined layer draws such a frame
directly (the recorded picture is played on the recorder of the context), and because the
same draws are counted in direct frames (`ISkiaGrContext::note_direct_only_draw`), a layer
whose frames keep having them never begins to pipeline:

- a draw with an image filter: a filter reads around the pixels it writes, across the clip
  of a band;
- a lease of the Skia API (`ISkiaApiLeaseFeature`): what the client draws belongs to the
  recorder the lease hands out. In a recorded frame the lease records what its client draws
  into a picture of its own (`RecordedLeases`), which the context draws into the frame in
  its place, and the frame is then played on the recorder of the context. What a client of
  a lease can see in such a frame: the lease has no surface, the canvas does not carry the
  clip of the context (the drawing is clipped all the same), and the state it leaves on the
  canvas does not last to its next `with_sk_canvas`;
- a frame of a retained GPU producer, whose storage is held until the next flush of the
  context only.

**The order on the GPU** is the order of insertion: what the recorder of the context has
pending when the frame ends, the draw of the layer onto the target with the flush of the
tick, then the recordings of the bands, from the top band down (`InsertOrder`). The next frame
of the layer, the next read of the layer and the next flush of the context wait for the
bands before they add anything, so a second window that shares the context waits for the
frame in the threads (at most what is left of it) before its own flush. The fixed order of
the bands costs nothing, because the insertions are serialized by the lock of the context
anyway, and it is what makes a frame the same every time it is drawn: Graphite resolves the
pixel where an antialiased edge crosses the boundary between two clips in favour of the band
that is inserted second, so with the bands racing for the lock that one pixel differed from
run to run (by 6 of 255 in the test scene).

**What the pixels are.** A pipelined frame is, byte for byte, the frame the ordinary layer
draws when it draws the same content band by band, under the same clips, with the recorder
of the context and each band a recording of its own
(`a_pipelined_frame_shows_what_the_layer_draws_band_by_band`,
`a_frame_in_any_number_of_bands_shows_what_the_layer_draws_band_by_band` for one, three and
four bands, `the_frames_of_a_pipelined_layer_follow_each_other_in_order`). Against a frame drawn in one
piece it differs as any frame Skia draws under a clip differs: a gradient fill rounds
differently by 1 of 255 at isolated pixels, and the pixel where an antialiased edge crosses
the clip boundary differs by about the coverage of one sample. In the test scene (rounded
rectangles, gradients, antialiased lines and text across the boundary) that is 7 of 81,920
pixels and 6 of 255 at most (`a_pipelined_frame_differs_from_an_unclipped_frame_as_any_clipped_frame_does`,
which bounds it). Direct frames have drawn under such clips all along: every partial redraw
draws under the clip of its dirty rectangle. Text is drawn by the same Skia code from the
same strikes; each band has a glyph atlas of its own, so a glyph that several bands draw is
rasterized and uploaded once for each, with the same result.

**A band is a recording of its own, and Graphite decides about multisampling for each
recording.** Graphite draws the draws of a recording in passes it builds for that recording.
When one draw of a pass needs multisampling (in the test scene a stroked ellipse that is
not a circle), the whole pass is multisampled, and the shapes Graphite otherwise draws with
coverage it computes (lines, rectangles) get the edges multisampling gives them. A frame
that is drawn directly is one recording, so one such draw anywhere in the frame decides
for all of it. Of a pipelined frame, the bands that contain such a draw are multisampled
and the others are not: every band is drawn exactly as a frame that contains the draws of
that band alone. Both renderings are Skia's own for the same draws, and which one a shape
gets already depends on what else is in its frame when frames are drawn directly; with
bands it depends on what else is in its band. In the test scene, in four bands, the two
bands with the ellipse are byte for byte the frame drawn as one recording, and in the two
without it the antialiased edge pixels of lines and rectangles differ, by up to 52 of 255
(`a_band_is_drawn_as_a_frame_with_its_draws_alone_is`, which bounds it at half a percent of
the pixels). This holds for the default two bands as for any other number, and it is the
one place where a pipelined frame can be told from a direct one by more than a clip does.

**When the threads are not used.** A band that fails to record or to insert ends the
threads of the context (a recording that was snapped and never inserted would make every
later recording of its recorder fail), the frame is replayed on the recorder of the context,
and the layers of that context draw directly from then on. A frame that ends while the
context is in use or disposed is played directly. A layer that is disposed with a frame in
the threads waits for it and leaves the context usable
(`a_layer_disposed_with_a_frame_pending_leaves_the_context_usable`).

What a program can see: with the default, a window whose frames cost the render thread more
than half a refresh interval four times in a row shows its frames one refresh later than
before and at the rate of the display instead of half of it; a `rendered` completion of such a
frame is raised one tick later, when the frame is presented. A program that needs every frame
in the tick that drew it sets `SkiaOptions { frame_pipelining: SkiaFramePipelining::Off, .. }`.
A custom drawing operation that leases the Skia API keeps its frame on the recorder of the
context, with the differences listed above when the frame was recorded. The counters `skia:
layers that began to pipeline their frames`, `skia: frames pipelined`, `skia: recorded frames
played on the recorder of the context` and `skia: reads of a pipelined layer while a frame is
drawn` tell what a run did.

What it measures (M3 Pro, gauntlet at tier 16, 60 Hz, quiet machine, runs of 51 s, two runs
each; the frames per second are the mean of a run):

| Configuration | Frames per second |
|---|---|
| Section 9.32, every frame drawn directly | 39.60 / 39.52 |
| This section, the defaults (`Adaptive`, two bands) | 59.99 / 59.95 |

The frames per second of this table, of the table of bands below and of section 9.34 are
callbacks of the animation frame, which is what `PERF fps` counts, and not pictures: with
the defaults of this section the compositor committed a batch for every second callback
(section 9.46), so the surface received about thirty frames a second where these tables say
sixty. The times of the stages are not affected. What the number
of bands changes is where the time goes (the stage timed by a build made for the
measurement; the processor time is that of the whole process for a frame):

| Bands | Frames per second | Slowest band | All bands together | Inserting the recordings | Processor time for a frame |
|---|---|---|---|---|---|
| 1 | 60.08 / 59.85 | 13.4 ms | 13.4 ms | 2.2 ms | 27.5 ms |
| 2 (default) | 59.93 / 59.98 | 8.2 ms | 15.6 ms | 2.4 ms | 29.0 ms |
| 3 | | 9.0 ms | 25.3 ms | 2.4 ms | |
| 4 | 60.05 / 59.94 | 8.0 ms | 27.1 ms | 2.5 ms | 32.3 ms |
| 6 | | 9.3 ms | 43.1 ms | 2.7 ms | |
| 8 | 60.03 / 60.05 | 9.5 ms | 57.4 ms | 2.8 ms | 43.8 ms |

Two bands are the default because nothing is gained after them in this frame: the slowest
band, which is what a frame waits for, is as slow with four or eight bands as with two, and
every further band costs about 4.5 ms of a core whatever its share of the frame is (with
eight bands the threads spill onto the efficiency cores). One band already frees the render
thread, and is the choice for a machine with few cores; its recording takes 13.4 ms, which
fits a 60 Hz frame and does not fit a 120 Hz frame, where two bands do.

**The picture of a frame is recorded without a bounding hierarchy.** Skia can keep the
bounds of the draws of a picture in a tree, so that a playback under a clip skips the draws
outside it without looking at them (`begin_recording(bounds, true)`). Measured the same way:
building the tree costs the render thread 3.2 ms a frame, when the picture is finished, and
saves the slowest band 0.7 ms with two bands, 1.2 ms with four and 1.6 ms with eight (all
bands together 1.4 ms, 5.4 ms and 14 ms). The render thread is the thread the stage exists
to free, so the picture is recorded without the tree; the cost of a band is mostly what
Graphite does for a recording of its own, not the walk over the draws outside it.

Tests: `render_thread_tests.rs` (`a_batch_is_accepted_when_the_render_thread_takes_it`,
`a_batch_that_follows_an_idle_tick_asks_for_a_frame_out_of_turn`,
`a_compositor_that_renders_on_its_own_thread_asks_for_no_frame_out_of_turn`),
`thread_proxy_render_timer.rs` (`ticks_out_of_turn_on_its_own_thread_while_started`),
`compositor_tests.rs` (`a_frame_a_layer_holds_back_is_presented_by_the_tick_after`,
`the_batches_of_a_stream_of_held_frames_are_rendered_a_tick_later`,
`a_synchronous_render_presents_the_frame_a_layer_holds_back`,
`no_batch_waits_for_a_held_frame_once_rendering_is_stopped`), `pipelined_layer.rs`
(`a_layer_pipelines_after_four_late_frames_in_a_row`,
`a_late_frame_costs_half_its_period_and_four_milliseconds_at_least`,
`a_pipelined_layer_draws_directly_again_after_ninety_light_frames`,
`a_pause_begins_a_new_stream_that_is_drawn_directly`,
`a_frame_that_needs_the_recorder_of_the_context_holds_the_layer_off`,
`the_options_can_ask_for_pipelined_frames_always_or_never`,
`the_threads_insert_in_the_order_the_bands_were_sent_in`,
`a_turn_that_is_dropped_passes_to_the_next`) and, on a Metal device,
`gpu/metal/tests/pipelined_frames.rs` (the pixel tests named above,
`a_layer_has_the_bands_it_is_asked_for_as_far_as_they_fit_it`,
`a_layer_has_no_more_bands_than_its_context_has_threads`,
`the_options_ask_for_two_bands_by_default_and_for_sixteen_at_most`,
`the_bands_of_a_layer_are_disjoint_and_cover_it`,
`a_pending_frame_is_completed_on_request`,
`a_frame_with_an_image_filter_is_drawn_by_the_recorder_of_the_context`,
`a_frame_that_leases_the_skia_api_is_drawn_by_the_recorder_of_the_context`,
`an_adaptive_layer_draws_its_first_frames_directly`,
`a_layer_is_ordinary_when_pipelining_is_off_or_the_layer_is_too_low`,
`a_layer_disposed_with_a_frame_pending_leaves_the_context_usable`).

### 9.34 Layout and text shaping on worker threads: measured, not adopted

Section 9.33 moved render work to threads beside the render thread. The same question was
asked of the UI thread, which after it is the limit of a gauntlet frame (about 14.4 ms at
tier 16, half of it in the layout pass): can the layout pass or the text it lays out be
computed by worker threads while the UI thread goes on? Two prototypes were built and
measured. Neither is in the tree as a mechanism; what they measured is recorded here so the
question is not asked twice, and the benchmark of the second stays as its record.

**Plain texts shaped ahead of the layout pass.** A text block whose text changes announced
the text, its typeface, size, features and culture to the shaper, which handed batches of
sixteen runs to worker threads; the layout pass then took the shaped glyphs of a run that
was ready and shaped the others itself. The glyphs were the same byte for byte (the same
HarfBuzz font data, shared, the same features). In a gauntlet frame 468 texts were
announced, 401 shaped by workers and 295 of those used; none was shaped twice. Three runs
each way, interleaved, on a quiet machine:

| Text preparation | fps | UI thread per frame |
|---|---|---|
| on | 59.49 / 59.93 / 59.88 | 15.0 / 14.4 / 14.3 ms |
| off | 59.91 / 59.99 / 61.45 | 14.2 / 14.3 / 13.8 ms |

Shaping on the UI thread fell from 3.1 % to 2.4 % of its frame (about 0.12 ms), and the
announcement cost about 0.23 ms: the copy of the text, the language of its culture and the
wake of a worker cost a text about as much as HarfBuzz costs for the short texts a busy
frame is made of (about 0.7 µs each). A hand-off pays only for work that is much larger than
the hand-off, and a single short run is not. The larger cost of a plain text layout is
outside the shaper: about 4 µs of formatting for each of the 430 layouts of a frame, which
is work of one thread whatever shapes the glyphs.

**The measure and arrange of a panel tree, computed apart from the elements.** The second
prototype is the benchmark `layout::ComputeApplySpike`: a tree of 10,101 elements (a stack
panel of borders, each with a stack panel of borders) whose layout it computes three ways.
`existing` is the layout pass of the framework. `extract` copies the inputs of every element
(sizes, limits, margin, alignment, visibility, padding, border, orientation, spacing) into
plain data, `compute` runs the measure and arrange algorithms of the same panels over that
data on one, two or four threads, and `apply` writes the results back. The setup asserts
that the computed sizes and bounds are the framework's exactly. In nanoseconds per element:

| Step | ns per element |
|---|---|
| `existing`, every element invalid | 487.1 |
| `existing_incremental`, one width changed | 54.7 |
| `extract` | 158.4 |
| `compute`, one thread | 16.2 |
| `compute`, two threads | 13.0 |
| `compute`, four threads | 10.1 |
| `apply` | 20.4 |
| starting the scoped threads, two / four | 2.7 / 4.9 |

The arithmetic of layout is 3 % of a layout pass, and four threads take 6 ns per element
off it. What a pass costs is reading the properties of each element and keeping its state:
`extract` alone, which reads each property once, is a third of the pass. A design that
computes layout on other threads has to extract first and apply afterwards on the UI
thread, so it can save at most the 16 ns it parallelizes, and `apply` here is a lower
bound (it has no setter for the previous constraints and the measure hold of an element).
The work is therefore in what a pass reads, not in where it computes: a profile of
`existing` shows the values the pass still reads through the value store for every element
(the inherited `UseLayoutRounding`, the container query provider, the layout scale of the
root) at about a quarter of the pass, beside the sizes, margin, alignment and visibility
that are kept copies since section 9.15.

**The benchmarks of the two subsystems.** The classes of `layout_subsystem::` and
`styling_subsystem::` in `tests/FerroUI.Benchmarks` measure one mechanism each (the measure
and arrange of each panel, depth, invalidation, the passes of the layout manager, size
changes, visibility, scrolled content, attached layout properties and layout listeners;
selector matching, the styles in scope, activation, setters, control themes, resources,
attaching and detaching, and changes of the style collections). They are in the framework
group of the local gate (`scripts/perf/gate.py --only framework`), so a change to what a
layout or styling pass reads is measured against the commit before it.

### 9.35 What a read and a write of a property cost a control

The layout pass of section 9.34 spends about a fifth of its time reading properties, so the
property system was measured on its own, on controls and not on objects that have nothing
but properties. The classes `property_subsystem::ControlRead` and
`property_subsystem::ControlWrite` of `tests/FerroUI.Benchmarks` measure one read and one
write, and are in the framework group of the local gate. M3 Pro, quiet machine; a walk over
16 controls stays in the first cache of the processor, a walk over 10,000 does not:

| One read of | 16 controls | 10,000 controls |
|---|---|---|
| `Width`, a kept copy (section 9.9) | 0.7 ns | 4.0 ns |
| `Opacity`, unset, the store empty | 4.1 ns | 4.1 ns |
| `Opacity`, unset, six other values in the store | 4.0 ns | 7.5 ns |
| `Opacity`, a local value | 4.1 ns | 5.4 ns |
| `Background`, a shared brush | 3.5 ns | 10.4 ns |
| `UseLayoutRounding`, inherited, held by no control | 5.4 ns | 9.4 ns |
| the font size, inherited, held by the panel above | 7.2 ns | 16.3 ns |
| `Canvas.Left`, attached, unset | 4.0 ns | 7.9 ns |

| One write of | Border | Button |
|---|---|---|
| `Opacity`, changed | 34.9 ns | 45.0 ns |
| `Opacity`, unchanged | 9.8 ns | |
| `Width`, changed | 35.7 ns | 47.6 ns |
| `Background`, changed | 85.5 ns | |
| `Canvas.Left`, changed | 38.5 ns | |
| the font size, changed, on a control without children | 33.4 ns | |
| `Opacity` with the priority of a style, set and taken back | 206.5 ns | |
| `Opacity`, set and cleared | 109.5 ns | |

What the numbers say:

- **A read is a few nanoseconds; a pass pays for how many it makes and for the memory they
  reach.** A read that finds its value walks from the object to the buffer of its values and
  from there to the allocation of the value: three places. A kept copy is in the object. In
  a walk over many controls the difference is the memory, not the instructions: the copy
  costs what reaching the object costs.
- **A control is large.** A `Border` is 2,224 bytes (a `FerroObject` 344, a `Visual` 1,248,
  a `Layoutable` 1,648, a `Control` 1,888), thirty-five cache lines, of which a pass touches
  the few that hold what it reads. A kept copy of every property of every class would add
  about a kilobyte to each control and move what a pass reads further apart; the copies are
  for the values the passes read, kept together.
- **A value that is set costs about 216 bytes** (an allocation of 192 bytes for a number, with
  its priorities, its base value and its entries, and 24 bytes in the buffer of the store); a
  property at its default costs nothing.
- **A write is mostly its notification.** Of the 45 ns of a changed `Opacity` on a button,
  the store takes about a quarter. The rest is the chain of class handlers
  (`on_property_changed` of each class of the hierarchy, eight for a button), each of which
  compares the changed property with the properties of its class, and each comparison reads
  the property from the storage of the thread (17 % of the write is that lookup alone).
- **A changed brush allocates twice.** The visual subscribes to the invalidation of the new
  value, and builds the handler of the subscription before it knows that an immutable brush
  has nothing to subscribe to.
- **A value set with a priority other than the local one costs about six local writes:** an
  entry and, for the first of an object, a frame are allocated, and the effective value is
  made and destroyed with it.


### 9.36 The kept copies of a layoutable share one stamp, and two more values are kept

Two of the findings of section 9.35 were acted on first, with the pattern of section 9.9 and
nothing new beside it: a value a pass reads is kept in the object as a copy, and the copy is
the value for as long as the generation of the store and of the metadata it was read at.

**The handler of a brush is made for a brush that can change.** A change of a property that
affects rendering subscribed the visual to the invalidation of the new value, and built the
handler of that subscription, a reference-counted closure over a weak handle of the visual,
before it asked whether the value can be subscribed to. An immutable brush, the common
value, cannot. The visual asks first.

| One write of | Before | After |
|---|---|---|
| `Background`, changed, on a `Border` | 85.5 ns | 64.8 ns |

**One stamp for the copies of a layoutable.** Each of the nine kept values of a
`Layoutable` (the six sizes, the margin and the two alignments) had a stamp of its own,
eight bytes each, and a measure that read all nine compared nine stamps that are equal by
construction: they are all the generation of the same store. They share one stamp now
(`property_store::HotStamp`) with a mask that says which of the copies were read at it. A
changed generation empties the mask. The stamp, the mask and the six sizes are the first
sixty-four bytes of the group, one cache line for what a measure reads first.

**`UseLayoutRounding` and `Container.Sizing` are kept.** A measure and an arrange read both
for every element, and neither was a kept copy: the first because it inherits, the second
because it is attached. An inherited value depends on the stores of the ancestors, which
the stamp of the element does not cover, so the copy is used in one case only: while no
store of the thread holds a value of the property (`FerroProperty::holders`, the count the
inherited lookup already keeps). Then the value is the default of the class, which only its
metadata changes, and the metadata is in the stamp. As soon as any element sets the
property every read goes through the store again, as before. `Container.Sizing` does not
inherit and is kept as the sizes are.

Median of three interleaved rounds of the build before and the build after, M3 Pro, quiet
machine:

| One read of | Before | After |
|---|---|---|
| `UseLayoutRounding`, held by no control, 16 controls | 5.42 ns | 0.98 ns |
| `UseLayoutRounding`, held by no control, 10,000 controls | 9.10 ns | 4.79 ns |
| `Width`, a kept copy, 16 controls | 0.66 ns | 0.71 ns |
| `Width`, a kept copy, 10,000 controls | 3.56 ns | 4.26 ns |

| Measure and arrange of a panel with 1,000 children | Before | After | |
|---|---|---|---|
| `StackPanel`, vertical | 108.50 us | 101.17 us | -6.8 % |
| `StackPanel`, horizontal | 109.17 us | 99.62 us | -8.7 % |
| `Grid` | 126.92 us | 118.71 us | -6.5 % |
| `Grid` with spans | 140.80 us | 127.07 us | -9.7 % |
| `DockPanel` | 124.03 us | 113.30 us | -8.7 % |
| `WrapPanel` | 109.77 us | 103.80 us | -5.4 % |
| `Canvas` | 107.92 us | 102.11 us | -5.4 % |
| `UniformGrid` | 98.72 us | 92.77 us | -6.0 % |

The passes over a nested tree (`NestedDepth`) and the layout listeners did not move beyond
the noise of the measurement.

What it costs, and what it does not:

- **A kept read alone is slower by the test of one bit**: 0.05 ns in the cache, 0.7 ns in a
  walk over 10,000 controls that reads nothing else of each. A pass reads several values of
  an element, and there the one comparison in place of nine is the larger effect, which is
  what the panels measure.
- **Memory goes down.** The group was 168 bytes with nine stamps; it is 104 bytes with one
  stamp and two more values: 64 bytes less for every layoutable.
- **Nothing else is kept.** A copy of every property would add about a kilobyte to a
  control (section 9.35); the two values added here are the two a pass reads for every
  element that had no copy.


### 9.37 A control theme keeps, for each style key, the nested styles that can match it

A control theme is applied to the control it themes and to every part of the template of
that control: the theme of a `CheckBox` is asked about the check box, and then about the
border, the grid, the path and the content presenter its template makes. The index of
section 9.26 leaves a nested style out of such an application when its selector begins
with a type that rules the element out and the style has no styles nested in it. The
styles of a theme are mostly not of that kind. They begin with the nesting selector
(`^:checked`), which names no type of its own, and they hold the styles for the parts of
the template (`^ /template/ Border#NormalRectangle`), each in a collection that was looked
up on its own. A part that no style selects still paid for those, one selector match each,
which ends at the type the selector requires.

**A plan for a style key.** The theme keeps, for each style key it is applied to, the list
of its nested styles whose selector does not rule that key out, in the order the traversal
of all of them visits them in (`styling::theme_candidates`). An application reads the plan
of the key of its element and visits those entries and no other. An entry says one of three
things: the style is attached if it matches and what is nested in it follows in the plan;
the style cannot match and only what is nested in it follows; or the entry is applied by
the ordinary traversal with everything below it.

**What is left out, and why that is what the traversal would do.** A style is left out
only when the type its selector requires is known and rules the key out. That type is the
type of the first node of the target compound of the selector, or, for a selector that
begins with the nesting selector, the type the selector of the parent style requires, up to
the target type of the theme. Either is what the matcher evaluates first, and a mismatch
ends the match there, before it subscribes to anything: a style left out is a style the
traversal would have rejected with no other effect. Only a `Style` whose selector was
index-enabled (section 9.26) has such a type. A selector that says nothing of a type, a
container query, another implementation of `IStyle` and a style whose parent is not the
style that holds it are never left out; the last three are applied by the ordinary
traversal, in their place.

**When a plan is not used.** The plans of a theme carry the version of the theme they were
made at. A changed collection of nested styles, a replaced selector, a changed parent and
a changed target type advance the version, and the plans of an older version are dropped.
Attaching a style can run code that makes such a change, or that changes the style key of
the element, so the version and the key are compared after every entry that was attached.
After a change the plan is left, and the traversal goes on from that entry as the ordinary
one does: with the nested styles the entry has now, then with every style that follows it
in its collection and in the collections of the styles that hold it. A theme nested deeper
than sixteen levels, and a key beyond the sixty-four a theme remembers, are applied by the
ordinary traversal.

**The key of a dynamic resource is converted once.** A `DynamicResource` in a template
makes one expression for every control the template is built for, and each converted the
key of the extension, a text, into a key of its own. The extension keeps the converted key,
and the expressions share it.

Median of three interleaved rounds of the build before and the build after, M3 Pro, quiet
machine (`styling_subsystem::ControlThemeApply`, 200 controls of the Fluent theme):

| | Before | After | |
|---|---|---|---|
| `Button`, the theme attached | 1.21 ms | 1.13 ms | -6.8 % |
| `Button`, the template applied | 2.45 ms | 2.39 ms | -2.5 % |
| `CheckBox`, the theme attached | 1.56 ms | 1.27 ms | -18.5 % |
| `CheckBox`, the template applied | 13.10 ms | 11.41 ms | -12.9 % |
| `TextBox`, the theme attached | 2.41 ms | 2.34 ms | -2.8 % |
| `TextBox`, the template applied | 13.28 ms | 12.73 ms | -4.1 % |

Re-attaching the controls (`reattach`), a change of the theme variant and the other classes
of `styling_subsystem` did not move beyond the noise of the measurement (five interleaved
rounds of `reattach`: `Button` +2.3 %, `CheckBox` -8.5 %, `TextBox` -3.5 %).

What it costs, and what it does not:

- **Memory: sixteen bytes for a theme, and the plans of the themes in use.** A plan is
  three numbers for an entry, and keys with the same entries share one plan: two to three
  kilobytes for a theme of the size of the `CheckBox` theme once its control and the parts
  of its template were styled. A theme no control uses has no plan. Nothing is added to an
  element; a `Style` is the size it was (the type its selector requires was already kept,
  and the same field now says "the parent's" as well).
- **A change of the nested styles of a theme costs a walk to the theme**, at most the depth
  of the nesting, and the plans are made again by the next application. Themes are built
  once and not changed afterwards, so the plans of a theme are made once for each key.
- **A re-attached control does not gain.** Re-attaching does not apply styles again; its
  time goes to the dynamic resources and the template bindings that publish their values
  again (a `TextBox`: 38 us), which is the subject of the template work that follows this.
- **The styles of an application or a control (`Styles`) are not touched.** They are
  filtered by the index of section 9.26, one collection at a time. The plan is that idea
  for a theme as a whole: it knows the target type of the theme, which is the type a
  nesting selector stands for, and it follows the nesting, so a style with nested styles
  can be left out with them.

### 9.38 A template build binds its resources and template bindings without the extension objects

The content of a control template is built by generated code for every control the template
is applied to. For a `CheckBox` of the Fluent theme that is five
`{DynamicResource}` values and nine `{TemplateBinding}` values, and each was bound by the
statements the loader runs for a markup extension: create the extension, box the key and the
property, ask the extension to provide a value in the context of the build, convert the
provided value to its untyped form, cast that back to a binding, and bind it. The extension
was reachable from nothing afterwards: the expression of a dynamic resource keeps the key,
the anchor, the theme variant and the priority, and the expression of a template binding
copies what it reads from the binding.

**The two binds make the expression from what the extension would have read.**
`compiled::bind_dynamic_resource` reads the place of the declaration from the service
provider of the build with the lookups `DynamicResourceExtension::provide_value` makes, in
the same order (one function, `Place::of`, is now the body of both): whether a control
template is being built, the anchor when the target is not an element, and the theme
variant of the theme dictionary the declaration is in. It binds a binding that lives on the
stack for the call and creates the expression from those. `compiled::bind_template_binding`
binds a `TemplateBinding` that lives on the stack for the call. Neither can fail where the
statements could not: the value an extension provides is a binding by its type, so the cast
the binding setter made always succeeded.

A development build with hot reload keeps the statements for a dynamic resource: it records
the extension, to move its anchor when the document is loaded again.

**The parents of the build are handed out, not copied.** The theme variant of a dynamic
resource is found by walking the parents of the declaration from the nearest one. The
context of the build copied its parent stack twice for each walk (the stack, then the
entries of it that are not null). It now keeps the second list beside the first, changed
by the same push and pop, and hands it out shared, as the contract of the eager parent
stack provider describes; a push copies it only while a walk still holds it, which no walk
does.

**`ValueTypes::as_object` asks one table when the first answers.** A value is viewed as an
object by the table its type is in, one of three. All three were asked for every value; the
tables after the one that has the type are no longer asked.

Median of three interleaved rounds of the build before and the build after, M3 Pro, quiet
machine (`styling_subsystem::ControlThemeApply`, 200 controls of the Fluent theme):

| | Before | After | |
|---|---|---|---|
| `Button`, the template applied | 2.46 ms | 2.29 ms | -6.9 % |
| `CheckBox`, the template applied | 11.41 ms | 10.83 ms | -5.1 % |
| `TextBox`, the template applied | 13.34 ms | 12.41 ms | -7.0 % |

Attaching the theme (five rounds: -0.2 %, -1.0 %, -1.8 %) and re-attaching a control
(+2.5 %, -1.0 %, +0.5 %) are within the noise of the runs: neither builds a template.

Two smaller costs of the same build were found in the profile of the result and removed
after it.

**The base URI of a document is parsed once.** Each build of a template makes a context
for its content, and the context keeps the base URI of the document as a parsed URI. The
text was parsed for every build. A document is a constant of the generated code, so the
parsed URI is kept for the document (by the address and the length of its text) and
copied into the context of each build; a context whose base URI is changed afterwards
changes its own copy.

**The registry keeps the direct property a class answers with.** A direct property is
registered for its owner and for every class it was added to, and a read or a write on an
object of a derived class walked the classes from the class of the object to the one that
has the property, one table lookup each (a template binding to `IsChecked` on a
`CheckBox`: three). The registry keeps the answer of that walk for each pair of a class
and a property identity it was asked about, "none" included, and drops all of them when
a direct property is registered, as it does with its other lists. A property asked for on
its owner never reached the walk and does not reach the table.

In the profile of the `CheckBox` build (about 3 250 samples) the lookup of a direct
property went from 43 samples to 5 and the creation of the context from 47 to 25. Median
of five interleaved rounds of the build with the binds above and the build with these two
as well:

| | Before | After | |
|---|---|---|---|
| `Button`, the template applied | 2.31 ms | 2.17 ms | -6.1 % |
| `CheckBox`, the template applied | 10.83 ms | 10.86 ms | +0.3 % |
| `TextBox`, the template applied | 12.01 ms | 11.98 ms | -0.3 % |

About 0.6 us is saved for a build. The template of a `Button` is built in 11 us, where
that is one part in twenty and every round was lower; the two large templates are built
in 54 us and 60 us, where it is below the noise of the runs. `DirectPropertyBenchmark`
(a read and a write on the owner) is unchanged: -0.2 % and -0.1 %.

What it costs, and what it does not:

- **Memory: one more list for a context, while it builds.** The list of the parents that
  are not null holds one reference for each parent on the stack, and goes with the context
  when the build returns. An element, a style, a template and an expression are the size
  they were, and a built template holds one object less for each dynamic resource and each
  template binding only while it is built (the extension was dropped after the bind).
  One parsed URI is kept for each document a deferred build was made from, and one entry
  of thirty-two bytes for each pair of a class and a direct property that was asked for
  on a class other than the owner; the entries are dropped by the next registration of a
  direct property.
- **The run-time loader is not changed.** It creates the extension and calls
  `ProvideValue`, as before; so does generated code for an extension with any other
  argument than a text key, and for a setter whose value is a dynamic resource.
- **Each bind is compared with the statements it stands for** (`compiled_tests`,
  `template_binds`): the priority in and outside a control template, the anchor of an
  object that is not an element, the variant of a theme dictionary, the target property
  cleared after the bind, and a template binding before and after its templated parent is
  set. The kept base URI is compared with the one a context of the document parses
  itself, for two documents, a relative text and no URI; the kept direct property is
  compared for a derived class, for a class the property is added to after the answer
  was kept, and for a class outside its owners.

### 9.39 The bidi algorithm is linear in the length of a paragraph

The levels of a paragraph are resolved by `BidiAlgorithm` (`media/text_formatting/unicode/`)
once for every paragraph the text formatter is given. Three of its steps cost more for a
character the longer the paragraph was, and one of them made a paragraph of numbers cost
the square of its length. It was found in the profile of a text block whose text is digits
only (the paragraph patch of the patchwork benchmark of the cross-framework comparison,
section 8), where rule W2 was the first entry.

- **Rule W2** changes a European number to an Arabic number when the last strong class
  before it is an Arabic letter. It was resolved by a search backward from each European
  number to the last strong class or the start of the run sequence. Text with no strong
  class before its numbers (digits, spaces and separators only) searched to the start for
  every digit.
- **A level run was removed from the list when a run sequence took it.** The runs of a
  paragraph are in a list; every sequence removed its runs from it (`RemoveAt`), which
  moves all later runs down, once for each run.
- **The run an isolate continues in was found by reading the list** (`FindRunForIndex`),
  from its start, for every isolate initiator that has a matching end.

**Each is now one pass or one lookup.** Rule W2 walks the sequence forward and remembers
whether the last strong class was an Arabic letter. A level run has a flag that says a
sequence took it: the list is not changed while the sequences are made, and it is cleared
when they all are (`LevelRun` is the size it was, 24 bytes). The runs are in the order of
the text, so the run that starts at an index is found by halving the list.

`text::BidiScalingBenchmark` (`tests/FerroUI.Benchmarks`) resolves seven shapes of text at
1 024 and 16 384 code units, so that the cost for a character can be compared between the
two lengths. One run of each build on the same machine, which was not quiet (the figures
of the shapes that did not change differ by its load):

| Shape | 1 024 before | 1 024 after | 16 384 before | 16 384 after | |
|---|---|---|---|---|---|
| Prose | 6.72 us | 6.55 us | 110.2 us | 108.1 us | |
| Numbers and spaces | 302.4 us | 6.84 us | 76 487 us | 109.0 us | 702 times |
| Arabic with numbers | 10.41 us | 9.52 us | 167.9 us | 153.0 us | |
| Hebrew, Latin, numbers | 8.59 us | 7.93 us | 138.7 us | 126.5 us | |
| Brackets | 11.55 us | 10.57 us | 197.1 us | 179.7 us | |
| Isolates | 66.56 us | 18.93 us | 13 462 us | 331.0 us | 40.7 times |
| Embeddings | 15.82 us | 9.94 us | 1 471 us | 159.1 us | 9.2 times |

Before, sixteen times the text cost 253 times as much for numbers, 202 times for isolates
and 93 times for embeddings; after, between 15.9 and 17.5 times for every shape. The
benchmark is part of the framework filter of the local gate (`scripts/perf/gate.py`).

What it costs, and what it does not:

- **Memory: none.** The flag of a level run is in the padding the structure had; nothing
  else is kept.
- **The levels are the ones the old code gave.** The two versions were run side by side
  (the old file as a module of a test build, not kept in the tree) over every sequence of
  up to five of the twenty-three classes at a left to right, a right to left and an
  automatic paragraph level (20.2 million cases), four million random sequences of six to
  forty-eight classes and one and a half million sequences of isolates, embeddings and
  overrides of up to three hundred classes: the resolved levels, the classes the weak
  rules changed and the paragraph level were equal for every one, and so was the one case
  in which the old code stops with an error (below). The sequences had no bracket pairs;
  the rule of the brackets is not changed.
- **Three tests are kept** (`bidi_algorithm_tests`):
  `a_european_number_becomes_arabic_after_an_arabic_letter` (rule W2 across a run
  sequence), `resolves_a_long_paragraph_of_numbers` and
  `resolves_a_long_paragraph_of_isolates_and_embeddings`, which resolve paragraphs long
  enough that the old cost would not finish in the time of a test.
- **Not changed:** an isolate whose end is after a paragraph separator, in classes given
  to `process_classes` directly, stops with "Internal error" as it did. The text formatter
  gives the algorithm one paragraph at a time, where it cannot happen.


### 9.40 The start of a large tree: what an element costs before its first frame

The frames of a scene were measured from the first; what a window costs until it shows its
first frame was not. Two scenes of the start are measured now, phase by phase and with the
memory each phase adds (section 7): `startup/tiles`, 58,696 tiles of six elements on a
canvas (352,187 elements), and `startup/controls`, 1,600 templated controls of the Fluent
theme (24,017 visuals). The tiles start is the one that shows what an element costs: it
took 2.6 seconds and 3.7 GB, about 10.5 KB for an element, of which the element itself was
under 2.2 KB. The rest was its composition visual on the two threads, the batch that
carried it and the queues the first pass filled.

Seven changes, each measured before the next:

- **The dirty sets of the renderer are stamps on the visuals.** `CompositingRenderer` kept
  the visuals to update, those whose properties changed and those whose children are
  recalculated in three hash sets of handles: every invalidation of every element hashed a
  handle and probed a table. A visual now carries the epoch of the sets of its renderer and
  one bit for each (`Visual::renderer_sets`); the sets are lists in the order of adding
  (`VisualSets`), and emptying them takes a new epoch and visits no visual.
- **A new composition visual does not send its defaults.** The constructor set eight
  defaults through the setters, so the first batch of every visual carried them: about 130
  bytes a visual, 48 MB for the tiles. The generator writes the defaults into the fields of
  both sides (one table, `DIRECT_DEFAULTS`), and the first changes a server visual reads
  report the eight fields as changed, so the same inputs are dirty in the same pass.
- **A list of handlers is one pointer wide.** `HandlerList` (`ferroui-system`) held a cell
  with a borrow flag and the source of its tokens, thirty-two bytes for an event nobody
  listens to, and an element has twelve of them in its base classes. The list is the
  pointer alone, taken out of its cell for the length of an operation; the tokens come
  from one counter of the process, so they are unique and increasing across lists.
- **The coercion fields of an effective value are boxed, and so is the hit test index of a
  visual.** An effective value kept the two uncoerced values of a property with a coercion
  callback inline in every value (a `Thickness` value is 176 bytes instead of 264, a
  reference 144 instead of 200); a visual kept the index of its children for hit testing
  inline, 120 bytes that only a visual with thirty-two children or more uses.
- **The handles of a generated property block are in a cell without a borrow flag.**
  `OptionCell` is what `RefCell<Option<Rc<_>>>` was used for (a read clones, a write
  replaces), eight bytes smaller for each of the eight handles of a visual on each side.
- **An emptied transport buffer keeps at most four megabytes.** The streams of a batch,
  the readback queue, the serialization queue and its set, the own-properties queue and
  the loaded queue grew to the size of the first batch and kept it for the life of the
  window, about 390 MB for the tiles. Each gives back what is beyond
  `RETAINED_BUFFER_BYTES` when it is emptied (`utilities::ReleaseExcess`). The largest
  batch of the steady-state scenes is under three percent of the limit.
- **The rarely set properties of a visual are in a second block.** The transform other
  than the offset, the geometry clip, the opacity mask, the effect, the cache mode, the
  adorner link and the WinUI additions (twenty-one properties, `RARE_FIELDS` of
  `scripts/generate_composition.py`) are about four hundred bytes on each side, and few
  visuals set any of them. The block is made, on both threads, when one of them is first
  given another value than the one it is made with (`has_rare_properties`).

The sizes, in bytes (`sizes_of_an_element`, kept exact by the gate):

| Object | Before | After |
|---|---:|---:|
| `Border` | 2168 | 1880 |
| `TextBlock` | 2136 | 1848 |
| `StackPanel` | 1864 | 1576 |
| `CompositionVisual` (UI thread) | 1032 | 480 |
| `ServerCompositionVisual` | 1080 | 624 |

The tiles start, median of five runs alternating the two builds on a quiet machine:

| Figure | Before | After | Change |
|---|---:|---:|---:|
| Build of the tree (`build_ms`) | 470.1 | 409.6 | -12.9 % |
| Show: attach, styles, layout (`show_ms`) | 899.5 | 732.0 | -18.6 % |
| First frame, UI thread (`first_ui_ms`) | 374.3 | 337.1 | -9.9 % |
| First frame, render thread (`first_render_ms`) | 731.9 | 632.5 | -13.6 % |
| Show to first frame (`first_frame_ms`) | 1035.3 | 898.1 | -13.3 % |
| Start (`startup_ms`) | 2596.1 | 2201.7 | -15.2 % |
| Memory of the build (`build_mb`) | 1422.2 | 1129.8 | -20.6 % |
| Memory of the show (`show_mb`) | 1005.7 | 713.1 | -29.1 % |
| Memory of the first frame (`first_frame_mb`) | 1277.4 | 1009.0 | -21.0 % |
| Memory of the start (`startup_mb`) | 3705.3 | 2851.9 | -23.0 % |

The controls start, where an element is a small part of what a control costs, was measured
for the last change alone: 302.6 to 297.9 ms and 367.2 to 350.5 MB.

What it costs, and what it does not:

- **Resident memory does not show the transport buffers.** The buffers that are given
  back are about 230 MB of heap for the tiles, and the macOS allocator keeps freed large
  blocks resident until the system asks for memory: `first_frame_mb` is the same with and
  without that change. What lowers the footprint is not making them, which is later work.
- **A visual that sets a rare property pays one allocation for the block,** once, and a
  read of such a property of a visual without the block returns the value it would be
  made with. A visual that sets none of them never has it.
- **A removed visual stays in a set of the renderer until the set is compacted** (when
  the stale entries are half of the list, or the set is read), not until the removal.
- **`HandlerList::with_shared_ids` and `with_ordered_ids` are gone:** every list has what
  they asked for.
- **Tests:** `object_size_tests` keeps the two visuals in the allocator blocks they fit in
  now; `compositor_tests` covers the rare block on both threads and the defaults of a new
  visual; `release_excess` and `option_cell` have their own.

### 9.41 Composition visuals on demand: an element that draws nothing and sets nothing has none

Every attached element had a composition visual on the UI thread and a server visual on the
render thread, made when it joined the tree. A third of the elements of the tiles start
(117,399 of 352,187) record no drawing and set nothing the compositor applies: a panel
without a background, a presenter, a decorator. Their two visuals did nothing but move
their children, which an offset does as well.

An attached element now has a composition visual or has none yet. It gets one, and keeps
it until it leaves the tree, when one of these holds (`Visual::needs_own_composition_visual`):

- it is the root of its presentation source;
- its recording is not empty (`RenderDataDrawingContext::recorded_nothing`);
- it sets something the compositor applies: an opacity other than one, a clip (to its bounds or a
  geometry), an opacity mask, an effect, a cache mode, a render transform, a mirror or manipulation transform,
  render or text options other than the defaults, or it is not visible;
- it has composition extras (a layer, unloading visuals, facades, a translation, implicit
  animations) or a child composition visual, or it casts a theme shadow (the shadow asks
  for the visual of its caster);
- it has a custom hit test, or its class says so (`requires_composition_visual`, a new
  virtual beside `create_composition_visual`);
- it is asked for: `composition_visual()` and `attach_to_compositor` make it. The parent
  of an element whose visual was handed out has one as well, so that the `Offset` and the
  `Parent` a caller reads keep their meaning.

What an element without a composition visual changes for the others:

- **Offsets.** The offset of a composition visual is the position of its element plus the
  positions of the elements without one between it and the nearest ancestor that has one.
  An element without a visual that moves moves the offsets of the nearest visuals below
  it (up to eight visuals and thirty-two elements; beyond that it gets a visual).
- **Children.** The children of a composition visual are the nearest composition visuals
  below its element, in the Z order of each level.
- **The update pass.** The renderer records an element first and decides then
  (`CompositingRenderer::update_core`): an element that recorded nothing and needs nothing
  is passed over, and the nearest ancestor with a visual synchronizes its children. A
  visual that is made in a pass, or between two passes, is placed by the renderer
  (`IRenderer::composition_visual_materialized`): its children leave the ancestor that
  showed them and their offsets are taken from it.
- **Hit testing.** The filter of a hit test is asked for the elements without a visual
  between a visual and the element of its parent visual as well. A hit test from a root
  without a visual tests the nearest visuals below it, topmost first. A composition visual
  that was made for an element the compositor already showed without one has no readback
  until the first frame with it: until then a hit test goes through it to its children
  with the input of its parent, so what is under the pointer is found in the frame between.

The process-wide switch `Visual::set_composition_visuals_on_demand(false)` gives every
element a visual again; the start scenes are measured with both.

The two start scenes, the median of seven runs that alternate the two settings of the
switch in one build (Apple M3 Pro, release build, a quiet machine; the live heap and the
counts are from the build with counters):

| The tiles start (352,187 elements) | Every element has one | On demand | Change |
|---|---:|---:|---:|
| Composition visuals made | 352,187 | 234,791 | -33.3 % |
| Objects in the first batch | 4,284,937 | 3,228,373 | -24.7 % |
| Show: attach, styles, layout (ms) | 753.7 | 669.1 | -11.2 % |
| First frame, UI thread (ms) | 394.6 | 459.1 | +16.3 % |
| First frame, render thread (ms) | 733.5 | 586.6 | -20.0 % |
| Show to first frame (ms) | 1080.3 | 966.8 | -10.5 % |
| Start (ms) | 2399.6 | 2234.0 | -6.9 % |
| Memory of the start (MB) | 2578.7 | 2439.3 | -5.4 % |
| Live heap after the first frame (MB) | 2322.4 | 2098.4 | -9.6 % |

| The controls start (24,017 visuals) | Every element has one | On demand | Change |
|---|---:|---:|---:|
| Composition visuals made | 24,017 | 14,811 | -38.3 % |
| Objects in the first batch | 253,580 | 171,926 | -32.2 % |
| Show: attach, styles, layout (ms) | 227.7 | 222.0 | -2.5 % |
| First frame, UI thread (ms) | 28.0 | 30.1 | +7.5 % |
| First frame, render thread (ms) | 24.1 | 19.2 | -20.5 % |
| Show to first frame (ms) | 48.7 | 45.8 | -5.9 % |
| Start (ms) | 295.5 | 288.7 | -2.3 % |
| Memory of the start (MB) | 350.6 | 329.7 | -6.0 % |
| Live heap after the first frame (MB) | 302.4 | 283.3 | -6.3 % |

The UI thread part of the first frame grows because the visuals that are made are made
there, in the first pass of the renderer, and no longer in the show, which shrinks by
more. What remains is what the visuals that draw cost: in the tiles start the first frame
leaves 799 MB allocated for 234,791 of them, 3.4 KB each on the two threads.

What it costs, and what it does not:

- **`composition_visual()` of an element that had none makes it,** and the server has it
  after the next batch: code that reads the server visual right after asking waits for the
  commit, as it does for any new visual.
- **A visual asked for after a pass is synchronized when it is made** (offset, size, the
  children around it), and the next pass does not synchronize its properties again: what
  the caller sets on it in between, the `Offset` an entrance animation starts from, stays,
  as it does on a visual that existed all along (`CompositingRenderer::place_materialized`).
- **A class that overrides `create_composition_visual` to show something itself** (as
  `ExperimentalAcrylicBorder` does) overrides `requires_composition_visual` to return
  `true`; a class whose `custom_hit_test` answers only for some points does the same.
- **The children list of a visual can be longer:** the canvas of the tiles start has the
  visuals of the parts of its tiles as children, not the tiles. A change of the list is
  the clear and refill it was; the hit test index is the one of a visual with thirty-two
  children or more.
- **The cache mode invalidates the composition properties** when it changes (it had no
  change handler), so an element that gets one gets its visual in the next pass.
- **A value assigned before an animation is started, in one batch, is sent ahead of the
  animation.** The transport carried the animation or else the value, so the value was
  lost and the server kept its own for the components a sub-channel animation does not
  write. Nothing showed it while the two were equal. The selection indicator of a top
  navigation view asks for the visual of its indicator, which gives the parent panel a
  visual and moves the position of the panel out of the offset of the indicator, resets
  `Offset` and animates `Offset.X`: the server kept the old `Y` and drew the indicator
  below the clip of the pane. The generated serialization now writes the value and then
  the animation; an implicit animation still starts from the value the server has
  (DEVIATIONS.md).
- **Tests:** `compositor_tests` covers the elements without a visual (offsets through
  them, flattened children, an element that starts to draw, one that is asked for its
  visual in and out of a pass, an offset set on a visual asked for after a pass, hit
  tests from a root without a visual and through a new visual before its first frame);
  the counter check `composition_work` counts the visuals made.

### 9.42 Drawing items: an element that only draws has no composition visual

After section 9.41 the composition visuals that are left are those of the elements that
draw, and most of those need nothing else of one: a text block, a border, a shape records
a drawing and its layout says where it is. Each still had a `CompositionVisual` and a
draw list on the UI thread and a `ServerCompositionVisual` on the render thread, with a
transform, bounds, a dirty state, a readback and a place in two lists of children of its
own: in the tiles start 1.7 KB of live memory on the two threads beside the drawing
itself, for an offset.

An element that draws, and needs a composition visual for nothing else, now has none
(`Visual::needs_own_composition_visual` no longer asks whether the recording is empty).
What it drew is a *drawing item* of the composition visual of the nearest element above
it that has one, the holder: the render data, where the drawing is in the holder (the
position of the element plus the positions of the elements without a visual between), and
the size of the element when it clips to its bounds. `Visual::has_composition_item` says
that an element is shown this way.

- **Which elements.** The rule of section 9.41 without its second line. An element that
  clips to its bounds is an item while nothing is under it: the clip of an item is the
  rectangle of its size, and an element that clips its children keeps its composition
  visual, which clips them. A class whose drawing needs its composition visual says so
  with `requires_composition_visual`: a `Border` that clips to its bounds (the clip follows
  its corner radius) and the selection indicator (its content draws it) do.
- **Which holders.** A draw-list visual, with or without an extension that says it shows
  items (`ICompositionDrawListVisualExtension::holds_drawing_items`, `false` by default;
  the border's says `true`). A visual that draws acrylic, or one with another extension,
  shows its own drawing only: the elements below it that draw get composition visuals as
  before.
- **The order.** The items and the child visuals of a holder are painted in one order,
  that of the visual tree below the holder (the Z order of each level). The holder keeps
  the items in that order and, for each child visual, the number of items painted before
  it.
- **From an item to a visual, and not back.** An element that comes to need a composition
  visual (an opacity, a transform, something under an element that clips, a caller that
  asks for `composition_visual()`) gets one, as in section 9.41: its item leaves the
  holder and the drawing it has becomes the draw list of the visual, without a new
  recording. It keeps the visual until it leaves the tree. An element without a visual
  that gets one above its items becomes their holder, and they move to it.
- **Moves.** An item moves with its element and with the elements without a visual above
  it; it counts as a visual in the bound of section 9.41 (eight visuals and thirty-two
  elements below an element that moves, beyond which that element gets a visual).
- **Out of the tree.** The item of an element that leaves its tree is kept where the
  composition visual of an element is kept (section 9.32: until the next pass of the
  renderer, or beside a row in a recycle pool) and is listed again with the drawing it
  has when the element is back (`IRenderer::keep_departed_composition_item`; the counter
  `drawing items kept` counts the items that come back this way).

**The transport.** The serialization of a holder carries the changes of its items: the
one range of the list that was replaced (where it starts, how many items it removes, and
the items that take their place: the render data, the offset, the clip), the positions of
the child visuals when they changed, and for each item that stays and changed its index
and what changed (another drawing, the same drawing recorded again, the offset, the
clip). An item is not an object of the compositor: it has no identifier, is not
registered for serialization and has no readback.

**The render thread.** A `ServerCompositionDrawListVisual` has the items of its holder
(`ServerDrawItems`): for each the render data, the fragment the backend kept of it (as
for a draw list), the offset, the clip and the bounds it last reported.

- The bounds of the own content of the holder are those of its drawing and of its items.
- A change of an item makes dirty where the item was and where it is, not the holder: a
  holder of thousands of items whose one item changes redraws that item and what
  overlaps it. The rectangles of removed items are kept until the next frame (sixteen
  apart, the rest united with the last).
- The holder draws its own drawing, the items before its first child visual, the child,
  the items before the next, and the rest after the last child. An item outside the clip
  or the dirty region of the frame is passed over. An item is drawn with the transform of
  the holder moved by its offset, inside its clip when it has one.
- An item observes its render data: a resource of the drawing that changes (a brush that
  is animated) drops the fragment of that item and makes its bounds dirty.
- The bitmap cache of a visual above is rendered again when an item
  below it changes, as for a visual.

**Hit testing.** The signatures of the hit tests are the same, and an item is found as
its element. The items and the child visuals of a holder are tested in one order, from
the last painted: the items after a child, the child, the items before it. An item is
tested with the input in the coordinates of the holder moved by its offset, against its
clip and then its drawing; the filter is asked for the element of the item and for the
elements without a visual between it and the holder. A geometry is tested against a
drawing only when it meets the bounds of that drawing
(`ICompositionHitTester::drawing_bounds_match`), which is what the bounds of the subtree
of a composition visual did for the drawing of that visual. The hit test of the
composition API (`CompositionTarget::try_hit_test`) answers with the holder, once, for
any number of its items that are hit. An item is where the last layout put it; a
composition visual is where the last frame drew it (DEVIATIONS.md).

A holder with 32 child visuals and more looks them up in the index of its children
(`CompositionHitTestAabbTree`) as a visual without items does: the index answers with each child and where it is
among the children, and the items painted after that child, down to the child the index
answered with before, are tested ahead of it. A holder of 32 items and more keeps the
bounds of every 32 items that follow each other (`DrawItemList::chunk_bounds`), calculated
when a hit test first asks for them and again for a chunk after one of its items changed
(a drawing, an offset, a clip) or the list changed from that chunk on; a test passes over
the chunks whose bounds the input misses. A list of fewer items has no such bounds, and
none is kept for a holder that was never hit tested.

**A holder that is drawn again.** Upstream lists the child composition visuals of every
visual that was drawn again in a pass, which is a walk over the children of that visual. The
walk of a holder goes down through every element without a composition visual to the nearest
ones that have one: the root of the `width` scene, drawn again every frame, walked the 1,200
elements below it every frame to make the lists it had (0.127 ms of UI time a frame
where the base has 0.074 ms). A visual with a composition visual now remembers that
its lists are those of the tree below it (a bit of the state a visual has for section 9.41;
a visual is no larger), and a pass does not list it again when it was only drawn again
(0.064 ms). What makes it forget:

- the renderer is told that its children changed (`IRenderer::recalculate_children`: a
  child attached, the Z order of a child, a visual below without a composition visual that
  gained, lost or moved a child);
- an item of it came, left, was released or stopped being drawn, or a visual below it got a
  composition visual;
- its own visual children changed (a child removed or moved, which upstream leaves to the
  listing of a visual that is drawn again);
- it left its tree.

A debug build lists it all the same and fails when a list changed (the child visuals, the
items, the positions of the child visuals among the items), so every test that runs a pass
of the renderer checks the rule; the counter `composition children listed` counts the
listings, and `composition_work_of_a_holder_that_is_drawn_again` asserts them for a holder
drawn again, for each of the changes above and for the passes after each.

The two start scenes, before (section 9.41, the base of the gate) and after, the medians
of the gate (5 rounds that alternate the two builds; Apple M3 Pro, release
build; the counts and the live heap are from the build with counters):

| The tiles start (352,187 elements) | Section 9.41 | Drawing items | Change |
|---|---:|---:|---:|
| Composition visuals made | 234,791 | 58,699 | -75.0 % |
| Drawing items | 0 | 176,092 |  |
| Objects in the first batch | 3,228,373 | 1,760,940 | -45.5 % |
| Build (ms) | 423.2 | 425.0 | +0.4 % |
| Show: attach, styles, layout (ms) | 644.2 | 646.4 | +0.3 % |
| First frame, UI thread (ms) | 421.5 | 370.6 | -12.1 % |
| First frame, render thread (ms) | 581.8 | 470.4 | -19.2 % |
| Show to first frame (ms) | 932.6 | 767.7 | -17.7 % |
| Start (ms) | 2158.1 | 1981.6 | -8.2 % |
| Memory of the first frame (MB) | 1017.5 | 594.4 | -41.6 % |
| Memory of the start (MB) | 2434.9 | 1974.3 | -18.9 % |
| Live heap after the first frame (MB) | 2098.4 | 1803.0 | -14.1 % |

| The controls start (24,017 visuals) | Section 9.41 | Drawing items | Change |
|---|---:|---:|---:|
| Composition visuals made | 14,811 | 9,073 | -38.7 % |
| Drawing items | 0 | 5,738 |  |
| Objects in the first batch | 171,926 | 123,487 | -28.2 % |
| Build (ms) | 3.5 | 3.4 | -1.4 % |
| Show: attach, styles, layout (ms) | 225.1 | 227.0 | +0.9 % |
| First frame, UI thread (ms) | 32.1 | 30.7 | -4.1 % |
| First frame, render thread (ms) | 20.0 | 17.4 | -13.3 % |
| Show to first frame (ms) | 48.0 | 44.3 | -7.8 % |
| Start (ms) | 292.6 | 290.4 | -0.8 % |
| Memory of the first frame (MB) | 90.5 | 77.6 | -14.3 % |
| Memory of the start (MB) | 322.5 | 309.6 | -4.0 % |
| Live heap after the first frame (MB) | 283.3 | 272.5 | -3.8 % |

**What the first gate run measured.** The first run of the gate for this change reported the
build of the `list` scene 67 % slower and the wall time of a frame of `list` and `width` 5 to
6 % slower. Neither is a cost of the change, and the gate measures both differently now
(section 8.1):

- The scenes of the frames ran in one process, and `list` was built after six others. With
  fewer composition objects the scenes before it left other blocks free, and its build moved
  from 1.6 to 2.7 ms; in processes of their own the two sides build it in 1.19 and 1.10 ms.
  Every test of the frame benchmarks now runs in a process of its own.
- The loop of the benchmark does not wait for a display. The clock of the UI thread goes on
  when the render thread has accepted a batch, and a commit waits until the batch before it
  is processed (as upstream), so a frame of the clock that starts in between commits
  nothing (section 9.46 moved the commit to the acceptance too, and no frame of the clock
  commits nothing since). With the batch of a thousand visuals of these two scenes the base presented 226
  to 228 of its 240 frames; with the batch of one holder the candidate presents 237 to 240.
  The base was faster by the frames it did not draw: for a frame the surface received the
  two are within 2 % (`width` 3.72 and 3.73 ms, `list` 3.57 and 3.51 ms; 6,000 frames of
  `width`: 3.906 and 3.859 ms). The gate now compares the render thread and wall time for a
  frame the surface received. The scenes whose render thread is slower than their UI thread
  present every second or third frame of the clock on both sides (`lines`, `moves` and
  `items` 120 of 240, `text` 160), so their render and wall times in the tables of the gate
  are from here on two and one and a half times those of the sections before.

What it costs, and what it does not:

- **A holder lists its items again when the tree below it changes** (an element added,
  removed or moved, an element that starts or stops drawing, an item that becomes a
  visual), and not when it is only drawn again: the walk is over the elements below it
  down to the nearest composition visuals, and the difference to the list the server has
  is sent as one range. A holder of thousands of items of which a few come and leave every
  frame pays the walk every frame (the `items` scene measures it); the positions of the
  child visuals are sent whole when they change.
- **The items of a chunk that the input meets are tested one by one**, from the last: a
  holder of thousands of items that lie over each other tests them all, as a visual does
  the children whose bounds overlap. The bounds of a drawing are calculated from its
  recording for a geometry test and for the bounds of its chunk.
- **`composition_visual()` of an element that is an item makes its visual**, as for an
  element that had none (section 9.41). Code that walks the composition tree finds the
  holder where it found a visual of each element; the element says which with
  `has_composition_visual` and `has_composition_item`.
- **A custom extension of a draw-list visual** whose server content draws the render data
  of the visual itself shows no items unless both sides say so
  (`holds_drawing_items`, and `IServerVisualContent::draw_items` on the render thread).
- **Sizes.** An item is 72 bytes on the UI thread (a block of 96) and 120 on the render
  thread (a block of 144), where the two visuals were in blocks of 512 and 640 bytes
  with their handles and lists around them (`object_size_tests`). The list of a holder
  is made with its first item.
- **Tests:** `compositor_tests` covers an element that draws as an item, the clip of an
  item, the paint and hit order of items and child visuals, a geometry that meets and
  misses the bounds of items, items among indexed child visuals, the bounds of the chunks
  of a long list of items and what makes them stale, items that follow their elements and the elements above
  them, an item that moves to the visual an element above it gets, an item that becomes
  a visual with its drawing, an item kept while its element is out of the tree, the
  dirty rectangles of changed and removed items, the items drawn in a dirty area and a
  bitmap cache above an item that changes; the counter checks `composition_work` count
  the items recorded and drawn and the listings of a holder; the frame scene `items` (section 8.1) measures a holder
  of 6,000 items of which some are drawn anew, some move and some leave the tree and
  come back every frame.
### 9.43 The binding of a setter is instanced when its value is first asked for

A setter whose value is a binding (a `{DynamicResource}`, a `{Binding}`, a
`{TemplateBinding}` in a style) gave the frame of its style instance a binding
expression as the entry of its property, as upstream does: the expression was created
and attached when the style was attached, and started when the store first asked the
entry for a value. Most of them are never asked. A style that is not active (a
`:pointerover` or `:pressed` style of a control nobody points at) is not evaluated,
and neither is a setter below the effective value (a local value, a style later in the
order, a control theme under a style). In the start of the controls scene
45,198 of the 120,487 binding expressions that were created were never
started, every one of them the dynamic resource of a setter, and the expression is the
largest object a style attach creates: a block of 512 bytes, about 23 MB (45,198 blocks of 512 bytes) of live
heap at the first frame for expressions that did nothing.

The entry of such a setter is now a `SetterBindingEntry`: the binding, the property,
the owner and the priority of the frame, 64 bytes (a block of 80). It makes the
expression from the binding and attaches it the first time the store asks it for a
value (`has_value`, `try_get_value`, `get_value_boxed`) or for the expression itself
(`BindingOperations::get_binding_expression_base` of a property whose value comes from
the setter), and from then on answers with the expression.

- **The entry stays the entry.** The store compares entries by identity (the entry of
  an effective value, the order of two entries of one priority), so the record is the
  entry of the frame before and after the expression exists, and it is the sink of its
  expression: a change of the expression reaches the store as a change of the record
  (`ValueStore::on_entry_changed`, which `on_expression_changed` now calls for an
  expression that is its own entry).
- **What is asked of an expression that was never started** is its property and to stop
  (`unsubscribe`, when the style is deactivated or detached). The record answers both
  without an expression.
- **An inactive frame instances nothing.** `get_expression` asks the entries of the
  active frames only, as the evaluation of the effective value does.
- **A direct property** has no value entries; its setter binds as before.

What does not change: when a binding of a setter is started (when its value is first
evaluated), stopped (when the style is deactivated or detached, or the value is no
longer the effective one) and started again, the priority its value has, the expression
`get_binding_expression_base` answers with, and data validation.

The start of the controls scene, before (main, the base of the gate) and after; the
medians of the gate (5 rounds that alternate the two builds; Apple M3 Pro, release
build), and the counts and the live heap from one run of each with counters:

| `startup/controls` | Before | After | Change |
|---|---:|---:|---:|
| Show | 225.7 ms | 223.3 ms | -1.0 % |
| Start (to the first frame) | 292.4 ms | 289.9 ms | -0.8 % |
| Memory of the show | 228.3 MB | 210.7 MB | -7.7 % |
| Memory of the start | 329.6 MB | 312.5 MB | -5.2 % |
| Live heap the show leaves (counters) | 189.1 MB | 174.4 MB | -7.8 % |

| Counted in the start of the controls scene | Before | After |
|---|---:|---:|
| Binding expressions created | 120,487 | 75,289 |
| Binding expressions started | 75,289 | 75,289 |
| Dynamic resource expressions created | 69,239 | 24,041 |
| Dynamic resource expressions started | 24,041 | 24,041 |
| Setter binding entries created | | 60,816 |

The time hardly moves: an expression that is never started costs its allocation and its
attach, and the time of the show is in the expressions that start (the resource lookup
of each, its subscriptions) and in the rest of the styling. The other scenes of the gate
do not change beyond their tolerances (`startup/tiles` has no styles with bindings).

- **Tests:** `binding_setter_tests`: a setter below the effective value is not
  instanced, also after the style is detached; a setter of an inactive style is
  instanced once when the style becomes active and keeps its expression while the
  style goes inactive and active again; a setter below a local value is instanced when
  the local value is cleared; the value follows its source and replaces a current
  value; the expression of the property is the expression of the entry; the size of
  the record. The counter check `setter_binding_work_of_styles_that_are_not_evaluated` asserts the expressions created and
  started in the start of the controls scene.

### 9.44 A dynamic resource does not look up again for a change it has seen

A dynamic resource follows the resources and the theme variant of its host: it subscribes
to both and looks its resource up again for every notification. When an element is
attached to a tree its dynamic resources were updated twice there. Setting the parent sets
the inheritance parent first, and the theme variant the element inherits from it makes
each of its dynamic resources look up (with the new styling parent, since the walk of a
lookup goes up the inheritance parents); then the attachment applies the styles of the
element, whose setters start their dynamic resources, and makes a notification of changed
resources, which makes every dynamic resource of the element look up again. The second
lookup sees the tree the first saw and publishes the value the first published.

- **The notification of an attachment is numbered when the element gets its parent.**
  `set_parent` makes it (`ResourcesChangedEventArgs::create`) before it sets the
  inheritance parent, and every element of the attached subtree receives that one
  notification (`LogicalTreeAttachmentEventArgs` carries it to the elements below and to
  the template children). Each element made its own, after its styles were applied.
- **A dynamic resource remembers when it looked up.** `DynamicResourceExpression` takes
  the last sequence number of the notifications
  (`ResourcesChangedEventArgs::last_sequence_number`) before each lookup and does not
  look up again for a notification that is not newer
  (`ResourcesChangedEventArgs::is_newer_than`).

Why it is correct: every notification of the framework is made after the change it
describes, as upstream's is (a dictionary that is changed, a dictionary or a style that
gets or loses its owner, the accent or the density of a theme, an element that gets or
loses its parent, the application forwarding its own). A lookup made after the
notification was numbered was made after the change, and saw it. The number is taken
before the lookup, so a change notified while the lookup runs is newer. Arguments that
`create` did not number (the default) are newer than any moment.

The start of the controls scene and the `list` scene, before (section 9.43, the base of the
gate) and after; the medians of the gate (5 rounds that alternate the two builds; Apple M3
Pro, release build), and the counts from one run of each build with counters:

| Scene | Before | After | Change |
|---|---:|---:|---:|
| `startup/controls` show | 223.7 ms | 216.8 ms | -3.1 % |
| `startup/controls` start (to the first frame) | 292.1 ms | 281.9 ms | -3.5 % |
| `frames/list` show | 2.79 ms | 2.00 ms | -28.2 % |
| `frames/list` start | 4.02 ms | 3.11 ms | -22.7 % |

| Counted in the start of the controls scene | Before | After |
|---|---:|---:|
| Resource lookups | 79,670 | 56,059 |
| Resource hosts probed | 1,127,451 | 778,141 |
| Lookups of dynamic resources | 56,853 | 33,242 |
| Notifications already seen | | 23,611 |
| Binding values published | 112,904 | 89,293 |
| Property changes raised | 426,905 | 426,905 |

The property changes are the same: the lookups that are left out published the value the
property had. The memory of the starts does not change.

The framework benchmark `styling_subsystem::DynamicResources` guards it: a section of 1,000
texts whose tags are a dynamic resource of the root, in a theme variant scope, detached and
attached again (the gate, base section 9.43, 5 rounds):

| `styling_subsystem::DynamicResources` | Before | After | Change |
|---|---:|---:|---:|
| `reattach` | 1.862 ms | 1.693 ms | -9.1 % |
| `detach` | 1.470 ms | 1.377 ms | -6.3 % |
| `reattach_with_inactive_setters` (four dynamic-resource setters of a style that does not match yet) | 2.331 ms | 2.127 ms | -8.8 % |

- **Tests:** `resource_tests::a_notification_is_newer_than_the_moments_before_it_was_made`;
  `dynamic_resource_expression_tests::attachment` attaches a group of elements with
  dynamic resources to a root, with and without a theme variant to inherit, and checks
  the values after the attachment, a later change and the detachment; with the counters
  it asserts one lookup for each resource in the attachment.

### 9.45 A template binding is told of its templated parent by the store of its target

A template binding reads a property of the templated parent of its target. To see the
templated parent change, upstream's expression subscribes to every property change of its
target, and so did this one: a handler (a closure in a block of its own) on the handler list
of the target for each template binding, called for every property change of the target to
compare the property with `TemplatedParent`. An element of a control template has several
template bindings (a border of a button binds its background, its border brush and
thickness, its corner radius and its padding), and its properties change many times while the
template is applied and laid out. The start of the controls scene has 45,214 template
bindings.

- **The store tells them.** `StyledElement::set_templated_parent` asks the value store of the
  element to tell its binding expressions (`ValueStore::templated_parent_changed`): the local
  value bindings, and the entries of the frames the element does not share. A template
  binding that runs reads the new templated parent, subscribes to it and publishes, as it did
  in its handler. A setter entry of a style (section 9.43) tells the expression it made, and a
  multi-binding the expressions of its bindings, which have its target and are no entries of
  the store.
- **A template binding that writes to the source** (`TwoWay`, `OneWayToSource`) still
  subscribes to the property changes of its target, for its target property.

The store is told before the property change of `TemplatedParent` is raised, where the
handler ran with the other handlers of the target: the values the template bindings publish
are in place when the handlers of `TemplatedParent` run.

The start of the controls scene, before (section 9.44, the base of the gate) and after; the
medians of the gate (5 rounds that alternate the two builds; Apple M3 Pro, release build):

| `startup/controls` | Before | After | Change |
|---|---:|---:|---:|
| Show | 219.8 ms | 221.3 ms | +0.7 % |
| Start (to the first frame) | 286.3 ms | 288.5 ms | +0.8 % |
| Memory of the show | 210.8 MB | 207.1 MB | -1.8 % |
| Memory of the start | 312.3 MB | 308.9 MB | -1.1 % |

The time does not change beyond the noise (an earlier run of the gate measured -0.8 %): the
handlers it removes were short calls, and the work of a template binding is the one value it
publishes when its templated parent is set (section 9.43, the profile). What it removes is the
handler block of each template binding and the entries on the handler lists of its target,
3.4 MB in the start of the controls scene.

- **Tests:** `binding_model_tests::template_binding_follows_the_templated_parent_without_observing_its_target`
  (a template binding follows a templated parent set, replaced and cleared, with no handler
  on its target, and a two-way one with one);
  `a_template_binding_of_a_style_setter_follows_the_templated_parent`.

### 9.46 A commit is asked for when the batch before it is accepted: one render pass for a picture

Section 9.33 let the media context start its next render pass when the render thread accepts
a batch (takes it from the queue), not when it has processed it. The compositor kept
upstream's rule for the other half: a change made while a batch is in flight asks the
scheduler for a commit only once that batch is processed
(`Compositor::request_composition_batch_commit_async`). The two no longer met. The pass that
acceptance starts runs the animation frame callbacks and the layout while the render thread
is still reading the batch; the first change of that pass finds the batch in flight and
defers its request; the pass reaches its commit step with no commit asked for and ends. The
request arrives when the batch is processed, and a second pass, with a second run of the
animation frame callbacks and a second layout, commits what the first one changed.

Every picture of an animated screen therefore cost two render passes. The gauntlet at tier
16 ran its animation frame callback sixty times a second and the compositor committed
thirty batches; at tier 12 it was 120 callbacks for 60 batches. `PERF fps` counts callbacks,
which is why the tables of section 9.33 read sixty. The harness of the benchmark, which
counts presented frames from outside, and the frame gate, whose `GATE` lines said
`"frames":240` and `"rendered":120`, both had it; the gate compared the UI thread time for a
frame of the clock, so two half-empty frames read as one cheap one.

**The change.** The request for a commit is held back until the batch before it is
accepted, the signal the media context starts its pass on, and is made at once when it
already is. The compositor reads the completion itself (`is_completed`) instead of relying
on a continuation having run, so the order in which the continuations of a batch run does
not decide which pass commits. One batch at most is still in flight beside the one being
prepared (the commit itself waits for the acceptance in `MediaContext::commit_compositor`),
and what observes the effects of a batch still waits for `processed` or `rendered`.

What it measures (M3 Pro, 60 Hz, the harness of the benchmark with FerroUI and Cranpose in
the same run, first and second launch; the machine was not quiet, so the figures compare
the columns and are not calibrated; the column after also has the balanced bands of the
Skia layer):

| Scene | FerroUI before | FerroUI after | Cranpose |
|---|---|---|---|
| gauntlet, tier 16, frames per second | 21.8 to 22.8 / 27.5 to 29.1 | 37.1 / 47.1 | 55.5 / 58.1 |
| gauntlet, tier 16, processor time for a frame | 61 to 69 ms | 51.3 to 51.5 ms | 20.4 to 20.5 ms |
| patchwork, tier 17, frames per second | 33.1 to 36.7 / 42.6 to 46.2 | 38.1 / 48.2 | 28.2 / 30.0 |
| patchwork, tier 12, frames per second | 48.3 / 57.2 | 48.7 / 57.3 | 55.7 / 57.5 |

Run alone, the gauntlet at tier 16 commits 44 to 48 batches a second for as many callbacks
(30 for 60 before), and at tier 12 sixty for sixty (60 for 120): the UI thread does half the
work for the same pictures. The frame gate's gauntlet (raster Skia, no display) receives 240
of 240 frames where it received 120, at 14.3 ms of UI thread time for a frame received
where it was 20.4, and 34.2 ms of wall time where it was 37.0.

The gauntlet at tier 16 is not at the rate of the display yet. A frame costs the UI thread
about 15 ms (the recording of the render data 7.4 ms, layout 6.9 ms), which leaves no room
in a refresh interval beside the wait for the acceptance, and the render thread waits about
9 ms of a frame for a drawable of the layer.

- **Tests:** `render_thread_tests::a_commit_is_asked_for_at_once_when_the_batch_before_is_accepted`
  (the render thread is held between the acceptance and the processing of a batch: a change
  made then reaches the scheduler at once; it failed before the change) and
  `a_commit_asked_for_before_the_batch_before_is_accepted_waits_for_the_acceptance`.
- **Gate:** the frame gate fails a scenario whose surface receives fewer than nine in ten of
  its frames, and `scripts/perf/gate.py` compares the UI thread time, as the render thread and
  wall time, for a frame the surface received.

### 9.48 Children tables: the update pass and the render walk leave out children with nothing to do

A composition visual with many children (the items of a wall, the rows of a long list that
have visuals of their own) was walked child by child twice in every frame, as upstream
walks it. The update pass called `pre_subgraph` for every child and returned at once for
the ones without a flag; the render walk called `handle_pre_graph_transform_clip_opacity`
for every child and left the ones whose bounds do not meet the dirty region. Neither does
any work for such a child, but each reads the visual (the flags, the bounds, the
transform), and a visual is 640 bytes somewhere on the heap: with twenty thousand children
and one that changed, a frame brought twenty thousand visuals into the cache twice. On the
still wall of the frame gate (`wall/20`: 20 000 items, a few change in a frame) this was
most of the render thread's frame.

**The change.** A collection of 32 children or more (`TABLE_FROM`) keeps a table beside
the list of its children (`ChildrenTable` in `server_composition_visual_collection.rs`),
with a row for each child in the order of the children:

- whether the update pass has something to do in the child or below it (one byte);
- the bounds of the child's subtree in the space of the holder, its
  `transformed_sub_tree_bounds` (32 bytes; a child without bounds has bounds that meet
  nothing).

Each visual knows its place among the children (`index_in_parent`, 32 bits; the index of
the visual in the readback queue became 32 bits too, and the visual keeps its block of 640
bytes). The table is read from the children whenever the list is (`read_table`), and a
row is written from three places:

- `propagate_flags`, which every change of a visual ends in, marks the row of the visual
  and the row of each ancestor it gives a flag to;
- `post_subgraph` of the update pass clears the row of the visual it leaves and, when it
  computed bounds, writes them;
- nothing else: a write checks that the child is where it says in the list, and a child
  that is not ends the table until the list is read again, so that a table that is wrong
  is not used.

The walker has one more question for its visitor, asked at each step over the children of
a holder with a table: `next_child(holder, from, count)`, the next child to visit.

- The update pass answers with the next marked row (`next_dirty_child`). The bounds of
  the holder, which upstream unites child by child, are united from the rows when the
  holder computes its bounds (`bounds_of_children`): a pass over 32-byte rows that are
  next to each other, not over the visuals.
- The render walk answers with the next row whose bounds meet the clip of the walk
  (`next_child_in`). The clip of the walk is in the space of the world and the rows are in
  the space of the holder, so the clip is mapped into the holder's space, made larger by
  more than the arithmetic of the two directions can differ: the test leaves out only
  children that the exact test of the visual leaves out as well. A holder whose transform
  turns or skews, a render that stops at a visual and a visual drawn without its transform
  ask every child, as before. The drawing items of the holder that come before a child
  that is left out are drawn as they are before one that is visited, and the child counts
  as visited in the debug events.

One thing the exact test does had to be kept. A child with a clip of its own that lies
outside the clip of the walk gets the empty rectangle at the origin of the world as its
clip (`intersect_or_empty`), and bounds that lie around the origin of the world meet that
rectangle: such a child is entered, and what it draws is clipped away. The row test
therefore keeps the children whose bounds lie at or around the origin of the world, and
the exact test decides for them as before. It is one child in most scenes; taking the
visit away would change what the debug events count and is left for a change of its own.

In a debug build the update pass checks, for every child it leaves out, that the visual
has no flag: a flag set by a path that does not end in `propagate_flags` fails the tests
instead of leaving a visual stale.

**Other backends.** All of it is in `ferroui_base`, in the server side of the composition
that every rendering backend runs (Skia, Vello, ProGPU, the browser): no backend has
anything to implement, and the drawing contexts see the same calls in the same order for
the children that are drawn.

**Measured** with the local gate (nine rounds, the two sides alternately, the medians;
base `cc82b74b96`):

| Metric | Before | After | Change |
|---|---:|---:|---:|
| `wall/20` render thread, ms a frame | 4.682 | 1.670 | -64.3 % |
| `wall/20` wall time, ms a frame | 4.685 | 1.672 | -64.3 % |
| `wall/20` UI thread, ms a frame | 0.067 | 0.026 | -61.4 % |
| `wall/20+pointer` render thread, ms a frame | 4.804 | 1.738 | -63.8 % |
| `wall/20+pointer` wall time, ms a frame | 4.798 | 1.855 | -61.3 % |
| `wall/20+pointer` UI thread, ms a frame | 0.872 | 1.163 | +33.5 % |
| `frames/wall` render thread, ms a frame | 2.453 | 1.827 | -25.5 % |
| `frames/recolor` render thread, ms a frame | 2.880 | 2.370 | -17.7 % |
| `frames/pile` render thread, ms a frame | 1.701 | 1.520 | -10.7 % |
| `gauntlet/16` render thread, ms a frame | 34.293 | 34.411 | +0.3 % |
| `gauntlet/16` UI thread, ms a frame | 14.418 | 14.541 | +0.9 % |

The gauntlet has no holder of many child visuals whose children stand still (its rows are
drawing items of their lists), so it is unchanged, as are the other scenes of the gate.

The one metric that rose, the UI thread's time in `wall/20+pointer`, is the hit test of
the pointer, not the tables. With the pointer in the window every frame that was rendered
is followed by two hit tests on the UI thread (pointer-over and the tooltip), and a hit
test brings its index of the holder's children up to date whenever the render thread has
written a newer revision of the readback: a walk over every child of the holder
(`CompositionHitTestAabbTree::update_bucket`). Before, the render thread took 4.7 ms a
frame and the two hit tests of a frame saw one revision, so the second one found its index
up to date; now a frame takes 1.7 ms, the next revision is there between the two hit tests
of nearly half of the frames, and the second hit test walks the children again (in the
profile of the UI thread: 9 samples of 487 in the tooltip's hit test before, 501 of 1660
after). The frame gate runs frames as fast as they render; on a display the two hit tests
of a frame see one revision as before. The walk itself is the cost that should go (a hit
test index that hears which children were written, as the tables hear which children
changed), and that is the change after this one (section 9.49).

**Tests.** `many_children_are_updated_and_rendered_as_without_their_table`: holders of 36
to 400 children, clipped and not, scaled, mirrored and turned, through 160 frames of
random moves, visibility, backgrounds, opacity, added, removed and nested children and
moves of the holder, once with tables and once without (`set_without_children_tables`):
the invalidated rectangles, the visited and rendered visuals, the bounds of the holder and
every drawing call are the same frame by frame, and the run with tables left children out
of both passes. `one_child_that_changes_leaves_the_other_children_out_of_both_passes`: of
200 children, a frame without a change leaves nothing to visit, a change of one child
leaves 199 out of both passes, and a child moved over its neighbour leaves 199 out of the
update pass and 198 out of the render walk. `a_visual_of_the_server_fits_a_block_of_640_bytes`
keeps the size of the visual.

**Gate.** `wall/20` and `wall/20+pointer` of the frame gate (`scripts/perf/gate.py --only
gauntlet`) hold the render thread's time of a holder of 20 000 children with a few that
change.

### 9.49 The hit-test index hears which children the render thread wrote

A holder of 32 children or more answers a hit test from an index of its children
(`CompositionHitTestAabbTree`, as upstream has it: buckets of 32 children in the order of
the children, a tree of bounds in each). The bounds come from the readback of each child,
which the render thread writes and the UI thread reads at a revision (`ReadbackIndices`).
A bucket remembers the revision it was read at, and a hit test at a newer revision read
every child of every bucket it came to before it asked the tree: for a point over the
holder that is every child of the holder. A child whose readback was not written since
returns after comparing two revisions (section 9.6), but the comparison reads the visual,
and the walk is as long as the list of children whatever changed. On the still wall with
the pointer in the window (`wall/20+pointer`: 20 000 items, two hit tests after each
frame) this walk was the UI thread's frame, and it grew when the render thread became
faster (section 9.48).

**The change.** The readback pass says which readbacks it wrote, and the index reads
those.

- `ReadbackIndices` keeps a list of the readbacks written in the latest revisions
  (`Written`: pairs of the revision and the key of the readback, the oldest first). The
  key of a readback is its address: the UI visual and the server visual share one
  readback, and it stays where it is while either of them lives. The pass
  (`ServerCompositor::visual_readback_update_pass`) asks its write scope for the list
  (`ReadbackWriteScope::written`) and adds the key of each readback it writes, visual or
  snapshot, before the revision is published.
- The list is bounded. A pass that writes more than 1 024 readbacks
  (`WRITTEN_IN_A_REVISION`) lists none of them, and the list keeps at most 4 096 keys
  (`WRITTEN_KEPT`), dropping whole revisions from the old end. Both move the floor of the
  list, the revision from which it is complete, and a question about revisions before the
  floor (`written_between`) is answered with "not known".
- The index keeps its entries by the key of the child's readback, the revision it last
  heard at (`heard`) and the set of the children it has not settled: the ones whose
  entry was made without a readback (a child the render thread has not written yet, and
  one that a hit test passes through until it has a readback, section 9.41).
- At the start of a hit test of the holder the index hears (`hear_written`): it asks for
  the readbacks written after `heard` up to the revision of the hit test and reads those
  of them that are its children, and the children that are not settled. When the answer
  was complete and every child was where its entry says, each bucket that was current at
  `heard` is current at the revision of the hit test, without a walk.
- Anything else falls back to the walk, bucket by bucket as before: a list that does not
  reach back to `heard` (a pass that wrote more than the list takes, or a holder that was
  not hit-tested for many frames), a key whose child is no longer at its place, and a
  bucket that was not current at `heard`. A bucket that is made while the index is in use
  starts at `heard`, because each entry that comes to it is read as it comes or is not
  settled.

What a hit test finds is what the walk finds: the walk changes an entry only for a child
whose readback has a newer revision than the entry, a readback gets a newer revision only
in a pass, and every pass since `heard` is in the list or moved its floor. The statement
kept between hit tests is that a bucket whose revision is `heard` has every entry either
read at `heard` or later, or in the set of the children that are not settled.

**Other backends.** The list is in the readback bookkeeping of `ferroui_base` and the
index is on the UI side of the composition: every rendering backend (Skia, Vello, ProGPU,
the browser) runs the same readback pass, and none has anything to implement.

**Measured.** In the tests, the buckets walked by four hit tests a frame through random
frames (a frame in sixteen moves every child, the others add, remove, hide and move a
few), with the list and without:

| Children | Frames | Bucket walks without the list | With the list |
|---:|---:|---:|---:|
| 40 | 80 | 92 | 0 |
| 200 | 120 | 518 | 0 |
| 1 200 | 40 | 950 | 418 |

With 1 200 children the frames that move every child write more than the list takes, and
those are the walks that remain.

With the local gate (the two sides alternately, the medians; base `2cc3671f21`, the
children tables of section 9.48):

| Metric | Before | After | Change |
|---|---:|---:|---:|
| `wall/20+pointer` UI thread, ms a frame | 1.220 | 0.039 | -96.8 % |
| `wall/20+pointer` wall time, ms a frame | 1.902 | 1.633 | -14.1 % |
| `wall/20+pointer` render thread, ms a frame | 1.752 | 1.632 | -6.9 % |
| `frames/wall+pointer` UI thread, ms a frame | 0.092 | 0.028 | -69.1 % |
| `wall/20` UI thread, ms a frame | 0.027 | 0.024 | -9.4 % |
| `wall/20` render thread, ms a frame | 1.659 | 1.626 | -2.0 % |
| `gauntlet/16` UI thread, ms a frame | 14.614 | 14.469 | -1.0 % |
| `gauntlet/16` render thread, ms a frame | 34.331 | 34.166 | -0.5 % |

The UI thread's frame with the pointer in the window is now what it is without the
pointer, to a hundredth of a millisecond: the two hit tests of a frame read the few
children that were written. Against the commit before the children tables the same
metric went from 0.872 ms to 0.039 ms. None of the 121 metrics of the gate is over its
tolerance.

**Found on the way.** A child added to or removed from a holder has the UI side clear the
holder's composition children and add them all again
(`Visual::synchronize_composition_child_visuals`, as upstream does on any difference).
The index is emptied with the list, and the next readback pass writes every sibling: one
child added among 200 lists about 205 readbacks. The index handles it (the children come
back unsettled and are read once), but the cost is that of the whole list for one child,
on both threads. Making the synchronization insert and remove the children that differ
is a change of its own, to be measured on a list that grows and shrinks.

**Tests.** `the_readbacks_of_the_latest_revisions_are_listed`: what the list answers for
ranges of revisions, a pass over the limit, and the oldest revisions leaving at the bound.
`a_hit_test_index_that_hears_the_written_children_finds_what_one_that_reads_every_child_finds`:
40, 200 and 1 200 children through random frames (moves of every child, children added,
removed, hidden and resized, frames without a render), four hit tests a frame, once with
the list and once without (`HEARS_WRITTEN`): the same visuals in the same order every
time, and fewer walks with the list.
`a_child_that_changes_among_many_does_not_have_the_index_read_the_other_children`: of
2 000 children one moves in each of 20 frames; the hit tests find it at its new place and
the last child at its own, and no bucket is walked after the first hit test.

**Gate.** `wall/20+pointer` of the frame gate holds the UI thread's time of the hit tests
over a holder of 20 000 children.
