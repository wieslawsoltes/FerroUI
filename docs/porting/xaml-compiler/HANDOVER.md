# XAML ahead-of-time compiler: handover

## State (read this first; every worker updates it before stopping)

- Stage E1 is DONE and merged into `main` (pull request #7).
- Stage E2 is DONE on branch `xaml-compiler-e2` (pull request #9, against `main`, linear, rebased
  on `main` after E1 merged). Corpus: 77 of 81 documents eligible, all 77 match the run-time
  loader. The four that are not eligible need E3/E4 (a binding: parent stack; a style selector;
  a control template: deferred content; resources). Theme documents: 0 of 163 eligible (each theme
  compiled as one group, without its `x:Class` document and the documents its `excluded.txt`
  leaves out): 146 are blocked first by the parent stack (E3), 14 by `Content` setters, 2 by
  `EnsureCapacityNode`, 1 by `x:Static` of a static property; measured by the ignored test
  `emitter::repository_documents::measure_theme_documents`.
- Everything E2 adds exists only with the `compiler-metadata` feature of the base crate (section
  7, "Cost"); a default build carries none of it.
- Stage E3 is next (section 7). Nothing is half-done in the working tree.

Design documents in the repository: `docs/porting/xaml.md`, sections 9 (emitter design: call forms
A/B/C, build integration, code-behind), 9.5 (source scanner), 9.12 (the 14 rulings), 9.13, and
decisions 1 to 27. Read section 9 before touching the emitter. `DRAFT-AUTHOR-REPORT.md` is the
report of the author of the uncompiled draft (history: everything it lists as unverified was
verified when E1 built it, see section 4).

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

## 3. What stage E1 delivered

Run-time library (`ferroui-markup-xaml`):

- `xaml_il::runtime::compiled` (new, the `rt` of generated code): `at` (a load error at a position,
  with exactly the message the run-time loader gives: `<message> Line L, position P. (line L
  position P)`), `name_scope_of` (the name scope field of a context), `register_name`,
  `complete_root_name_scope`, `untyped_object_form` (moved here from the loader's
  `runtime/type_system/values.rs`, which re-exports it: one implementation for both back ends) and
  `to_object` (a value as the argument of a member typed `object`, as `to_exact` converts it).

Base (`ferroui-base`):

- `metadata::property_accessors` (new, feature `compiler-metadata` only):
  `record_property_accessor`, `declared_property_accessors`, `property_accessors`. The
  `ferro_property!(for Owner; ..)` form (so every accessor of a `ferro_properties!` block) records
  the owner type, the accessor name and the definition when the type registers its properties
  (its static initialisation). `property_accessors(property, preferred)` initialises the types it
  consults (the resolving type and its bases, then the registering type and its bases) and lists
  their accessors in declaration order, so the choice is independent of execution order. Without
  the feature the macro records nothing and the table does not exist. The loader's `emitter`
  feature (which gates `rust_emitter`) enables it; the XAML test crate turns `emitter` on.
  The emitter names a registered property by a recorded accessor, never by a naming rule.
  Accessors declared with the plain `ferro_property!(..)` form outside a block are not recorded;
  such a property makes a document not eligible (measure and fix in E2 if any is reachable).

Loader (`ferroui-markup-xaml-loader`, `rust_emitter/`):

- `emitter.rs`: values (new object with a default constructor, `XamlValueNodeWithBeginInit`,
  compiler locals: `XamlAstLocalInitializationNodeEmitter` / `XamlAstCompilerLocalNode`, text,
  every numeric / boolean / character / enumeration constant as `constant_value` reads it, NaN and
  infinities, `{x:Null}`, `{x:Static}` of an enumeration member, vector-like and grid length
  constants), manipulations (object initialisation, groups, `XamlAstManipulationImperativeNode`
  over `XamlAstImperativeValueManipulation`, property assignments of styled, attached and direct
  properties through the accessors the type system projects for them, name registration, the scope
  of the root object). The value conversions are the ones `to_exact` applies: exact type,
  `Some(..)` into the nullable form, `rt::to_object(..)` into `object`, class upcasts. Every
  object creation and assignment carries a position marker comment. Paths are absolute (`::crate`).
- `compiled.rs`: the generated file has `use ::ferroui_markup_xaml::xaml_il::runtime::compiled as rt;`;
  `transformed_tree` (feature `testing`) prints the AST the emitter walks.
- `runtime/interpreter`: `numeric_constant` is shared with the emitter (`pub(crate)`).

Tests (`tests/FerroUI.Markup.Xaml.UnitTests/emitter`): the corpus has 63 documents; the dump
covers set registered properties, every direct property, the initialised state, the name scope of
the root (named elements found, completion) and logical children; a failed load compares by
message; `compiled_documents_are_registered_by_uri` loads through `FerroXamlLoader` with a
`StandardAssetLoader` bound in a locator scope; `print_transformed_tree` is an ignored diagnostic
(`FERROUI_EMITTER_DOCUMENT=<name>`). The test crate enables the loader's `testing` feature.

Not eligible in E1 and why (the reasons the harness prints): declared accessors of attached
properties (`Canvas.Left`, `TextBlock.TextAlignment` set on a text block, `BaselineOffset`), plain
properties and collection adds (`Children`, `Styles`, `Resources`), types without a recorded public
Rust path (`FontStyle`, `Viewbox`, `LayoutTransformControl`: the hand lists), bindings (parent
stack), templates (`ControlTemplate` is not a class of the object model), constructor arguments
(`Background='Red'` builds a brush with arguments).

Review fixes (after the E1 review):

- Determinism of accessor selection and the `compiler-metadata` feature (above).
- `rt::at(type_name, message, line, position)`: load errors of generated code carry the exception
  type the run-time loader reports (`CompiledLoadError` as the inner error); the harness compares
  type and message.
- `CompiledXamlLoader` returns `Result<Option<BoxedValue>, XamlLoadException>` (ruling 7);
  `FerroXamlLoader` propagates the error; the generated `try_load` returns the build error
  instead of panicking; the hand-written loaders of both themes and of the catalog follow.
- Prelude names in generated code are fully qualified (`::core::option::Option::Some`, ...).
- Colliding build function names (`a-b.xaml` / `a_b.xaml`, case, `x.xaml_untyped`) are a
  not-eligible diagnostic of the later document.
- Corpus: multi-line documents (error on line 4) and a control whose `EndInit` fails
  (`support::emitter::FailingEndInit`, registered with its Rust path; the test crate has
  `extern crate self as ferroui_markup_xaml_tests` for that).

Open items left for E2 and later (from the E1 review): the `.map.json` position map of 9.3.6
(only marker comments exist); value priority in the canonical dump (it shows set values, not the
priority they were set at); case folding of document URIs in the generated `try_load` is ASCII
only (`eq_ignore_ascii_case`), check it against upstream's URI comparison; group transforms (the
emitter compiles single documents with `transform_document`, the run-time loader transforms
groups with includes, section 6).

