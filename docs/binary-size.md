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

- **Finding what roots code**: why the linker keeps a function or a static. A fat-LTO build that
  also emits IR (`cargo rustc --release -p hello-world --bin hello-world -- -C lto=fat
  --emit=llvm-ir,link`) gives the whole program as one module,
  `<target>/release/deps/hello_world-<hash>.ll`. `scripts/irpaths.py <module.ll> <regex>...` builds
  the graph from every function and global to the globals it names, searches it breadth-first
  from `main` and the constructor lists, and prints one shortest chain to each symbol that
  matches, for example `'NavigationView\$u20\$as.*StaticType.*TYPE'` for the `TypeInfo` of a
  control. Apple's linker ignores `-why_live`, and the chains often run through anonymous tables
  that a linker reports only by address.
  `scripts/ircut.py <module.ll> <binary> ...` answers the next question on the same graph, before
  anything is changed: what the executable loses when a reference goes. With patterns it gives the
  bytes reachable only through the symbols that match (a family of classes, say); `--classes`
  lists every class by what its `TYPE` alone keeps; `--referrers` lists where a family is entered
  from outside; `--edges <from> <to>` gives the effect of removing the references between two
  sets of symbols and nothing else, which is what one change in the source does. A class that
  several things name keeps little by itself, and the sum over its family is what counts: cut the
  family first, then find its referrers, then try the edges (section 4.20).

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

### 4.4 Property core

The property system was compiled once per value type of a registered property: in the hello
world, `ferroui_base` is 28.85 MB of 71 MB, and about twenty functions of the store exist 438 times
each (#565). The typed facade (`StyledProperty<T>`, `DirectProperty<T>`, the typed reads and
writes) and the typed storage are unchanged; what does not depend on the value type moved into
shared code, and what only some properties need is linked only for them. The mechanisms are
those of the unmerged #496 that need no binary markup, re-applied to the current sources:

- **Shared state over typed frames.** The priorities, value entries and flags of an effective
  value (`EffectiveValueState`), the subscription phase and validation state of a binding entry
  (`BindingEntryState`) and of the local binding observers (`LocalBindingState`) are non-generic,
  with their transitions; the typed parts keep the values, the caches and the typed observers.
  Inherited-change traversal, unset notification and untyped value conversion are compiled once.
- **Demand-rooted algorithms.** The two-value recoercion, the coercion of the default value and
  at the end of a reevaluation, and the uncoerced-value fields are reached through one dispatch
  pointer stored with a registered coercer (`Coercion<T>`, borrowed request variants); the
  inherited-change notification through a callback stored on the property by an inheriting
  registration. A value type whose properties never coerce or inherit does not retain them.
- **Installation by overload.** Typed, binding-value and boxed sources choose their observer
  where they are bound; boxed sources are installed by one non-generic `add_boxed_binding`. In
  #565 a boxed source still delivered through the typed `BindingEntry<T>` cache and a typed
  observer (a fully shared observer measured 7 to 21 % slower boxed style delivery in #496 and was
  not taken then); #576 erased that path (section 4.10).
- **Cold construction.** Property identity, registration per owner type, direct property
  construction, class vtable initialisation, metadata host lookup and the nullable operations of
  the value type registry (one typed operation per nullable type) are shared non-generic paths.
- **Metadata invokers.** Generated invokers share argument counting, receiver preparation (shared
  handles are borrowed) and the slow tail of argument conversion.

Behaviour is unchanged: precedence, notification and cleanup order, coercion and validation, and
panic safety, which the regressions ported with the mechanisms check against the original
algorithms under injected panics and reentrant reads and writes. One corner case changes: when
the metadata of a type is unregistered, a derived type that had cached it now resolves its
metadata again instead of keeping the stale entry.

### 4.5 Handle conversions derived, not registered (#566, part 1)

Every class registered two conversions of the root handle with the value type registry, to
`Ref<T>` and to `Option<Ref<T>>`: two closures and their vtables and drop glue per class, in
every application whether it converts handles or not. The registry already has the object entry
of each class (its `from_object` function), so it derives both conversions when they are looked
up (`Conversion::find` in `data/core/value_type.rs`). Behaviour is unchanged: the conversion is
found and refused exactly as before.

| `samples/HelloWorld`, `dist`, macOS ARM64 | main | after | Change |
|---|---:|---:|---:|
| Executable | 51.26 MB | 50.71 MB | -0.55 MB (-1.1 %) |

The rest of #566 makes the tables of types that every crate registers at start-up (6.5 MB of hello
world, measured by building it without them) registered only by what looks types up by name
(section 4.6).

### 4.6 Trimming the tables of types (#566)

Every crate's `register_types()` registers its table of classes, its markup assembly and its markup
types at start-up. The tables reference every class with its constructor, properties and markup
accessors, so the linker kept all of them. By default the tables are now trimmed: an application
that loads only compiled markup does not look types up by name, and does not register them.

A crate of the framework registers its tables with the trimmable forms `TypeInfo::register_table`,
`MarkupType::register_table` and `MarkupAssembly::register_table`. Without the `ferroui-base`
feature `type-tables` these are inlined no-ops, and link-time optimisation drops what only the
tables reach. A type is still registered when it is used:
- a class when it is initialised, with its markup metadata (`TypeInfo::markup`);
- the metadata of the value type of a property (`T` or `Option<T>` with metadata) when the property
  is registered (`MarkupType::register_used`), so enumeration members convert to text and text
  parses to value types;
- the metadata of the value types of the properties of a registered type
  (`MarkupProperty::value_markup`), which a binding through a view-model property finds by handle;
- the classes named by the reflection-binding paths of a compiled document (`$parent[TextBox]`,
  `(DataValidationErrors.HasErrors)`), which the compiler resolves into typed references in the
  namespace table of the document.

The plain functions `TypeInfo::register_all`, `MarkupType::register`, `MarkupType::register_all`
and `MarkupAssembly::register` always register, whatever the features say. An application
registers its own types (its view models, its assembly) with them from its `register_types()`, so
its reflection bindings find its own properties with no change.

What looks types up by name enables the feature `type-tables`, which keeps the tables. These are:
- the run-time XAML loader, the designer and `ferroui-designer-support`;
- diagnostics, DevTools, hot reload and Inspect;
- the declarative View API, which parses selector strings;
- the behaviors' member lookup.

Feature unification gives the tables to every application that links one of them. An application
that looks types up by name itself enables the feature:

```toml
ferroui-base = { workspace = true, features = ["type-tables"] }
```

