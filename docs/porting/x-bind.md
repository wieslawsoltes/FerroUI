# x:Bind: one compiled-binding core for FerroUI XAML and WinUI XAML (#525)

The plan for full `{x:Bind}` support in 0.5: one markup extension in the XAML runtime library,
compiled by the one XAML compiler for both dialects. Native FerroUI XAML (the port of the markup of
the upstream framework) writes it directly; the WinUI importer maps WinUI's `x:Bind` onto it, and
the property-path adapter of `ferroui-markup-xaml-winui` goes away. This page is the design and the
order of the work; each step below is one pull request from main, built and tested on its own.

The rules that bind it: the Rust emitter generates the binding code (typed accessors, no lookup by
name on the hot path), the run-time loader runs the same core interpreted (documents loaded at run
time, hot reload), and tests compare the two on the same documents. The diagnostics are compile
errors of the emitter and load errors of the run-time loader, with the source position, and come
from the same transformer. The behaviour is WinUI's, from the XAML compiler of the pinned
`microsoft-ui-xaml` checkout, revision `7e4b0c2096be402f51cc8f7d203ebf67cff183d0` (paths below are
relative to `src/XamlCompiler` of that checkout unless they start with another directory).

## 1. What there is

### 1.1 The WinUI side (`ferroui-markup-xaml-winui`)

- **The adapter** (`src/Markup/FerroUI.Markup.Xaml.WinUI/bindings/`): `WinUIBindExtension`
  (`binding.rs`; `Path` as a string, `Mode`, `Scope`, `DataType`, `UpdateSourceTrigger`,
  `SourceLine`, `SourceColumn`), `WinUIBinding` (the binding of a target), `WinUIBindings`
  (`bindings.rs`; `Initialize`, `Update`, `StopTracking`, opt-in stop on unload) and the nodes
  `RootGate` and `PathNode` (`path_node.rs`). A path is split at dots and every segment is looked
  up by name at run time through the registered property, model and markup metadata. OneTime
  reads without subscribing. The lifecycle is explicit: the host initializes a root after
  construction, and a template instance only after its data context is set.
- **The importer** (`importer/migration.rs`, `MigrationContext::x_bind` and `parse_x_bind`): with
  `WinUIXBindPolicy::PropertyPathsWithExplicitLifecycle` it rewrites `{x:Bind}` to
  `{fui:WinUIBind Path=..., Mode=..., Scope=Root|Template, SourceLine=..., SourceColumn=...,
  DataType={x:Type ...}}` and reports `WXAML2004` (the host must drive the lifecycle). Every other
  form is `WXAML1007`: functions, casts, indexers, operators, converters, `FallbackValue`,
  `TargetNullValue`, `BindBack`, nested x:Bind, an event target, x:Bind in a `ControlTemplate` or
  an `ItemsPanelTemplate`, a template without `x:DataType`. `x:Phase`, `x:DefaultBindMode` and
  `x:Load` are in the list of directives the importer does not migrate (`migration.rs`, the
  `"Bind" | "Load" | "DeferLoadStrategy" | "Phase" | "DefaultBindMode" | "Uid"` arm).
- **The tests**: `bindings_tests.rs` (the lifecycle), `importer_tests.rs` (the migration and its
  diagnostics) and `compiled_tests.rs`, where the migrated `fixtures/winui_root.xaml` and
  `fixtures/winui_template.xaml` run through the interpreter and through the emitter, whose output
  `fixtures/winui_bindings.generated.rs` is checked in and compared with the current compiler.
- **The documentation**: [winui-controls.md](winui-controls.md), section 10.6, lists x:Bind as
  "Missing" except property paths.

### 1.2 Compiled bindings of FerroUI XAML

- **The extension** (`src/Markup/FerroUI.Markup.Xaml/markup_extensions/compiled_binding_extension.rs`):
  `{CompiledBinding}` is a `CompiledBinding` with a `CompiledBindingPath` and a `DataType`; its
  `ProvideValue` creates the binding with the default anchor of the place it is declared in.
  `compiled_bindings/property_info_accessor_factory.rs` makes the accessors of plain properties.