## 4. Verified when E1 built the draft

Every signature the draft's author listed as unverified compiled as written, except that the
draft's handling of errors and the conversions to `object` did not match the interpreter (fixed
in E1 by `rt::at` and `rt::to_object`). The draft's predicted `generated.rs` compiled too; it was
replaced by the real output.

## 5. Why the draft stopped at plain members and collection adds (history; E2 did it as section 7 describes)

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

## 7. What stage E2 delivered, and the next steps

E2 (branch `xaml-compiler-e2`):

- Public Rust paths, mechanically: `scripts/rust_paths.py` (called by
  `generate_markup_types.py`, target `rust-paths`) resolves the module tree of each framework crate
  and writes `rust_paths.rs` per crate: classes, non-generic markup types, contracts, and the
  registered instantiations of generic types (rendered with absolute paths). `ferro_rust_paths!`
  registers `CLASS_RUST_PATHS`, `MARKUP_RUST_PATHS` (with a trait flag) and `TYPE_RUST_PATHS` (by
  `TypeId` of the type itself). The hand lists of the draft are gone. The XAML test crate's
  `emitter/rust_paths_check.rs` (checked in, drift-tested) names every recorded path and every
  public property accessor from outside the crates: 346 classes, 511 markup types, 1136 accessors.
- Typed functions of declared members: the declaration macros generate, next to the declaration, an
  inherent `impl` of the declared type with `__markup_get_<Name>`, `__markup_set_<Name>`,
  `__markup_static_get_<Name>`, `__markup_static_set_<Name>`, `__markup_<Method>_<n>`,
  `__markup_new_<n>` and `__markup_parse` (`MarkupEmit` in the metadata; a pool of indices keeps
  overloads apart). `MarkupType::this` and `MarkupType::value` record the instance type and the
  value type (what constructors and `Parse` return: `this:` if stated, else the first handle).
  The run-time type system tags projected methods (`DeclaredMember`) and constructors with their
  declarations.
