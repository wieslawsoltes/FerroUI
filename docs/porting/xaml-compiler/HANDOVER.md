# XAML ahead-of-time compiler: handover of the E1 draft

State at handover: the run-time loader (`FerroRuntimeXamlLoader`) is complete for what the two themes
and the ControlCatalog need and all its suites are green (stage 2b-10 is on main). The emitter exists
only as an UNCOMPILED draft. Nothing of the draft has been built or run. This document is everything
needed to continue without the local history.

Files of this handover:

- `e1-draft-on-main.patch`: the draft against main (the commit named at the top of
  `e1-draft-on-main.new-files.txt`). `patch -p1` from the repository root; it creates the new files.
- `e1-draft-on-main.new-files.txt`: the files the patch creates.
- `e1-draft.patch`: the original draft against 4850763-minus-2b-10 (history only; do not apply).
- `AUTHOR-REPORT.md`: the full report of the draft's author (file list, metadata additions, node
  coverage, unverified signatures).

Design documents in the repository: `docs/porting/xaml.md`, sections 9 (emitter design: call forms
A/B/C, build integration, code-behind), 9.5 (source scanner), 9.12 (the 14 rulings), 9.13, and
decisions 1 to 27. Read section 9 before touching the emitter.

## 1. Rules that are not negotiable

Naming and conduct (they apply to every file, comment, string and file name):

- The strings matched by `grep -rniE "avalonia|\bavn|axaml|avares"` must not appear. Upstream names
  map to `Ferro`/`FerroUI`, `Frn`; markup files are `.xaml`; the asset scheme is `ferres://`; the
  default namespace is `https://github.com/ferroui`. The upstream checkout
  (the upstream checkout described in `CLOUD-WORKERS.md`) is read-only reference.
- No `unsafe` outside the class model/FFI (the loader crate forbids it), no `todo!()` /
  `unimplemented!()`, no weakened or deleted tests, no `catch_unwind` in tests. Exact upstream
  semantics; upstream quirks are preserved and pinned by a test.
- `ferroui-markup-xaml-loader` must never depend on `ferroui-controls` directly.
- `classes.rs`, `enums.rs`, `named_values.rs` under `markup_types/` are GENERATED:
  `python3 scripts/generate_markup_types.py --upstream <upstream checkout>`; `--check` must pass;
  hand members go in `scripts/markup_types_overrides.py`.
- `samples/` belongs to the catalog worker (only un-ignoring gap tests is allowed); theme crates
  belong to the theme worker.
- G8 stays as upstream: a missing `StaticResource` inside a control template throws; outside
  templates it is delayed.

