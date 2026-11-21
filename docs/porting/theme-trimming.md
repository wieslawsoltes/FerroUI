# Theme trimming and compact theme code (#568)

Design note, 2026-11-06. Steps 1 to 6 of the plan are implemented (sections 8 to 13). Counts are taken from the checked-in
`compiled_xaml.rs` of the themes at `main` 72cd992f8 (the build writes the same text into
`OUT_DIR`), sizes from `docs/binary-size.md` (section 5.4) and from the measurements recorded with
PR #496 (`docs/benchmarks/compiled-theme-linux-x86_64.json` on that branch).

## 1. Summary

- **Why everything is linked.** The WinUI theme builds one dictionary that holds every control theme
  and every theme resource as deferred content. Each entry is created from the address of its
  generated build function (`rt::defer(.., build_controls_winuicontrols_xaml_deferred_13, ..)`,
  `rt::add_deferred_resource(.., populate_winuitheme_xaml_deferred_97, ..)`). Taking the address is a
  reference, so the linker keeps all 6,226 build functions, the templates they reach, and the control
  code those templates construct. Which entry is used is decided at run time by a key
  (`ResourceKey::Type(style_key)`), which no linker or LTO pass can see. `register_types()` alone
  already roots all of it, through the URI loader `compiled_xaml::try_load`.
- **What PR #496 shows.** Rooting the theme at `Button` alone took the generated machine code of the
  theme from 7.83 MB to 0.81 MB and the probe executable from 71.1 MB to 60.1 MB (Linux x86-64,
  release, no LTO): about 4 MB of control code that only the templates reached went with it. Table
  codegen without binary XAML took the full theme from 7.83 MB to 5.92 MB of machine code. Binary
  XAML added nothing over the Rust tables (5.94 MB). The roots in #496 come from an environment
  variable read by the theme's own build, which gives one subset per cargo invocation, not one per
  application.
- **Recommendation.** Compile each theme once, whole, but emit it as units (one function or static
  table per control theme and per group of resources) with no reference from the theme's
  markup-construction path to all units. The application's build (`ferroui-build`, which already
  compiles its markup) computes the reachable units from its documents, its sources, the type models
  and explicit roots. It writes a static table that references only those units. Compiled markup that
  constructs `<WinUITheme />` passes that table to the constructor. The linker then drops the rest.
  `WinUITheme::new()` called from Rust code, and every build with `ferroui-base/type-tables` (the
  run-time loader, hot reload, the designer, DevTools), keep the full theme. Separately, lower the
  cold, repetitive shapes to typed tables: 95 % of the 5,695 theme resource factories have four
  shapes, and ControlTheme construction (selectors, nested styles, setters) runs once per theme.
  Templates, which run once per control instance, stay native code.

## 2. Today's mechanism

### 2.1 Build

- `src/FerroUI.Themes.WinUI/build.rs` collects every `.xaml` of the crate except those listed in
  `Controls/excluded.txt`. It compiles them as one group with `ferroui_build::Build::from_env()`,
  `TypeSystem::Model` and `XamlGroup::new("compiled_xaml").class_document("WinUITheme.xaml")
  .constructor(ClassConstructor::ServiceProvider("with_service_provider"))`. The output goes to
  `$OUT_DIR/xaml/compiled_xaml.rs`, with `compiled_xaml.xamlmeta` beside it.
- `src/FerroUI.Build.Tasks/lib.rs` (`Build`, `XamlGroup` at line 216, `compile_group` at 487) runs
  the transform and the emitter. It exports the `.xamlmeta` to dependent build scripts through the
  `links` key (`cargo::metadata=xamlmeta=..`, line 894). An application's build script therefore
  already sees `DEP_FERROUI_THEMES_WINUI_XAML_XAMLMETA`. Today that file lists only three documents
  with their Rust paths.
- `lib.rs` includes the output with `ferroui_markup_xaml::include_compiled_xaml!()`.
- `tests/compiled_xaml_tests.rs` holds the three texts together. `compiled_xaml_is_up_to_date`
  checks that the emitter, run against the run-time type system, writes the checked-in
  `compiled_xaml.rs` and `.xamlmeta` byte for byte. `build_script_output_is_the_checked_in_reference`
  checks that the build script, run against the type models, writes the same text. Fluent, Simple and
  macOS have the same pair.
- The emitter is `src/Markup/FerroUI.Markup.Xaml.Loader/rust_emitter/`. `emitter.rs` writes one
  function per piece of deferred content (`{function}_deferred_{slot}`, line 2772) and splits large
  dictionary bodies into `{function}_part_{n}` (line 4865). `compiled.rs` writes the file, the
  loader table `try_load` and the `.xamlmeta`.

### 2.2 What the generated file contains

`src/FerroUI.Themes.WinUI/compiled_xaml.rs`: 153,387 lines, 6,265 functions. The issue measures
7.28 MB of machine code in 4,851 functions in hello world (inlining merges some).

| Family | Functions | Lines | What it is |
|---|---:|---:|---|
| `populate_winuitheme_xaml_deferred_*` | 5,695 | 64,307 | one build function per deferred theme resource (brushes, aliases, geometries), across the Default, Light, Dark and HighContrast dictionaries |
| `build_controls_winuicontrols_xaml_deferred_*` | 531 | 71,924 | 277 ControlTheme factories (22,882 code lines), 244 template contents (25,577), 10 other |
| `populate_winuitheme_xaml_part_*` | 33 | 15,771 | the bodies of the theme dictionaries: plain values added directly, and the `add_deferred_resource` calls |
| `build_controls_winuicontrols_xaml` | 1 | 769 | the dictionary of control themes: 103 type-keyed entries (`AddDeferred_1`) and 185 text-keyed ones |
| the rest | 5 | 524 | `populate`, `populate_winuitheme_xaml`, compact density styles, `try_load` |

Fluent (`src/FerroUI.Themes.Fluent/compiled_xaml.rs`, 58,002 lines) has the same structure:
1,371 resource factories, 402 control-family functions (121 ControlTheme factories, 141 template
contents), 9 populate parts.

### 2.3 Run time

1. `WinUITheme::with_service_provider` (`winui_theme.rs`, line 126) calls `compiled_xaml::populate`,
   then `populate_winuitheme_xaml`. That function creates the root `ResourceDictionary`, the theme
   dictionaries of the four variants (each filled by its ten `_part_` functions), and the merged
   dictionaries, among them `build_controls_winuicontrols_xaml`.
2. A deferred entry is made by `rt::add_deferred_resource(dictionary, key, type, &context, build,
   line, column)` or by `rt::defer(type, &context, build, ..)` followed by
   `ResourceDictionary::__markup_AddDeferred_1(dictionary, rt::boxed(<X as StaticType>::TYPE), ..)`
   (`src/Markup/FerroUI.Markup.Xaml/xaml_il/runtime/compiled.rs`, lines 456 and 1090). `defer`
   wraps the `DeferredBuild` function pointer (line 1085) in a `DeferredContentBuilder`
   (`xaml_il_runtime_helpers.rs`, line 42). `ResourceDictionary::add_deferred`
   (`src/FerroUI.Base/controls/resource_dictionary.rs`, line 400) stores it as
   `ResourceItem::Deferred`, built on first lookup. Every entry also allocates an
   `Rc<DeferredContent>` and captures the context: 5,880 such sites run at start-up.
3. A control finds its theme in `StyledElement::get_effective_theme`
   (`src/FerroUI.Base/styled_element.rs`, line 884). Unless `Theme` is set, it looks up
   `ResourceKey::Type(self.style_key())` with `try_find_resource` and caches the result. The style
   key is the control's own type unless `style_key_override` names another; base types are not
   searched. The first lookup builds the ControlTheme factory, once per dictionary. The template in
   the theme then runs its native content builder once for each control instance.

### 2.4 Why every factory is reachable

The linker keeps a function when a live function or live data refers to it, by a call or by its
address. Here:

- `register_types()` (`register_types.rs`) calls
  `FerroXamlLoader::register_compiled_xaml(ASSEMBLY.name, compiled_xaml::try_load)`. That stores
  `try_load` in a global table, so `try_load` is live, and it calls
  `WinUITheme::with_service_provider`. An application that only calls
  `ferroui_themes_winui::register_types()` therefore links the whole theme.
- `populate_winuitheme_xaml` and its parts pass the address of each of the 5,695 resource factories
  to `add_deferred_resource`. `build_controls_winuicontrols_xaml` passes the address of each control
  theme factory. Each factory passes the address of its template contents to `rt::defer`.
- The template contents call the constructors of the controls they build
  (`::ferroui_controls::CalendarView::new()` and so on). With them come those controls' code, their
  property registrations and their own templates.

The choice of entry is data, made at run time by a `TypeInfo` key. Nothing in the call graph says
that a `CalendarView` theme is used only when some code constructs a `CalendarView`. LTO and
dead-code elimination work on the call graph, so they cannot remove any entry. The only levers are
to not emit the reference, or to emit it only in code that the application itself links.

## 3. What PR #496 offers

### 3.1 Application resource roots

`src/FerroUI.Build.Tasks/resource_roots.rs` (379 lines with tests) and
`XamlGroup::resource_roots(&[..])` with `trim_resource_documents` (`lib.rs`):

- It parses each document of the group with `roxmltree`. Every direct, explicitly keyed child of a
  `ResourceDictionary` is a removable entry, under its text key or under the local type name of an
  `{x:Type X}` key. Entries without a key, and entries that register a name in the outer scope, are
  ambient and always kept.
- The dependencies of an entry are every identifier token in its text, attribute values and tag
  names that names a key of the group. This is conservative: `TargetType="Button"` or a selector
  `Button` keeps the Button theme. `Source` paths of includes and `Design.PreviewWith` are ignored.
- From the roots and the ambient entries it walks the graph and keeps *every* definition of a live
  name: overrides and the four theme variants. Unknown roots are an error.
- Dead entries are blanked in place, keeping UTF-16 positions for diagnostics. A DTD in any document
  disables trimming for the group.
- The WinUI build reads the roots from `FERROUI_WINUI_THEME_ROOTS` (comma-separated) and always adds
  `CompactStyles`.

Measured in #496 with a probe that constructs `WinUITheme` and looks up the Button theme (Linux
x86-64, release, no cross-crate LTO, 2026-11-02; MB is 10^6 bytes):

| Case | Generated Rust | Machine code of `compiled_xaml` | Stripped probe | Theme creation, median |
|---|---:|---:|---:|---:|
| `main` at the time | 14.42 MB | 7.83 MB, 4,736 symbols | 71.12 MB | 11.2 ms |
| table codegen, no binary XAML | 9.23 MB | 5.92 MB, 784 symbols | 69.29 MB | 10.5 ms |
| table codegen with binary resources | 8.75 MB | 5.94 MB, 784 symbols (plus 0.18 MB of data files) | 69.00 MB | 11.9 ms |
| roots `Button`, table codegen | 1.22 MB | 0.81 MB, 107 symbols | 60.14 MB | 3.5 ms |
| roots `Button`, binary resources | 1.17 MB | 0.81 MB | 60.12 MB | 4.4 ms |

Two conclusions: trimming is worth an order of magnitude more than any codegen change, and binary
resources bring nothing over Rust tables. The second supports the decision not to take binary XAML.

### 3.2 Compact codegen

These emitter passes in `rust_emitter/` and runtime modules in
`src/Markup/FerroUI.Markup.Xaml/xaml_il/runtime/compiled/` do not reference the binary crate:

| Part | Files | What it does |
|---|---|---|
| Theme resource rows | `theme_tables.rs`, `theme_tables/constants.rs`; runtime `theme_resources.rs` | Runs of deferred dictionary additions of one shape become a static row table and one shared builder per shape: `aliases`, `literal_brushes`, `dynamic_brushes`, `static_brushes`, `deferred_factories`, and the constants `doubles`, `thicknesses`, `corner_radii`, `colors`. A run ends at any other statement, dictionary, scope or factory. Runs shorter than 8 stay direct calls. WinUI: 5,695 resource factories become 271. |
| Factory shapes | `factory_shapes.rs` | Interns deferred factories that differ only in literal, type and child-factory operands. A small thunk per site passes the operands to one shared body. |
| Selector plans | `selector_plans.rs` | Interns linear selector chains by their operation sequence. Strings and types stay at the use site. |
| Style programs and setters | `style_setters.rs`; runtime `style_program.rs` | `begin`, `attach` and `finish_dynamic` helpers for nested styles and runs of dynamic-resource setters. |
| Template bindings and values | `template_bindings.rs`, `template_values.rs` | Consecutive TemplateBindings on one receiver become a slice of property pairs. Repeated closed template setters are shared. |
| StaticResource assignments | `resource_assignments.rs` | The run-time choice of setter for `{StaticResource}` becomes an operation with a typed overload plan. |
| ControlTheme programs | `control_theme_programs.rs`, `control_theme_programs/regions.rs`; runtime `control_theme_program.rs`, `style_region.rs` | Lowers a complete ControlTheme factory, or a region of nested styles of at least 16 statements, to an instruction list with per-factory imports and no global opcode table. Locals live in a caller-owned array in reverse declaration order, so drop order matches. In WinUI only 18 whole factories and 34 regions were accepted. The measured saving of construction, metadata and regions together was 167 kB of `.text`, partly offset by data and unwind tables. |
| Imports | `imports.rs` | Shortens long paths with per-module `use` lines. This affects text size only, not machine code. |

`theme_tables.rs` and `theme_tables/constants.rs` import `Kind` and `Value` from
`ferroui_xaml_binary` only to name shapes and operands, and branch to `binary_resources` when it is
enabled. The non-binary path is self-contained and would be rewritten without those types.

Every lowering pass in #496 re-reads the Rust text the emitter has just written. For example,
`template_bindings.rs` matches lines that start with `    rt::apply_template_binding(`, and
`control_theme_programs.rs` validates a closed grammar of emitted statements. That is how they stay
correct without touching the emitter, but it is fragile: any change to the emitter's formatting
silently turns a lowering off. Re-implemented here, the same decisions belong in the emitter, at the
node it is about to write, where the shape is known exactly.

### 3.3 Taken and not taken

| Taken (re-implemented) | Not taken |
|---|---|
| The key graph of `resource_roots.rs`: keyed entries, ambient entries, every definition of a live key, conservative token references, its tests | Delivery of roots by an environment variable to the theme's own build (one subset per invocation, rebuilds of the theme crate, reference tests that fail when it is set) |
| Theme resource rows and constant rows, one builder per shape | `ferroui-xaml-binary`, `binary_document/`, `binary_files.rs`, `binary_resources.rs`, `compiled_archive.rs`, `archive_includes.rs`, `operand_update.rs`, the `ferroui-fxd` tool, the theme features `binary-resources`, `packed-theme`, `solid-theme`, `portable-solid-theme`, `precompiled-theme`, `compiled-updates`, the binary themes of hot reload |
| Selector interning, style and setter helpers, template binding runs, the StaticResource assignment plan | The `Kind`/`Value` types of the binary crate in `theme_tables` |
| The ControlTheme program format: per-factory imports, no global dispatch, caller-owned locals in reverse declaration order | Lowering by re-parsing emitted text |
| The size and startup measurements as the baseline to beat | The post-link unwind rewriting tool; the property-core, nullable and coercion work, which belongs to #566, #569 and #576 |

