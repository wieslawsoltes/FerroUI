# Binary size

Issue #306 reported that the release executable of the ControlCatalog on Windows
(`control-catalog-desktop.exe`, built with `cargo build --release`) was 193 MB, bigger than the
gallery of the upstream C# framework, and asked whether Direct3D 12 was the reason for the memory
use. This document records what an executable of FerroUI is made of, what was changed to make it
smaller, the measurements before and after each change, the settings an application should ship
with, and the answer on memory and the graphics backends.

Everything here is measured unless a sentence says otherwise. Sizes are in MB of 1,000,000 bytes.

## 1. Summary

- **Ship with the `dist` profile**: `cargo build --profile dist`. The release profile of the
  workspace is set up for build time (thin LTO in sixteen code generation units,
  `docs/porting/desktop-performance.md`, section 7) and keeps the symbol table; `dist` is fat LTO in
  one unit with the symbols stripped. On macOS that alone makes the catalog of the baseline
  140.1 MB instead of 220.2 MB; with the changes below it is 118.3 MB. On Windows (x86-64, where
  the symbols are in the PDB anyway) the catalog goes from 200.8 MB (release, baseline) to
  149.5 MB (`dist`, this change) (section 3).
- **The catalog compiled and linked three themes.** It starts in the WinUI theme of the port, but
  the Simple and Fluent themes of the original were compiled into it as well (7.9 MB of code in
  the release build on macOS, 26.2 MB of the executable with their unwind tables and symbols).
  They are now behind the Cargo feature `classic-themes`, off by default (section 4.1).
- **The compiled markup of a theme was mostly repeated statement sequences.** The compiler of
  markup now writes the sequences that recur at thousands of sites as calls of shared helpers of
  its runtime library, with the same calls in the same order. The code of the compiled WinUI theme
  went from 12.4 to 6.4 MB (section 4.2).
- **What dominates now** is the framework itself: the object model and the property system
  instantiated per value type (`ferroui_base`, 31 % of the code and data of the catalog and 42 % of
  the gallery), the embedded pictures of the catalog (24 MB, the same pictures the upstream catalog embeds), the unwind tables
  that unwinding needs, and Skia (section 5).
- **A hello world in the WinUI theme** (`samples/HelloWorld`, `dist`) is 71.5 MB on Windows and
  51.1 MB on macOS, against 150.4 and 118.1 MB for the catalog; the style of the API (XAML,
  builders, `rsx!`, hooks) changes it by less than 0.6 MB (section 3.6).
- **Direct3D 12 is not used.** On Windows Skia draws with OpenGL ES through ANGLE, which runs on
  Direct3D 11 (as upstream does), with software rendering as the fallback; ANGLE is linked into the
  executable instead of being shipped as a DLL beside it, and is about 4 % of it (section 7).

No functionality was removed, no executable packer is used, and the panic strategy is unchanged.

## 2. Method

- **macOS**: Apple M3 Pro (11 cores, 18 GB), macOS 26.6, rustc 1.90.0, Xcode 26.4.1. The machine was
  shared with other builds; sizes do not depend on that, start-up times do.
- **Windows**: GitHub's `windows-latest` runner (x86-64, the instruction set of the reporter's
  executable), stable Rust and MSVC of the image, through the workflow `Performance`
  (`.github/workflows/perf.yml`, input `windows`). The owner's Windows virtual machine was not used:
  it is ARM64, and the host was short of memory while other builds ran.
- **What was built**: `control-catalog-desktop` and `winui-theme-gallery` together, in one cargo
  invocation (`-p control-catalog-desktop -p winui-theme-gallery --bin ..`), so that the features of
  their common dependencies are the same in every measurement. Building one of them alone resolves
  other features for the framework crates and gives other numbers.
- **The baseline** is `main` at `58770fcb`. Each change was measured on top of the previous one;
  the first change was also built in a second checkout, whose gallery differs from the baseline by
  16 kB (paths in panic messages), so the two checkouts compare.