Rulings on the emitter (coordinator, on the owner's instruction "exact, high-performance port, no
compromise that is not needed"):

1. The emitter's output is REAL, TYPED Rust that rustc type-checks: direct constructor calls, direct
   typed setter/getter/add calls (forms A and B of section 9), typed constants, `x:Static` /
   `x:Type` as paths, name-scope registration, begin/end init, parent stack.
2. A frozen instruction list run by an executor ("build plan") is rejected, also as a first
   increment: it is a second interpreter. Call form C (by-name invokers through the metadata) is
   ONLY the fallback for a member that has no static path.
3. Rust paths and accessors come from the registries (recorded by the macros and the generator),
   never from string heuristics at emit time. The hand lists of the draft (`ROOT_RUST_PATHS`,
   `VALUE_RUST_PATHS`, `PUBLIC_MODULES`) are a stopgap to be replaced by mechanically derived tables
   in E2.
4. Generated sources of the test corpus are CHECKED IN and compiled into the test crate; a normal
   test fails when the emitter's output differs from the checked-in text (drift test); an ignored
   test regenerates the file. (Chosen over a build script: a build script that links the framework
   doubles the build; the generated text is reviewable in a pull request.)
5. The differential harness compiles the generated code for real and compares each document built
   both ways (run-time loader and generated function) by a canonical dump. "Syntax checked" is not
   coverage.
6. What is not covered is reported as NOT ELIGIBLE, never approximated.
7. From section 9.12: code-behind option A with the fixed field `xaml`; named-element accessors
   return `Ref<T>` and panic when unset; handlers found by scanning inherent impls;
   `CompiledXamlLoader` returns `Result`; `RuntimeContext` moves into the runtime library as the
   single `XamlIlContext`; `RuntimeTypeSystem` is re-seated on a shared `MarkupModel`;
   `rerun-if-changed=src` and a `links` key are acceptable; `register_types()` is generated for
   applications; hot reload behind a feature; `x:FieldModifier` mirrors upstream; marker comments
   plus a `.map.json`.

## 2. Stage plan and targets

Every stage ends with MEASURED numbers: documents that compile AND match the run-time loader in the
differential harness. The run-time loader stays fully working and green at every stage.

- E1: build the draft, regenerate `generated.rs` with the real emitter, harness green. Registered
  (styled/attached) properties, constants, names.
- E2: emission metadata done properly. Mechanically derived public Rust paths for every registered
  class, enumeration, value type and contract (all hand lists gone). Accessor text for plain
  properties, content properties and collection adds (the ~15 internal macros may be rewritten,
  with the full suites green). Direct properties.
- E3: markup extensions; bindings (compiled binding paths as typed accessors, as upstream's
  compiled bindings); resources (static/dynamic); `x:Static` / `x:Type`; parent stack / context.
- E4: templates and deferred content, styles and selectors, control themes, setters, includes and
  merged dictionaries: the node set the two themes need. Target: both themes and their suites pass
  when loaded from compiled output, with the run-time loader no longer linked into `themed_window`.
- E5: build integration (build-script helper plus include macro, section 9), code-behind, event
  handlers, `x:Class` documents; ControlCatalog on compiled XAML; the transform at build time
  without linking the framework (the 9.5 scanner) if E2's tables allow it.

For E3 to E5 report the startup and binary-size effect on `themed_window`. Baselines measured by
another worker: hello_window 190 ms / 22.9 MB; themed Simple 417 ms / 48.7 MB; Fluent 543 ms;
catalog 876 ms / 85.8 MB stripped.

Suite counts on main with 2b-10 (passed/ignored): base 3755/0, controls 3803/0, markup 255/0,
xamlx 99/0, markup-xaml 183/0, loader 371/1, XAML tests 546/11, Simple 195/2, Fluent 192/1,
control-catalog 435/156. Any stage must keep these (catalog and themes may only grow).

## 3. What the draft contains

New files:

| File | Content |
|---|---|
| `src/Markup/FerroUI.Markup.Xaml.Loader/rust_emitter/mod.rs` | module documentation, re-exports |
| `.../rust_emitter/emitter.rs` | `emit_document(root, configuration, function_name, document_name) -> Result<String, UnsupportedNode>`; the walk over the transformed tree |
| `.../rust_emitter/source.rs` | `rust_string_literal`, `function_name_of`, unit tests |
| `.../rust_emitter/compiled.rs` | `compile_documents`, `generate_file`: the functions, a `DOCUMENTS` table, `try_load`, `register_compiled_xaml` |
| `tests/FerroUI.Markup.Xaml.UnitTests/emitter/mod.rs` | why the output is checked in; the regeneration command |
| `.../emitter/corpus.rs` | 40 small documents; `EXPECTED_ELIGIBLE` (22), `EXPECTED_NOT_ELIGIBLE` (7) |
| `.../emitter/generated.rs` | A PREDICTION of the emitter's output written by a throwaway script. It may not compile and may differ from the real output |
| `.../emitter/differential_tests.rs` | drift test, ignored regenerate test, eligibility test, dump comparison, load by URI |

Changed files:

| File | Change |
|---|---|
| `src/FerroUI.Base/metadata/markup_type.rs` | `MarkupType.rust_path`, `MarkupType::register_rust_paths`, `rust_path_of_handle`, `rust_path()`; `MarkupConstructor.emit`; `MarkupEnumMember.rust_variant` |
| `src/FerroUI.Base/metadata/markup_macros.rs` | constructors record `stringify!($new)`; plain enumeration members record the variant identifier (helper `__ferro_markup_enum_variant_name!`), flags members `None` |
| `src/FerroUI.Base/type_system.rs` | `TypeInfo::register_rust_paths`, `TypeInfo::rust_path` |
| `src/FerroUI.Base/register_types.rs` | `types![TYPES, RUST_PATHS; ..]` yields both tables; hand list `VALUE_RUST_PATHS` |
| `src/FerroUI.Controls/register_types.rs` | same; `PUBLIC_MODULES` filter; hand lists `ROOT_RUST_PATHS` (12 classes), `VALUE_RUST_PATHS` |
| `scripts/generate_markup_types.py` | finds the type list in its new form (`types![TYPES`); merged by hand with the 2b-10 changes |
| loader `lib.rs` | `pub mod rust_emitter;` (feature `runtime`) |
| loader `runtime/interpreter/evaluators.rs`, `mod.rs` | `pub(crate) fn single_setter(..)`: the setter an assignment always uses when its plan leaves exactly one |
| loader `runtime/framework/methods.rs` | `RuntimeDocumentTypeBuilderProvider::transformed_root()` |
| loader `ferro_xaml_il_runtime_compiler.rs` | `transform_document(xaml, name, configuration)`: parse and transform ONE document exactly as `load_group`, return the root, the configuration and the type system; nothing built or cached |
| test crate `lib.rs` | `pub mod emitter;` |

The C003 part of the original draft (the `IBitmap` contract and its converter mapping) is NOT in
`e1-draft-on-main.patch`: it went to main with 2b-10.

Node coverage of the draft (see `AUTHOR-REPORT.md` for the exact list): a root object of an
object-model class with a default constructor and a public path; value-with-manipulation and
manipulation groups; object initialization (`begin_init` / `try_end_init`); a property assignment
with one direct-call setter over a styled or attached property (`target.set_value(Owner::x_property(), typed)`);
text, constants (string, double, boolean, 32/64-bit integers, plain enumeration by value), null,
`x:Static` of a plain enumeration member; vector-like constants and grid lengths through the
recorded constructor text; root-scope handling; name-scope registration with a text name.
Everything else returns `UnsupportedNode` (direct properties, hence `Name`; plain properties and
adders; markup extensions; templates; styles; resources; events; `x:Type`; flags enumerations;
constructor arguments; anything needing the parent stack).

Regeneration command (after the draft compiles):
`cargo test -p ferroui-markup-xaml-tests --lib emitter::differential_tests::regenerate_emitter_output -- --ignored --exact`.
If the checked-in `generated.rs` does not compile, empty its functions and its `DOCUMENTS` table by
hand first, then regenerate.

## 4. Signatures the author could not verify (check these first when building)

- The `types!` pattern `$(crate :: $($segment:ident)::+),*` matching every entry of the lists.
- Closure-to-function-pointer coercion inside the constant tuple arrays of `value_paths!`.
- `XamlAstNewClrObjectNode.constructor.as_any()`; `IXamlPropertySetter::as_any()`; `visit_node`
  accepting `&mut NeedsParentStack` and visiting the whole tree.
- `IXamlType::name()` / `namespace()` / `full_name()` as used in `emitter.rs`.
- In generated code: `Ref<T>::begin_init()` / `try_end_init()` reached by deref to `StyledElement`;
  `&Ref<Border>` coercing to `&StyledElement` for `NameScope::set_name_scope`;
  `INameScope::complete(&**scope)`; `ServiceProviderExtensions::get_name_scope(&**provider)`;
  `std::rc::Rc::new(value) as ferroui_base::BoxedValue`; `Type::new()` existing for every corpus
  class; attached definitions (`Grid::row_property()`) passing to `set_value` by deref coercion;
  `ferroui_base::input::InputElement`, `ferroui_base::layout::Layoutable`, `ferroui_base::Visual`
  being nameable from another crate.
- In the tests: `FerroPropertyRegistry::get_registered` / `get_registered_attached` returning
  `Rc<Vec<&'static FerroProperty>>`; `FerroList::to_vec()`; `RuntimeXamlLoaderConfiguration::new()`
  matching the defaults of `FerroRuntimeXamlLoader::load`; whether `xaml_test_base()` installs an
  `IAssetLoader` (if not, the URI test only checks `generated::try_load`).
- `CompiledXamlLoader` still returns `Option<BoxedValue>` in the tree (ruling: `Result`); the
  generated `try_load` panics with the message of a failed build until that is changed.

## 5. Why plain members and collection adds were stopped, and how E2 should do it

Reasons the draft stopped:

- In `ferro_class_info!` / `ferro_markup_type!` and the ~15 internal macros behind them
  (`src/FerroUI.Base/metadata/markup_macros.rs`) an accessor is captured as `$get:expr` /
  `$set:expr`. Once captured as `expr` it is opaque: the macro cannot tell a path
  (`Border::child`) from a closure, and cannot take the closure apart.
- `stringify!` of the accessor gives text relative to the imports of the DECLARING module; only the
  `Type::function` form can be made absolute mechanically. Counts by declaration form (approximate,
  path/closure): base accessors 159/566, methods 15/21; controls 85/117, methods 0/7; markup-xaml
  51/30, methods 2/9. Most accessors are closures.
- `Panel.Children` adds do not come from any declaration: the loader's type system projects a
  synthetic `FerroList<T>` type (the `Add` of the list) from the handle of the property value, so
  neither macros nor generator have a place to record text.

My view for E2 (in order of preference):

1. Record TYPED FUNCTION ITEMS, not text, as the primary form. For every plain property, method and
   constructor the macro already has the receiver type (`this:`) and the value/argument types. Make
   the macro expand each accessor into a named, public, monomorphic function in a generated public
   module of the declaring crate (for example
   `pub mod __markup { pub fn border__get_child(this: &Ref<Border>) -> Option<Ref<Control>> { ($get)(this) } }`)
   and record the PATH of that function (`concat!(module_path!(), "::__markup::border__get_child")`)
   in `MarkupProperty.emit_get` / `emit_set` / `MarkupMethod.emit`. The emitter then prints a direct,
   typed call to a real function whatever the accessor's form (path or closure), with no token
   munching and no dependence on the declaring module's imports. rustc inlines the wrapper. This
   needs `module_path!()` of a public module: the wrapper module must be re-exported from a public
   place (the crate root: `pub use` of the `__markup` module per declaring module, or one
   `#[doc(hidden)] pub mod __markup` per crate filled by the macro through an item-position
   expansion). The names must be unique per (type, member, overload index).
2. Where the accessor IS a path to a public inherent method, the generated call can name it directly
   (`Border::set_child`) as an optimisation later; correctness does not need it.
3. Public paths of types: do not keep `PUBLIC_MODULES`. 104 of the 250 entries of the controls type
   table are declaring-module paths behind private modules. Derive the public path mechanically:
   either (a) the generator reads the `pub use` / `pub mod` tree of each crate's `lib.rs` and writes
   a generated `rust_paths.rs` per crate (it already parses the Rust sources for member signatures),
   or (b) every class macro expands a `#[doc(hidden)] pub use` alias into the same per-crate
   `__markup` module and records that alias path. (b) is checked by rustc and survives moves; I
   would do (b) for classes and enumerations/value types alike, and drop the hand lists.
4. Collection adds: give the list projection a declared origin. `FerroList<T>` instantiations that
   markup adds to are either declared (`ferro_markup_list!`, the `FerroListOf<T>` declarations in
   `markup_types/plain.rs`) or synthetic. Make the synthetic projection carry generic emit text
   (`{target}.add({value})` with `{target}` typed `&FerroList<T>`): `FerroList<T>::add` is a public
   inherent method of a public type, so for THIS one type a template is exact. The getter of the
   collection property comes from item 1.
5. Direct properties: `set_value` does not apply; the property definition is
   `DirectProperty<Owner, T>` and the draft's convention `Owner::x_property()` still names it. Use
   the typed `set_and_raise`-backed setter through the property definition (check
   `FerroObject::set_direct_value` or the equivalent typed call the run-time path reaches through
   `set_value_untyped`), or item 1's wrapper of the CLR-style setter when the class declares one.

## 6. Things about the transformers and the interpreter a new worker would otherwise rediscover

Crates and layout:

- `xamlx` (`external/XamlX/src/XamlX`): parser, AST, transformers, type-system traits.
- `ferroui-markup-xaml` (`src/Markup/FerroUI.Markup.Xaml`): the runtime library (markup extensions,
  converters, `FerroXamlLoader`, `XamlIlRuntimeHelpers`, service-provider contracts). Compiled
  output must depend only on this crate and the framework, never on the loader.
- `ferroui-markup-xaml-loader` (`src/Markup/FerroUI.Markup.Xaml.Loader`): `compiler_extensions/`
  (the framework transformers, group transformers, binding-path helper and parser, the language
  definition), `runtime/type_system` (the type system over the registries), `runtime/interpreter`
  (evaluators for every node), `runtime/framework` (document providers, generated-method
  equivalents), `ferro_runtime_xaml_loader.rs`, `ferro_xaml_il_runtime_compiler.rs`. Feature
  `runtime`.
- Tests: `tests/FerroUI.Markup.Xaml.UnitTests` (`ferroui-markup-xaml-tests`), the theme crates and
  `samples/ControlCatalog`.

The type system (`runtime/type_system/runtime_type_system.rs`):

- Types come from two registries: `TypeInfo` (classes of the object model, with
  `markup: Option<&MarkupType>`) and `MarkupType` (everything declared with `ferro_markup_type!` /
  `ferro_markup_enum!`). A Rust handle type (`TypeId`) resolves to a type through
  `TypeInfo::find_by_handle`, then `MarkupType::find_by_handle`, then the nullable handle, then
  bindable arrays, then `Option<T>` of a reference type by NAME of the handle. A handle nothing
  knows becomes an OPAQUE type named by its Rust type name: when an error message shows a Rust path
  such as `core::option::Option<..>` as a setter parameter, a handle is not registered.
- `MarkupType::register_handle::<H>(type_)` (2b-10) adds a handle of another crate to a type
  (`AssignedBinding` is a handle of `BindingBase`).
- Registered properties are projected as a CLR-style property (`get_X` / `set_X` methods, dynamic
  invokers over `get_value_untyped` / `set_value_untyped`) plus the `XProperty` static field;
  attached ones as static `GetX` / `SetX` unless the metadata declares those accessors itself.
  `[AssignBinding]` comes from `FerroProperty::assign_binding()` or the declared attributes.
- Typed lists (`FerroList<T>`) are projected synthetically from the handle when not declared.
- Value conversion is `ValueTypes` (per thread; `register_global` reaches every thread):
  `try_cast` performs only assignability casts (registered casts, class upcasts by STATIC handle
  class, class to interface, nullable wrapping); `try_convert` adds registered conversions and ends
  with the target's `parse:`. Since 2b-10 `from_markup_value` (argument unboxing of every metadata
  invoker) additionally accepts a base-class handle whose OBJECT is of the class asked for.