- **The engine** (`src/FerroUI.Base/data/`): `CompiledBindingPath` and `CompiledBindingPathBuilder`
  (`compiled_binding_path.rs`: registered properties, typed plain properties, untyped members,
  method commands, streams, `$self`, ancestors, named elements, the templated parent, array
  elements, casts; `build_expression` makes the expression nodes), `CompiledBinding`
  (`compiled_binding.rs`, with the typed fast path of a single typed property),
  `core/binding_expression.rs` (`BindingExpression`, the modes, `OneTime` reset by a data context
  node, `LostFocus` and delayed source updates), `core/expression_nodes/` and `core/plugins/`
  (registered properties, `INotifyPropertyChanged`, the model metadata).
- **The compiler** (`src/Markup/FerroUI.Markup.Xaml.Loader/compiler_extensions/`):
  `FerroBindingExtensionTransformer` (`{Binding}` to `CompiledBinding` or `ReflectionBinding`,
  `x:CompileBindings`), `XDataTypeTransformer` (`x:DataType` to the `DataType` property of a
  template), `FerroXamlIlBindingPathParser` (the path text into `ParsedBindingPathNode`, with the
  grammar of the engine, `BindingExpressionGrammar`), `FerroXamlIlDataContextTypeTransformer`
  (the data context type of each scope), `FerroXamlIlBindingPathTransformer` and
  `XamlIlBindingPathHelper::update_compiled_binding_extension` (the path resolved against its start
  type into `XamlIlBindingPathNode`, element by element).
- **The two back ends of the compiler**: the Rust emitter writes a path as builder calls with typed
  accessors (`rust_emitter/emitter.rs`, `binding_path` and `path_element`: `builder.ferro_property`,
  `rt::path_property` with the typed getter and setter of a plain property, `builder.element_name`
  with the name scope of the document, `rt::path_indexer`, `rt::path_command`); the interpreter makes
  the same calls with the invokers of the run-time type system (`runtime/framework/binding_path.rs`).
  The emitter's corpus (`tests/FerroUI.Markup.Xaml.UnitTests/emitter/`) compares the trees the two
  build.
- **Phasing**: the ItemsRepeater port has WinUI's phasing contract,
  `IDataTemplateComponent` (`ProcessBindings(item, index, phase)`, `Recycle`) and `Phaser`
  (`src/FerroUI.Controls.Repeater/phaser.rs`, `view_manager.rs`: phase zero before the data
  context, as WinUI does for an x:Bind component).

### 1.3 Findings

1. The compiled-binding pipeline is already shared by the two back ends, and it already builds
   typed accessor code. What x:Bind needs beyond it is a different root source (the root object
   of the document, or the item of a typed template, instead of the data context), the default
   mode `OneTime`, a lifecycle (`Initialize`, `Update`, `StopTracking`), the grammar forms of
   WinUI that the engine's grammar does not have (functions, string indexers, `BindBack`), event
   binding and phasing. One extension in the runtime library, with its own path resolution in the
   existing transformers, gives both dialects the same core.
2. The adapter's run-time lookup by name (`PathNode`) is what the compiled path replaces: a path
   that does not resolve is a compile error instead of a run-time diagnostic.
3. A compiled path holds the name scope of a named element weakly (DEVIATIONS.md, Bindings). A
   `DataTemplate` returns only its root (`DataTemplate::build`), so nothing kept the template's
   name scope, and a path that starts at a named element of a template could not find it. The
   x:Bind core keeps it (section 3.5).
4. The engine's grammar parses forms that are not x:Bind syntax (`$self`, `$parent`, `#name`,
   `!`, `^`); the x:Bind path rejects them.

## 2. The WinUI semantics

