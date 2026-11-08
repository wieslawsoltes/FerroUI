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
cargo test -p ferroui-themes-winui --test compiled_xaml_tests regenerate_compiled_xaml -- --ignored
```

Run the same command for `ferroui-themes-fluent`, `ferroui-themes-simple` and
`ferroui-themes-macos`. Commit the regenerated files with the change that caused them.

### 2.1 What the generated code is made of

| Part | Form | When it runs |
|---|---|---|
| Plain resources: brushes with a literal colour or a resource colour, `x:Double`, `Thickness`, `CornerRadius`, `Color`, `x:String`, `{StaticResource}` aliases | Static row tables, one shared builder per shape (`rt::add_literal_brushes`, `rt::add_double_resources`, ...) | Added at load; each row is built on its first lookup |
| Other resources | One deferred build function per resource (`rt::add_deferred_resource`) | On first lookup |
| Control themes | One static table per theme, interpreted by `rt::build_control_theme`; selector chains are static slices, shared when identical | Once per theme, on first use of the control type |
| Control templates | Native code | Every time a control applies its template |
| Template bindings in a template | Runs of (target, source) pairs, applied by `rt::bind_template_bindings` | With the template |
| Units | `THEME_UNIT_<n>` statics and the table `THEME_UNITS` | See section 3 |

A control theme whose content the table cannot express keeps native code for that theme only.
Today that means one with animations, a resource dictionary of its own, or a use of the name
scope.

## 3. Units and theme trimming

**Units.** The documents a theme merges (`MergeResourceInclude`) are each a unit, such as
`Controls/Button.xaml` and `Resources/Button_themeresources.xaml`. One more unit, the ambient
unit (index 0), holds what is always needed:
- the class document;
- the host documents (`Window`, `PopupRoot`, overlay and adorner hosts);
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
   templates);
4. the class names and string literals in the application's Rust sources that match a type key
   or a key of the theme (`Button::new()`, `"AccentButtonStyle"`);
5. the creates edges of the type models: the controls a class's code constructs (`ListBox`
   creates `ListBoxItem`), transitively;
6. the names in `[package.metadata.ferroui] theme-roots = [..]`.

From the roots, the closure follows the unit graph. The kept units are listed in
`$OUT_DIR/xaml/theme_parts.txt`.

**Reflection-binding paths follow their unit.** A path such as
`{Binding (controls:TextBox.Text)}` names a class. The generated code yields it through a function
in a namespace table, so the class (its initialisation and markup; its constructor only with
`type-tables`, #612 B3) is linked wherever the table is reachable. The table of the document carries only the classes named by paths
outside every unit. Each unit whose documents have such paths gets a table of its own, which its
entries push around their code and give to their templates and deferred resources. A unit the
application trims takes the classes its paths name with it (#612, root D: hello world no longer
links NavigationView, RichEditBox, InfoBar, NavigationPage and DrawerPage). The theme tests check
that no table naming classes is reachable from code outside the units.

**Turning trimming down or off:**
- `theme-roots = "all"` keeps every unit, for comparison.
- `theme-trimming = false` turns trimming off.
- A crate that can show any control enables `ferroui-base/full-themes`, and every unit is kept.
  These are the run-time XAML loader, hot reload, the designer, DevTools, Inspect and
  Diagnostics. Looking types up by name (`type-tables`) does not keep every unit:
  `ferroui-view` and the behaviors only need the lookup, and their applications are trimmed.

**When a unit is missing.** A debug build logs it once ("... was trimmed") with the control's
name, the first time the control looks for its theme. Release builds do not check.

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
- Path data (`Data="M 0 0 L 10 10"`) is parsed when it is loaded today (section 9). Prefer a
  keyed geometry shared by the templates that draw it.
- Converters belong in resources (`<local:BoolToOpacityConverter x:Key=.. />`) and are referred
  to with `{StaticResource}`. They should not be created inline in every binding.

### 4.7 The theme's Rust code

- The theme class and the controls it styles must not look types up by name (`TypeInfo::find`,
  `find_by_name`). If a control needs a type, it names it (`X::TYPE`).
- A control that creates another control in code does it in its own file, so the type models
  record the creates edge (4.4).
- No `thread_local!` (use `ferro_thread_local!`), no panics for markup errors: return
  `XamlLoadError` (the move to `panic = "abort"` in `dist`).

## 5. Compiled bindings in applications

The build compiles bindings by default for every crate: themes, control libraries, applications
and the templates of `FerroUI/templates`. Opt out per crate with
`ferroui_build::Build::default_compile_bindings(false)` or
`[package.metadata.ferroui] compile-bindings = false`, and per element with
`x:CompileBindings="False"`. Hot reload and the designer compile a reloaded document with the
application's setting, so it binds the same way.

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
| Resource and constant rows (step 1) | #568 | On `main` |
| Units and the unit graph, all four checked-in themes (step 2) | #568 | On `main` |
| Root tables, `with_parts`, debug diagnostic (step 3) | #568, #598 | On `main` |
| Roots from code: creates edges, source names and literals (step 4) | #568, #600 | On `main` |
| Roots from the dependencies' documents; `full-themes` as the keep-all switch | #602 | On `main` |
| Theme trimming on by default (`theme-trimming = false` to turn off) | #602 | On `main` |
| Control theme tables, selector slices, StaticResource setter helper, template binding runs (step 5) | #568, #607 | On `main` |
| Namespace tables of reflection-binding paths per unit (root D) | #612 | On `main` |
| Constructors only with `type-tables`; compiled `{x:Type}` of a page registers its constructor (B3) | #612 | In review |
| Units for the Material theme (step 6) | #568, #603 | Planned |
| Compiled bindings in all themes | #603 | Planned |
| Compiled bindings as the build's default; `compile-bindings` manifest key | #603 | Planned |
| Samples, sample apps and component samples on compiled bindings | #604 | Planned |
| Templates on compiled bindings | FerroUI/templates#2 | Planned |
| `rt::clr_property_info` resolved at compile time; remaining reflection paths parsed at compile time | #603 | Planned |
| Path data converted at compile time instead of at load | #603 | Planned |
| `panic = "abort"` in `dist` | #595 | On `main` |
| `panic = "abort"` in `release`, `release-unwind` | #567 | Planned |
| Theme crates at `opt-level = "z"` in `dist` | #581 | In review |
