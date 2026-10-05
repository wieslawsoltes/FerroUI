Stage E1 is written in the redirected form (real typed Rust, no plan/executor), but narrower than both redirects asked for, and nothing has been compiled or run.

## Deviations you should know first
- **Plain members and collection adds are not done** (your second message). I stopped that part under your escape clause; reasons and counts are below. E1 covers registered styled/attached properties only.
- **`generated.rs` is a prediction.** I produced it with a throwaway script that mimics the emitter's format, for the 32 documents I expect to be eligible. It may differ from real output, and it may not compile.
- **Controls' type table does not hold public paths.** 104 of 250 entries in `src/FerroUI.Controls/register_types.rs` are declaring-module paths behind private modules (`crate::border::Border`). I record only entries that are public as written, plus a hand list of 12 root re-exports for the corpus classes.
- **The generator cannot supply enum/value-type paths.** It writes glob imports (`use crate::layout::*`), so it has no per-type path. I used a compiler-checked hand list in each crate's `register_types.rs` instead; there is no `rust_paths.rs`.
- **`emit_document` takes the transformer configuration, not the type system**: `emit_document(root, configuration, function_name, document_name)`.
- **Fallback if the checked-in file fails to compile**: empty the functions and the `DOCUMENTS` table in `generated.rs` by hand, then run the regenerate command.

## Commands to run
- Regenerate: `cargo test -p ferroui-markup-xaml-tests --lib emitter::differential_tests::regenerate_emitter_output -- --ignored --exact`
- No `generate_markup_types.py` run is needed; generated markup files are untouched. I edited the script only so it still finds the type list (`types![TYPES` instead of `const TYPES`).

## Files written
| Path | Purpose | Lines |
|---|---|---|
| `src/Markup/FerroUI.Markup.Xaml.Loader/rust_emitter/mod.rs` | module doc, re-exports | ~65 |
| `.../rust_emitter/emitter.rs` | `emit_document`, `UnsupportedNode`, AST walk | ~560 |
| `.../rust_emitter/source.rs` | `rust_string_literal`, `function_name_of`, unit tests | ~80 |
| `.../rust_emitter/compiled.rs` | `compile_documents`, `generate_file` (functions, `DOCUMENTS`, `try_load`, `register_compiled_xaml`) | ~150 |
| `tests/FerroUI.Markup.Xaml.UnitTests/emitter/mod.rs` | why checked-in, regeneration command | ~35 |
| `.../emitter/corpus.rs` | 40 documents, `EXPECTED_ELIGIBLE` (22), `EXPECTED_NOT_ELIGIBLE` (7) | ~80 |
| `.../emitter/generated.rs` | predicted emitter output | ~920 |
| `.../emitter/differential_tests.rs` | up-to-date check, ignored regenerate, eligibility, dump comparison, URI registration | ~220 |

Also `pub mod rust_emitter;` in the loader's `lib.rs` (feature `runtime`) and `pub mod emitter;` in the test crate's `lib.rs`.

## Metadata additions
- `src/FerroUI.Base/metadata/markup_type.rs`
  - `MarkupType.rust_path: Option<&'static str>` (default `None`)
  - `MarkupType::register_rust_paths(&[(TypeOf, &'static str)])`
  - `MarkupType::rust_path_of_handle(TypeId) -> Option<&'static str>`
  - `MarkupType::rust_path(&self) -> Option<&'static str>`
  - `MarkupConstructor.emit: Option<&'static str>`
  - `MarkupEnumMember.rust_variant: Option<&'static str>`
- `src/FerroUI.Base/metadata/markup_macros.rs`
  - constructors record `stringify!($new)`
  - plain enum members record the variant identifier (new helper `__ferro_markup_enum_variant_name!`); flags members record `None`
- `src/FerroUI.Base/type_system.rs`
  - `TypeInfo::register_rust_paths(&[(&'static TypeInfo, &'static str)])`
  - `TypeInfo::rust_path(&'static self) -> Option<&'static str>`