| `samples/HelloWorld`, `dist`, macOS ARM64 | Size | Start-up (median of 9) |
|---|---:|---:|
| with the tables (before #566) | 50.78 MB | 214 ms |
| trimmed, opt-in (#582) | 44.36 MB | 215 ms |
| trimmed, with what trimmed builds need (#583, the default) | 45.28 MB (**-5.50 MB, -10.8 %**) | |

The default was flipped after a run of 24 compiled desktop samples without the tables: with the
fixes of #583, each logs exactly what it logs with the tables (`FERROUI_SMOKE_EXIT_MS`, binding
diagnostics compared sample for sample).

### 4.7 Unwinding (#567)

Every function that owns a value with a destructor has a landing pad and unwind records, so a panic
can run the destructors on its way up. In hello world's `dist` build these are 6.63 MB of unwind
sections (`__eh_frame` 3.67 MB, `__gcc_except_tab` 2.23 MB, `__unwind_info` 0.73 MB) and about
3.1 MB of landing-pad code in `__text`.

Panics used as exceptions are not what keeps them. `ferroui-base` has 30 sites of
`catch_unwind`, `resume_unwind` and `panic_any` outside tests, and every one is a boundary:
the dispatcher loop and its jobs, the render loop, the isolation of user handlers, or the rethrow
of a caught panic after cleanup (the try/finally of the original). The classification is in
[porting/panics-as-exceptions.md](porting/panics-as-exceptions.md). Turning panics into `Result`
removes few of these records. The cost comes from unwinding itself.

The shipped profile `dist` builds without unwinding (`panic = "abort"`, since #567; before, this
was the opt-in profile `dist-abort`):

| `samples/HelloWorld`, macOS ARM64, tables of types trimmed | with unwinding | `dist` (abort) |
|---|---:|---:|
| executable | 45.28 MB | 35.71 MB (**-9.57 MB, -21 %**) |
| `__text` | 34.48 MB | 31.35 MB |
| unwind sections | 6.63 MB | 0.26 MB |

The desktop control catalog (`samples/ControlCatalog.Desktop`, macOS ARM64, with its images and
fonts) is 98.9 MB built this way.

What changes without unwinding: a panic ends the process at once, after the panic hook has run,
without running destructors.
- No production path carries an expected error as a panic: the markup error scope, the page
  transitions, OpenGL loading, making a GL context current, UI Automation refusals and the Vello
  inputs the third-party sweep refuses return errors (the classification of every site is in
  [porting/panics-as-exceptions.md](porting/panics-as-exceptions.md)).
- What a catch of a panic still isolates is a bug: the dispatcher no longer isolates a panicking
  job or event handler, and the render loop no longer survives a panicking frame. With unwinding
  the panic is caught, the frame is cleaned up and the panic is raised again on the dispatcher, the
  way the original rethrows an exception of a handler.
- The development tools that isolate the code of the application they inspect (the Designer, the
  previewer of `DesignerSupport`, hot reload, DevTools) need unwinding for that. They are built
  with `dev`, `fast-release`, `release-unwind` or `dist-unwind`. The errors of the edited
  document no longer travel as panics there: a reload step, the previewer's load and show and
  the Designer's stage layouts run in the error scope of the XAML runtime, and the animation
  preview rejects a program with `Animation::try_apply_for_preview` (#567). The (E) sites left,
  each with its reason, are listed in section 2 of the classification.

`release` aborts on panic as well (owner decision of 2026-11-07, #567): it is what
`cargo build --release` gives an application, and the unwind tables are as large there as in
`dist`. `release-unwind` and `dist-unwind` are the same profiles with unwinding, for a program that
catches panics to carry on. `fast-release`, the profile for running an application or a tool
locally, keeps unwinding so the Designer and the previewer isolate the code they preview. The
browser profile inherits `release` and aborts too; the threaded module rebuilds the standard library
with `panic_abort` (`scripts/build-browser.sh`). Cargo ignores `panic` for tests, benches, build
scripts and procedural macros, so the tests and benchmarks are unchanged.

| `samples/HelloWorld`, macOS ARM64, `release` | with unwinding | `release` (abort) |
|---|---:|---:|
| executable | 48.92 MB | 38.66 MB (**-10.26 MB, -21 %**) |
| `__text` | 23.67 MB | 20.48 MB |
| unwind sections (`__eh_frame`, `__gcc_except_tab`, `__unwind_info`) | 5.57 MB | 0.19 MB |

The abort build ran its smoke run (`FERROUI_SMOKE_EXIT_MS=5000`) to a clean exit.

Workspace profiles do not reach an application in a workspace of its own: its root `Cargo.toml`
states its own. The application templates (the `FerroUI/templates` repository) and the shared
workspace settings that `scripts/apps/sync-workspace.py` writes carry `panic = "abort"` in their
`release` and `dist` profiles. An application that wants a report of a panic before the process ends installs a
panic hook (`std::panic::set_hook`), which runs before the abort, as the Android and browser
platforms do for their crash reports.

What is left of the unwind sections after `panic = "abort"` (0.26 MB of hello world's `dist`
build above) was checked against `-C force-unwind-tables=no` (#567, 2026-11-19), and the flag
is left out:

- rustc already leaves the `uwtable` attribute off every function of the workspace with
  `panic = "abort"`, unless the target sets `default-uwtable` or `requires-uwtable`
  (`rustc --print target-spec-json`). Without either: `aarch64-apple-darwin`,
  `x86_64-apple-darwin`, `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`. The flag gives
  these targets nothing they do not have already.
- On macOS the code generator still writes the call frame information of every function. The
  linker builds `__unwind_info` from it, and the system unwinder and the crash reporter read it.
  A test program built for `aarch64-apple-darwin` with `panic = "abort"` had the same number of
  `.cfi_startproc` in its assembly, the same section sizes (`__unwind_info` 4,992 bytes,
  `__eh_frame` 27,044 bytes) and the same file size, with and without the flag.
- `x86_64-pc-windows-msvc` and `aarch64-pc-windows-msvc` require unwind tables
  (`requires-uwtable`): the `.pdata` and `.xdata` records are what structured exception handling
  and every stack walk on Windows use. rustc refuses `-C force-unwind-tables=no` there.
- `aarch64-linux-android` sets `default-uwtable`: there the flag would remove the tables of the
  workspace's code. The platform's crash dumps (`debuggerd`) use them to unwind, so they stay.
- The unwind records that remain come from the precompiled standard library, which is built
  with unwinding, and from the C and C++ libraries (Skia, HarfBuzz). Only `-Zbuild-std` (nightly)
  rebuilds the standard library without them, and FerroUI builds on stable. The C++ libraries
  are compiled by their own build scripts.
- A profile cannot set rustflags on stable (`profile-rustflags` is unstable).
  `[target.<triple>] rustflags` in `.cargo/config.toml` reach every profile, the tests and the
  build scripts too, and an application in a workspace of its own does not read the
  framework's configuration.

### 4.8 Class tables are constants (#569)

A class's table of virtual members was built at run time by `Subclassable::build_vtable`, a
function per pair of a class and each of its ancestors: every class built the part of each
ancestor with its own overrides. That was 1,146 functions in hello world (1.1 MB of code for
`InputElement`, `StyledElement` and `Visual` alone).

Every slot is known when the program is compiled: the function of the class, or, for a member
the class states it does not override, the slot of its parent's table. So the table is now a
constant, `Subclassable::VTABLE`. The table of a class (`ObjectType::CLASS_VTABLE`) is a static
of the executable, and `ObjectType::vtable` returns it with no `OnceLock`. A slot that forwards
reads its parent's table through a `const fn`, so the root class (its own parent) never names
its own table.

| `samples/HelloWorld`, macOS ARM64 | before | after |
|---|---:|---:|
| `release` (with symbols) | 73.56 MB | 72.21 MB (-1.35 MB) |
| `dist` | 45.28 MB | 43.03 MB (**-2.25 MB, -5.0 %**) |

### 4.9 Fewer closures in the metadata of types (#569)

The declaration macros generated closures per type and per member, each a function in the
executable (1.72 MB of closure bodies in hello world, most of them in markup metadata):
- an enumeration member was a closure that made its value; it is now its numeric value, made by
  the `enum_from_value` of its type (`MarkupEnumMember::get`), and found by
  `MarkupType::find_enum_member_of`;
- a cast and a reference type registered a closure per form of the value; they register one typed
  function, which a shared adapter calls;
- a property whose typed path hook could only give nothing has none;
- the untyped routes of properties copy a value through one function per value type instead of
  three closures.

| `samples/HelloWorld`, `dist`, macOS ARM64 | Size |
|---|---:|
| class tables as constants (4.8) | 43.03 MB |
| and fewer closures | 41.95 MB (**-1.07 MB**) |

### 4.10 Type-erased binding observers (#576, part 1)

A binding of untyped values (`IObservable<BoxedValue>`: style setters, markup, `bind_untyped`) was
delivered by observers compiled once per value type of a property: the entry of a value frame
(`BindingEntry<T>` with its `UntypedObserver<T>`), the local value observer (`Value<T>` with its
`Observer<T>`) and the direct property observer (`DirectBindingObserver<T>` with `DirectUntyped<T>`).
The routes of every property reach them, so every value type had all three (1.43 MB in 2,477
functions with the animation observers, section 5.4).

They are now one type each, not generic: `BoxedBindingEntry`, `BoxedLocalValueBindingObserver` and
`DirectBoxedBindingObserver`. A value stays boxed (`BindingValue<BoxedValue>`), and the box always
holds exactly the property's value type. The interpretation of an untyped value (the value type,
`BindingValue<T>`, the unset and do-nothing markers, notifications, implicit conversion) is one
function, `resolve_boxed_binding_value`. What names the value type is a route of the property, the
table of `fn` pointers every property already has (`PropertyRoutes`), with four new entries:

- `route_unpack_binding_value`: the parts of a `BindingValue<T>`, its value in a box;
- `route_enable_data_validation`: the data validation flag of the metadata, read without a copy;
- `route_set_local_value` (styled) and `route_set_direct_binding_value` (direct): the typed write
  of the store, which moves the value out of a box the observer made and clones it otherwise;

and the existing `route_box_value`, `route_copy_value`, `route_is_valid_value` and
`route_get_default_value`. Two routes that constructed typed observers
(`route_create_boxed_local_binding`, `route_create_binding_entry`) are gone.

The typed paths are unchanged: `bind` and `bind_value` with their typed entries and observers,
typed `set_value`, and the reads of `EffectiveValue<T>`. Behaviour is unchanged as well: priority,
notification order, data validation, `UnsetValue` and `DoNothing`, disposal and reentrancy, and the
number of copies of a value. A value in a frame is a snapshot in a box of the entry's own, as it was
a clone in the typed entry, so each value delivered to a frame costs one allocation. The one
difference is equality: an entry compares a new value with the comparer of the store
(`value_equals`, which takes two NaN to be equal) instead of `PartialEq` alone, so a NaN delivered
again is not published again, which the store would not have raised either.

The animation observers (`StepObserver<A>`, `PreviewValue<T>`) are not part of this change: they
are generic over the animator, not the property, and their code is the timing of the animation
instance, inlined into them.

| `samples/HelloWorld`, `dist`, macOS ARM64 | Size |
|---|---:|
| fewer closures (4.9) | 41.95 MB |
| one observer for untyped bindings | 40.39 MB (**-1.56 MB**) |

The property, binding and styling benchmarks (`base::`, `data::`, `styling::` of
`ferroui-benchmarks`, release, alternate runs) are within the noise of the machine: the median
benchmark is 2 % faster, and the two resource lookups that are slower in both runs (`find_pre_resource`,
`find_not_existing_resource`, 8 to 14 %) differ by as much between two runs of the same code.

### 4.11 Theme trimming in applications (#568, on by default)

A compiled theme is partitioned into units, one per control or resource document plus the ambient
unit (docs/porting/theme-trimming.md, section 9). An application can construct the themes its
documents construct from only the units it needs. The linker then removes the other units, and
the control code that only their templates reach. This is the default; an application turns it
off in its manifest:

```toml
[package.metadata.ferroui]
theme-trimming = false
```

The build of the application (`ferroui-build`) computes the units from what its documents name:
the types they construct or select and their base classes, and every token that may be a key of
the theme. It adds what the compiled documents of its dependencies name (a control library's
templates), what its Rust sources name (type names that are controls, string literals that
are keys) and the controls the code of each live control creates (the creates edges of the type
models). It follows the keys, `BasedOn` chains and template types of the unit graph, and writes
the table that `<WinUITheme />` is constructed with (`WinUITheme::with_parts`). Two optional forms
of `theme-roots`:

```toml
# A control or style whose name is built at run time, or a unit by its document.
theme-roots = ["CalendarView", "AccentButtonStyle", "Controls/Popup.xaml"]
# Every unit, as without trimming (for comparing).
theme-roots = "all"
```

A control or key the sources name, or that the code of a live control creates, is kept without a
`theme-roots` entry (step 4, docs/porting/theme-trimming.md section 11); a name built at run time
needs one. In debug builds the first lookup of a trimmed control's theme logs
`the WinUI control theme of <Type> was trimmed: add it to theme-roots`.
Builds with `ferroui-base/full-themes` (the run-time loader, hot reload, the designer, DevTools,
Inspect, Diagnostics) keep every unit; the tables of types alone (`type-tables`, which
`ferroui-view` and the behaviors enable) do not. So do themes constructed in Rust with `WinUITheme::new()`;
an application without markup trims its theme with `Build::theme` and `WinUITheme::with_parts`
(section 4.24).
The kept units are listed in `$OUT_DIR/xaml/theme_parts.txt`.

| `samples/HelloWorld`, `dist`, macOS ARM64 | Size |
|---|---:|
| full theme (step 2) | 37.98 MB |
| trimmed (30 of 164 units) | 28.63 MB (-9.35 MB, -24.6 %) |

### 4.12 Type checks name the identity of a class, not its type (#612, B1)

`is::<T>()`, `cast::<T>()` and `downcast_ref::<T>()` compared the runtime type of the object with
`T::TYPE`, the `TypeInfo` of `T`. A `TypeInfo` holds the class's initialisation, its constructor,
its markup metadata and the registration of its handle, so every type check anywhere kept all of
these for `T` in the executable, even in an application that never creates a `T`: a check whether
a text element sits in a `RichTextBlock` linked the rich text block, its properties and their
generic code into a window without one.

Each class now has an identity of its own (`TypeIdentity`, `ObjectType::IDENTITY`, one static per
class made by `ferro_class!`), which its `TypeInfo` points to. A type check walks the base chain of
the object's runtime type and compares identities (`TypeInfo::derives_from`); it gives the answers
the comparison of `TypeInfo`s gave, and it names only the identity. The rest of a class is linked
when something uses it.

| `ferroui-desktop --example hello_window`, release, macOS ARM64 | main | after | Change |
|---|---:|---:|---:|
| Executable | 41.76 MB | 38.64 MB | -3.12 MB (-7.5 %) |
| Classes initialised | 188 | 153 | -35 |
| `ferroui_base` (code and named data) | 8.70 MB | 8.29 MB | -0.41 MB |
| `ferroui_controls` | 2.59 MB | 1.76 MB | -0.83 MB |

The next steps of #612 keep the initialisation, the constructor and the markup metadata of a class
reachable from its `TypeInfo` only with the tables of types (reflection), and remove the other
defaults that construct what an application does not use.

### 4.13 Trimmed theme units take the classes of their binding paths with them (#612, root D)

With theme trimming on, `samples/HelloWorld` (WinUI theme, XAML) kept 30 of the theme's 164 units,
none of them for NavigationView, InfoBar or RichEditBox, yet it still initialised those classes. The
chains of the IR trace (section 2, "Finding what roots code") all ran:

`WinUITheme::with_parts` → `build_controls_winuicontrols_xaml` → the namespace table of the
document → the cast-target functions of its reflection-binding paths →
`NavigationView::TYPE`, `RichEditBox::TYPE`, `InfoBar::TYPE`, `ProgressBar::TYPE`,
`SplitView::TYPE`, `NavigationPage::TYPE` (and from NavigationPage's virtual table, DrawerPage and
its automation peer).

A path such as `(controls:TextBox.Text)` names a class, which the generated code yields through a
function in the document's namespace table. The table carried every class any path of the merged
theme names, and every unit's code reaches it, so dropping a unit left its controls linked. The
emitter now collects the path types per unit: the document's table keeps the paths outside every
unit, and an entry of a unit with paths pushes a table of the unit around its code, which its
templates and deferred resources build with ([xaml-themes.md](xaml-themes.md), section 3). A test
of the WinUI theme checks that no table naming classes is reachable from code outside the units.

One more chain led from `register_value_types` to `XBinding::create_instance`, whose default
update trigger for `TextBox.Text` called `TextBox::text_property()`, the accessor that registers
the property for `TextBox`. `TextBox.Text` is `TextBlock.Text` with TextBox as another owner, so
x:Bind compares with `TextBlock::text_property()` and checks the target by identity (4.12).

| `samples/HelloWorld`, macOS ARM64 | main | after | Change |
|---|---:|---:|---:|
| `dist` executable | 22.13 MB | 19.45 MB | -2.67 MB (-12.1 %) |
| fat LTO `release` (the build of the trace) | 38.55 MB | 33.66 MB | -4.89 MB (-12.7 %) |
| Unused controls reachable from `main` | NavigationView, RichEditBox, InfoBar, ProgressBar, SplitView, NavigationPage, DrawerPage, TextBox | none of these | |

### 4.14 Theme crates optimised for size in `dist` (#581, step 1)

Compiled theme markup runs once, when a theme is constructed. In the shipped build the five theme
crates (`ferroui-themes-winui`, `-fluent`, `-simple`, `-macos`, `-material`) are at
`opt-level = "z"` (`[profile.dist.package.*]` in `Cargo.toml`); the rest stays at `3`. Per-crate
levels survive fat LTO: `z` marks the crate's functions `minsize`, which link-time optimisation
respects after inlining across crates. Start-up is the median wall time of launches that open the
window and close it at once (`FERROUI_SMOKE_EXIT_MS=1`), alternating between the builds.

| macOS ARM64, `dist` | main | theme crates at `z` | Change | Start-up (median) |
|---|---:|---:|---:|---|
| `samples/HelloWorld` (theme trimmed to the units it uses) | 22.13 MB | 21.62 MB | -0.51 MB (-2.3 %) | 193 ms and 191 ms, 11 launches each |
| `samples/ControlCatalog.Desktop` (every unit) | 67.59 MB | 65.71 MB | -1.87 MB (-2.8 %) | 373 ms and 364 ms, 9 launches each |

Before theme trimming, the same change gave hello world 5.16 MB (#581, first measurement): trimming
already removes the units an application does not use, so what remains to shrink is the units it
keeps. Start-up is within noise in both. The browser profile already builds every crate at `z`
by default (section 4.1 and the comments of the profile).

### 4.15 The default constructor needs the tables of types (#612, B3)

The `TypeInfo` of a class carried its default constructor (`new` of `ferro_class_info!`). Every
`TypeInfo` an application reaches is reachable from start-up (a type check of an older form, a
`TargetType`, a property owner, a base class), so the constructor of every class it names was
linked, and with it what the constructor alone reaches: the construction of the class, its
default values and its templates. An application creates only a few of those classes.

The constructor is now in the `TypeInfo` only with the tables of types (the `ferroui-base` feature
`type-tables`, section 4.6), which the run-time XAML loader, the designer, DevTools, hot reload and
Inspect enable: those builds still create any class by reflection. `ferro_class!` passes the
constructor to `TypeInfo::with_constructor`, a `const fn` of `ferroui-base` whose body keeps it or
drops it by the features of `ferroui-base`, wherever the class is declared; the evaluated `TypeInfo`
static then holds no pointer to it, and the linker leaves it out. Without the tables,
`TypeInfo::default_constructor` and `TypeInfo::create_instance` return `None` for a class whose
constructor nothing registered.

Code that names a class creates it statically:
- `DefaultConstructible::new_instance()`, implemented by `ferro_class_info!` for every class with
  `new`; `PageType::of::<T>()` of a frame (B3, part 1), `PageNavigationExtensions::push_async_of`,
  `replace_async_of` and `push_modal_async_of`, and
  `ManagedFileDialogExtensions::use_managed_system_dialogs_with` take the constructor this way,
  with a `DefaultConstructible` bound the compiler checks;
- `TypeInfo::register_constructor::<T>()`, for code that names a class and hands its runtime type to
  something that creates it by reflection. Compiled markup does this for `{x:Type local:Page}`
  given to `Frame.SourcePageType` or `Frame.Navigate`, and an application whose view locator
  creates its views from their runtime types registers their constructors from its
  `register_types()` (`samples/SafeAreaDemo`).

The constructor is not registered when a class is initialised: the initialisation is reachable from
the `TypeInfo` too, and would link every constructor again. Reflection over a class that nothing
registered fails with an error that names the fix: `NavigationError::MissingConstructor` names
`PageType::of` and `type-tables`.

| `samples/HelloWorld`, macOS ARM64 | before | after | Change |
|---|---:|---:|---:|
| `release` executable | 33.73 MB | 32.87 MB | -0.87 MB (-2.6 %) |
| `dist` executable | 18.94 MB | 18.53 MB | -0.42 MB (-2.2 %) |

The estimate of 2.4 MB (#612) was the share of the constructors before root D and the other
trims of this section had already removed most of the classes they reach; what remains is the
constructors of the classes an application links for other reasons. The `dist` build ran its smoke
run to a clean exit.

### 4.16 Value types registered on first use (#566)

`ferroui-base` registered the nullable forms, texts, casts and conversions of every value type,
enumeration, contract and collection it declares from one function, `register_value_types`, which
`register_types()` handed to `ValueTypes::register_global` at start-up whatever the features. That
list named about 480 types, so each of them was linked into every application with the code of its
registrations: `nullable_operation` (0.31 MB, 731 instances in hello world), the closures of
`register_conversion` (0.12 MB, 484 instances) and of `register_cast`, and the `AnyValue` vtable of
every type the registrations box (`any_value_eq`, 0.33 MB, 1,874 instances). `ferroui-controls` did
the same for its own types.

**On first use.** A type now states its registrations in an implementation of
`ValueTypeRegistration` (`register_value_conversions`), next to its metadata in `markup_types/`.
They are made on every thread, once (`ValueTypes::register_used::<T>()`, which `register_global`
deduplicates by the address of the function), when the type is first used:
- a styled, attached or direct property of `T`, `Option<T>`, `Rc<T>` or `Option<Rc<T>>` is
  registered: the declaration macro selects this at compile time, by the probe that #583 uses for
  the markup metadata of a property's value type;
- its markup metadata becomes known: `MarkupType::value_types`, which the declaration macros fill
  for every type they declare, runs when the metadata is first registered (the value type of a
  property of a registered type, a type an application registers, the tables of types).

A type that no linked code uses this way is not named by anything reachable from start-up, and the
linker drops its registrations, its `nullable_operation` and the vtables only they create.

**With the tables of types.** Each crate keeps a table, `register_value_type_table`, that makes all
of its types used. `ValueTypes::register_table` registers it only with the `ferroui-base` feature
`type-tables` (and in the unit tests of `ferroui-base`), decided by the features of `ferroui-base`
wherever it is called, as `MarkupType::register_table` is (section 4.6); `ValueTypes::TYPE_TABLES`
says which. The run-time XAML loader, the designer, DevTools, hot reload and Inspect see every type
from the start, as before.

**What stays at start-up.** Some registrations are needed before any first use the registry can
see, and stay in the list each crate registers at start-up (`register_value_types`):
- the casts and nullable forms of the types that members of markup types hold, such as the
  converter of a binding (`Rc<dyn IValueConverter>` and the casts of the converters to it), the
  setters of a style, the routed event of an event trigger, the easing of a transition. Compiled
  markup converts a value for such a member (`rt::cast::<Option<Rc<dyn IValueConverter>>, _>`)
  before it calls the typed setter of the member, and setting a member registers nothing. The
  registrations of a type stay at start-up when `Option<T>` (for its nullable form) or the target
  of a cast is the type of a property, a constructor parameter or a method parameter of a
  `ferro_markup_type!` declaration anywhere in the workspace;
- in `ferroui-controls`, the registrations of instantiations of the generic types of
  `ferroui-base` with a type of the crate (`RoutedEvent<SpinEventArgs>`, `FerroList<Ref<Page>>`)
  and the casts to types of `ferroui-base` (a converter to `IValueConverter`): the orphan rule
  leaves a `ValueTypeRegistration` of a type of another crate to that crate;
- the defaults of the registry (the primitives, `Duration`, `Uri`, `UriRef`, `Url`, `Locale`, the
  numeric conversions), which every application converts.

| Registration calls of the lists | on first use | at start-up |
|---|---:|---:|
| `ferroui-base` | 352, of 273 types | 112 |
| `ferroui-controls` | 163, of 150 types | 81 |
| `ferroui-controls-colorpicker` (the enumerations) | 10, of 10 types | 0 |

The Dock crates register their types on the calling thread from their `register_types()`, mostly
nullable forms of contracts of the model crates and casts of converters to the contracts of
`ferroui-base`: a `ValueTypeRegistration` of those would belong to the model crates and to
`ferroui-base`. They are left as they are; the change of the casts below applies to them.

**Functions, not closures.** `ValueTypes::register_cast`, `register_conversion`,
`register_try_cast` and `register_parse` take function pointers (a closure that captures nothing
coerces to one). A registration is data: the typed function and the adapter of its pair of types,
which downcasts the value, calls the function and boxes the result. The wrap of a nullable form
and the casts of a reference type are entries of the same kind. No registration makes a closure, a
vtable with its call shims or drop glue any more. A cast or conversion that captures state
registers with `register_cast_with` or `register_conversion_with`.

**`nullable_operation` and `any_value_eq`.** `nullable_operation::<T>` boxes and unboxes
`Option<T>`: it clones the `T`, which only typed code can do, so it stays generic, and is now
instantiated only for the types that are used. `any_value_eq` is the comparison of the `AnyValue`
vtable of a type, which every boxed value needs; a comparison shared between plain-old-data types
would compare bytes, which differs from `PartialEq` for floating point fields (`NaN`, `-0.0`) and
padding, so it is not shared. Its instances drop with the types that are no longer boxed.

`ferroui-build-scan` reads the registrations of a `ValueTypeRegistration` implementation as it
reads the body of a function, so the type models of the crates, and the compiled markup made from
them, are unchanged.

| `samples/HelloWorld`, macOS ARM64 | before | after | Change |
|---|---:|---:|---:|
| `release` executable | 32.87 MB | 31.98 MB | -0.89 MB (-2.7 %) |
| `dist` executable | 18.53 MB | 17.98 MB | -0.55 MB (-3.0 %) |
| `nullable_operation` (instances) | 731 | 458 | -273 |
| `register_conversion` closures (instances) | 484 | 0 | -484 (data, `fn` pointers) |
| `any_value_eq` (instances) | 1,874 | 1,351 | -523 |

The ControlCatalog page tour (127 pages) and the trimmed samples (HelloWorld, RenderDemo,
VirtualizationDemo, BehaviorsGallery) ran with no conversion or registration error and the same
binding messages as main; the `dist` build of hello world smoke-runs cleanly.

### 4.17 The write of a value carries its coercion only where a callback is registered (#570)

`EffectiveValue<T>::set_and_raise_core` is the write of a value: every local value, style value
and binding result goes through it, and it is compiled once for each value type (section 5.5:
0.44 MB in 232 copies in hello world). Every copy handled a coerced value (the callback, the two
uncoerced values), though 12 of the 159 value types of hello world have a property with a coercion
callback, and every copy began with a copy of the value it was about to replace.

- The write of a property with a callback is `set_and_raise_coerced`: the body the write had. It is
  reached through the uncommon fields of the value, which only the registration of a coercion
  callback makes (section 4.4), so a value type without one does not carry it.
- Its typed leaves (the comparison and the two notifications) are named once, where the value is
  made, and kept with the uncommon fields. Named in the write itself, each copy of the write kept
  copies of the notifications: 233 where there were 159. A function named by its address in a
  generic function is kept for every copy of that function.
- The write of every other property takes the previous value out of its cell and keeps it to the
  end, where the copy was dropped: it is the old value of the change, and what dropping it does
  still happens after the notification. Under an animation, where the replaced value becomes the
  base value, it is copied only if the value changed.

| `samples/HelloWorld`, release, macOS | Before | After |
|---|---:|---:|
| `set_and_raise_core`, 232 copies | 0.438 MB | 0.381 MB |
| `set_and_raise_coerced`, 12 copies | | 0.016 MB |
| Machine code | 17,630,412 bytes | 17,612,808 bytes |

The writes lose a copy and a drop of the value. The property, setter, styling and tree benchmarks
of the gate (65 metrics, none over tolerance):

| Benchmark | Before | After | Time | Instructions |
|---|---:|---:|---:|---:|
| `ControlWrite.local_unchanged[Control=Border]` | 9.69 ns | 8.59 ns | -11.4 % | -6.19 % |
| `ControlWrite.non_copy_changed[Control=Border]` | 45.71 ns | 44.47 ns | -2.7 % | -4.83 % |
| `ControlWrite.local_changed[Control=Border]` | 25.50 ns | 25.40 ns | -0.4 % | -2.44 % |
| `ControlWrite.local_changed[Control=Button]` | 35.92 ns | 35.16 ns | -2.1 % | -1.84 % |
| `ControlWrite.inherited_changed[Control=Border]` | 23.61 ns | 24.25 ns | +2.7 % | -2.56 % |
| `ControlWrite.attached_changed[Control=Border]` | 28.48 ns | 30.03 ns | +5.4 % | -2.50 % |
| `SetterApply.plain[Controls=1000, Setters=32]` | 2.084 ms | 2.033 ms | -2.4 % | -0.81 % |

### 4.18 The About dialog, its document and its font are linked with the first dialog (#612)

Hello world shows one text block and linked 171 classes, `ferroui_dialogs::AboutFerroDialog` among
them. Root A of #612 took the dialog out of the macOS application menu; it was still reached another
way. Every theme registers the types of `ferroui-dialogs`, for the managed file chooser it styles,
and that registration also made the compiled markup of the crate known: the loader table that
answers a load of `AboutFerroDialog.xaml` by its URI with an instance of the dialog, and the font of
the document (`Roboto-Light.ttf`, 170 kB). A table of loaders is a list of functions, so the dialog,
its compiled document and what the document uses were in every application with a theme.

The registration of the types now makes the compiled markup known only with the tables of types
(`ValueTypes::TYPE_TABLES`: the run-time loader, the designer, hot reload, where a document is
loaded by a URI that nothing in the code names). Without them the constructor of the dialog
registers the font and the loader table, so an application that shows the dialog has both, and one
that never does links neither.

| `samples/HelloWorld`, release, macOS (`scripts/symsize.py`) | Before | After | Change |
|---|---:|---:|---:|
| Attributed to symbols | 21.83 MB | 21.36 MB | -0.47 MB (-2.2 %) |
| Machine code | 17,612,808 bytes | 17,365,212 bytes | -247,596 bytes |
| `ferroui_base` | 8.12 MB | 7.96 MB | -0.16 MB |
| Anonymous data (the font) | 1.86 MB | 1.66 MB | -0.20 MB |

A load of `AboutFerroDialog.xaml` by its URI before any dialog was made finds the document only
with the tables of types; the dialog itself is unchanged.

### 4.19 A type a theme style only selects is not a type its unit constructs (#568, #612)

Hello world kept 30 of the 166 units of the WinUI theme: besides the text block and the button of
the window's caption, the menu item, the menu scroll viewer, the scroll viewer, the scroll bar, the
repeat button, the thumb and the items control, with their resources. None of them is in a window
with one text block.

A kept unit makes the types it names live, so that the controls its templates construct root their
own themes (`docs/porting/theme-trimming.md`, section 10). The types of a unit were those of its
elements, of its `TargetType`s, of its `{x:Type}`s and of the type selectors of its selectors. The
ambient unit, which every application keeps, has one global style, `MenuItem > PathIcon`, for the
icon of a menu item. Its selector made the menu item a type of every application, and the unit of
the menu item brought the rest: its template constructs a popup and a scroll viewer, whose
templates construct scroll bars, repeat buttons and thumbs.

A selector is a condition: a style that selects a type styles it where something else made it. The
unit graph (`theme_unit_graph.rs`) now keeps the types of selectors apart from the types a unit
constructs, and only those make a unit's types live. The documents of an application root what
their selectors name as before: an application that styles a type is taken to use it.

One type was kept by a selector alone and is constructed by no element: a tool tip, which the
framework makes for an element with `ToolTip.Tip`. The Fluent theme named it in a selector of its
validation errors; under the WinUI theme an application with `ToolTip.Tip` on a button kept no
theme for it at all. A document that sets `ToolTip.Tip`, as an attribute or as a property element,
now constructs a `ToolTip` (`ATTACHED_CONSTRUCTIONS`).

| `samples/HelloWorld`, release, macOS (`scripts/symsize.py`) | Before | After | Change |
|---|---:|---:|---:|
| Units of the theme | 30 of 166 | 16 of 166 | |
| Attributed to symbols | 21.36 MB | 19.48 MB | -1.88 MB (-8.8 %) |
| Machine code | 17,365,212 bytes | 15,831,876 bytes | -1,533,336 bytes |
| `ferroui_base` | 7.96 MB | 7.24 MB | -0.72 MB |
| `ferroui_controls` | 1.33 MB | 1.10 MB | -0.23 MB |
| `ferroui_themes_winui` | 0.98 MB | 0.46 MB | -0.52 MB |

The units other applications keep, before and after:

| Sample | Theme | Before | After | |
|---|---|---:|---:|---|
| `binding-demo` | Fluent | 29 of 75 | 29 of 75 | the same units |
| `virtualization-demo` | Fluent | 32 of 75 | 32 of 75 | the same units |
| `render-demo` | Fluent | 33 of 75 | 33 of 75 | the same units |
| `reactive-demo` | WinUI | 45 of 166 | 47 of 166 | the tool tip and its resources, which its documents set and it had no theme for |

### 4.20 A style, a native menu and an automation peer do not keep the menu classes (#570, #612)

Hello world still linked 136 classes, among them the context menu, the menu item, the menu base,
the selecting, headered and plain items controls, the radio and toggle buttons, the virtualizing
and wrap panels and the presenter of a menu flyout: 14 classes of menus and lists in a window with
one text block. `scripts/ircut.py` (section 2) put a number on them before anything was changed:
no class of the family kept more than 16 kB by itself, the family together kept 372 kB, and three
references from outside were all that entered it.

- **A style.** `MenuItem > PathIcon`, the one global style of the WinUI theme, is built by the
  function that builds the theme for every application. The unit graph no longer counts the menu
  item as a type of the application (section 4.19), but the generated code still wrote the selector
  with `<MenuItem as StaticType>::TYPE`, and the `TypeInfo` of a class names its initialisation: the
  base classes, every property and every class handler. A selector needs none of that. Compiled
  markup now writes `Selectors::of_type_identity` and `Selectors::is_type_identity` with the
  identity of the class (`ObjectType::IDENTITY`, the static that `is::<T>()` compares since #612),
  in the generated functions and in the static selector chains of the themes alike. The selector
  matches by the identity, the candidate index of a style collection files it by the identity, and
  its text comes from the name the identity carries. `Selector::target_type` finds the `TypeInfo`
  through the identity once the class is initialised (`TypeIdentity::registered_type`, a look-up
  among the known types of the name, which the selector remembers); before that nothing is of the
  class. Selectors built in code and by the run-time loader are as they were.
- **The native menu.** Every application on macOS has a native menu. `NativeMenuItem` took
  `IsChecked`, `Command` and `CommandParameter` from `MenuItem` by adding itself as an owner, which
  initialises `MenuItem` and, through `GroupName`, the radio button. `Command` and
  `CommandParameter` of the menu item are the properties of the button: the native menu item now
  adds itself to those, and they stay one property in the three classes. `IsChecked` is a
  property of the native menu item alone; that of the menu item is as it was (the snapshot of the
  property defaults only grows).
- **The automation peer of every control.** `ControlAutomationPeer::show_context_menu_core` called
  `ContextMenu::open_at`. It now calls `Control::open_context_menu`, which goes through a function
  the context menu class states when it is initialised; a control cannot have a context menu
  before that.

`ircut.py --edges` on the whole-program module predicted 342 kB and the 14 classes from these
three changes; the thin-LTO build of the default profile gives more, because value types that only
those classes used go with them. The local gate on styling, the trees, element construction, the
counters, the gauntlet and the frame benchmarks shows no change (218 metrics; one line slower in
time alone with the instructions of the base did not repeat in a second run):

| `samples/HelloWorld`, release, macOS (`scripts/symsize.py`) | Before | After | Change |
|---|---:|---:|---:|
| Classes linked | 136 | 122 | -14 |
| Attributed to symbols | 19.48 MB | 19.01 MB | -0.47 MB (-2.4 %) |
| Machine code | 15,831,928 bytes | 15,461,764 bytes | -370,164 bytes |
| `ferroui_base` | 7.24 MB | 7.07 MB | -0.17 MB |
| `ferroui_controls` | 1.09 MB | 0.94 MB | -0.15 MB |

What the same analysis says of the rest, for later: the automation peers together keep 469 kB,
which only an application that gives up accessibility could leave out; the typography and inline
classes of the text block keep 165 kB; the view box, the layout transform control and the
validation errors keep 125 kB, the first two through templates of the kept theme units.

### 4.21 The typography properties are linked with their first use (#570)

A text block asks for the effective font features of its text each time it lays the text out
(`Typography::effective_font_features`). The function resolved the 43 inherited typography
properties only after one of them had changed on the thread, behind a flag, so an application
without typography paid one read of a flag. The executable paid all the same: the function named
the resolution (`TypographyFormat::effective`), which reads the 43 properties, whose registration
brings the property code of their value types. A flag that is tested at run time unlinks nothing.

`scripts/ircut.py` found a second reference. The property-changed handlers of the text block and
of the text element ask whether a changed property is a typography property, and compared its
owner with `Typography::TYPE`. Naming the type keeps its initialisation, which registers the 43
properties. Eleven references from outside the typography modules kept 144 kB.

- The flag is the function now: a typography property that changes sets how typography is
  resolved (`TYPOGRAPHY_IN_USE`, an `Option` of a function), and `effective_font_features` calls
  what was set or reads `FontFeatures` when nothing was.
- `Typography::is_property` compares the owner of the property with the type the typography
  properties stated when they were registered on the thread (`TYPOGRAPHY_TYPE`). Before one is
  registered no property is a typography property.

An application that uses typography behaves as before: reading or setting its first typography
property registers the properties, and the first change turns the resolution on.

| `samples/HelloWorld`, release, macOS (`scripts/symsize.py`) | Before | After | Change |
|---|---:|---:|---:|
| Attributed to symbols | 19.01 MB | 18.84 MB | -0.17 MB (-0.9 %) |
| Machine code | 15,461,764 bytes | 15,315,420 bytes | -146,344 bytes |
| `ferroui_base` | 7.07 MB | 6.98 MB | -0.09 MB |
| `ferroui_controls` | 0.94 MB | 0.89 MB | -0.05 MB |
| Symbols of the typography modules | 170 | 5 | |

The local gate on the text benchmarks, element construction, the property system and the frame
benchmarks shows no change (348 metrics). `frames/reorder` was 17 % over on the UI thread in the
first run and equal in a second run of five rounds, in which both sides ran at the slower of the
two levels this scene has (`docs/porting/desktop-performance.md`, section 9.67).

The pattern, for the next case: where code is needed only after something was used, let the use
state the function and let the common path call what was stated. A boolean keeps the code.

### 4.22 Adding and removing a registration of a metadata table are shared (#570)

`MetadataTable<M>` holds the metadata a property has for its owners: the registrations, the
look-up along the base classes of the type that is asked for, and the cache of that look-up. Its
values were `Rc<M>`, so its functions were compiled once for every value type of a property: in
hello world 161 copies of `unregister` (103 kB), 215 of `get` (73 kB) and 8 of
`override_metadata`. `unregister` is named by nothing but the table of virtual functions of a
property (`route_unregister`, for `FerroPropertyRegistry::unregister_by_module`): every property
type carried a copy that no application calls.

Adding and removing a registration work on the types of the owners and not on the metadata. The
entries (`MetadataEntries`, not generic) now hold each value as the pointer `Rc::into_raw` gives
for the handle the table owns, and one copy of `insert` and `remove` serves every table. The
table knows `M` and turns a pointer back; a value that leaves the entries is given up through
one small function per metadata type, which the shared code is handed. A table is as large as it
was.

The look-up stays with the metadata type. It runs for every read of a default value and of the
metadata of a property, and sharing it was measured twice before it was left alone:

| Version | Machine code | Speed |
|---|---:|---|
| `get` inlined, the look-up shared | +5 kB | not measured |
| `get` out of line, the look-up shared behind it | -141 kB | `styling_subsystem::SelectorMatch.none` +11 % in time alone, +9.5 % with the code moved (section 8.1 of `desktop-performance.md`): real |
| the whole look-up in one shared call | -139 kB | the indexed styling benchmarks +7 to +9 %, the propagation of a property +7 % |
| the look-up as it was, adding and removing shared | -125 kB (-119 kB on the main branch of the table below) | nine benchmarks of styling, propagation and the property system between -3.9 % and +0.1 % |

Two things worth keeping from the first two rows. The sum of the symbols
(`scripts/symsize.py`) fell by 0.10 MB in the first version while the machine code of the
executable (`size -m`, `__TEXT,__text`) grew: with `get` inlined, each typed caller carried the
choice between a registered value and the default, and the `Drop` of a table was compiled per
type. The section sizes of the linker are the measure; the sum of the symbols follows what is
inlined where. And a call is not free where the code runs ten thousand times a pass: the second
version retired fewer instructions than the base and took a tenth longer.

| `samples/HelloWorld`, release, macOS (`size -m`) | Before | After | Change |
|---|---:|---:|---:|
| Machine code (`__TEXT,__text`) | 15,315,420 bytes | 15,196,012 bytes | -119,408 bytes (-0.8 %) |
| Constants (`__TEXT,__const` and `__DATA_CONST,__const`) | 2,315,118 bytes | 2,316,750 bytes | +1,632 bytes |
| File | 28,133,136 bytes | 27,900,928 bytes | -232,208 bytes |

The local gate on the property system, styling, the trees, element construction, the counters,
the gauntlet and the frame benchmarks: 244 metrics, one over its tolerance,
`trees::Propagation.data_context` at +15.4 % in time alone with the instructions of the base. A
second run of nine rounds had it at +1.1 %, and the candidate built with 36 bytes in front of
its code (the placement check of section 8.1 of `desktop-performance.md`) ran the eight
propagation benchmarks between -5.0 % and +3.0 % of the base. The eight move together with the
build: the two gates of section 4.20, a change that touches no propagation, had them between
-31 % and +16 %.

### 4.23 A style written as text does not keep every control (the declarative layer)

An application on the declarative layer (`ferroui-view`) was 10 MB larger than hello world
whatever it showed: the published comparison's FerroUI application, three screens of borders,
text, panels and a list, was 30.5 MB.

**The root.** `ferroui-view` turned the tables of types on (`ferroui-base/type-tables`, section
4.6) for one function, `parse_selector`, behind `style("ListBoxItem")`: a selector in text names
its classes, the parser looked a name up in the table of every type, and the function registered
the tables of the framework first. With the tables registered every control is reachable, and
the linker keeps all of them. One style in text anywhere in an application was enough.

**The change.** A selector may know a class by its name alone
(`Selectors::of_type_name`, `Selectors::is_type_name`): it matches an element whose class, or
with `:is(..)` one of its base classes, has that name, and takes the type itself once the class
is known. The parser of selectors falls back to it for a name its resolver does not know
(`SelectorParser::unknown_types_by_name`). `parse_selector` looks a name up among the classes
that are known when it runs (a class is known once an object of it exists) and uses the name for
the rest; it registers no table. `ferroui-view` turns the tables on only with its feature
`xaml`, which loads markup at run time and needs them.

A name no class has matches nothing, where it was an error before; the selector of a property
(`Button[IsDefault=True]`) still needs its class known.

| The comparison's FerroUI application (release, macOS, stripped) | Before | After |
|---|---:|---:|
| File | 30.53 MB | 23.05 MB |
| `__TEXT,__text` | 25.58 MB | 19.46 MB |

Its three screens draw the same pixels before and after (the gauntlet and the patchwork frozen
on one frame, compared pixel by pixel). A first start of a program the system never ran checks
the whole file, so the size is start time too: the first frame of a first start went from 1.41 s
to 1.27 s at the median of the comparison's harness (docs/porting/desktop-performance.md,
section 9.83).

Tests: `selectors_by_name_match_what_the_selectors_by_type_match` (`styling/selector_tests.rs`),
`a_type_the_resolver_does_not_know_is_an_error_unless_it_may_be_matched_by_name` (the parser's
tests); the 75 tests of `ferroui-view` pass without the tables.

### 4.24 An application without markup trims its theme (the declarative layer, #570)

Theme trimming (section 4.11 and docs/xaml-themes.md, section 3) started from a document: the
build wrote the table of the units an application needs where its compiled markup constructs
the theme (`<WinUITheme />`). An application whose interface is Rust code, the builders and
`rsx!` of `ferroui-view`, constructs its theme in code, and `WinUITheme::new()` is every unit:
it linked every control of the theme, and was 12 MB larger than the same application in XAML.

**The change.** Two additions to the build, none to the run time:

- the functions and methods the sources call are roots, read as the names of classes
  (`text_block()` is `TextBlock`, `.tool_tip("..")` is `ToolTip`): a builder is named after
  the class it makes, and the rule that roots `Button::new()` roots `button()`;
- `Build::theme("WinUITheme")` states a theme the code constructs, and the build writes its
  table (`compiled_markup::THEME_PARTS_..`) for a crate without documents. The code constructs
  the theme with `WinUITheme::with_parts(None, THEME_PARTS)`.

The three declarative versions of hello world are built this way (a `build.rs` of one statement,
two lines in `main.rs`): they keep 16 of the 166 units of the WinUI theme, the list the XAML
version keeps.

| Release, macOS ARM64 | Stripped before | Stripped after | `__TEXT,__text` before | after |
|---|---:|---:|---:|---:|
| `hello-world` (XAML) | | 19.29 MB | | 15.23 MB |
| `hello-world-builders` | 31.77 MB | 19.35 MB | 25.13 MB | 15.29 MB |
| `hello-world-macros` | 31.77 MB | 19.35 MB | 25.13 MB | 15.29 MB |
| `hello-world-hooks` | 31.77 MB | 19.35 MB | 25.13 MB | 15.29 MB |

The style of the API costs 0.06 MB again (section 3.6): the declarative versions are 39 %
smaller than they were.

The calls are as conservative as the names of step 4 are: a function of an application named
after a control keeps the unit of that control. They add no unit to the applications that were
trimmed already (the kept units of eight samples, docs/porting/theme-trimming.md, section
11.9). An application that keeps `WinUITheme::new()` is as large as before.

Tests: `source_references_are_the_names_and_strings_of_the_code` (the scanner),
`a_theme_the_code_constructs_is_trimmed_by_what_the_sources_call` (`ferroui-build`).

### 4.25 One property store per storage type (#986)

The property store was compiled once for every value type of a styled property:
`EffectiveValue<T>` (its write, `set_and_raise_core`, 1.2 to 2.8 kB a copy), the typed paths of
`ValueStore`, the immediate value entries, about 6 kB for each of the 157 value types of the
builders hello world (section 5.6).

**The change.** `StyledValue` (declared in `ferroui-system`, re-exported by `ferroui-base`) names
the type the store keeps a value type as, its storage type, and the two conversions:

- every enumeration declared with `ferro_markup_enum!` (or `styled_enum!`) is kept as one
  `EnumValue`, its numeric value; the macro lists the members, and the conversions are a `match`
  each way, which does not compile when a variant is missing;
- an `Option<T>` is kept as an `Option` of the storage type of `T`;
- every other type (handles `Ref<T>`, sets of flags, `ElementRef<T>`, `Rc<dyn ..>`, the other
  structures) is its own storage type (`styled_value!(Type)` where the type is declared).

Two forms were measured and left out after their gates: handles kept as `Ref<FerroObject>` (the
checked downcast of every read of a handle, +45 % instructions for the read of a handle, and the
conversion of a change notification; it saved 0.11 MB, 19 handle types in one store) and points,
sizes, vectors, thicknesses, rectangles and corner radii kept as `[f64; 2]` and `[f64; 4]` (13.5
kB for +2.7 to 3.7 % instructions in the reads of inherited values, the moves of a line and the
start of the tiles).

The store is compiled per storage type: a read is `T::from_storage` of the stored value, a write
`T::into_storage` and the store's write. What a property hands out still holds its value type: a
boxed value, a binding value, a setter value and a change notification (a storage type that
other types share converts the two values back through the routes of the property, one call
where a value changed; a member of an enumeration is handed out as the static member the macro
lists, `StyledValue::MEMBERS`, with no call). The metadata the store reads is the typed metadata
itself for a type that is its own storage type; for the others a copy made once, with the
default converted and a coercion callback wrapped. The typed API of `StyledProperty<T>` and its
routes stay per value type.

| Release, macOS ARM64 | Before | After |
|---|---:|---:|
| `hello-world-builders` stripped | 20,610,240 B | 20,463,376 B (-0.71 %) |
| `hello-world-builders` `__TEXT,__text` | 16,336,672 B | 16,213,900 B |
| `hello-world-builders` Rust code in families of two or more instances | 7.00 of 11.60 MB (60.3 %) | 6.84 of 11.49 MB (59.6 %) |
| `hello-world` (XAML) stripped | 20,529,096 B | 20,381,928 B (-0.72 %) |
| `hello-world` (XAML) `__TEXT,__text` | 16,256,708 B | 16,134,144 B |

The first form (handles shared too, `try_set_value_typed` through the untyped route) was
20,310,896 B: the handles, 0.11 MB, and the typed route of `try_set_value_typed`
(`route_take_value`, 40.7 kB, which moves the value instead of copying it) are the difference.

`scripts/generic_instances.py`, the families of the store in the builders hello world (instances,
kB, measured with handles shared; with handles their own storage type the engine families have
116 instances instead of 97):

| Family | Before | After |
|---|---:|---:|
| `EffectiveValue<T>::set_and_raise_core` | 202, 318.3 | 125, 206.1 |
| `ValueStore::set_value` | 201, 114.9 | 115, 59.2 |
| `ValueStore::get_base_value` | 169, 99.8 | 99, 56.7 |
| `EffectiveValue<T>::new` | 157, 78.6 | 97, 45.5 |
| `raise_unset`, `apply_entry`, `notify_value_changed`, `notify_base_value_changed` | 157 each, 252.0 | 97 each, 163.3 |
| `ValueStore::create_effective_value` | 157, 38.8 | 89, 27.5 |
| `ImmediateValueEntry<T>::try_get_value` | 201, 34.6 | 126, 23.1 |
| New: `route_with_values`, the storage metadata (`storage_copy`) | | 76, 30.8; 137, 24.9 |

157 value types reach the store through 116 storage types. What stays per value type is what
handles a `T` itself: the routes of `StyledProperty<T>` (about 3.6 kB a type), the typed
`ValueStore::get_value` and `inherited_or_default` (their default value comes from the typed
metadata), the class registrations of handles. A storage-typed default (the cache of the single
default and the metadata of the store kept beside the typed metadata) would move the reads of a
default to the storage type as well: the next step, about 1.2 kB for each value type that shares
a storage type.

Breaking: the value type of a styled or attached property implements `StyledValue`
(`styled_value!(Type)` for a type of an application, `styled_enum!` for an enumeration without
markup metadata); `StyledProperty<T>` and `AttachedProperty<T>` read and write values only where
`T: StyledValue`. `FerroObject::try_set_value_typed` sets a styled property through a route of its
own (`route_take_value`, the value moved), so that it keeps `T: PropertyValue`.

Tests: the conversions (`styled_value` tests of `ferroui-system` and `ferroui-base`), and every
test of the property store, which now runs on the shared storage type for its enumerations.

**The parts that follow** (one stack, each a pull request of its own):

- *The routes shared, a codec per value type* (#997): one implementation of the routes of a
  styled and an attached property for every value type (`StyledRoutes`), with what depends on the
  value type in `StyledCodec`, implemented by `StyledProperty<T>` (the effective value, the coerced
  default, the default boxed, the metadata, validation, the copy into the storage type, boxing, the
  values of a notification, data validation, binding values). The untyped paths (markup, bindings,
  setters, resources, the C ABI) read and write the store through its untyped interface; the typed
  paths are unchanged. The 20 routes of `StyledProperty<T>` (157 copies of about 3.6 kB in the
  builders hello world) are one.
- *The conversion of an untyped value shared*: a value of another type is converted, copied into
  a box of its own through the codec and the temporary released before the write, as the typed
  routes did (`StyledCodec::set_converted`, 157 instances, 54 kB, is gone).
- *The failures compiled once*: the panics of the direct accessors, of the registry look-up of a
  direct property and of the registration of a styled property are cold functions compiled once,
  and the setter of a direct property is called from one place.

Two lines of the gate of the first part were fixed in it: the read of an inherited value converts
the stored value where it reads it (as main did), `TextBlock::set_font_size` is pinned as a call
of its own (inlined into a caller, the caller copied the text block it had just made,
`trees::Text.create_and_measure`), and the storage metadata of an enumeration is held as what it
is, so that the store reads it without a call through a vtable. The rest of
`ControlWrite.enum_style_priority_set_and_dispose` (+1.65 % instructions) is the static member
looked up for the two values of a change and of an unset, and the storage metadata of a new
effective value.

| Release, macOS ARM64, stripped | Before | After the stack |
|---|---:|---:|
| `hello-world-builders` | 20,610,240 B | 19,800,832 B (-0.81 MB, -3.9 %) |
| `hello-world` (XAML) | 20,529,096 B | 19,719,208 B (-0.81 MB, -3.9 %) |
| `winui-gallery` | 57,201,768 B | 55,175,480 B (-2.03 MB, -3.5 %) |
| `control-catalog-desktop` | 75,508,096 B | 73,252,880 B (-2.26 MB, -3.0 %) |

The gallery and the catalog are measured against main at the base of the stack (a3b7a45bf), the
hello worlds against the base of the first part. The gallery is built with its feature `desktop`
(`cargo build --release -p winui-gallery --features winui-gallery/desktop --bin winui-gallery`):
without it, cargo builds the library alone and the binary in the target folder is whatever was
built there last.

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
  Section 4.4 moves the parts that do not depend on the value type out of these families.
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

### 5.4 Hello world by mechanism (macOS, release, after #565)

Machine code of `samples/HelloWorld` (the WinUI theme, compiled markup), 43.2 MB, from the
symbol table. Sizes are the distance to the next symbol; instances are the functions of each
family. These are the .NET mechanisms whose instances add up. The porting guide turns each into
a rule ([PORTING-GUIDE.md](porting/PORTING-GUIDE.md), "Binary size: what a port costs").

| Mechanism | Size | Instances | Where | Ticket |
|---|---:|---:|---|---|
| Compiled markup of the WinUI theme | 7.28 MB | 4,851 | `ferroui_themes_winui` | #568 |
| Closures: `FnOnce::call_once` bodies and their shims | 2.69 MB | 20,368 | registrations, callbacks, `Disposable::create` | #569 |
| Drop glue (`drop_in_place`) | 2.86 MB | 14,754 | cells, binding expressions, entries, `Rc` | #569 |
| `IObserver<T>` per value type (`UntypedObserver`, `Observer`, `DirectUntyped`, animation observers) | 1.43 MB | 2,477 | property store, bindings, animations | #576 |
| Class vtables per class and subclass (`Subclassable::build_vtable`) | 1.39 MB | 1,146 | `Control`, `StyledElement`, `Visual`, `Layoutable` | #569 |
| Property routes per value type (`PropertyRoutes` of `StyledProperty<T>`, `DirectProperty<T>`) | 1.08 MB | 5,511 | property core | #576 |
| Value entries per value type (`EffectiveValueDyn`, `IValueEntry`) | 0.76 MB | 4,025 | property store | #576 |
| `AnyValue` per value type | 0.58 MB | 12,063 | every property value type | #576 |
| Nullable operations, conversions, casts and object handles registered per type | 1.70 MB | 4,700 | value type registry | #566 |
| Change notification by name (`INotifyPropertyChanged`) | 0.47 MB | 752 | view models, controls | |
| Markup metadata closures | 0.40 MB | 1,489 | `MarkupType` members | #566 |
| Class registration and static constructors | 0.38 MB | 1,326 | `register_class` | #566 |
| `IDisposable` adapters and subscriptions | 0.34 MB | 1,451 | `AnonymousDisposable`, guards | #569 |
| CLR-style property info (`TypedClrPropertyInfo`) | 0.27 MB | 822 | binding paths | #576 |
| Routed events | 0.25 MB | 930 | `Interactive` | |
| Rx operators | 0.19 MB | 426 | `ObservableExt` | #576 |

Unwind tables and landing pads (`__eh_frame`, `__gcc_except_tab`, `__unwind_info`) are 10.3 MB on
top of the machine code (#567).

### 5.5 Hello world by mechanism on main after 0.5.0 (macOS, release, 2026-11-15)

The same executable after the changes of sections 4.5 to 4.16 (`scripts/symsize.py`, unstripped
release): the file is 31.18 MB, and 21.85 MB of it are attributed to symbols (18.65 MB of machine
code, 2.85 MB of constants).

| Origin | Size | Share |
|---|---:|---:|
| `ferroui_base` | 8.14 MB | 37.3 % |
| Skia and the other C, C++ and Objective-C code | 4.70 MB | 21.5 % |
| `core` | 1.91 MB | 8.7 % |
| Anonymous data | 1.86 MB | 8.5 % |
| `ferroui_controls` | 1.35 MB | 6.2 % |
| `ferroui_themes_winui` (the compiled markup that is left after trimming) | 0.98 MB | 4.5 % |
| `alloc` | 0.80 MB | 3.7 % |
| `std` | 0.31 MB | 1.4 % |
| `ferroui_native`, `ferroui_markup_xaml`, `ferroui_skia`, `hashbrown`, `smallvec` | 1.08 MB | 4.9 % |

The mechanisms of section 5.4 in the 400 largest families of functions (10.96 MB):

| Mechanism | Size | Instances | After #565 (section 5.4) |
|---|---:|---:|---:|
| Property code per value type: `EffectiveValue<T>`, the generic paths of `ValueStore`, the routes of `StyledProperty<T>` and `DirectProperty<T>`, `MetadataTable<T>` | 2.81 MB | 7,769 | 1.84 MB of routes and entries, counted more narrowly |
| Drop glue (`drop_in_place`, `Rc::drop_slow`) | 0.93 MB | 4,903 | 2.86 MB |
| Closures: `FnOnce::call_once` bodies and their shims | 0.76 MB | 8,606 | 2.69 MB |
| Nullable operations, conversions and casts per type | 0.55 MB | 2,542 | 1.70 MB |
| `AnyValue` per value type (`any_value_eq`, `type_id`) | 0.32 MB | 5,215 | 0.58 MB |
| Observers per value type | 0.30 MB | 435 | 1.43 MB |
| Class registration and vtables | 0.15 MB | 500 | 1.77 MB |
| `IDisposable` adapters and subscriptions | 0.14 MB | 728 | 0.34 MB |

The property code per value type is now the largest mechanism: 159 value types reach hello world,
about 18 kB of code each. Its largest families:

| Family | Size | Instances |
|---|---:|---:|
| `EffectiveValue<T>::set_and_raise_core` (1.2 to 4.0 kB a copy) | 0.44 MB | 232 |
| `ValueStore::set_value`, `get_value`, `get_base_value`, `inherited_or_default`, `create_effective_value` | 0.44 MB | 149 to 233 each |
| The routes of `StyledProperty<T>` (`route_get_value`, `route_set_value`, `route_set_current_value`, `route_set_local_value`, `route_copy_value` and nine smaller ones) | 0.60 MB | 159 each |
| `EffectiveValueDyn` of `EffectiveValue<T>` (`raise_unset`, `apply_entry`, `remove_animation_and_raise`, `boxed_value`), `new`, the two notifications | 0.42 MB | 159 each |
| `MetadataTable<T>::get` and `unregister` | 0.21 MB | 197 to 256 |

What the smallest copy of `set_and_raise_core` is made of (290 instructions): nine checks of a
borrow with their panic calls, seven places where a copy of the value is dropped, and the handling
of a coerced value, which every copy carries though few properties have a coercion callback.
`MetadataTable<T>::unregister` is reached only through the route of
`FerroPropertyRegistry::unregister_by_module`, which no application calls, and is kept alive by the
property's table of routes.

Section 4.17 took the handling of a coerced value out of the set of every value type and the copy
of the previous value out of every set. What is left of this mechanism is thin typed code in many
small functions (the routes, the entries, the generic paths of `ValueStore`), 159 times: less of it
needs either fewer value types reaching an application, which is what the trimming of sections 4.11
to 4.16 does, or one untyped core under the typed surface, which is a redesign of the property
store and not a step.

### 5.6 Generic instances by family (#986, 2026-11-20)

`scripts/generic_instances.py` reads a link map or an unstripped binary and groups the symbols of
one generic function (one path, different instance hashes in the legacy mangling) into a family;
a family of two or more instances is generic code. Counted on the builders hello world (release,
unstripped, a build of 2026-11-18) and on the statically linked C hello world over the whole
framework (`samples/CApi/gauge.c`, release, `-Wl,-dead_strip`, the tables of every type, #976):

| | Rust code | In families of 2+ instances | Share |
|---|---:|---:|---:|
| `hello-world-builders` | 10.61 MB | 6.31 MB | 59.5 % |
| `gauge.c`, static, every type | 24.95 MB | 14.57 MB | 58.4 % |

By crate (hello world): `ferroui_base` 3.97 of 6.14 MB generic, `core` 1.28 of 1.55, `alloc` 0.38
of 0.39, `hashbrown` 0.17 of 0.17, `ferroui_controls` 0.05 of 0.76 (its classes are not generic).

The largest families of the hello world:

| Family | Instances | Size |
|---|---:|---:|
| Closures: `FnOnce::call_once` and its vtable shims | 4,449 + 1,334 | 0.61 MB |
| Property code per value type (`set_and_raise_core`, `ValueStore::set_value`/`get_value`/`get_base_value`/`inherited_or_default`, the `StyledProperty<T>` routes, `EffectiveValue<T>` entries), 129 to 175 value types | about 2,000 | about 1.5 MB |
| `AnyValue::any_value_eq` and `Any::type_id` per type | 1,113 + 3,223 | 0.26 MB |
| `nullable_operation` per value type | 380 | 0.16 MB |
| `Rc::drop_slow` | 554 | 0.13 MB |
| Value conversions (`TypedConvert` conversion and cast adapters) | 455 + 218 | 0.13 MB |
| Animation per animator type (`AnimationInstance<A>`, its observer and interpolation) | 28 each | about 0.17 MB |
| `hashbrown::RawTable::reserve_rehash` | 85 | 0.12 MB |
| Stable sorts (`quicksort`, `drift::sort`) | 37 each | 0.11 MB |
| `LocalKey::with` | 109 | 0.05 MB |

With every type registered (the C program) the same families grow with the number of types:
closures 25,107 instances (1.80 MB), the property code about 297 value types (more than 3 MB),
`any_value_eq` 2,855 instances (0.51 MB), `nullable_operation` 1,110 (0.47 MB), typed binding
expressions 120 instances of about ten functions (0.6 MB).

The candidates of #986, in the order of the count: the property code per value type (the untyped
core of section 5.5), closures (generic over a closure only on hot paths), the per-type
`AnyValue`/`nullable_operation`/conversion functions (made only for the types that need them),
animation per animator type, and the `core`/`alloc` generics (one sort and one table per element
size). Each step is measured by size and by the local gate (section 8.4); the shared setter and
handler functions of the builders (#956) are the example of a step left out: 40 to 50 kB for
+3.9 % instructions on a re-run.

## 6. The settings applications should use

- `cargo run --profile fast-release -p <application>` to try an application with optimised code:
  the release optimisation level without link-time optimisation, so the final link takes seconds
  instead of minutes. Its executables are somewhat larger and slower than release; benchmarks use
  release, and shipping uses `dist`.
- `cargo build --profile dist` for what you ship. **Profiles do not reach an application.** Cargo
  reads the `[profile.*]` sections of the root manifest of the workspace that is built, and of no
  other: an application that depends on the framework from crates.io, from a git dependency or
  from a path into a checkout of this repository builds with its own profiles, and gets neither
  `dist`, nor the abort on panic of `release` and `dist`, nor the per-crate levels below, unless its
  own root `Cargo.toml` states them. An application in a repository of its own that has this one as
  a submodule copies every `[profile.*]` section with `scripts/apps/sync-workspace.py`
  (docs/apps.md, section 2); any other application copies them by hand:

  ```toml
  [profile.release]
  lto = "thin"
  panic = "abort"

  [profile.dist]
  inherits = "release"
  lto = "fat"
  codegen-units = 1
  strip = "symbols"
  panic = "abort"

  # Compiled theme markup runs once: the theme crates it uses at "z" (section 4.14, #581).
  [profile.dist.package.ferroui-themes-winui]
  opt-level = "z"

  [profile.dist.package.ferroui-themes-fluent]
  opt-level = "z"
  ```

  A per-crate section names a package of the dependency graph; cargo warns about one the
  application does not use, and the warning is harmless. `release-unwind` and `dist-unwind`
  (`inherits` and `panic = "unwind"`) are for a program that catches panics to carry on, such as
  a plug-in host. The `dist` build takes about four times as long as the release profile and its
  link step needs about 6 GB of memory. Keep the unstripped executable of a release if you need
  symbolised backtraces from the field (on Windows keep the PDB). The mobile packages take the
  profile as an option: `scripts/android/apk.sh --profile dist` and `scripts/ios/bundle.sh
  --profile dist`.
- Depend only on the themes the application uses. A theme declared in a document, or registered in
  `register_types()`, is linked; a theme that is not named is not.
- Keep theme trimming on (the default, section 4.11), leave the developer tools and the run-time
  loader out of the shipped build (they keep every unit), and
  name in `theme-roots` the controls and keys whose names the application builds at run time.
- Keep `panic = "abort"` in `release` and `dist` (section 4.7): no path of the framework relies on
  catching a panic; a panic runs the panic hook (install one for crash reports) and ends the
  process.
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
executable of the catalog (section 3.3). Skia uses its Ganesh backend on that OpenGL context with the GPU resource cache limited to 128 MiB (`SkiaOptions`; upstream sets 29.5 MB, which
evicted the textures of images that scrolled out of view: docs/porting/gpu-rendering.md).

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
