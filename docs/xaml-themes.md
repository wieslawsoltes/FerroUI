# Compiled XAML themes: how they are built, and how to write them

This guide covers how a theme is compiled, how an application gets only the part of a theme it
uses, and the rules for writing or porting theme markup so that it costs no reflection, as little
binary size as possible, and no time at start-up or in the first template of a control. It
describes the final design. Section 9 says what is on `main` and what is still in progress. It is
kept current: every change to theme compilation, theme trimming, compiled bindings or theme size
updates this document and its status table in the same pull request.

Background and measurements are in [docs/porting/theme-trimming.md](porting/theme-trimming.md),
the design record of #568, and in [docs/binary-size.md](binary-size.md).

## 1. The three goals

- **No reflection.** Nothing in a theme looks a type, a property or a member up by its name at run
  time. A type is a `&'static TypeInfo`, a property a `&'static FerroProperty`, a binding a
  compiled path. The tables of types (`ferroui-base/type-tables`) are not needed to run a theme.
- **Only what is used is linked.** A theme is split into units, and an application links only the
  units its markup and code can reach. The rest is removed by the linker.
- **Data over code, compile time over run time.** Values are converted when the markup is
  compiled, not when it is loaded. Repeated shapes (resources, control themes, selectors, template
  bindings) are static tables read by one shared function each, not a generated function each.
  Template content stays native code, because it runs every time a control is created.

## 2. How a theme is compiled

A theme is a crate (`ferroui-themes-winui`, `-fluent`, `-simple`, `-macos`, `-material`) with
the documents of the theme and a Rust class (`WinUITheme`). Its `build.rs` does three things:

1. **Embeds the documents that are not compiled** as assets. `Controls/excluded.txt` lists the
   control documents that are not merged into the theme yet, and why.
2. **Compiles the theme** with `ferroui_build::Build::from_env().type_system(TypeSystem::Model)`.
   Every `{Binding}` is a compiled binding, the default of the build (section 4.5).
   It compiles one group: the document of the class (`WinUITheme.xaml`) and every document it
   includes. The compilation runs against type models, not a running program. The models are the
   scan of the crate's own sources and the `.xamlmeta` files of the crates it depends on. The
   constructor of the class is stated (`ClassConstructor::ServiceProvider("with_registered_parts")`).
3. **Writes the outputs.**
   - `$OUT_DIR/xaml/compiled_xaml.rs`: the generated code.
   - `$OUT_DIR/<crate>.xamlmeta`: the type model of the crate and the description of its compiled
     markup, including the unit graph (section 3). Other crates read it through the `links` key.

The crates with checked-in output (WinUI, Fluent, Simple, macOS) also keep a reference copy,
`compiled_xaml.rs` and `compiled_xaml.xamlmeta`, so that a change to the generated code shows in
review. Their tests `compiled_xaml_is_up_to_date` and
`build_script_output_is_the_checked_in_reference` fail when the reference is stale. Regenerate
with:

```bash
cargo test -p ferroui-themes-winui --lib tests::compiled_xaml_tests::regenerate_compiled_xaml -- --ignored
```

Run the same command for `ferroui-themes-fluent`, `ferroui-themes-simple` and
`ferroui-themes-macos`. Commit the regenerated files with the change that caused them. The
Material theme has no checked-in reference; its tests (`tests/compiled_xaml_tests.rs`) check the
units of the build's own output.

### 2.1 What the generated code is made of

| Part | Form | When it runs |
|---|---|---|
| Plain resources: brushes with a literal colour or a resource colour, `x:Double`, `Thickness`, `CornerRadius`, `Color`, `x:String`, `{StaticResource}` aliases | Static row tables, one shared builder per shape (`rt::add_literal_brushes`, `rt::add_double_resources`, ...) | Added at load. A value row (`x:Double`, `Thickness`, `CornerRadius`, `Color`, `x:String`) is added as its value; a brush or alias row is built on its first lookup (section 2.2) |
| Other resources | One deferred build function per resource (`rt::add_deferred_resource`) | On first lookup |
| Control themes | One static table per theme, interpreted by `rt::build_control_theme`; selector chains are static slices, shared when identical | Once per theme, on first use of the control type |
| Control templates | Native code; what every element repeats (the parent stack, the end of the initialisation, the name, a `{CompiledBinding}` bound to a property) is a call of a shared helper of `rt` (`docs/porting/xaml.md`, 9.3.7) | Every time a control applies its template |
| Template bindings in a template | Runs of (target, source) pairs, applied by `rt::bind_template_bindings` | With the template |
| Dynamic resources in a template | Runs on one object of (property, key) pairs, bound by `rt::bind_dynamic_resources`; one alone is a call of `rt::bind_dynamic_resource` | With the template |
| Units | `THEME_UNIT_<n>` statics and the table `THEME_UNITS` | See section 3 |

A control theme whose content the table cannot express keeps native code for that theme only.
Today that means one with animations, a resource dictionary of its own, or a use of the name
scope.