- Property accessors are recorded with the type whose `impl` declares them (`Self`) and their
  visibility (some framework accessors live in another type's `impl`, e.g. `ThemeVariant` holds
  two of `StyledElement`'s).
- Emitter: declared setters and static attached accessors, collection adds (`AdderSetter` with a
  declared or registered getter), method-call nodes, declared constructors with arguments, list
  constants, `rt::cast` (the loader's cast where `ValueTypes::is_assignable` proves it),
  `rt::argument` (a nullable instance converted as the loader converts it). The untyped-value
  normalisation (`to_untyped`, `normalize_object`, `box_object`) moved to the runtime library.
- The carriers `FerroListOf<T>` and `AddChildOf<T>` of the controls crate are public (hidden) so
  generated code can call their functions.
- Review fixes of pull request #9 (orchestrator review):
  - A list constant (`RowDefinitions='Auto,*'`) is held in a local: capacity, every `Add` and the
    setter receive the same list (before, every use created a new one). `Emitter::bind` holds any
    created value that a manipulation or several uses need. Test
    `a_list_built_from_text_is_one_list`.
  - The differential dump shows the priority of every set registered property, every plain
    property read through its declared getter, and the items of every collection markup adds to
    (`collection_items`, a table of the collection types; a collection it cannot enumerate is a
    difference, never a match).
  - Static conversions: a receiver is borrowed through the declared base chain (deref coercion)
    or as a handle of the declaring class (`Ref::upcast_ref`, new, the one new `unsafe`, in the
    class model); a value whose declaration lists a contract becomes the contract's handle by an
    unsizing coercion. `rt::cast` remains for registered casts only and returns the loader's
    `InvalidCastException`. Locals of value types are cloned where they are passed by value.
  - Cost: metadata fields only the emitter reads (`MarkupEmit` names, `MarkupType::this` /
    `value`) have the type `CompilerMetadata<T>` (the value with `compiler-metadata`, the
    zero-sized `NotRecorded<T>` without it); `__ferro_compiler_metadata!` expands the typed
    `__markup_*` functions and those values, or nothing; `ferro_rust_paths!` defines
    `register_rust_paths()`, empty without the feature, and the path registries exist only with
    it. Measured: a stripped release binary that registers every type of base, controls and
    markup-xaml is 39,521,328 bytes and holds no `__markup_*` name and no recorded path by
    default, 39,846,672 bytes (131 function names, 118 controls paths) with the feature. Generated
    code calls the typed functions, so a crate that compiles generated code enables the feature
    (E5 decides how applications get it).
- Open items of E1 done in E2: the position map of 9.3.6 (`GeneratedFile::position_map`, checked
  in as `emitter/generated.map.json` and drift-tested); value priority in the dump; URIs compared
  as upstream's `OrdinalIgnoreCase` (`rt::uri_equals`); group transforms (`compile_documents`
  transforms the documents of an assembly as one group with their URIs as base URIs,
  `FerroXamlIlRuntimeCompiler::transform_documents`).
- Direct properties: assigned through `set_direct_value` (the registered direct setter, the path
  of the interpreter's `set_value_untyped`); the corpus sets `Name`, `SelectedIndex` and
  `SelectedItem`.

Next, E3 (stack it on `xaml-compiler-e2`):

1. The compiled context (ruling 7, design R1): move `RuntimeContext` (`runtime/interpreter/
   runtime_context.rs`, with the name scope field and the eager parent stack provider of
   `runtime/framework/services.rs`) into `ferroui-markup-xaml` as `xaml_il::runtime::XamlIlContext`,
   re-seat the interpreter on it, then emit `ctx.push_parent` / `pop_parent` exactly where the
   interpreter pushes (`needs_parent_stack` of the node, `ParentStackVisitor`). That alone makes
   most theme documents get past the first blocker.
2. Markup extensions (9.3.4): the provide-value target property set and cleared on the context,
   `provide_value` called through the declared member, the dynamic setter chain of
   `property_assignment` unrolled over the setters `assignment_plan` keeps (design 9.3.3, last row).
3. Static and dynamic resources, `x:Static` of properties and fields (typed functions exist for
   static properties; fields need one: add `__markup_field_<Name>` to the macros).
4. Compiled bindings: binding path nodes as typed builder calls (`runtime/framework/binding_path.rs`
   is the interpreter; the typed hook `MarkupProperty::typed_path_element` shows the typed form).
5. Measure the start-up effect on `themed_window` from E3 on (the task describes how on Linux).

Members still without a typed function (form C would be needed, or a macro extension): events,
fields, indexers, and declarations of types that are not local to the declaring crate (none
exist today: every declaration compiled with the generated `impl`).