## 4. Options for application-level trimming

The questions for each option: what is rooted and how; what it costs the build; what happens to the
catalog and the galleries, which use nearly every control; what happens with dynamic loading (the
run-time loader, hot reload, the designer, `ferroui-base/type-tables`); and whether the byte-for-byte
reference tests of the theme crates still hold.

### (a) The application's build compiles its own theme subset

The application's build script computes roots (the types its markup and code construct, the keys
they name, explicit roots from `[package.metadata]`). It trims the theme's documents with the key
graph of 3.1 and compiles the subset into the application crate. The theme's whole compiled markup
must then be neither referenced nor compiled (a feature of the theme crate), and the class
`WinUITheme` has to populate itself from the application's subset (for example through
`FerroXamlLoader::populate_override`).

- **Rooted:** at the source level, before compilation. Finest granularity (per key).
- **Build:** each application transforms and emits up to 169 documents, then rustc compiles the
  generated Rust in the application crate: about 150,000 lines for an application that uses
  everything. A workspace build pays this once per application. The catalog and the gallery would
  compile the theme twice (once in the theme crate, unless a feature turns it off, and once in the
  application).
- **Catalog and gallery:** no size gain, slower builds.
- **Dynamic loading:** must compile the full set; detecting that from the build script needs the
  same metadata as option (d).
- **Reference tests:** the theme's reference is unchanged, but the subset is generated in another
  crate. Paths change (`crate::accents::..` becomes `::ferroui_themes_winui::accents::..`), and
  internal items of the theme must become public. No reference covers that output.
- **Variant with roots in the theme's build (#496):** compiles once per root set, but one set per
  cargo invocation and per target directory, not per application. Changing it rebuilds the theme, and
  the reference tests fail while it is set.

### (b) One factory per control theme, registered lazily by a key only construction references

The idea: the theme emits one factory per control theme and registers it under a key, so that the
factory is linked only when code that constructs the control is linked. What Rust allows:

- **No reverse references.** `ferroui-controls` cannot name `ferroui-themes-winui`. The constructor of
  `Button` cannot refer to any theme's factory, and an application may hold several themes and switch
  between them at run time (the catalog does).
- **Distributed slices root everything.** `linkme` places each element in a section that is kept as
  a whole (`#[used]`, `KEEP`), and `inventory` runs constructors before `main`. Both make every
  registered factory live.
- **No conditional retention in stable Rust.** "Keep A only if B is kept" exists in linkers (ELF
  `SHF_LINK_ORDER` with LLVM `!associated` metadata, COFF associative COMDATs, Mach-O live-support
  sections), but Rust has no stable way to emit it, and the three formats differ. Weak linkage
  (`#[linkage = "extern_weak"]`) is unstable.
- **Monomorphisation.** A generic construction path (`Button::new_in::<WinUI>()`) whose
  instantiation names `<Button as Themed<WinUI>>::FACTORY` would work, but only if every
  construction site names the theme as a type parameter. That changes the public API, which must
  stay .NET-shaped, and the theme is not known where code constructs controls.

What survives of (b) is the one case where the constructing code *does* know the theme: the
application's own compiled markup and build. That is option (d).

### (c) Cargo features per control group

`ferroui-themes-winui/{foundation, buttons, text, date-time, navigation, collections, ..}`. Each
gates its documents in `build.rs` (or `#[cfg]` on the generated units), and features depend on each
other by hand (`date-time` needs `buttons`, `popups`, `text`).

- **Rooted:** by the features the application enables. Coarse, and kept up to date by hand.
- **Build:** one theme compile per feature set. Feature unification makes a workspace build of the
  catalog and hello world give both the union.
- **Catalog and gallery:** enable everything.
- **Dynamic loading:** enable everything.
- **Reference tests:** one reference for the full set. Every partial set needs its own build in CI to
  stay correct, and a wrong set fails silently at run time (a control without a theme draws
  nothing).
- Users have to know the group of each control, including the ones templates construct (a
  `ComboBox` needs `Popup`, `ScrollViewer`, `ToggleButton`). Not recommended except as an escape
  hatch.

### (d) Recommended: one compile, application root tables, the linker trims

The theme crate compiles its documents once, as today. The emitter partitions the output:

- **Units.** Each control document (`Controls/Button.xaml`: its ControlThemes, text-keyed or
  type-keyed, and its local resources) is a unit. The theme resources are grouped into units by the
  key graph. The ambient content is one unit that is always kept: the global `Style` elements of
  `WinUIControls.xaml`, the hosts the framework creates (`Window`, `AdornerLayer`,
  `EmbeddableControlRoot`, `ThemeVariantScope`; the hosts of popups since section 25),
  `CompactStyles`, and
  `SystemAccentColors`. Each unit is one function, or one set of static row tables per variant
  dictionary, that adds its entries to the dictionaries it is given.
- **Graph in the `.xamlmeta`.** For each unit: the keys and type keys it defines, the keys it
  references (StaticResource, DynamicResource, BasedOn, `Theme=`), and the types it constructs or
  selects (TargetType, selectors, template content). The `.xamlmeta` is already exported to
  dependent build scripts.
- **Entry points.** `WinUITheme::with_parts(sp, parts: &'static [&'static ThemeUnit])` applies the
  given units in canonical order. `ThemeUnit` holds a name, a canonical index and the function or row
  tables. `WinUITheme::new()` and `with_service_provider()` keep their meaning, the full theme, and
  are the only references to the static table of all units. `try_load` (the URI loader that
  `register_types` registers) applies the units the application registered and refers to all units
  only under a cfg that the theme's build sets when `ferroui-base` is built with `full-themes`
  (`type-tables` until 11.7). `ferroui-base` already has `links = "ferroui_base_xaml"`: its build
  script can print `cargo::metadata=full_themes=1`, so dependent build scripts see
  `DEP_FERROUI_BASE_XAML_FULL_THEMES`.
- **Application side.** When a compiled document constructs a class whose `.xamlmeta` has units
  (`<WinUITheme />` in `App.xaml`), the emitter writes
  `WinUITheme::with_parts(sp, &crate::compiled_markup::THEME_PARTS_FERROUI_THEMES_WINUI)`. After all
  groups are compiled, `ferroui-build` computes the closure and writes that static into
  `$OUT_DIR/xaml/mod.rs`. It lists only the needed units, in canonical order. The roots are:
  - the types the application's compiled documents construct, select or name in `{x:Type}`, and
    their base classes (the application's type model gives them; this covers subclasses without a
    style key override);
  - the keys its documents reference;
  - type names and string literals in the application's Rust sources that match the theme's type
    keys and keys (`ferroui-build-scan` already reads those sources);
  - "creates" edges of the type models: the control types that a class's code constructs, for
    example `ListBox` creates `ListBoxItem` containers and `TextBox` creates `TextSelectionHandle`;
  - explicit roots in `[package.metadata.ferroui] theme-roots = ["AccentButtonStyle",
    "CalendarView"]`, or `theme-roots = "all"`;
  - the keys and types the compiled documents of the dependencies name (11.7);
  - everything, when `DEP_FERROUI_BASE_XAML_FULL_THEMES` is set.
- **Linker.** Unit functions and tables that nothing references are removed by section garbage
  collection (`-dead_strip`, `--gc-sections`, `/OPT:REF`). Rust emits function sections by default,
  so this works without LTO. Removing a unit also removes the controls that only its templates
  construct. That is where the extra 4 MB of the #496 Button probe came from.
- **Diagnostics.** In debug builds, the theme adds a deferred entry for each type key that was left
  out. When built, it logs `the WinUI control theme of CalendarView was trimmed: add it to
  theme-roots` and yields nothing, so the control behaves as it does today without a theme. Release
  builds carry no names.

Evaluation:

- **Rooted:** by references that the application's own crate emits, decided per application. A
  workspace build of several applications gives each its own table.
- **Build:** the theme crate compiles once, unchanged in cost. The application's build adds a graph
  walk over data it already reads, which takes milliseconds.
- **Catalog and gallery:** computed roots, which come close to all units. Comparing their screenshots
  with `theme-roots = "all"` is the strongest test of the root computation.
- **Dynamic loading:** the full table whenever `type-tables` is on. That is the same switch #566
  uses for lookups by name, and it already covers the run-time loader, hot reload, the designer,
  DevTools, Inspect and the declarative View API.
- **Reference tests:** the theme's output no longer depends on any application, so both reference
  tests hold. The checked-in `.xamlmeta` gains the unit graph and is compared with the build as
  today. The root computation is tested in `ferroui-build` with fixtures. Test builds of an
  application whose dev-dependencies enable the run-time loader get the full theme, so the trimmed
  configuration is checked by release runs (screenshots, sizes), not by unit tests.
- **Compatibility:** applications that construct the theme in Rust (`samples/HelloWorld.Builders`,
  `.Hooks`, `.Macros`) keep the full theme until they pass a table. A small macro or build helper can
  produce one from a list of controls.

### Comparison

| | (a) subset in the application | (a) roots in the theme's build | (b) lazy keys | (c) features | (d) root tables |
|---|---|---|---|---|---|
| Per application | yes | no | not feasible | no (unified) | yes |
| Theme compiled | once per application, plus once | once per root set | | once per feature set | once |
| Automatic roots | possible | no | | no | yes |
| Catalog and gallery | slower build, no gain | no gain | | no gain | no gain, validates roots |
| Dynamic loading | full subset needed | env must be unset | | all features | automatic, through `type-tables` |
| Theme reference tests | unchanged, subset untested | fail when set | | one per feature set | unchanged, plus the unit graph |

## 5. Compact codegen: what the generated code repeats

All counts are from the WinUI file at 72cd992f8 unless Fluent is named. "Lines" are generated lines.
Machine-code shares need the symbol attribution of step 1 of the plan.

### 5.1 Theme resource factories (5,695 functions, 42 % of the lines)

41 distinct shapes, if literals and positions are ignored. Four of them cover 5,424 factories
(95.2 %):

| Shape | Factories | Lines | Body |
|---|---:|---:|---|
| Alias (`{StaticResource Key}` alone) | 3,634 | 21,804 | one call of `rt::static_resource_content` |
| Brush with a literal colour | 738 | 8,118 | `deferred_context`, `SolidColorBrush::new()`, `set_value(color, Color::from_uint32(..))` |
| Brush bound to a dynamic resource | 714 | 8,568 | `deferred_context`, `new`, `push_parent`, `rt::bind_dynamic_resource`, `pop_parent` |
| Brush whose colour is a StaticResource | 338 | 7,774 | 21 lines: create the extension, `provide_value_invoked`, then the run-time choice of setter (`is_instance` for unset, binding, `Color`, else `no_setter`) |

The rest include 56 acrylic brushes with two StaticResource colours (34 lines each) and 45
`StreamGeometry::__markup_parse` geometries. Fluent: 1,371 factories in 17 shapes, of which 1,021
(74 %) are aliases and 162 are StaticResource brushes.

As rows, each of these is a key, an operand (target key, ARGB value or path text) and four
positions. One non-generic builder per shape serves every row. This is the part of #496 that took
5,695 functions down to 271.

The additions themselves are repetitive too: 5,880 `rt::add_deferred_resource` sites with seven
arguments and a `?` each. They become one loop per run of rows.

### 5.2 Plain values in the populate parts (33 functions, 15,771 lines)

1,661 direct value additions, each `rt::invoked(IResourceDictionary::__markup_Add_0(&rt::cast(..)?,
rt::string_object(key), rt::to_object(value)), line, column)?`. Of the 1,196 that a simple pattern
matches, 568 are `Thickness`, 502 `f64`, 72 strings and 34 `CornerRadius`. The issue's largest
single function, `populate_winuitheme_xaml_part_31` (104 kB), is 250 of these. Constant row tables
(key, value, position) with one loop per value type remove almost all of that code. Values with no
literal form (enumerations, `GridLength`) stay direct calls.

### 5.3 ControlTheme factories (277 functions, 22,882 code lines; run once per theme)

- **Selectors.** 1,586 selector chains with 6,952 `Selectors::` calls (6,917 of them in ControlTheme
  factories), but only 611 distinct chains. A chain is a short sequence (`nesting`, `class`,
  `template`, `of_type_identity`, `name`, ...). As a static slice of selector operations built by one
  function, a chain costs data instead of code, and identical chains share one slice.
- **Nested styles.** 1,540 `Style::new()` with `__markup_set_Selector` and `StyleBase::__markup_Add_1`.
  About 273 style bodies occur more than once, ignoring local names and positions.
- **Setters.** 1,942 dynamic-resource setters (`rt::add_dynamic_resource_setter_with_parent`, already
  a helper), constant setters (`rt::add_setter`), and 329 setters with a StaticResource or template
  value (`new_setter_with_parent` and `add_setter_value_with_parent`).
- **Table form.** A ControlTheme factory is a target type, a based-on key, setters, nested styles
  (selector plus setters, recursively) and template setters that point to a native content builder.
  All of it fits a static description interpreted by one runtime function. Unlike #496's closed
  instruction grammar, which accepted only 18 of the factories, a declarative table emitted from the
  nodes accepts every factory whose values are constants, resource references or templates. A value
  of any other kind becomes an imported `fn() -> MarkupValue` of that factory only. Each factory keeps
  its own table, so units stay separately removable.

### 5.4 Template contents (244 functions, 25,577 code lines; run once per control instance)

These are hot and stay native code. Shared helpers still apply where the call already is a call:

- 1,572 `rt::bind_template_binding` sites. Runs on one receiver become a slice of
  (target, source) property pairs (#496 `template_bindings.rs`).
- 493 run-time setter choices for `{StaticResource}` values (the `is_instance` chain), plus 567 in
  resource factories and 23 elsewhere: 1,083 in all. One non-generic helper per property value type
  replaces the chain while keeping its order and error messages.
- 2,782 `set_value_with_priority` calls, 1,200 `begin_init` and `try_end_init` pairs, and 1,039
  collection additions. These are the same typed calls the run-time loader makes and are already
  small. Leave them.

The gate for this part is first-template activation time, not size.

### 5.5 Text only

30 % of the lines of the control family are position comments, and every function repeats the full
path of `IServiceProvider` and `XamlLoadError`. Neither costs machine code, but both slow down
compiling the theme. Interning the per-function `rt::DocumentInfo { .. }` literals into statics is
free.

## 6. Plan

Steps 1 and 2 are independent and can run in parallel worktrees. Every step regenerates the
checked-in references of the themes it touches. Sizes are measured locally (hello world `dist` and
release, the catalog), with the symbol attribution script of step 1.

| Step | Content | Gates |
|---|---|---|
| 1. Resource and constant rows | `rt::theme_resources` (re-implemented, no binary types): row tables for the four resource shapes and for `f64`, `Thickness`, `CornerRadius`, `Color`, string constants. One builder per shape. The emitter emits rows at the node, per run. A local script attributes `nm` symbol sizes of each theme's `compiled_xaml` to families and, after step 2, to units. All five themes. | Theme reference tests (all themes); the compiled-against-run-time document tests; gallery and catalog screenshots unchanged; `tests/load_time.rs` and first-template activation no slower; hello world and catalog sizes (expected about -1.9 MB of WinUI machine code) |
| 2. Units and the unit graph | Port the key graph of `resource_roots.rs` (and its tests) into the theme build, extended with types. Emit each unit as its own function or tables, and the unit graph into the `.xamlmeta`. The whole-theme constructor applies all units, so behaviour does not change. | Theme reference tests (`.rs` and `.xamlmeta`); graph unit tests; screenshots unchanged; theme load time unchanged |
| 3. Root tables in applications | `WinUITheme::with_parts`; `try_load` through the registered units; the `type_tables` metadata of `ferroui-base` and the cfg in the theme builds; the emitter writes `with_parts(..)` for classes with units; `ferroui-build` computes roots from compiled documents, keys, base classes, `theme-roots` and the ambient unit, and writes the table; debug diagnostics. Hello world trims automatically. | `ferroui-build` fixture tests; hello world screenshot identical to `theme-roots = "all"`; catalog and gallery screenshots with computed roots identical to `all`; first-template activation; hello world size (target: most of the 7.28 MB, plus the control code only templates reached) |
| 4. Roots from code | `ferroui-build-scan` records the "creates" edges of each class (`X::new`, `X::builder`, `X::default` in its impls and its file) in the type models. `ferroui-build` reads the type names and string literals of the application's sources. This replaces any hand list of companions from step 3. | Scanner tests; every compiled desktop sample (as run for #566) shows no new missing-theme diagnostic and renders identically with computed and `all` roots; hello world and catalog sizes |
| 5. ControlTheme tables | Declarative per-factory tables for ControlTheme construction (5.3), selector slices, the StaticResource setter helper and template binding runs (5.4). Templates stay native. | Theme reference tests; screenshots; first-template activation (first Button, ComboBox, NavigationView, CalendarView) within noise; sizes |
| 6. The other themes | Units and `with_parts` for Fluent, Simple, macOS and Material; their galleries. Fluent, Simple and macOS came with steps 2 and 3; Material is section 13. | Their reference tests, gallery screenshots, sizes |

## 7. Open questions for the owner

1. **Default.** Trim by default for every application whose compiled markup constructs a theme, as
   #566 did for the type tables, with `theme-roots = "all"` as the way out? Or opt in per
   application?
2. **Theme constructed in code.** Keep `WinUITheme::new()` as the full theme (no break for the
   code-only samples), and offer `with_parts` plus a helper for those who want trimming?
3. **Granularity.** One unit per control document plus key-graph groups of resources, as proposed,
   or one unit per key (finest, but a much larger root table for the catalog: about 8,000
   references)?
4. **Shared resources.** `Common_themeresources*`, `Generic.xaml` and `SystemResources.xaml` hold
   2,242 of the 7,542 keyed resources in the WinUI documents, and applications look many of them up
   by key (`TextFillColorPrimaryBrush`). Keep the shared documents whole, or trim them by key like the
   rest and rely on the key scan of sources and markup?
5. **String literals as roots.** Is a conservative scan of the application's string literals for
   theme keys acceptable, or should code lookups always be explicit `theme-roots`?
6. **Missing theme at run time.** Debug-only warning, as proposed, or a panic in debug builds?
7. **Allocation per deferred row.** Keep one `Rc<DeferredContent>` per row, or add a
   `ResourceItem` variant in `ferroui-base` that refers to a static row and allocates on first
   lookup? The variant saves 5,880 allocations at start-up, but changes the dictionary.
8. **Step 5 scope.** Lower every ControlTheme factory to a table, or only those above a size
   threshold? And what bound on first-template activation counts as "no regression": within the
   noise of the paired runs, or a stated percentage?
9. **Other libraries' controls.** For control libraries the application depends on (dialogs,
   TreeDataGrid, colour picker), root the themes of their types only through creates edges, or all of
   them whenever the application depends on the crate directly?

## 8. Step 1 as implemented: resource and constant rows

The emitter writes a run of resources of one shape, added to one dictionary, as a static table of
rows and one call of the builder of the shape. Everything else is written as before.

### 8.1 Runtime

`src/Markup/FerroUI.Markup.Xaml/xaml_il/runtime/theme_resources.rs`, re-exported by
`xaml_il::runtime::compiled` (the `rt` of generated code). One `#[inline(never)]`, non-generic
builder per shape adds its rows in order and returns the first error:

| Shape | Row | Builder |
|---|---|---|
| `{StaticResource Key}` alone | `ResourceAliasRow { key, target, namespaces, line, position, value_line, value_position }` | `add_resource_aliases(dictionary, context, document, rows)` |
| `SolidColorBrush` with a literal colour | `LiteralBrushRow { key, color, namespaces, line, position }` | `add_literal_brushes(..)` |
| `SolidColorBrush` bound to `{DynamicResource}` | `DynamicBrushRow { key, color_key, namespaces, line, position, value_line, value_position }` | `add_dynamic_brushes(..)` |
| `SolidColorBrush` whose colour is `{StaticResource}` | `StaticBrushRow { .. }`, the fields of `DynamicBrushRow` | `add_static_brushes(..)` |
| `x:Double` | `ValueRow<f64> { key, value, line, position }` | `add_double_resources(dictionary, rows)` |
| `Thickness`, `CornerRadius` | `ValueRow<[f64; 4]>` | `add_thickness_resources`, `add_corner_radius_resources` |
| `Color` | `ValueRow<u32>` (ARGB) | `add_color_resources` |
| `x:String` | `ValueRow<&'static str>` | `add_string_resources` |

The deferred builders take `&Ref<ResourceDictionary>`, the context of the declaration, the
`&'static DocumentInfo` of the document and a `&'static` slice of rows. Each row is still added as
deferred content of its own: one `Rc<DeferredContent>` per row, created by `deferred_content` with
the context, as `rt::defer` creates it, and built on first lookup by a closure over the row. The
build of a row makes the calls the generated build function made, in the same order: the context of
the build (`deferred_context`, with the namespaces of the declaration when the row names them), then
`static_resource_content`, `Color.FromUInt32` and `set_value`, `bind_dynamic_resource`, or the
extension, `provide_value_invoked` and the run-time choice of setter (`is_instance` for the unset
marker, a binding and a `Color`, else `no_setter`). Errors have the same messages and positions. A
plain value is added as the statement added it: `rt::cast` of the dictionary to its contract, the
key, the value, `IDictionary.Add`, at the position of the row.

### 8.2 Emitter

`src/Markup/FerroUI.Markup.Xaml.Loader/rust_emitter/theme_rows.rs`, called by `emitter.rs`. It
recognises the shapes where the emitter writes them, not by re-reading the file:

- A deferred resource is a row candidate when its build function is complete
  (`Emitter::deferred_content`): its statements must be exactly those of one of the four shapes, with
  only the key, the colour and the positions free, and it must not use the name scope.
- Its addition is recorded where the emitter fuses it into `rt::add_deferred_resource` (a local
  dictionary, a text key, the result type `object`); a plain value where the emitter writes
  `IDictionary.Add` of a literal number, thickness, corner radius, `Color.FromUInt32` or text to a
  local that holds a `ResourceDictionary`.
- When a body is complete (the function of a document, before it is split into parts, and each
  body of deferred content), each run of consecutive recorded additions of one shape to one
  dictionary is replaced by the position marker of its first addition and one call of the builder
  over a static table `<FUNCTION>_ROWS_<n>`. A run ends at any other statement, so the order of the
  additions is kept. Deferred rows form a run even alone, since each replaces a build function; plain
  values need a run of two. The build functions of the rows of a run are no longer emitted; their
  slots keep their numbers, so the names of the other functions do not change.

The framework paths the recognition compares with come from `EmitTypes::framework_path` (new:
`IResourceDictionary`, `SolidColorBrush`, `Color`, `Thickness`, `CornerRadius`), in both the
run-time and the build-time type systems, so both write the same text.

### 8.3 What stays

The other resource shapes (acrylic brushes, geometries, font families, enumerations, `GridLength`,
`bool`, `i32`), additions with source information, additions to dictionaries that are not locals
of known type, and everything outside dictionaries are written as before. The ControlTheme
factories and the template contents are untouched (steps 2 and 5).

### 8.4 Measurements

Measured on macOS ARM64 against main at 2f8e8c192:

| Theme | Generated text | Functions |
|---|---:|---:|
| WinUI | 14.64 MB to 10.08 MB | 6,265 to 815 |
| macOS | 13.25 MB to 9.16 MB | 6,100 to 680 |
| Fluent | 5.70 MB to 4.47 MB | 1,788 to 362 |
| Simple | 3.24 MB to 3.10 MB | 354 to 284 |

- Hello world (`samples/HelloWorld`, WinUI theme), `dist`: 40.39 MB to 37.73 MB (-2.66 MB, -6.6 %).
  It opens, renders and closes with no diagnostic.
- Theme load (`tests/load_time.rs`, release, WinUI, three runs each, alternated): first load about
  7.0 ms with the rows and 7.1 ms before, second load 6.1 ms and 6.2 ms. No change beyond noise.

## 9. Step 2 as implemented: units and the unit graph

The emitter partitions the compiled markup of a class whose documents merge other documents
(`MergeResourceInclude`, inlined by `XamlMergeResourceGroupTransformer`) into units. Behaviour
does not change: the whole theme applies every unit, at the places its entries had, in the same
order. Step 2 chooses no subset; `with_parts`, the root computation and the application tables
are step 3 (section 10).

### 9.1 Units

- **One unit per merged document.** A control document (`Controls/Button.xaml`: its control
  themes, type-keyed and text-keyed, and its local resources, theme dictionaries included) and a
  resource document (`Resources/Button_themeresources.xaml`: its resources and those of its theme
  dictionaries) are each a unit, named by the path of the document below the root of the
  assembly.
- **The ambient unit**, index 0, named `ambient`, is always kept: the documents that are not merged
  (the document of the class, `Controls/WinUIControls.xaml` with its global `Style` elements,
  `DensityStyles/Compact.xaml`, which `CompactStyles` includes), the structure of the builds (the
  dictionaries, `SystemAccentColors`, the theme variants, `CompactStyles`), the documents that define
  the theme of a host the framework creates (`Window`, `AdornerLayer`, `EmbeddableControlRoot`,
  `ThemeVariantScope`; `PopupRoot` and `OverlayPopupHost` until section 25), and the documents with
  an entry no root can name (an entry without a key, with a key that is neither text nor
  `{x:Type}`, or one that registers a name in the scope of the document; in WinUI only
  `Controls/WindowDrawnDecorations.xaml`, until section 24).
- **Canonical order** is the order of the documents of the group: the class document, then its
  includes in the order they are met. The ambient unit is first.
- **Resources are grouped by document.** The unit graph links the units through their keys, and step
  3 keeps every unit that defines a live key (every definition: overrides and the theme variants).
  Grouping the resources of the shared documents finer (open questions 3 and 4) splits a resource
  unit without changing the format.

### 9.2 Generated code