- Arguments reach invokers through `to_exact(value, handle, index)` in `runtime_type.rs`; a failure
  is `MarkupInvokeError::Argument { index, expected, actual }`.

Loading and transforming (`ferro_xaml_il_runtime_compiler.rs`):

- Every load is a GROUP load: the documents named by `Source` of `MergeResourceInclude`,
  `ResourceInclude` and `StyleInclude` are found by scanning the text and pulled through the asset
  loader into the same group, so the group transformers (merge of resource includes, control-theme
  and style includes) see them, as upstream's build does. The compiled back end must reproduce the
  group, not single documents.
- There is a per-thread cache of transformed groups (`FerroXamlIlRuntimeCompiler::clear_cache`);
  `FERROUI_XAML_TIMINGS` prints parse/transform/build times. `XamlRuntimeIncludeFallback` covers
  includes not found at transform time.
- The transform is the same pipeline as upstream's compiler. After it, the tree contains ONLY
  resolved nodes: the emitter never resolves a name. Setter/adder selection is already done; an
  assignment carries its list of possible setters and `assignment_plan` (in `evaluators.rs`)
  decides which apply, by the STATIC type of the value where one setter remains and by the run-time
  type otherwise. `single_setter` exposes the first case; an assignment with several remaining
  setters needs a run-time type switch in generated code (upstream emits the same chain of type
  checks) and is "not eligible" in the draft.