- `src/FerroUI.Base/register_types.rs`
  - `types![TYPES, RUST_PATHS; crate::a::B, ..]` now yields both tables
  - `value_paths!` / `VALUE_RUST_PATHS` for `CornerRadius`, `Thickness`, `HorizontalAlignment`, `Orientation`, `VerticalAlignment`, `TextAlignment`, `TextWrapping`
- `src/FerroUI.Controls/register_types.rs`
  - same `types!` change
  - `PUBLIC_MODULES` + `is_public_path` filter
  - `ROOT_RUST_PATHS` for `Border`, `Button`, `Canvas`, `ContentControl`, `Control`, `Decorator`, `DockPanel`, `Grid`, `Panel`, `StackPanel`, `TextBlock`, `UserControl`
  - `VALUE_RUST_PATHS` for `Dock`, `GridLength`, `GridUnitType`
- `src/Markup/FerroUI.Markup.Xaml/register_types.rs` is unchanged; it only lists `XamlSourceInfo`.

## Accessors added to existing files
- `runtime/interpreter/evaluators.rs`: `pub(crate) fn single_setter(node: &Rc<dyn IXamlAstNode>, assignment: &XamlPropertyAssignmentNode) -> XamlResult<Option<Rc<dyn IXamlPropertySetter>>>`, re-exported in `runtime/interpreter/mod.rs`.
- `runtime/framework/methods.rs`: `RuntimeDocumentTypeBuilderProvider::transformed_root(&self) -> Option<(Rc<dyn IXamlAstNode>, Rc<TransformerConfiguration>)>` (`pub(crate)`).
- `ferro_xaml_il_runtime_compiler.rs`: `pub(crate) fn transform_document(xaml, name, configuration) -> XamlResult<(root, Rc<TransformerConfiguration>, Rc<RuntimeTypeSystem>)>`.

## Node coverage
**Supported:**
- root value-with-manipulation whose value is `XamlAstNewClrObjectNode` (object-model class, default constructor, public path) → `Path::new()`
- `XamlValueWithManipulationNode`, `XamlManipulationGroupNode`
- `XamlObjectInitializationNode` → `begin_init()` / `try_end_init()?` (styled elements only)
- `XamlPropertyAssignmentNode` with one setter that is a `XamlDirectCallPropertySetter` over the type-system-built accessor of a styled or attached property → `target.set_value(Owner::name_property(), typed)`
- `XamlAstTextNode`; `XamlConstantNode` (String, Double, Boolean, Int32, Int64, plain enum by value); `XamlNullExtensionNode`; `XamlStaticExtensionNode` for a plain enum member
- `FerroXamlIlVectorLikeConstantAstNode` and `FerroXamlIlGridLengthAstNode`, through the constructor's recorded text when it has the form `TypeName::function`
- `HandleRootObjectScopeNode`; `FerroNameScopeRegistrationXamlIlNode` with a text name

The value's Rust type is matched against the property's `TypeId` (`T`, `Option<T>`, `Option<BoxedValue>`, class handles with `upcast`); no match means not eligible.

**Not eligible:**
- direct properties, which includes `Name`, so named elements are out even though registration is emitted
- plain properties, adders/collections, dynamic setters
- markup extensions, bindings, templates/deferred content, styles/selectors, resources, events
- `x:Type`, static properties/fields other than enum members, flags enums
- value-type element construction, constructor arguments, classes with a registered class document
- any document where a node needs the parent stack

## Why plain members and adds were stopped
- Accessors are captured as `$get:expr` / `$set:expr`, which are opaque once captured. Telling a path from a closure means rewriting the accessor matchers to token munching across about 15 internal macros, blind.
- Text from `stringify!` is relative to the declaring module's imports, so only a `Type::function` path can be made absolute mechanically.
- `Panel.Children` adds go through the synthetic `FerroList<T>` projection in the loader's type system, not through any declaration, so macros and the generator cannot supply them.