- **Attribution**: a script over the symbol table of the unstripped macOS executable (`nm -n -m`:
  the size of a symbol is the distance to the next one in its section; a Rust symbol is attributed
  to the crate named first in its demangled path, `<A as B>::f` to the crate of `A`; C and C++
  symbols are grouped as such; private constants that are not in the symbol table, which hold the
  embedded files, are "anonymous data"). The generic families group the instances of a generic
  function by its path with the type arguments erased. `size -m` gives the sections. Windows
  executables were measured by their file size and section table (the symbols of a Windows
  executable are in its PDB, not in the file).
- **Memory and start-up**: the catalog launched with `FERROUI_SMOKE_EXIT_MS=6000` under
  `/usr/bin/time -l`, three launches per build alternating between builds, home page shown;
  "resident" is the maximum resident set size, "footprint" the peak memory footprint, which on
  macOS includes the memory of the GPU driver.

Every number below comes from this procedure; the commands are in section 9.

## 3. Measurements, before and after

### 3.1 macOS, release profile (what `cargo build --release` gives)

| Build | Catalog | stripped | Gallery | stripped |
|---|---:|---:|---:|---:|
| Baseline (`58770fcb`) | 220.20 | 172.45 | 101.60 | 75.64 |
| The Simple theme out of the catalog (the Fluent theme still linked through the TreeDataGrid demo) | 209.12 | 163.80 | | |
| The Fluent theme out as well (the demo's application behind its feature `app`) | 194.03 | 151.47 | 101.58 | 75.62 |
| Shared helpers in compiled markup, first round | 186.83 | 144.35 | 94.84 | 68.93 |
| `rt::bind_template_binding` | 186.32 | 143.79 | 94.42 | 68.50 |
| **Change** | **-33.88 (-15.4 %)** | **-28.66 (-16.6 %)** | **-7.18 (-7.1 %)** | **-7.14 (-9.4 %)** |

"Stripped" is the file after `strip`, which is what `strip = "symbols"` of a profile gives.

### 3.2 macOS, `dist` profile (what to ship)

| Build | Catalog | Gallery |
|---|---:|---:|
| Baseline, `dist` as it was (fat LTO, one unit, symbols kept) | 168.11 | 76.46 |
| Baseline, the same stripped | 140.07 | 60.36 |
| This change, `dist` (fat LTO, one unit, `strip = "symbols"`) | 118.25 | 54.04 |
| **Change from the baseline release build** (what the report measured) | **-101.95 (-46.3 %)** | **-47.56 (-46.8 %)** |
| **Change from the baseline `dist`, stripped** (the effect of the code changes alone) | **-21.82 (-15.6 %)** | **-6.32 (-10.5 %)** |

### 3.3 Windows (x86-64), release and `dist`

Measured on GitHub's `windows-latest` runner (x86-64, the instruction set of the report) by the
workflow `Performance` with its input `windows` (run 39945714924): each tree and profile built
in a job of its own. The file is what is shipped: a Windows executable holds no symbols (they are
in the PDB beside it), so `strip` changes nothing here.

| Build | Catalog | Gallery |
|---|---:|---:|
| Baseline, release | 200.81 | 92.05 |
| Baseline, `dist` | 184.40 | 85.29 |
| This change, release | 162.14 | 80.14 |
| This change, `dist` | 149.52 | 74.04 |
| **Change, release** | **-38.67 (-19.3 %)** | **-11.91 (-12.9 %)** |
| **Change, from the baseline release build to this change with `dist`** | **-51.29 (-25.5 %)** | **-18.01 (-19.6 %)** |

The reporter's 193 MB was a release build of an earlier commit. The sections of the catalog with
`dist` after the change: code (`.text`) 94.32 MB, read-only data (`.rdata`: the embedded pictures
and fonts, the unwind data `.xdata`, the constant data of Skia and ANGLE) 49.04 MB, function table
(`.pdata`) 4.96 MB, relocations 0.52 MB, data 0.68 MB. The executables are larger than the stripped
macOS ones: ANGLE (6.6 MB, below) is linked into them, x86-64 code is larger than ARM64 code, and
the function table and unwind data of the x86-64 ABI (`.pdata`, `.xdata`) are larger than the
compact unwind tables of macOS.

**By library** (the release executable of the catalog after the change, linked once more with a
map of the linker, `/MAP`; run 39952613814; 155.8 MB attributed to symbols, the largest objects):

| Library or object | MB | Share |
|---|---:|---:|
| The objects of `control_catalog` (its compiled pages and code; the largest object, 27.4 MB, holds its 24 MB of embedded pictures) | 60.8 | 39 % |
| The first object of `ferroui_themes_winui` (the compiled theme) | 10.7 | 7 % |
| Six objects of `ferroui_controls` and `ferroui_base` among the largest (the rest of the framework is in objects below 1.7 MB each) | 12.7 | 8 % |
| ANGLE: `libGLESv2` (OpenGL ES on Direct3D 11) and `libmozangle` (EGL and the shader translator) | 6.6 | 4 % |
| Skia | 5.1 | 3 % |

With thin LTO the Rust code stays in the objects of its crate, sixteen per crate, so the framework
is spread over many objects smaller than the ones listed; the workflow now groups them by crate.
What the table shows for the question of the report: the graphics stack of Windows (ANGLE with
its Direct3D 11 renderer) is about 4 % of the executable and Skia 3 %; the rest is the framework,
the compiled markup and the pictures of the catalog.

### 3.4 Code of the themes (release, macOS, from the symbol table)

| Crate | Baseline | After |
|---|---:|---:|
| `ferroui_themes_winui` (compiled markup and the code of the theme) | 12.44 | 6.36 |
| `ferroui_themes_fluent` (in the catalog) | 5.42 | not linked |
| `ferroui_themes_simple` (in the catalog) | 2.48 | not linked |
| `control_catalog` (its compiled pages and its code) | 11.13 | 10.85 |

The generated Rust of the WinUI theme went from 17.2 to 12.2 MB, of the Fluent theme from 7.3 to
5.5 MB and of the Simple theme from 3.7 to 3.2 MB.

### 3.5 Start-up and the macOS runner of CI

The workflow `Performance` (`.github/workflows/perf.yml`, run 39936080676) built the baseline and
this change on a GitHub `macos-15` runner (Apple M1, 3 processors) with the release profile, each
application by itself, and launched the two builds alternately, seven warm launches each, load
average 6 to 10:

| Application | Size before (stripped) | after | Start-up before, median (range) | after |
|---|---:|---:|---:|---:|
| control-catalog-desktop | 171.45 | 142.85 | 1012 ms (830 to 1152) | 1061 ms (829 to 1198) |
| themed_window (Simple) | 73.89 | 70.59 | 162 ms (122 to 289) | 170 ms (129 to 339) |
| hello_window | 52.17 | 52.17 | 139 ms (127 to 173) | 162 ms (143 to 197) |

The start-up times are the same within the spread of the launches (hello_window, which no change
touches, moved as much as the others). The helpers of compiled markup are calls where the
statements were calls too; the theme is populated as fast as before.

### 3.6 Hello, World!: the floor of an application, against the catalog

The samples `samples/HelloWorld` (one window with "Hello, World!" in the WinUI 3 theme, in XAML)
and its versions in each style of the declarative API (`samples/HelloWorld.Builders`, `.Macros`,
`.Hooks`, `.ComponentHost`; docs/declarative-ui.md) depend only on what they use, so they show
what an application in the WinUI theme costs before it adds anything of its own. `hello_window`
(`ferroui-desktop`'s example: a window without a theme and without markup) is the floor of the
framework itself. Each is built by itself, as an application is.

Windows x86-64 (run 40002097460 of the workflow `Performance`):

| Application | Release | `dist` |
|---|---:|---:|
| `hello_window` (no theme, no markup) | 34.58 | 32.62 |
| `hello-world` (XAML, WinUI theme) | 77.14 | 71.50 |
| `hello-world-builders` | 77.54 | 71.79 |
| `hello-world-macros` | 77.54 | 71.79 |
| `hello-world-hooks` | 77.54 | 71.79 |
| `hello-world-component-host` (XAML with a component) | 77.69 | 71.94 |
| `winui-theme-gallery` | 80.14 | 74.04 |
| `control-catalog-desktop` | 163.08 | 150.40 |

macOS (GitHub's `macos-15` runner, Apple M1; runs 40034843569 and 40040788406; as written and
stripped; `dist` strips the symbols itself):

| Application | Release | stripped | `dist` |
|---|---:|---:|---:|
| `hello_window` (no theme, no markup) | 88.03 | 52.15 | 41.26 |
| `hello-world` (XAML, WinUI theme) | 105.71 | 64.87 | 51.09 |
| `hello-world-builders` | 106.11 | 65.22 | 51.29 |
| `hello-world-macros` | 106.11 | 65.22 | 51.29 |
| `hello-world-hooks` | 106.11 | 65.22 | 51.29 |
| `hello-world-component-host` (XAML with a component) | 106.40 | 65.37 | 51.42 |
| `winui-theme-gallery` | 110.29 | 67.87 | 53.28 |
| `control-catalog-desktop` | 215.50 | 143.67 | 118.06 |

By crate (macOS, release, `scripts/symsize.py`; code and named data, MB):

| Crate | `hello_window` | `hello-world` | `hello-world-builders` | Catalog |
|---|---:|---:|---:|---:|
| `ferroui_base` | 23.39 | 26.94 | 27.02 | 45.56 |
| C, C++ and Objective-C (Skia, HarfBuzz, the native backend) | 7.26 | 8.13 | 8.15 | 11.66 |
| `ferroui_themes_winui` | | 5.93 | 5.93 | 5.93 |
| `ferroui_controls` | 4.02 | 4.15 | 4.16 | 4.91 |
| `core`, `alloc`, `std`, `hashbrown` | 7.71 | 8.52 | 8.62 | 14.81 |
| Embedded files and other private constants | 2.81 | 3.16 | 3.15 | 29.83 |
| `ferroui_markup_xaml` | 0.32 | 0.43 | 0.42 | 1.45 |
| `ferroui_view` | | | 0.08 | |
| `control_catalog`, the TreeDataGrid with its demo, the colour picker | | | | 15.78 |

What the numbers say:

- **The style of the API costs almost nothing.** The four declarative versions are within 0.6 MB
  of the XAML one on both systems; `ferroui-view` itself is 0.08 MB of code on macOS.
- **The theme is most of an application on Windows, less on macOS.** On Windows the WinUI theme
  takes a window from 32.6 to 71.5 MB (`dist`): the theme has a control theme for every control,
  so every control is linked with it, and its compiled markup (15.6 MB of the Windows catalog).
  On macOS the theme-less window is larger already: it links the controls through the native
  backend (4.0 MB of `ferroui_controls` in `hello_window`, as much as in `hello-world`), so the
  theme adds 9.8 MB (`dist`, 41.3 to 51.1 MB), most of it its compiled markup (5.9 MB of code). The upstream framework
  is built the same way: a theme is a resource dictionary of every control.
- **What the catalog adds over a hello world** (`dist`, Windows: 150.4 against 71.5 MB, 78.9 MB):
  its 219 compiled pages and their code, its 24 MB of embedded pictures, the TreeDataGrid with its
  demo, the colour picker, the dialogs, and the VNC and headless hosts of its desktop entry point.
  The per-crate attribution of the catalog (Windows, release, the linker map of run 40002097460):
  `control_catalog` 62.9 MB (with the pictures), `ferroui_controls` 22.1, `ferroui_base` 18.4,
  `ferroui_themes_winui` 15.6, `treedatagrid_demo` 7.4, the colour picker 5.2, Skia 5.1, the
  TreeDataGrid 5.0, ANGLE 6.6 (`libGLESv2` 4.6, `libmozangle` 2.0), the runtime library of
  markup 2.0, the dialogs 1.9, the Windows backend 1.4, HarfBuzz 0.8.

## 4. The changes

### 4.1 The themes of the catalog

The catalog of the port starts in its WinUI theme, which styles every control
(`docs/porting/winui-theme.md`), and also declared the two themes of the original in `App.xaml`.
Every declared theme was compiled into the catalog. As the owner asked, the catalog is now built
with the WinUI theme alone:

- `ClassicThemes.xaml`, a resource dictionary class of the catalog, holds what `App.xaml` of the
  original declares for its two themes (the themes, the Simple styles of the colour picker, the
  generic styles of the TreeDataGrid). The build compiles it, and the application merges it into
  its resources, only with the feature `classic-themes` of `control-catalog`, which
  `control-catalog-desktop` forwards: `cargo run -p control-catalog-desktop --features
  classic-themes`.
- The settings page lists `App.AvailableThemes`, the themes the catalog is built with.
  `FERROUI_CATALOG_THEME=fluent` (or `simple`) needs the feature and says so without it.
- The TreeDataGrid demo, whose views the catalog shows on its TreeDataGrid page, had its own
  application in its library: `App.xaml` with the Fluent theme, which linked the theme into the
  catalog after the catalog's own declaration was gone (the second row of 3.1). The application,
  the main window and the entry point of the demo are now behind its default feature `app`; the
  catalog depends on the demo without default features.
- The two documents are still the output of `scripts/convert_catalog_xaml.py` (its `ADDITIONS`
  and the new `REMOVALS`), so `scripts/sync-control-catalog.sh --check` holds them to upstream.
  `samples/ControlCatalog/GAPS.md` and `docs/porting/DEVIATIONS.md` record the change.

Note that `cargo build --workspace` builds the TreeDataGrid demo with its default features, and
cargo then links the Fluent theme into the catalog through it; build the catalog by its package.

### 4.2 Compiled markup: shared helpers

The compiler of markup (`docs/porting/xaml.md`, section 9) writes Rust in the shape of upstream's
compiled IL: a statement per member call, with the error of each fallible call carried to the
caller. A theme is mostly resources and control themes, so the same few sequences recur thousands
of times, and each site carries the code of its calls, of the strings it creates and of the clean-up
of its locals on an error. The WinUI theme had 5,850 functions of deferred content, 3,435 of which
were one `{StaticResource}`, and 5,639 additions of deferred resources.

The emitter already wrote setters of styles as calls of shared helpers
(`docs/porting/xaml.md`, 9.3.7). It now does so for six more sequences; each helper is
non-generic and `#[inline(never)]` and makes the calls the statements made, through the same typed
functions, in the same order:

| Helper | Replaces | Sites in the WinUI theme |
|---|---|---:|
| `rt::string_object(text)` | `rt::to_object(String::from(text))` | 2,969 left after the fusions below (15,653 before) |
| `rt::static_resource_content` | the body of deferred content that is `{StaticResource key}` alone | 3,435 |
| `rt::add_deferred_resource` | `rt::defer(..)` and `ResourceDictionary::__markup_AddDeferred_1(..)` with a text key | 5,554 |
| `rt::bind_dynamic_resource` | `{DynamicResource key}` created, provided and bound to a property | 869 |
| `rt::add_dynamic_resource_setter_with_parent` | a setter whose value is `{DynamicResource key}` | 1,675 |
| `rt::bind_template_binding` | `{TemplateBinding source}` created, provided and bound to a property | 1,400 |

Where the emitter does not recognise the exact shape, it writes the statements as before. The
checked-in `compiled_xaml.rs` of the three themes and the emitter output of the markup tests were
regenerated; the tests that compare every compiled document with the same document loaded by the
run-time loader (the trees and the calls of handlers) pass unchanged, in the themes, the catalog,
the TreeDataGrid demo, the colour picker, the TreeDataGrid, the dialogs and the gallery.

### 4.3 The `dist` profile strips the symbols

`[profile.dist]` gains `strip = "symbols"`. The symbol table is 17 % (fat LTO) to 27 % (thin LTO)
of a macOS executable (sections 3 and 5.1) and nothing of the program reads it except the symbolisation of a panic backtrace,
which then shows addresses. On Windows the setting changes nothing: the executable never holds the
symbols, the linker writes them into the PDB beside it. Build with the release profile for a
backtrace with names. The release profile is unchanged.

## 5. What an executable is made of

### 5.1 Sections (macOS, release, unstripped)

| Section | Catalog, baseline | Catalog, after | Gallery, baseline | Gallery, after |
|---|---:|---:|---:|---:|
| Machine code (`__text`) | 110.56 | 87.49 | 55.47 | 49.38 |
| Read-only data (`__TEXT,__const`: embedded files, tables) | 26.90 | 26.72 | 1.78 | 1.78 |
| Unwind tables (`__eh_frame`, `__unwind_info`) and landing pads (`__gcc_except_tab`) | 25.65 | 20.97 | 13.00 | 12.02 |
| Other data (`__DATA_CONST`, `__DATA`) | 5.29 | 4.86 | 2.93 | 2.93 |
| Symbol table and strings (`__LINKEDIT`; removed by stripping) | 51.46 | 45.94 | 28.16 | 28.05 |

### 5.2 By origin (macOS, release, after the change; code and named data)

| Origin | Catalog | Gallery |
|---|---:|---:|
| `ferroui_base` (object model, property system, bindings, styling, layout, compositor, text) | 41.39 | 25.63 |
| Embedded files and other private constants (the 24 MB of pictures and fonts of the catalog) | 30.00 | 3.49 |
| `core`, `alloc`, `std`, `hashbrown` (instances for the types of the framework: closures, drop glue, collections) | 19.64 | 11.86 |
| C, C++ and Objective-C (Skia, HarfBuzz, the native macOS backend) | 11.64 | 8.37 |
| `control_catalog` (its compiled pages and its code) | 10.85 | |
| `ferroui_themes_winui` | 6.36 | 6.36 |
| `ferroui_controls` | 4.07 | 3.45 |
| The TreeDataGrid, its demo and the colour picker | 4.16 | |
| `ferroui_markup_xaml` (the runtime library of compiled markup) | 1.22 | 0.33 |

### 5.3 What dominates, and why

- **The property system, instantiated per value type.** The largest generic families are the
  typed entries of the property store: `BindingEntry<T>::set_value` (771 instances in the catalog of
  the baseline, 1.7 MB), `EffectiveValue<T>` (coerce, set, dispose; 771 instances each, 0.8 to 1.0 MB each),
  `StyledProperty<T>` routes, `ValueStore` methods, and the closures of the value type registry and
  the class vtables (`FnOnce::call_once`, 27,607 instances, 3.2 MB). Each distinct type of a
  registered property instantiates the whole typed path. Fat LTO merges identical instances across
  crates (the `dist` profile), but the instances for distinct types remain. Making them smaller
  needs a type-erased property store, which `docs/porting/desktop-performance.md` (section 4.2,
  items 10 and 11) proposes; it is a redesign of the hot path, not a setting.
- **Embedded files.** The catalog embeds its pictures (23 MB, mostly JPEG photographs), as the
  upstream catalog embeds them as resources of its assembly, and the Inter fonts (1.9 MB). They
  are already compressed; the browser host of the catalog fetches them as files instead
  (`separate-assets`, `docs/porting/browser-platform.md`, section 18).
- **Unwind tables.** The framework relies on unwinding (`catch_unwind` in the render loop and the
  dispatcher, panics of dispatcher operations re-raised in their awaiters), so `panic = "abort"` is
  not an option; the tables are about a sixth of the sections that load.
- **Skia.** The C++ of Skia, HarfBuzz and the native backend is 8 to 12 MB; the linker drops what
  is not used (no PDF back end is linked; the JPEG and PNG encoders are, because bitmaps can be
  saved).

## 6. The settings applications should use

- `cargo run --profile fast-release -p <application>` to try an application with optimised code:
  the release optimisation level without link-time optimisation, so the final link takes seconds
  instead of minutes. Its executables are somewhat larger and slower than release; benchmarks use
  release, and shipping uses `dist`.
- `cargo build --profile dist` for what you ship (the workspace defines it; an application crate
  outside the workspace copies it into its own manifest):

  ```toml
  [profile.dist]
  inherits = "release"
  lto = "fat"
  codegen-units = 1
  strip = "symbols"
  ```

  The build takes about four times as long as the release profile and its link step needs about
  6 GB of memory. Keep the unstripped executable of a release if you need symbolised backtraces
  from the field (on Windows keep the PDB).
- Depend only on the themes the application uses. A theme declared in a document, or registered in
  `register_types()`, is linked; a theme that is not named is not.
- Leave the panic strategy alone: the framework catches panics in its render loop and dispatcher.
- `opt-level = "s"` for the whole program makes it about 15 % smaller still and costs about 19 % of
  processor time in layout and rendering (`docs/porting/desktop-performance.md`, section 4.1); it is
  not set by default. The browser profile uses it, with level 3 for the hot crates, because a
  WebAssembly module is downloaded.
- An application that has its pictures and fonts on disk instead of in the executable can register
  them with the asset loader at start (as the browser host of the catalog does).

## 7. Memory, and Direct3D 12 versus the other backends

**No Direct3D 12.** The Windows backend renders through ANGLE (OpenGL ES on Direct3D 11) by
default, with software rendering as the fallback, the order upstream uses
(`Win32RenderingMode::AngleEgl`, `Software`; WGL and Vulkan on request). ANGLE comes from the crate
`mozangle`, which builds it from source and links it statically, so it is inside the executable,
where upstream ships it as a DLL beside its assemblies; it is 6.6 MB of the 162 MB release
executable of the catalog (section 3.3). Skia uses its Ganesh backend on that OpenGL context with the GPU resource cache limited to 29.5 MB (`1024 * 600 * 4 * 12`
bytes), the limit upstream sets.

**Memory, measured on macOS** (the catalog showing its home page, three launches each, median;
not measured on Windows, where the hosted runners have no GPU and the virtual machine was not
used):

| Build, rendering | Resident | Peak footprint |
|---|---:|---:|
| Baseline, Metal (Skia Graphite) | 197 MB | 355 MB |
| After, Metal (Skia Graphite) | 192 MB | 357 MB |
| After, software rendering | 229 MB | 146 MB |

Removing the two themes did not change the memory measurably: the original creates both themes at
start too, and their control themes are deferred until a control asks for them, so they cost
little memory. The GPU path adds about 210 MB of footprint over software rendering: the drawables
of the window, the pipelines and the caches of Skia and of the Metal driver, which the footprint
counts and the resident size does not. The resident size is about 40 MB lower with the GPU than
with software rendering, which keeps a full-size framebuffer per window in process memory.

What makes the process larger than a gallery built on a smaller toolkit is the same as for the
executable: the framework is a port of a retained-mode, styled, data-bound object model (every
control has a property store, a style cascade and a template of controls), with Skia as its
renderer and its caches. Direct3D 12 plays no part in it.

## 8. Not done, and why

- **`panic = "abort"`**: the framework depends on unwinding (section 5.3).
- **`opt-level = "s"` or `"z"` by default**: smaller, but slower in layout and rendering (section 6).
- **Executable packers or compressed embedded files**: excluded by the owner; the pictures are
  already compressed.
- **A Skia without features**: the prebuilt Skia binaries come with a fixed feature set, and any
  other set builds Skia from source; the linker already drops the unused back ends.
- **Further fusions in the compiler of markup**: the next candidates are the property assignments
  whose setter is chosen at run time (about 400 chains in the WinUI theme) and the brushes of
  resources made of one `{DynamicResource}` (704); each saves less than the rounds above.
- **A type-erased property store**: the largest remaining item, a redesign (section 5.3).

## 9. Commands

```sh
# Both executables, in one invocation, on the external volume
CARGO_TARGET_DIR=<dir> cargo build --release -j 4 -p control-catalog-desktop -p winui-theme-gallery \
    --bin control-catalog-desktop --bin winui-theme-gallery
CARGO_TARGET_DIR=<dir> cargo build --profile dist -j 4 -p control-catalog-desktop -p winui-theme-gallery \
    --bin control-catalog-desktop --bin winui-theme-gallery
# Sections and the stripped size
size -m <dir>/release/control-catalog-desktop
strip -o control-catalog-desktop.stripped <dir>/release/control-catalog-desktop
# Attribution by crate and by generic family, from the symbol table (an unstripped build)
python3 scripts/symsize.py <dir>/release/control-catalog-desktop -n 30 --crates 30 --families 40
# The same on GitHub's runners: the workflow Performance, inputs `samples` (macOS) and `windows`
gh workflow run perf.yml -f ref=<branch> -f samples=true -f windows=true
# Memory and start-up
FERROUI_SMOKE_EXIT_MS=6000 /usr/bin/time -l <dir>/release/control-catalog-desktop
FERROUI_SMOKE_EXIT_MS=6000 FERROUI_SMOKE_RENDERING=software /usr/bin/time -l <dir>/release/control-catalog-desktop
# The catalog with the themes of the original
cargo run -p control-catalog-desktop --features classic-themes
```