- Order of operations of an object is fixed and must be mirrored exactly: constructor, push on the
  parent stack, `BeginInit`, assignments in document order (after the `DependsOn` reordering the
  transformers already did), `EndInit`, pop. Name-scope registration and root-object handling are
  explicit nodes.
- Deferred content (templates, deferred resources) is a node whose body is built later with the
  PARENT STACK captured at the point of definition; `StaticResource` walks that stack and the
  ambient providers. That is why E3 needs the context type before E4 can do templates.
- Compiled bindings: the path is parsed and typed at transform time
  (`xaml_il_binding_path_helper.rs`, `transformers/ferro_xaml_il_binding_path_parser.rs`); the
  interpreter builds `CompiledBindingPath` through `CompiledBindingPathBuilder`, using
  `MarkupProperty::typed_path_element` where a plain property has a typed form and an untyped
  element otherwise. The emitter should print the typed builder calls (upstream emits property
  accessors as static methods); the typed hook is the place accessor wrappers of section 5 plug in.
- The data type of a binding comes from `x:DataType`, the parent data context, or
  `InheritDataTypeFromItems` (with `AncestorType`): an items source WITHOUT an element type
  (`ItemsSource` declared untyped) gives no data type; declare typed lists
  (`ferroui_controls::ferro_markup_list!`).