| Feature | WinUI | Reference | Step |
|---|---|---|---|
| Root source | The page or control class (`x:Class`): its fields, properties and the fields generated for `x:Name`. Without code-behind x:Bind is an error (`XamlCompiler_XBindWithoutCodeBehind`, WMC1119). | `BuildTasks/Microsoft/Xaml/XamlCompiler/XamlHarvester.cs`, `BindUniverse.cs` | X1 |
| Typed templates | Inside a `DataTemplate` the root is the item, of the template's `x:DataType`; a template without it is an error (`Harvester_DataTemplateDoesNotDefineDataType`, WMC1111). | `XamlHarvester.cs` (the scope root of a `DataTemplate`, the check after it) | X1 |
| `ControlTemplate` | The root is the templated control, of the template's `TargetType` (`Harvester_ControlTemplateDoesNotDefineTargetType`). | `XamlHarvester.cs` (the scope root of a `ControlTemplate`) | X5 |
| Styles | x:Bind in a style is an error (`BindAssignment_XamlXBindUsedInStyleError`, WMC1112). | `CompileError.cs`, `XamlXBindUsedInStyleError` | X1 |
| Modes | `OneTime` (the default), `OneWay`, `TwoWay`; `OneWay` and `TwoWay` follow `INotifyPropertyChanged` and dependency properties. | `DefaultBindMode.cs`, `BindAssignment.cs` | X1 |
| `x:DefaultBindMode` | Changes the default mode of the element and its subtree. | `DefaultBindMode.cs`, `XamlHarvester.cs` (the members stripped from the tree) | X5 |
| `UpdateSourceTrigger` | `Default` (on `LostFocus` for `TextBox.Text`, else on change), `PropertyChanged`, `LostFocus`; `Explicit` is an error; a trigger needs `TwoWay`. | `BindAssignment.cs`, `NeedsLostFocusForTwoWay`, `ValidateUpdateSourceTrigger`; `Properties/XamlCompilerResources.resx`, `BindAssignment_UpdateSourceTrigger_*` | X1 |
| TwoWay sources | The path must end in a writable member (`BindAssignment_TwoWaySourceNotWritable`); not after a cast (`BindPathParser_CantTwoWayCastStep`). | `BindAssignment.cs`, `BindPathStep` | X1, X2 |
| Paths | `a.b`, static `ns:Type.Member`, indexers `[0]` and `['key']`, attached `path.(ns:Owner.Prop)` (a path that starts with an attached property is an error), casts `(ns:Type)path` and `((ns:Type)path).b`. | `BuildTasks/BindingPath/BindingPath.g4` | X1 (members, attached, named elements), X2 (the rest) |
| Named elements | A first step that is an `x:Name` is the generated field of that element; in a template the data root wins (`BindAssignment_FieldNameElementName_Conflict`). | `XamlCompilerResources.resx`, `BindAssignment_FieldNameElementName_*` | X1 |
| Functions | `f(a, b)`, `path.f(...)`, `ns:Type.f(...)`; arguments are paths, booleans, numbers, strings or `x:Null`; a function re-evaluates when an argument path changes; the return type must match the target. | `BindingPath.g4`, `CodeGenerators/BindPathStepCodeGenerators.cs`, `BindPathParameterCodeGenerators.cs`, `BindAssignment_FunctionReturnTypeInvalid` | X2 |
| `BindBack` | `TwoWay` through a method of one argument. | `BindAssignment_BindBack_*` | X2 |
| Converters | `Converter`, `ConverterParameter`, `ConverterLanguage`; not with a function. | `BindAssignment_ConverterWithFunctionBindingNotSupported`, `BindAssignment_OrphanConverterParam` | X3 |
| `FallbackValue`, `TargetNullValue` | A string or `{x:Null}` (`BindAssignment_InvalidFallbackValue`); `TargetNullValue` on a non-nullable target is an error (WMC1120). | `CompileError.cs`, `XamlXBindTargetNullValueOnNonNullableTypeError` | X3 |
| Events | `Click="{x:Bind Model.OnClick}"`: a method with the event's signature, no parameters, or a base signature. | `BoundEventAssignment.cs` | X4 |
| `x:Phase` | Elements of an item template with a phase are hidden until their phase; their bindings are read in the phase, through `ContainerContentChanging` / `ProcessBindings`. | `PhaseAssignment.cs`, `CodeGenerators/CSharpPagePass2.tt` (`ProcessBindings`, `Recycle`) | X5 |
| `x:Load` | Deferred creation of an element, driven by a binding. | `BoundLoadAssignment.cs`, `DeferLoadStrategy.cs` | X5 |
| Lifecycle | The generated `Bindings`: `Initialize` (unless initialized, `Update`), `Update` (every binding, `OneTime` included), `StopTracking` (releases the listeners, uninitialized). The root calls `Initialize` on `Loading` (a `Window` on `Activated`); a template instance updates on `DataContextChanged` with a new item (`SetDataRoot` ignores null) until a container drives it through `ProcessBindings`. | `CodeGenerators/CSharpPagePass2.tt` (`Loading +=`, `DataContextChanged +=`, `DataContextChangedHandler`, `ProcessBindings`, `Initialize`, `Update`, `StopTracking`, `Loading`, `Activated`) | X1 (Window: X5) |