- `MergedEntryOrigins` (an extra of the transformer configuration that only the transform of the
  emitter adds) records, for each entry a merge inlines, the URI of the document it comes from,
  whether the merge is into the dictionary of the document root or into an inline dictionary of
  the document (#633, section 13.1).
  `rust_emitter/theme_unit_graph.rs` plans the units from the documents and those URIs.
- While the function of a document is written, each entry of a dictionary that comes from a document
  of a unit is written between two marker comments with the index of the unit. Only entries at the
  top level of the function are marked, never those of deferred content. The runs of rows of step 1
  end where the unit changes.
- `rust_emitter/theme_units.rs` then takes each run (consecutive entries of one unit added to one
  dictionary) out of the body into `<function>_unit_<n>(context, dictionary)`, and writes
  `rt::apply_theme_unit(THEME_UNITS, <unit>, <run>, &context, &<dictionary>)?` in its place. An entry
  stays in the body when it is not complete statements at the top level, names a local of the body
  other than the context and its dictionary, or declares a local that is named after it. Row tables
  and deferred build functions stay statics and functions of their own, referred to only by the
  run. The markers never reach the file.
- The file ends with one `static THEME_UNIT_<index>: rt::ThemeUnit` per unit and
  `pub static THEME_UNITS: &[&rt::ThemeUnit]`, in canonical order. Nothing else refers to a run.
- The runtime (`xaml_il/runtime/theme_units.rs`, re-exported by `rt`):
  `ThemeUnit { name, index, runs: &'static [ThemeUnitRun] }`,
  `ThemeUnitRun = fn(&Rc<XamlIlContext>, &Ref<ResourceDictionary>) -> Result<(), XamlLoadError>`,
  `apply_theme_unit(units, index, run, context, dictionary)` (a unit the table does not have adds
  nothing) and `find_theme_unit(units, name)`.
- `WinUITheme::new()`, `with_service_provider()` and `try_load` are unchanged: they populate the
  class, whose builds apply every run through `THEME_UNITS`.

### 9.3 The `.xamlmeta`

`XamlMetadata` and `AssemblyModel` (format 2) gain `theme_units`, written only when a class has
units, and ignored by a reader that does not know it (abridged; the indices follow the order of the group):

```text
"theme_units": [
  {
    "class": "FerroUI.Themes.WinUI.WinUITheme",
    "table_path": "::ferroui_themes_winui::compiled_xaml::THEME_UNITS",
    "units": [
      {
        "name": "Controls/Button.xaml",
        "index": 12,
        "documents": ["Controls/Button.xaml"],
        "keys": ["AccentButtonStyle", ...],
        "type_keys": ["Button"],
        "references": ["AccentButtonBackground", ...],
        "types": ["Border", "Button", "ContentPresenter", ...]
      }
    ]
  }
]
```

- `keys` and `type_keys`: the text keys and the `{x:Type}` keys (by local name) of the outermost
  entries of the dictionaries of the documents of the unit. The entries of a dictionary nested in an
  entry belong to the entry.
- `references`: every key of the group the text of the unit names. That is every identifier token
  of its attribute values and text that is a key (conservative, as in #496), and the argument of
  every `{StaticResource}`, `{DynamicResource}` and `ResourceKey`, which may be any key.
- `types`: the local names of its elements (template content included), of its `TargetType`s, of
  the type selectors of its selectors and of its `{x:Type}`s.
- The `Source` of an include and the content of `Design.PreviewWith` are not read. A group with a
  document that is not plain XML (a DTD) has no units.

Build scripts receive the file through `DEP_<CRATE>_XAML_XAMLMETA` as before; `ferroui-build`
copies the units of a class into the `.xamlmeta` it writes. The checked-in `.xamlmeta` of a theme
is regenerated by the same ignored test as its `compiled_xaml.rs` (`regenerate_compiled_xaml`), and
`build_script_output_is_the_checked_in_reference` compares the units of the build with it.

### 9.4 Measurements

Measured on macOS ARM64 against step 1 (section 8):

| Theme | Generated text | Units |
|---|---:|---:|
| WinUI | 10.08 MB to 10.36 MB | 164 |
| macOS | 9.16 MB to 9.44 MB | 164 |
| Fluent | 4.47 MB to 4.55 MB | 75 |
| Simple | 3.10 MB to 3.17 MB | 72 |

- Hello world `dist`: 37.73 MB to 37.98 MB (+0.25 MB). The unit functions and the table of all units
  cost this while every unit is still applied; step 3 removes the units an application does not
  reach.
- Theme load (`tests/load_time.rs`, release, WinUI, alternated with step 1): second load 6.2 ms and
  6.6 ms against 6.1 ms and 6.0 ms, first load 6.7 ms and 7.2 ms against 6.8 ms and 6.7 ms. Within
  the noise of these runs, possibly up to 0.3 ms for the extra calls through the table.
- Catalog (`fast-release`): the 14 screenshots are byte-identical to step 1 except the home page,
  whose animated banner differs between any two runs; the page tour selects all 127 pages and logs
  the same lines.

## 10. Step 3 as implemented: root tables in applications

An application opts in, and its build computes the units of each theme its documents construct.
The theme crates still compile once, and their output does not depend on any application.
Without the opt-in an application's generated code is what it was before, and it gets the full
theme.

### 10.1 The opt-in and the roots

The opt-in is a key of the application's manifest. That is the owner's rule for size features:
opt-in first, the default later.

```toml
[package.metadata.ferroui]
theme-trimming = true
# Optional: units (by the path of their document), types or keys that the documents do not name.
theme-roots = ["CalendarView", "AccentButtonStyle", "Controls/Popup.xaml"]
# Or keep every unit (the trimmed construction with the full table), for comparing:
# theme-roots = "all"
```

`ferroui-build` reads the table from `Cargo.toml` in the crate directory
(`theme_roots::ThemeTrimming::from_manifest`). Since #850 it reads the manifest as TOML (section
15.2): a header, dotted keys and an inline table are all the same table. A value in any other form fails the
build and names the key. A build script can add roots with `Build::theme_roots(&[..])`; step 4
adds the roots it reads from sources beside them, through `Roots::extend` (section 11.2). The
roots are:

- the types the compiled documents of the application construct, select or name in `{x:Type}`
  (elements, template content included, `TargetType`, the type selectors of selectors, the owner
  of `ToolTip.Tip`). The unit graph reads them as it reads theme documents
  (`rust_emitter::document_references`), with one difference: the types a theme unit only selects
  are conditions and do not make its types live (`docs/binary-size.md`, section 4.19), while an
  application's selectors root what they name. Each type comes with its base classes, which the type
  models give (`theme_roots::BaseClasses`, by the Rust name of the base; `TypeGraph` since step 4). That covers a subclass
  that does not override its style key;
- every identifier token of the attribute values and text of the documents, and the argument of
  every `{StaticResource}`, `{DynamicResource}` and `ResourceKey`. A token that is a key of the theme
  roots the units that define it (conservative, as for the units);
- the names of `theme-roots` and of `Build::theme_roots`: a unit by the path of its document, or else
  a type and a key;
- everything for `theme-roots = "all"`, and everything when the build has the tables of types
  (below).

The closure (`theme_roots::kept_units`): the ambient unit is always kept. A unit is kept when it
defines a live key or the type key of a live type, or when it is named. A kept unit makes its
`references` live keys (StaticResource, DynamicResource, BasedOn, `Theme=` and every other mention
of a key of the group). It makes its `types` live types, with their base classes, so the controls
its templates construct root their own control themes. The types of a unit are what it constructs
(its elements, its `TargetType`s, its `{x:Type}`s, the owner of `ToolTip.Tip`), not what its
selectors select. The result is the canonical indices,
ascending. Every definition of a live key is kept: overrides and the theme dictionaries of every
variant. A control that only Rust code creates was not a root in step 3 and needed a
`theme-roots` entry (see 10.5). Since step 4 the names of the application's sources and the
creates edges of the type models root it (section 11); only a name built at run time still needs
`theme-roots`.

### 10.2 The full themes switch

`ferroui-base` already has `links = "ferroui_base_xaml"`. With its feature `full-themes` (which
includes `type-tables`) its build script prints `cargo::metadata=full_themes=1`, which the build
scripts of the crates that depend on it directly see as `DEP_FERROUI_BASE_XAML_FULL_THEMES`.
`ferroui-build` reads the variable (`Build::from_env`, `Build::full_themes`):

- a crate whose compiled markup has units gets `cargo::rustc-check-cfg=cfg(ferroui_full_themes)`
  and, when the variable is set, `cargo::rustc-cfg=ferroui_full_themes`;
- an application that trims keeps every unit when the variable is set. That is the run-time loader,
  hot reload, the designer, DevTools and Inspect: each of them can show a control that no compiled
  document and no source of the application names. Test builds whose dev-dependencies link the
  run-time loader get the full theme for the same reason. The trimmed configuration is checked by
  release runs (screenshots, sizes).

As first implemented, the switch was `type-tables` itself, so the declarative View API and the
behaviors (which look types up by name but construct only what their application's code names)
also kept every unit. 11.7 separates the two.

### 10.3 Runtime and theme API

`xaml_il/runtime/theme_units.rs` (re-exported by `rt`):

- `ThemeParts = &'static [&'static ThemeUnit]`;
- `set_active_theme_units(class, parts)` and `active_theme_units(class)`: the table the builds of a
  class apply (a thread-local table of tables keyed by the full name of the class). The populate of
  the class sets it before it builds anything and leaves it set, for content built later;
- `register_theme_parts(class, parts)`, `register_theme_parts_if_absent(class, parts)` and
  `registered_theme_parts(class)`: what an application registers for loads by URI (process-wide);
- `trimmed_type_keys(type_keys, parts)` and `report_trimmed_theme(theme, key, trimmed)` for the
  diagnostics (10.4).

The generated file of a class with units (the emitter, `rust_emitter/theme_units.rs` and
`compiled.rs`):

- the builds call `rt::apply_theme_unit(active_theme_units(), ..)` instead of naming
  `THEME_UNITS`;
- `pub fn populate_with_parts(sp, root, parts)` makes `parts` active and populates.
  `populate(sp, root)` is `populate_with_parts(sp, root, THEME_UNITS)`;
- the unit statics are public (`pub static THEME_UNIT_<index>`), so that an application's table
  names them. `AMBIENT_THEME_UNITS` is the ambient unit alone. `THEME_CLASS` is the full name of the
  class;
- `pub fn registered_theme_units()` returns `THEME_UNITS` under `cfg(ferroui_full_themes)`, else the
  registered parts, else `AMBIENT_THEME_UNITS`;
- `THEME_UNIT_TYPE_KEYS`, the type keys of each unit by index, exists under `cfg(debug_assertions)`
  only.

The theme classes (`WinUITheme`, `FluentTheme`, `SimpleTheme`, `MacOSTheme`, and `MaterialTheme`
since step 6, section 13):

| Entry point | Units |
|---|---|
| `new()`, `with_service_provider(sp)` | all (`THEME_UNITS`). They also register the full table for the class unless parts are registered, so that a later load by URI gets the full theme, as before |
| `with_parts(sp, parts: &'static [&'static rt::ThemeUnit])` | the given units, in canonical order, at the places of the full theme. `parts` must include the ambient unit (index 0), which has `CompactStyles` and the structure of the theme |
| `with_registered_parts(sp)`, `with_registered_parts_default()` (hidden) | `registered_theme_units()` |

The constructors of the type information (`new:` of `ferro_class_info!`, the service-provider
constructor of its markup metadata) and the loader table (`build.rs` states
`ClassConstructor::ServiceProvider("with_registered_parts")`) are the hidden constructors. Both are
reachable from every construction: `populate_with_parts` names the class's `TYPE`, and
`register_types()` registers `try_load`. Naming `THEME_UNITS` there would keep every unit. Compiled
markup is unaffected by the change of `new:`, since the emitter writes `T::new()` by convention. One
behaviour changes for an application that does not opt in. A load of the theme by URI, in a
build without the tables of types and before any full theme was constructed, now gets the ambient
unit alone, where it got the full theme before. No sample does this: the compiled markup of an
include of the theme constructs it with `new()`, and every run-time loader links the tables.

### 10.4 The application side

- The emitter (`Emitter::default_object`): when the options of the transform have a parts table for
  the class (`TransformOptions::theme_parts`, a `ThemePartsTables` extra of the configuration),
  `<WinUITheme />` is written as
  `::ferroui_themes_winui::WinUITheme::with_parts(None, crate::compiled_markup::THEME_PARTS_FERROUI_THEMES_WINUI_WINUITHEME)`.
  Without trimming it writes `WinUITheme::new()` as before. The tables record the classes they were
  used for. Since #849 the tables hold only the classes that can be constructed from selected units
  (section 14.1); any other class is written as `T::new()`.
- `ferroui-build`, after every group: for each class used, the kept units become
  `pub static THEME_PARTS_<CLASS>: &[&rt::ThemeUnit]` in `compiled_markup` of `$OUT_DIR/xaml/mod.rs`
  (the name the allocator of section 14.2 gives),
  in canonical order, each entry commented with the name of its unit. `compiled_markup::register()`
  calls `rt::register_theme_parts(<class>, THEME_PARTS_<CLASS>)`. The kept units are also listed in
  `$OUT_DIR/xaml/theme_parts.txt`, for measuring and debugging.
- Diagnostics: in debug builds `with_parts` records the type keys of the units left out. The lookup
  of the WinUI, Fluent, macOS and Material themes (`try_get_resource`) logs once per type through the
  framework logger (warning, area `Control`) when a lookup of such a type key misses:
  `the WinUI control theme of CalendarView was trimmed: add it to theme-roots`. It yields nothing,
  so the control behaves as it does today without a theme. This is the deferred entry of 4(d) done
  as a hook on the miss: an entry keyed by the type would need the type information of every
  trimmed type, which a build without the tables of types does not have by name. The Simple theme
  has no lookup of its own and logs nothing. Release builds carry no names.

### 10.5 Limits until step 4

Step 4 (section 11) removes the first two limits for every control or key the sources name or the
code of a live class creates. What is left is in 11.5.

- Controls created only from Rust code (`Button::new()` in code-behind, the containers a
  `ListBox` creates when no document names `ListBoxItem`, the selection handles of a `TextBox`)
  are not roots. The closure reaches many of them through the templates of the controls the
  documents name. Any others need `theme-roots` until step 4 reads the creates edges and the
  sources. The debug diagnostic names each one the first time it is looked up.
- Keys that code looks up by string (`TryFindResource("TextFillColorPrimaryBrush")`) are roots only
  when a document or `theme-roots` names them, or when a kept unit references them.
- An application that constructs a theme in Rust (`WinUITheme::new()`) keeps the full theme. It can
  pass a table to `with_parts` itself.

### 10.6 Tests

- `ferroui-build`, `theme_roots_tests.rs`: fixture graphs for the ambient unit and its references,
  a type root, key tokens, a BasedOn chain, types constructed by templates, base classes,
  explicit roots (unit, type, key), `all`, the type-tables override, roots added by a build script,
  the static names, and the manifest reader (opt-in, roots over lines with comments, `"all"`,
  errors). `lib.rs`: the parts table and its registration in `mod.rs`, and a wrong opt-in failing
  the build.
- `ferroui-markup-xaml-loader`: the references of an application document
  (`theme_unit_graph_tests.rs`), and the new items of the table and the active-table calls
  (`theme_units_tests.rs`).
- `ferroui-markup-xaml`: the active table, registration, trimmed type keys and the report
  (`theme_units_tests.rs`).
- `ferroui-themes-winui`: `with_parts` without the unit of the rating control leaves its control
  theme out, and the full table has it (`compiled_xaml_tests.rs`).

### 10.7 Measurements

`samples/HelloWorld` opts in (`theme-trimming = true`).

| `samples/HelloWorld`, macOS ARM64 | Size |
|---|---:|
| `dist`, step 2 (section 9.4) | 37.98 MB |
| `dist`, trimmed (30 of the 164 WinUI units) | 28.63 MB (-9.35 MB, -24.6 %) |
| `dist`, `theme-roots = "all"` | 37.98 MB |

Screenshots of the trimmed build and of `theme-roots = "all"` (macOS, `fast-release`, the window captured after 3.5 s): the same picture; the only difference is one grey level in the antialiasing of the window title. The trimmed `dist` build opens, renders and closes with no diagnostic. Applications that do not opt in are unchanged: the control catalog's 14 screenshots are byte-identical to step 2 (except its animated home banner, which differs between any two runs), its page tour selects all 127 pages and logs the same lines, and the include fixture that loads the Simple and Fluent themes by URI passes.

## 11. Step 4 as implemented: roots from code

The roots of step 3 come from the documents. Step 4 adds what Rust code creates or names: the
"creates" edges of the type models, read from the sources of every crate the scanner reads, and
the names and string literals of the application's own sources. Both apply only to an application
that opts in (`theme-trimming = true`); the theme crates still compile once, and their output and
their checked-in references do not change. Step 3 added no hand list of companions (the controls a
control creates in code): the only stopgap was the advice to name them in `theme-roots`, and the
edges replace it.

### 11.1 Creates edges in the type models

`ferroui-build-scan` records, for each class of the object model, the classes its code constructs:
`TypeModel::creates`, the namespace-qualified names of the created classes, sorted, the class
itself left out (`FerroUI.Controls.ListBox` creates `FerroUI.Controls.ListBoxItem`). In the
`.xamlmeta` it is the list `"creates"` of a type, written only when it has items. A file without it
reads as a type that creates nothing, so files of earlier scans and the readers of format 1 are
unaffected, and the format stays 2.

The scan rules (`scanner/source.rs`, `Code`; `scanner/mod.rs`, `Builder::creates`):

- **What is a construction.** A path whose last segment is `new`, `builder`, `default` or starts
  with `new_` (`new_with_size`), called or handed on as a function value. The constructed type is
  the path before the function: `ListBoxItem::new()`, `TextSelectionHandle::new()`,
  `crate::primitives::Thumb::new_with(..)`, `Flyout::builder()`, `Marker::default()`, and the
  qualified forms `<X>::new()` and `<X as Default>::default()`.
- **Where.** In the code of every item of a file that is not test code (`#[cfg(test)]` items and
  members are left out, as everywhere in the scanner), read from the syntax tree for expressions and
  from the tokens for the bodies of macro invocations (`vec![X::new()]`, `rsx! { .. }`). Comments
  are no tokens, attributes (documentation included) are skipped, and string literals and the paths
  of types (`Ref<X>`) are no constructions.
- **Whose edge.** A construction is an edge of every class declared in the file it is written in,
  and of the class of the `impl` block it is written in, wherever that block is (an `impl Separator`
  in another file). `Self::..` is the class itself and makes no edge.
- **Which class.** Only a path whose last segment names a class of the object model, of the crate
  or of a crate it is built on (the models of its dependencies), is resolved; everything else
  (`Rc::new`, `Thickness::default`, a name of no class) is dropped before resolution, which keeps
  the scan cheap. The created class is the one the path resolves to through the `use` items and
  modules of the crate. A path that resolves into another crate and to no class of it (a facade's
  re-export), and a path the modules do not resolve (a name a function imports for itself), stand
  for every class of that name. An extra edge keeps a control theme the application does not need;
  a missing one trims a theme it shows, so the rules err on the side of the edge.

The edges of the framework crates come with their `.xamlmeta` (their build scripts scan them with
`Export`), so an application's build has the edges of every crate it is built on.

### 11.2 Roots from the application's sources

When the application trims, `ferroui-build` scans its sources with
`ScanOptions::source_references` on (only then: the framework crates collect nothing). The scan
returns `Scan::references` (`SourceReferences`):

- `names`: every identifier that starts with an upper-case letter, in expressions, types, `use`
  items and the bodies of macro invocations;
- `strings`: the value of every string literal up to 256 bytes, in code and in macro bodies.

Test code, comments and attributes (documentation) are left out. `theme_roots::source_roots` turns
them into roots: a name that is a type key of a unit of the themes, or a class of the type models,
is a type root (`CalendarView::new()`, `let list: Ref<ListBox>`, `use ..::CalendarView`); a string
that equals a key a unit defines is a key root (`"AccentButtonStyle"`,
`TryFindResource("TextFillColorPrimaryBrush")`). They join the other roots through `Roots::extend`.
This is the conservative scan of string literals of question 5 of section 7: a string that matches
a key keeps its unit whatever the code does with it.

### 11.3 The closure

`theme_roots::BaseClasses` became `TypeGraph`: the base class of each class and the classes each
class creates, by local name, read from all the type models available to the build (the
dependencies' and the application's own). Every type that becomes live (a root, a name of
`theme-roots`, a type a kept unit constructs or selects) makes `TypeGraph::live` live: the type, its
base classes, the classes any of them creates, and so on, transitively, before types are mapped to
their type-keyed units. A created class therefore roots its own control theme, and the types that
theme's templates construct root theirs.

### 11.4 Tests

- `ferroui-build-scan`: the fixture `tests/fixtures/creates` (a list that creates its items by
  `new`, a spacer by a path with its module and `new_with_width`, a separator through `<X>::default`,
  an item that creates a handle by `builder` and markers by `default` in a macro body, a block of a
  class in another file, a class of the crate it is built on), with negatives (comments,
  documentation, string literals, types, other functions, names of no class, `Self`, test code);
  the border of the scanner fixture creating its brush; the names and strings of the sources; the
  round trip of `creates` through the `.xamlmeta`, and a file without it.
- `ferroui-build`, `theme_roots_tests.rs`: transitive creates (through a base class and a cycle),
  creates of a type a kept unit constructs, the type graph read from models, and the roots of the
  sources (type keys, classes, keys; names and strings that match nothing).

### 11.5 Limits

- A name built at run time (`format!("{name}Style")`, a type looked up by a name read from a file)
  matches nothing and still needs `theme-roots`. The debug diagnostic of 10.4 names each such control
  the first time it is looked up.
- A construction through a function of another name (`X::create()`, a factory trait) or through a
  generic `T::new()` makes no edge.
- Only the sources the scanner reads are scanned: the files reached from the crate's root along its
  `mod` items. An application whose build does not scan its sources (neither `export_metadata` nor
  `TypeSystem::Model`) gets no roots from code.
- The edges are per file, not per function: a class whose file constructs another class for a
  feature the application does not use still keeps that class's theme.

### 11.6 Measurements

Measured on macOS ARM64: `samples/HelloWorld` keeps the same 30 of the 164 WinUI units as with
step 3, and its `dist` executable is the same 28.63 MB: the roots from its code and the creates
edges of the controls it reaches add no unit. It opens, renders and closes with no diagnostic. The
other samples do not opt in and are unchanged; opting them in, with their screenshots compared
against `theme-roots = "all"`, is the gate for making trimming the default.

### 11.7 The documents of the dependencies, and `full-themes`

Opting the desktop samples in (the gate of 11.6) found two gaps.

- **The documents of the dependencies.** A control library's compiled document can name a key of
  the application's theme: `samples/SampleControls/HamburgerMenu/HamburgerMenu.xaml` animates its
  pane with `{StaticResource SplitViewPaneAnimationEasing}`, a key of the Fluent `SplitView`
  document. The roots read only the application's documents, so `RenderDemo` and
  `VirtualizationDemo` trimmed the unit away and failed at start-up ("Static resource ... not
  found"). The `.xamlmeta` of a crate whose compiled markup has no theme units now records what its
  documents name (`referenced_types`, `referenced_keys`: `document_references` of each document,
  sorted), and an application that trims roots what all of its dependencies' documents name. The
  documents of a theme are rooted through its unit graph instead, so a theme crate records none.
  This is conservative (open question 9): a library's documents root their keys and types whenever
  the application depends on the library, used or not.
- **`type-tables` is not `full-themes`.** `ferroui-view` (the selector parser of `style(..)`) and
  `ferroui-xaml-interactivity` (the member lookup of the behaviors) look types up by name, so they
  enable `type-tables`; that made every application that uses them keep every unit
  (`HelloWorld.ComponentHost` kept 164 of 164 WinUI units, `BehaviorsGallery` 75 of 75 Fluent
  units). Neither constructs a control that its application's documents or sources do not name:
  `rsx!` names its elements in the sources, which step 4 roots. `ferroui-base` therefore has a
  separate feature, `full-themes = ["type-tables"]`, for what can show any control (the run-time
  loader, hot reload, the designer, DevTools, Inspect, Diagnostics, and the unit-test builds of
  `ferroui-controls` and `ferroui-markup-xaml`), and the switch of 10.2 reads it.

### 11.8 Trimming by default

With the two gaps closed, trimming is the default: an application whose compiled markup
constructs a theme is trimmed unless its manifest says `theme-trimming = false`
(`ThemeTrimming::default()`); `theme-roots` and `"all"` keep their meaning. `samples/HelloWorld`
no longer states the opt-in.

Validated on macOS ARM64 (debug builds, a smoke run of each, and screenshots against
`theme-roots = "all"`):

- the 14 desktop samples of the opt-in run (11.6), with the four of 11.7 rerun: every one opens,
  renders and closes with no "was trimmed" diagnostic. Kept units: AppWithoutLifetime 7 of 75,
  BindingDemo 29, GpuInterop 10, IntegrationTestApp 29, Sandbox 5, TextTestApp 32,
  TreeDataGridDemo 30, RenderDemo 33, VirtualizationDemo 32, BehaviorsGallery 45 (Fluent);
  ProTranslateDemo 34 of 164, ReactiveDemo 45, ReactiveUIExample 49, HelloWorld.ComponentHost 30
  (WinUI). Screenshots of the four of 11.7 match `"all"` (HelloWorld.ComponentHost identical; the
  others differ only in the title bar's active state, an animation frame or a scroll bar's fade);
- with the default and no manifest entry: WinUIThemeGallery, MaterialThemeGallery,
  PlatformSanityChecks, ViewInterop, HelloWorld, the hot reload demo (which keeps every unit:
  `full-themes`), FerroDockReactiveUISample, SingleProjectSandbox, ChartsDemo and LiveChartsDemo
  (which has no smoke exit; its window was captured and closed): no diagnostic, no panic;
- the desktop ControlCatalog at the time linked `ferroui-diagnostics` and kept every unit.
  Its tour selected all 127 pages with no diagnostic, and 12 of its 14 screenshots were identical to
  `"all"` (the home page differs in its animated banner, as between two runs of `main`).

The desktop ControlCatalog links `ferroui-diagnostics` (the F12 inspector) only with its
feature `diagnostics`, so its default build trims the WinUI theme. On native targets it keeps
`ManagedFileChooser` as a theme root (`Build::theme_roots`), because the Dialogs page opens it
through the managed storage provider; its template also supplies the overwrite prompt and
references the ContentDialog resources.

### 11.9 An application without markup

An application whose interface is Rust code (the builders and `rsx!` of `ferroui-view`) has no
document that constructs its theme: it constructed the theme in code with `WinUITheme::new()`,
which is every unit, and linked every control of the theme. Two additions give it the trimming
of 11.8:

- **The calls of the sources are roots** (step 4). The scanner records, with the names that
  start with an upper-case letter, the functions and methods the code calls, in the case of a
  type (`SourceReferences::names`: `text_block()` gives `TextBlock`, `.tool_tip("..")` gives
  `ToolTip`), in the bodies of macros too and outside test code. A builder is named after the
  class it makes, so the rule of step 4 roots it unchanged: a name that is a type key of a unit
  or a class of the type models. It is as conservative as the names are: a function of the
  application named after a control keeps the unit of that control.
- **`Build::theme(class)`** states a theme the code constructs. The build writes its table
  (`compiled_markup::THEME_PARTS_<FULL NAME>`) from the roots of the sources, `theme-roots` and
  `Build::theme_roots`, also for a crate without documents, and the code constructs the theme
  with `with_parts` and the table. With `theme-trimming = false` the table has every unit. A
  theme that no direct dependency has is an error of the build.

The three declarative versions of hello world (`samples/HelloWorld.Builders`, `.Macros`,
`.Hooks`) keep 16 of the 166 units of the WinUI theme, the list the XAML version keeps; the
sizes are in docs/binary-size.md, section 4.24.

The calls add no unit to the applications that were trimmed already: the kept units of
HelloWorld (16 of 166), HelloWorld.ComponentHost (16), ReactiveDemo (47), BindingDemo (29 of 75),
TreeDataGridDemo (30), Sandbox (5), RenderDemo (33) and TextTestApp (32) are the same lists with
and without them.

Tests: `source_references_are_the_names_and_strings_of_the_code` (the scanner),
`a_theme_the_code_constructs_is_trimmed_by_what_the_sources_call` (`ferroui-build`).

## 12. Step 5 as implemented: ControlTheme tables

The factory of a control theme runs once per theme dictionary, on the first lookup of its key, so
it is cold code: the emitter writes it as a declarative table and one call of an interpreter.
Templates, which run once per control instance, stay native code; two shared helpers replace the
repetitive calls inside them (12.3). Behaviour does not change: the interpreters make the calls
the statements made, in the same order, with the same errors at the same positions, and the
objects built are the same.

### 12.1 Runtime

`src/Markup/FerroUI.Markup.Xaml/xaml_il/runtime/theme_styles.rs`, re-exported by
`xaml_il::runtime::compiled` (the `rt` of generated code):

| Generated code before | Table | Interpreter |
|---|---|---|
| the factory of a `ControlTheme` | `ControlThemeTable { parent, members: &[StyleMember] }` | `build_control_theme(context, table)` |
| a selector chain (`Selectors::nesting(..)`, `class`, `template`, `of_type_identity`, `name`, `not`, `or`, `property_equals_untyped`, ..) | `&'static [SelectorStep]` | `build_selector(steps)` |
| the `is_instance` chain of a `{StaticResource}` value | `ResourceChoice` | `assign_resource_value(target, property, value, choice, priority)` |
| consecutive `rt::bind_template_binding` on one object | `&'static [TemplateBindingRow]` | `bind_template_bindings(target, rows)` |

- `StyleMember` is `TargetType(type)`, `BasedOn { key: ThemeKey, line, position }` (a text key or
  a type key, `{StaticResource}` provided with `ControlTheme.BasedOn` as the target property),
  `Setter(ThemeSetter)` or `Style { parent, selector, members, line, position }`, recursively.
  `parent` says whether the theme or style is on the parent stack of the context while its
  members are added, as the emitter pushed it.
- `ThemeSetter { property, form, value }`. `property` is a `PropertyFn`
  (`fn() -> &'static FerroProperty`), since a property definition is no constant. `form` is how
  the statements made the setter: `Plain` (`rt::add_setter`, the value first), `Created`
  (`rt::new_setter`, the value, `rt::add_setter_value`) or `Parented` (the `_with_parent`
  helpers, the setter on the parent stack while its value is made).
- `ThemeValue` is `Dynamic(key)` (`rt::add_dynamic_resource_setter_with_parent`),
  `Static { key, line, position }`, `Template(ThemeTemplate)` (a `ControlTemplate` with its
  optional target type and the build function of its content, which stays native code), the
  constants `Null`, `Bool`, `Int`, `Double`, `Text`, `Thickness` and `CornerRadius`,
  `Expression(fn() -> MarkupValue)` for an expression that needs no context and cannot fail (an
  enumeration member), `Computed(ThemeValueFn)` for any other value, and `Absent` for a setter
  without a value.
- `ResourceChoice { name, member, index, unset, instance: ValueHandle, null, line, position }`:
  the unset marker clears the property when `unset` is set, a binding is bound, an instance of
  `instance` (or null when `null` is set) goes through the typed setter, else the error of no
  setter. The checks are one non-generic function; `assign_resource_value::<T>` is instantiated
  once per property value type, and sets the local value or the value at the priority of a
  template. `ValueHandle` names the type of the check as data (`Class`, `Markup`, `Of`), made
  when the check is made.
- Every interpreter returns the first error, as the statements did; nothing panics on data from
  markup. A member of the theme itself (`TargetType`, `BasedOn`) in a nested style, which the
  emitter never writes, adds nothing.

### 12.2 Emitter

`src/Markup/FerroUI.Markup.Xaml.Loader/rust_emitter/theme_styles.rs`, called by `emitter.rs`.
As for the rows of step 1, the shapes are recognised where the emitter has just written them,
with the statements matched exactly; anything else is written as before.

- **Factories.** When the body of deferred content is complete (`Emitter::deferred_content`) and
  returns `control_theme_0`, `parse_theme` reads its top-level statements into a tree: the
  target type, `BasedOn`, the setters of each form, and the nested styles with their selector
  chains, recursively. A setter value that is not a constant, a resource reference or a
  `ControlTemplate` of the usual shape is still accepted: its statements become a function of the
  factory, `<factory>_value_<n>`, provided they name no local of the factory except their own and
  the context. `control_theme_table` writes `static <FACTORY>_THEME: rt::ControlThemeTable` and the
  value functions, and the factory becomes `let context = rt::deferred_context(..);` and
  `rt::build_control_theme(&context, &<FACTORY>_THEME)`.
- **Shared leaves.** The tables of one generated function share the leaves, never each other:
  one function per property definition (`<function>_property_<n>`), one static per distinct
  selector chain (`<FUNCTION>_SELECTOR_<n>`) and one function per value of a property selector.
  Each factory keeps its own table and value functions, so a unit of step 2 refers only to its
  own factories and the linker still removes the units an application does not reach. Items are
  written in the order they are first needed, so the output does not depend on hashing.
- **Setter choices.** `Emitter::dynamic_assignment` gives the `if` chain it built to
  `resource_choice`; a chain of the helper's shape (the optional unset marker, a binding, one
  typed setter, optionally null through the same setter, else no setter, all on one object and
  property at one position) becomes one `rt::assign_resource_value` statement. That is the shape
  the emitter writes for a `{StaticResource}` value; other chains stay. The rows of step 1 still
  recognise a brush whose colour is a static resource in this form.
- **Template bindings.** `template_binding_runs` rewrites every complete body of deferred content:
  two or more consecutive `rt::bind_template_binding` statements on one object, position markers
  between them allowed, become `rt::bind_template_bindings(object, <FUNCTION>_BINDINGS_<n>)?` over
  a static table of (target, source, position) rows.

### 12.3 What stays native

- Factories with a member the table does not hold: animations (`StyleBase.Animations` with key
  frames), a resource dictionary of the theme, the name scope, or any statement outside the
  shapes. Counted on the checked-in output of the four themes with units before this step, 772
  of the 784 factories are tables; the other 12 are 11 with animations and one with a resource
  dictionary.
- Template contents, apart from the two helpers: they are hot (once per control instance), and
  the gate for them is first-template activation time, not size.
- Choices of a setter of another shape (`set_direct_value`, two typed setters, a binding or a
  template binding without a type check), and single template bindings.

### 12.4 Tests

- `ferroui-markup-xaml`, `theme_styles_tests.rs`: selector chains (with `not`, `or`, property
  and `nth` steps) against the chains the statements built; a table against the theme of its
  factory (target type, the setters of each form, a template, nested styles and their selectors,
  the parent stack left as it was); the error of a failed `BasedOn` against the statements' error,
  and of a missing static resource at its position; `assign_resource_value` for a colour, the
  unset marker, a value of another type and null; the order in which template bindings read
  their properties.
- `ferroui-markup-xaml-loader`, `theme_styles_tests.rs`: the tree of a factory (taken from the
  Fluent button theme), the table and the shared items written for it, `BasedOn` with a type key
  and a static resource setter, selector chains with `or`, `not` and property steps, factories
  that keep their code, constants, template binding runs, and setter choices that become one call
  or keep their chain. `theme_rows_tests.rs`: the static brush in the new form.
- The theme reference tests (`compiled_xaml_is_up_to_date`,
  `build_script_output_is_the_checked_in_reference`), the emitter fixtures
  (`tests/FerroUI.Markup.Xaml.UnitTests/emitter/generated.rs`,
  `src/Markup/FerroUI.Markup.Xaml.WinUI/fixtures/winui_bindings.generated.rs`) and the
  compiled-against-run-time document tests run on the regenerated output.

### 12.5 Measurements

To be filled in after the checked-in output is regenerated and measured (generated text and
functions per theme, hello world `dist`, theme load time and first-template activation).

## 13. Step 6 as implemented: the Material theme

Fluent, Simple and macOS got their units with step 2 and `with_parts` with step 3. The Material
theme (`ferroui-themes-material`) got units with step 2 too, for the documents it merged: its control
documents, which `Controls/MaterialControls.xaml` merges, and its own resources
(`Resources/Material*.xaml`, `Strings/InvariantResources.xaml`). Its class had no `with_parts`, its
inherited resource documents were part of the ambient unit, and three control documents were
public. Step 6 closes the three gaps. Nothing in the build tasks or the emitter names a theme:
`ferroui-build` reads the classes with units and their tables from the `.xamlmeta` of the
dependencies (`theme_units`, `table_path`), so the application side needed no change.

### 13.1 The inherited resource documents

`MaterialTheme.xaml` included its 72 inherited resource documents (the documents of the WinUI
theme with the namespace of this theme: `SystemResources.xaml`, `Generic.xaml`, the
`*_themeresources.xaml`, `ThemeShadows.xaml`, `Glyphs.xaml`) with `ResourceInclude`, each a merged
dictionary of the theme's dictionary, and merged its own resources with `MergeResourceInclude` into
the theme's dictionary itself. A `ResourceInclude` is not inlined, so those documents were not
units: about 2.3 MB of the 10 MB of generated text, always linked.

The WinUI theme merges the same documents with `MergeResourceInclude` into its dictionary, but
the Material theme cannot, for three reasons found by reading the documents:

- `Resources/MaterialCompatibility.xaml` redefines five keys of the inherited documents
  (`ButtonPadding`, `ComboBoxMinHeight` and the three `ThemeShadow*Margin`); one dictionary cannot
  hold a key twice.
- The theme replaces the theme dictionaries of its own dictionary with its color schemes when it is
  constructed (`set_color_scheme`, `ResourceDictionary::set_theme_dictionary`); merged into it, the
  theme dictionaries of the inherited documents would be replaced too.
- A `{StaticResource}` looks in its own document's dictionary first, then in the theme's dictionary,
  where the color schemes override about 1,900 inherited keys (`color_scheme.rs`,
  `install_compatibility`, 1,923 keys of `scripts/material-theme/compatibility-map.json`). The
  inherited documents have 836 such references (distinct pairs of a document and a key) from one
  document to an overridden key of another. Merged into one dictionary of their own, those references would find the
  inherited definition first; for 84 entries whose own keys the schemes do not override
  (`InfoBadgeBackground`, `RadioButtonsHeaderForeground`, the rating control and person picture
  brushes) the value would change.

The inherited documents define no key twice among themselves, and none of them has an entry
without a key. So each is to be merged, with `MergeResourceInclude`, into a merged dictionary of its own,
written inline in `MaterialTheme.xaml`:

```xml
<ResourceDictionary><ResourceDictionary.MergedDictionaries><MergeResourceInclude Source="/Resources/Button_themeresources.xaml" /></ResourceDictionary.MergedDictionaries></ResourceDictionary>
```

Step 6 of #568 could not do it: the compiler did not inline a `MergeResourceInclude` that sits
inside an inline dictionary (`InjectServiceProviderNode: no emitter for this value node`), so the
72 documents stayed `ResourceInclude`s in the ambient unit. #633 adds the compiler support and
changes the theme:

- `XamlMergeResourceGroupTransformer` inlines a `MergeResourceInclude` into the dictionary whose
  merged dictionaries hold it, wherever that dictionary is: the dictionary of the document root,
  as before, or an inline `ResourceDictionary` anywhere in the document, such as a value of the
  merged dictionaries of another one. The group transformers visit every node, so the transformer
  already saw the inline dictionaries; it skipped them because the manipulation of a dictionary
  whose only content is its merged dictionaries is that one assignment, not a group
  (`FlattenAstTransformer` unwraps a group of one). Such a manipulation is now read in a group of
  its own, which becomes the manipulation of the dictionary when an include is inlined. The
  semantics are those of the root: the merged entries are added to the inline dictionary after its
  own entries (an own entry with the key of a merged one is the error of a key added twice, as at
  the root), its theme dictionaries are merged by theme variant, `EnsureCapacity` is the count of
  the inline dictionary, and an include outside of the compilation stays a run-time include
  (decision 22 of xaml.md).
- The inline dictionary stays a merged dictionary of its parent at the place of the include, so a
  lookup through the parent finds its entries as it found those of the `ResourceInclude` (the merged
  dictionaries from the last one, after the own resources and theme dictionaries of the parent), and
  a `{StaticResource}` of the document looks in its own dictionary first, then in the theme's.
- `MergedEntryOrigins` records the inlined entries with the URI of their document, as for a merge
  at the root, so the unit graph makes the document a unit, the emitter writes its entries between
  the markers of the unit, and they become runs added to the inline dictionary (`rt::apply_theme_unit`
  with the local of that dictionary). A theme with parts that leaves the unit out builds the inline
  dictionary empty.

`MaterialTheme.xaml` merges its inherited documents that way, in their order, as in the example
above; `Resources/Material*.xaml` and `Strings/InvariantResources.xaml` are still merged into the
theme's dictionary itself, and `CompactStyles` stays a keyed `ResourceInclude`. The inherited
documents are internal, so, merged, they are no longer emitted as build functions of their own.

### 13.2 The class and the documents

- `MaterialTheme` has the constructors of 10.3: `new()` and `with_service_provider` apply every unit
  and register the full table unless parts are registered; `with_parts(sp, parts)` applies the
  given units and then sets the color schemes, the high contrast scheme and the motion resources as
  the full theme does; `with_registered_parts` and `with_registered_parts_default` (hidden) are the
  constructors of the type information, of the markup metadata and of the loader table
  (`build.rs` states `ClassConstructor::ServiceProvider("with_registered_parts")`). In debug builds
  its `try_get_resource` reports a trimmed control theme ("the Material control theme of .. was
  trimmed").
- `Controls/MaterialBottomSheet.xaml`, `MaterialRangeSlider.xaml` and `MaterialTimePicker.xaml` had
  no `x:ClassModifier="internal"`, so the loader table (`try_load`) answered their URIs and
  referred to their build functions, which kept the three documents linked in every application
  that registers the theme's types. They are internal now, as every other document of the theme;
  nothing loads them by URI.

### 13.3 Tests

`src/FerroUI.Themes.Material/tests/compiled_xaml_tests.rs`. The crate has no checked-in reference
of its compiled markup, so the tests read the output of the build (`$OUT_DIR/xaml/compiled_xaml.rs`
and `FERROUI_XAMLMETA`):

- `the_whole_theme_applies_every_unit`: the table in canonical order, the unit graph of the
  `.xamlmeta` for `MaterialTheme`, the hosts in the ambient unit, the button unit, inherited resource
  documents as units of one document each, and every key of every unit in the full theme (except
  those of `Resources/MaterialColors.xaml`, whose theme dictionaries the color schemes replace).
- `a_theme_with_parts_leaves_out_the_units_it_is_not_given`: without the rating control's unit, its
  control theme is left out; with every unit it is there; `registered_theme_units()` is the full
  table in the test build.
- `a_theme_with_parts_leaves_out_an_inherited_resource_document` (#633): without
  `Resources/RatingControl_themeresources.xaml`, its keys are absent in every variant, the full
  table has them, the motion resources are set, and an inherited resource of a unit it is given
  still resolves the theme's color-scheme override.
- `an_inherited_resource_resolves_the_overrides_of_the_theme`: `RadioButtonsHeaderForeground`
  resolves to the on-surface role of a color scheme, not to the inherited `TextFillColorPrimaryBrush`.

The compiler side of #633 is tested in `ferroui-markup-xaml-loader`
(`merge_resource_include_is_inlined_into_an_inline_dictionary` and
`merge_resource_include_into_an_inline_dictionary_keeps_its_theme_dictionaries` in
`compiler_extensions/ferro_xaml_il_compiler_tests.rs`: the shape is inlined, each inline dictionary
keeps its place, its entries, its theme dictionaries and its capacity, and the entries are recorded
with their documents; `merge_resource_include_in_an_inline_dictionary_is_compiled_into_it` in
`rust_emitter/runtime_host.rs`: the shape compiles; `a_document_merged_into_an_inline_dictionary_is_a_unit`
in `rust_emitter/theme_unit_graph_tests.rs`), and the lookup order through the run-time loader in
`tests/FerroUI.Markup.Xaml.UnitTests/xaml/merge_resource_include_tests.rs`
(`merge_resource_include_works_in_an_inline_dictionary`: own resources first, then the merged
dictionaries from the last one, each with its theme dictionaries).
- `the_classes_of_reflection_paths_are_reached_only_through_their_units` (#612, root D), and the
  loader table answering the class document alone.

### 13.4 Measurements

The theme has 107 units (the ambient unit, five of its own resource documents and the control
documents) in 9.84 MB of generated text. `samples/MaterialThemeGallery` constructs its theme in Rust
(`MaterialTheme::new()`), so it keeps every unit; trimming applies to an application whose compiled
markup constructs `<MaterialTheme />`, and no sample of the repository does yet. With #633 the
72 inherited resource documents (the largest part of the ambient unit) are units of their own.
Hello world with `<MaterialTheme />` in place of `<WinUITheme />` (`dist`, macOS ARM64, measured on the
branch of #633 with main's `MaterialTheme.xaml` for the first number): 19.11 MB with the inherited documents
in the ambient unit, 18.72 MB with them as units (-0.39 MB, -2.1%); both run.

## 14. Generated Rust that compiles (#849)

A review of the compiler found two cases where the build script succeeded and rustc refused the
Rust it generated. `tests/GeneratedRustFixture` reproduces both: two workspace crates, and rustc
compiles what the build script of the consumer writes.

### 14.1 Constructed from units is a capability of the class

Step 3 treated every class with units in a `.xamlmeta` as a theme with `with_parts`. Any class
whose document merges a dictionary has units, though. For example, an application class
`HelloWorld.App` with one `MergeResourceInclude`, constructed by another crate as
`<local:App />`, became `::hello_world::App::with_parts(None, ..)`. `App` has `new()` only, so
rustc refused it (E0599).

Partitioning and construction from units are now separate:

- **The capability.** A class can be constructed from selected units when its type has a public
  associated function `with_parts` without `self` (the name is
  `rust_emitter::PARTS_CONSTRUCTOR`). The function must have the signature
  `fn(Option<Rc<dyn IServiceProvider>>, rt::ThemeParts) -> Ref<Self>`.
- **The class's own build verifies it.** `theme_roots::verify_parts_constructor` compares the
  signature by the last segment of each path. It does this for every class group with units. A
  public `with_parts` with two parameters and another signature is an error of that build.
- **Other crates read the capability from the type model.** In `.xamlmeta`, the type model of the
  class's crate states the public functions of its types (`functions`: the name, the number of
  parameters, `self` or not). `theme_roots::parts_constructor` finds the class there, so no field
  of the metadata changes. A crate without a type model (a `.xamlmeta` of format 1) states no
  capability, and its classes are constructed with `new()`.
- **Only capable classes get a table.** `ThemePartsTables` holds only the capable classes, so the
  emitter writes `with_parts` only for them. Any other class is `T::new()`, with every unit.
  `Build::theme` does not depend on the capability: the application's code constructs the theme,
  and the build writes the table for it.
- **References follow the capability.** The `references` a crate exports are the references of
  all its documents except those of capable classes. Before, a crate with any units exported
  none. An ordinary class is constructed whole, so what its documents name must root the units of
  the applications that depend on its crate. In the repository only the five themes have units,
  and all five are capable. Their `.xamlmeta` and the kept units of the samples are therefore
  unchanged.

### 14.2 One allocator for the generated names

`rust_emitter::allocate_names` names:

- the build functions of an ordinary group, with their untyped wrappers;
- the populate and build functions of a class group;
- through `theme_roots::parts_static_names`, the parts tables of `compiled_markup`.

The relation `collides` decides when two names clash. It also covers the names derived from a
function (`_deferred_<n>`, `_part_<n>`, `_unit_<n>`, `_untyped`).

- **Unique names keep their established form.** A name that collides with no other keeps its
  established form (`function_name_of`, `parts_static_name`). The checked-in generated files and
  the names applications write (`THEME_PARTS_FERROUI_THEMES_WINUI_WINUITHEME`) are unchanged.
- **Every colliding name is disambiguated.** Each one gets `_` and the 32-bit FNV-1a hash of its
  text, in eight hexadecimal digits in the case of the name. The text is the document path below
  the root URI, or the full name of the class. The result does not depend on the order of the
  documents. `A-B.xaml` and `A_B.xaml` in one group, or in a class group, gave `build_a_b_xaml`
  twice (E0428). `A.B_C` and `A_B.C` gave one parts static. Each now gets a name of its own, and
  the `.xamlmeta` records the final build path. An ordinary group used to refuse the second
  document ("rename one of them"); it now compiles both.
- **A public name that cannot change is an error that names both places:**
  - two public documents whose URIs differ only in case, because the loader table compares
    without regard to case (`rt::uri_equals`);
  - a theme stated with `Build::theme` whose established table name another theme of the
    dependencies also has, because the application's code names the table.

### 14.3 Tests

- `tests/GeneratedRustFixture/Library` has these classes, each partitioned into units:
  - an application class with `new()` only;
  - `GeneratedRust.Library.A_B.PartsTheme` and `GeneratedRust.Library.A.B.Partstheme`, themes with
    `with_parts` whose full names differ only in case and punctuation;
  - a theme whose constructor from units is named `from_parts`.
- `tests/GeneratedRustFixture/Consumer` constructs these classes and each theme of the framework in
  its documents. It also has an ordinary group and a class group that both contain `A-B.xaml` and
  `A_B.xaml`. Its tests name the final functions and tables, so rustc checks them:
  - the framework themes keep their established table names;
  - the two colliding themes get their hashed names;
  - the application class and the renamed theme are constructed with `new()`.
- `rust_emitter/source.rs`: the allocator, and the case-insensitive comparison of URIs.
- `rust_emitter/runtime_host.rs` (`colliding_function_names_are_disambiguated`): case, punctuation,
  `_untyped` and `_deferred_<n>` collisions, an internal document beside a public one, and the
  error for two public URIs.
- `theme_roots_tests.rs`: the parts statics, the capability, and the producer check.
- `tests/FerroUI.Markup.Xaml.UnitTests/emitter/build_diagnostics.rs`: the error and the
  disambiguation reported by a build.

## 17. Roots across targets, from dependencies, and only of the documents compiled (#847)

The review found roots of theme trimming that were missed and roots that were over-collected.

### 17.1 Every production target

The build read the sources of a crate from one root file: `src/lib.rs`, else `lib.rs`, else
`src/main.rs`. `ListBox::new()` in the binary of a package that also has a library therefore rooted
nothing (#832 names the same limit for the samples).

`theme_roots::binary_roots` (new) reads the binaries of the package as Cargo finds them:
- the `path` of each `[[bin]]` table, or for a `[[bin]]` without a path, the file its name gives
  (`src/bin/<name>.rs`, `src/bin/<name>/main.rs` or `src/main.rs`);
- unless `[package]` states `autobins = false`, `src/main.rs`, `src/bin/*.rs` and
  `src/bin/*/main.rs`.

The build scans each binary for its names and string literals, as it scans the library, and adds
them to the roots of the sources. Examples, tests and benchmarks are not production targets and are
not read.

`$OUT_DIR/xaml/theme_roots.txt` lists, for each target (by its root file), the types and keys its
sources root. A kept unit can therefore be traced to the target that named it.

The manifest is read as TOML (section 22.2): `[package] autobins` and the `name` and `path` of each
`[[bin]]` table.

### 17.2 What a dependency's code makes

A dependency that is code only (`ListBox::new()` in a free function) exported nothing. The creates
edges belong to classes of the model, and a free function has no owner. Exporting every name and
literal of every dependency's code would root every control of `ferroui-view` (whose builders name
each one) in every application that uses it, so this is not done automatically.

Instead, a crate states what its code constructs with `Build::theme_roots(&[..])`. The build exports
those names with the references of the crate's documents (`references.types` and `.keys` of its
`.xamlmeta`), and every application that depends on the crate roots them.

### 17.3 Only the documents a class compiles

The build collected roots from every document given to a group, before it knew which documents
belong to the class. A `NeverIncluded.xaml` with a `<ListBox />`, given to a class group but
included by no document of it, is never compiled, yet it rooted 13 more units.

For the group of a class, the roots now come from the documents of its group only: the document of
the class and the documents it includes, directly or not (`class_document_group`). An ordinary
group still roots every document it compiles.

### 17.4 Tests

- `theme_roots_tests.rs` (`the_binaries_of_a_package_are_its_production_targets`): explicit
  binaries, a binary by its name, discovered binaries, `autobins = false`, and files that do not
  exist.
- `lib.rs` (`every_production_target_roots_the_themes`): a package whose binary names `ListBox` and a
  key keeps their units while its library names neither. The binary is an input of the build,
  `theme_roots.txt` names it with its types and keys, and a root the build script states is in the
  exported references.

## 19. Why each unit is kept (#852, first step)

Issue #852 asks whether a graph of entries could split the high-fanout dictionaries of a theme
into clusters that link independently: `Resources/Generic.xaml` of WinUI has 330 keys, and
`Accents/FluentControlResources.xaml` of Fluent 626 in one unit. Its rule is to count first, and to
stop if the count does not carry the idea. This step provides the count and the reasons. It changes
no kept unit.

`$OUT_DIR/xaml/theme_parts_why.txt`, written next to `theme_parts.txt`, has a block for each theme
the application trims:

- **A line of totals:** the units kept, and of the keys the kept units define, how many are live.
  A key is live when it is a root or a kept unit references it. A unit with few live keys is the
  candidate for entry-level selection that the issue describes.
- **A line for each kept unit:** its name, its live and total keys, whether its reason is
  `conservative` or `exact`, and the first reason the closure found. A reason is one of these:
  - the ambient unit;
  - named by the manifest or the build script;
  - the theme of a live type, with what made the type live (a root of the documents or the sources,
    a type a kept unit constructs, or a base class or created class of another);
  - a live key, with where it came from (a root, or the unit that references it).

  A reason is conservative when its root is a token of a document, a name of the sources or a
  reference of a dependency: something that may only spell the key or the type. Stated roots,
  constructions and references of kept units are exact.

`theme_roots::explain_kept_units` (new) runs the closure of `kept_units` and records the first reason
it meets for each unit. Its units are those of `kept_units` over the union of its sources, and a test
checks this. `theme_roots::why_kept_listing` (new) writes the block.

Not done in this step:

- the count for hello world and a larger sample, and the bytes behind the counts. The listing gives
  the keys; the bytes need a link map of a build of a sample with the Skia backend.
- clusters inside one dictionary. Whether to build them waits for that count.

## 20. A library that constructs a theme (#846)

### 20.1 The selection reaches the library

A library document with `<WinUITheme />` was compiled to
`WinUITheme::with_parts(None, crate::compiled_markup::THEME_PARTS_..)`, with the units the library
needs. An application that uses the library and adds `<ListBox />` computed no table for the theme,
because the emitter records a theme as used only in the crate that writes the construction. The
theme the library constructed then had no `Controls/ListBox.xaml`.

Now:

- **The construction takes the wider table.** The construction is written as
  `with_parts(None, rt::wider_theme_parts("<class>", crate::compiled_markup::THEME_PARTS_..))`.
  `rt::wider_theme_parts` (new) returns the parts registered for the class when they have more
  units than the table of the crate that writes the construction, and that table otherwise.
- **Registration keeps the wider table.** The generated `register()` of a crate registers its
  tables with `rt::register_wider_theme_parts` (new), which keeps the wider table whichever crate
  registers first.
- **The application computes the table.** For each theme that the compiled documents of a direct
  dependency name (the `references.types` of its `.xamlmeta` hold the class name), the build of the
  application writes and registers a table. The table is computed from all the roots of the
  application, whether or not the application's own documents construct the theme. This needs no
  new field in the metadata: the references of the library's documents are the record of what
  they construct.
- **Limits.** The table is written only when the theme's crate is a direct dependency too, since
  the table names its unit statics. A library used alone, or in an application that does not depend
  on the theme crate, keeps its own table. A crate without markup writes the file of its tables when
  it has one, and `register()` must run for it to take effect.

The rule "the wider table" assumes that the application's table holds the library's. That is the
usual case, since the application's roots include what the library's documents name. A library
whose code constructs controls that its documents do not name states them with `Build::theme_roots`,
which #847 exports (section 17).

### 20.2 The selection as state of the thread (not done)

`set_active_theme_units` still writes a thread-local entry for each theme class, and the generated
builds read it. A second construction of the same theme with other parts on the same thread, or a
deferred producer that runs later, can read the selection of another instance. Carrying the
selection in the creation context of the document, captured by its nested and deferred factories,
changes the generated code of the theme builds. That is left for the work on the output of the
themes (#853 and the related issues). `register_theme_parts` still replaces an earlier
registration; the generated `register()` now uses the wider registration instead.

### 20.3 Tests

- `theme_units_tests.rs` (`the_wider_table_of_a_class_is_registered_and_applied`): registering in
  either order keeps the wider table, and a construction applies the wider table.
- `lib.rs` (`a_theme_a_library_constructs_gets_the_table_of_the_application`):
  - an application that never names the theme, whose control is named in its sources, gets the
    theme's table with that control's unit, registered with the wider registration;
  - without the theme crate among its direct dependencies, it gets no table.

## 21. Edges of the unit graph across themes, crates and types of one name (#848)

The review found three places where the unit graph lost an edge.

### 21.1 Themes used together

`kept_units` closed over one theme at a time. Suppose a kept unit of theme A references a key that
only theme B defines, or constructs a type whose control theme only B supplies. Then B kept only
its ambient unit.

`theme_roots::kept_units_across` (new) closes over every theme the application uses, in one loop:

- the keys a kept unit of any theme references, and the types it constructs, become roots of every
  theme;
- the loop runs until no theme keeps another unit, so a cycle of references between two themes
  ends;
- the result for each theme does not depend on the order of the themes.

The build calls it for the themes it writes tables for. An application with one theme keeps what it
kept before.

The reasons of `theme_parts_why.txt` (section 19) follow the same closure: for each theme, what
the kept units of the other themes reference and construct is one more source of roots, named
`other themes used together`, which is exact. With one theme that source is empty, and the
listing is the one of section 19.

A unit records the keys of `{StaticResource}`, `{DynamicResource}` and `ResourceKey` that its
documents name, including keys no document of its own theme defines (21.4). Without this, a key that
only another theme provides could not root that theme.

### 21.2 Crates with a theme and ordinary documents

Since #849 a crate exports the references of every document except those of a class constructed
from selected units (section 14.1). A crate with a theme class and an ordinary `Other.xaml` with
`<ListBox />` therefore exports `ListBox`. The path that imports checked-in metadata
(`Build::checked_in_metadata`) copied the documents and the units and dropped the references. It now
merges them into the references the crate exports.

### 21.3 Types of one name

`TypeGraph` keyed its edges by local name. Two assemblies with `Left.Twin : LeftBase` and
`Right.Twin : RightBase` gave one chain or the other, depending on the order of the models, and
their creates edges were merged.

The graph now identifies each type by its full markup name. A base class is found by its Rust path,
and by its local Rust name only when that name is unique. Roots and units still name types by local
name, as documents write them. A local name stands for every type of that name, and is live with the
edges of each. Where the name is not unique, every possible provider is kept rather than one picked
by order. No ambiguity is reported yet: old metadata and unqualified roots cannot say which type they
mean.

### 21.4 References to keys of other themes

The unit graph used to drop the argument of a resource extension unless the same theme defines the
key. It now keeps every such argument as a reference of its unit. Identifier tokens of attribute
values are still kept only when they spell a key of the theme, since they are only conservative
guesses. This changes the `references` of the theme units in the `.xamlmeta` of the themes, so the
checked-in `compiled_xaml.xamlmeta` of a theme whose documents name such a key has to be regenerated
(its `compiled_xaml_tests` say so). The generated Rust does not change.

### 21.5 Tests

- `theme_roots_tests.rs`:
  - `themes_used_together_are_closed_over_together`: a key from the other theme, a type theme from
    the other theme, a cycle, and both orders of the themes;
  - `types_of_one_local_name_keep_their_own_edges`: both orders of the models;
  - the existing graph tests, unchanged.
- `theme_unit_graph_tests.rs`: an extension key that no document of the group defines is a
  reference of its unit.

## 22. Build inputs read as what they are (#850)

The review that found the problems of section 14 found three places where the build read an input
as text. Each one has a test that failed on main.

### 22.1 Includes

The group of a document is the document and the documents it includes, directly or not. Both the
build (`rust_emitter::class_document_group`) and the run-time loader collect it with
`back_end::include_sources`. That function used to scan the raw text of the document, so:

- `Source="/A&amp;B.xaml"` was collected with its entity, and `A&B.xaml` was left out of the group;
- `x:Key="a>b"` ended the tag early, and the include behind it was not found;
- an include written as text inside CDATA was added to the group.

`include_sources` now reads the document with `roxmltree`, over the text the parser of the compiler
reads. `XDocumentXamlParser::xml_text` (new in `xamlx`) gives that text: the document type
declaration is blanked out, as `parse` does. The rules are these:

- an include is an element whose local name is `MergeResourceInclude`, `ResourceInclude` or
  `StyleInclude`, with any prefix, as before;
- its source is the value of its `Source` attribute that has no namespace, with entities decoded;
- text, CDATA and comments are not elements.

A document that is not well formed has no includes. It fails when it is compiled or loaded, with
the diagnostic of the parser, the same diagnostic it gets without includes. No new dependency is
needed: the parser of `xamlx` already uses `roxmltree`.

### 22.2 The manifest

`[package.metadata.ferroui]` (`theme-trimming`, `theme-roots` and `compile-bindings`) was read by a
parser of its own lines, which had four faults:

- dotted keys under `[package.metadata]` were ignored;
- `"Accent\u0042uttonStyle"` lost its escape;
- `["Accent[Brush"]` was reported as an array that is not closed;
- a top-level table whose quoted name is `"package.metadata.ferroui"` was taken for the nested
  table.

`ferroui-build` now parses the manifest with the `toml` crate (1.1.6). That version was already in
the lock file of the workspace, as a build dependency of `skia-bindings`. `ferroui-build` is a
build dependency of every crate that uses it, so the parser runs only in build scripts and is linked
into no application.

The options are read from the table at the path `package.metadata.ferroui`, whatever form the
manifest writes it in: a header, dotted keys, an inline table, or dotted keys at the top level. A
value of the wrong type is refused, and the error names the option. A manifest that is not TOML is
an error of the build; Cargo has already parsed the manifest, so this does not happen in practice.
The binaries of a package (section 17.1) are read from the same parsed manifest.

The `toml` dependency is kept by a decision of the owner: it is build-only (about 3 s more of a
cold build, no cost in a binary or at run time).

### 22.3 Assets

`Build::embed_assets` embeds the files of its directories as raw bytes, except the documents the
compiler consumed. For a class group, that list held only the documents the compiler emitted one by
one. A dictionary merged into the class has no build function of its own: it is compiled into a
unit of the class. Such a dictionary under an embedded directory was therefore embedded again. A
class group now leaves out every document of its group: the document of the class and every
document it includes, merged documents included.

Every other file is embedded as before, including a document of an embedded directory that no group
consumed. The theme crates do not use `embed_assets` for their documents; they have filters of their
own. Their assets, their `.xamlmeta` and the checked-in generated files are unchanged. There is no
explicit option yet to ship the original text of a consumed document (see the issue).

### 22.4 Tests

- `back_end/includes.rs`:
  - an entity in a source;
  - a `>` in an attribute;
  - an include in CDATA or in a comment;
  - a document that is not well formed;
  - a document type declaration.
- `rust_emitter/compiled.rs` (`the_group_of_a_class_follows_its_includes_as_parsed`): the group of a
  class with these cases.
- `tests/FerroUI.Markup.Xaml.UnitTests/xaml/style_include_tests.rs`: a document that is not well
  formed, with an include, fails to load with the diagnostic of the parser, position included.
- `theme_roots_tests.rs`:
  - equivalent forms of the table give the same options;
  - escapes, and a bracket inside a string;
  - a quoted top-level table name is a different table;
  - a value of the wrong type names its option.
- `tests/GeneratedRustFixture/Library`: `Dictionaries/Part.xaml` is merged into `App.xaml` and its
  directory is embedded. Neither its path nor the canary comment of the file is in `ASSETS`, while
  `Dictionaries/Loose.xaml`, which no document consumes, is.

## 23. What the documents of an ordinary class root (#1002)

`samples/HelloWorld` (a window with text) kept the theme of `Button`, and with it
`Resources/Button_themeresources.xaml`, `Resources/HyperlinkButton_themeresources.xaml` and part of
`Resources/Deprecated_themeresources.xaml`. `theme_parts_why.txt` gave the reason "the type
Button, a root of the dependencies". The dependency is `ferroui-dialogs`, reached through
`ferroui-native` and `ferroui-themes-winui`. Its one document, `AboutFerroDialog.xaml`, is the
document of an ordinary class (a `Window`). Its `referenced_types` were `Button`, `DrawingGroup`,
`DrawingImage`, `GeometryDrawing`, `Grid`, `Image`, `Style`, `TextBlock` and `Window`. Section 11.7
exports what every document of a crate names, and an application roots what all of its
dependencies export, so the references of the dialog rooted the themes of every application that
links the crate. Only `samples/ControlCatalog` constructs the dialog.

### 23.1 References by class

A crate now exports the references of the document of an ordinary class (the group of a class
that is not constructed from selected units, section 14.1) by class. The `.xamlmeta` gets
`class_references`: for each class, its full markup name (`x:Class`), `referenced_types` and
`referenced_keys`. The list is written only when a class names something, so the metadata of
every other crate is unchanged. Both readers read the list as optional.

Unchanged:

- The references of style and resource groups are assembly-wide (`referenced_types`,
  `referenced_keys`).
- So are those of a class constructed from selected units (rooted through its unit graph, so in
  practice none), and of a document whose `x:Class` the build cannot read.
- The check of #846 (a theme a direct dependency's documents construct) reads both lists
  (`MarkupReferencesModel::names_type`).

### 23.2 Live classes

An application adds the references of a class to its roots only when the class is live
(`live_class_references`). A class is live when:

- its local or full name is a name of the application's sources: `AboutFerroDialog::new()`, or
  the builder `about_ferro_dialog()`, which the scan records in the case of the type; or
- the types live with the roots include it. These roots are the documents, the sources, the
  manifest, the build script, the assembly-wide references of the dependencies, and the
  references of the classes found live so far, each with its base classes and created classes
  (`TypeGraph::live`).

The search repeats until nothing changes, so a class whose document constructs another class
makes that class live too. A base class being live does not make its subclass live. The reason in
`theme_parts_why.txt` names the class: "the type Button, referenced by the document of the live
class FerroUI.Dialogs.AboutFerroDialog".

### 23.3 Classes their own crate constructs

The build cannot see a class that the code of its own crate constructs: a window a function
opens, or a view a view locator makes. The type models record no edge from the application to
that code. A group states it with `XamlGroup::constructed_by_its_crate()`, and the references of
its class are then exported assembly-wide, as before. The framework's library crates with
documents of ordinary classes:

| Crate | Classes | Constructed by its own code | Exported |
|---|---|---|---|
| `ferroui-dialogs` | `AboutFerroDialog` | no (only `samples/ControlCatalog`, and the builder of `ferroui-view`) | by class |
| `ferroui-diagnostics` | `MainWindow`, `MainView`, the pages and the views of DevTools | yes: `DevTools` opens `MainWindow`, and its view locator makes the pages | assembly-wide (`constructed_by_its_crate`) |
| `ferrodock-ferro-diagnostics` | `RootDockDebug` and the debug views | yes: `root_dock_debug_extensions` constructs the inspector, which shows the views | assembly-wide (`constructed_by_its_crate`) |
| `ferrodock-ferro-themes-fluent`, `-simple`, `-browser` | `DockFluentTheme`, `DockSimpleTheme`, `BrowserTabTheme` | no (applications construct them, by name) | by class |

The theme crates (`ferroui-themes-*`) construct their classes from selected units, and are
unchanged.

### 23.4 Tests

- `xaml_metadata.rs` (`class_references_round_trip_through_the_text`) and `model.rs`
  (`class_references_are_read_by_both_readers`): the list is written and read by both formats,
  and a file without it reads as none.
- `theme_roots_tests.rs`:
  - `the_references_of_an_ordinary_class_root_only_where_the_class_is_live`: a class nothing
    constructs roots nothing. A class is live when the sources name it, by its local or full
    name; when a live type creates it; or when the document of another live class constructs it.
  - `a_unit_kept_for_a_live_class_says_which_class`: the reason names the class.
- `lib.rs` (`the_references_of_an_ordinary_class_root_the_themes_only_where_it_is_live`): a build
  of an application whose dependency's class is not constructed keeps only the ambient unit. One
  that constructs it keeps the theme of `Button`, with the reason that names the class.

## 24. Creates edges that hold on some targets (#1002)

`samples/HelloWorld` on macOS kept the theme of `Button`, the caption-button resources of
`Resources/Generic.xaml`, `Resources/TitleBar_themeresources.xaml`, the caption glyphs of
`Resources/Glyphs.xaml` and `Controls/PathIcon.xaml`. The reason was the ambient unit:
`Controls/WindowDrawnDecorations.xaml`, the drawn frame and title bar of a window, was part of it.
No backend of macOS draws these decorations.

### 24.1 The document of the drawn decorations was ambient

A document is ambient when one of its entries registers a name in the scope of the document
(`registers_outer_name`). The names inside a framework template are in the scope of the template,
and the list of templates did not include `WindowDrawnDecorationsTemplate`, whose content has a
name scope of its own (`WindowDrawnDecorationsTemplate::build`). The `PART_` names of the caption
buttons therefore made the document opaque. With the template in the list, the document is a unit
keyed by `WindowDrawnDecorations`, in every theme that has it (WinUI, Fluent, macOS, Material and
Simple).

### 24.2 The class is live in every application with a window

The type models have the creates edges `TopLevel` to `TopLevelHost` to `WindowDrawnDecorations`
(and `LayerWrapper` to `WindowDrawnDecorations`). Through them, every application with a window
makes the new unit live. At run time the class is constructed only by
`TopLevelHost::update_drawn_decorations`, when the platform window needs managed decorations
(`IWindowImpl::needs_managed_decorations`):

| Backend | Target | `needs_managed_decorations` |
|---|---|---|
| `ferroui-native` | macOS | always `false`: the system draws the decorations |
| `ferroui-win32` | Windows | while the client area is extended, a setting the application can change at run time |
| `ferroui-x11` | Linux, BSDs | for managed decorations or `WindowDecorations::BorderOnly` |
| `ferroui-wayland` | Linux, BSDs | when the compositor asks for client-side decorations, at run time |
| iOS, Android, browser | | no window backend with decorations |

### 24.3 The condition next to the code that decides it

`TypeModel::not_created_on` states the targets on which no code constructs a class, as `cfg`
conditions (`target_os = "macos"`, `target_arch = "wasm32"`). The build script of the crate of
the class states them (`Export::not_created_on`), and the field is written only on such a class.

`ferroui-controls` keeps the table in `chrome/drawn_decorations_targets.rs`: macOS, iOS, Android
and `wasm32`. Its build script reads the file. The test of the file reads the
`needs_managed_decorations` of the backends of the repository and checks two things:

- a backend that is always `false` serves only targets of the table, and a backend that can need
  managed decorations serves none of them;
- no other crate implements the method.

The application build evaluates the conditions against `CARGO_CFG_TARGET_OS` and
`CARGO_CFG_TARGET_ARCH` (`BuildTarget::from_env`). `TypeGraph::from_models_for` drops the creates
edges into a class that is not created on the target. A build without a target, such as a test,
keeps every edge. Where the application names the class, it is still live.

Windows and Linux keep the unit in every application. A run-time setting the build cannot see
(the extended client area of Windows, the decoration mode of a Wayland compositor) must not lose
the theme of the caption buttons. A later option could drop it for Windows when no document or
source names `ExtendClientAreaToDecorationsHint`, with its own risk note. The designer and the
previewer build with every unit of the themes (`full-themes`), and are unchanged.

### 24.4 Tests

- `theme_unit_graph_tests.rs`
  (`names_in_a_template_of_the_drawn_decorations_do_not_make_a_document_ambient`): the document
  is a unit keyed by `WindowDrawnDecorations` that constructs `Button`, while a name outside a
  template keeps a document ambient.
- `export.rs` (`export_writes_the_targets_a_class_is_not_created_on`): the conditions are written
  on the class, and an unknown class is an error.
- `theme_roots_tests.rs` (`a_class_not_created_on_the_target_is_not_live_through_its_creates_edges`):
  the edges into the class hold on Linux, Windows and without a target, and not on macOS and
  `wasm32`. There the class is live only where it is named.
- `ferroui-controls` (`chrome::drawn_decorations_targets::tests`): the table is in step with the
  backends.

## 25. The hosts of popups are units of their own (#994)

The ambient unit of every theme held the theme of `PopupRoot` and of `OverlayPopupHost`
(`FRAMEWORK_HOSTS`): hosts the framework creates, which no application names. An application
that never opens a popup kept their control themes, and through them the members of `PopupRoot`
(a top level) that the themes set: about 243 kB of hello world, by the estimate of its IR.

A popup host is created only when a `Popup` opens: `Popup` calls
`OverlayPopupHost::create_popup_host`, which makes a `PopupRoot` (a platform popup) or an
`OverlayPopupHost` (on the overlay layer). A popup is constructed by what shows one: a tool tip,
a flyout, a context menu, or a template with a `Popup` element.

The change:

- `FRAMEWORK_HOSTS` no longer names the two hosts. Their documents are units keyed by their type.
- The scan reads a construction only as `X::new` and its kin in the code of a class, not as a
  factory function of another class. So `ferroui-controls` states the edges from `Popup` to both
  hosts in `primitives/popup_hosts.rs`, which its build script reads and applies with the new
  `Export::creates`. The edges are added to `TypeModel::creates`.
- The test of the file checks three things: `Popup` calls the factory, the factory constructs
  both hosts, and no other source of the repository creates a host outside tests. (The view models
  of DevTools have a `PopupRoot` of their own.)

An application keeps the themes of the hosts where a popup is live: a `Popup` it constructs or
whose class it names, or one a live class creates (`ToolTip`, `ContextMenu`, `PopupFlyoutBase`
create one).

### 25.1 Tests

- `export.rs` (`export_adds_the_stated_creates_edges`): the stated classes are added once,
  sorted, and an unknown class is an error.
- `ferroui-controls` (`primitives::popup_hosts::tests`): the table is in step with the code that
  creates the hosts.

## 26. Keys inside the kept units (#852, step 2)

A unit is a whole document, and keeping it keeps every key of the document. Of the 336 keys of
`Resources/Generic.xaml`, hello world names a few dozen. The closure over units also counts every
reference of every entry of a kept unit, so an entry nothing needs still keeps the units it names.

### 26.1 What each entry names

The unit graph records, for each text-keyed entry of a unit, the keys it names
(`ThemeUnitModel::entries`, `ThemeEntryModel`). The entries of one key in the theme dictionaries of
the unit are one entry. `body_references` holds what the rest of the unit names: its control
themes by type, its styles, its unkeyed entries and its structure. A kept unit applies all of
that whatever keys are live. The rules of a reference are those of the unit: a token that spells
a key of the group, and the argument of a resource extension.

### 26.2 The form in the `.xamlmeta`

With the name of every key, the graph made the metadata of the WinUI theme 2.8 times as large
(325,502 to 914,887 bytes), and an application build reads that file. So an entry is a row of
indices in the sorted lists the unit already has:

```json
"keys": ["Accent", "AccentBrush"],
"references": ["Accent", "Other"],
"entry_references": [[1, 0]],
"body_references": [1]
```

The first index of a row is the key of the entry in `keys`, and the others are what it names in
`references`. An entry that names nothing has no row. A name that is not in its list is written
as text. A unit without `entry_references` (a file of an earlier build) has no entry graph, and
every key of it counts as live.

### 26.3 The count

An application build writes `$OUT_DIR/xaml/theme_keys.txt`: for each kept unit, its keys, the
keys live by key and the dead keys. A key is live when it is a root, when the rest of a kept unit
names it, or when a live entry names it. The listing changes nothing in the application.