**Template bindings whose setter is chosen at run time (#854, an experiment, off).** A property
with several setters (`ContentPresenter.Content`, of type `object`) whose value is
`{TemplateBinding Content}` is written as the choice of the setter when the template runs: the
binding, its provided value, a test of the value, the binding branch and a branch that sets the
value with the template priority. The provided value of a template binding is the binding
itself, never null, so the binding branch is the one that runs. With
`DEFAULT_RESOLVED_TEMPLATE_BINDINGS = true` (`rust_emitter/theme_styles.rs`; or
`with_resolved_template_bindings` for the emission of a closure) the nine statements are
`rt::bind_template_binding(..)`, the statement of a property with one setter, and they join the
runs of template bindings of their object. The WinUI theme has 141 of these choices, Fluent 88,
Simple 82, macOS 134. A choice whose locals are named elsewhere, or of any other shape, stays
as it was. `scripts/themes/repeated_sequences.py` counts the repeated sequences of the template
functions in the source and in a linked executable, for the next shared sequences.

### 2.2 Lazy resource rows: indexed deferred groups

A brush row (literal, `{StaticResource}` or `{DynamicResource}` colour) and a `{StaticResource}`
alias row are built when the resource is first looked up. The generated code is a static table
of rows for each shape and one call of the shared builder (`rt::add_literal_brushes`,
`rt::add_static_brushes`, `rt::add_dynamic_brushes`, `rt::add_resource_aliases`) with the
dictionary, the context of the declaration and the table. What the builder puts into the
dictionary:

- **One group for the table.** The builder creates one object for the whole table (an
  `IDeferredResourceGroup` behind a `DeferredResourceGroup` handle, `ferroui-base`,
  `controls/i_deferred_content.rs`) that holds the static rows and, for each row, the capture of
  its declaration: the name scope, the root object and the resource nodes above it, all held
  weakly. The capture is taken immediately before the row is added, so a handler of the
  dictionary's change notification that moves the parents between two rows is seen by the next
  row.
- **One entry for each row.** `ResourceDictionary::add_indexed_deferred(key, group, index)` adds
  an entry that is the group and the index of the row. It is the size of an entry with deferred
  content, and adding it allocates nothing for the row: the capture is a slot of the group's
  vector, and rows declared under the same parents share the list of resource nodes.
- **First lookup.** The dictionary asks the group to build the row
  (`IDeferredResourceGroup::try_build(index, ..)`), keeps the value in place of the entry, as
  for any shared deferred resource, and tells the group that the row is built
  (`IDeferredResourceGroup::release(index)`): the group drops the capture of that row and the
  other rows keep their indices. A build that fails reports its error to the error scope of the
  load and keeps the entry: the next lookup builds again. The group lives while an entry of it
  is not built.

A row falls back to deferred content of its own (`ResourceDictionary::add_deferred`, as the
other resources of the document) in three cases:

| Case | Why |
|---|---|
| The capture of the row owns a value: the root object or a parent is a plain value, not an object or a registered model | The value must be released when that resource is built, not when the last row of the table is |
| A development build with hot reload (`debug_assertions` and the `hot-reload` feature of `ferroui-markup-xaml`) | A reload replaces the entries of a dictionary one by one and uses the reload hooks of deferred content |
| A build with `source-overlays` (the designer) | The same hooks |

The fallback is decided for each row: the rows of one table can be of both kinds, and the
indices of the group stay those of the table. An application built for release has the indexed
entries; a workspace build that includes hot reload or the designer has the fallback in every
crate, because features unify. The tests of the indexed form are the library tests of
`ferroui-markup-xaml` built on its own (`cargo test -p ferroui-markup-xaml --lib`), which CI
runs as a step of its own.

A group is an implementation detail of the markup runtime. Application code adds lazy resources
with `ResourceDictionary::add_deferred` or `add_deferred_fn`.

**Packed tables (#855, on by default).** A row of the four deferred shapes is a struct of
sized strings, an optional namespace slice and 32-bit positions: 64 bytes for an alias and for
the brushes bound to a resource, 48 for a literal brush, each string a pointer the loader
relocates. With `DEFAULT_PACKED_THEME_ROWS` (`rust_emitter/theme_rows.rs`, on;
`with_packed_theme_rows(false, ..)` turns it off for the emission of a closure) a run of one of
these shapes is written as an `rt::PackedRows` instead: one pool of the strings of the table, each written once (a
string inside one already in the pool is found there), the namespace tables the rows name, and
one record of 16-bit fields for each row (key offset and length, target or colour key offset and
length or the two halves of the colour, the namespace table as its index plus one, the
positions). `rt::add_packed_resource_aliases`, `add_packed_literal_brushes`,
`add_packed_dynamic_brushes` and `add_packed_static_brushes` decode a record when its row is
added or built and do what the builders of the slices do, in the same order, with the same
errors at the same positions. A run with an offset, a length or a position beyond 16 bits, a
negative position or a key in a form the emitter does not write keeps its slice of rows. A
record that does not decode is the error of its table, not a panic. The packed tables of one
unit of a theme share one pool when the unit has two or more of them (`row_tables_in`): the
constant `{DOCUMENT}_UNIT_{n}_STRINGS`, written before the first table of the unit, which each
table names in place of its own literal. The tables of the theme dictionaries of a unit (light,
dark, high contrast) name the same keys and targets, which a pool for each table repeated;
counted on the WinUI theme, the pools of all its tables are 266,234 bytes with one pool for each
table and 102,833 with one for each unit, and those of the 15 units hello world keeps 55,257
and 18,339 (one pool for the whole theme would be 69,411, kept whole by every application). The
pool is the unit's, so a trimmed unit takes its strings with it; a table whose offsets do not fit
16 bits in the pool of its unit keeps a pool of its own. The helpers of the slices stay for the plain values.
It is on because the measurement kept it: on macOS the stripped executables were smaller in
every measured application (release: hello world 20.44 to 20.40 MB, the WinUI gallery 34.25 to
34.15 MB, the catalog 77.04 to 76.92 MB; dist alike), and start-up was not slower. The
typed backend encodes the same runs as its programs; with it no packed table is written.

### 2.3 Applying styles: a batch reevaluates only when it changed something

An element changes its styles in a batch (`ValueStore::begin_styling`, `end_styling`): applying
its styles and control themes attaches the frames of those that match, invalidating its styles
or its control theme removes their frames, and the effective values are computed once, when the
outermost batch ends. The end of a batch reevaluates the values only when the batch changed
something: a frame was inserted, or a reevaluation was put off while the batch was open (a frame
removed, a binding of a style that produced a value, a style that became active or inactive). A
batch that changed nothing leaves the effective values untouched.

For a theme this means that an element no style or control theme matches, and an invalidation
that reaches an element holding no frame of the kind it removes (an invalidation of styles
recurses over the tree), cost the matching of the selectors and nothing in the property system.
The deviation from upstream, which reevaluates at the end of every batch, is recorded in
[docs/porting/desktop-performance.md](porting/desktop-performance.md), section 9.25.

### 2.4 Applying styles: the selector candidate index

A control theme of a control with a large template has many nested styles, and most of them name
a part of another type than the element being styled: of the nested styles of a button's theme, a
`Border` in the template can match only those whose selector ends in `Border`. Matching every
nested style against every element of the template is the cost the index removes.

**What is indexed.** A compiled control theme table index-enables each nested style it builds,
right after it sets the selector and before the style joins the children of its parent
(`theme_styles.rs`, `Style::enable_selector_index`, a hidden method). An index-enabled style
records the type its selector requires of the element it matches, when the selector has one: the
last compound of the selector must start with a type, exact (`Border`) or with derived types
(`:is(Border)`); a name, classes, property conditions and nth-child conditions may follow in the
same compound, and anything may come before the compound's combinator
(`^ /template/ Border#PART_Border:pointerover`). The collection that holds such styles (the
children of a control theme or of a style, a `Styles` collection) keeps, for each style key it is
asked for, the list of its entries that can match an element of that key, in source order, and
applying styles visits the entries of the list.

**How compiled markup names the type.** A type selector of compiled markup, in a generated
function or in a static selector chain, names its class by the identity of the class
(`Selectors::of_type_identity`, `Selectors::is_type_identity` with `ObjectType::IDENTITY`), not by
its `TypeInfo`. The selector matches by the identity and the index files it by the identity, with
the results they had by type. The `TypeInfo` of a class names the initialisation of the class, so
a style written with it kept the class, its base classes, their properties and their handlers in
the executable whether or not the application creates one; a style that only selects a class no
longer does (`docs/binary-size.md`, section 4.20). `Selector::target_type` of such a selector is
the `TypeInfo` once the class is initialised and `None` before.

**What falls back.** An entry is left out of a list only when it is an index-enabled style without
child styles and the type of its selector rules the style key out. Everything else stays in every
list and goes through the ordinary matcher:

- a style that was never index-enabled (every style the run-time loader, hot reload or code
  creates), and a style whose selector was replaced after it was enabled;
- a selector that gives no type to index by: an OR of selectors (`,`), `:not()`, a last compound
  that starts with `^` (`^:pointerover`), with a name or a class alone (`/template/ #PART_Border`)
  or with a custom selector node;
- a style with child styles, because the type of a parent says nothing about its children;
- a style whose parent is a container query, which is evaluated before the selector;
- an entry that is not a style: a nested `Styles` collection, a style include, a custom `IStyle`;
- every entry of a collection with fewer than eight entries.

The index reads the type of the selector and nothing else: classes, names, pseudo-classes and
property conditions are evaluated by the matcher, with their activators, for every style in the
list. The styles of the list apply in source order with the priorities they always had, so the
values of an element are the same with and without the index.

A selector in a nested style of a theme that names the type of the part
(`^ /template/ Border#PART_Border`) is indexed; one that names only the part
(`^ /template/ #PART_Border`) is matched against every element of the template.

**Invalidation.** The lists belong to the collection. They are dropped when

- the collection changes (an entry is added, removed, replaced or moved, or it is cleared);
- a style the collection counts as index-enabled gets another selector, is index-enabled again,
  or gets another parent;
- the child styles of such a style change.

A change of any other collection, or of a style in another collection, leaves the lists in place:
there is no state shared between collections. A traversal that is under way when one of these
happens (a setter applied by an earlier style can run code that edits a later one), or during
which the style key of the element changes, goes on from its position through every remaining
entry of the collection.

A collection counts a style as index-enabled when the style is added to it while it is enabled.
A style that is enabled for the first time after it was added is matched for every element until
it is added again, and a style that two collections hold is narrowed by the first of them only.
Generated code enables before it adds; nothing else calls the method.

**Cost.** A collection that never held an index-enabled style allocates nothing for the index:
asking it for the candidates of an element is one check, and the collection is traversed as it
always was. This is the case of the styles of an application, of a document loaded at run time
and of a theme replaced by a hot reload. A collection with index-enabled styles shares one small
state with them (each `Style` holds one pointer to it), scans its entries once for each style key
it meets and remembers the result: up to 1024 keys for each collection, and keys with the same
candidates share one list of entry indices. A key beyond the bound gets its list computed at each
use, and nothing that is remembered is dropped for it.

The deviation from upstream, which matches every child style against every element, is recorded
in [docs/porting/desktop-performance.md](porting/desktop-performance.md), section 9.26.

### 2.5 Typed construction programs (opt-in)

`Build::xaml_backend` (or `FERROUI_XAML_BACKEND` read by `Build::from_env`) selects how the
operations the compiler resolved are emitted. `native`, the default, is the generated code of
section 2.1. `theme-bytecode` encodes resource runs and recognised control styles as instruction
streams; `bytecode` also encodes construction, resolved calls, staged setters, the extracted
theme unit runs and compiled binding paths of pages and templates. The streams are constant
arrays of the generated Rust (a Cargo build packs a module's streams in `OUT_DIR` and embeds them
with `include_bytes!`); the typed imports they call are what the linker reaches, so units and
trimming (section 3) are unchanged, and a shape the encoder does not recognise stays native. The
checked-in references of section 2 are native output: with another backend their comparison
tests fail by design. Design, coverage and measurements:
[docs/porting/typed-xaml.md](porting/typed-xaml.md).

## 3. Units and theme trimming

**Units.** The documents a theme merges (`MergeResourceInclude`) are each a unit, such as
`Controls/Button.xaml` and `Resources/Button_themeresources.xaml`. One more unit, the ambient
unit (index 0), holds what is always needed:
- the class document;
- the host documents (`Window`, the adorner and embedding hosts, the theme variant scope); the
  hosts of popups are units of their own, kept where a popup is live (#994);
- documents with entries no key can name.

The `.xamlmeta` records each unit's unit graph: the keys and type keys it defines, the keys it
references, and the types it constructs or selects.

**Trimming is the default.** When an application's compiled markup constructs a theme
(`<WinUITheme />` in `App.xaml`), the generated code calls
`WinUITheme::with_parts(sp, &THEME_PARTS_..)` with a table of only the units the application
needs. The build computes the table from the roots:

1. the types the application's documents construct, select or name in `{x:Type}`, with their
   base classes;
2. the keys and key-like tokens of those documents;
3. the types and keys the compiled documents of its dependencies name (a control library's
   templates); for the document of an ordinary class of a dependency (a dialog, a window), only
   when the class is live in the application: the application names it (`AboutFerroDialog::new()`,
   the builder `about_ferro_dialog()`), or a live class or document constructs it (#1002). A crate
   whose own code constructs such a class states it (`XamlGroup::constructed_by_its_crate`), and
   its documents then root the themes whenever the crate is linked;
4. the class names and string literals in the application's Rust sources that match a type key
   or a key of the theme (`Button::new()`, `"AccentButtonStyle"`), and the functions and
   methods the sources call, read as class names (`text_block()` is `TextBlock`,
   `.tool_tip("..")` is `ToolTip`): the builders of the declarative layer are named after the
   classes they make. A typed key of a theme is read exactly, as what follows `keys::` in a path,
   a `use` item or a macro (`keys::ButtonBackground`, #1076). An identifier that merely equals a
   key is no root, unless the crate imports `keys::*` or `keys as k`, which hide the names;
5. the creates edges of the type models: the controls a class's code constructs (`ListBox`
   creates `ListBoxItem`), transitively. An edge into a class that no code constructs on the
   target of the build does not hold there (`Export::not_created_on`, #1002): the host of a window
   creates its drawn decorations (`WindowDrawnDecorations`, with the caption buttons and the title
   bar resources they need) everywhere but on macOS, iOS, Android and the browser, so applications
   for those targets keep that theme only when they name it;
6. the names in `[package.metadata.ferroui] theme-roots = [..]`.

From the roots, the closure follows the unit graph. The kept units are listed in
`$OUT_DIR/xaml/theme_parts.txt`. `$OUT_DIR/xaml/theme_parts_why.txt` (#852) explains each one: its
first reason, whether that reason is conservative or exact, and how many of its keys are live.
See docs/porting/theme-trimming.md, section 19.

**Which classes are constructed from units (#849).** Partitioning a class into units and
constructing it from some of them are two separate things. Any class whose document merges
another one has units: a theme, but also an application class or a user control with a merged
dictionary. Only a class with a public associated function
`with_parts(Option<Rc<dyn IServiceProvider>>, rt::ThemeParts) -> Ref<Self>` can be constructed
from selected units, and the generated code calls `with_parts` only for such a class. Every
other class is constructed with `new()` and keeps all of its units, however its markup is
partitioned. The build of the class's crate checks the signature. A public `with_parts` with two
parameters and another signature is a build error, because other crates would call it. The
crates that construct the class read this capability from the class's type model in the
`.xamlmeta` (its public functions). An ordinary class is constructed whole, so the references of
its documents are exported to the applications that depend on its crate, like the references of
a crate without units. Only the documents of a class constructed from units are rooted through
its unit graph.

**Generated names are allocated, not assumed (#849).** One allocator names the build and
populate functions of the documents of a group (with their untyped, deferred, part and unit
functions) and the parts tables of `compiled_markup`. A name that no other item of the module has
keeps its established form, so every existing name stays the same (`THEME_PARTS_` and the full
name of the class in upper case; `build_` and the document path in snake case). When names
collide, every one of them gets `_` and the FNV-1a hash of its document path or class full name,
in eight hexadecimal digits, whatever the order. `A-B.xaml` and `A_B.xaml` get
`build_a_b_xaml_<hash>` and `build_a_b_xaml_<other hash>`. `A.B_C` and `A_B.C` get
`THEME_PARTS_A_B_C_<HASH>` and `THEME_PARTS_A_B_C_<OTHER HASH>`. The `.xamlmeta` records the
final paths. Two cases are errors that name both places instead:
- two public documents whose URIs differ only in case, which the loader table cannot tell apart;
- a theme named in `Build::theme` whose established table name another theme of the dependencies
  also has, because the application's code names the table by that name.

**An application without markup.** An application whose interface is Rust code (the builders and
`rsx!` of `ferroui-view`) constructs its theme in code, and `WinUITheme::new()` is every unit of
the theme. Its build script names the theme, and its code constructs the theme from the table
the build writes:

```rust
// build.rs; the manifest has ferroui-build under [build-dependencies] and ferroui-markup-xaml
// under [dependencies] (the table names the type of a unit).
fn main() {
    ferroui_build::Build::from_env().type_system(ferroui_build::TypeSystem::Model).theme("WinUITheme").run();
}

// main.rs
ferroui_markup_xaml::include_compiled_xaml!();
use compiled_markup::THEME_PARTS_FERROUI_THEMES_WINUI_WINUITHEME as THEME_PARTS;

launch_themed(|| WinUITheme::with_parts(None, THEME_PARTS).as_style(), view, |_| {});
```

Since #847 the build reads every binary of the package as well as its library: the explicit
`[[bin]]` targets and, unless `autobins = false`, the discovered ones (`src/main.rs`, `src/bin`).
`$OUT_DIR/xaml/theme_roots.txt` says which target rooted which types and keys. A library whose code
makes controls that no document names states them with `Build::theme_roots`, and the build exports
them to the applications that depend on it. The documents of a class group that the class never
includes root nothing.

The roots are the ones of the list above without the documents: what the sources name and call
(4), what the code of the live classes creates (5), and `theme-roots` (6). The build reads the
sources from the root file of the crate (`src/lib.rs`, else `lib.rs`, else `src/main.rs`) and
the modules it declares, and from the root file of each binary of the package (#847). The table is named after
the full name of the class of the theme, in upper case. With `theme-trimming = false` it holds
every unit, so the code is the same either way. The three declarative versions of hello world
(`samples/HelloWorld.Builders`, `.Macros`, `.Hooks`) are built this way, and keep the 16 units of
the WinUI theme the XAML version keeps.

**Reflection-binding paths follow their unit.** A path such as
`{Binding (controls:TextBox.Text)}` names a class. The generated code yields it through a function
in a namespace table, so the class (its initialisation and markup; its constructor only with
`type-tables`, #612 B3) is linked wherever the table is reachable. The table of the document carries only the classes named by paths
outside every unit. Each unit whose documents have such paths gets a table of its own, which its
entries push around their code and give to their templates and deferred resources. A unit the
application trims takes the classes its paths name with it (#612, root D: hello world no longer
links NavigationView, RichEditBox, InfoBar, NavigationPage and DrawerPage). The theme tests check
that no table naming classes is reachable from code outside the units.

**A library that constructs a theme (#846).** A library document with `<WinUITheme />` constructs
the theme with the wider of its own table and the table the application registers
(`rt::wider_theme_parts`). The application writes and registers a table for every theme the
documents of a direct dependency construct, so its controls keep their templates whichever crate
constructs the theme. The theme crate must be a direct dependency of the application; otherwise the
library keeps its own table. See docs/porting/theme-trimming.md, section 20.

**Themes used together, and types of one name (#848).** An application that uses several themes
closes over all of them together: a key one theme references and another defines, or a type one
constructs and another styles, keeps the providing unit. Types of one local name in two assemblies
keep their own base classes and creates edges. See docs/porting/theme-trimming.md, section 21.

**How the build reads its inputs (#850).**
- The includes that make up the group of a document are found by the XML parser of the compiler,
  so entities are decoded and CDATA and comments are skipped. The build and the run-time loader use
  the same rules.
- The `[package.metadata.ferroui]` table is read as TOML, so dotted keys, inline tables and string
  escapes all work.
- A document consumed by a class (merged into it as well) is not embedded again by
  `Build::embed_assets`.

Details are in docs/porting/theme-trimming.md, section 22.

**Turning trimming down or off:**
- `theme-roots = "all"` keeps every unit, for comparison.
- `theme-trimming = false` turns trimming off.
- A crate that can show any control enables `ferroui-base/full-themes`, and every unit is kept.
  These are the run-time XAML loader, hot reload, the designer, DevTools, Inspect and
  Diagnostics. Looking types up by name (`type-tables`) does not keep every unit:
  `ferroui-view` and the behaviors only need the lookup, and their applications are trimmed.
- The tables of types themselves keep every class of the framework they register, units or not.
  Bounded reflection (#851, an experiment) builds with `--cfg ferroui_bounded_reflection` in
  `RUSTFLAGS`: `type-tables` then registers no table of the framework and keeps no default
  constructor, as without the feature, and the application registers what its dynamic use needs
  one by one (`TypeInfo::register_all` for a type found by name, such as the owner of an
  attached property a behavior names; `TypeInfo::register_constructor` for a class created by
  reflection; `MarkupType::register_all`). The members a behavior finds by name on a value
  (`find_markup_property`, `find_markup_methods`, `find_markup_event`) come from the markup
  metadata of the value's own type and need no table.

**What the registration of a crate keeps (#857).** `compiled_markup::register()` registers the
loader table of the crate and its embedded assets. Without a plan the loader table answers
every public document of every exported group and the asset table holds every embedded asset,
so a load by URI keeps every build function and every payload. A crate that loads a known set
of documents by URI and reads a known set of assets states it in its build script:

```rust
ferroui_build::Build::from_env()
    .export_plan(
        ferroui_build::ExportPlan::closed()
            .document("ferres://myapp/Views/Settings.xaml")
            .asset("/Assets/logo.png"),
    )
    .run();
```

The loader table then answers only the listed documents, each through its own build function
(or the loader of its class), after the checked-in tables, with the same comparison without
regard to case and the same root service provider; the asset table holds only the listed
assets. A listed URI that is not a public document of an exported group, and a listed asset the
crate does not embed, fail the build. Code that names a document's build function or class
still links it. A library whose documents others load by URI keeps the default
(`ExportPlan::all()`). Either way the build writes `$OUT_DIR/xaml/exports.txt`: every compiled
document with its build or populate path, whether it is public and whether the loader table
answers it, and every embedded asset with its size and whether it is registered.

**When a unit is missing.** A debug build logs it once ("... was trimmed") with the control's
name, the first time the control looks for its theme. Release builds do not check.

**How a build applies its runs.** The function of a document that merges units calls
`rt::apply_theme_unit(active_theme_units(), unit, run, ..)` where the entries of a run were:
the call looks the active table up and searches it for the unit, and a unit the application
trimmed adds nothing. The WinUI theme has 347 such calls in its two population functions, and
hello world keeps 16 of its 166 units. An experiment of #853 changes two things, each with a
switch of `ThemeUnitPlan` (`rust_emitter`), both off: the generated files are what they were.

- `keys_in_runs`: every merged `*_themeresources.xaml` assigns the key of its theme dictionary
  (`<ResourceDictionary x:Key="Default">`) to the theme dictionary of the theme it was merged
  into. The merge joins only dictionaries with equal keys, so after the first assignment, which
  is outside every unit, the others write the value the dictionary has. With the switch they
  are entries of the unit of their document: they go into its run, and a trimmed unit takes
  them with it. Without it they stay in the population function, one between each two calls.
  Since #994 the merge leaves these repeated assignments out (an equal `Key` is not assigned
  again; docs/porting/xaml.md, deviation 28), so the switch has almost nothing left to move:
  each theme variant assigns its key once, before its first run.
- `regions`: calls of runs that add to one dictionary, with only comments between them, are
  one call of `rt::apply_theme_region(active_theme_units(), &[(unit, run), ..], ..)`. It
  applies the same runs in the same order and stops at the same failure; the units of a region
  ascend, and it walks the table once. A region needs its indices in 16 bits, else its calls
  stay. A call alone stays a call of `rt::apply_theme_unit`.

Turning it on: `DEFAULT_THEME_UNIT_PLAN = ThemeUnitPlan::ON` in
`rust_emitter/theme_units.rs`, then the checked-in generated files are regenerated (section 2,
and `tests/FerroUI.Markup.Xaml.UnitTests/emitter/generated.rs` where it changes). Whether it
stays on is decided by the size of the stripped hello world and of a larger sample, and by the
start (`docs/binary-size.md`).

## 4. Rules for theme markup

### 4.1 Document structure

- **One control per document.** `Controls/Button.xaml` holds Button's control themes and the
  resources only Button uses. Merge it into the theme with `MergeResourceInclude`. One unit per
  document lets an application drop a control it does not use. A document with several unrelated
  controls is one unit, and they are kept together.
- **Shared resources in shared resource documents**, by area (colours, typography, the
  per-control `*_themeresources.xaml`). A key referenced by many controls belongs in a document
  that is merged as its own unit, not inside one control's document.
- **Keep the ambient unit small.** Only host-level styles and the structure of the theme
  dictionaries belong in the class document. Everything in it is linked into every application.
- **Name every entry.** An entry without a key, or with a key that is neither text nor
  `{x:Type}`, cannot be rooted, so its whole document stays in the ambient unit.
- **No `ResourceInclude` for a document that should be trimmed.** A `ResourceInclude` is not
  inlined, so the document it includes is part of the ambient unit. A document that must stay a
  dictionary of its own (its keys repeat keys of the theme's dictionary, or its `{StaticResource}`
  references must find the theme's overrides after their own document, as for the inherited
  documents of the Material theme) is merged into an inline dictionary of its own (#633):
  `<ResourceDictionary><ResourceDictionary.MergedDictionaries><MergeResourceInclude Source="…" /></ResourceDictionary.MergedDictionaries></ResourceDictionary>`
  as a merged dictionary of the theme's dictionary, where the `ResourceInclude` was. The lookups
  find its entries in the same order, and the document is a unit.
- **Mark every document but the class document internal** (`x:ClassModifier="internal"`). The
  loader table answers the URI of a public document and so keeps its build function, and its unit,
  linked in every application.

### 4.2 Resources

- Prefer the shapes that become rows: `SolidColorBrush` with a literal colour or a
  `{StaticResource}` or `{DynamicResource}` colour, `Color`, `x:Double`, `Thickness`,
  `CornerRadius`, `x:String`, and plain `{StaticResource}` aliases. Each costs a row of static
  data instead of a function.
- Use `{StaticResource}` when the value does not change with the theme variant, and
  `{DynamicResource}` only when it does. A dynamic
  reference subscribes to resource changes for every element that uses it.
- Do not build a key at run time (`format!("{name}Background")`). A computed key cannot be
  rooted. If code must look a key up, write it as a string literal, or add it to `theme-roots`.
- Define a geometry once as a keyed resource and refer to it, instead of repeating the same path
  data inline in several templates.
- Colours the system reports (the accent, the system colours of high contrast) come from a
  resource provider of the theme's Rust code (`Accents/SystemAccentColors`,
  `Accents/SystemColors` of the WinUI theme), merged into the theme's dictionary, never from a
  theme dictionary: theme dictionaries are searched before merged dictionaries, so a fixed value
  there would hide the provider. Reference them with `{DynamicResource}`; the provider raises a
  change only when its colours change.

### 4.3 Control themes

- Setter values that are constants, resource references or templates go into the static table.
  Any other value becomes a small function of that theme only. Keep setters declarative.
- Selectors are shared static slices. The same chain written the same way in several themes costs
  nothing extra.
- Animations, a `Resources` dictionary inside a `ControlTheme`, and setters that use the name
  scope keep the whole control theme as native code. Use them where the control needs them, not
  for convenience.
- `BasedOn` a theme of the same document, or of a shared document, so the edge stays inside the
  units that are kept anyway.

### 4.4 Templates

- **`{TemplateBinding}` for a property of the templated parent.** It is the cheapest binding: a
  static pair, applied in runs.
- `{Binding ..., RelativeSource={RelativeSource TemplatedParent}}` only when it needs a
  converter, a mode other than one way, or a sub-property path. It is compiled (section 5), with
  the template's `TargetType` as its source type.
- A part a template constructs in markup roots that part's theme through the unit graph. A part
  the control's Rust code constructs (`TextSelectionHandle::new()` in
  `text_selection_canvas.rs`) roots its theme through the creates edge of the class, so construct
  it with `X::new`, `X::builder` or `X::default` in the class's own file.

### 4.5 Bindings

- **Every `{Binding}` compiles.** Compiled bindings are the default of the build. A compiled
  binding resolves its path when the markup is compiled, to static property references and
  accessors. Nothing parses a path or looks a member up by name at run time.
- The source type of a binding comes from:

  | Binding | Source type |
  |---|---|
  | `RelativeSource={RelativeSource TemplatedParent}` | the `TargetType` of the template |
  | `RelativeSource={RelativeSource AncestorType=X}` (or `$parent[X]`) | `X` |
  | `ElementName=name` (or `#name`) | the type of the named element |
  | `RelativeSource={RelativeSource Self}` (or `$self`) | the element |
  | a path on the data context | `x:DataType` on the element, template or column, or of the nearest ancestor that has one |

- **Give every `DataTemplate`, item template and data column an `x:DataType`.** A data-context
  binding without one is a compile error ("Cannot parse a compiled binding without an explicit
  x:DataType directive").
- `{ReflectionBinding}` is for the rare binding whose source type cannot be known (a template
  shared by unrelated item types). Write it explicitly, with a comment saying why. Never turn
  compiled bindings off for a whole document (`x:CompileBindings="False"`).

### 4.6 Values

- Write values in the forms the compiler converts at build time: numbers, colours, thicknesses,
  enumerations, durations, key splines, font weights, `{x:Static}` and `{x:Type}`.
- Path data (`Data="M 0 0 L 10 10"`, `<StreamGeometry x:Key="..">M ..</StreamGeometry>`) is
  parsed when the markup is compiled, by the parser of the run-time library: the generated code
  builds the geometry from a constant table of the commands the parser recorded
  (`PathCommand`, `StreamGeometry::from_commands`), and nothing parses the text at load. Text
  the parser refuses is left to the run-time parse, which fails the load with its error: fix the
  path data. Still prefer a keyed geometry shared by the templates that draw it, so that the
  table and the geometry exist once.
- Converters belong in resources (`<local:BoolToOpacityConverter x:Key=.. />`) and are referred
  to with `{StaticResource}`. They should not be created inline in every binding.

### 4.7 The theme's Rust code

- The theme class and the controls it styles must not look types up by name (`TypeInfo::find`,
  `find_by_name`). If a control needs a type, it names it (`X::TYPE`).
- A control that creates another control in code does it in its own file, so the type models
  record the creates edge (4.4).
- No `thread_local!` (use `ferro_thread_local!`), no panics for markup errors: return
  `XamlLoadError` (the move to `panic = "abort"` in `dist`).
- A theme with a high contrast variant sets it with `ThemeVariant::set_high_contrast_variant`
  while it is in use (the WinUI theme does so with `UseSystemHighContrast`); the theme variant
  roots then resolve to it while the platform asks for high contrast. A theme without one sets
  nothing, and the contrast setting selects nothing for it.

## 5. Compiled bindings in applications

The build compiles bindings by default for every crate: themes, control libraries, applications
and the templates of `FerroUI/templates`. Opt out per crate with
`[package.metadata.ferroui] compile-bindings = false` or
`ferroui_build::Build::default_compile_bindings(false)` (the manifest key, when it is there, wins
over the call of the build script), and per element with `x:CompileBindings="False"`. The class
file the reference tests of a theme generate (`generate_class_file`) compiles bindings unless the
assembly states `FerroUseCompiledBindingsByDefault = False` in its metadata. A build script
does not call `default_compile_bindings(true)`: it is the default.

Documents loaded at run time bind the way the built application does (#604):

- **Hot reload.** The hot-reload manifest the build writes records the setting of the build
  (`"compile_bindings"`), and `ReloadManifest::configuration(assembly)` is the configuration of the
  run-time loader for the reloaded documents.
- **The previewer of the designer** (`DesignWindowLoader`) and other run-time loads of the
  documents of an application (`RuntimeXamlLoaderConfiguration::for_assembly`) read the metadata
  of the application's assembly (`MarkupAssembly::compiles_bindings_by_default`): compiled
  bindings, unless `FerroUseCompiledBindingsByDefault` is `False`. The build does not write the
  `MarkupAssembly` static of a crate, so a crate that opts out with `compile-bindings = false`
  states `False` in its metadata as well, for the previewer to follow it.
- A configuration made with `RuntimeXamlLoaderConfiguration::new()` keeps upstream's default
  (reflection bindings), for documents that belong to no application.

### 5.1 Property lookups by name in compiled themes (#603)

Two run-time helpers of compiled markup still find a declared (plain) property by its name:

- `rt::clr_property_info`: the provide-value target of a nested markup extension, such as
  `TemplateBinding.Converter` with a `{StaticResource}`, or `MultiBinding.Bindings` and `Converter`.
- `rt::path_property`: a plain property in a compiled binding path, such as
  `ProgressBar.TemplateSettings`, `CommandBar.VisiblePrimaryCommands` or the view models of the
  managed file chooser.

`python3 scripts/themes/by_name_lookups.py` counts the calls in the checked-in compiled themes;
`--by-property` breaks them down by owner and property. On the branch of the change the counts were:

| Theme | `rt::clr_property_info` | `rt::path_property` |
|---|---:|---:|
| WinUI | 70 | 59 |
| Fluent | 65 | 53 |
| Simple | 36 | 43 |
| macOS | 67 | 43 |
| All four | 238 | 198 |

**Resolving the property at compile time.** The emitter can resolve the declared property at compile
time wherever the type system knows the declaring type. It then writes
`rt::declared_clr_property_info` and `rt::declared_path_property`, which take the position of the
property in its declaration. At run time these helpers take the property at that position and do not
search the declaration by name. If the declaration changed since the compilation and another
property is at that position, they still find the property by name. Both forms give the same
description, which is cached once per property and thread.

**Off by default.** The option is off by default, because it changes the generated files. To turn it
on:

- in a build script, call `Build::compile_time_declared_properties(true)`;
- in a host of the emitter, call `rust_emitter::set_compile_time_declared_properties(true)` around a
  compilation.

Turning it on for the themes means regenerating their checked-in `compiled_xaml.rs` (with the
regeneration of each theme's `compiled_xaml_tests`) and the emitter corpus
(`tests/FerroUI.Markup.Xaml.UnitTests/emitter/generated.rs`). The typed accessors that would also
drop the untyped accessors of the declaration are a further step (the next row of the table in
section 9).

## 6. Porting a theme from upstream

- Port each control document to its own document and merge it with `MergeResourceInclude`, as
  upstream merges it. Keep keys and resource names as upstream has them: applications look many
  of them up by name.
- Upstream themes use `{Binding}` with `RelativeSource TemplatedParent` and data-context bindings
  without `x:DataType` that upstream resolves by reflection. Keep the binding. Add `x:DataType`
  where the data type is known, or write `{ReflectionBinding}` with a comment where it is not.
- A document that cannot be compiled yet (a missing control or markup feature) goes into
  `Controls/excluded.txt` with what it is missing, not into the theme with a workaround.
- Record each difference from upstream in [docs/porting/DEVIATIONS.md](porting/DEVIATIONS.md), as
  the [porting guide](porting/PORTING-GUIDE.md) requires.

## 7. Checklist for a change to a theme

1. Build the theme crate and run its tests: the reference tests, `control_theme_tests`,
   `resource_dictionary_tests` and `load_time`.
2. Regenerate the checked-in reference (section 2) and read its diff. A new deferred function, a
   new native control theme or a new reflection binding is a regression unless it was intended.
3. Check `theme_parts.txt` of `samples/HelloWorld` (WinUI) or another small sample of the theme:
   the units it keeps should not grow unless the change is meant to add to every application.
4. Run the theme's gallery and the ControlCatalog page tour and compare screenshots. A debug run
   must log no "was trimmed" diagnostic.
5. For a change that may affect size, measure `samples/HelloWorld` with `--profile dist` before
   and after, and attribute with `scripts/symsize.py`. Record the result in
   [docs/binary-size.md](binary-size.md).

## 8. Diagnosing

| Symptom | Cause | Fix |
|---|---|---|
| A control has no theme in a release build; a debug build logs "... was trimmed" | Its type or key is not rooted: created from a computed name, or only in a crate the build does not scan | Name it in code or markup, or add it to `theme-roots` |
| "Static resource 'X' not found" at start-up | A document refers to a key whose unit was not kept | Check that the referring document is compiled (application or dependency); add the key to `theme-roots` while the root computation is fixed |
| "Cannot parse a compiled binding without an explicit x:DataType" | A data-context binding with compiled bindings on | Add `x:DataType`, or `{ReflectionBinding}` with a comment |
| Every unit kept (`N of N units` in `theme_parts.txt`) | A dependency enables `ferroui-base/full-themes` (a run-time loader, the designer, DevTools), or `theme-roots = "all"` | Expected for tools; ship without them |

## 9. Status

| Part | Issue / PR | State |
|---|---|---|
| Tables of types trimmed by default | #566 | On `main` |
| Bounded reflection: `type-tables` without the tables of the framework (`--cfg ferroui_bounded_reflection`, an experiment) | #851 | In review, to be measured |
| Resource and constant rows (step 1) | #568 | On `main` |
| Units and the unit graph, all four checked-in themes (step 2) | #568 | On `main` |
| Root tables, `with_parts`, debug diagnostic (step 3) | #568, #598 | On `main` |
| The runs of a build as regions, key assignments of merged theme dictionaries in their units (`ThemeUnitPlan`, off) | #853 | In review, to be measured |
| `with_parts` only for a class that has it (the capability in its type model); one allocator for generated names | #849 | On `main` |
| Includes found by the XML parser; the manifest table read as TOML; consumed documents not embedded again | #850 | In review |
| Roots from code: creates edges, source names and literals (step 4) | #568, #600 | On `main` |
| A closed export plan of the documents and assets a crate registers (`Build::export_plan`, opt-in); `exports.txt` | #857 | In review, to be measured |
| Roots from the dependencies' documents; `full-themes` as the keep-all switch | #602 | On `main` |
| Theme trimming on by default (`theme-trimming = false` to turn off) | #602 | On `main` |
| Control theme tables, selector slices, StaticResource setter helper, template binding runs (step 5) | #568, #607 | On `main` |
| Template bindings whose setter is chosen at run time resolved (`DEFAULT_RESOLVED_TEMPLATE_BINDINGS`, off); the count of repeated template sequences | #854 | In review, to be measured |
| Namespace tables of reflection-binding paths per unit (root D) | #612 | On `main` |
| Constructors only with `type-tables`; compiled `{x:Type}` of a page registers its constructor (B3) | #612 | In review |
| Units for the Material theme (step 6): `MaterialTheme::with_parts`, control documents as units | #568 | In review |
| Material: the packed installer of the compatibility palette (`PACKED_INSTALLER`, off); the embedded font assets as the feature `embedded-font-assets` (on) | #856 | In review, to be measured |
| `MergeResourceInclude` inside an inline dictionary; the Material theme's inherited resource documents as units | #633 | In review |
| Compiled bindings in all themes | #603 | In review |
| Compiled bindings as the build's default; `compile-bindings` manifest key | #603 | In review |
| Samples, sample apps and component samples on compiled bindings; no explicit `default_compile_bindings(true)`; hot reload and the previewer follow the application's setting | #604 | In review |
| Templates on compiled bindings | FerroUI/templates#2 | Planned |
| Relative-source paths of the themes resolved at compile time (no `ReflectionBinding.RelativeSource` targets) | #603 | In review |
| `rt::clr_property_info` and `rt::path_property` counted (`scripts/themes/by_name_lookups.py`); declared properties resolved at compile time behind `Build::compile_time_declared_properties` (off by default) | #603 follow-up | In review |
| `rt::clr_property_info` of plain properties (nested markup extension targets, plain path elements) resolved to typed accessors | #603 follow-up | Planned |
| Path data converted at compile time instead of at load | #603 | In review |
| `panic = "abort"` in `dist` | #595 | On `main` |
| `panic = "abort"` in `release`, `release-unwind` | #567 | Planned |
| Theme crates at `opt-level = "z"` in `dist` | #581 | In review |
| Brush and alias rows as indexed deferred groups (section 2.2); styling batches reevaluate only when they changed something (section 2.3) | #670 | In review |
| Packed tables of the deferred rows (section 2.2, `DEFAULT_PACKED_THEME_ROWS`, on by default) | #855 | Kept by its measurement, on by default |
| Selector candidate index for the nested styles of compiled control themes (section 2.4) | #671 | In review |
