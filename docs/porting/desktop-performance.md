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