WinUI's own tests are golden files of its compiler: the projects under
`Tests/RegressionProjects/Features/CompiledBinding/BindTestbedCS` (`BasicTests.xaml`,
`TwoWayTests.xaml`, `FunctionTests.xaml`, `CastingTests.xaml`, `EventTests.xaml`,
`PhasingTests.xaml`, `ListAndTemplateTests.xaml`, `NullableTests.xaml`, ...) and their generated
code in `TestMasters/RegressionProjects/common/Features/CompiledBinding`. They show what the
generated code does for each form; the steps port the behaviours they exercise as tests of the
two back ends, step by step.

## 3. The core design

### 3.1 One markup extension

`XBindExtension` in `ferroui_markup_xaml::markup_extensions` (namespace
`FerroUI.Markup.Xaml.MarkupExtensions`, so `{XBind ...}` in the framework XML namespace):

- `Path: Option<CompiledBindingPath>` (the first constructor argument), `Mode` (default
  `OneTime`), `UpdateSourceTrigger`, and `DataType`, which the compiler sets for a binding inside a
  typed `DataTemplate`. Later steps add `Converter`, `ConverterParameter`, `ConverterLanguage`,
  `FallbackValue`, `TargetNullValue` (X3) and `BindBack` (X2).
- `ProvideValue` gives the target an `XBinding`, registered with the `XBindings` of its root: the
  root object of the document (`IRootObjectProvider::root_object`), or, with a `DataType`, the root
  of the template instance (`intermediate_root_object`).
- `x:Bind` (the XAML language namespace) is mapped onto it by the compiler, as `{Binding}` is
  mapped onto `CompiledBinding`. The WinUI importer writes `x:Bind` unchanged once X6 lands.

### 3.2 The compiler

The existing compiled-binding transformers build the x:Bind path; the parts that are x:Bind's
own are in `compiler_extensions/transformers/x_bind.rs`:

1. `FerroBindingExtensionTransformer` maps the type reference `x:Bind` to `XBind` of the framework
   namespace (`x_bind::map_type_reference`).
2. `FerroXamlIlBindingPathParser` parses the path of an `XBindExtension` as it parses the path of
   a `CompiledBindingExtension`. Before, `x_bind::prepare_scope` looks at the ancestors: in a
   `DataTemplate`, the template's `x:DataType` (its text, or the `{x:Type}`) becomes the `DataType`
   of the extension, a resolved property value added to the node; a template without it, a
   `ControlTemplate` and a style are errors.
