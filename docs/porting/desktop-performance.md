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
beside the other frameworks. `PERF_REPORT=1` prints the frame rate the app's animation-frame
callback sees, for A/B runs of FerroUI builds:

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
with a target folder each, then runs them alternately, base then candidate, for `--rounds` rounds
(default 3), and compares the medians of the rounds. A metric over its tolerance by less than the
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
  sections of chapter 9 it guards: `text` (text that changes every frame), `list` (a scrolling list
  of shadowed cards), `transforms` (panels moved by their render transform and opacity), `width` (a
  layout under a changing width), `wall` and `wall+pointer` (a still wall), `moves` (tiles of a
  canvas that move), `lines` (charts whose lines change every frame). The harness is the frame gate's; besides the times a frame, every scene
  reports the time it takes to build (`build_ms`) and the window to show it (`show_ms`: its first
  layout, recording and commit) and their sum (`startup_ms`). Alone:

  ```sh
  cargo test -p ferroui-frame-benchmarks --release --test frames -- --ignored --nocapture --test-threads=1
  ```
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

Tolerances. A metric regresses when the candidate is slower than the base by more than
`--tolerance` (default 5 %) times the factor of the metric, and by more than a floor below which
the measurement resolves nothing (0.05 ms a frame, 2 ns an operation):

| Metric | Factor | Why |
|---|---:|---|
| UI thread and wall time of a frame | 1 | hundreds of frames of a deterministic CPU workload |
| Render thread time of a frame | 1.5 | it runs beside the UI thread and moves with core placement and glyph cache state |
| TreeDataGrid and tree benchmark operation | 2 (3 under 100 ns) | short iterations in a new process per run; heap and code layout move a benchmark by several percent, and the shortest operations are near the timer |
| Build and show of a scene (`build_ms`, `show_ms`) | 1.5 | measured once a run: one sample a round (floor 0.5 ms) |
| Start of a scene (`startup_ms`) | 1 | the sum of the two (floor 0.5 ms) |
| Window: first frame and time a frame | 2 | a real process: the display, the system compositor, the disk and the shader caches (floors 20 ms and 0.5 ms) |

A regression the gate reports after its extra rounds is a real one; a change that is meant to
cost time says so in the pull request, with the table.

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
`long_texts_nothing_holds_are_let_go_by_their_length`,
`a_change_of_the_fonts_lets_every_layout_go`) and `text_block_tests.rs`
(`text_blocks_that_show_the_same_text_the_same_way_share_a_layout`,
`a_text_that_comes_back_has_the_layout_it_had`,
`a_shared_layout_outlives_a_block_that_lets_it_go`,
`a_layout_made_of_more_than_a_plain_text_is_not_shared`). The gate measures it with
`text::TextBlockChangingTextBenchmark` (`change_and_measure`: a text block whose text changes
and is measured, with texts that repeat and with texts that are new every time; the second is
the cost the cache adds to a layout it cannot find).

Counters (`--features counters`): `text layout cache hits`, `text layout cache misses` and
`text layout cache evictions`, beside `text layouts created`.

Gauntlet (tier 16), Skia window, per render pass: text layouts created 796 to 450 (346 found,
446 made and kept, 418 let go: the cache is full and turns over, as the table of this section
says it would at 4,096), text runs shaped 768 to 427, text lines formatted 856 to 507.

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
