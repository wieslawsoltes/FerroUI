# Theme trimming and compact theme code (#568)

Design note, 2026-11-06. Nothing here is implemented yet. Counts are taken from the checked-in
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
  `WinUIControls.xaml`, the hosts the framework creates (`Window`, `PopupRoot`, `OverlayPopupHost`,
  `AdornerLayer`, `EmbeddableControlRoot`, `ThemeVariantScope`), `CompactStyles`, and
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
  only under a cfg that the theme's build sets when `ferroui-base` is built with `type-tables`.
  `ferroui-base` already has `links = "ferroui_base_xaml"`: its build script can print
  `cargo::metadata=type_tables=1`, so dependent build scripts see `DEP_FERROUI_BASE_XAML_TYPE_TABLES`.
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
  - everything, when `DEP_FERROUI_BASE_XAML_TYPE_TABLES` is set.
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
  `template`, `of_type_info`, `name`, ...). As a static slice of selector operations built by one
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
| 6. The other themes | Units and `with_parts` for Fluent, Simple, macOS and Material; their galleries. | Their reference tests, gallery screenshots, sizes |

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