3. `FerroXamlIlBindingPathTransformer` resolves the path with `x_bind::transform_path`: the start
   type is the `DataType`, or else the type of the root of the document
   (`AstTransformationContext::root_object`, the `x:Class` type for a class document). A first
   step that is not a member of the data type of a template, or any first step at the root of a
   document, that is an `x:Name` of the name scope becomes a named-element step (the `#name` of the
   engine's grammar). The resolution is `XamlIlBindingPathHelper::update_compiled_binding_extension`,
   the one of `{CompiledBinding}`. Then the checks of section 3.7.

A document without `x:Class` binds to the type of its root element. WinUI requires code-behind;
FerroUI documents are often classless, and the root element's properties are a natural source, so
this is a superset, as is a path that starts with an attached property of the root
(`{x:Bind (Grid.Column)}`, which WinUI's grammar rejects in favour of `name.(Grid.Column)`).

### 3.3 The compiled form

The emitter writes an x:Bind as it writes a `{CompiledBinding}`: the path as builder calls with
typed accessors, the extension, then `ProvideValue` and the binding of the target. For
`<TextBlock Text="{x:Bind editor.Text, Mode=OneWay}"/>` at the root of a document:

```rust
let path_0 = {
    let builder = ::ferroui_base::data::CompiledBindingPathBuilder::new();
    let builder = builder.element_name(rt::path_name_scope(name_scope.as_ref(), 8, 30)?, "editor");
    let builder = builder.ferro_property(rt::property(::ferroui_controls::TextBox::text_property()));
    builder.build()
};
let x_bind_extension_0 = ::ferroui_markup_xaml::markup_extensions::XBindExtension::__markup_new_1(path_0.clone());
::ferroui_markup_xaml::markup_extensions::XBindExtension::__markup_set_Mode(&x_bind_extension_0, ::ferroui_base::data::BindingMode::OneWay);
let provided_0 = rt::provide_value_invoked(&context, rt::property_value(::ferroui_controls::TextBlock::text_property()), &x_bind_extension_0, ::ferroui_markup_xaml::markup_extensions::XBindExtension::__markup_ProvideValue_0, 8, 30)?;
rt::bind(text_block_0.upcast_ref::<::ferroui_base::FerroObject>(), ::ferroui_controls::TextBlock::text_property(), provided_0, 8, 30)?;
```

A plain property of a class is `rt::path_property(&builder, ...)` with its typed getter and
setter, as for `{CompiledBinding}`. Nothing is looked up by name when the binding runs. Functions
(X2) become a path element with a generated closure over the typed arguments; events (X4) a
generated handler that calls the typed method.

### 3.4 The run-time loader

The run-time loader runs the same transformer pipeline and interprets the resolved path with the
invokers of the run-time type system (`runtime/framework/binding_path.rs`), then invokes
`ProvideValue` through the markup metadata. Both back ends therefore make the same
`XBindExtension` with the same path, and only the accessors differ (typed code or invokers).

### 3.5 The runtime: binding, root source, lifecycle

- `XBinding::create_instance` builds the ordinary `BindingExpression` of the engine: a root node
  (`x_bind/root_node.rs`) followed by the nodes of the compiled path
  (`CompiledBindingPath::build_expression`, public for this). The root node publishes the root
  source only while the bindings of the root are initialized. It is a data context node of the
  engine, so a `OneTime` expression reads its leaf again whenever the root source is published:
  `Update` reads `OneTime` bindings again, and nothing else does.
- `XBindings` (`x_bind/bindings.rs`), one per root, owned by the root through an attached
  property (`XBindings.Bindings`), holding the root and its bindings weakly: `initialize`,
  `update`, `stop_tracking`, `is_initialized`, `diagnostics`, and `XBindings::get_for_root`.
  It is the per-root `Bindings` object WinUI generates; it is created by the first x:Bind of the
  root, in both back ends.
- **Loading.** A document root is initialized when it is attached to a visual tree
  (`Visual::attached_to_visual_tree`), the counterpart of `Loading`: after construction, and after
  code-behind has set what the bindings read. Each attach initializes (a no-op when initialized);
  after `StopTracking` the next attach reads again, as WinUI's `Loading` does. A root that is not a
  visual, or that is already in a tree when its document is built (a populated live root, hot
  reload), is initialized by calling `initialize`.
- **Templates.** A template instance is updated when its data context changes to an item
  (WinUI's `DataContextChangedHandler`): a recycled container reads its new item, `OneTime`
  included. An item that is not of the `x:DataType` is a diagnostic. The instance keeps the
  template's name scope (`NameScope::set_name_scope` on its root, as a document root keeps its
  own), so a path that starts at a named element of the template finds it.
- `TwoWay` uses the engine's write-back; the default trigger is `LostFocus` for `TextBox.Text` and
  `PropertyChanged` otherwise.

### 3.6 The WinUI importer onto the core (X6)

The importer stops rewriting x:Bind: WinUI's `{x:Bind ...}` stays as it is (its namespace and
grammar are those of the core), and the importer only validates what WinUI itself rejects. The
adapter (`WinUIBindExtension`, `WinUIBinding`, `WinUIBindingScope`, `WinUIBindings`) is removed,
with `WinUIXBindPolicy` reduced to `Reject` and `Compile` (or removed, see section 6), and the
diagnostics `WXAML1007` and `WXAML2004` are dropped except for forms WinUI rejects. The compiled
fixture of the WinUI crate is regenerated from the core.

### 3.7 Diagnostics

All are `XamlBindingsTransformException` (code `FRN2100`) with the position of the binding; the
emitter reports them as the reason a document is not eligible, the run-time loader as the load
error.

| Case | Message (WinUI's where it has one) | Step |
|---|---|---|
| A member that does not resolve | `Unable to resolve property or method of name '...' on type '...'.` | X1 |
| `TwoWay` to a read-only member | `TwoWay binding path cannot write to read-only source member '...'` | X1 |
| `TwoWay` to a named element, or without a path | `TwoWay binding path cannot write to the named element '...'`; `A TwoWay x:Bind needs a path that ends in a writable property` | X1 |
| A mode other than `OneTime`, `OneWay`, `TwoWay` | `x:Bind Mode must be OneTime, OneWay or TwoWay` | X1 |
| `UpdateSourceTrigger` | `'Explicit' is not a supported value for 'UpdateSourceTrigger'`; `'UpdateSourceTrigger' may only be used with binding 'Mode=TwoWay'` | X1 |
| A template without `x:DataType` | `DataTemplates containing x:Bind need a DataType to be specified using 'x:DataType'` | X1 |
| In a style | `Cannot use x:Bind in Styles` | X1 |
| Not x:Bind syntax | `Invalid binding path: '$self' is not x:Bind syntax` (and `$parent`, `#name`, `!`, `^`) | X1 |
| A form of a later step | `Invalid binding path: a cast is not supported yet (... step X2)`, the same for indexers, methods (X2, X4) and `ControlTemplate` (X5); each step removes its own | X1 |
| A function with the wrong arity or argument types; a return type that does not match | WinUI's `BindAssignment_FunctionReturnTypeInvalid` | X2 |
| `BindBack` | `BindAssignment_BindBack_*` | X2 |
| A converter with a function; `ConverterParameter` without a converter; `TargetNullValue` on a non-nullable target | WinUI's messages | X3 |
| An event method with the wrong signature | WinUI's `BoundEventAssignment` validation | X4 |

At run time (`XBindings::diagnostics`, and the result of `initialize` and `update`): a root that
was released, a template item that is not of the `x:DataType`, a path that could not be read or a
value that could not be converted to the target.

### 3.8 Tests

Each step adds its forms to the fixtures of `tests/FerroUI.Markup.Xaml.UnitTests/markup_extensions/x_bind/`,
which run through the run-time loader and through the emitter (`x_bind.generated.rs`, checked in,
compiled with the tests, compared with the current compiler), with the same assertions on both,
following `compiled_tests.rs` of the WinUI crate; and its diagnostics as load errors and compile
errors (`generate_file`). The behaviours of WinUI's regression projects are ported with the step
of their forms (`BasicTests`, `TwoWayTests` in X1; `FunctionTests`, `CastingTests` in X2;
`EventTests` in X4; `PhasingTests`, `ListAndTemplateTests` in X5).

## 4. Steps

| Step | Content | Public API | Risk | Guard tests |
|---|---|---|---|---|
| X0 | This plan | none | none | none |
| X1 | The core: `XBindExtension`, `XBinding`, `XBindings` and the root node in `ferroui-markup-xaml`; `x:Bind` mapped onto it; the root source (the document root, the `x:DataType` of a `DataTemplate`); property paths, attached properties, named elements; `OneTime`/`OneWay`/`TwoWay` with `UpdateSourceTrigger`; the lifecycle with initialization on attach and the template update on a data context change; the diagnostics of section 3.7 for X1. **Done** | additions (`CompiledBindingPath::build_expression` public) | low: a new extension; the compiled-binding transformers gain a branch for its type only; no generated output of an existing document changes | `x_bind_extension_tests.rs`: both back ends on the root and template fixtures (modes, triggers, named elements, attached property, `Update`, `StopTracking`, attach, recycling, isolation of instances, release of the root); diagnostics as load errors with the position and as compile errors; the freshness of the generated fixture; the pipeline order test unchanged |
| X2 | Paths of WinUI's grammar: casts `(ns:T)path` and `((ns:T)path).b`, `path.(ns:Owner.Prop)` with interface members, static members `ns:Type.Member`, integer and string indexers (`Items[0]`, `Map['key']`), functions (instance, path and static, with path and literal arguments, re-evaluated when an argument path changes) as a path element with a generated typed closure, `BindBack`; the TwoWay-after-cast error | additions (a function element of `CompiledBindingPathBuilder`) | medium: the engine's grammar is extended or a WinUI path parser added beside it; the emitter writes closures | `FunctionTests` and `CastingTests` behaviours in both back ends; arity, argument and return-type diagnostics |
| X3 | `Converter`, `ConverterParameter`, `ConverterLanguage`, `FallbackValue`, `TargetNullValue` on `XBindExtension`, into the expression options; their WinUI checks | additions | low: the engine already has the options | converter, fallback and null-value cases of `TwoWayTests` and `NullableTests`; diagnostics |
| X4 | Event binding: `Click="{x:Bind Model.OnClick}"`, a method with the event's signature, none, or a base signature; the emitter writes a typed handler, the interpreter an invoker; the handler re-reads its path on `Update` as WinUI does | additions | medium: the event assignment path of the compiler (`FerroXamlIlTransformRoutedEvent`) gains a binding form | `EventTests` behaviours; the signature diagnostic |
| X5 | `x:DefaultBindMode` (element and subtree); `x:Phase` through `IDataTemplateComponent` (`XBindings` implements `ProcessBindings` and `Recycle`; the contract moves from `ferroui-controls-repeater` to `ferroui-controls` so that the markup library can implement it, with a re-export); `x:Load`; x:Bind in a `ControlTemplate` with `TargetType`; `Window` roots on `Activated` | additions; a moved trait (re-exported) | medium: phasing touches ItemsRepeater recycling | `PhasingTests` and `ListAndTemplateTests` behaviours; deferred load; the template root |
| X6 | The WinUI importer onto the core: x:Bind kept as written, validation only for forms WinUI rejects; the adapter and its diagnostics removed; `winui-controls.md` 10.6 rows updated; the WinUI compiled fixture regenerated | removals in `ferroui-markup-xaml-winui` (a preview release) | medium: the Gallery and calculator pages that use x:Bind move to the core | the WinUI crate's compiled tests on the core; the importer's remaining diagnostics |
| X7 | Validation and documentation: the WinUI Gallery (#418) and calculator (#443) pages that use x:Bind load and run without the adapter; hot reload calls `update` on a repopulated root; migration notes; the typed accessor of the bindings of a class document (`MyPage::bindings()`, section 6) | additions | low | gallery and calculator tests; hot reload test |

## 5. Not in scope

- WinUI's generated C#/C++ (the `Bindings` classes, `XamlBindingInfo`, connection ids): the core is
  the Rust emitter's output and the runtime above.
- `x:Bind` in `ResourceDictionary` files with code-behind (WinUI's `IsResourceDictionary` case of
  `CSharpPagePass2.tt`) beyond what the root source gives: a dictionary root is not a visual, and
  is initialized with `initialize`.
- `INotifyDataErrorInfo` validation through x:Bind (`INotifyDataErrorInfoTests`): the engine's
  data validation is not wired to x:Bind in 0.5.
- The `DisableXBind` project switch of WinUI.

## 6. Decisions and open questions

Decided on the merits:

- **One extension type, mapped from `x:Bind`.** No x:Bind-specific binding engine: the expression
  is the engine's, the path the compiled one, so there is one implementation of modes, triggers,
  notification and conversion for `{CompiledBinding}` and `{x:Bind}`.
- **The scope is decided by the compiler.** The `DataType` set on the extension says "template";
  without it the root is the document's. The run-time loader and the emitter agree because they run
  the same transformer.
- **Classless documents bind to their root element.** A superset of WinUI (section 3.2).
- **Initialization on attach to a visual tree** stands for `Loading` (the framework has no
  `Loading` event); hot reload and live roots call `initialize` (X7 wires hot reload).
- **A template instance keeps its name scope** (section 3.5), so named elements work in templates;
  the scope holds the root weakly (it is the scope's owner), so there is no cycle.
- **The adapter is removed in X6, not kept as a compatibility layer.** The importer is opt-in and
  at a preview; keeping two x:Bind implementations would keep two sets of semantics.

Decided (2026-11-03, on the merits, as the owner asked for the open decisions of 0.5):

- **`WinUIXBindPolicy` after X6** stays, with `Compile` (the core) as the default and `Reject`
  for hosts that want every x:Bind of an imported document reported as an error, for example
  during a migration that rewrites x:Bind to `{Binding}` by hand. The run-time adapter's policy
  (`PropertyPathsWithExplicitLifecycle`) is removed with the adapter.
- **A typed accessor of the bindings for class documents** is generated in X7:
  `MyPage::bindings()` returns the root's `XBindings`, as WinUI's `this.Bindings`. It is a thin
  generated method over `XBindings::get_for_root`, emitted only for class documents that use
  x:Bind, so documents without x:Bind gain no API.

## 7. Status

- **X0** (this plan) and **X1** (the core) are done: `{x:Bind}` and `{XBind}` in FerroUI XAML,
  through the run-time loader and the Rust emitter. The next step is X2.