- Markup extensions are run through `ProvideValue(serviceProvider)` with a service provider built
  from the context (root object, parent stack, name scope, base URI, type resolver, target
  property). `XamlIlRuntimeHelpers` in the runtime library has the helpers upstream's generated
  code calls; generated Rust should call the same ones.
- Type converters: the language definition maps types to converters
  (`ferro_xaml_il_language.rs`); text that the transformers can convert at transform time is
  already a constant node (vector-like constants, enumerations, grid lengths, numbers, brushes,
  colors); the rest is a converter call at run time with the culture and the type-descriptor
  context (base URI).
- Run-time errors of members declared with the `try` forms surface as failed invocations with the
  member's message; tests compare messages with upstream's.

Known open items outside the emitter (not started): converter culture; the remaining ignored tests
of the XAML test crate (most wait for the emitter); catalog gaps C101, C102, C201/C309
(`ArrayList`), C202 (`OnPlatform` element syntax), C203, C301 (method name for a delegate
property), C305 (`List<T>` with `x:TypeArguments`), C306 (`Slider.Ticks` from text), C310
(`TimeSpan` from text); C006/C007 belong to rendering.

## 7. First steps for the cloud worker

1. Apply `e1-draft-on-main.patch`; `cargo check -p ferroui-base -p ferroui-controls` (macro and
   table changes first), then the loader with feature `runtime`, then the test crate.
2. Fix signatures from section 4. Empty `generated.rs` if it does not compile; regenerate.
3. Make the drift test, the eligibility test and the dump comparison green; report measured
   eligible / matching / not eligible counts for the 40 documents.
4. Run the generator `--check` and the full suites; the numbers of section 2 must hold.
5. Then E2 as in section 5.