Approximate grep counts of declaration forms (path form could carry emit text, closure form could not):

| Crate | Accessors, path | Accessors, closure | Methods, path | Methods, closure |
|---|---|---|---|---|
| base | 159 | 566 | 15 | 21 |
| controls | 85 | 117 | 0 | 7 |
| markup-xaml | 51 | 30 | 2 | 9 |

Corpus eligibility: 32 of 40 predicted eligible without the addition. With it, `panel_children` and probably `named_element` / `name_property` would be added — an estimate, not a measurement.

## C003
- `src/FerroUI.Base/markup_types/contracts.rs`: contract metadata for `dyn IBitmap` (`FerroUI.Media.Imaging.IBitmap`, handles `Rc<dyn IBitmap>` / `Option<..>`), added to the file's type list, plus `ValueTypes::register_nullable::<Rc<dyn IBitmap>>()`.
- `compiler_extensions/ferro_xaml_il_language.rs`: maps the contract to `BitmapTypeConverter` only when `type_system.find_type("FerroUI.Media.Imaging.IBitmap")` finds it, so the reduced test type system is unaffected.
- The `Rc<Bitmap>` → `Rc<dyn IBitmap>` cast already existed in `well_known.rs`.

## Signatures I could not verify — check these first
- `types!` pattern `$(crate :: $($segment:ident)::+),*` matching every list entry (all current entries have that shape).
- Closure-to-fn-pointer coercion inside the const tuple arrays of `value_paths!`.
- `XamlAstNewClrObjectNode.constructor.as_any()`; `IXamlPropertySetter::as_any()`; `visit_node` accepting `&mut NeedsParentStack` and visiting the whole tree.
- `IXamlType::name()` / `namespace()` / `full_name()` as used in `emitter.rs`.
- `ValueTypes::register_nullable::<Rc<dyn IBitmap>>()` bounds (`PropertyValue`).
- In generated code:
  - `Ref<T>::begin_init()` / `try_end_init()` reached by deref to `StyledElement`
  - `&Ref<Border>` coercing to `&StyledElement` for `NameScope::set_name_scope`
  - `INameScope::complete(&**scope)`
  - `ServiceProviderExtensions::get_name_scope(&**provider)`
  - `std::rc::Rc::new(ref_or_value) as ferroui_base::BoxedValue`
  - `Type::new()` existing for every corpus class
  - attached definitions (`Grid::row_property()`) passing to `set_value` by deref coercion (existing code does this)
  - `ferroui_base::input::InputElement`, `ferroui_base::layout::Layoutable` and `ferroui_base::Visual` being nameable from another crate
- In the tests:
  - `FerroPropertyRegistry::get_registered` / `get_registered_attached` returning `Rc<Vec<&'static FerroProperty>>`
  - `FerroList::to_vec()`
  - `RuntimeXamlLoaderConfiguration::new()` matching the defaults `FerroRuntimeXamlLoader::load` uses
  - whether `xaml_test_base()` has an `IAssetLoader`; if not, the URI test checks `generated::try_load` only and prints that `FerroXamlLoader::load` was skipped

## Known limitations
- `CompiledXamlLoader` still returns `Option<BoxedValue>` in the tree, so the generated `try_load` panics with the load error's message on a failed build.
- Whether the "?" corpus documents (object-typed `Content` / `Tag`, attached properties, `x:Null`, `x:Static`, `FontSize`) are really eligible depends on transformer output I could only read, not observe.
- The dump covers set registered properties and logical children only.

## Left for the source-scanner stage
- Complete public-path tables: read re-exports, replacing `PUBLIC_MODULES`, `ROOT_RUST_PATHS` and both `VALUE_RUST_PATHS`.
- Emit text for plain members and collection adds (macro matcher rewrite, plus the list projection).
- Direct properties and names; a compiled context type for the parent stack and markup extensions.
- Transform without the framework linked; the real build-script integration.