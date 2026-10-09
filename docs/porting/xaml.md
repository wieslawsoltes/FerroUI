# XAML in FerroUI: porting XamlX and the markup stack

Status: design proposal. Scope: `external/XamlX`, `src/Markup/*`, `src/Avalonia.Build.Tasks`, `src/tools/Avalonia.Generators` of the upstream (Avalonia, branch `main`).
Goal: existing `.xaml` files keep working unchanged, with the same semantics.

## 0. Summary

1. **Port the XamlX front end 1:1, replace the back end twice.** Parser, AST, transform pipeline, the `IXamlType*` type-system abstraction and the backend-generic `Emit` layer port almost line for line (about 7k of 12.5k lines). `IL/`, `XamlX.IL.Cecil` and the SRE type system are dropped. Two new backends consume the *same transformed AST*:
   - **Rust source emitter** (build time, default path). Replaces Cecil IL emission and `Avalonia.Build.Tasks`.
   - **AST interpreter** (run time). Replaces `System.Reflection.Emit`; it is the only way to reach parity with `AvaloniaRuntimeXamlLoader` and is the base for previewer / hot reload.
2. **One metadata model feeds both.** There is no reflection in Rust, so every XAML-visible type publishes a `XamlTypeMetadata` record (constructors, properties including plain ones, events, content property, collection adders, attributes, casts, parse function). The class declaration macros are the single source of truth; they are (a) expanded by rustc into static metadata for the runtime registry and (b) read syntactically by the build tool from source. Both produce the same `IXamlTypeSystem` implementation.
3. **Build integration is `build.rs`, not a proc macro.** `ferroui_build::compile_xaml()` scans the crate's `.xaml` files and Rust declarations, runs the pipeline (including the cross-document group transformers), writes Rust into `OUT_DIR`, embeds assets, and exports a `.xamlmeta` file to dependent crates through Cargo `links` metadata. A declarative `include_xaml!` pulls the generated code into the class's module.
4. **User types in the crate being compiled** (the hard case) are handled by a source scanner (`syn`) over our own rigid declaration grammar, the same trick upstream's `Avalonia.Generators` uses with `RoslynTypeSystem`. Where the scanner cannot type something, the emitter defers to rustc through conversion traits.
5. **Compatibility naming.** Types carry an explicit dotted XAML namespace and assembly name (`FerroUI.Controls`), independent of Rust module paths. Upstream URIs, `clr-namespace:Avalonia.*` and `avares://Avalonia.*` are mapped by a data file (`xmlns-compat.toml`), so the upstream name never appears in code.
6. **Code-behind**: `x:Class` maps to a user-declared `ferro_class!` struct that embeds a generated `<Class>Xaml` parts struct (typed named elements) and gets a generated `initialize_component()`. Event handlers are Rust methods found by PascalCase to snake_case name mapping.
7. **MVP**: one Window (StackPanel, Border, TextBlock, Button with `Click`, `x:Name`, attached property, parsed values) loaded through both paths with identical tree dumps.
8. **Most urgent**: section 7.2 lists what Base/Controls must provide. The two items that shape core code being written now are the per-class metadata block in `ferro_class!` and the cast table for boxed values.

## 1. Architecture

### 1.1 Options evaluated

| Option | Verdict | Why |
|---|---|---|
| (a) Rust source emission at build time | **Default path** | Type-checked by rustc, no runtime metadata needed in release binaries, zero startup parse cost, dead code elimination keeps working. Direct analogue of the Cecil path. |
| (b) Runtime interpreter over a metadata registry | **Required, second path** | Only possible equivalent of `AvaloniaRuntimeXamlLoader` (no runtime codegen in Rust). Needed for previewer, hot reload, `Load(string)` APIs, tests. Costs binary size, so it sits behind a Cargo feature. |
| (c) Both, sharing one transformed AST | **Recommended** | Upstream already runs one pipeline with two IL backends (Cecil and SRE). We keep that shape: one pipeline, two backends. Semantics cannot drift because all language logic lives in transformers. |
| Proc macro `xaml!{}` / `include_xaml!` as the compiler | Rejected as the driver | No reliable file dependency tracking, no view of all documents (group transformers need them), re-runs on every IDE expansion, cannot emit assets or `.xamlmeta`. Kept only as optional sugar. |
| Interpreter only | Rejected | Loses compile-time errors and typed compiled bindings, forces full metadata and all types into every binary. |
| Translate XAML to a Rust DSL once, drop XAML | Rejected | Breaks the "existing .xaml keeps working" requirement. |

### 1.2 What ports 1:1 and what is replaced

| XamlX part | Lines | Decision |
|---|---|---|
| `Parsers/` | 1.7k | Port. `XDocumentXamlParser` rewritten over a Rust XML reader with namespace and position support (`roxmltree` or `quick-xml`); `CompatibleXmlReader` (mc:Ignorable handling) and the markup-extension scanner (`MeScanner`) port line for line. |
| `Ast/` | 1.8k | Port. Class hierarchy becomes `Rc<dyn XamlAstNode>` with downcasts (the transformers pattern-match on node type heavily), `IXamlAstVisitor` kept. IL-specific members (`IXamlILOptimizedEmitablePropertySetter`) become backend-neutral setter descriptions. |
| `Transform/` | 2.9k | Port unchanged in structure and order. |
| `TypeSystem/` | 1.0k | Port the traits; new implementation (section 2). |
| `Emit/` | 0.6k | Port. Already generic over `TBackendEmitter, TEmitResult`. |
| `Compiler/` | 0.2k | Port `XamlCompiler<B, R>`; `XamlImperativeCompiler` only if needed (unused by upstream markup). |
| `IL/` | 3.9k | Replaced. `Emitters/*` semantics (property assignment with dynamic setter dispatch, BeginInit/EndInit, markup extension `ProvideValue` with non-matching results, deferred content, parent stack push/pop) are re-implemented once per backend. `RuntimeContext.cs` (context class, service provider, parent stack) becomes a normal Rust struct in the markup runtime instead of an emitted type. `CheckingIlEmitter`, `RecordingIlEmitter`, `SreTypeSystem` are dropped. |
| `XamlX.IL.Cecil` | n/a | Dropped. |
| `XamlX.Runtime` | small | Port (`IProvideValueTarget`, `IRootObjectProvider`, `IUriContext`, parent stack, namespace info interfaces as traits). |

### 1.3 Pipeline

```
.xaml ──parse──► XamlDocument ──transformers──► typed AST ──group transformers──► final AST
                                   ▲                                              │
                        IXamlTypeSystem (metadata)                  ┌─────────────┴─────────────┐
                        ▲                    ▲                      ▼                           ▼
              .xamlmeta + source scan   runtime registry     RustEmitter (build.rs)     Interpreter (runtime)
                 (build time)             (run time)         -> OUT_DIR/*.rs            -> live objects
```

Rules that keep the two backends equivalent:
- A backend never makes language decisions. If a decision needs type knowledge it belongs in a transformer.
- Every AST node that reaches a backend has exactly one `emit` (Rust) and one `eval` (interpreter) implementation, registered in the same table; a missing pair is a build failure of the loader crate.
- A conformance suite loads each test document through both paths and compares a canonical tree dump (types, set properties, bindings, name scopes).

## 2. Type-system backend

### 2.1 Mapping of the abstraction

| XamlX | Rust (`xamlx::type_system`) | Backed by |
|---|---|---|
| `IXamlTypeSystem` | `trait XamlTypeSystem` | `MetadataTypeSystem` over a set of `CrateMetadata` |
| `IXamlAssembly` | `trait XamlAssembly` | one per crate; name is the crate's *assembly name* (`FerroUI.Controls`), carries `XmlnsDefinition` attributes |
| `IXamlType` | `trait XamlType` | `XamlTypeMetadata`; `Id` is the metadata address (runtime) or FQN (build) |
| `IXamlProperty` | `trait XamlProperty` | `XamlPropertyMetadata` (plain) or the registered property projected as a CLR-style property |
| `IXamlMethod` | `trait XamlMethod` | getters/setters, adders, `Parse`, static factories, handler methods, attached `GetX/SetX` |
| `IXamlConstructor` | `trait XamlConstructor` | `XamlConstructorMetadata` (named Rust functions, since Rust has no overloads) |
| `IXamlEventInfo` | `trait XamlEventInfo` | `XamlEventMetadata` |
| `IXamlField` | `trait XamlField` | registered-property statics (`BackgroundProperty`), routed-event statics, enum members, constants |
| `IXamlCustomAttribute` | `trait XamlCustomAttribute` | `XamlAttribute { type, positional, named }` synthesised from metadata fields |
| generics (`MakeGenericType`, `GenericArguments`) | supported on definitions | build path: any instantiation; runtime path: pre-registered instantiations only |
| `IXamlTypeBuilder`, `IXamlMethodBuilder`, `IXamlILEmitter` | dropped | replaced by `RustEmitter` (a structured source writer) |

Synthetic members keep the transformers unchanged: a styled property `Background` appears both as field `BackgroundProperty` and as property `Background`, exactly what the upstream transformers look for. Well-known types (`AvaloniaXamlIlWellKnownTypes`) resolve by the same dotted names after the compat prefix mapping.

### 2.2 Reflection metadata each class must expose

```rust
pub struct XamlTypeMetadata {
    pub type_info: Option<&'static TypeInfo>,   // classes; None for values/enums/interfaces
    pub type_id: fn() -> TypeId,                // of the *boxed* representation
    pub name: &'static str,                     // "Border"
    pub namespace: &'static str,                // "FerroUI.Controls" (dotted, XAML-facing)
    pub rust_path: &'static str,                // "ferroui_controls::Border" (for the emitter)
    pub kind: Class | Struct | Enum { flags, members } | Interface | Primitive,
    pub base: Option<&'static XamlTypeMetadata>,
    pub interfaces: &'static [&'static XamlTypeMetadata],
    pub generic: Option<GenericInfo>,           // definition + arguments
    pub constructors: &'static [XamlConstructorMetadata],  // params, invoke: fn(&[BoxedValue]) -> BoxedValue
    pub properties: &'static [XamlPropertyMetadata],
    pub registered: fn() -> &'static [&'static FerroProperty], // styled/direct/attached owned here
    pub events: &'static [XamlEventMetadata],   // routed (static RoutedEvent) and plain
    pub methods: &'static [XamlMethodMetadata], // handlers, command methods, statics, factory methods
    pub statics: &'static [XamlStaticMetadata], // for x:Static
    pub content_property: Option<&'static str>,
    pub collection: Option<CollectionInfo>,     // add(item) per accepted item type, dictionary add(key, value), IAddChild
    pub parse: Option<fn(&str, &ParseContext) -> Result<BoxedValue, ParseError>>,
    pub type_converter: Option<&'static XamlTypeMetadata>,
    pub casts: &'static [CastEntry],            // boxed-value conversions, see below
    pub attributes: TypeAttributes,
}
```

Per property: XAML name (PascalCase), Rust accessor names, value type, `get: Option<fn(&dyn Any) -> BoxedValue>`, `set: Option<fn(&dyn Any, BoxedValue)>`, and attributes.

Attribute coverage (all consumed by upstream transformers, so all are needed):

| Attribute | Where | Stored as |
|---|---|---|
| `XmlnsDefinition`, `XmlnsPrefix` | crate | `CrateMetadata::xmlns_definitions` |
| `Content` | property / type | `content_property` |
| `TemplateContent` (+ `TemplateResultType`) | property | `deferred_content: Option<result type>` |
| `DependsOn` | property | `depends_on: &[&str]` |
| `UsableDuringInitialization` | type | flag |
| `TypeConverter` | type / property | `type_converter` |
| `ControlTemplateScope` | type | flag |
| `DataType` | property | flag |
| `InheritDataTypeFrom`, `InheritDataTypeFromItems` | property | `{ scope kind, ancestor property, ancestor type }` |
| `AssignBinding` | property | flag |
| `ResolveByName` | property | flag |
| `MarkupExtensionOption`, `MarkupExtensionDefaultOption` | property (On* extensions) | option value |
| `AvaloniaList` (separators, split options) | type / property | `ListParseInfo` |
| `WhitespaceSignificantCollection`, `TrimSurroundingWhitespace` | type | flags |
| `TemplatePart`, `PseudoClasses` | type | lists (template parts checker, selector validation, tooling) |
| `Obsolete`, `Unstable`/`Experimental` | any | message (warning transformer) |
| `ISupportInitialize`, `IAddChild<T>`, `INameScope`, `IThemeVariantProvider` | interfaces | `interfaces` |

**Casts.** Boxed values hold exactly `T`, so the interpreter and reflection bindings need explicit conversions that C# gets for free: `Rc<Border>` to `Rc<dyn ControlClass>`, to each ancestor handle, to implemented interface handles (`Rc<dyn IBrush>`), into `Option<_>`, and numeric widenings. `casts` is a table `(target TypeId, fn(&BoxedValue) -> Option<BoxedValue>)` generated by the class macro for every ancestor and declared interface. The Rust emitter does not use it (it writes `as Rc<dyn Trait>` or `.into()`).

### 2.3 How metadata is produced

- `ferro_class!` grows an optional metadata block; `ferro_property!` already states name and value type. Proposed surface:

```rust
ferro_class!(Border: Decorator, xaml {
    namespace: "FerroUI.Controls",
    new: Border::new(),                         // default constructor
    content: Child,                             // inherited ones need not repeat
    interfaces: [IAddChild<Control>],
    properties: [ /* plain (non-registered) ones only */ ],
    events: [], attrs: [],
});
```

- Value types and enums: `#[derive(XamlValue)]` / `ferro_enum!` / `xaml_value!(Thickness, parse = Thickness::parse)`. View models: `#[derive(XamlObject)]` or `ferro_view_model!`, generating property metadata plus change notification.
- Expansion produces `impl XamlTyped for Border { const XAML: &'static XamlTypeMetadata }` and adds the type to the crate's table. Each crate exposes `pub fn xaml_metadata() -> &'static CrateMetadata` listing its types and its dependencies' tables. **Registration is explicit** (`FerroXamlLoader::register(my_app::xaml_metadata())`, called by generated app startup code), not `inventory`/`ctor`: life-before-main is unreliable on wasm, iOS static libs and with dead stripping.
- Runtime metadata is behind the Cargo feature `xaml-runtime` on every framework crate. Compiled-XAML-only applications do not pay for invoker thunks or lose dead-code elimination.

### 2.4 Where the build tool gets metadata

| Source | Mechanism | Used for |
|---|---|---|
| Dependency crates | Each crate's `build.rs` calls `ferroui_build::export_metadata()`, which scans its own sources and writes `OUT_DIR/<crate>.xamlmeta` (JSON, includes paths of its dependencies' files). Cargo `links = "<crate>_xaml"` + `cargo::metadata=xamlmeta=<path>` hands the path to dependents as `DEP_<CRATE>_XAML_XAMLMETA`. | Framework and library types |
| The crate being compiled | Same scanner, run in-process over `src/**/*.rs` before compiling XAML. | User controls, view models, `x:Class` types |
| Runtime registry dump | A generated `#[test]` in every crate compares the scanned `.xamlmeta` with the registry built from macro expansion. | Guard against scanner/macro drift |

Rejected: **linking dependency crates into the build script** as the primary mechanism. It works (and `MetadataTypeSystem` can run over a linked registry, which the test above does), but it compiles the whole framework a second time for the host profile, breaks down when a dependency links native code, and still does nothing for the local crate. Rejected: proc macros writing metadata files as a side effect (ordering and caching are undefined).

### 2.5 User types in the crate being compiled: solution and limits

The scanner parses Rust with `syn` and recognises only our declaration forms (`ferro_class!`, `ferro_property!`, `ferro_event!`, `#[derive(XamlObject|XamlValue)]`, `ferro_enum!`, `#[xaml_handlers] impl`). This is the upstream `Avalonia.Generators` approach (`RoslynTypeSystem` + `MiniCompiler`) applied to the full compiler.

Type names inside declarations are resolved by simple name against the crate's own types, then dependency metadata, honouring `use` items of the declaring file. Fallback when a member type cannot be resolved: the property is typed `XamlPseudoType::Unknown` and the emitter writes trait-directed code, letting rustc finish the job:

```rust
ferroui_markup_xaml::rt::set_from_xaml(&obj, MyControl::ratio_property(), "0.5"); // T: FromXamlStr inferred
```

Limits (documented, each produces a targeted diagnostic):
- Types produced by user macros or declared outside `src/` are invisible; they need a hand-written `xaml_extern!{}` description or must live in a dependency crate.
- `cfg`-gated declarations are evaluated only for simple `feature`/`target_*` predicates.
- Type aliases and glob re-exports from non-FerroUI crates are not followed.
- Generic user types work on the build path; the runtime loader sees only instantiations listed in `xaml { instantiate: [...] }`.
- Splitting types into a library crate and XAML into a dependent crate always works and is the documented escape hatch.

## 3. XAML compatibility surface

### 3.1 Namespaces

- Every type has `namespace` (dotted, upstream-shaped: `FerroUI.Controls.Primitives`) and every crate an assembly name (`[package.metadata.ferroui] assembly = "FerroUI.Controls"`; default: package name PascalCased). User-type default namespace: assembly name + module path PascalCased (`my_app::view_models` gives `MyApp.ViewModels`), overridable in the declaration.
- `XmlnsDefinition(uri, namespace)` lists are crate metadata, mirroring upstream `AssemblyInfo.cs` one to one. FerroUI gets its own default URI.
- `clr-namespace:NS;assembly=ASM` and `using:NS` are resolved by `NamespaceInfoHelper` exactly as upstream, against the dotted names.
- **Compat data file** `src/Markup/FerroUI.Markup.Xaml.Loader/compat/xmlns-compat.toml` (embedded with `include_str!`, data, not code): URI aliases (upstream default URI to FerroUI URI), namespace prefix rewrite (`Avalonia` to `FerroUI`), assembly aliases (also used for `avares://Avalonia.Themes.Fluent/...`), and the BCL table below. Applied in exactly two places: xmlns resolution and URI resolution.
- BCL types (`clr-namespace:System;assembly=mscorlib|netstandard|System.Runtime`, `x:String`, `x:Double`, `x:Int32`, `x:Boolean`, `x:Object`, `sys:TimeSpan`, `sys:Uri`, ...) map through a closed table to Rust types (`String`, `f64`, `i32`, `bool`, boxed value, `TimeSpan`, `Uri`). Anything outside the table is an error naming the type.

### 3.2 Directives and intrinsics

| Feature | Build path | Runtime path |
|---|---|---|
| `x:Class` | Root type resolved by dotted name to the user's struct; generates `populate`/`initialize_component` in that type's module (3.8) | With a root instance: populate it. Without: construct via metadata |
| `x:Name` / `Name` | `NameScope` registration + typed field in `<Class>Xaml` | `NameScope` registration; fields filled through a generated or metadata setter |
| `x:Key` | unchanged transformer; type keys become `&'static TypeInfo`-based keys | same |
| `x:DataType`, `x:CompileBindings` | unchanged transformers | same; compiled bindings fall back to metadata accessors |
| `x:TypeArguments` | emits `Foo::<A, B>` | only registered instantiations |
| `x:Static` | resolved through `statics` metadata to a Rust path (`Brushes::red()`, enum variant, const) | invoker thunk |
| `x:Type` | `T::XAML` / `T::TYPE` | registry lookup |
| `x:Null` | `None` (target must be `Option<_>`, else compile error like upstream's null-to-value-type error) | boxed `None` via cast table |
| `x:Array` | `vec![...]` / `Rc<[T]>` per target | boxed `Vec<T>` (element type must be registered) |
| `x:FactoryMethod`, `x:Arguments` | named constructor / static method from metadata; argument conversion by `ConstructableObjectTransformer` | invoker |
| `x:Shared`, `x:Precompile`, `x:ClassModifier`, `x:FieldModifier` | as upstream (`FieldModifier` maps to field visibility) | ignored like upstream |
| `xml:space`, whitespace rules | `WhitespaceNormalization` ports unchanged | same |
| `mc:Ignorable`, `d:` | `CompatibleXmlReader` + design transformer; `d:` kept only in design mode | same |

### 3.3 Markup extensions

All are ordinary classes with metadata plus `provide_value(&self, sp: &dyn ServiceProvider) -> BoxedValue`; third-party extensions work the same way. The service provider is a `TypeId`-keyed lookup returning trait objects (`IProvideValueTarget`, `IRootObjectProvider`, `IUriContext`, parent stack, namespace info, name scope).

| Extension | Notes |
|---|---|
| `Binding` | Routed by `AvaloniaBindingExtensionTransformer` to compiled or reflection binding depending on `x:CompileBindings` and loader config, as upstream |
| `CompiledBinding` | 3.6 |
| `ReflectionBinding` | 3.7 |
| `TemplateBinding` | property resolved at compile time against the template's target type |
| `StaticResource` | runtime walk of the parent stack; `XamlIlRuntimeHelpers` parent-stack provider ports as is |
| `DynamicResource` | `DynamicResourceExpression` port; depends on core resource-changed notifications |
| `OnPlatform`, `OnFormFactor` | `AvaloniaXamlIlOptionMarkupExtensionTransformer` ports; branches stay lazily evaluated. Build path may fold `OnPlatform` to `cfg!(target_os)` as an optimisation (upstream relies on trimming feature switches) |
| `RelativeSource`, `ResolveByName`, `x:*` | direct ports |

### 3.4 Templates, styles, resources

- **Deferred content** (`TemplateContent` properties): the AST's `XamlDeferredContentNode` becomes a free build function (`fn(&Rc<dyn IServiceProvider>) -> Result<MarkupValue, XamlLoadException>`, passed to `rt::defer`) on the build path and a captured AST subtree + interpreter on the runtime path. `DeferredTransformationFactoryV3` semantics (new name scope per instantiation, captured parent stack) port into `rt::deferred_content`.
- `ControlTemplate`, `DataTemplate`, `TreeDataTemplate`, `ItemsPanelTemplate`, `FocusAdornerTemplate`, `Template` port as thin classes over `TemplateContent`.
- **Selectors**: `SelectorGrammar` ports unchanged; `AvaloniaXamlIlSelectorTransformer` turns the syntax tree into typed nodes; the emitter writes builder calls (`Selectors::of_type(None, Button::TYPE).class("accent")`), the interpreter calls the same builders. Property selectors (`[IsEnabled=true]`) use the property-typed value conversion. Container queries (`ContainerQueryGrammar`, `AvaloniaXamlIlQueryTransformer`) likewise.
- **Setters**: `AvaloniaXamlIlSetterTransformer` resolves `Property` against the style/template target type and converts `Value` with that property's type. Unchanged logic.
- **ControlTheme**, target-type metadata nodes, duplicate setter checker, template parts checker: unchanged logic.
- **Resources**: deferred `ResourceDictionary` entries (`AddDeferred`, `AddNotSharedDeferred`, `EnsureCapacity`) port; requires those methods on the core `ResourceDictionary`.
- **Includes**: `StyleInclude`/`ResourceInclude`/`MergeResourceInclude` with `Source`. Same-crate documents are linked at compile time by `XamlIncludeGroupTransformer` to a direct call of the target's generated `build_*` function; cross-crate ones call the other crate's generated loader function when the target is in dependency metadata (each `.xamlmeta` lists compiled document URIs), else fall back to `FerroXamlLoader::load(uri)`.
- **Assets and `avares://`**: `ferroui_build::embed_assets()` replaces `GenerateAvaloniaResourcesTask`. It writes an index of `include_bytes!` entries (`OUT_DIR/assets.rs`), registered with the core asset loader under the crate's assembly name. URI scheme and resolution rules (`avares://Assembly/path`, relative, rooted `/path`, `resm:` rejected with a clear message unless the owner wants it) are kept. Compiled documents are registered in a per-crate `xaml_loader::try_load(uri)` table, the equivalent of `CompiledAvaloniaXaml.!XamlLoader.TryLoad`.

### 3.5 Property-type-directed parsing

Order is exactly `XamlTransformHelpers.TryConvertValue`: custom converter (`AvaloniaXamlIlLanguage.CustomValueConverter` + `ParseIntrinsics`), enum, primitive constant, `Parse(string[, IFormatProvider])`, `TypeConverter` attribute, single-string constructor, implicit conversion.

- Compile-time intrinsics (Thickness, Point, Vector, Size, Matrix, CornerRadius, Color, RelativePoint, GridLength, Row/ColumnDefinitions, Cursor, TextTrimming, TextDecorations, WindowTransparencyLevel, Uri, ThemeVariant, FontFamily, TimeSpan, point lists, `AvaloniaList` constants) are evaluated by the compiler and emitted as constructor calls. To do this at build time without linking the framework, the handful of value parsers live in a dependency-free crate (`ferroui-xaml-values`, or simply re-used from `FerroUI.Base` since Base has no native deps; decision in section 8).
- Everything else: build path emits `<T as FromXamlStr>::from_xaml_str("...", &ctx)` (blanket impl over `FromStr`); runtime path calls `parse` from metadata.
- .NET invariant-culture parsing differs from Rust `FromStr` (leading/trailing whitespace, `NaN`/`Infinity` spelling, `1e3`, `,` vs space separators, case-insensitive enums and booleans, flag enums with commas, numeric enum values). A small `clr_parse` module provides compatible parsers and is the only thing `FromXamlStr` for primitives uses.

### 3.6 Compiled bindings

`AvaloniaXamlIlBindingPathParser` + `XamlIlBindingPathHelper` port with one change: the element emitters. Upstream emits IL accessors and `ClrPropertyInfo` instances (`XamlIlClrPropertyInfoHelper`, `XamlIlPropertyInfoAccessorFactoryEmitter`, `XamlIlTrampolineBuilder`); we emit **Rust closures**:

```rust
CompiledBindingPathBuilder::new()
    .property(ClrPropertyInfo::new("Name",
        |o| o.downcast_ref::<PersonVm>().map(|p| box_value(p.name())),
        Some(|o, v| o.downcast_ref::<PersonVm>().unwrap().set_name(unbox(v)))),
        PropertyInfoAccessorFactory::create_inpc_property_accessor)
    .build()
```

| Path element | Replacement |
|---|---|
| registered property | `.property(Type::x_property(), ...)` |
| plain property | typed getter/setter closures as above; change notification through the `NotifyPropertyChanged` trait |
| method, method-as-command | closure wrapping the Rust method; `CanExecute` method and `DependsOn` from metadata |
| indexer / array index | closure over `Index`-like metadata accessor; list/dictionary change notifications from core collections |
| `^` stream | `Task<T>` and `IObservable<T>` equivalents from Base (`reactive`, future wrapper) |
| `!`, type cast, `$self`, `$parent[T; n]`, `#name`, `$templatedParent`, attached `(Owner.Prop)` | direct ports; casts use `casts` table at runtime, `downcast` in emitted code |

On the runtime path the same nodes evaluate to `ClrPropertyInfo` built from metadata `get`/`set` thunks.

#### 3.6.1 Bindings from expression trees (`BindingExpressionVisitor`): design note

Status: written for task 3 of `CONTINUATION.md` (core port, 2026-10-08). The decision is with the owner (`CONTINUATION.md`, "Decisions waiting for the owner"). Until then `Data/Core/Parsers/BindingExpressionVisitorTests.cs` (36 tests) is not ported.

**What upstream does.** `CompiledBinding.Create<TIn, TOut>(Expression<Func<TIn, TOut>>, ...)` is the only caller of `BindingExpressionVisitor<TIn>.BuildPath`. The C# compiler turns a lambda such as `x => x.Child!.Items[0].Name` into a LINQ expression tree; the visitor walks it from the parameter outwards and calls `CompiledBindingPathBuilder` once per step: `Property` with reflection-built getter and setter for a member access, `Property` with an INPC or indexer accessor for `get_Item`, `ArrayElement` for an array index, `Property` over a registered property for `x[SomeProperty]`, `TypeCast` for `Convert`/`TypeAs` between reference types, `StreamTask`/`StreamObservable` for the `StreamBinding()` marker method, and `Not` for `!`. Any other node type throws `ExpressionParseException` ("Invalid expression type in binding expression: Add.", "Invalid method call ..."). The upstream binding expression tests (`BindingExpressionTests*.cs`) use the same entry point for their compiled flavour.

**What stands for it in the port.** Rust has no expression trees: a closure `|x: &Vm| x.child().name()` is opaque, so nothing can walk it at run time. The port has the two halves the visitor connects, and no visitor:

- `CompiledBindingPathBuilder` (`data/compiled_binding_path.rs`) is ported whole. The steps the visitor emits are its members: `property`/`typed_property`/`notifying_property` (member access), `indexer_property`, `list_item` and `dictionary_item` (`get_Item`), `array_element` (array index, any number of dimensions), `ferro_property` (`x[SomeProperty]`), `type_cast`/`type_cast_value` (casts), `stream_task`/`stream_observable` (`StreamBinding()`), `not` (`!`). `CompiledBindingPath::build_expression` turns the elements into the same expression nodes as upstream's `CompiledBindingPath.BuildExpression`, with the negations appended at the end.
- The calls are written by whoever holds the syntax: the XAML compiler emits them for `{CompiledBinding}` (section 9.8.2, row `XamlIlBindingPathNode`), the run-time loader builds them from metadata, and the compiled flavour of the ported binding expression tests writes them by hand (`tests/binding_test_support.rs`, `Flavor::Compiled`), one builder chain for each upstream lambda.

So the role of the expression tree is taken by the builder chain itself; the translation from source syntax to builder calls happens in the XAML compiler or in the hand-written chain, at build time, not at run time.

**Options.**

- **A. No counterpart (recommended).** `CompiledBinding.Create<TIn, TOut>(Expression)` and `BindingExpressionVisitor` stay unported and are recorded as **Missing** in `DEVIATIONS.md` (Bindings). Code builds a path with `CompiledBindingPathBuilder` and passes it to `CompiledBinding`. The 36 tests are not ported; the node shapes they assert are covered where the builder is (table below).
- **B. A declarative macro.** `binding_path!(Vm => x.child.items[0].name)` would parse a restricted Rust expression grammar and expand to builder calls, using the typed accessors of markup metadata (`MarkupProperty::typed_path_element`) for member access. It is the closest analogue of the visitor (a syntax tree translated into builder calls), but it runs at compile time: the 8 tests that expect `ExpressionParseException` become compile errors, which need a compile-fail harness the workspace does not have, and the macro would be a second path grammar beside the XAML one. It adds API that no upstream caller in the port needs today: upstream calls `CompiledBinding.Create` from no framework code, only from applications and tests.
- **C. A run-time expression model.** A small enum tree (`Member("Child")`, `Index(0)`, `Not(..)`) built by hand and walked by a ported visitor. It ports the visitor's control flow, but the tree would be written by hand exactly like the builder chain it replaces, so it adds a layer without adding a capability.

**Counterparts of the 36 tests** under option A. "Builder" means the node shape is produced by the builder member named, through `CompiledBindingPath::build_expression`; "binding tests" names a ported suite that exercises the same shape end to end in its compiled flavour.

| Upstream test | Counterpart |
|---|---|
| `BuildNodes_Should_Parse_Simple_Property`, `_Property_Chain`, `_Long_Property_Chain` | Builder `property`/`typed_property` per member, `PropertyAccessorNode` each; binding tests (`binding_expression_tests_*.rs`). |
| `BuildNodes_Should_Parse_Indexer`, `_Chained_Indexers`, `_Property_After_Indexer` | Builder `list_item`, a `PropertyAccessorNode` with an indexer accessor, as upstream; `binding_expression_tests_indexer.rs`. |
| `BuildNodes_Should_Parse_Indexer_With_String_Key`, `_With_Variable_Key` | Builder `dictionary_item`, a `PropertyAccessorNode`; a variable key is an ordinary argument. |
| `BuildNodes_Should_Parse_Array_Index`, `_Multi_Dimensional_Array` | Builder `array_element(&[..])`, an `ArrayIndexerNode`. |
| `BuildNodes_Should_Parse_AvaloniaProperty_Access`, `_In_Chain` | Builder `ferro_property` gives a `FerroPropertyAccessorNode`, not upstream's `PropertyAccessorNode` over a registered-property accessor; the value and change notification are the same. Not asserted anywhere. |
| `BuildNodes_Should_Parse_Logical_Not`, `_Logical_Not_In_Chain`, `_Multiple_Logical_Not_Operators`, `_Logical_Not_After_StreamBinding` | Builder `not`, `LogicalNotNode` appended after the other nodes, as upstream; `binding_expression_tests_negation.rs`. |
| `BuildNodes_Should_Parse_Task_StreamBinding`, `_Void_Task_StreamBinding`, `_Observable_StreamBinding`, `_StreamBinding_In_Property_Chain` | Builder `stream_task`/`stream_observable`, a `StreamNode`; `binding_expression_tests_task.rs` and `binding_expression_tests_observable.rs`. |
| `BuildNodes_Should_Create_Node_For_Upcast`, `_Upcast_In_Property_Chain`, `_Downcast`, `_Downcast_In_Property_Chain`, `_Nodes_For_Casting_Through_Object`, `_Node_For_TypeAs_Operator` | Builder `type_cast::<T>()`/`type_cast_value`, a `FuncTransformNode` each. |
| `BuildNodes_Should_Handle_Empty_Expression` | `CompiledBindingPath::new()`, no nodes. |
| `BuildNodes_Should_Handle_Unary_Plus_Operator` | None: an artefact of the C# compiler, which drops unary plus from the tree. |
| `BuildNodes_Should_Throw_For_Value_Type_Cast`, `_Addition_Operator`, `_Subtraction_Operator`, `_Multiplication_Operator`, `_Equality_Operator`, `_Conditional_Expression`, `_Method_Call_That_Is_Not_Indexer_Or_StreamBinding`, `_Unary_Minus_Operator` | None: the builder has no member for these, so they cannot be expressed. Under option B they would be compile errors. |

Count: 27 tests have a builder counterpart (25 with the same node types, 2 with `FerroPropertyAccessorNode`); 9 have none (the 8 rejections, the value-type cast among them, and unary plus). Porting the node-shape assertions of the 27 as tests of `CompiledBindingPathBuilder` (under the upstream names, with the lambda in a comment) is possible under any option and is the cheapest way to keep the shapes pinned.

### 3.7 Reflection bindings without reflection

`ReflectionBinding` (and `Binding` when not compiled) resolve each path segment at run time by: runtime type of the current value (`TypeInfo` for objects, `TypeId` lookup for boxed view models) to `XamlTypeMetadata`, property by name, `get`/`set` thunk. Change tracking uses the registered-property system or `NotifyPropertyChanged`. Consequences: the target types must have metadata and the app must enable `xaml-runtime`; a path segment on a type without metadata yields the same binding error upstream logs for a missing member. This is the main reason to make compiled bindings the default in new projects (upstream templates already do).

### 3.8 `x:Class`, code-behind, event handlers

Rust has no partial types, so the generated half is a separate struct the user embeds:

```rust
#[repr(C)]
pub struct MainWindow { base: Window, xaml: MainWindowXaml, clicks: Cell<u32> }
ferro_class!(MainWindow: Window, xaml { namespace: "MyApp.Views" });
include_xaml!("MainWindow.xaml");      // -> include!(concat!(env!("OUT_DIR"), "/xaml/MainWindow.xaml.rs"))

#[xaml_handlers]
impl MainWindow {
    pub fn new() -> Rc<Self> { let w = instantiate(/* ... */); w.initialize_component(); w }
    fn on_click(&self, sender: &BoxedObject, e: &RoutedEventArgs) { /* self.xaml.greeting() */ }
}
```

Generated per document: `pub struct MainWindowXaml` (one `OnceCell<Rc<T>>` per `x:Name`, typed accessors honouring `x:FieldModifier`), `impl MainWindow { fn initialize_component(&self); fn populate(sp, &Rc<Self>) }`, and for classless documents `pub fn build_<name>(sp) -> Rc<T>`. `initialize_component` checks a per-type override slot first, the equivalent of `!XamlIlPopulateOverride`, so the previewer/hot reload can substitute the interpreter.

Event attributes: `AvaloniaXamlIlTransformRoutedEvent` ports. `Click="OnClick"` emits `Button::click_event().add_handler(&btn, weak_closure(this, MainWindow::on_click))`; the handler name is mapped PascalCase to snake_case (exact name tried first). The scanner validates existence and signature when it can; otherwise rustc reports it at a generated line that carries a `// MainWindow.xaml(12,34)` marker. The runtime path needs the handler in metadata, which `#[xaml_handlers]` (a tiny attribute macro, or a declarative `ferro_handlers!`) provides.

Because generated code is included in the class's own module, private fields and methods are reachable (the role `IgnoresAccessChecksTo` plays upstream).

`Avalonia.Generators` (name generator, `InitializeComponent` generator) is therefore not ported as a separate tool: its output is part of the build crate's output. `AvaloniaPropertyIncrementalGenerator` corresponds to `ferro_property!` and stays in core.

### 3.9 Design-time, conditional and platform markup

- `d:DesignWidth`, `d:DataContext`, `Design.PreviewWith` etc.: `AvaloniaXamlIlDesignPropertiesTransformer` ports; active only when `IsDesignMode` (runtime loader option).
- `mc:Ignorable` prefixes are dropped by `CompatibleXmlReader` as upstream.
- Platform switches are the `On*` extensions (3.3). XAML conditional namespaces (`?ApiInformation...`) are not supported upstream either.

## 4. Diagnostics and tooling

- **Positions**: every AST node keeps `IXamlLineInfo`; diagnostics keep upstream codes (`AVLN2xxx`/`XAMLXxxxx` equivalents under an `FRN` prefix, mapping table in docs).
- **Reporting from `build.rs`**: errors as `cargo::error=path/File.xaml(12,34): FRN2000: message` (Cargo 1.84+, within our MSRV 1.86) and warnings as `cargo::warning=`; all diagnostics are also written to `OUT_DIR/xaml-diagnostics.json` for editors. The build continues past the first error to report them all, then fails.
- **Generated code** carries `// file(line,col)` markers and `#[allow]`s; type errors that only rustc can find (deferred typing, handler signatures) therefore point at a line whose comment names the XAML location.
- **Incremental builds**: `cargo::rerun-if-changed` for each `.xaml`, each asset, each scanned `.rs` that contained a declaration, the directory roots (new files), and dependency `.xamlmeta` paths. Output files are written only when content changes so rustc's own incremental cache stays warm. Per-document parse/transform results are cached by content hash; group transformers force re-emit of dependents only.
- **IDE**: generated files in `OUT_DIR` are visible to rust-analyzer (named fields and handlers complete in Rust code). The `.xamlmeta` files are a ready-made completion/validation database for a future `.xaml` language server; upstream's editor tooling depends on .NET assemblies and will not work.
- **Previewer / hot reload**: there is no dynamic assembly loading. The previewer host is the user's own binary built with `xaml-runtime` and a design entry point; it watches files, re-runs parse + transform in-process against the runtime registry, and swaps content through the populate-override slot. State-preserving reload is limited to documents whose `x:Class` layout (named fields, handlers) did not change; anything else needs a rebuild.

## 5. Crate layout

| Crate (dir) | Upstream | Contents | Depends on |
|---|---|---|---|
| `xamlx` (`external/XamlX`) | XamlX | parser, AST, transforms, type-system traits, `MetadataTypeSystem`, emit layer, `RustEmitter` primitives, interpreter core. No framework knowledge. | XML reader only |
| `ferroui-xaml-metadata` (`src/Markup/FerroUI.Markup.Xaml.Metadata`) | (attributes in Avalonia.Base `Metadata/`) | `XamlTypeMetadata` structs, `.xamlmeta` serde model. No deps, usable from build scripts and runtime. | none |
| `ferroui-markup` (`src/Markup/FerroUI.Markup`) | Avalonia.Markup | selector / property-path / container-query grammars, `Binding` shell | base |
| `ferroui-markup-xaml` (`src/Markup/FerroUI.Markup.Xaml`) | Avalonia.Markup.Xaml | markup extensions, templates, includes, converters, runtime helpers (`rt`), `FerroXamlLoader`, `include_xaml!` | base, controls, markup, metadata |
| `ferroui-markup-xaml-loader` (`src/Markup/FerroUI.Markup.Xaml.Loader`) | Avalonia.Markup.Xaml.Loader | language definition, transformers, group transformers, binding-path helpers, compat data, both backends' framework nodes, `FerroRuntimeXamlLoader` (feature `runtime`) | xamlx, metadata, markup (grammars); markup-xaml only with `runtime` |
| `ferroui-build` (`src/FerroUI.Build.Tasks`) | Avalonia.Build.Tasks + Generators | `compile_xaml()`, `export_metadata()`, `embed_assets()`, source scanner (`syn`), diagnostics | loader (no `runtime`), xamlx, metadata, value parsers |
| `ferroui-macros` (`src/tools/FerroUI.Macros`) | (none) | optional proc macros: `#[xaml_handlers]`, derives `XamlObject`/`XamlValue` | syn/quote only |

Direction: `xamlx` and `metadata` are leaves. Core (`base`, `controls`) depends on `metadata` only (to publish it). `build` never links `controls`. The loader's transformers reference framework types by name only, the same discipline upstream enforces in `AvaloniaXamlIlLanguage.cs` ("ONLY strings").

## 6. File inventory

Legend: **P** port as is, **A** adapt (same logic, Rust-specific changes), **R** replaced, **D** dropped.

### 6.1 XamlX

| Path | | Notes |
|---|---|---|
| `Ast/Common.cs` | P | node/visitor/line-info traits |
| `Ast/Xaml.cs` | P | directive, property-value, object, text, name-property-reference nodes |
| `Ast/Xml.cs` | P | `XamlAstXmlTypeReference` (+ generic args) |
| `Ast/XamlDocument.cs` | P | |
| `Ast/Intrinsics.cs` | A | `x:Null/Type/Static`, constants, root nodes; IL emit bodies become emit/eval pairs |
| `Ast/Clr.cs` | A | CLR-typed nodes, setters, manipulation, deferred content; `IXamlILOptimizedEmitablePropertySetter` becomes a backend-neutral `SetterKind` |
| `Ast/CompilerHelpers.cs` | A | locals, BeginInit wrapper, runtime cast, parent-stack nodes |
| `Parsers/XDocumentXamlParser.cs` | A | over Rust XML reader; keep line info and `x:TypeArguments` parsing |
| `Parsers/CompatibleXmlReader.cs` | A | `mc:Ignorable` |
| `Parsers/CommaSeparatedParenthesesTreeParser.cs` | P | |
| `Parsers/SystemXamlMarkupExtensionParser/*` | P | scanner ported character for character (escaping rules matter) |
| `Transform/AstTransformationContext.cs`, `XamlContextBase.cs`, `IXamlAstTransformer.cs`, `IXamlIdentifierGenerator.cs`, `XamlDiagnosticsHandler.cs` | P | |
| `Transform/TransformerConfiguration.cs`, `XamlLanguageTypeMappings.cs`, `XamlXmlnsMappings.cs`, `NamespaceInfoHelper.cs` | P | compat aliasing hooks added in `XamlXmlnsMappings` |
| `Transform/XamlTransformHelpers.cs` | A | conversion order kept; `Parse`/converter/ctor lookups go through metadata |
| `Transform/WhitespaceNormalization.cs` | P | |
| `Transformers/KnownDirectivesTransformer.cs` | P | turns configured directive elements into directive nodes |
| `Transformers/XamlIntrinsicsTransformer.cs` | P | `x:Null`, `x:Type`, `x:Static`, `x:Array` nodes |
| `Transformers/XArgumentsTransformer.cs` | P | `x:Arguments` into constructor arguments |
| `Transformers/TypeReferenceResolver.cs` | P | xmlns + name to type, `*Extension` suffix, generics |
| `Transformers/MarkupExtensionTransformer.cs` | P | object node with `ProvideValue` becomes markup-extension node |
| `Transformers/TextNodeMerger.cs` | P | |
| `Transformers/PropertyReferenceResolver.cs` | P | name to property/attached setter/event |
| `Transformers/ContentConvertTransformer.cs` | P | text content to object via conversion |
| `Transformers/RemoveWhitespaceBetweenPropertyValuesTransformer.cs` | P | |
| `Transformers/ResolveContentPropertyTransformer.cs` | P | |
| `Transformers/ResolvePropertyValueAddersTransformer.cs` | A | adders from `CollectionInfo` instead of `Add` method discovery |
| `Transformers/ApplyWhitespaceNormalization.cs` | P | |
| `Transformers/ObsoleteWarningsTransformer.cs` | P | from metadata `obsolete` |
| `Transformers/StaticIntrinsicsPostProcessTransformer.cs` | P | validates `x:Static` members |
| `Transformers/ConvertPropertyValuesToAssignmentsTransformer.cs` | P | core of typed assignment, incl. dynamic setter groups |
| `Transformers/ConstructableObjectTransformer.cs` | A | constructor choice among named constructors by argument types |
| `Transformers/DeferredContentTransformer.cs` | P | |
| `Transformers/TopDownInitializationTransformer.cs` | P | `UsableDuringInitialization` |
| `Transformers/NewObjectTransformer.cs`, `FlattenAstTransformer.cs` | P | |
| `TypeSystem/TypeSystem.cs` | A | traits; builder interfaces dropped |
| `TypeSystem/TypeSystemHelpers.cs`, `XamlTypeWellKnownTypes.cs` | A | well-known types from the BCL table |
| `TypeSystem/XamlLocalsPool.cs` | D | Rust locals are named `let`s |
| `Emit/*` | A | context and emit mappings generic over backend; locals variant simplified |
| `Compiler/XamlCompiler.cs` | P | |
| `Compiler/XamlImperativeCompiler.cs` | D | unused by markup |
| `IL/Emitters/*`, `IL/XamlIlCompiler.cs`, `IL/ILEmitContext.cs` | R | re-implemented as `RustEmitter` and `Interpreter` node handlers |
| `IL/RuntimeContext.cs`, `IL/NamespaceInfoProvider.cs` | R | plain Rust runtime structs (`rt::XamlContext`, `rt::NamespaceInfo`) |
| `IL/SreTypeSystem.cs`, `CheckingIlEmitter.cs`, `RecordingIlEmitter.cs`, `IXamlILEmitter.cs`, `ILEmitHelpers.cs`, `XamlILEmitterExtensions.cs`, `IXamlILLocal.cs` | D | |
| `Diagnostics/*`, `Exceptions.cs`, `XamlNamespaces.cs` | P | |
| `Compatibility/*`, `Extensions/*` | D | .NET polyfills |
| `XamlX.IL.Cecil/*` | D | |
| `XamlX.Runtime/Interfaces.cs` | P | traits |
| `XamlX.Parser.GuiLabs` | D | alternative parser, unused by the framework |
| `tests/XamlParserTests`, `tests/TestClasses` | A | port as the `xamlx` conformance suite, run against both backends |

### 6.2 `Markup.Xaml.Loader/CompilerExtensions`

| File | What it does | |
|---|---|---|
| `AvaloniaXamlIlCompiler.cs` | pipeline assembly and ordering | A (same order; two backends) |
| `AvaloniaXamlIlLanguage.cs` | type mappings, converter table, custom value converter, context fields | A (IL context emit becomes `rt::XamlContext`) |
| `AvaloniaXamlIlLanguageParseIntrinsics.cs` | compile-time constants for value types | A (emits constructor calls) |
| `AvaloniaXamlIlCompilerConfiguration.cs`, `AvaloniaXamlDiagnosticCodes.cs` | config, codes | P |
| `AstNodes/*ArrayConstant, *AvaloniaListConstant, *VectorLikeConstant, *FontFamily, *GridLength` | constant nodes | A (emit/eval pairs) |
| `XamlIlAvaloniaPropertyHelper.cs` | registered-property nodes and setters (SetValue, Bind, priority, UnsetValue) | A |
| `XamlIlBindingPathHelper.cs` | compiled binding path model and emit | A (closures, 3.6) |
| `XamlIlClrPropertyInfoHelper.cs`, `XamlIlPropertyInfoAccessorFactoryEmitter.cs`, `XamlIlTrampolineBuilder.cs` | IL accessors, indexer closures, private-method trampolines | R (Rust closures; trampolines unnecessary) |
| `XamlAstNewClrObjectHelper.cs`, `XamlDocumentResource.cs`, `IXamlDocumentResource.cs`, `XamlDocumentUsage.cs` | helpers | P |
| `XamlDocumentTypeBuilderProvider.cs` | type builders per document | R (output module per document) |
| `Visitors/NameScopeRegistrationVisitor.cs` | finds name registrations | P |
| **Transformers** | | |
| `XNameTransformer` | `x:Name` to `Name` property + registration marker | P |
| `IgnoredDirectivesTransformer` | strips `x:Class/Precompile/FieldModifier/ClassModifier` | P |
| `AvaloniaXamlIlDesignPropertiesTransformer` | `d:` and `Design.*` handling | P |
| `AvaloniaBindingExtensionTransformer` | `Binding` to compiled/reflection, `x:CompileBindings` scope | P |
| `AvaloniaXamlIlResolveClassesPropertiesTransformer` (`ClassesPropertyResolver.cs`) | `Classes.foo="..."` properties | P |
| `AvaloniaXamlIlTransformInstanceAttachedProperties` | attached registered properties without CLR accessors | P |
| `AvaloniaXamlIlTransformSyntheticCompiledBindingMembers` | `x:DataType`/`x:CompileBindings` as synthetic members | P |
| `AvaloniaXamlIlTransformRoutedEvent` | routed-event attributes to `AddHandler` | A (handler closure, name mapping) |
| `AvaloniaXamlIlAvaloniaPropertyResolver` | CLR property to registered property with binding-aware setters | P |
| `AvaloniaXamlIlReorderClassesPropertiesTransformer` | `Classes` before `Classes.x` | P |
| `AvaloniaXAmlIlClassesTransformer` | `Classes="a b"` to adds | P |
| `XDataTypeTransformer` | `x:DataType` directive to typed metadata | P |
| `AvaloniaXamlIlControlThemeTransformer` | `ControlTheme` target-type scope | P |
| `AvaloniaXamlIlSelectorTransformer` | selector text to typed selector nodes | A (emit builder calls) |
| `AvaloniaXamlIlQueryTransformer` | container query text to nodes | A |
| `AvaloniaXamlIlDuplicateSettersChecker` | warning | P |
| `AvaloniaXamlIlControlTemplateTargetTypeMetadataTransformer` | template target-type scope | P |
| `AvaloniaXamlIlBindingPathParser` | binding path text to path AST | P |
| `AvaloniaXamlIlSetterTargetTypeMetadataTransformer` | setter scope inside templates | P |
| `AvaloniaXamlIlSetterTransformer` | setter `Property`/`Value` typing | P |
| `AvaloniaXamlIlStyleValidatorTransformer` | style placement checks | P |
| `AvaloniaXamlIlConstructorServiceProviderTransformer` | passes service provider to constructors that take one | A (named constructor flag) |
| `AvaloniaXamlIlTransitionsTypeMetadataTransformer` | `Transitions` property scope | P |
| `AvaloniaXamlIlResolveByNameMarkupExtensionReplacer` | `[ResolveByName]` text to extension | P |
| `AvaloniaXamlIlThemeVariantProviderTransformer` | `x:Key` to `Key` in theme dictionaries | P |
| `AvaloniaXamlIlDataTemplateWarningsTransformer` | missing `DataType` warning | P |
| `AvaloniaXamlIlOptionMarkupExtensionTransformer` | `OnPlatform`/`OnFormFactor` branch nodes | A (branch emit, optional `cfg!` folding) |
| `AddNameScopeRegistration` | registers names, completes scopes | A (emitter node) |
| `AvaloniaXamlIlControlTemplatePartsChecker` | `TemplatePart` validation | P |
| `AvaloniaXamlIlDataContextTypeTransformer` | infers data context type (`DataType`, `InheritDataTypeFrom*`) | P |
| `AvaloniaXamlIlBindingPathTransformer` | resolves path AST against types | P |
| `AvaloniaXamlIlCompiledBindingsMetadataRemover`, `AvaloniaXamlIlMetadataRemover` | strip metadata nodes | P |
| `AvaloniaXamlResourceTransformer` | deferred resource entries, `x:Shared` | P |
| `AvaloniaXamlIlControlTemplatePriorityTransformer` | `BindingPriority.Template` inside templates | P |
| `AvaloniaXamlIlEnsureResourceDictionaryCapacityTransformer` | capacity hint | P |
| `AvaloniaXamlIlRootObjectScope` | root name scope set/complete | A (emitter node) |
| `AvaloniaXamlIlAddSourceInfoTransformer` | attaches source info for dev tools | P |
| `AvaloniaXamlIlWellKnownTypes` | well-known type/method lookup | A (names via compat prefix; methods become runtime helper references) |
| **Group transformers** | | |
| `XamlMergeResourceGroupTransformer` | inlines `MergeResourceInclude` at compile time | P |
| `XamlIncludeGroupTransformer` | links includes to compiled documents | A (calls generated `build_*` fns; cross-crate via `.xamlmeta` URI list) |
| **Loader root** | | |
| `AvaloniaRuntimeXamlLoader.cs` | public runtime API | A (same API shape) |
| `AvaloniaXamlIlRuntimeCompiler.cs` | SRE compile and load | R (parse, transform, interpret) |
| `CompilerDynamicDependencies.cs` | trimming annotations | D |

### 6.3 `Markup.Xaml`

| File(s) | |
|---|---|
| `AvaloniaXamlLoader.cs` | A: `load(obj)` is a no-op hook to generated code; `load(uri)` dispatches to per-crate compiled tables, then the runtime loader if registered |
| `XamlIl/Runtime/XamlIlRuntimeHelpers.cs`, `*ParentStackProvider*.cs`, `IAvaloniaXamlIl*.cs`, `EagerParentStackEnumerator.cs` | A: become `rt` module (service providers, deferred content, parent stack, non-matching markup-extension application) |
| `MarkupExtensions/*` (CompiledBinding, ReflectionBinding, StaticResource, DynamicResource, OnPlatform, OnFormFactor, On, RelativeSource, ResolveByName) | P |
| `MarkupExtensions/CompiledBindings/PropertyInfoAccessorFactory.cs` | A: accessors over closures and Base notification traits |
| `MarkupExtensions/CompiledBindings/TaskStreamPlugin.cs` | A: future-based |
| `Data/DynamicResourceExpression.cs` | P |
| `Templates/*` (Template, TemplateContent, ControlTemplate, DataTemplate, TreeDataTemplate, ItemsPanelTemplate, FocusAdornerTemplate, WindowDrawnDecorationsTemplate) | P |
| `Styling/ResourceInclude.cs`, `StyleInclude.cs`, `MergeResourceInclude.cs` | P |
| `Converters/*` | A: become `parse` functions / `TypeConverter` metadata (Bitmap and Icon need the asset loader) |
| `Parsers/PropertyParser.cs` | P |
| `RuntimeXamlLoaderConfiguration.cs`, `RuntimeXamlLoaderDocument.cs` | A: `LocalAssembly` becomes a crate metadata handle |
| `Diagnostics/XamlSourceInfo.cs`, `XamlLoadException.cs`, `Extensions.cs`, `XamlTypes.cs`, `MarkupExtension.cs` | P |
| `PortableXaml/AvaloniaResourceXamlInfo.cs` | D |

`Markup`: `SelectorGrammar`, `SelectorParser`, `PropertyPathGrammar`, `ContainerQueryGrammar`, `ContainerQueryParser` port unchanged; `DelayedBinding` ports; `Binding.cs` is a shell.
`Build.Tasks`: `XamlCompilerTaskExecutor*` becomes `ferroui_build::compile_xaml`; `GenerateAvaloniaResourcesTask` becomes `embed_assets`; `XamlCompilerDiagnosticsFilter` ports (warning-as-error / suppress from `[package.metadata.ferroui]`); `DeterministicIdGenerator` ports; MSBuild task shells, `ComInteropHelper`, `SpanCompat` dropped.
`Generators`: folded into the build crate (3.8).

## 7. Plan

### 7.1 Phases

| Phase | Deliverable | Exit test |
|---|---|---|
| 0 | Metadata contract agreed and landed in core macros (7.2 items 1 to 6) | Border/TextBlock/Button publish metadata; scanner and registry agree |
| 1 | `xamlx`: parser, AST, base transformers, `MetadataTypeSystem` | ported XamlX parser/transform tests pass against test classes |
| 2 | `RustEmitter` + `ferroui-build` + `include_xaml!` | **MVP, build path** |
| 3 | Interpreter + `FerroRuntimeXamlLoader` | **MVP, runtime path**, identical tree dump |
| 4 | Framework transformers: properties, classes, events, name scopes, styles, selectors, setters, resources, templates (deferred content) | a styled, templated Button renders from XAML both ways |
| 5 | Bindings: compiled paths, reflection bindings, `x:DataType` inference, template bindings | upstream binding XAML tests ported |
| 6 | Includes, group transformers, assets/`avares://`, ControlTheme, On* extensions | upstream Fluent theme `.xaml` compiles unmodified (the compatibility acid test) |
| 7 | Design mode, source info, previewer host, hot reload, diagnostics polish, language-server data | |

**MVP definition.** `MainWindow.xaml` with the upstream default xmlns, `x:Class`, a `StackPanel` containing a `Border` (`Background`, `Padding`, `CornerRadius` from strings), a `TextBlock` with `x:Name`, a `Button` with `Content`, `Click="OnClick"`, an attached property (`DockPanel.Dock` or `Grid.Row`), an enum value and one object-element property. Loaded (1) via `build.rs` + `include_xaml!` + `initialize_component()` and (2) via `FerroRuntimeXamlLoader::load(str)` with a root instance; both produce the same canonical tree dump, the named field is populated, the click handler runs.

### 7.2 What core (Base / Controls) must provide for XAML

Ordered by urgency. Items 1 to 6 affect declarations being written now.

1. **Metadata block in `ferro_class!`** (and a stable grammar the scanner can read): XAML namespace, default constructor and named constructors with parameter types, content property, plain (non-registered) properties with getter/setter, events, implemented interfaces, attributes of section 2.2. Expanded into `XamlTypeMetadata` under feature `xaml-runtime`. Every public class needs a parameterless constructor function (`Type::new() -> Rc<Type>`) unless XAML-unconstructible.
2. **PascalCase XAML names everywhere**: `FerroProperty::name()` returns the upstream member name (`Background`); `TypeInfo::name()` the upstream type name; plus a dotted namespace per type. A crate-level `XmlnsDefinition` list mirroring upstream `AssemblyInfo.cs`.
3. **Cast table for boxed values**: for each class, conversions from `Rc<Self>` to every ancestor handle form used as a property type (`Rc<Base>`, `Rc<dyn BaseClass>`), to each implemented interface trait object, and `T` to `Option<T>`. A single agreed boxing convention for object references (what exactly a `BoxedValue` of a control holds) and a `BoxedValue` to `&'static TypeInfo` / `TypeId` query.
4. **Untyped property API that covers everything the compiler emits**: `set_value(prop, boxed, BindingPriority)`, `get_value`, `bind(prop, binding, priority)`, `clear_value`, `UnsetValue` handling, attached-property get/set on any object, property lookup by owner type + name including attached and `AddOwner`-ed properties, and `property_type` as metadata (not only `TypeId`).
5. **String parsing with upstream semantics** on every value type and enum: `Parse`-equivalent that matches .NET invariant-culture behaviour (separators, whitespace, case-insensitive enums and booleans, flags enums, `Auto`/`*` grid lengths, named colors and `#aarrggbb`, etc.), exposed as `FromStr` plus a `parse` thunk in metadata. Enums need name/value tables. Brushes, `Geometry` (path mini-language), `FontFamily`, `Cursor`, `TextDecorations`, `Transform` operations, `KeyGesture`, `Easing`, `IterationCount`, `RelativePoint/Rect`, `BoxShadows`, `Classes`, `Selector`-typed and `FerroProperty`-typed properties included.
6. **Collection semantics**: `Add` (per accepted item type) on all collection types reachable from XAML (`Controls`, `Children`, `Classes`, `Styles`, `Setters`, `Transitions`, `GradientStops`, `RowDefinitions`, `Items`...), dictionary `Add(key, value)`, `IAddChild<T>`; `ResourceDictionary::add_deferred`, `add_not_shared_deferred`, `ensure_capacity`, `count`.
7. **Initialization protocol**: `ISupportInitialize` equivalent (`begin_init`/`end_init`) on `StyledElement`, honoured by styling/binding activation.
8. **Name scopes**: `INameScope`, `NameScope::set_name_scope`, `register`, `complete`, find-by-name with deferred completion (templates).
9. **Routed events as statics with typed handlers**: `Button::click_event()`, `add_handler(event, closure, routes, handled_too)`, plain events with `add_*` functions, all listed in metadata with their args type.
10. **Styling API as builder functions**: `Selectors::{of_type, is, class, name, property_equals, child, descendant, template, not, nth_child, nth_last_child, or, nesting}`, `Style`, `Setter { property, value }` accepting boxed values, bindings and templates, `ControlTheme`, `Styles`, container queries, `IStyle`, `SetterBase`, `IThemeVariantProvider`, `ThemeVariant` statics.
11. **Templates in Controls**: `IControlTemplate`, `IDataTemplate`, `ITemplate<T>`, `TemplateResult<T>` with name scope, `IDeferredContent`, `TemplatedParent`, `BindingPriority::Template`, `TemplatePart` metadata.
12. **Resources**: `IResourceNode/Host/Provider`, `try_find_resource` with theme variant, resource-changed notifications (for `DynamicResource`), `ThemeDictionaries`, `MergedDictionaries`.
13. **Binding infrastructure**: `BindingBase`, `CompiledBinding`, `CompiledBindingPath` + `CompiledBindingPathBuilder`, `ClrPropertyInfo` over closures, `IPropertyAccessor` plugins, `RelativeSource`, `MultiBinding`, `TemplateBinding`, converters (`IValueConverter`, `IMultiValueConverter`), `StringFormat` (.NET composite format compatibility is a sizeable sub-port), `FallbackValue`/`TargetNullValue`, data validation.
14. **Change notification traits for non-`FerroObject` data**: `NotifyPropertyChanged`, `NotifyCollectionChanged`, `ICommand` (with `can_execute_changed`), plus derive support so view models publish metadata.
15. **Asset loader with `avares://`** resolution keyed by assembly name, accepting per-crate embedded indexes; `Uri` type with base-URI resolution; `Bitmap`/`WindowIcon` creation from asset URIs.
16. **Service-provider plumbing types** in Base: a `TypeId`-keyed `ServiceProvider` trait object; `Application` as resource/style root for the parent stack.
17. **Runtime type queries**: `FerroObject` to `TypeInfo` (exists), `TypeInfo` to `XamlTypeMetadata`, global lookup by namespace + name, and `TypeInfo` usable as a resource/`DataTemplate` key (`x:Type`).
18. **Design-mode surface**: `Design` attached properties and an `is_design_mode` flag.

## 8. Risks and open questions

Full-compatibility blockers found upstream (cannot be made identical, only bounded):

- **Arbitrary CLR types in XAML** (`sys:`, `scg:`, any `clr-namespace` into the BCL or third-party .NET libraries). Only a closed mapping table can work.
- **Code-behind and view models are C#**. `.xaml` files port unchanged; `.xaml.cs` and every type they reference must be rewritten and declared with our macros. "Existing files keep working" holds for markup only.
- **Reflection over undeclared types**: reflection bindings, `x:Static`, constructors and `Parse` discovery on types without metadata fail where .NET would succeed. `dynamic`/`ExpandoObject`, anonymous types and data-annotation validation have no equivalent.
- **Open generics at run time**: the runtime loader cannot instantiate a generic type that was not monomorphised into the binary.
- **Overloads and implicit conversions**: constructor/`Add`/setter overload resolution and `op_Implicit` become explicit metadata entries; behaviour matches only for declared combinations.
- **.NET text semantics**: invariant-culture parsing, `StringFormat` composite format strings, `CultureInfo`, `TimeSpan`/`DateTime` formats. Needs compatibility code, never exact for every corner.
- **Runtime loader cannot use non-public members or types absent from the binary**; no dynamic assembly loading, so the upstream previewer architecture (separate host loading the user assembly) does not transfer.
- **XML reader differences** versus `System.Xml` (entities, DTD, whitespace reporting, position reporting on attributes) affect diagnostics and edge-case whitespace handling.
- **Member name mapping** between PascalCase markup and snake_case Rust for handlers and plain properties is a convention, not derivable for odd names (acronyms); needs an override attribute.

Decisions needed from the owner:

1. Is a data file containing upstream URIs/prefixes (`xmlns-compat.toml`, embedded with `include_str!`) acceptable under the "no upstream name in code" rule? Should compat mapping be on by default or a Cargo feature?
2. FerroUI's own default xmlns URI and the dotted namespace root (`FerroUI.*` assumed).
3. Metadata transport: source scan + `.xamlmeta` through Cargo `links` (recommended) versus linking dependency crates into the build script (simpler to reason about, doubles framework compile time, no answer for local types).
4. Is the scanner limitation acceptable (XAML-visible user types must use our declaration forms), with the two-crate split as escape hatch?
5. Registration: explicit `xaml_metadata()` chain (recommended) versus `inventory`/`linkme` auto-registration.
6. Runtime metadata behind a `xaml-runtime` feature (smaller binaries, but reflection bindings and the runtime loader need it on) versus always on.
7. Code-behind shape: embedded `<Class>Xaml` struct without proc macros (recommended for MVP) versus an attribute proc macro that injects fields into the user's struct.
8. Compile-time value parsers: reuse `FerroUI.Base` as a build dependency of `ferroui-build` (Base must then stay free of native/link dependencies) or split value types into a small crate.
9. Default for `x:CompileBindings` in FerroUI projects (recommend `true`; upstream runtime loader defaults to `false`).
10. Scope of BCL emulation: which `System.*` types go in the table, and whether `StringFormat` must be .NET-compatible in the first release.
11. XML reader choice (`roxmltree`: simple, positions, whole-document; `quick-xml`: streaming, closer to `XmlReader`) and whether third-party crates are acceptable in `external/XamlX`.
12. `resm:` URIs and `x:Precompile="False"` (documents loaded only at run time): support or reject.
13. Diagnostic code prefix and whether upstream codes should be preserved numerically for searchability.

## Owner decisions (2026-10-04)

- **No upstream xmlns compatibility.** FerroUI XAML uses FerroUI's own default xmlns URI and `using:`/`clr-namespace:FerroUI.*` forms only. The `xmlns-compat.toml` mapping described above is dropped; open question 1 is closed. Existing upstream `.xaml` files need their namespace declarations changed when brought over, nothing else.
- **No simplified stand-ins for critical subsystems.** Both back ends (Rust source emitter and runtime interpreter) are to be ported/implemented in full, as designed above.

## Decisions taken while implementing the back ends (2026-10-05)

Recorded by the XAML back-end work; each follows "exact upstream behaviour, idiomatic safe Rust" where this document left a choice open.

1. **Default xmlns URI**: `https://github.com/ferroui` (`ferroui_base::metadata::FERRO_XML_NAMESPACE`). The `x:` namespace and `mc:`/`d:` namespaces are the standard ones, unchanged.
2. **Metadata lives in `ferroui-base`, not in a separate `ferroui-xaml-metadata` crate** (changes sections 2.2, 2.3 and 5). `ferroui_base::metadata` holds `MarkupType` (constant data: handles, base, interfaces, generic info, content property, parse, constructors, plain properties, methods, static fields, events, enum members, attributes), `MarkupAssembly` (assembly name + `XmlnsDefinition`s), `IServiceProvider` and `IAddChild<T>`. Reason: invokers need `BoxedValue` and the `ValueTypes` casts, which are base types; a separate crate would only hold the serialised (`.xamlmeta`) model, which the build tool can define itself when it is written.
3. **Declaration forms** (changes 2.3): `ferro_class_info!(X { new, interfaces, markup: { .. } })` for classes, `ferro_markup_type!` and `ferro_markup_enum!` for everything else; no derive/proc macros. The grammar is rigid on purpose so that the source scanner of section 2.5 can read it. Registered properties are not repeated in metadata; the type system adapter projects them from `TypeInfo::properties()` as CLR-style properties plus `<Name>Property` fields.
4. **Attributes are generic data** (`MarkupAttribute { name, arguments, properties }`), not typed fields, so they map one to one onto `IXamlCustomAttribute` and the upstream transformers run unchanged. Attribute names are the upstream class names without `Attribute`; the adapter resolves them to `FerroUI.Metadata.<Name>Attribute`.
5. **No `xaml-runtime` feature gate yet** (open question 6): metadata is always present but is constant data referenced only from the type's `TypeInfo`/`MarkupTyped` constant; invoker thunks of types that are never registered are removed by dead-code elimination. A feature gate can be added around the `markup` part later without changing declarations.
6. **Registration is explicit** (open question 5 closed as recommended): `register_types()` per crate, chained to the crates below it.
7. **Untyped values**: `MarkupValue = Option<BoxedValue>` in the canonical untyped form of `ValueTypes` (null is `None`; a nullable is its contents; a class instance is its handle `Ref<T>`). Invokers perform assignability casts only; conversions are the transformers' job, as upstream.
8. **Service provider**: `IServiceProvider::get_service(TypeId) -> Option<Rc<dyn Any>>` keyed by the Rust type of the service handle (`Rc<dyn IProvideValueTarget>`), with the typed helper `get_service_of::<T>()`.
9. **Runtime type system**: the loader builds its `IXamlTypeSystem` by projecting the registered `TypeInfo`/`MarkupType`/`MarkupAssembly` tables onto the in-memory type system of `xamlx` (the former test-only `xamlx::testing` model), attaching the invoker to each projected member. `System.*` types are a closed table inside the loader (section 3.1).
10. **Loader layering**: `ferroui-markup-xaml-loader` never depends on `ferroui-controls`. Compile-time intrinsics that need a controls parser go through `IXamlCompileTimeValueParser` (configuration extra `XamlCompileTimeValueParsers`); the run-time implementation (`RuntimeCompileTimeValueParser`) resolves `MarkupType::parse` of the target type. The interpreter and runtime type system live in the loader crate (`runtime/`, feature `runtime`), not in `xamlx`, because their values are base's `MarkupValue`.
11. **Text conversion is `parse:`**, also where upstream has a parse-only `[TypeConverter]`; converter attributes are declared only for real converter classes (`ferroui-markup-xaml`).
12. **More metadata**: attributes of registered properties (`property_attributes`), static types linked to their `TypeInfo` (`type_info:`), renamed enum members, `enum_from_value` (numeric and flag-combination text; a number that is not a member or a combination of flags is an error, where .NET would produce an unnamed enum value), `FerroProperty::host_type()` for attached accessors.
13. **Change notification of view models** (non-object-model types): the existing base mechanism is reused. A view model implements `ferroui_base::data::model::INotifyPropertyChanged` and states it in metadata with `notify_property_changed: T`; bindings built from metadata at run time (reflection bindings, compiled bindings of the runtime loader) obtain the notifier through `MarkupType::notify_property_changed`, compiled bindings emitted as Rust use the typed `CompiledBindingPathBuilder::notifying_*` members directly. A view model without it is read once, as a .NET object without `INotifyPropertyChanged` is.
14. **Plain events**: `MarkupEvent::arguments` are the upstream handler parameters; the adapter passes the instance as sender where the Rust event has none.
15. **Upstream quirks are preserved** (exact-port rule) and pinned by tests; they are listed in the stage reports for raising upstream (container-query `and` handling, `x:Shared` value ignored, instance-attached setter arity, group transformers visiting root children twice, binding-path quirks).
16. **`System.Type` values (one convention for runtime library and loader).** A member that takes or returns a type is declared in metadata with the Rust type it stores, and only two representations exist: `&'static TypeInfo` when only classes of the object model make sense (`ControlTheme.TargetType`, `Style` selectors, `TemplatePart` types, ancestor types of relative sources) and `ferroui_base::data::core::ValueType` when any type may be named (`DataTemplate.DataType`, `x:DataType`, data types of bindings). The core table of the runtime type system maps both (and their `Option<_>` forms) to `System.Type`. The loader evaluates `x:Type` (and text converted to a type) to its own `RuntimeTypeValue` and converts it at the call site to the representation the parameter declares; naming a type that is not a class where `&'static TypeInfo` is declared is a load error that names the type and the member. Where the target is untyped (`object`, e.g. a resource key or `x:Key="{x:Type ..}"`), the value is boxed as `&'static TypeInfo` for a class and as `ValueType` otherwise, so type keys compare equal however they were produced (markup, code, emitted Rust).
17. **Bindings as values (`[AssignBinding]`, setters, markup extension results).** A binding is boxed as the handle of its concrete class (`Rc<CompiledBinding>`, `Rc<ReflectionBinding>`, `Rc<Binding>`, ...; amended 2026-10-05: the handle form, not the object form, because the binding classes are not registered as `ValueTypes` reference types); a member typed `Rc<dyn BindingBase>` / `Option<Rc<dyn BindingBase>>` receives it through the registered interface cast. A markup extension that produces a binding returns the concrete binding object from `ProvideValue`; the compiler's binding-aware setters (`BindingSetter`, `SetterValueProperty`) and `[AssignBinding]` properties therefore see the same value whichever back end produced it.
18. **Fallible members**: `try fn` / `try (..) =>` / `try_get:` / `try_set:` declare members whose Rust callable returns `Result`; `Err` is `MarkupInvokeError::Failed`, the counterpart of an exception thrown by the member. Metadata invocation never panics on user input.
19. **Per-thread value knowledge**: a crate registers the `ValueTypes` knowledge of its metadata types with `ValueTypes::register_global(fn)` from its `register_types()`; every thread applies it on its next use of the table, so no per-thread call is needed.
20. **`PropertyParser`** has one copy, in `ferroui-markup` (`markup::parsers`), used by the compiler extensions and the runtime library.
21. **`CreateSourceInfo` default** comes from `MarkupAssembly::metadata` (key `MarkupAssembly::CREATE_SOURCE_INFO`), the counterpart of the upstream assembly metadata attribute.
22. **Run-time include fallback** (ruling relayed by the coordinator, 2026-10-05; run-time loader only). Upstream every assembly with markup is compiled, so `XamlIncludeGroupTransformer` reports an include whose document is neither in the current group nor listed by the compiled-resources type of its assembly. In the port no assembly has compiled markup until the emitter exists, so the configuration extra `XamlRuntimeIncludeFallback` (`compiler_extensions::group_transformers`, a predicate "the document with this absolute URI can be loaded at run time") lets such an include stay a run-time object: a `StyleInclude` / `ResourceInclude` keeps its `Source` and loads on first use of `Loaded` (`FerroXamlLoader::load` -> compiled table -> `IRuntimeXamlLoader` + asset loader); a `MergeResourceInclude` outside of the group is left in `MergedDictionaries` (the class is a `ResourceInclude`), while the ones inside the group are still inlined. The fallback replaces only the two upstream errors "assembly not found" and "no compiled-resources type"; a document missing from an assembly that has compiled markup stays upstream's error. `FerroXamlIlRuntimeCompiler` installs the predicate (asset exists and an `IRuntimeXamlLoader` is registered); when it is installed and the asset does not exist, the error is upstream's "Unable to resolve XAML resource \"<uri>\" in the \"<assembly>\" assembly ..." (it names the document). Without the extra nothing changes: the emitter (`FerroXamlIlRuntimeCompiler::transform_documents`) removes the predicate, so the build-time compiler links an include of another crate through that crate's `.xamlmeta` (9.7.3) or reports upstream's error, whatever the generating process can load (2026-10-08, step 2 of E5). Messages carry the URI in canonical form (lower-case host), as upstream's do.
23. **`x:Class` at run time** (same ruling). `FerroXamlLoader::load_object(instance)` delegates to `IRuntimeXamlLoader::load_object` when no compiled populate exists. The run-time loader finds the markup of a class through a registry: `FerroRuntimeXamlLoader::register_class_document(class: &'static TypeInfo, uri: &str)`, called from the crate's `register_types()` (the document is an asset of the crate). `load_object` takes the document of the class of the instance or of its nearest base class that has one, opens it through the asset loader (the registered service, else the registered assets directly) and loads it with the instance as root instance, the URI as base URI and the asset's assembly as local assembly. A class with a registered document is the counterpart of a class with compiled markup for the populate override too: when the run-time loader loads a document whose root type is such a class (and no root instance is given), it creates the instance with the constructor of the class and gives the constructor's `load_object` call the populate of the document being loaded instead of the registered one (upstream `!XamlIlPopulateOverride`). The emitter replaces the registry with generated populate methods; the call in the constructor stays the same.

24. **Stage 2b metadata additions**: array-valued attribute arguments (`MarkupAttributeValue::Array`, projected as `XamlValue::Array`); names and attributes of constructor parameters (`MarkupConstructor::parameter_info`; the inference from `[ConstructorArgument]` properties remains only as a marked fallback for constructors in the positional form); static properties apart from static fields (`MarkupType::static_properties`; a type that declares none keeps the marked fallback that shows its `fields:` as properties too); the typed path hook of plain properties (`MarkupProperty::typed_path_element`, used by the run-time evaluator of compiled binding paths exactly where upstream enables typed emission); booleans read into untyped values use the cached boxes (`utilities::BooleanBoxes`).
25. **Fallible initialization**: `EndInit` of markup is `ISupportInitialize::try_end_init`; a duplicate setter of a style or control theme is `InitializationError::DuplicateSetter` out of `StyledElement::try_end_init` / `try_apply_styling` and surfaces as the error of the load. The panicking forms are built on the fallible ones.
26. **Composite format and cultures**: `composite_format::format_with` / `format_arg_with` / `format_values_with` take a `CultureInfo`; numbers use its `NumberFormatInfo`, `Decimal`, `DateTime`, `DateTimeOffset` and `TimeSpan` arguments take their format strings. `format_values` (the string format of bindings) uses the current culture until the converter culture of bindings is ported; `format` / `format_arg` stay invariant.
27. **Text to typed values**: `ValueTypes::try_convert` ends with the `parse:` of the markup metadata of the target type (also for classes of the object model, walking base classes), so every conversion of a binding value consults it as upstream consults the type converter; the reverse direction is the registered display form.

## 9. Rust emitter back end: design

Status: design note for the implementation of the build-time back end (phase 2 of section 7.1 and everything that follows it on the build path). Nothing in this section is implemented yet. It is written against the state of the working copy on 2026-10-05: the loader crate with the complete transformer pipeline, the interpreter back end (`runtime/interpreter`, `runtime/framework`) and the run-time type system exist; `ferroui-build`, the source scanner, `include_xaml!` and a context type for compiled code do not. Where this section differs from sections 1 to 8 it follows the owner decisions and the numbered decisions above (cited as "decision N").

API names are marked as follows: plain code font means the name was found in the working copy; **(new)** means the emitter needs it and it does not exist; *(unverified)* means it was not looked up. All **(new)** items are collected in 9.9.

### 9.1 Scope and principle

1. **Same input as the interpreter.** The emitter consumes the output of `FerroXamlIlCompiler::transform` / `transform_group` and `FerroXamlIlCompiler::get_transformed_root` (root value node, root manipulation node, root type). It adds no transformer, removes none and never looks at XAML text. The pipeline order, the group transformers and the diagnostics are the ones of `compiler_extensions/ferro_xaml_il_compiler.rs`.
2. **One `emit` per `eval`.** Every node, property setter and pseudo method the interpreter evaluates gets exactly one emit function that writes Rust with the same observable effect, in the same order (argument evaluation order, the moment a collection getter is called, the moment the provide-value target property is set and cleared). The emitter is structured like the interpreter: `StandardNodeEmitter` (mirror of `StandardNodeEvaluator`), `emit_intrinsic` (mirror of `evaluate_intrinsic`), `FrameworkNodeEmitter`, `FrameworkSetterEmitter`, `FrameworkMethodEmitter`, binding-path emit. A table `FRAMEWORK_EMITTERS` with the same keys as `runtime::framework::FRAMEWORK_EVALUATORS` is checked by a test against the table in the module doc of `compiler_extensions/mod.rs`, as the interpreter's table already is; a row without both an evaluator and an emitter fails the loader's tests.
3. **No language decisions in the back end.** Which setter is chosen, which conversion applies, whether a parent stack push is needed (`needs_parent_stack`), whether a document has a build method: all of these are computed from the AST and the type system by the same code the interpreter uses (`assignment_plan`, `EvalContext::convert`, the `ParentStackVisitor`). Phase E0 (9.10) moves that code to a place both back ends call; the emitter then turns each decision into source text and the interpreter into an action.
4. **rustc is the second type checker, not the first.** Everything the transformers can type is typed by them and reported with XAML positions. rustc only finishes what the build tool cannot see (9.5.4); such places carry a `// file(line,col)` marker.
5. **Conformance.** Every test document is loaded through both back ends and a canonical dump of the resulting object tree is compared (9.10.2). The dump format is the contract; a difference is a bug in one of the two back ends, never an accepted deviation.

Where the generated code lives relative to the emit layer of `xamlx`: `external/XamlX/src/XamlX/emit/` (`XamlEmitContextBase<TBackendEmitter, TEmitResult>`, `IXamlAstNodeEmitter`, `IXamlPropertySetterEmitter`, `IXamlWrappedMethodEmitter`, `XamlLanguageEmitMappings`, `XamlRuntimeContext`) is generic over the back end and is instantiated with `TBackendEmitter = RustWriter` **(new)** and `TEmitResult = RustEmitResult` **(new)** (an `IXamlEmitResult` that carries the static `IXamlType` and the Rust expression text or the name of the `let` holding it). `XamlEmitContextWithLocalsBase` is not used: Rust locals are named `let`s (section 6.1, `XamlLocalsPool` dropped).

### 9.2 What the generated Rust looks like

#### 9.2.1 The document

`src/views/MainWindow.xaml` of the crate `my-app` (assembly name `MyApp`):

```xml
<Window xmlns="https://github.com/ferroui"
        xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"
        xmlns:vm="using:MyApp.ViewModels"
        x:Class="MyApp.Views.MainWindow"
        x:DataType="vm:MainViewModel"
        Title="Demo">
  <Window.Resources>
    <SolidColorBrush x:Key="Accent" Color="#FF0078D7"/>
  </Window.Resources>
  <Window.Styles>
    <Style Selector="TextBlock.title">
      <Setter Property="FontSize" Value="20"/>
    </Style>
  </Window.Styles>
  <Window.DataTemplates>
    <DataTemplate DataType="vm:Person">
      <TextBlock Text="{Binding FullName}"/>
    </DataTemplate>
  </Window.DataTemplates>
  <StackPanel Margin="8,4">
    <TextBlock x:Name="Greeting" Classes="title" DockPanel.Dock="Top"
               Text="{Binding Name}" Foreground="{StaticResource Accent}"/>
    <Button Content="Click" Click="OnClick"/>
  </StackPanel>
</Window>
```

It contains every form the task of this section names: `x:Class`, a named element, compiled bindings under `x:DataType` (one in the document body, one inside a template with its own data type), a routed event handler, a style with a selector and a setter, a static resource (whose target is itself a deferred resource), a data template (deferred content), an attached property and a parsed `Thickness`.

#### 9.2.2 The user's side

```rust
// src/views/main_window.rs  (module my_app::views::main_window)
#[repr(C)]
pub struct MainWindow {
    base: Window,
    xaml: MainWindowXaml,                 // generated parts struct, 9.4
    clicks: Cell<u32>,
}

ferro_class!(MainWindow: Window);
ferro_impl_classes!(MainWindow: FerroObjectImpl, /* every level below */);
ferro_class_info!(MainWindow {
    new: MainWindow::new,
    markup: { namespace: "MyApp.Views" },
});

include_xaml!("views/MainWindow.xaml");  // (new) expands to include!(concat!(env!("OUT_DIR"), "/xaml/views/MainWindow.xaml.rs"))

impl FerroObjectImpl for MainWindow {
    fn constructed(this: &Self) {         // the body of the C# constructor
        Self::parent_constructed(this);
        this.initialize_component();
    }
}

impl MainWindow {
    pub fn construct() -> Self {
        Self { base: Window::construct(), xaml: MainWindowXaml::default(), clicks: Cell::new(0) }
    }
    pub fn new() -> Ref<Self> { instantiate(Self::construct()) }

    fn on_click(&self, _sender: &Interactive, _e: &RoutedEventArgs) {
        self.clicks.set(self.clicks.get() + 1);
        self.xaml.greeting().set_text(Some("clicked"));
    }
}
```

#### 9.2.3 The generated file

`$OUT_DIR/xaml/views/MainWindow.xaml.rs`, complete. Conventions used in it are explained in 9.3. The `use` list is what the emitter collects while writing; module paths of framework types come from metadata (`rust_path`, 9.5.2).

```rust
// @generated by ferroui-build from src/views/MainWindow.xaml. Do not edit.
// Included by include_xaml!("views/MainWindow.xaml") in module my_app::views::main_window.

pub(crate) use self::__main_window_xaml::MainWindowXaml;

#[allow(unused_imports, unused_variables, unused_mut, non_snake_case, clippy::all)]
mod __main_window_xaml {
    use super::MainWindow; // a child module sees the private items of the class's module

    use ::std::cell::RefCell;
    use ::std::rc::Rc;

    use ::ferroui_base::controls::{NameScope, NameScopeExtensions, NameScopeRef, ResourceDictionary};
    use ::ferroui_base::data::core::{Value, ValueType};
    use ::ferroui_base::data::{BindingBase, BindingPriority, CompiledBindingPathBuilder};
    use ::ferroui_base::interactivity::{Interactive, RoutedEventArgs, RoutingStrategies};
    use ::ferroui_base::layout::Layoutable;
    use ::ferroui_base::media::{Color, IBrush, SolidColorBrush};
    use ::ferroui_base::metadata::{from_markup_value, into_markup_value, IServiceProvider, MarkupTyped, MarkupValue};
    use ::ferroui_base::styling::{Selectors, Setter, Style};
    use ::ferroui_base::utilities::{Uri, UriKind};
    use ::ferroui_base::{BoxedValue, Ref, StyledElement, Thickness, UnsetValueType, WeakRef};
    use ::ferroui_controls::templates::IDataTemplate;
    use ::ferroui_controls::{Button, ContentControl, Control, Dock, DockPanel, StackPanel, TextBlock, Window};
    use ::ferroui_markup_xaml::markup_extensions::{CompiledBindingExtension, StaticResourceExtension};
    use ::ferroui_markup_xaml::templates::DataTemplate;
    use ::ferroui_markup_xaml::xaml_il::runtime::compiled as rt;                   // (new) 9.9 R2..R5
    use ::ferroui_markup_xaml::xaml_il::runtime::{DeferredContentBuilder, XamlIlContext /* (new) R1 */, XamlIlRuntimeHelpers};
    use ::ferroui_markup_xaml::{FerroXamlLoader, XamlLoadException};

    use ::my_app::view_models::{MainViewModel, Person};

    // ---- document constants -------------------------------------------------------------

    const SOURCE: &str = "views/MainWindow.xaml";
    const BASE_URI: &str = "avares://MyApp/views/MainWindow.xaml";

    /// The static providers of the document (upstream: the `NamespaceInfo:<name>` type).
    static DOCUMENT: rt::DocumentInfo = rt::DocumentInfo {
        base_uri: BASE_URI,
        xml_namespaces: &[
            ("", &[("FerroUI", "FerroUI.Base"), ("FerroUI.Controls", "FerroUI.Controls"), /* one pair per XmlnsDefinition */]),
            ("x", &[]),
            ("vm", &[("MyApp.ViewModels", "MyApp")]),
        ],
    };

    // ---- x:Class parts (upstream: the name generator's fields) ------------------------------

    #[derive(Default)]
    pub struct MainWindowXaml {
        greeting: RefCell<Option<Ref<TextBlock>>>,
    }

    impl MainWindowXaml {
        /// `x:Name="Greeting"`, MainWindow.xaml(21,16).
        #[track_caller]
        pub(crate) fn greeting(&self) -> Ref<TextBlock> {
            self.greeting.borrow().clone().expect("Greeting: initialize_component() has not run")
        }
    }

    // ---- entry points ---------------------------------------------------------------------

    impl MainWindow {
        /// Upstream `InitializeComponent(loadXaml: true)` + `!XamlIlPopulateTrampoline`.
        pub(crate) fn initialize_component(&self) {
            let this: Ref<MainWindow> = self.to_ref();
            match FerroXamlLoader::populate_override(MainWindow::TYPE) {                  // (new) R6
                Some(populate) => populate(&into_markup_value(this.clone())),
                None => {
                    let sp = XamlIlRuntimeHelpers::create_root_service_provider_v3(None);
                    rt::throw(Self::xaml_populate(&sp, &this));
                }
            }
            let scope = NameScopeExtensions::find_name_scope(&this);
            *self.xaml.greeting.borrow_mut() =
                scope.as_ref().and_then(|s| s.find("Greeting")).and_then(|o| o.cast::<TextBlock>());
        }

        /// Upstream `!XamlIlPopulate(IServiceProvider, MainWindow)`.
        pub(crate) fn xaml_populate(
            sp: &Rc<dyn IServiceProvider>,
            target: &Ref<MainWindow>,
        ) -> Result<(), XamlLoadException> {
            let ctx = XamlIlContext::new(Some(sp.clone()), &DOCUMENT);
            ctx.set_root_object(into_markup_value(target.clone()));
            ctx.set_intermediate_root_object(into_markup_value(target.clone()));
            let ctx_sp: Rc<dyn IServiceProvider> = ctx.clone();

            // MainWindow.xaml(1,2) Window
            target.begin_init();
            ctx.push_parent(into_markup_value(target.clone()));

            // MainWindow.xaml(6,9) Title
            target.set_value(Window::title_property(), Some(String::from("Demo")));

            // MainWindow.xaml(7,4) Window.Resources
            {
                let resources: Ref<ResourceDictionary> = target.resources();
                // EnsureCapacityNode(1)
                resources.ensure_capacity(resources.count() + 1);
                // MainWindow.xaml(8,6) SolidColorBrush x:Key="Accent"  (ResourceAdderSetter: AddDeferred)
                let key = "Accent";
                let value = XamlIlRuntimeHelpers::deferred_transformation_factory_v3::<Option<BoxedValue>>(
                    DeferredContentBuilder::try_new(deferred_0),
                    &ctx_sp,
                );
                resources.add_deferred(key, value.as_deferred_content());
            }

            // MainWindow.xaml(10,4) Window.Styles
            {
                let styles = target.styles();
                // MainWindow.xaml(11,6) Style
                let style_0: Ref<Style> = Style::new();
                // MainWindow.xaml(11,13) Selector
                style_0.set_selector(Some(Selectors::class(Some(Selectors::of_type_info(None, TextBlock::TYPE)), "title")));
                // MainWindow.xaml(12,8) Setter
                let setter_0: Rc<Setter> = Setter::empty();
                // MainWindow.xaml(12,16) Property
                setter_0.set_property(Some(TextBlock::font_size_property().as_property()));
                // MainWindow.xaml(12,36) Value  (XamlIlDirectCallPropertySetter; untyped member, 9.5.3 form C)
                rt::set_property(<Setter as MarkupTyped>::MARKUP, "Value", &setter_0, into_markup_value(20.0_f64))
                    .map_err(|e| rt::at(e, SOURCE, 12, 36))?;
                style_0.add(setter_0);
                styles.add(style_0);
            }

            // MainWindow.xaml(15,4) Window.DataTemplates
            {
                let templates = target.data_templates();
                // MainWindow.xaml(16,6) DataTemplate
                let template_0: Rc<DataTemplate> = DataTemplate::new();
                ctx.push_parent(into_markup_value(template_0.clone()));
                // MainWindow.xaml(16,20) DataType
                template_0.set_data_type(Some(ValueType::of::<Person>()));
                // MainWindow.xaml(17,8) content (XamlDeferredContentNode)
                let content = XamlIlRuntimeHelpers::deferred_transformation_factory_v3::<Ref<Control>>(
                    DeferredContentBuilder::try_new(deferred_1),
                    &ctx_sp,
                );
                template_0.set_content(into_markup_value(content));
                ctx.pop_parent();
                templates.add(template_0.as_data_template());
            }

            // MainWindow.xaml(20,4) StackPanel  (top-down initialisation: BeginInit, attach, populate, EndInit)
            let stack_panel_0: Ref<StackPanel> = StackPanel::new();
            stack_panel_0.begin_init();
            target.set_value(ContentControl::content_property(), into_markup_value(stack_panel_0.clone()));
            ctx.push_parent(into_markup_value(stack_panel_0.clone()));
            // MainWindow.xaml(20,16) Margin  (compile-time constant)
            stack_panel_0.set_value(Layoutable::margin_property(), Thickness::new(8.0, 4.0, 8.0, 4.0));

            // MainWindow.xaml(21,6) TextBlock
            let text_block_0: Ref<TextBlock> = TextBlock::new();
            text_block_0.begin_init();
            stack_panel_0.children().add(text_block_0.clone());
            ctx.push_parent(into_markup_value(text_block_0.clone()));
            // MainWindow.xaml(21,16) x:Name  (Name assignment + FerroNameScopeRegistrationXamlIlNode)
            // (`Name` is a direct property: its declared accessor is called)
            text_block_0.set_name(Some(String::from("Greeting")));
            rt::register_name(ctx.name_scope(), "Greeting", &text_block_0).map_err(|e| rt::at(e, SOURCE, 21, 16))?;
            // MainWindow.xaml(21,34) Classes
            text_block_0.classes().add("title");
            // MainWindow.xaml(21,50) DockPanel.Dock  (attached property, enum constant)
            text_block_0.set_value(DockPanel::dock_property(), Dock::Top);
            // MainWindow.xaml(22,16) Text="{Binding Name}"  (compiled binding; BindingSetter)
            {
                let path = CompiledBindingPathBuilder::new()
                    .notifying_property::<MainViewModel, Value<String>>(
                        "Name",
                        |o| o.name(),
                        |o, v| o.set_name(v),
                    )
                    .build();
                let extension: Rc<CompiledBindingExtension> = CompiledBindingExtension::with_path(path);
                ctx.set_target_property(into_markup_value(TextBlock::text_property().as_property()));
                let binding = extension.provide_value(Some(&ctx_sp));
                ctx.set_target_property(None);
                text_block_0.bind_binding(TextBlock::text_property().as_property(), &binding);
            }
            // MainWindow.xaml(22,38) Foreground="{StaticResource Accent}"  (untyped result; dynamic setter, 3 candidates)
            {
                let extension: Rc<StaticResourceExtension> =
                    StaticResourceExtension::with_resource_key(into_markup_value(String::from("Accent")));
                ctx.set_target_property(into_markup_value(TextBlock::foreground_property().as_property()));
                let value: MarkupValue = extension.provide_value(&ctx_sp).map_err(|e| rt::at(e, SOURCE, 22, 38))?;
                ctx.set_target_property(None);
                if let Some(v) = rt::is_instance::<Rc<dyn BindingBase>>(&value) {
                    // BindingSetter
                    text_block_0.bind_binding(TextBlock::foreground_property().as_property(), &*v);
                } else if let Some(v) = rt::is_instance::<UnsetValueType>(&value) {
                    // UnsetValueSetter
                    text_block_0.set_value_untyped(
                        TextBlock::foreground_property().as_property(), &UnsetValueType, BindingPriority::LocalValue);
                } else if let Some(v) = rt::is_instance::<Rc<dyn IBrush>>(&value) {
                    // registered property setter
                    text_block_0.set_value(TextBlock::foreground_property(), Some(v));
                } else if value.is_none() {
                    // first setter that allows a run-time null
                    text_block_0.set_value(TextBlock::foreground_property(), None);
                } else {
                    return Err(rt::no_setter("Foreground", &value, SOURCE, 22, 38));
                }
            }
            ctx.pop_parent();
            text_block_0.end_init();

            // MainWindow.xaml(23,6) Button
            let button_0: Ref<Button> = Button::new();
            button_0.begin_init();
            stack_panel_0.children().add(button_0.clone());
            // MainWindow.xaml(23,14) Content
            button_0.set_value(ContentControl::content_property(), into_markup_value(String::from("Click")));
            // MainWindow.xaml(23,30) Click="OnClick"  (XamlDirectCallAddHandler + XamlLoadMethodDelegateNode)
            {
                let root: WeakRef<MainWindow> = target.downgrade();
                button_0.add_handler_with(
                    &Button::click_event(),
                    move |sender: &Interactive, e: &RoutedEventArgs| {
                        if let Some(root) = root.upgrade() {
                            root.on_click(sender, e)
                        }
                    },
                    RoutingStrategies::DIRECT.union(RoutingStrategies::BUBBLE),
                    false,
                );
            }
            button_0.end_init();

            ctx.pop_parent();
            stack_panel_0.end_init();

            // HandleRootObjectScopeNode
            NameScope::set_name_scope(target, ctx.name_scope().cloned().map(NameScopeRef));
            rt::complete_name_scope(ctx.name_scope()).map_err(|e| rt::at(e, SOURCE, 1, 2))?;

            ctx.pop_parent();
            target.end_init();
            Ok(())
        }
    }

    // ---- deferred content (upstream: one `XamlClosure_N.Build(IServiceProvider)` each) ---------

    /// MainWindow.xaml(8,6) the resource "Accent".
    fn deferred_0(sp: &Rc<dyn IServiceProvider>) -> Result<Option<BoxedValue>, XamlLoadException> {
        let ctx = XamlIlContext::new(Some(sp.clone()), &DOCUMENT);
        ctx.set_root_object(rt::parent_root_object(sp));
        // MainWindow.xaml(8,6) SolidColorBrush
        let brush_0: Ref<SolidColorBrush> = SolidColorBrush::new();
        ctx.set_intermediate_root_object(into_markup_value(brush_0.clone()));
        // MainWindow.xaml(8,38) Color  (compile-time constant: Color.FromUInt32)
        brush_0.set_value(SolidColorBrush::color_property(), Color::from_uint32(0xFF0078D7));
        Ok(into_markup_value(brush_0))
    }

    /// MainWindow.xaml(17,8) the content of the data template for `Person`.
    fn deferred_1(sp: &Rc<dyn IServiceProvider>) -> Result<Option<BoxedValue>, XamlLoadException> {
        let ctx = XamlIlContext::new(Some(sp.clone()), &DOCUMENT);
        ctx.set_root_object(rt::parent_root_object(sp));
        let ctx_sp: Rc<dyn IServiceProvider> = ctx.clone();
        // MainWindow.xaml(17,8) TextBlock
        let text_block_0: Ref<TextBlock> = TextBlock::new();
        ctx.set_intermediate_root_object(into_markup_value(text_block_0.clone()));
        text_block_0.begin_init();
        ctx.push_parent(into_markup_value(text_block_0.clone()));
        // MainWindow.xaml(17,19) Text="{Binding FullName}"  (read-only property of Person)
        {
            let path = CompiledBindingPathBuilder::new()
                .notifying_read_only_property::<Person, Value<String>>("FullName", |o| o.full_name())
                .build();
            let extension: Rc<CompiledBindingExtension> = CompiledBindingExtension::with_path(path);
            ctx.set_target_property(into_markup_value(TextBlock::text_property().as_property()));
            let binding = extension.provide_value(Some(&ctx_sp));
            ctx.set_target_property(None);
            text_block_0.bind_binding(TextBlock::text_property().as_property(), &binding);
        }
        ctx.pop_parent();
        text_block_0.end_init();
        Ok(into_markup_value(text_block_0))
    }
}
```

Notes on the example (each is a rule of the emitter, not a special case):

- **Verified names used above**: `XamlIlRuntimeHelpers::{create_root_service_provider_v3, deferred_transformation_factory_v3}`, `DeferredContentBuilder::try_new`, `DeferredContent::as_deferred_content`, `ResourceDictionary::{count, ensure_capacity, add_deferred}`, `StyledElement::{resources, styles, classes, begin_init, end_init}`, `Style::{new, set_selector}`, `StyleBase::add`, `Styles::add`, `Setter::{empty, set_property}`, `Selectors::{of_type_info, class}`, `Control::data_templates` (derefs to `FerroList::add`), `Window::title_property`, `Layoutable::margin_property`, `StyledElement::set_name` (`name_property` is a `DirectProperty`), `SolidColorBrush::color_property`, `CompiledBindingPathBuilder::build`, `ferroui_base::UnsetValueType`, `DataTemplate::{new, set_data_type, set_content, as_data_template}`, `Panel::children` + `Controls::add`, `FerroObject::{set_value, set_value_untyped, bind_binding}`, `StyledProperty::as_property`, `Classes::add`, `DockPanel::dock_property`, `Interactive::add_handler_with`, `Button::click_event`, `RoutingStrategies::{DIRECT, BUBBLE}`, `CompiledBindingPathBuilder::{new, notifying_property, notifying_read_only_property}`, `CompiledBindingExtension::{with_path, provide_value}`, `StaticResourceExtension::{with_resource_key, provide_value}`, `Thickness::new`, `Color::from_uint32`, `NameScope::set_name_scope`, `NameScopeExtensions::find_name_scope`, `INameScope::find`, `Ref::{cast, downgrade}`, `WeakRef::upgrade`, `into_markup_value`, `from_markup_value`, `<T as MarkupTyped>::MARKUP`, `ValueType::of`.
- *(unverified)*: the module paths in the `use` list for `Interactive`, `Layoutable`, `IBrush`, `IDataTemplate` and the view-model accessors (`name`, `set_name`, `full_name` are the user's). Property accessors are written with the owner the declaration names (`Layoutable::margin_property()`, not `StackPanel::..`); the emitter takes owner, accessor name and value type from the scanned `ferro_properties!` block, never from the naming rule alone (9.5.1). Note that `FerroProperty::register::<Window, _>("Title", ..)` leaves the value type to inference, so the scanner reads it from the accessor's return type.
- **(new)** names: `XamlIlContext`, `rt::*`, `FerroXamlLoader::populate_override`, `include_xaml!` (9.9).
- The trailing `Thickness::new(8.0, 4.0, 8.0, 4.0)` is `FerroXamlIlVectorLikeConstantAstNode`: the intrinsic parser of the loader (`ferro_xaml_il_language_parse_intrinsics.rs`, which already calls `Thickness::parse` at compile time) produced four doubles; the emitter writes the constructor call, not a parse call.
- `Foreground` shows the dynamic setter chain of `evaluators::property_assignment` unrolled over the setters that survive `assignment_plan`. `Text` shows the single-setter case (`BindingSetter`), `Title` and `Margin` the statically typed case.
- The `Click` closure captures the root **weakly**. Upstream the delegate holds the root strongly, which is harmless under a tracing collector; here root → button → handler → root would be a reference cycle. The interpreter's `load_method_delegate` currently captures `instance.clone()` strongly; it has to change in the same way so that the two back ends agree (open issue O1).

### 9.3 Emission rules

#### 9.3.1 Values

Every AST value node has a static `IXamlType`. The emitter maps it to one Rust type, the *exact form* (decision 7 and the `handles:` lists of metadata):

| Static XAML type | Rust type in generated code | Source of the spelling |
|---|---|---|
| class of the object model | `Ref<path::Type>` | `ferro_class!` + module path |
| nullable class reference (property type) | `Option<Ref<Type>>` | property declaration |
| plain shared class (`ferro_markup_type!(class ..)`) | the `this:` type, normally `Rc<Type>` | metadata |
| interface | `Rc<dyn ITrait>` (first `handles:` entry) | metadata |
| value type, enum | `Type` | metadata |
| `Nullable<T>` | `Option<T>` | generic projection |
| `System.Object` | `MarkupValue` (`Option<BoxedValue>`) | fixed |
| `System.String` | `String` (literals: `String::from("..")`; `&str` where the declared parameter is `&str`) | core table |
| `System.Type` | `&'static TypeInfo` or `ValueType` per decision 16, chosen by the receiving member | member declaration |
| markup delegate | closure of the handler's signature | event declaration |
| unknown to the build tool | no annotation; rustc infers | 9.5.4 |

Conversions between a node's type and the type its consumer expects are the cases of `EvalContext::convert` (`ILEmitHelpers.EmitConvert` upstream), written as source:

| From → to | Emitted |
|---|---|
| equal | nothing |
| null → reference / nullable | `None` |
| null → non-nullable value type | build error (the transformer already reports it) |
| `T` → `Nullable<T>` | `Some(v)` |
| value or reference → `object` | `into_markup_value(v)` |
| `object` → `T` (unbox / checked cast) | `rt::cast::<T>(&v).map_err(\|e\| rt::at(e, SOURCE, l, c))?` **(new)**, built on `from_markup_value::<T>` |
| class → base class | `v.upcast()` (or nothing where the parameter is `impl IntoRef<Base>`) |
| class → interface handle | `v.into()` (porting guide, "A class that implements an interface") |
| base / interface → derived (checked) | `v.cast::<T>().ok_or_else(..)?` for classes, `rt::cast` otherwise |
| `T` → `Option<Ref<T>>` property type | `Some(v)` |

Where upstream's XamlIl emits a run-time cast (`castclass`, an unbox) whose success the static types already prove, generated Rust states the conversion statically, and rustc checks it: a registered property definition passed where its nullable form is declared is `Some(rt::property(..))`; a value passed to a contract that its declaration lists among its interfaces or names as a base is an unsizing coercion (`v as Rc<dyn Contract>`). The result is the same value the run-time cast produces, and the cast could not fail, so no error path is lost. This is a difference in the shape of the code only, not in behaviour. Any conversion the static types do not prove stays `rt::cast` with its position.

The boxed form of an object pushed on the parent stack, stored as root object or passed as `object` is always `into_markup_value(x.clone())`, so that parents found by `ServiceProviderExtensions::get_first_parent` and resources compare exactly as they do under the interpreter.

#### 9.3.2 Statements, locals, blocks

- One `let` per object creation, named `<snake type name>_<n>`, with an explicit type annotation when the type is known. `XamlAstCompilerLocalNode` / `XamlAstLocalInitializationNodeEmitter` map to the same `let`.
- A property whose value needs temporaries is wrapped in `{ .. }` so names do not leak; the block is purely lexical.
- `XamlObjectInitializationNode` writes `begin_init()` (unless `skip_begin_init`), the `push_parent` when `needs_parent_stack` holds for the node and the type is not a value type, the manipulation, `pop_parent`, `end_init()`. The begin/end pair is written only when the static type is assignable to the `ISupportInitialize` mapping; on a statically unknown type it is `rt::begin_init(&x)` **(new)** (trait-directed, 9.5.4).
- `XamlValueNodeWithBeginInit` (top-down initialisation of `UsableDuringInitialization` types) writes creation + `begin_init()` and the consumer writes the assignment before the rest of the initialisation, as in the example.

#### 9.3.3 Property assignment forms

`XamlPropertyAssignmentNode` after `assignment_plan`:

| Setter that takes the value | Emitted |
|---|---|
| registered property, plain value (projected CLR setter of a styled/attached property) | `target.set_value(Owner::name_property(), v)` |
| direct property | `target.set_name(v)` via the declared accessor when one is declared, else `target.set_value_untyped(Owner::name_property().as_property(), &v, BindingPriority::LocalValue)` |
| `SetValueWithPrioritySetter` | `target.set_value_with_priority(Owner::name_property(), v, BindingPriority::Template)` (value evaluated before the priority, as the interpreter) |
| `BindingSetter`, `BindingWithPrioritySetter` | `target.bind_binding(Owner::name_property().as_property(), &*binding)`; the priority node of the second is not emitted at all |
| `UnsetValueSetter` | `target.set_value_untyped(P.as_property(), &UnsetValueType, BindingPriority::LocalValue)`; the argument node is not emitted |
| `FerroAttachedInstancePropertySetterMethod` | `target.set_value_untyped(Field, v, BindingPriority::LocalValue)` after `rt::exact_property_value` |
| plain property (`XamlDirectCallPropertySetter`) | typed call of the declared setter (9.5.3 form B) or `rt::set_property` (form C) |
| collection adder (`AdderSetter`) | `let c = <getter>(&target); c.add(v);` with the getter called before the value is evaluated |
| several setters left | the `if let .. else if ..` chain shown for `Foreground`; candidates in plan order; a candidate whose parameter is statically assignable from the value type is written without a test; `allow_runtime_null` decides the `None` arm; the final `else` is `rt::no_setter` (the interpreter's `InvalidCastException` / `NullReferenceException` texts) |

The typed `set_value` goes through the same property store path as the interpreter's `set_value_untyped` with an exact value; hand-written wrappers such as `set_title` are deliberately not called, because the interpreter cannot call them either and they may add `impl Into<..>` conveniences that change inference.

#### 9.3.4 Markup extensions

`XamlMarkupExtensionNode`, in the interpreter's order: (1) the extension object is created and initialised like any object; (2) when the `ProvideValue` method takes a context and the enclosing node is a property assignment, the provide-value target property is set on the context: the registered property (`into_markup_value(P.as_property())`), a `PropertyInfoValue` for a plain property, or the property name as a string (`XamlIlFerroPropertyHelper::try_get_provide_value_target`); (3) `provide_value` is called through the declared member; (4) the target property is reset to `None`; (5) the result has the static return type of the `ProvideValue` overload the transformer chose. Steps (2) to (4) are one call of `rt::provide_value` (or `rt::provide_value_invoked` for a fallible `ProvideValue`) when the extension is a local and the service provider is the context itself (9.3.7). A typed result (`CompiledBinding`) goes straight to the single setter; an `object` result goes through the dynamic chain. `XamlIlRuntimeHelpers::apply_non_matching_markup_extension_v1` is not referenced by the pipeline or the interpreter and is not emitted.

#### 9.3.5 Errors

- Generated `xaml_populate`, `build` and deferred builders return `Result<_, XamlLoadException>`. Fallible runtime calls carry the position the interpreter attaches through `IXamlLineInfo`: the `rt` helpers that can fail take the line and the position, and the call of a fallible member (a declared setter, method or constructor that returns a `Result`, and `EndInit`) is written `rt::invoked(<call>, line, position)?`, which reports the failure as the exception that wraps the exception of the member (`TargetInvocationException`), as the run-time loader does.
- Calls that panic in the object model (duplicate name registration, validators) panic in generated code too, as they do under the interpreter.
- `initialize_component()` has no error channel (it stands for a C# constructor body); it calls `rt::throw` **(new, the existing `pub(crate) fn throw` of the runtime library made public)**, which panics with the message.

#### 9.3.6 Position markers

Each object creation and each property assignment is preceded by `// <file name>(line,col) <what>`. The markers are the only link from a rustc error inside generated code back to XAML, so the build tool also writes `$OUT_DIR/xaml/<doc>.map.json` (generated line → XAML position) and, when `cargo build` fails after a successful `build.rs`, the same mapping lets an editor plugin or a `cargo ferroui explain` helper translate the error; the mapping file is cheap and is produced from phase E1 on.

#### 9.3.7 Shared helpers of generated code

Statement sequences that recur at thousands of sites are calls of one helper of `rt` (browser-platform.md section 20, plan item 2). Each helper makes the calls the statements made, through the same typed functions and in the same order; a helper is non-generic, or generic only over a type that has a handful of instances (the extension and the result of `ProvideValue`), and `#[inline(never)]`, so the code is in the helper and not at the call site. Where the emitter does not recognise the exact shape, the statements are written as before. These are differences in the shape of the code compared with upstream's XamlIl, not in behaviour, with the two exceptions stated in the list (the evaluation of a setter's value, the lifetime of locals of a split function); neither is observable.

- **Provide-value bookkeeping** (9.3.4): `rt::provide_value(&context, <target property>, &extension, Extension::__markup_ProvideValue_N)` sets the provide-value target property, calls `ProvideValue` with the extension and the context, and clears the target property. `rt::provide_value_invoked(.., line, position)?` is the form for a fallible `ProvideValue`: a failure is the error `rt::invoked` reports, and the target property stays set, as it does when the statements fail. Not used for a markup extension with options, or when the extension is not a local borrowed as it is or the service provider is passed in another form (`CompiledBindingExtension` takes an optional one).
- **Setters of a style** (`<Setter Property=".." Value=".."/>` added to a style or a control theme through `StyleBase.Add`): `rt::add_setter(style, property, value)` when the value has no statements of its own; otherwise `let setter_n = rt::new_setter(property)` before the statements of the value and `rt::add_setter_value(style, &setter_n, value)` after them, with the `_with_parent` forms for a setter on the parent stack (push after the creation, pop after `Value`, before `Add`). The calls are `Setter::__markup_new_0`, `__markup_set_Property`, `__markup_set_Value` with `rt::setter_value`, `StyleBase::__markup_Add_0`. With `rt::add_setter` the value expression is evaluated before the setter is created instead of after `Property` is set: such a value has no statements, does not name the setter and cannot observe the setter or the style (a constant, a constructor of a value). Other setters (another order of the members, a fallible member, `KeyFrame` setters) keep their statements. `ferroui-markup-xaml` enables `ferroui-base/markup-functions` for these helpers.
- **The description of `Setter.Value`** as the provide-value target property is `rt::setter_value_property()`. `rt::clr_property_info` keeps one description per declared property and thread, created on first use, as upstream keeps one per property in a static field of the generated helper type of the assembly (`XamlIlClrPropertyInfoEmitter`, `<Type>.<Name>!Field`). The interpreter creates one per evaluation; nothing compares descriptions by identity.
- **The document of a context**: each document has `static <FUNCTION>_DOCUMENT: rt::DocumentInfo` with its base URI and its XML namespace table; `rt::populate_context` and `rt::deferred_context` take a reference to it.
- **Split functions**: a build or populate function with more than 1,024 top-level statements is split into functions `<function>_part_<n>` of at most 256 statements each, called in place, with the locals of the function that a part uses passed to it as cloned handles, in the order the part first names them (the output does not depend on hashing; the drift tests compare it byte for byte). A part never declares a local a later statement uses; a statement that names a local whose Rust type the emitter does not state (the `let` of an object created with its default constructor, a `let` with a type, `context`, `name_scope`, `root`) stays in the function. The statements run in the same order with the same errors; a local of a part is dropped when the part returns instead of at the end of the function, which reference counting only observes as an earlier decrement of a handle that the object tree still holds (upstream has no deterministic destruction to compare with). Only the populate function of the Fluent theme is that large.

### 9.4 `x:Class` and code-behind without partial classes

#### 9.4.1 What upstream generates

| Upstream artefact | Produced by | Role |
|---|---|---|
| `!XamlIlPopulate(IServiceProvider, T)` on the class | `XamlCompilerTaskExecutor` | the populate method |
| `!XamlIlPopulateTrampoline(T)` / `(IServiceProvider, T)` | same | `if (!XamlIlPopulateOverride != null) override(this); else Populate(CreateRootServiceProviderV3(sp), this)` |
| static field `!XamlIlPopulateOverride` (`Action<object>`) | same | set by the designer/hot reload to redirect population |
| rewrite of `AvaloniaXamlLoader.Load(this)` to the trampoline; injection into a trivial constructor when no call is found; error otherwise | same | wiring |
| `Build:<name>` / `Populate:<name>` on `CompiledAvaloniaXaml.!<group>` | same | classless documents |
| `CompiledAvaloniaXaml.!XamlLoader.TryLoad(IServiceProvider, string)` | same | URI dispatch; for a class with a public parameterless constructor it is `new T()` |
| `InitializeComponent(bool loadXaml = true)` + one field per `x:Name` (`__thisNameScope__?.Find<T>(name)`) | `Avalonia.Generators` name generator (`InitializeComponentCodeGenerator`) | named fields, visibility from `x:FieldModifier` (default `internal`) |

#### 9.4.2 Options

| | A. Parts struct + per-class `include_xaml!` | B. Attribute proc macro on the struct | C. Trait, one crate-level include |
|---|---|---|---|
| Shape | user struct has a field `xaml: <Class>Xaml`; the generated file is included in the class's module and adds `impl <Class> { initialize_component, xaml_populate }` | `#[xaml_class("MainWindow.xaml")] struct MainWindow {..}` injects fields and methods | generated code lives in one crate-level module and implements `XamlComponent for <Class>`; user implements `fn xaml(&self) -> &<Class>Xaml` |
| Private handlers and fields reachable | yes: the generated module is a child of the class's module | yes | no: handlers and the parts accessor must be `pub(crate)` |
| Proc macro needed | no | yes (contradicts decision 3; cannot see the other documents, cannot depend on `build.rs` output except by `include!`, re-expands in the IDE) | no |
| User boilerplate | one field, one `include_xaml!`, one call | one attribute | one field, one trait impl, one call |
| Named fields look like fields | accessor methods (`self.xaml.greeting()`) | real fields possible | accessor methods |
| Works with `ferro_class!`'s `#[repr(C)]`, `base` first | yes (ordinary field) | macro must preserve layout rules | yes |
| Failure mode when wiring is missing | compile error at a generated line with a marker | macro error | compile error on the trait impl |

**Recommendation: A**, as already sketched in section 3.8 and open question 7. It needs no proc macro, keeps private code-behind private, and the generated file stays a plain Rust file that can be read and debugged. C is kept as the mechanism for classless documents only (they have no user module to be included in). B can be added later as sugar that expands to exactly A.

#### 9.4.3 What is generated for a class document

Inside `mod __<file>_xaml` (child module of the class's module), re-exporting the parts struct:

- `pub struct <Class>Xaml` with one private field per `x:Name` in the root name scope (names inside templates are in other scopes and get no field, as upstream), `#[derive(Default)]`.
  - Field type `RefCell<Option<Ref<T>>>` for descendants; `RefCell<Option<WeakRef<T>>>` when the named element is the root itself (a strong self-reference would leak).
  - One accessor per field, `fn <snake_case name>(&self) -> Ref<T>`, `#[track_caller]`, panicking when unset. Visibility from `x:FieldModifier`: `private` → no accessor visibility modifier (visible in the class's module through the re-export only), `internal`/`NotPublic`/absent → `pub(crate)` (upstream default is `internal`), `protected` → `pub(crate)` with a warning (no Rust equivalent), `public` → `pub`.
  - `T` is the static type the transformer resolved for the element. Name → accessor mapping is the PascalCase → snake_case rule of the porting guide; two names that map to the same identifier are a build error naming both positions.
- `impl <Class>`:
  - `pub(crate) fn initialize_component(&self)`: trampoline, then fills the parts from `NameScopeExtensions::find_name_scope(self)` + `find(name)` + `cast::<T>()`. Filling from the name scope (not from the locals of `xaml_populate`) is what upstream does and is what makes the fields correct when the populate override or the interpreter built the tree.
  - `pub(crate) fn xaml_populate(sp: &Rc<dyn IServiceProvider>, target: &Ref<Self>) -> Result<(), XamlLoadException>`.
- Visibility of the struct re-export from `x:ClassModifier`: it is validated against the visibility of the user's struct as upstream does (`XAML file x:ClassModifier doesn't match the x:Class type modifiers`); `Public` → `pub use`, `NotPublic` → `pub(crate) use`. Absent: `pub(crate) use`.
- A wiring check so that a missing field produces a readable error:
  `const _: fn(&MainWindow) -> &MainWindowXaml = |c| &c.xaml; // MainWindow.xaml(4,9): the x:Class struct needs a field `xaml: MainWindowXaml``.

Upstream's "no call to `Load(this)` found" check has no equivalent the build tool can perform reliably (it would need to analyse function bodies). Instead `xaml_populate`/`initialize_component` are ordinary items: if the user never calls `initialize_component`, rustc's dead-code warning on it is promoted to an error in the generated module with `#[deny(dead_code)]` on that one function.

#### 9.4.4 Event handlers

Implemented (2026-10-09). The design this section had (a closure with typed parameters over the upgraded weak root, handlers found among the inherent functions of the class) is replaced by what was built, which follows the run-time loader exactly.

**What the emitter sees.** A method name assigned to an event or to a property of a delegate type is turned into a `XamlLoadMethodDelegateNode` over the root object by the transform, before the back end, in both type systems alike:

| Markup | Setter of the assignment | Value |
|---|---|---|
| a routed event (`Click="OnClick"`, `Button.Click="OnClick"`): the declaring type has a static field `<Name>Event` that holds a routed event (`FerroXamlIlTransformRoutedEvent`) | `XamlDirectCallAddHandler` (`AddHandler(RoutedEvent, Delegate, RoutingStrategies, bool)`; for an event with arguments of a class of their own a second one over `AddHandler<TEventArgs>`, of which the plan of the assignment leaves the first) | `XamlLoadMethodDelegateNode` |
| an event that is not a routed event (`events:` of the metadata: `PropertyChanged`, `Opening`) | `XamlDirectCallPropertySetter` over the subscription `add_<Name>` the type system projects for the event | `XamlLoadMethodDelegateNode` |
| a property of a delegate type (`ToolTip.CustomPopupPlacementCallback="OnPlacement"`) | the setters of the property | `XamlLoadMethodDelegateNode` |

**Which method.** The one the transform finds, and no other: `XamlTransformHelpers.TryConvertValue` looks for a method of the type of the root object with the name, the return type and exactly the parameter types of the `Invoke` of the delegate type (`FindMethod(name, returnType, allowDowncast: false, parameters)`); when none matches, `FerroXamlIlLanguage::try_convert_method_delegate` accepts the method of that name whose parameters are assignable from the ones the delegate passes (the deviation recorded in DEVIATIONS.md, "Markup metadata and markup events": a handler that takes the arguments untyped receives whatever the event raises). The methods of a type are the ones its markup metadata declares (`markup: { methods: [fn OnClick(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) => ..] }`), which the scanner reads into the model as it reads every other member. The root object is an instance of the class of the document (`x:Class`); a document without one has the type of its root element, which has the methods of that type only. The signature is therefore checked at compile time by the same code and the same rule as at run time, and rustc checks the emitted call a second time.

Not implemented, and not a rule of the run-time loader either: handlers found among the inherent functions of the class (`impl <Class> { fn on_click(&self, ..) }`, rules 2 and 3 of the earlier text of this section, ruling 3 of 9.12). The scanner records those functions (`Scan::functions`), and the type system does not project them as methods: the interpreter could not call them, so a document would compile and not load. They become handlers when the build generates their metadata entries, which is ruling 3 as a whole and is not started.

**What is emitted.** In the build function, the populate function and the build function of deferred content alike (a handler named inside a template):

```rust
// MainWindow.xaml(23,30) Click
rt::add_handler(&button_0, &::ferroui_controls::Button::__markup_field_ClickEvent(), rt::method_delegate(&context, 2, |this, arguments| { crate::MainWindow::__markup_OnClick_0(this, arguments.next()?, arguments.next()?); ::core::result::Result::Ok(::core::option::Option::None) }));
// MainWindow.xaml(24,12) PropertyChanged
rt::add_event_handler(rt::class_markup(<::ferroui_base::FerroObject as ::ferroui_base::StaticType>::TYPE), "PropertyChanged", rt::to_value(border_0.clone()), rt::method_delegate(&context, 2, |this, arguments| { .. }), 24, 12)?;
// MainWindow.xaml(25,12) CustomPopupPlacementCallback
border_0.set_value(::ferroui_controls::ToolTip::custom_popup_placement_callback_property(), rt::cast(rt::method_delegate(&context, 1, |this, arguments| { .. }), 25, 12)?);
```

- **The delegate** (`XamlLoadMethodDelegateNode`, upstream's `new TDelegate(root, &Method)`) is `rt::method_delegate(&context, <number of parameters>, |this, arguments| ..)`, a `MarkupDelegate`. The closure calls the typed function of the declared method (`<Class>::__markup_<Name>_<n>`, 9.5.3 form B: `EmitTypes::method` of both hosts states it) with the instance and with each argument read from `arguments` in the type the method declares, which rustc infers from the function. The helper does what `load_method_delegate` of the interpreter does: the root object is read from the context (`root_object_field`), which is why the same text serves a template; it is held weakly (root, element and handler would otherwise form a cycle) and upgraded on each call; a call with another number of arguments than the method takes, an argument that is not of the declared type, a failing fallible method and a root object that is gone all make the call do nothing, since a handler has nowhere to report a failure to; an object passed to an untyped parameter is handed over in the untyped form of the framework; a returned value goes back in its untyped form. A static method is `rt::static_method_delegate`. Refused: a delegate over another object than the root object, a method that is not a declared one or has no typed function, and a method that returns a type (the run-time loader hands a class to the caller in a form of its own).
- **A routed event** (`XamlDirectCallAddHandler`) is `rt::add_handler(&target, &<Owner>::__markup_field_<Name>Event(), delegate)`: `Interactive::add_handler_untyped(event, delegate, Direct | Bubble, false)`, the call the interpreter makes through `AddHandler`, with the event read through the typed function of the declared field. The handler is called with the element it is attached to and a shared handle to the arguments. A choice of the setter at run time is refused; the plan of an assignment of a method name leaves one setter.
- **An event that is not a routed event** is `rt::add_event_handler(<metadata of the declaring type>, "<Name>", target, delegate, line, position)?`. Metadata declares the subscription of an event as a callable without a typed function (`events: [Name(..) => |this, handler: MarkupDelegate| ..]`), so this is call form C of 9.5.3: the subscription the interpreter invokes, found by the name of the event in the metadata of the type that declares it. The emitter recognises the case by the method of the setter being the `add` of an event of its declaring type.
- **A property of a delegate type** takes the delegate through the rule every value follows: the cast the crates register from `MarkupDelegate` to the type of the property (`rt::cast`), which is the conversion the run-time loader applies to the argument of the setter.

**A method that is not found.** No method fits when the class has none of that name, when its parameters are neither the delegate's nor wider, or when the document has no class. The compiler reports that as any assignment no setter takes (`Unable to find suitable setter or adder for property Click of type .. for argument System.String, available setter parameter lists are: ..`, upstream's text, code `FRN3000`, at the attribute), and the run-time loader gives exactly that. A compile describes it (`rust_emitter::transform`, `METHOD_NOT_FOUND`): when a setter of the member takes a delegate, the text assigned is read back from the document at the place of the diagnostic and the diagnostic becomes

```text
Views/Buttons.xaml(3,13): error FRN3000: No method `OnGo` for `Click` (System.EventHandler`1[FerroUI.Interactivity.RoutedEventArgs]): the document has no class (`x:Class`), and the type of its root object has no method of that name that the delegate can call. Unable to find suitable setter or adder for property Click ..
```

or, for a document with a class, "the class `Ns.Class` of the document has no method of that name that the delegate can call (a method the markup metadata of the class declares, `methods: [fn OnGo(..) => ..]`, with the parameters of the delegate or wider ones)". A build reports it once, with the document, the place, the code, the method and the member; the group of the document is not compiled.

**Proof.** `tests/FerroUI.Markup.Xaml.UnitTests/emitter/event_handlers.rs` with the class documents of the corpus (`corpus::CLASS_DOCUMENTS`), each compiled as the document of its class by both hosts of the emitter:

| Document | Class | Form | Both hosts | Run |
|---|---|---|---|---|
| `Handlers/RoutedEvent.xaml` | `MyButton` | a routed event of the element, and one of another class | the same bytes | the method runs when the event is raised; the object is freed when dropped |
| `Handlers/AttachedEvent.xaml` | `MyPanel` | a routed event of a child handled on the root; an event with arguments of a class of their own | the same bytes | both methods run; a handler whose object is gone does nothing |
| `Handlers/PlainEvent.xaml` | `MyHost` | an event of the class with a handler of wider parameters; the property-changed event of an element | the same bytes | the handler cancels the arguments the event raises; the changed property is received |
| `Handlers/DelegateProperty.xaml` | `MyHost` | a method as the value of an attached and of a plain property of a delegate type; a handler on the event of a flyout | the same bytes | the callbacks are set |
| `Handlers/TemplateEvent.xaml` | `MyPanel` | a handler named inside a control template | the same bytes | the content the template builds calls the method of the root object |
| `Handlers/UnknownMethod.xaml` | `MyButton` | no method of the name | the same diagnostic, not compiled | |
| `Handlers/OtherParameters.xaml` | `MyHost` | a method of the name with other parameters | the same diagnostic, not compiled | |
| `Handlers/UnknownMethodOfProperty.xaml` | `MyHost` | a property of a delegate type, no method | the same diagnostic, not compiled | |
| `Handlers/WithoutClass.xaml` | none | a handler on a document without `x:Class` | the same diagnostic, not compiled | |

The output of the five that compile is checked in (`emitter/generated_handlers/`), compiled with the test crate, and kept current by a test; a class populated by it is compared with a class populated by the run-time loader from the same document through the dump of 9.10.2, and the same assertions are made on both. `build_diagnostics.rs` has the error of a build.

#### 9.4.5 Populate override (hot reload, previewer)

Upstream stores the override in a static field found by reflection. There is no reflection, so the slot lives in the runtime library, keyed by class:

```rust
impl FerroXamlLoader {                                   // (new) R6
    pub fn set_populate_override(class: &'static TypeInfo, f: Option<Rc<dyn Fn(&MarkupValue)>>);
    pub fn populate_override(class: &'static TypeInfo) -> Option<Rc<dyn Fn(&MarkupValue)>>;
}
```

Thread-local map; `initialize_component` performs one lookup. The lookup is compiled out unless the crate enables the Cargo feature `hot-reload` of `ferroui-markup-xaml` (the generated code writes `rt::populate_override(Self::TYPE)`, which is `None` as a `const fn`-like inline without the feature). The previewer host registers, per class, a closure that runs the interpreter's `populate` on the instance with the current text of the document.

#### 9.4.6 What option A requires from the class model

1. `ferro_class!` structs may contain a non-`base` field of a generated type with `Default` (already true: any field is allowed after `base`).
2. `self.to_ref()` available in inherent methods (exists).
3. `constructed` (or `new`) is the documented place to call `initialize_component()`; `constructed` runs after the base classes' `constructed`, like a C# constructor body.
4. `X::TYPE` (exists) as the key of the override slot and of `x:Type`.
5. A stable, scanner-readable statement of the class's dotted namespace (`markup: { namespace: ".." }`, exists) and, for the build tool, the module path of the file declaring it (derived from the file's position under `src/`, 9.5.1).
6. Handler methods are inherent methods of the class in the same file as `ferro_class!` (rule 2 of 9.4.4), or are declared in metadata.

#### 9.4.6 Implemented (2026-10-09): a text a type converter converts when the document is loaded

A text assigned to a member whose type (or the member itself) states a type converter the compiler has no intrinsic for is converted when the document is loaded. The transform writes it as upstream's compiler does (`XamlTransformHelpers.ConvertWithConverter`):

```text
XamlAstNeedsParentStackValueNode
  XamlAstRuntimeCastNode -> T
    XamlStaticOrTargetedReturnMethodCallNode Converter.ConvertFrom(ITypeDescriptorContext, CultureInfo, object)
      XamlAstNewClrObjectNode Converter
      XamlAstContextLocalNode ITypeDescriptorContext
      XamlStaticOrTargetedReturnMethodCallNode CultureInfo.get_InvariantCulture
      XamlAstTextNode
```

`Source="/Assets/missing.png"` of an `Image` is the case of the catalog (here the document `image_source_rooted.xaml` of the corpus): `(IImage) new BitmapTypeConverter().ConvertFrom(context, CultureInfo.InvariantCulture, "/Assets/missing.png")`. The interpreter evaluates the nodes one by one (`runtime/interpreter/evaluators.rs`), and the emitter writes one statement per evaluation, with the typed functions the declarations give (form A of 9.3: nothing goes through the metadata by name):

```rust
// image_source_rooted.xaml(2,10) Source
let value_0 = ::ferroui_base::utilities::CultureInfo::__markup_static_get_InvariantCulture();
let value_1 = rt::invoked(::ferroui_markup_xaml::converters::BitmapTypeConverter::__markup_ConvertFrom_1(
    &::ferroui_markup_xaml::converters::BitmapTypeConverter::__markup_new_0(),
    ::core::option::Option::Some(rt::type_descriptor_context(&context)),
    rt::cast(::core::clone::Clone::clone(&value_0), 2, 10)?,
    rt::to_object(::std::string::String::from("/Assets/missing.png"))), 2, 10)?;
let cast_0 = rt::cast_checked(::core::clone::Clone::clone(&value_1), rt::markup_handle(<dyn ::ferroui_base::media::IImage as ::ferroui_base::metadata::MarkupTyped>::MARKUP), "FerroUI.Media.IImage", 2, 10)?;
image_0.set_value(::ferroui_controls::Image::source_property(), rt::exact(cast_0.clone(), "FerroUI.Controls.Image.set_Source", 0, 2, 10)?);
```

| Node | What the interpreter does | What is emitted |
|---|---|---|
| `XamlAstNeedsParentStackValueNode` | checks that the parent stack is provided, evaluates the value | the value. The context of generated code has the parents: the initialisation of every object around a node that needs them pushes it (`context.push_parent`), which the emitter decides from the same nodes (`ParentStackNodes`) |
| `XamlAstContextLocalNode` of `ITypeDescriptorContext` | the context of the document as the type descriptor context (`RuntimeContext`) | `rt::type_descriptor_context(&context)`: the context of generated code (`XamlIlContext`), which states the base URI of the document (`DocumentInfo::base_uri`) and the parents |
| the call of the converter | `ConvertFrom` with the converter, the context, the culture and the text as `object`; a failure is the exception that wraps the converter's | the typed function of the declared method, its failure through `rt::invoked` (the same wrapping) |
| `XamlAstRuntimeCastNode` to a reference type | `castclass`: null passes, an instance of the type is held in the handle of its class, anything else is `InvalidCastException: Unable to cast object of type 'A' to type 'T'.` | `rt::cast_checked`, with the same three outcomes and the same text |
| the assignment | the setter converts the value to the type it declares | the value is still untyped after the cast, so the setter gets `rt::exact(value, member, index, line, position)?`, the conversion of the run-time loader (`SetterValues::Untyped`, the form a setter chosen at run time already had); a declared setter that takes a reference type goes through `checked_cast` as before |

The type descriptor context is a Rust type the emitter names, in both hosts (`Known::TypeDescriptorContext` and its nullable form): the type is one of the closed table of runtime library types, which has no metadata of its own to take a path from.

**Not emitted:** the cast to a value type (`unbox.any`, where null is `NullReferenceException`). The document is refused with the converter and the type in the reason. No document of the catalog has one: its 37 are the converter of bitmaps to `IImage` (22), `IImageBrushSource` (14) and `IBitmap` (1).

**Deviations.** (1) The call of `get_InvariantCulture` is written before the converter is created (a call that yields a value is held in a local, and the converter is created in the argument list), where the interpreter creates the converter first; neither has an effect the other can see. (2) The base URI of the context is the text the build was given for the document (`ferres://ControlCatalog/Pages/ImagePage.xaml`), where the run-time loader's is the text of the `Uri` it parsed (`Uri::to_string`, with the host in lower case); they are the same URI, and differ as text only through `Uri::original_string`.

**Proof.** Seven documents of the corpus (9.10.2), emitted the same by both hosts: a converter of the test crate on a registered property (a text, the nearest parent that is a control, null, the base URI, a failure of the converter, a value of another type), and the converter of bitmaps for an `Image` and an `ImageBrush`. A bitmap that exists is not loaded by any test (the test services decode none): the documents of the bitmap converter are compared by the failure both back ends raise for an asset that does not exist, looked for below the base URI of the document.

#### 9.4.7 Implemented (2026-10-09): the type of an untyped value as a value

`{x:Type x:Object}` and `x:DataType="x:Object"` (`TabControlPage`). `System.Object` is no class of the object model, has no markup metadata and is no primitive of `EmitTypes::primitive_type_name`, so `XamlTypeExtensionNode` had no form for it. The run-time loader passes the value type of the handle of the type (`RuntimeTypeValue::to_handle`, `to_untyped_value`), and the handle of `System.Object` is the Rust type that holds an untyped value. The emitter writes that type where the handle of the named type is the untyped one (`Handle::is_object`, answered by both hosts): `::ferroui_base::data::core::ValueType::of::<::core::option::Option<::ferroui_base::BoxedValue>>()`, which is `ValueType::object()`, as a value type, in its nullable form (`DataTemplate.DataType`) and boxed for an untyped target (`Tag`). Where a class is declared (`&'static TypeInfo`) there is no form, as at run time (`.. is not a class of the object model`). Corpus: `type_extension_object.xaml`.

#### 9.4.8 Implemented (2026-10-09): a compiled binding path over a property whose type generated code cannot name

The element of a plain property in a compiled binding path is `rt::path_property(&builder, <Owner>::MARKUP, "Name", is_static, <type of the property>, ..)`, and so is the description of a provide-value target property (`rt::clr_property_info`). The type of the property was written as the handle of a type with metadata, of a class, of a primitive or of an element reference (`Emitter::handle_expr`), and a document was refused for every other property type: `T has no metadata` and `no public Rust path is recorded for T` (six documents of the catalog: a list of a type of the sample, `SelectedItemsList`, `ErrorConverter`, `Nullable<DateTime>`, `Nullable<FlexAlignItems>`, `IObservable<Object>`).

The run-time loader passes `handle_of(property.property_type())`. For a type nothing declares, that handle is the Rust type the declaration of the property states (the type system made the type from it). So where `handle_expr` has no form, and the handle of the type of the property is the Rust type the declaration states for the getter (`DeclaredMethod::returns` of `EmitTypes::method`, compared by the emitter, in both hosts), the emitter writes

```rust
rt::declared_property_type(<Owner as MarkupTyped>::MARKUP, "Name", is_static)
```

which reads the type from the declaration at run time (`MarkupProperty::type_`). Where the two differ (a type of the runtime library with several Rust handles, of which the property states another than the first) or the property has no getter, the document is refused as before: the type generated code would pass is not the one the run-time loader passes. No existing output has the new form (a type `handle_expr` names is written as before). Corpus: `compiled_binding_unnamed_types.xaml` (`Row.Stamp`, a Rust type without metadata, and `Row.Length`, `Option<i32>`), with `compiled_bindings_read_properties_of_types_generated_code_cannot_name`, which sets a data context and reads the values both back ends deliver.

#### 9.4.9 Implemented (2026-10-09): a list of the runtime library created in markup

`<generic:List x:TypeArguments="T">` and `<collections:ArrayList>` (five documents of the catalog, each as the items source of an items control). The run-time loader creates one list for both (DEVIATIONS.md, "Run-time type system of markup"): the element type and the untyped items in a shared `FerroList<Option<BoxedValue>>`. The type was a type of the loader (`RuntimeList`, `runtime/type_system/values.rs`), which generated code cannot name. It is now a type of the runtime library (`ferroui_markup_xaml::xaml_il::runtime::RuntimeList`, `rt::RuntimeList`), and the run-time loader creates that same type: a list is the same value whichever back end built its document. The element type of a list is what it is named by (`RuntimeListElement`): the type of its type system for a list the run-time loader creates, the full name the compiler resolved for a list generated code creates; the type system of the run-time loader finds the type of the second by that name when it is asked the type of the value.

```rust
// runtime_lists.xaml(6,8) ItemsSource
let list_1_0 = rt::RuntimeList::of("FerroUI.Media.Stretch");
// runtime_lists.xaml(7,18) Content
rt::RuntimeList::add(&list_1_0, rt::to_value(::ferroui_base::media::Stretch::Uniform));
combo_box_0.set_value(::ferroui_controls::ItemsControl::items_source_property(), rt::list_cast(&list_1_0, 8, 18)?);
```

| Node | What the interpreter does | What is emitted |
|---|---|---|
| `XamlAstNewClrObjectNode` of ``System.Collections.Generic.List`1[T]`` or `System.Collections.ArrayList`, whose constructor no metadata declares | the constructor of the table of runtime library types: `RuntimeList::new(element type)` | `rt::RuntimeList::of("<full name of T>")`, `rt::RuntimeList::untyped()`. An instantiation of `List<T>` a crate declares metadata for (``List`1[String]``) has a declared constructor and keeps the Rust type of its metadata, as at run time |
| the children: `XamlPropertyAssignmentNode` with the `XamlDirectCallPropertySetter` of `Add` | the item evaluated as the type `Add` takes (the element type, `object` for an `ArrayList`), then `RuntimeList::add`, which holds an object of the object model in its untyped form; the index `ArrayList.Add` returns is dropped | the item stated as the handle of that type (`EmitTypes::handle_of`: the adder is a member the type system builds and states no Rust type), held untyped as the run-time loader holds the value of a node (`rt::to_value`, or the value as an object), then `rt::RuntimeList::add` |
| the list assigned to a member | `to_exact`: an untyped target and a target of the type of the list take the list; any other target the shared list of the items through the cast the crate of the target registers for it (`RuntimeList::to_declared`: `Rc<FerroList<Option<BoxedValue>>>` to `ItemsSource`) | `Emitter::coerce_runtime_list`: `rt::list_cast(&list, line, position)?`, the same call of `to_declared`, written where `EmitTypes::is_assignable(Known::ListItems, target)` proves the cast exists; `rt::to_object` for an untyped target |

Both hosts name the two Rust types (`Known::RuntimeList`, `Known::ListItems`). Neither type has a handle in the type system (the table of runtime library types gives the two lists none), so nothing else is asked of a host.

**Not emitted:** an array (`x:Array` is not a directive of the language of the port, and the array constant of the compiler, `FerroXamlIlArrayConstantAstNode`, is in no document of the catalog; it is refused by name as before). **Proof:** `runtime_lists.xaml` of the corpus (a typed list and an untyped one as items sources, with null, a text, a value of an enumeration and a control; an untyped and an empty typed list as a `Tag`), emitted the same by both hosts and equal to the run-time loader's tree, and `a_list_created_in_markup_is_the_list_of_the_run_time_loader`, which reads the items both back ends deliver; the five pages of the catalog are in the fixture of 9.5.19 and load to the trees of the run-time loader.

### 9.5 Typing without the framework linked into the build tool

#### 9.5.1 Inputs of the build-time type system

| Input | Read from | Gives |
|---|---|---|
| `ferro_class!(X: Base [, virtuals ..])` | source scan | class, base class, Rust path (crate + module of the file + `X`) |
| `ferro_static_type!(X)` | source scan | static owner types of attached properties |
| `ferro_properties! { impl X [, also [..]] [, fn name] { .. } }` and `ferro_property!` | source scan | registered properties: accessor name (`child_property`), kind and exact value type from the return type (`StyledProperty<T>`, `AttachedProperty<T>`, `DirectProperty<O, T>`), XAML name and owner/host from the `FerroProperty::register*::<..>("Name", ..)` expression, `add_owner::<Y>()`, `.assign_binding(true)` / `.inherits(true)` builder calls |
| `ferro_class_info!(X { new, interfaces, markup: {..} })` | source scan | default constructor, interface handles, content property, constructors, plain properties, methods, fields (incl. routed events), events, attributes, `property_attributes`, `namespace` |
| `ferro_markup_type!(struct/class/interface/static ..)`, `ferro_markup_enum!` | source scan | everything else; `handles:`, `this:`, `parse:`, enum members and values |
| `register_types.rs` | source scan | module → dotted namespace table (`NAMESPACES`), the `MarkupAssembly` (assembly name, `XmlnsDefinition`s, metadata such as `CREATE_SOURCE_INFO`); since 9.5.8 the list of registered classes (`TYPES`) and, from any function of the crate, the registered handles and casts |
| `impl X { fn .. }` blocks of `x:Class` types | source scan | handler candidates (9.4.4) |
| `<crate>.xamlmeta` of every dependency | Cargo `links` metadata (9.6.3) | the same model, serialised, for crates whose sources the tool does not read |
| closed `System.*` table | loader (`core_table`, shared with the run-time type system since 9.5.7) | BCL types |

The scanner uses `syn` on file level and a small hand-written parser for the bodies of our macros (they are token trees with the rigid grammar documented in the porting guide; decision 3 made them rigid for this purpose). It does not expand macros and does not resolve arbitrary Rust.

The serialised model (`.xamlmeta`, JSON, defined in `ferroui-build` per decision 2):

```text
AssemblyModel { name, crate_name, xmlns_definitions[], xmlns_prefixes[], metadata[], namespaces[(module, dotted)],
                types[], documents[DocumentModel], dependencies[path of .xamlmeta] }
TypeModel     { namespace, name, kind, rust_path, handles[RustType], this, base, interfaces[], generic,
                content_property, has_parse, constructors[], properties[], registered[], methods[], fields[],
                events[], enum_members[(name, rust_variant, value)], is_flags, attributes[], notify_property_changed }
MemberModel   { name, parameters[RustType], return_type, is_static, fallible, attributes[],
                call: Path("crate::module::Type::function") | Invoker }          // 9.5.3
RegisteredModel { name, kind: Styled|Direct|Attached, value_type: RustType, owner, host, accessor: "Type::name_property",
                  assign_binding, added_owners[] }
DocumentModel { uri, root_type, class_rust_path?, build_path?, populate_path, public }
RustType      = normalised token text with every path made absolute ("::ferroui_base::Ref<::ferroui_controls::Control>")
```

#### 9.5.2 Rust type text → XAML type

Property and member types are Rust types. The runtime type system maps them through `ValueType` (a `TypeId`); the build-time one maps the *normalised type text*:

1. Resolve every path segment against the `use` items of the declaring file, then the crate's own type table, then dependency models; make it absolute.
2. Look the result up in the table of `handles:` (every `TypeModel` lists the Rust types that hold a value of it; `Ref<X>` and `Option<Ref<X>>` are implied for classes).
3. Structural rules: `Option<T>` of a value type → `Nullable<T>`; `BoxedValue` / `Option<BoxedValue>` → `System.Object`; `&'static TypeInfo` / `ValueType` → `System.Type`; primitives and `String` from the core table; `Rc<dyn BindingBase>` etc. through interface handles.
4. Anything else → an **opaque type** that carries the Rust text (the counterpart of `RuntimeTypeOrigin::Opaque`): not assignable from or to anything but itself and `object`, no members.

#### 9.5.3 How a member is called from generated code

Metadata callables are paths or closures *relative to the module that declares them* (`set: set_setter_value` in `markup_types/plain.rs` names a private function; many getters are closures). They cannot be pasted into another crate. Three forms, chosen per member by the scanner and recorded in `MemberModel::call`:

| Form | When | Emitted | Cost |
|---|---|---|---|
| **A. Structural** | registered properties, routed-event fields, enum members, `x:Type`, class default constructors (`new:` path), selectors, binding-path builder, runtime helpers | direct typed code derived from the declaration (`Owner::name_property()`, `Type::Variant`, `Type::new()`) | none |
| **B. Typed path** | the callable is a path `Type::function` whose first segment resolves to a type of the model and whose function the scanner found as `pub` (or `pub(crate)` when the type is in the crate being compiled) in an `impl Type` block | `Type::function(&this, args..)` with the declared argument types (the porting guide guarantees the callable takes `(&this, args..)` in exactly the declared types) | none |
| **C. Invoker** | everything else: closures, private helper functions, callables of crates without readable source and without a `Path` in their `.xamlmeta` | `rt::set_property(<T as MarkupTyped>::MARKUP, "Name", &this, value)?`, `rt::get_property`, `rt::call_method`, `rt::construct` **(new)**, which run the very `MarkupInvoke` thunk the interpreter runs (for classes the metadata is `T::TYPE.markup()`) | boxing + a name lookup; cached per call site with a `OnceCell` index when it shows up in profiles |

Form C makes every declared member usable from day one with semantics identical to the interpreter by construction. Form B is an optimisation the scanner applies opportunistically; the `Setter.Value` line of the example is form C, `DataTemplate::set_content` form B. A lint in the scanner (`--report-invokers`) lists the members that fall to form C so that framework declarations can be turned into `pub` paths where it matters.

As built: 9.5.9 (the choice, what form A covers among the callables, the form for a function visible in its crate only, the numbers of the framework crates).

#### 9.5.4 Partially known types: trait-directed fallbacks

When a type or member type is opaque or unresolved, the transformers have already accepted the document on weaker information (opaque types convert only from text through "unknown `Parse`"), and the emitter writes code that lets rustc finish:

| Situation | Emitted | Trait **(new, in `rt`)** |
|---|---|---|
| text → value of an opaque or parse-declared type | `rt::from_xaml_str::<_>("0.5", &ctx_sp).map_err(..)?` (type inferred from the consumer) | `FromXamlStr`, implemented for every type with `parse:` by the declaration macros and for primitives with the invariant-culture parsers of section 3.5 |
| value of unknown type → `object` | `rt::to_markup(v)` | blanket over `PartialEq + 'static` (`into_markup_value`) |
| begin/end init on unknown type | `rt::begin_init(&x)` / `rt::end_init(&x)` | `MaybeSupportInitialize` (autoref-specialisation shim: calls `begin_init` when the type has it) |
| assignment to a registered property whose value type is opaque | `rt::set_from_xaml(&obj, Owner::p_property(), "text", &ctx_sp)?` | `FromXamlStr` on the property's `T` |
| handler with unscanned signature | plain method call; rustc checks | none |

Every such line carries a marker comment, and the build prints a note (`FRN-I...`, not a warning) counting them per document, so that "typed by rustc" places are visible and can be driven to zero for framework types.

Limits are the ones of section 2.5 (types produced by user macros, `cfg`-gated declarations, aliases): each produces a targeted diagnostic naming the declaration form that would make the type visible.

#### 9.5.5 One projection for both type systems

Today `runtime/type_system` (about 3.5k lines) projects `TypeInfo` / `MarkupType` / `MarkupAssembly` directly onto `IXamlTypeSystem`, with `ValueType` keys and `RuntimeInvoker`s. The language-relevant part of that projection must be identical at build time:

| Projection rule (today in) | Shared? |
|---|---|
| core `System.*` types and their members (`core_types.rs`) | yes, data-driven table |
| registered property → CLR-style property + `<Name>Property` static field; attached → `Get<Name>`/`Set<Name>` statics; `host_type` (`runtime_type_system.rs`) | yes |
| synthesised object-model members: `AddHandler` overloads, `GetValue`/`SetValue`/`Bind`, `Classes`, generic `RoutedEvent<T>` / `EventHandler<T>` instantiations (`object_model.rs`) | yes |
| attribute projection (`KNOWN_ATTRIBUTES`, `FerroUI.Metadata.<Name>Attribute`, argument conversion) | yes |
| xmlns definitions → assemblies/namespaces, module → namespace | yes |
| generic definitions and instantiation (`FerroList\`1`, `Nullable\`1`) | yes |
| assignability (base chain, interfaces, nullable, `object`) | yes |
| synthetic `CompiledFerroXaml.!FerroResources` type with `Build:<path>` methods from `documents[]` | yes (build path needs it for cross-crate includes; the runtime path can expose registered compiled loaders the same way) |
| member invocation (`RuntimeInvoker`, `to_exact`, `ValueTypes` casts) | no: runtime only |
| Rust spelling of types and call forms (9.5.3) | no: build only |

Design: an in-memory **`MarkupModel`** (the structs of 9.5.1, owned strings, type references by key) and one `ModelTypeSystem<B: ModelBacking>: IXamlTypeSystem` over it, placed in the loader crate outside the `runtime` feature. `B` attaches the per-member payload: `RuntimeBacking` (invoker, `ValueType` handles, `&'static FerroProperty`) or `EmitBacking` (`RustType`, call form). The model is filled either from the registries (`TypeOf` → `ValueType::name()` as key) or from scan + `.xamlmeta`.

Migration without destabilising the interpreter: (1) build `ModelTypeSystem` with `EmitBacking` first, extracting the tables of `core_types.rs` and `object_model.rs` into data both can read; (2) add the drift test: dump `RuntimeTypeSystem` and the scanned `ModelTypeSystem` through the `IXamlType` traits into a canonical text (types, members, parameter type names, attributes) and compare them for `ferroui-base`, `ferroui-controls`, `ferroui-markup-xaml`; this is the "scanner and registry agree" exit test of phase 0; (3) only then, optionally, re-seat the runtime type system on the model.

#### 9.5.6 Implemented (2026-10-09): the model and the source scanner, first stage

Written without a build (the validating session builds it; HANDOVER.md section 11 has the commands). In `ferroui-build` (`src/FerroUI.Build.Tasks`); the crate links neither the controls nor the themes, and the scanner links nothing of what it reads.

**The model (`model.rs`).** The structs of 9.5.1, owned text only, with these differences from the listing there:

| 9.5.1 | As built | Why |
|---|---|---|
| `RustType` = text | `RustType { text, unresolved[] }` | a path the scanner does not resolve stays as written and is listed, so that a reader of the model never takes a guessed path for a resolved one |
| `TypeModel::rust_path` | `rust_path` (the declaring module and the name: the key of the type) and `public_path` (the shortest path another crate names it by) | generated code needs the public path, the handle table needs one spelling per type |
| `has_parse` | `parse: Option<CallableModel>` | the callable is needed for the call form |
| `properties[]` of `MemberModel` | `PropertyModel { value_type, getter, setter, attributes }` with `AccessorModel` per accessor; `static_properties[]`, `indexers[]` (a `PropertyModel` with parameters) next to it | a property has two callables, each with its own `try` |
| `MemberModel::call: Path \| Invoker` | `callable: CallableModel { path as written, resolved }`, `typed_function` (the `__markup_*` function the declaration macro writes with `markup-functions`), `call: Option<CallForm>` | the scanner records what the choice of 9.5.3 needs and does not choose; `CallForm` is `Structural`, `Path`, `Invoker` |
| `RegisteredModel { name, .. }` | `name: Option`, `registration: Declared \| AddedOwner \| Alias \| Unknown`, `source` (the accessor an added owner calls), `visibility`, `inherits` | the name of a property another crate declares is in that crate's model, not in the body of the accessor |
| `enum_members[(name, rust_variant, value)]` | `EnumMemberModel { name, rust_variant, rust_value, value: Option }` | the value is known only when the crate declares the enumeration (or the `bitflags!` type) and the scanner can evaluate it |
| not listed | `explicit_namespace`, `module`, `cfg[]`, `type_info`, `default_constructor`, `property_attributes`, `ParameterModel` (name, attributes) | read from the declarations; `module` and `explicit_namespace` are what `namespace` is computed from |

**The file.** `.xamlmeta`, JSON (`json.rs`: a reader and a writer that keep integers, which the model has). Two formats, told apart by the member `"format"`: no such member is format 1, the file the emitter writes today (`rust_emitter::XamlMetadata`: `name`, `crate_name`, `documents`, `dependencies`); `"format": 2` is those four members unchanged plus the model. The reader of the compiler looks its four members up by name, so it reads a file of format 2 as the documents of the crate; `AssemblyModel::parse` reads both (a file of format 1 is a model without types), and refuses a format above 2. `AssemblyModel::metadata()` and `from_metadata()` convert to and from the compiler's `XamlMetadata`. A member with the default of its kind is left out of a file of format 2.

**The scanner (`scanner/`).** `scan_crate(&ScanOptions) -> Scan { model, diagnostics, files, functions, statistics }`. It reads the files from the crate root along the `mod` declarations (`#[path]`, inline modules; `#[cfg(test)]` modules and items are left out, other `cfg` conditions are recorded on the type), parses each with `syn`, and reads the bodies of the declaration macros from their tokens (`declarations.rs`, the grammar of the macro definitions and of PORTING-GUIDE.md "Markup metadata").

| Read | From | Gives |
|---|---|---|
| class, base, Rust path | `ferro_class!`, `ferro_static_type!`; `impl ObjectType` / `impl StaticType` written by hand (the root class) | `TypeModel` (a hand-written type has no base in the model) |
| registered properties | `ferro_properties!` (all four forms: wrapped or direct accessors, `also [..]`, `fn part`), `ferro_property!` in an `impl` block or with `for Owner;` | kind and value type from the type of the accessor; name, owner, host from `FerroProperty::register*::<..>("Name", ..)` found anywhere in the body; `Other::accessor().add_owner*::<..>(..)`; an accessor whose body is another accessor (an alias); `.assign_binding(true)`, `.set_assign_binding(true)`, `.inherits(true)` |
| class metadata | `ferro_class_info!`, any number per class, merged | `new`, `interfaces` (the `=> cast` form too), every part of `markup` |
| other types | `ferro_markup_type!` (the four kinds, `dyn Trait as "Name"`, generic heads), `ferro_markup_enum!` (plain, `flags`, renamed members, the second group) | handles, `this`, `base`, members; `static X { type_info: X }` is merged into the static type |
| enumeration values | `enum` items; `bitflags!` (integer literals, `<<`, `\|`, `Self::A.bits()`) | `EnumMemberModel::value` |
| assembly, namespaces | `static _: MarkupAssembly = MarkupAssembly { .. }`, `const NAMESPACES` (in any file, usually `register_types.rs`) | literals, text constants of the crate, `FERRO_XML_NAMESPACE` and `MarkupAssembly::CREATE_SOURCE_INFO` of the base crate |
| functions | `impl X { fn .. }`, `ferro_routed_event!` in an `impl` block | `Scan::functions`: owner, name, visibility, receiver, parameter and return types as written (for 9.5.3 form B and for the handlers of 9.4.4) |
| public paths | `mod`, `pub use` (names, lists, globs), `pub` items | the shortest public path of each item, by the rule of `scripts/rust_paths.py`; the entries of `ferro_rust_paths!` are compared with it (`FRN9024` where they differ) |

*Type text (9.5.2, step 1).* A path is resolved against the items of the module, its `use` items (an explicit import before a glob) and the glob imports of modules of the crate, followed through re-exports to the module that declares the item, and written absolute (`::ferroui_controls::border::Border`). A path into another crate is absolute as the `use` item spells it; it is not followed to the declaring module, which needs that crate's model. Primitive types and `String`, `Option`, `Vec`, `Box`, `Result` stay as they are. Not resolved, and kept as written with a `FRN9020` warning: a name that nothing of the module declares or imports (in a file with a glob import of another crate every such name may come from the glob), and a path whose unknown head is not the name of a crate.

*Macros of the crate.* The scanner does not expand macros, with one exception: an invocation of a `macro_rules!` macro the crate defines, whose rule takes only identifiers, types, a visibility, literals and attributes, is expanded by substitution, and the types (`pub struct $name`) and declaration macros at the top level of the expansion are read. `ferro_transition_class!` and `ferro_markup_list!` have that form. A macro of the crate that declares through a declaration macro and is not expanded (a repetition in its rule) is a `FRN9012` warning. (Since 9.5.16: a macro another crate exports is expanded the same way, with the rules the model of that crate has; and a rule that repeats is expanded for what a function registers with the untyped value conversions.)

*Diagnostics (`FRN9xxx`, 9.6.5).* `FRN9001` a file is not read; `FRN9010` a declaration has a form the reader does not know (the declaration is not in the model); `FRN9011` a declaration macro is invoked where the scanner does not read (inside a function); `FRN9012` a macro of the crate is not expanded; `FRN9013` a runtime type or metadata written by hand; `FRN9020` an unresolved path; `FRN9021` an accessor or a registration that is not read; `FRN9022` a type declared twice; `FRN9023` a declaration names a type the scanner has no declaration of; `FRN9024` a stated public path differs; `FRN9030` the assembly or the namespace table. Every file reports, per declaration macro, how its invocations were met (read, failed, in a macro definition, in an unread position, in test code); their sum is the number of invocations in the text of the file, which is what the test of the real crates compares.

**What the scanner cannot obtain from source**, and where it shows (the numbers are printed by the test `real_crates_are_scanned_without_skipping_a_declaration`; they are not in this document because the author did not run it):

1. The name of a registered property whose accessor adds an owner to, or is an alias of, a property of another crate (`RegisteredModel::name` is nothing; `source` names the accessor). Summary line "registered properties: .. without a name".
2. The declaring module of a type of another crate (the path is absolute as written, not canonical), so a base class or a handle of another crate is not yet linked to its `TypeModel`.
3. Names behind a glob import of another crate (`use ferroui_base::*;`): unresolved. Summary lines "unresolved: `Name` in N type texts".
4. What a macro of the crate declares when its rule repeats (`FRN9012`), and what a macro of another crate declares (`ferro_markup_list!` used outside the controls crate): the definition is not in the scanned sources.
5. Values of enumeration members whose enumeration is declared in another crate or whose discriminant is not a literal; values of flags built from anything but literals, shifts and other constants. Summary line "enumeration members: .. without a value".
6. Which types `register_types()` registers (the `TYPES` lists and `MarkupType::register_all`): the scanner reads every declaration, registered or not. The drift test of the next stage shows the difference.
7. `cfg` conditions on single accessors and members inside a declaration (the conditions around the declaration are recorded).
8. Hand-written `impl MarkupTyped` (`FRN9013`), and the base of a hand-written class.
9. Anything a closure does: a callable that is not a path is recorded as an expression; its call form is C or the typed function.

(Since 9.5.8: item 5 holds only for a member that is no constant expression, none in the two crates; item 6 is read for the classes, `TypeModel::unregistered`, and not for the types with markup metadata; a macro of the crate invoked among the members of an `impl` block is expanded like one in item position.)

**Seams left for the next stages** (9.10.1): `MemberModel::call` / `AccessorModel::call` are nothing; `Scan::functions` and `CallableModel::resolved` are the inputs of the choice; `Scan::normalise(module, text)` resolves further type text (handler signatures); `AssemblyModel::dependencies` and `find_rust_type` are where dependency models attach; `Build` does not call the scanner. (The last two are closed by 9.5.7: `ScanOptions::dependencies`, `ModelSet`, `Build::export_metadata`. With the model of the base crate, the points 1 to 3 of the list above no longer hold for the controls.)

#### 9.5.7 Implemented (2026-10-09): dependency models and the `ModelTypeSystem`, second stage

Written without a build, like 9.5.6 (HANDOVER.md section 12 has the commands and what its author doubts). It adds the stages 2 and 3 of 9.10.1: the models of the dependencies, and the type system of the compiler over the models. `Build::compile_xaml()` and the emitter are untouched and still transform against the run-time type system.

**Added to the model (`model.rs`).**

| Member | What | Why |
|---|---|---|
| `AssemblyModel::exports[ExportModel { path, declared }]` | the export table of the crate: every path another crate can name a type of the crate by (`::ferroui_base::Ref`), with the path of the declaring module (`::ferroui_base::type_system::Ref`); a path into another crate for a type the crate exports again | 9.10.1 stage 2 asked for it ("the scanner has the export table, the model needs it written out"). It is what resolves a glob import of the crate and what makes a path into the crate canonical. Functions, constants and statics are left out; a type a macro declares that the scanner did not read is kept under the path it is named by |
| `TypeModel::object_model` | the type is a class (`ferro_class!`) or a static owner type (`ferro_static_type!`, or a runtime type written by hand) | the type system projects such a type from its registered properties and implies its handles (`Ref<X>`, `Option<Ref<X>>`); until now only a heuristic told it from a plain markup class |
| `RegisteredModel::read_only` | the registration of a direct property states `None` where the setter goes (`register_direct*::<..>("Name", getter, None, ..)`, `.add_owner::<..>(getter, None, ..)`) | the property has no `set_Name` (`FerroProperty::is_read_only`) |
| `TypeModel::visit_types_mut`, `visit_callables_mut`, `named_path()` | every type text and every callable of a type; the path generated code names the type by | the canonical rewrite of a scan with dependencies; `MemberSource` |

The file stays format 2 (the new members are left out when they are the default, so a file written by the first stage is read unchanged, as a model without an export table).

**The models of several crates (`model_set.rs`, `ModelSet`).** `ModelSet::new(models)` indexes the export tables and the types of the models. `canonical_path` replaces the longest exported prefix of a path by the path of the declaring module, following a re-export of another crate; `canonical` does it for every absolute path of a normalised type text; `find_rust_type` (the one of 9.5.6, across models and in any spelling), `find_accessor`, `declaration_of` and `name_of` find a type, the accessor of a registered property and the declaration it is an owner or an alias of; `ModelSet::read` reads the `.xamlmeta` files of the dependencies and, transitively, the ones they name (9.6.3), each as an `AssemblyModel` (a file of format 1 is a model without types).

**The scan with dependency models (`ScanOptions::dependencies`).** With the models of the crates the crate is built on:

1. A glob import of a module of such a crate (`use ferroui_base::*;`, `use ferroui_base::media::*;`) is resolved by the export table: a name the table has is the type, by its declaring module; a path below a module the glob brings (`media::Brush`) likewise. A name no table has stays unresolved, as before: the module still counts as one with a glob of another crate, so nothing is guessed.
2. Every type text and every resolved callable of the model is canonical: a path into such a crate is the path of the declaring module, whichever re-export the file names it by. The text of a type is then the same in the model that declares it and in every model that names it, which is what the handle table of the type system is looked up by.
3. An owner added to, or an alias of, a property of such a crate has the name of the property (`RegisteredModel::name`), found through the accessors of this crate and of the others alike.

Not done with dependency models, and why it is not needed: the values of enumerations of other crates (a `ferro_markup_enum!` implements a trait of the base crate for the enumeration, so the enumeration is always of the crate that declares the metadata).

**`export_metadata()` (`Build`).** `Build::from_env().export_metadata()` scans the sources of the crate (root: `Build::source_root`, else `src/lib.rs`, `lib.rs`, `src/main.rs`) with the models of its dependencies (`DEP_<CRATE>_XAML_XAMLMETA`, transitively), and `run()` writes the model into `$OUT_DIR/<crate>.xamlmeta` next to the documents it wrote before, with `cargo::rerun-if-changed` for every file the scan read (a new file is reached through a `mod` item of a file that changed). A declaration the scanner cannot read is a `cargo::warning=` line. A crate that only exports its model states no `assembly()` (the scan reads the `MarkupAssembly` of the sources; a crate without one is the assembly named after the crate), links nothing of itself and writes no `mod.rs`.

*The framework crates do not call it yet.* The switch needs, and costs:

1. `links = "<crate>_xaml"` and a `build.rs` in `ferroui-base`, `ferroui-controls`, `ferroui-markup-xaml` and the other framework crates with declarations (the themes have both).
2. A build crate the base crate can use. `ferroui-build` depends on `ferroui-base` (and on the loader, which does), so the base crate cannot take it as a build dependency: Cargo refuses the cycle. The scanner, the model and `export_metadata()` use the base crate for two text constants (`FERRO_XML_NAMESPACE`, `MarkupAssembly::CREATE_SOURCE_INFO`) and the loader for `DocumentModel` only. They have to move into a leaf crate (`ferroui-build-scan`: `json`, `model`, `model_set`, `scanner`, the export) with the two constants stated there and `DocumentModel` owned there; `ferroui-build` then depends on it, and the loader takes `DocumentModel` from it.
3. The cost per build: `syn` (with `full`) and `proc-macro2` built for the host of every framework crate, and a scan on every source edit of the crate (the base crate: 1356 files). The scan is not cached yet (9.6.4); measure it before the switch, the target of 9.6.4 is a no-op run under 100 ms.
4. A crate above the base crate (`ferroui-controls`) can call `ferroui_build::Build::from_env().export_metadata().run()` today, at the price of building the base crate, the runtime library and the loader for the host as well.

**The shared table (`ferroui_markup_xaml_loader::core_table`).** The closed table of runtime library types is data in the loader, outside the `runtime` feature: for every `System.*` type its kind, type parameters, base, interfaces and members (`CoreType`, `CoreMember`, `CoreRef`), the lists markup creates (`list_members`), the list converter (`list_converter_members`), the property definition types (`property_types`), the names attributes are projected to (`attribute_type_name`, `KNOWN_ATTRIBUTES`) and the Rust type text of the handles (`core_handles`). `runtime/type_system/core_types.rs` defines its types from the table and keeps what only the run-time type system has: the Rust types of the handles (`handles_of`) and the invokers of the members the table marks as implemented (`invoker_of`, `list_members`). `ferroui-build` reads the same table. Two statements remain double and are small: the members of the list converter (the run-time `list_converter.rs` is not re-seated on `list_converter_members`), and the handles (types on one side, their text on the other).

**`ModelTypeSystem` (`ferroui-build`, `type_system/`).** `ModelTypeSystem::new(models) -> Rc<Self>` implements `IXamlTypeSystem` over a `ModelSet` and the table; `ModelType`, `ModelMethod`, `ModelConstructor`, `ModelProperty`, `ModelField`, `ModelEvent`, `ModelCustomAttribute`, `ModelAssembly` implement the other contracts. It mirrors `runtime/type_system` rule by rule (the table of `type_system/mod.rs`); the differences from the design of 9.5.5:

| 9.5.5 | As built | Why |
|---|---|---|
| `ModelTypeSystem<B: ModelBacking>` in the loader, the run-time type system re-seated on it | a second type system in `ferroui-build`, next to the run-time one | the model and the scanner are in `ferroui-build`; one projection for both back ends means re-seating the interpreter's type system, which 9.5.5 itself puts last ("only then, optionally"). The drift test is what holds the two together until then |
| `EmitBacking` (Rust type, call form) | `MemberSource` on every member: `Registered { type_path, accessor }`, `DefaultConstructor`, `Declared { type_path, callable, typed_function, fallible }`, `TypeSystem` | what the declaration states; the call form of 9.5.3 is chosen from it by the next stage |
| shared `object_model.rs` as data | the members of the root classes (`SetValue<T>`, `SetValue`, `GetValue`, `Bind`, `AddHandler`, `AddHandler<TEventArgs>`) are written a second time in `model_type_system.rs` | six signatures; the drift test compares them |
| synthetic `CompiledFerroXaml.!FerroResources` from `documents[]` | not in this type system | `CompiledMarkupTypeSystem` of the emitter adds it over any type system (9.7.3); it moves when `compile_xaml()` does |

*Rust type text to type (9.5.2).* `resolve(text)`: the text is made canonical, then it is a handle of the table, a handle a declaration states (`handles:`; `Ref<X>` and `Option<Ref<X>>` of a class; an enumeration), the optional form of a value type or an enumeration (`System.Nullable<T>`), the optional form of a handle of a reference type, and else an opaque type that carries the text.

*What it cannot know* (and the drift test lists): which declared types a crate registers; whether a `cfg` condition holds; the cast that makes a collection a list of its base (`ValueTypes::is_assignable`: the declared `base:` is taken at its word); bindable arrays registered at run time; which of two declarations of one name is looked up first. (Since 9.5.8 the registered classes and the casts are read from the registration functions; what is left is the `cfg` conditions, the classes that become known on first use, bindable arrays and the order of two declarations of one name.)

**The drift test** (`tests/FerroUI.Markup.Xaml.UnitTests/type_system_drift.rs`; the build crate must not link the controls, the XAML test crate links everything). It scans the base crate and the controls (with the model of the base crate), builds the `ModelTypeSystem`, and compares it with `RuntimeTypeSystem` over the registered types of the two crates, through the contracts: types by full name, kind, base, interfaces, attributes; properties, fields, methods, constructors and events by identity, with their types, shapes and attributes; the values of enumeration members. Every difference has a kind:

| Status | Kinds |
|---|---|
| must be empty (the test fails) | a type only at run time; a type only in the model (without a `cfg` condition); a member only at run time; a member only in the model |
| known, with the reason printed | a type under a `cfg` condition only in the model; a Rust type no metadata declares, spelled differently (`type_name` against the normalised text; compared without module paths first); an enumeration value the scanner did not evaluate; the base of a collection that declares an instantiation of the notifying list |
| open (the work list) | a Rust type one side maps to a type and the other does not; the kind of a type; base; interfaces; the type of a member; the shape of a member; attributes; an enumeration value |

Its output is the work list of the next stage: the "must be empty" lists are declarations the scanner misses or projection rules that differ, the "open" lists are handles to add to `core_handles` or rules to align. It was not run by its author; what it is expected to show first is in HANDOVER.md, section 12. The next stage worked the list off: 9.5.8 has the result, the kinds as they are now and the test as a guard.

#### 9.5.8 Implemented (2026-10-09): the two type systems aligned, third stage

Built and run by its author (three commands only: the tests of `ferroui-build`, the drift test, the tests of the run-time type system; the suites of the workspace are for the validating session, HANDOVER.md section 13). It works through the first result of the drift test (9.5.7): every entry was traced to its cause in the sources, and the cause was given to the scanner, the model or the `ModelTypeSystem` as a rule. No name is special-cased. The run-time type system is the reference and is unchanged.

**The drift test, before and after** (the base and the controls crates, 843 types on both sides):

| Kind | Before | After | Status |
|---|---:|---:|---|
| A type only at run time | 0 | 0 | must be empty |
| A type only in the model | 5 | 0 | must be empty |
| A member only at run time | 63 | 0 | must be empty |
| A member only in the model | 3 | 0 | must be empty |
| The kind of a type, its base, its interfaces, the type of a member, the shape of a member, the custom attributes, the value of an enumeration member | 0 each | 0 each | must be empty (was open) |
| A Rust type the run-time type system maps and the model does not | 42 | 0 | open |
| A Rust type the model maps and the run-time type system does not | 0 | 0 | open |
| An enumeration member whose value the scanner did not evaluate | 34 | 0 | known |
| The base of a collection declared as a notifying list | 1 | 0 | the kind is gone: compared as a base |
| A Rust type no metadata declares, spelled differently | 0 | 0 | known |
| A type under a `cfg` condition only in the model | 1 | 1 | known |
| A class its crate does not register | (among the 5 above) | 5 | known (new kind) |

The test is no longer ignored. A kind both type systems decide fails it; the two kinds of handles stay open (printed, not failing) for the one case that depends on the process, below.

**What each list was, and the rule that removed it.**

| Entries | Cause in the sources | Rule | Where |
|---|---|---|---|
| 52 members only at run time (`Animation`: 7 properties; `XYFocus`: 8 attached properties) | the accessors are written by a macro of the crate invoked among the members of `impl Owner` (`direct_cell_property!`, `override_property!`, `strategy_property!`); the scanner expanded such macros in item position only | an invocation of a macro of the crate among the members of an inherent `impl` block is expanded like one in item position; `ferro_property!` in its expansion is an accessor of the type of the block, or of the owner it states | `scanner/source.rs` (`read_impl`, `expand_local_macros`, `property_of`) |
| 8 members only at run time (`Application.ActualThemeVariant`, `RequestedThemeVariant`) | the owner is added through `ThemeVariant::.._property()`, an accessor `ferro_property!(for StyledElement; ..)` declares inside `impl ThemeVariant`; the model listed it under `StyledElement` without the type whose function it is | `RegisteredModel::function_of`: the Rust path of the type an accessor is a function of, when it is not the type it is listed under; `ModelSet::find_accessor` finds an accessor by the path of that type (and no longer by the path of the type it is only listed under); `ModelSet::accessor_type_path` is the path generated code calls it by (`MemberSource::Registered::type_path`) | `model.rs`, `model_set.rs`, `scanner/mod.rs` (`properties`, `locate`) |
| 3 members only at run time on `Visual`, the same 3 only in the model on `AdornerLayer` (`SavedAdornerLayer`) | the block of `AdornerLayer` (controls) registers `register_attached::<Visual, Visual, _>`: the registry lists a property under the owner the registration names, not under the type whose block has the accessor | a registered property is a member of the type its registration names as owner (`register::<Owner, _>`, and `add_owner::<Owner>`, which the model now states too), across the models of the set; a property whose owner no model has stays with the type it is listed under | `scanner/mod.rs` (`RegisteredModel::owner` of an added owner), `type_system/model_type_system.rs` (`Index::properties`, `project_class`) |
| 5 types only in the model | the classes are declared and are not in the `TYPES` list their `register_types()` hands to `TypeInfo::register_all` | the scanner reads `const _: &[&TypeInfo]` (the three forms the crates use: `types![Type, ..]`, `&[Type::TYPE]`, `&[<Type as StaticType>::TYPE]`); when a crate has such a list, a class or a static type that is not in it is `TypeModel::unregistered` (note `FRN9025`); the type system gives such a type its name and finds it neither by the name nor by `Ref<X>`; it stays the base of the classes that derive from it, as at run time. A crate without a list has no marked type | `scanner/source.rs` (`class_list_of`), `scanner/mod.rs` (`class_lists`), `model_type_system.rs` |
| 22 handles (`Option<AssignedBinding>` against `BindingBase`) | `markup_types::register()` of the controls calls `MarkupType::register_handle::<AssignedBinding>(<dyn BindingBase as MarkupTyped>::MARKUP)`: a handle of a type of another crate, stated in a function and in no declaration | the scanner reads the calls of `MarkupType::register_handle::<H>(..)` in the functions of the crate (the type as `<T as MarkupTyped>::MARKUP` or a variable of the function bound to it) into `AssemblyModel::handles`; the type system adds them after the declared handles | `scanner/source.rs` (`RegistrationFinder`), `model_type_system.rs` |
| 18 handles (`CommandBarElementList`, `PageList` against `FerroList<..>`) | `pub type PageList = FerroList<Ref<Page>>;`: the accessor names the alias, the handle is declared for the type | the scanner records the aliases without parameters whose type it resolves (`AssemblyModel::aliases`); `ModelSet::expanded` is the one text of a Rust type (canonical, every alias replaced by its type, through aliases of aliases), and the type system looks every handle up by it | `scanner/mod.rs` (`aliases_and_handles`), `model_set.rs`, `model_type_system.rs` |
| 2 handles (`IResourceDictionary.ThemeDictionaries`) | the type is written over several lines with a trailing comma (`FerroDictionary<K, V,>`), which was part of its text | a trailing comma of a list of type arguments is no part of a type text | `scanner/tokens.rs` (`pieces`) |
| 29 values of `Key` (`Enter`, `Capital`, `Oem1`, ..) | the members are associated constants of the enumeration (`pub const Enter: Key = Key::Return;`), not variants | a member of an enumeration that is no variant is the associated constant of that name of an inherent `impl` of the type, evaluated | `scanner/constants.rs`, `scanner/source.rs` (`member_value`) |
| 4 values of flags (`KeyModifiers.None`, ..) | `pub const NONE: KeyModifiers = KeyModifiers::empty();` next to the `bitflags!` invocation | the same rule; `empty()`, `all()`, `from_bits_retain(..)`, `from_bits_truncate(..)`, `.union(..)`, `.intersection(..)`, `.difference(..)`, `.bits()` are evaluated | the same |
| 1 value (`BindingPriority.Unset = i32::MAX`) | the discriminant is a constant of an integer type | a discriminant and a constant are constant expressions: integer literals, `-`, `\|`, `^`, `&`, `<<`, `>>`, `+`, `-`, `*`, parentheses, `as` to an integer type the value fits, `MIN` / `MAX` of the integer types, the other members of the type | `scanner/constants.rs` |
| 1 base (`Classes`, declared `base: FerroList<String>`) | the run-time type system keeps such a base only with a registered cast from the collection to the list, and every other collection has `ValueTypes::register_cast::<Collection, FerroList<..>>(..)` in a registration function | the scanner reads the calls of `ValueTypes::register_cast::<From, To>(..)` that state both types into `AssemblyModel::casts`; the type system keeps a declared list base only when the cast from the first handle of the collection to the base is among them (`ModelTypeSystem::is_cast`) | `scanner/source.rs`, `model_type_system.rs` (`project_markup_type`) |

A function of the registration (`register_assigned_binding`) imports the names it uses for itself (`use std::rc::Rc;` in its body); the types of its statements are resolved with those imports first (`LocalImports`), then as any type of the module.

**Added to the model.** The file stays format 2: every new member is left out when it is the default, so a file of the second stage is read unchanged.

| Member | What |
|---|---|
| `RegisteredModel::function_of` | the type whose function the accessor is, when it is not the type it is listed under |
| `RegisteredModel::owner` | now also the owner `add_owner::<Owner>` names |
| `TypeModel::unregistered` | the crate has a list of registered classes and the class is not in it |
| `AssemblyModel::aliases[AliasModel { path, target }]` | the type aliases without parameters whose type is resolved; an alias under a `cfg` condition or declared twice is left a name |
| `AssemblyModel::handles[HandleModel { handle, type_ }]` | the handles the registration functions add |
| `AssemblyModel::casts[CastModel { from, to }]` | the casts the registration functions state with both types |

Numbers of the two scans after the stage: `ferroui_base` 240 registered properties (225 before: 15 more from the macros in `impl` blocks), 0 of 825 enumeration members without a value (34 before), 1 class not registered, 136 casts, 52 aliases; `ferroui_controls` 935 registered properties, none without a name (2 before), 5 classes not registered (one of them under `cfg`), 2 registered handles, 46 casts, 26 aliases. The test of the real crates asserts zero nameless properties and zero unevaluated members again.

**What stays known, and why it cannot be decided from the sources.**

1. *A type under a `cfg` condition* (1: `UnitTestApplication`, `cfg(any(test, feature = "testing"))`). Whether the condition holds is decided by the build.
2. *A class its crate does not register* (5). The model and the type system now follow the list, so the two sides agree; the kind lists the classes because the run-time side is not stable for them: a class registers itself when it is first used (an instance is created, its properties are asked for), so in a process where another test used one the class is known at run time. The drift test does not compare such a class, and for the same reason the two kinds of handles do not fail it (a member typed `Ref<X>` of such a class is a handle only one side maps from then on).
3. *Opaque spellings* (0 now): `type_name` against the normalised text.

**Observations in the sources, not changed here** (the base and the controls crates are not edited by this stage).

1. Five classes are missing from the `TYPES` lists, against the rule stated at the top of both `register_types.rs` ("every `ferro_class!` and `ferro_static_type!` of the crate has an entry"): `GlyphRunDrawing` (base); `UnrealizedSelectionPeer`, `DecorationsOverlaysAutomationPeer`, `TopLevelHostAutomationPeer`, `NativeMenuItemPresenter` (controls). Markup cannot name them until an instance exists. Adding them to the lists makes the model follow without a change (the note `FRN9025` of the scan lists them).
2. `Visual.SavedAdornerLayer` exists at run time only once the properties of `AdornerLayer` are registered (the class of the controls registers a property of a class of the base crate): the members of `Visual` depend on which classes were initialised. The model has the property on `Visual` whenever the model of the controls is in the set.
3. The run-time type system as a whole is a function of the process (2. above and the registration of classes on first use). No defect of its projection rules was found: every difference was a declaration or a registration the build-time side did not read.

**What the scanner still does not read**, each guarded by the drift test (a case in the two crates fails it and names the entry):

1. The lists of the types with markup metadata a crate registers (`const TYPES: &[&MarkupType]`, `MarkupType::register` in functions and in the expansion of `ferro_markup_list!`): every declared type with markup metadata is taken as registered. All are, in the two crates.
2. The `also [..]` list of `ferro_properties!`: an accessor declared with `for Owner;` is taken as registered with the properties of its owner, in the order of the scan (the registry has the properties of the block first, then the listed functions). No comparison depends on the order.
3. A cast whose types the call leaves to inference (`register_cast::<A, _>`), `register_upcast` and `register_reference`: only a cast from a collection to its declared list base is used, and each of those states both types.
4. A macro of the crate whose rule repeats (`FRN9012`), an alias with parameters, a constant that is a function call or a constant of another type.

#### 9.5.9 Implemented (2026-10-09): the call forms, fourth stage

Built and run by its author (`cargo test -p ferroui-build --lib`, 49 tests). The choice of 9.5.3 is made for every callable of a model at the end of the scan (`src/FerroUI.Build.Tasks/call_forms.rs`, called from `Builder::finish` once the models of the dependencies are attached) and written into the model: `MemberModel::call`, `AccessorModel::call`, `TypeModel::parse_call`.

| Form | Chosen when | `CallForm` |
|---|---|---|
| A, structural | the callable is a path to a function a declaration macro writes for a type (the accessor of a routed event, `ferro_routed_event!`, or of a registered property), public, of a type a public path leads to; or the closure that dereferences what such a function returns, `\|\| *Button::click_event()`, which is the form every static field of a routed event is declared in | `Structural` |
| B, typed path | the callable is a path to a function of an inherent `impl` block the scanner read, the function takes as many arguments as the declaration macro passes, it is `pub`, and a public path leads to its type. The form holds the public path of the type and the name of the function | `Path("::crate::Type::function")` |
| B, within the crate | the same with a function that is `pub(crate)`: for code generated into the declaring crate; any other crate calls the member through the invoker (`CallForm::from_another_crate`) | `CratePath(..)` |
| C, invoker | everything else, with the reason counted: a closure; a path the scanner did not resolve; a path to no function of an inherent `impl` block (a free function, a function of a trait, a function a macro writes); a function that takes other arguments; a private function; a public function of a type no public path leads to | `Invoker` |

The registered properties, the members of enumerations and `new:` of a class have no callable to choose for; they are form A by their declaration (`MemberSource::Registered`, `MemberSource::DefaultConstructor`).

What the declaration macros pass to a callable, which is what "takes the arguments" is checked against (the instance by reference): a constructor its parameters; a method the instance and its parameters (a static one its parameters); a static field nothing; an event the instance and the handler; the getter of a property the instance, its setter the instance and the value (of a static property: nothing, and the value); the accessors of an indexer the instance and the indices, the setter the value after them; `Parse` the text. Only the number of arguments is compared: the types of the function are text as written and the declaration macro calls the callable with exactly the declared types, so rustc decides the rest at the emitted call.

**Added to the model** (format 2 unchanged: every new member is left out when it is the default).

| Member | What |
|---|---|
| `CallForm::CratePath` | form B for the declaring crate only |
| `CallableModel::dereferenced` | for a closure `\|\| *Type::function()`: the function, resolved and canonical |
| `TypeModel::parse_call` | the form of `parse:` |
| `AssemblyModel::functions[FunctionsModel { owner, public_path, functions[FunctionModel { name, receiver, parameters, declared_by }] }]` | the public functions of the inherent `impl` blocks of the types a public path leads to: what a callable of a crate built on this one is looked up in (the functions of a dependency are not in `Scan::functions`). Written as `[owner, public path, [[name, parameters, receiver?, macro?], ..]]` |
| `MemberSource::Declared::call` | the form, on the member of `ModelTypeSystem` |

**The numbers** (printed by `real_crates_are_scanned_without_skipping_a_declaration` in the line "call forms:" of each summary, and asserted to add up; `RUST_TEST_NOCAPTURE=1` shows them):

| Scan | Callables with a form | A | B | C | of C: closures | paths to no inherent function | public function, type without a public path |
|---|---:|---:|---:|---:|---:|---:|---:|
| `ferroui_base` | 1226 | 40 | 261 | 925 | 765 | 160 | 0 |
| `ferroui_controls` alone | 469 | 49 | 117 | 303 | 266 | 35 | 2 |
| `ferroui_controls` with the model of `ferroui_base` | 469 | 49 | 128 | 292 | 266 | 24 | 2 |
| `ferroui_markup_xaml` with both models | 187 | 0 | 68 | 119 | 84 | 35 | 0 |

No callable is unresolved, private, or of another number of arguments in the three crates. The callables without a form are `new:` of the classes and the sources of added owners (the difference to the "callables:" line). The 11 callables the controls gain with the model of the base crate are functions of types of the base crate, found in `AssemblyModel::functions` of that model.

**What the forms are not yet used for.** The emitter writes, for every declared member, the typed function the declaration macro generates with the feature `markup-functions` of the base crate (`MemberModel::typed_function`, `__markup_get_Child`), whatever the callable is; that is today's output and it does not change. The forms say what the emitter writes without the typed functions: a path for 29 % of the callables with a form in the three crates (A and B: 546 of 1882), the invoker for the rest. Whether the typed functions stay (they cover closures too, at the price of the feature) or the forms replace them is a decision for the stage that emits against the model; the model now holds both. The `--report-invokers` lint of 9.5.3 is the reason counts of the summary; a list per member is not printed.

#### 9.5.10 Started (2026-10-09): the emitter against the build-time type system, fifth stage

Stage 4 of 9.10.1 has two halves. The first is done and proven: the emitter no longer reads the run-time type system itself. The second, the implementation of that reading over the models, is not written; this section says what it has to answer and what the model lacks for it. The run-time type system stays the only host, so nothing is half switched. (Since then: the second half is written and proven over the corpus, 9.5.11 and 9.5.12. The table below is the list it worked off; "to write" in its last column is as the stage found it.)

**The seam (`rust_emitter/emit_types.rs`).** The emitter compared Rust types by `TypeId`, held classes as `&'static TypeInfo` and metadata as `&'static MarkupType`, downcast every member to `RuntimeMethod`, `RuntimeConstructor` and `RuntimeField`, and asked the registries of the process (`ValueTypes`, `TypeInfo::find_by_handle`, `MarkupType::find_by_handle`, `property_accessors`). All of it now goes through one trait, `EmitTypes`, and `rust_emitter/emitter.rs` names nothing of `crate::runtime::type_system` and nothing of the registries:

| The emitter asks | Through | At run time (`RuntimeEmitTypes`) | From the models (to write) |
|---|---|---|---|
| a Rust type, to compare the type of a value with the type a member declares | `TypeKey`: `Id(TypeId)` or `Text(&str)` | the `TypeId` | the normalised text with every alias expanded (`ModelSet::expanded`), borrowed from a table of the texts of the models built once |
| the Rust types it names itself (`String`, the numbers, `Option<BoxedValue>`, `Rc<dyn IServiceProvider>`, `&'static TypeInfo`, `ValueType`, ..: 31 of them) and their `Option<_>` | `known`, `option_of_known` | `TypeId::of` | the texts, by the declaring modules of the base crate |
| the class a type is; the class a handle is (`Ref<X>`, `Option<Ref<X>>`) | `class_of`, `class_by_handle`, `EmitClass` (name, public path, handle, default constructor, assignability) | `TypeInfo` | `TypeModel` with `object_model`, its base chain through `ModelSet` |
| the metadata of a type; the metadata a handle is declared by | `markup_of`, `metadata_of`, `markup_by_handle`, `EmitMarkup` (public path, whether it is a trait, handles, nullable form, interfaces, base, `this`, the type of a value, the members and the composition of flags) | `MarkupType` | `TypeModel`; the type of a value (`MarkupType::value`: the type itself, `Rc<dyn Trait>` for a contract) and the nullable form are not in the model |
| a method, a constructor, a static field as declared | `method`, `constructor`, `field` (`MethodInfo`, `ConstructorInfo`, `FieldInfo`: the typed function, whether it can fail, the declared Rust types of the parameters and of the result, whether the member is an accessor, whether the type system built it) | the `Runtime*` member and its `DeclaredMember` | `MemberSource` of the `Model*` member; the Rust types of the parameters and what kind of member the source is are in `MemberModel` / `PropertyModel` and are not carried to the member yet |
| a registered property and the expression of its definition | `EmitProperty`, `property_definition` | `FerroProperty`, `property_accessors` (the accessors of the type the property was resolved on and of its base types, then of the registering type and its base types; the first public one of a type with a public path) | `RegisteredModel` (`visibility`, `function_of`, `added_owners`), `ModelSet::accessor_type_path`; the order has to be the same |
| whether a value of one Rust type is a value of another through a registered cast | `is_assignable` | `ValueTypes::is_assignable`: the registered casts, the casts between the forms of a reference type, the handles of the classes, the interface handles a class declares, the nullable forms | `AssemblyModel::casts` has only `register_cast::<A, B>` with both types stated (9.5.8, item 3 of what the scanner does not read): `register_interface`, `register_upcast`, `register_reference`, `register_nullable`, `register_object`, `register_element_ref` are to read, and the rule to state once for both sides |
| the null of a nullable form; the inner type of one; the class of an element reference | `null_converts_to`, `nullable_inner`, `element_ref_class` | `ValueTypes::try_convert`, `nullable_inner`, `element_ref_class` | the same registrations |
| a styled element; a class with markup of its own; the paths of `Setter`, `SetterBase`, `StyleBase` | `is_styled_element`, `has_class_document`, `framework_path` | `StyledElement::TYPE`, `FerroRuntimeXamlLoader::has_class_document`, the recorded paths | the base chain; the class documents of the group and of `documents[]`; `TypeModel::named_path` |

A partial implementation over the models is not safe to land: a conversion `is_assignable` does not know does not always make a document not eligible: for the instance of a member the emitter then writes the conversion at run time (`rt::argument`) where it wrote the cast (`rt::cast`), which is other output without a diagnostic. The implementation has to answer every question or the document has to be refused; that is why none was started.

The descriptions `RuntimeEmitTypes` hands out (`RuntimeClass`, `RuntimeMarkup`, `RuntimeEmitProperty`) are one per class, metadata and property of the process, kept for the life of the thread like the registries they describe (`TypeInfo::rust_path` takes `&'static self`, so a description holds the static reference).

**The transform (`rust_emitter/transform.rs`).** `transform_group(type_system, sources, options, parsers)` parses and transforms a group against any `IXamlTypeSystem` with the sequence of the run-time loader, and nothing is interpreted: no interpreter is created, the namespace information of a document is read from the document (`XmlNamespaceInfoProvider`) instead of from a context, the base URI is the text given. `FerroXamlIlRuntimeCompiler::transform_documents` and `transform_document` are gone; the run-time loader keeps its own `transform_group` with its back end. `EmitterHost { type_system, types, parsers }` is what a group is compiled against; `compile_documents_with` and `generate_file_with` take one, and `compile_documents` / `generate_file` are those with `EmitterHost::runtime(dependencies)`. `generate_class_file` still reads the documents of a class from the run-time loader (`FerroRuntimeXamlLoader::class_document`, `document_group_without`) and is not host-neutral yet: the build path needs the same from files (the `x:Class` directive of a document and the documents it includes).

**The proof** (the emitted text is unchanged by a byte; each is a test that compares the emitter's output with a checked-in or a build-script file, run by the author after each of the two changes):

| Output | Test | Result |
|---|---|---|
| the corpus, 109 documents, `tests/FerroUI.Markup.Xaml.UnitTests/emitter/generated.rs` | `emitter::differential_tests::generated_output_is_up_to_date` (in `cargo test -p ferroui-markup-xaml-tests --lib`: 575 pass, 15 ignored) | same |
| the Fluent theme, `compiled_xaml.rs` | `cargo test -p ferroui-themes-fluent --lib tests::compiled_xaml_tests` | same |
| the Simple theme, `compiled_xaml.rs` | `cargo test -p ferroui-themes-simple --lib tests::compiled_xaml_tests` | same |
| the include fixture, the library and the application (build script against the emitter in the test; `StyleWithServiceProvider` checked in) | `cargo test -p xaml-include-fixture-theme --lib` (4), `cargo test -p xaml-include-fixture-application --lib` (28) | same |
| the emitter's own tests | `cargo test -p ferroui-markup-xaml-loader --lib` (397 pass, 1 ignored) | pass |

The differential test the stage asks for (both paths over the corpus, outputs compared) has nothing to compare until the second implementation exists. What exists is its first half, below.

**The transform against the models, measured** (`tests/FerroUI.Markup.Xaml.UnitTests/emitter/model_transform.rs`, `corpus_is_transformed_against_the_model_type_system`). Every document of the corpus is transformed twice by `transform_group`, against the run-time type system and against `ModelTypeSystem` over the scans of `ferroui_base`, `ferroui_controls` and `ferroui_markup_xaml`, and the transformed trees are compared as text. First result: **0 of 109**. All 109 stop at one place, before a document is read: the configuration of the language (`FerroXamlIlWellKnownTypes`) does not find the constructor `XamlSourceInfo(i32, i32, Option<String>)` on the type of the model (`Constructor with arguments System.Int32, System.Int32, System.String is not found on type FerroUI.Markup.Xaml.Diagnostics.XamlSourceInfo`). `ferroui_markup_xaml` was never in the drift test (9.5.8), and its scan has one declaration it does not read (`FRN9010`; the test prints it). So the next step is the one 9.5.8 already named: add the crate to `CRATES` of `type_system_drift` and empty its lists; the measurement then moves from the configuration to the documents. The test fails when fewer documents than its constant `IDENTICAL` (0 now) come out the same, so the number only grows.

The scan of `ferroui_markup_xaml` with the models of the two crates: 63 files, 50 types (5 static types, 45 markup types), 1 registered property, 39 constructors, 51 plain properties, 59 methods, 2 events, 468 type texts with none unresolved, 50 stated public paths all found, 2 casts, 8 aliases, 44 `ferro_markup_type!` invocations of which 1 is not read.

**What the build crate links (found while reading, not changed).** `ferroui-build` depends on `ferroui-markup-xaml` (and on the loader with the `runtime` feature, which depends on it), and `ferroui-markup-xaml` depends on `ferroui-controls` (`Cargo.lock`: `ferroui-markup-xaml` lists `ferroui-controls`). So a build script that uses `ferroui-build` builds the controls for the host today, whatever its documents name, and the rule of 9.6.1 and decision 10 ("it never links `ferroui-controls`") holds for the direct dependencies only. Two consequences for the stages that remain:

1. `compile_xaml()` on the model does not make the build script free of the controls by itself. The emitter (`rust_emitter`) is behind the `emitter` feature, which enables `runtime`, and it uses `runtime::interpreter` (`plan_setters`, `numeric_constant`, `context_definition`), `runtime::framework` (`adapt_type_mappings`, `RuntimeDocumentTypeBuilderProvider`) and `ferroui_markup_xaml` (`FRAMEWORK_CONTEXT`, `RuntimeXamlLoaderConfiguration`). Either those few items move out of the `runtime` feature (none of them needs a registered type), or `ferroui-markup-xaml` stops depending on the controls.
2. The cycle of 9.5.7 is not the base crate's alone: `ferroui-controls` cannot take `ferroui-build` as a build dependency either. The leaf crate (`ferroui-build-scan`) is needed by both.

**Stage 5 of 9.10.1 (`compile_xaml()` on the model) is not started**, for the reason above: it compiles with the emitter against the models, which is the half that is not written. Nothing of it was done ahead (no leaf crate, no `build.rs` in the framework crates), because without a consumer the framework crates would pay for a scan on every build and export a file nothing reads. What the switch of the framework crates costs a clean build, in steps, for the session that makes it to time:

1. `proc-macro2`, `quote` and `syn` (with `full`) built for the host, once per build; the leaf crate on top of them (the scanner, the model, the JSON reader and writer: about 8 thousand lines).
2. Per framework crate, its build script compiled and linked for the host (unoptimised unless the profile says otherwise), then run: the scan of its sources (1358 files for the base crate, 608 for the controls, 63 for the XAML runtime library), preceded by reading the `.xamlmeta` of every crate below it (the controls read the model of the base crate, the runtime library both), and the model written (`write_if_changed`).
3. `cargo::rerun-if-changed` for every file of the scan: the script runs again on any source edit of its crate, and the crates above it are rebuilt only if the file it writes changed. The scan is not cached (9.6.4).
4. Per crate that compiles documents, the models of everything below it read again (transitively, from the paths in each file) and one `ModelTypeSystem` built.

For scale only, not a measurement of a build script: the unoptimised tests of `ferroui-build`, dominated by the one that scans the base crate once and the controls twice, writes both models and reads them back, run in 8 to 11 seconds on the author's machine.

#### 9.5.11 Implemented (2026-10-09): the XAML runtime library in the drift test, and the transform agreeing on the corpus

Built and run by its author (the seven commands of 9.5.10's proof, each as written). It does the first two steps 9.5.10 named: `ferroui_markup_xaml` in the drift test, and the transform of the corpus agreeing against both type systems.

**The drift test with the third crate** (`type_system_drift`: the base crate, the controls on its model, the XAML runtime library on both models; `ferroui_markup` declares no type outside its tests and is not a crate of the comparison):

| Kind | Before the fixes | After | Status |
|---|---:|---:|---|
| Types on both sides, compared member by member | 893 at run time, 899 in the model | the same | |
| A type or a member only one side has; the kind, the base, the interfaces, the type or the shape of a member, the attributes, the value of an enumeration member | 0 each | 0 each | must be empty |
| A Rust type the run-time type system maps and the model does not | 35 | 0 | open |
| A Rust type the model maps and the run-time type system does not | 0 | 0 | open |
| A type under a `cfg` condition only in the model | 1 | 1 | known |
| A class its crate does not register | 5 | 5 | known |

The scan of the crate before the fixes had one declaration it did not read (`FRN9010`), which was the whole of the first measurement of the transform (0 of 109).

| What was wrong | Cause in the sources | Rule | Where |
|---|---|---|---|
| The metadata of `XamlSourceInfo` not read, so no constructor of it in the model | a callable that is a closure with a stated return type (`\|obj, info\| -> Result<(), XamlLoadException> { .. }`): the comma of `Result<(), E>` ended the expression | a closure that states its return type has a block for its body: the expression runs up to the block | `scanner/tokens.rs` (`take_expression`) |
| 35 handles (the members `CanConvertFrom`, `ConvertFrom` of the seven type converters) | a macro of the crate (`type_converter_markup!`) writes the types by their full path so that nothing of the invoking module shadows them: `::std::option::Option<::std::rc::Rc<dyn ..>>`, and `Option<T>` is a name of the prelude in every other declaration | a path of the standard library to a name of the prelude is the name (`option::Option`, `string::String`, `vec::Vec`, `boxed::Box`, `result::Result` below `std`, `core` or `alloc`) | `scanner/modules.rs` (`PRELUDE_PATHS`, `Modules::absolute`) |
| The crate whose documents are compiled (the XAML test crate) not scanned: `emitter/generated.rs` refused by the parser | `break 'label ::path::function(..)`, which the emitter writes for a block with a value: `syn` takes the first colon of the path for the colon of a labelled expression (`expr_break` peeks `:`) and reports "expected loop or block expression"; rustc reads the value | a file the parser refuses is read again with the value of every such `break` in parentheses (the tokens keep their lines); if that does not parse either, the file is not read (`FRN9001`) as before | `scanner/tokens.rs` (`with_parenthesised_break_values`), `scanner/source.rs` |

One warning stays in the scan of the crate and is no difference: `FRN9022` for `XamlSourceInfo`, declared as a static type (`ferro_static_type!`) and as a type with values (`ferro_markup_type!(class ..)`) in one module; both type systems make the two one type.

**The transform on the corpus** (`emitter/model_transform.rs`): every document is transformed against the run-time type system and against `ModelTypeSystem` over four models: the three framework crates and the crate that is compiled (the test crate, scanned without its test code as its build script would scan it, on the three models; one document of the corpus names a class of it). Result: **109 of 109** transformed trees are the same text; the test asserts all of them (`IDENTICAL = 109`), and fails when a scan does not read a declaration. The helper `scanned_models()` is what the emit test below uses too.

The compile-time value parser of the host is still not given to the transform against the models (the run-time host has `RuntimeCompileTimeValueParser`, which calls the `parse:` function of the type): the trees agree without it for every document of the corpus, because the built-in grammar gives the same value for valid text. What differs is text a type refuses: the run-time host reports it when the group is transformed, the build-time host would emit a call that fails when the document is built. That is the parser of 9.6.1, still to write.

#### 9.5.12 Implemented (2026-10-09): `EmitTypes` over the models

Built and run by its author with the same seven commands. The second half of stage 4 of 9.10.1: the emitter's questions (the table of 9.5.10) answered from the models, and the differential of the two implementations over the corpus. **The generated Rust of all 109 documents is the same byte for byte through both, and so is the whole generated file with its position map; no document is refused.** The run-time type system stays the only host of `Build::compile_xaml()`; the model path is used by the tests only.

**Where it lives.** `ModelEmitTypes` is in `ferroui-build` (`src/FerroUI.Build.Tasks/type_system/emit_types.rs`), next to `ModelTypeSystem`. The direction is forced: the trait `EmitTypes` and the emitter are in the loader, the models and the scanner are in `ferroui-build`, and `ferroui-build` depends on the loader (for the emitter and the table of runtime library types), so the loader cannot name the model. A host builds `EmitterHost { type_system: system.as_type_system(), types: &ModelEmitTypes::new(system, class_documents), parsers }` and calls `compile_documents_with` / `generate_file_with`.

**How each question is answered.**

| Question (9.5.10) | Answer from the models |
|---|---|
| a Rust type | `TypeKey::Text`: the one text of the type (`ModelSet::expanded`: canonical, aliases expanded), borrowed from a table built once from every type text of the models, the table of runtime library types and what the registrations imply. A text the table lacks is a question that is not answered |
| the 31 known types and their `Option<_>` | their texts by the paths the base crate and the XAML runtime library export them by, made canonical through the export tables (`KNOWN`) |
| `class_of`, `class_by_handle`, `EmitClass` | a `TypeModel` with `object_model`: its name, its public path (none for a class its crate does not register), `Ref<Path>` for a class and nothing for a static type, `new:`, and its chain of base classes; found by `Ref<X>` / `Option<Ref<X>>` only when the crate registers it |
| `markup_of`, `metadata_of`, `markup_by_handle`, `EmitMarkup` | the declaration, by the rules of the declaration macros: the handles as listed (`Ref<X>`, `Option<Ref<X>>` for the metadata of a class; the type for an enumeration); the nullable form `Option<T>` of a value type and of an enumeration; `this:`, else the declared type, nothing for a contract; the value `this:`, else the first handle, else `Rc<dyn Trait>` of a contract, else the type; the base of the metadata of a class is the metadata of the nearest base class that states some, of any other type the metadata its `base:` is a handle of; the members and the composition of a set of flags from the evaluated values. The metadata of a class is not found by a handle (it is not in the registry of types with markup metadata); a static type with `type_info:` is |
| `handle_of` | the first handle of the type; of `System.Type` the type the run-time loader holds a type value in, which no declaration names |
| `method`, `constructor`, `field` | `MemberRust` of the member (below) and its `MemberSource`: the typed function and whether it fails, the declared Rust types of the parameters, the type a getter, a method or `Parse` returns, the registered property of a definition field |
| `EmitProperty`, `property_definition` | one description per registration a static field holds; its identity is the declaration, or the added owner of a direct property (`DirectProperty::add_owner` makes a definition of its own). The definition: the accessors listed under the type the property was resolved on and under its base classes, then under the registering type and its base classes; of those that return the identity, the first that is `pub` and a function of a type with a public path |
| `is_assignable`, `null_converts_to`, `nullable_inner`, `element_ref_class` | `ValueTypes::is_assignable` rule by rule over what the models state (below) |
| `is_styled_element`, `has_class_document`, `framework_path`, `primitive_type_name` | the chain of base classes; the classes the host names as having markup of their own; the public paths of `Setter`, `SetterBase`, `StyleBase`; the names of the primitive types |

**Added to the model and the scanner** (format 2 unchanged: every member is left out when it is the default).

| Member | What | Read from |
|---|---|---|
| `MemberRust` on `ModelMethod`, `ModelConstructor`, `ModelField` (`parameters`, `declared: DeclaredRust`, `registered`) | the Rust side of a member: the one text of each declared parameter type (nothing for a parameter no declaration types: an event handler, a member the type system builds), what the member is in its declaration with the type it yields, and for a definition field the registration | the `MemberModel` / `PropertyModel` / `RegisteredModel` the member is projected from, as `runtime_type_system.rs` sets `parameter_handles` and `declared` |
| `TypeModel::class_markup` | the class states `markup: { .. }` in a `ferro_class_info!`: its runtime type has metadata | the declaration |
| `TypeModel::stated_path` | the path of an instantiation of a generic type, which no rule of public paths gives: the name the crate has for the generic type with the public paths of its arguments (`::ferroui_controls::FerroListOf<::ferroui_base::Ref<::ferroui_controls::RowDefinition>>`) | `generics:` and `generic_contracts:` of `ferro_rust_paths!`, which state it |
| `AssemblyModel::value_types[ValueTypeModel { registration, types }]` | the registrations with the untyped value conversions besides the casts: `nullable`, `reference`, `object`, `element_ref`, `interface`, `upcast`, each with its type arguments | `ValueTypes::register_<registration>::<Types>` with every type stated: called in a function; handed to a function that calls it (`register_deferred(ValueTypes::register_object::<T>)`); in a closure that is the value of a constant or a static (`value_types: \|\| { .. }`); in the body of a declaration (the accessor of a property that registers the element reference of its value type); in the expansion of a macro a function invokes (next row) |
| `AssemblyModel::casts` | now also the casts of a table written with a macro | a function invokes a macro of the crate or of its own body whose rule the scanner applies (identifiers, types, expressions, literal tokens; several rules; a rule that invokes the macro again; a macro that is handed the name of another and invokes it: `for_each_assignability!(assignable)`): the expansion is parsed as a block and read like the body, to a depth of 8 |
| `AssemblyModel::unread_value_types[(registration, count)]` | for each registration function, the calls the text of the crate has and the model does not | every `ValueTypes::register_*` in the tokens of the files (outside `#[cfg(test)]` items and outside the definition of a declaration macro, whose registrations are read as declarations), against the calls read. The calls in the definition of a macro count as read when every invocation of it that was met is expanded |

Numbers of the four scans:

| Crate | Registrations read (casts and the others) | Not read |
|---|---:|---|
| `ferroui_base` | 425 | 2 `register_nullable`, 1 `register_object`, 7 `register_reference` |
| `ferroui_controls` | 177 | 1 `register_nullable` |
| `ferroui_markup_xaml` | 115 (43 casts, all of the table `for_each_assignability!`; 72 nullable forms) | 1 `register_reference` (`references![..]`, a rule that repeats) |
| `ferroui_markup_xaml_tests` | 46 | none |

What is not read is generic code that registers the type it is called for (`register_reference::<S>()` of a binding source, `register_object::<T>` of the class initialisation), a macro whose rule repeats, and test helpers that are not under `#[cfg(test)]`.

**Assignability, and what the models cannot know.** The untyped value conversions are a table of the process, filled as classes are initialised and as generic code meets types. `ModelEmitTypes` states the complete table as far as the sources state it: every class is an object with its nullable handle (the rule "a class registers its handle when it is initialised"), a class with `interfaces:` converts to them, and the registrations read. A *yes* follows from that (`stated_assignable`, the rules of `ValueTypes::is_assignable` in their order: identity, the untyped value, a cast, a cast between the forms of a shared type, two classes, a contract of a class, the nullable forms). A *no* is answered only when no registration that is not read could make it a yes (`decided_not_assignable`); each registration function makes casts of one shape, so most pairs are decided whatever is not read:

| Not read | What it could make assignable | A pair is not decided when |
|---|---|---|
| `register_cast` | any two types | always: with one cast not read, no *no* is answered |
| `register_reference::<T>` | `T`, `Rc<T>`, `Option<Rc<T>>` among each other; a cast stated for one form, for the others | the target is the other form of the source (or its `Option`), or a cast is stated between the other forms of the two |
| `register_interface::<T, I>` | the handle of a class to `I` | the source is the handle of a class |
| `register_element_ref::<T>` | `Ref<T>` and `ElementRef<T>` and their nullable forms | one of the two is an element reference |
| `register_nullable::<T>` (and every registration that makes a nullable form) | `Option<T>` assigns as `T` | the type the form would hold is assignable, or is itself not decided |
| `register_object`, `register_upcast` | nothing new: they take classes, whose handles the models have in full | never |

A question that is not decided is answered *no*, recorded with the two types and the reason, and handed to the host by `EmitTypes::take_unanswered` (a default method of the trait; the run-time type system records nothing). `compile_documents_with` asks before and after it emits a document and refuses the document with what was recorded ("the type system of the host cannot answer: whether a value of `A` is a value of `B`: ..."), whatever the emitter wrote from the answer. That is the one change to the loader in this stage; its output for the run-time host is unchanged.

**The proof** (`tests/FerroUI.Markup.Xaml.UnitTests/emitter/model_emit.rs`).

| Test | What it shows |
|---|---|
| `corpus_is_emitted_the_same_against_the_model_type_system` | the corpus compiled as one group through both hosts: 109 the same (source, namespace table, root type, visibility), 0 different, 0 refused (`REFUSED` is empty and asserted), and the whole generated file and its position map are the same text |
| `a_document_the_models_cannot_answer_for_is_refused` | with one cast of the XAML runtime library marked as not read, the nine documents whose code depends on a cast that does not exist (the instance of `ResourceDictionary.AddDeferred` and of `InlineCollection.Add` converted at run time) are refused with the two types named, the other 100 are the same, none is different |

The first run of the differential, before the registrations beyond the casts were read, was 98 the same, 11 refused, 0 different: ten refused for a *no* that could not be proven, one for the path of an instantiation of a generic type. Nothing was ever emitted differently.

**What remains, and what is doubted.**

1. *The order of the accessors within a type.* The run-time table has the accessors of a `ferro_properties!` block first and the functions of its `also [..]` list after them; the model has them in the order of the scan. No document of the corpus chooses between two public accessors of one definition under one type. A drift test of the descriptions themselves (every class, metadata and property of the three crates through both `EmitTypes`, as `type_system_drift` compares the contracts) would decide this and the next two points for every type, not only the ones the corpus names.
2. *The accessors listed under a type.* The model lists an accessor under the type of its block, or the owner `for Owner;` states, which is what the declaration macro records. An accessor a crate declares for a type of another crate is listed in the model of the first crate under a type it does not have; none exists in the four crates.
3. *The process.* The run-time answers depend on which classes were initialised and which types generic code has met; the models state the complete table. The two agree on the corpus in the order the test crate runs its tests, and `output_does_not_depend_on_what_ran_before` holds for the run-time side. A document for which they differ would show as "different" in the differential, never silently.
4. *The reasons.* A reason the emitter gives for a document that is not eligible still says "of the run-time type system" (9.5.10, doubt 1): no document of the corpus is not eligible.
5. *The compile-time value parser of the build* (9.6.1), see 9.5.11.
6. *The themes and the fixture through the model path* were not measured: the differential is over the corpus. The 165 theme documents and the documents of the include fixture need the models of the theme crates and the compiled documents of dependencies (`CompiledMarkupTypeSystem` over `ModelTypeSystem`), which is the first work of the next stage.

**`compile_xaml()` on the model (stage 5 of 9.10.1) is not started.** *(Done since: 9.5.13.)* `EmitTypes` over the models is whole for the corpus, so nothing blocks it in the emitter any more. What it involves, in order:

1. *The models of the framework crates in a build script.* `Build` has to hand `ModelTypeSystem` the models of `ferroui_base`, `ferroui_controls` and `ferroui_markup_xaml`. They are not exported: the crates have no build script (9.5.7). Two ways: the leaf crate and the build scripts of 9.5.7 and 9.5.10 (the scanner out of `ferroui-build` so that the base crate and the controls can scan themselves, `links` in each), or, as an interim that touches no framework crate, an explicit option of `Build` that names the source roots of the crates to scan in place of their `.xamlmeta` (the tests do this with paths relative to the workspace; a build script outside the workspace has no such paths, so it is a fixture-only option). For scale: the two tests of `model_emit.rs` together scan the four crates twice and emit the corpus four times in about 8 seconds, unoptimised.
2. *The class file from files.* `generate_class_file` reads the document of a class and the documents it includes from the run-time loader (`FerroRuntimeXamlLoader::class_document`, `document_group_without`) and picks the constructor from `RuntimeConstructor`s (`ClassConstructor::of`). The model path needs the same from the files of the crate: the `x:Class` of each document (the directive names the class, `ModelTypeSystem` finds its type, `DocumentSource::root_type` takes it), the group, the constructor from the constructors the model projects (`MemberRust::parameters` has the Rust type of the single parameter), and `has_class_document` from the directives of the group and the `documents[]` of the dependencies.
3. *The compiled documents of dependencies.* `CompiledMarkupTypeSystem` wraps any `IXamlTypeSystem` and its `Build:<path>` methods are not `ModelMethod`s; the emitter reads them through `CompiledDocumentBuildMethod` already and asks `class_of` for the type they return, which `ModelEmitTypes` answers for a `ModelType`. `EmitTypes::is_own` for the types of the wrapper needs a look.
4. *The option.* `Build::compile_xaml()` keeps the run-time host; an explicit call (`type_system(TypeSystem::Models)` or the like) selects the models. `StyleWithServiceProvider` of the include fixture is the first class document to compile through it; the checked-in file is the differential.

#### 9.5.13 Implemented (2026-10-09): `compile_xaml()` on the build-time type system

Built and run by its author with the commands the owner allowed (HANDOVER.md, section 16). Stage 5 of 9.10.1 and the part of stage 6 it needs: a build compiles the documents of its own crate, the documents of its classes among them, against the type models, and links no crate for its types. The four things 9.5.12 named are done in its order. **The include fixture compiles every group of both crates through the models, the Simple theme compiles its 82 documents through the models, and each output is the same byte for byte as the emitter's against the run-time type system; no document is refused.** The run-time type system stays the default of `Build`: the Fluent theme, the dialogs and the catalog are not converted.

**The crate layout after the move.**

| Crate | Directory | Depends on | Holds |
|---|---|---|---|
| `ferroui-build-scan` (new) | `src/FerroUI.Build.Scan` | the standard library, `syn`, `proc-macro2`, `quote` | `model` (the type model, `.xamlmeta` format 2), `xaml_metadata` (`XamlMetadata`, `DocumentModel`: format 1, moved from the loader), `scanner`, `model_set`, `call_forms`, `json` (private), `export` (new), the fixtures of the scanner (`tests/fixtures`), and the two texts of the base crate the scanner reads an assembly with (`FERRO_XML_NAMESPACE`, `CREATE_SOURCE_INFO`; a test of `ferroui-build` compares them with the base crate's) |
| `ferroui-build` | `src/FerroUI.Build.Tasks` | the leaf crate, the loader (`runtime`, `emitter`), `ferroui-base`, `ferroui-markup-xaml`, `xamlx` | `Build`, `XamlGroup`, `TypeSystem`, `type_system` (`ModelTypeSystem`, `ModelEmitTypes`); `pub use ferroui_build_scan::{call_forms, export, model, model_set, scanner}`, so every public path it had still names the same item |
| `ferroui-markup-xaml-loader` | unchanged | with the `emitter` feature also the leaf crate | `rust_emitter::{XamlMetadata, DocumentModel}` are the leaf crate's, exported under the names they had |

The move is the files as they were (`git mv`), with three lines changed: the two constants and the path `XamlMetadata` is taken from.

**The framework crates export their models.** `ferroui-base`, `ferroui-controls` and `ferroui-markup-xaml` each have `links = "<crate>_xaml"`, a build dependency on the leaf crate and a `build.rs` of one line, `ferroui_build_scan::export::Export::from_env().run()`: the scan of the crate with the models of the crates below it (`DEP_<CRATE>_XAML_XAMLMETA`), written to `$OUT_DIR/<crate>.xamlmeta`. `ferroui-dialogs` exports the same way (its existing build script calls the export first), and the Fluent theme exports its model together with the documents of its checked-in `compiled_xaml.xamlmeta` (`Export::compiled_markup`), so that one file per crate carries the documents and the types.

| What | How |
|---|---|
| Deterministic | the text of a model is a function of the sources and of the models below: the scan reads no environment, records a `cfg` condition and never evaluates it, and writes in the order of the sources. A test runs the export twice and compares |
| When the script runs again | `cargo::rerun-if-changed` for `build.rs`, `Cargo.toml`, every source file the scan read and every `.xamlmeta` it read, and nothing else. A new source file is reached through the `mod` item of a file that changed |
| What the crates above see | the file is written only when its text changed (`write_if_changed`), so an edit that changes no declaration does not run the build scripts of the dependents |
| What a run cost | `$OUT_DIR/xamlmeta-scan.txt` (`export::ScanReport`): the models read, the source files, the types, the time of each step, the size of the model and whether it was written. Nothing is printed; a declaration the scanner does not read is a `cargo::warning=` (none in the framework crates) |
| A cross build | the build script is built and run for the host whatever the target, and the scan does not depend on the target: a `wasm32` build of the framework builds `syn`, `proc-macro2`, `quote` and the leaf crate for the host once and exports the same files. The author could not run the browser build |

The cost, read from the files of one clean build on the author's machine (unoptimised build scripts, the machine compiling other crates at the same time; not a measurement):

| Crate | Source files | Types | Models read | Scan | Model |
|---|---:|---:|---:|---:|---:|
| `ferroui_base` | 1358 | 442 | 0 | 4.19 s | 1.86 MB |
| `ferroui_controls` | 608 | 408 | 1 in 0.03 s | 2.05 s | 1.47 MB |
| `ferroui_markup_xaml` | 63 | 51 | 2 in 0.05 s | 0.23 s | 0.19 MB |
| `ferroui_dialogs` | 24 | 10 | 3 in 0.06 s | 0.11 s | 0.05 MB |
| `ferroui_themes_fluent` (with its checked-in `compiled_xaml.rs` as a module) | 10 | 5 | 3 in 0.05 s | 5.07 s | 0.04 MB |
| `ferroui_themes_simple` (as it was, with `compiled_xaml.rs` as a module) | 6 | 1 | 3 in 0.05 s | 2.27 s | |
| `ferroui_themes_simple` (converted: the generated file is in `OUT_DIR`) | 5 | 1 | 4 in 0.08 s | 0.05 s | |
| `xaml_include_fixture_theme` | 5 | 2 | 3 in 0.08 s | 0.05 s | 0.01 MB |

Read as: a source edit of the base crate costs about four seconds of scan before its compilation starts, of the controls two; the crates above do not run their scripts unless a declaration changed. A theme with checked-in compiled markup pays for the scanner parsing the generated file (35 thousand lines for the Simple theme, twice where the parser needs the `break` values in parentheses, 9.5.11): that cost goes with the conversion of the theme, as the Simple theme shows. Two ways to lower the rest, neither taken here: `[profile.dev.package.ferroui-build-scan] opt-level = 3` (and the same for `syn` and `proc-macro2`) in the workspace manifest, and the cache of 9.6.4.

**The option.** `Build::type_system(TypeSystem::Model)`; `TypeSystem::Runtime` is the default and is unchanged. With the models a build

1. reads the models of the dependencies (`ModelSet::read` over `DEP_*_XAMLMETA`, transitively) and scans the crate (as `export_metadata()` does; the model is written into the `.xamlmeta` of the crate either way), with `rerun-if-changed` for every file;
2. takes the assembly from the scan: no `Build::assembly()` and no `MarkupAssembly` in the build script. `CreateSourceInfo` of a group defaults to the metadata of the assembly as the configuration of the run-time loader reads it;
3. builds one `ModelTypeSystem` over the models of the dependencies and the model of the crate, one `ModelEmitTypes`, and `CompiledMarkupTypeSystem` over them for the compiled documents of the dependencies (the rule of `EmitterHost::runtime`: whenever there is a dependency);
4. compiles each group with `generate_file_with`, or `generate_class_file_with` for the group of a class;
5. registers nothing: no `register_types()`, no application services, no asset loader.

A refused document (`EmitTypes::take_unanswered`), a document that is not eligible and a class the models do not have are each one `cargo::error=<document>: <reason>` line; every group is compiled before the build fails. A build with errors writes no `.xamlmeta` and no `mod.rs`.

**The class file from files (item 2 of 9.5.12).** `XamlGroup::class_document(name)` makes a group the group of a class of the crate, and `XamlGroup::constructor` states its constructor where the rule would pick another (the themes). In the loader, `generate_class_file` is now the run-time host of `generate_class_file_with(host, ClassGroup, options)`:

| `generate_class_file` asked the run-time loader | `generate_class_file_with` takes |
|---|---|
| the document of the class (`FerroRuntimeXamlLoader::class_document`) | the group states it; `class_of_document(xaml)` reads `x:Class` of the root as the parser of the compiler reads it, and the type system of the host finds the type by that name |
| the group (`document_group_without`: the assets the includes reach) | `class_document_group(root_uri, documents, class_document)`: the same walk (`include_sources`, the URI of each source resolved against the including document, each document once, in the order met) over the documents of the crate; a document of another crate is not among them |
| the class as `&'static TypeInfo` (its name, its path, `class_of`) | `EmitTypes::class_of` of the type: `EmitClass` |
| the constructor from `RuntimeConstructor`s (`ClassConstructor::of`) | the same rule over the contracts and `EmitTypes::constructor` (the parameter handle compared with `Known::OptionServiceProvider`, the typed function of the declaration); the function is owned text inside the loader, `ClassConstructor` is unchanged for its callers |
| nothing | the questions the type system could not answer for a document, which make the class not eligible |

The file of a class starts with attributes of its module, so `mod.rs` declares it as a module file (`#[path = "<OUT_DIR>/xaml/<module>.rs"] pub mod <module>;`) where the file of any other group is included into a module block. It has no position map (`ClassFile` has none).

**The compiled documents of dependencies (item 3).** `CompiledMarkupTypeSystem` over `ModelTypeSystem` needed no change: its build methods are read through `CompiledDocumentBuildMethod`, their return types are types of the models, and the synthetic resources type never reaches `EmitTypes`. What it needs is the type of every compiled document of a dependency in the models, which is why a crate whose documents are included exports its model with them (the themes).

**`has_class_document`.** The build hands `ModelEmitTypes` no class "with markup of its own". At run time the question is whether the run-time loader populates the class (`FerroRuntimeXamlLoader::register_class_document`), in which case the emitter does not write `T::new()` for it; in a build every class document is compiled and the constructor of the class populates the instance from its compiled markup. It is also what the run-time host of a build script answered (nothing is registered there).

**The proof.**

| What | Test | Result |
|---|---|---|
| the library of the include fixture: `compiled_xaml` (the documents without a class), `compiled_xaml_source_info` (with `CreateSourceInfo`), `compiled_style_with_service_provider` (the document of a class of the crate) | `cargo test -p xaml-include-fixture-theme --lib`: `build_script_output_is_the_emitters` compares the three files of the build (the models) with the emitter run in the test (the run-time type system, every type registered), and the documents of the `.xamlmeta` | the same bytes; 3 tests pass. The checked-in `compiled_style_with_service_provider.rs`, its `.xamlmeta` and the regeneration test are deleted; before they were, the refactored `generate_class_file` gave the checked-in text |
| the application: `compiled_xaml` and `compiled_xaml_source_info`, which include documents of the library and of both themes and name types of them | `cargo test -p xaml-include-fixture-application --lib` | the same bytes; 28 tests pass (upstream's include tests run against the build's output) |
| the Simple theme: the document of `SimpleTheme` and the 81 documents it includes, one group | `cargo test -p ferroui-themes-simple --lib tests::compiled_xaml_tests`: `build_script_output_is_the_checked_in_reference` (the build against the models is the checked-in `compiled_xaml.rs`, and its documents the checked-in `.xamlmeta`), `compiled_xaml_is_up_to_date` (the checked-in file is the emitter's against the run-time type system) | 82 of 82 documents compiled the same, none refused: 35 001 lines, the same bytes |
| the Fluent theme, the corpus | their tests, unchanged | unchanged |

The build scripts of both fixture crates have `ferroui-build` as their only build dependency and no `MarkupAssembly` of their own (`assembly.rs`, which `lib.rs` and `build.rs` both included, is gone: the static is in `lib.rs`, where the scanner reads it).

**Found, and what was done about it.**

1. *The order of the assemblies.* The namespace table a compiled document carries lists the assemblies in the order of the type system: the order of registration at run time, the order of the models in a build (the names of the `DEP_*` variables, each file followed by the files it names). The application registered the Simple theme before the Fluent one and its build reads them the other way round: the one difference the fixture showed (four lines, the order of two entries). The fixture now registers in the order of the build. An application whose `register_types()` calls its dependencies in another order than the names of their crates gets namespace tables in the order of the names; only a name two of those assemblies both declare in the same XML namespace is resolved differently by it.
2. *The scanner does not follow `include!`.* A `MarkupAssembly` in a file the root includes is not read, and the crate is then the assembly named after the crate. The fixture states the assembly in `lib.rs`.
3. *The scanner parses checked-in generated code* (the cost table above). Not changed here: a rule that skips the emitter's own files is a change to what a scan reads (9.5.11 taught it to read them).
4. *What the build script links.* `ferroui-build` links the loader, which links the XAML runtime library and the controls (9.5.10). With the models the build script needs none of them for its types, but a crate that takes `ferroui-build` as a build dependency still builds them for the host: for the Simple theme that is now every build of the theme, a `wasm32` build included, where the framework is then built for the host as well as for the target. That is reason 2 of 9.6.8 against converting the themes, and it is not gone: the emitter has to come out of the `runtime` feature of the loader (9.5.10, item 1) before the Fluent theme, the dialogs and the catalog follow. The author could not run the browser build to see the cost.

**What remains.**

1. The Fluent theme, the dialogs (`AboutFerroDialog`, the one document that still loads through the run-time loader) and the catalog on `TypeSystem::Model`; then the run-time host out of `Build` (`Build::assembly`, `loader`, `checked_in_metadata`, the registration in `execute`), and the checked-in theme files.
2. The emitter and the transform out of the `runtime` feature of the loader, so that `ferroui-build` links neither the controls nor the XAML runtime library (item 4 above).
3. The compile-time value parser of the build (9.5.11), the manifest keys (`[package.metadata.ferroui]`), diagnostics with codes and the severity mapping (`XamlCompilerDiagnosticsFilter` is not ported; `ferro_xaml_diagnostic_codes.rs` is), a position map for the file of a class, `include_xaml!` per class, the generated `register_types` (9.7.4), the cache of 9.6.4.
4. The drift test of the descriptions `EmitTypes` hands out (9.5.12, doubts 1 to 3), now with three more bodies of evidence than the corpus: 82 theme documents and the two fixture crates agree.

#### 9.5.14 Implemented (2026-10-09): the compiler without the framework, and the Fluent theme compiled by its build

Until this stage `ferroui-build` depended on the loader with its `emitter` feature, which implied `runtime`, which links the XAML runtime library and through it the controls: every build script that compiled against the type models still built the controls for the host. The model host needs none of that. What the emitter took from the run-time side was small, and none of it needs the run-time type system:

| What the emitter used | Where it was | Where it is |
|---|---|---|
| the plan of a property assignment (`plan_setters`), the reading of a numeric constant | `runtime/interpreter/evaluators.rs` | `back_end/assignment.rs`; the interpreter imports it |
| what the language configured the context to be, and the context of the framework language | `runtime/interpreter/runtime_context.rs`, `FRAMEWORK_CONTEXT` of the runtime library | `back_end/context.rs` (`ContextDefinition`, its own `FRAMEWORK_CONTEXT`; a test with the `runtime` feature compares it with the constant of the runtime library, field by field) |
| the namespace information of a parsed document | `runtime/interpreter/runtime_context.rs` | `back_end/namespaces.rs`; re-exported where it was |
| the deferred content customisation as a generic method definition, `adapt_type_mappings` | `runtime/framework` | `back_end/methods.rs`; the call of the method stays in `runtime/framework/methods.rs` as a function over it |
| the build and populate methods of the documents of a group | `RuntimeDocumentTypeBuilderProvider` (methods with a body the interpreter runs) | `back_end/methods.rs`: `DocumentTypeBuilderProvider`, the same signatures without a body; the run-time provider keeps its own methods and shares the signature and the method contracts |
| the include sources of a document, from its text | the run-time loader | `back_end/includes.rs`; the run-time loader imports it |
| the diagnostic handler of a transform | the type of the configuration of the run-time loader | `rust_emitter::DiagnosticHandler` over the diagnostics of the compiler; `TransformOptions::of` (run-time host) wraps the handler of a configuration as before |

Features of the loader:

| Feature | Contents | Links |
|---|---|---|
| `compiler` | `rust_emitter`: `transform_group`, `compile_documents_with`, `generate_file_with`, `generate_class_file_with`, `EmitTypes`, `CompiledMarkupTypeSystem` | base, markup, `xamlx`, `ferroui-build-scan` |
| `runtime` (the default of the crate itself) | the run-time type system, the interpreter, the run-time loader | + the XAML runtime library, the controls |
| `emitter` = `runtime` + `compiler` + the accessor table of the base crate | the run-time host of the emitter: `EmitterHost::runtime`, `compile_documents`, `generate_file`, `generate_class_file`, `runtime_types` (`rust_emitter/runtime_host.rs`) | as `runtime` |

The workspace states the loader without its default features; every crate that loads markup at run time states `runtime` (they all did). `ferroui-build` takes `compiler` alone. Its run-time host (`TypeSystem::Runtime`, with `Build::assembly`) is the feature `runtime-host` of `ferroui-build`, which no crate of the repository enables: no build script used it any more (the two fixture crates and the Simple theme were converted in 9.5.13). Without the feature a build that compiles documents against the run-time type system fails with an error that names the feature; `TypeSystem::Runtime` stays the value of a build that states none.

What a build script that uses `ferroui-build` links (`cargo tree -p ferroui-build`), before and after:

```text
before                                  after
ferroui-build                           ferroui-build
├── ferroui-base                        ├── ferroui-base
├── ferroui-build-scan                  ├── ferroui-build-scan
├── ferroui-markup-xaml                 ├── ferroui-markup-xaml-loader
│   ├── ferroui-base                    │   ├── ferroui-base
│   ├── ferroui-controls                │   ├── ferroui-build-scan
│   └── ferroui-markup                  │   ├── ferroui-markup
├── ferroui-markup-xaml-loader          │   └── xamlx
│   ├── ferroui-base                    └── xamlx
│   ├── ferroui-build-scan
│   ├── ferroui-markup
│   ├── ferroui-markup-xaml
│   └── xamlx
└── xamlx
```

The base crate is still built for the host (the compile-time value parsers of the compiler extensions and `MarkupAssembly`); the controls and the XAML runtime library are not.

**The Fluent theme** is compiled by its build script as the Simple theme is: `Build::type_system(TypeSystem::Model)` with the group of `FluentTheme.xaml`, the crate includes the output (`include_compiled_xaml!`), the checked-in `compiled_xaml.rs` and `compiled_xaml.xamlmeta` are the reference, and `build_script_output_is_the_checked_in_reference` compares them byte for byte.

| Crate | Documents | Identical to the checked-in file | Refused |
|---|---|---|---|
| Simple theme | 82 | 82 | 0 |
| Fluent theme | 86 | 86 | 0 |
| include fixture (theme, application) | as 9.5.13 | all | 0 |
| corpus (model against run time, `model_emit`) | 109 | 109 | 0 |

Numbers of the stage (run by its author, debug profile): `ferroui-build --lib` 24 tests, the loader 397 with its default features and 272 with `--no-default-features --features compiler`, `ferroui-markup-xaml-tests` 578, the Simple theme 205, the Fluent theme 200, the fixture 3 and 28, the dialogs 47.

**The dialogs: not converted.** The crate has one class document, `AboutFerroDialog.xaml`, populated by the run-time loader in the constructor of the class (`markup::load_component`). Compiled by its build (`XamlGroup::class_document`, with `Build::default_compile_bindings(true)`: the upstream project is built with compiled bindings as the default), every node of the document is emitted except one, which the emitter refuses:

```text
AboutFerroDialog.xaml: XamlPropertyAssignmentNode: Click: not a plain property setter (line 63 position 8)
```

*(Closed since: the rule is 9.4.4 and the dialogs are compiled by their build, 9.5.15. As this stage left it:)* `Click="Button_OnClick"` reaches the emitter as `XamlDirectCallAddHandler` over a `XamlLoadMethodDelegateNode` (9.4.4), and the emitter has no rule for either: event handlers are the open item of 9.4.4. It is a rule of the emitter (the handler as a closure over the weak root that calls the typed function of the declared method, an `rt` helper that attaches it to the routed event, the two `EmitTypes` hosts answering for the event field and the method, corpus documents), not a rule of the scanner or the model: the transform against the models already produces the node. Until it exists the dialogs keep the run-time loader as a dependency, for that document alone (`markup.rs`, `register_types.rs`); nothing else in the crate uses it outside its tests.

Not done here: the compile-time value parser of the build; the manifest keys; removing the run-time host from `Build` and the `emitter` feature's use by the tests of the themes (the reference tests generate the checked-in file through it); `deterministic_id_generator.rs`, which is still not called (neither host passes an identifier generator to the compiler configuration; wiring it is a change of emitted names to measure against the references).

#### 9.5.15 Implemented (2026-10-09): event handlers, and the dialogs compiled by their build

The rule of the emitter for a method named in markup is 9.4.4. With it the one class document of the dialogs (`src/FerroUI.Dialogs/AboutFerroDialog.xaml`) compiles, and the crate is converted as 9.5.14 said it would be:

| | Before | Now |
|---|---|---|
| `build.rs` | exports the type model (`ferroui_build_scan::export`), writes its own asset table (every `.xaml` and `Assets/`) | `Build::from_env().type_system(TypeSystem::Model).default_compile_bindings(true).embed_assets(&["Assets"]).compile_group(XamlGroup::new("compiled_about_ferro_dialog").documents(..).class_document("AboutFerroDialog.xaml")).run()`: the compiled document, the assets, the loader table and the `.xamlmeta` with the type model and the compiled document |
| `AboutFerroDialog::new` | `markup::load_component(&this, "/AboutFerroDialog.xaml")`: the run-time loader with compiled bindings as the default | `crate::compiled_about_ferro_dialog::populate(None, &this)` |
| the loader table | written by hand in `register_types.rs`, with `FerroRuntimeXamlLoader::register_class_document` | `compiled_markup::register()` of the build: the assets and `try_load`, which creates the dialog with its constructor |
| the assets of the assembly | `/AboutFerroDialog.xaml`, `/Assets/Roboto-Light.ttf` | `/Assets/Roboto-Light.ttf` (a compiled document is not a resource of the assembly, as upstream's compiler removes it) |
| `markup.rs`, `assets.rs` | the loading of the documents of the crate; the asset table | removed |
| dependencies | `ferroui-markup-xaml-loader` with `runtime`; build: `ferroui-build-scan` | no dependency on the loader at all, not in its tests either (the two tests that registered the run-time loader did not need it); `ferroui-base` with `markup-functions`; build: `ferroui-build` |

The document is emitted whole: the compiled bindings to the static properties of the class (`{Binding Version}` under `x:DataType="dialogs:AboutFerroDialog"`), the dynamic resource of the logo, the font by its relative URI, and `Click="Button_OnClick"` as `rt::add_handler` over `AboutFerroDialog::__markup_Button_OnClick_0`. The tests of the crate are the guard (49 pass; 47 before): the dialog and its bound texts as before, the button opening the address of the project through the launcher of the platform when it is clicked, and a load of the document by its URI creating the dialog.

Both themes take the dialogs as a dependency and read their `.xamlmeta`, which now also lists the compiled document; their output is unchanged, byte for byte (the reference tests of both).

A crate that depends on the dialogs now builds `ferroui-build` for the host (the base crate, the loader with `compiler`, `xamlx`, the scanner), as a crate that depends on a theme already does. Not run by the author: a `wasm32` build.

#### 9.5.16 Implemented (2026-10-09): macros in the scanner (the work list of the catalog)

Two forms the scan of the colour picker and of the catalog did not read (HANDOVER.md, section 19, has the measure of the catalog before and after).

*A rule that repeats, for registrations.* `expand_registration_macros` (9.5.12) expands the macros a function invokes and reads what the expansions register with the untyped value conversions. The matcher and the transcriber now do repetitions (`$($type_:ty),* $(,)?` and `$( .. )*`, with a separator, with `*`, `+` or `?`, nested): a variable of a repetition stands for one text per round, and a repetition of the transcriber is written once for each round of the variables it names. The colour picker registers the casts of its palettes and of its converters that way (`register_palettes!(FlatColorPalette, ..)`), and with a cast of a crate not read the models answer no question about a cast (9.5.12), which refused 72 documents of the catalog. The expansion of a macro in item position, for what it *declares* (9.5.6), does not do repetitions and is unchanged.

*A macro another crate exports.* `ferroui_controls::ferro_markup_list!(pub CountryList: Rc<Country>);` declares the list of a view model as a type of markup (``FerroList`1[Country]``, with its indexer), and its definition is in the controls crate, which the scan of the crate that invokes it never reads (limit 4 of 9.5.6). The model of a crate now has the macros it exports (`#[macro_export]`) that declare through a declaration macro, as text (`AssemblyModel::macros`, `MacroModel { name, rules[(matcher, transcriber)] }`; one definition, no `cfg` condition), and the scan of a crate built on it expands an invocation of a macro its own crate does not define with those rules, `$crate` written as the path of the exporting crate. Without the type, the property bound to `ItemsSource` was a Rust type no metadata declares, and the transform found no data type for the item template (`Unable to resolve property or method of name 'Name' on type 'XamlX.TypeSystem.XamlPseudoType'`: 6 documents of the catalog; the run-time type system has the type because the expansion registers it).

| Model | Before | Now |
|---|---|---|
| `ferroui_controls` | no macros | `ferro_markup_list!` with its rule |
| `ferroui_controls_color_picker` | 2 `register_cast` and 2 `register_nullable` not read; no list of palette colours | 32 types, the list among them; nothing not read; the 13 casts of its two macros |
| `control_catalog` | 289 types | 295 types (its six lists) |

The drift test (`type_system_drift`) asserts that no crate of it has a cast its scan did not read, and `the_colour_picker_is_read_with_its_macros` asserts both forms on the colour picker, read as files. The colour picker cannot be a crate of the comparison itself in that process (HANDOVER.md, section 19): it was compared once, linked, with nothing in a kind that must be empty.

#### 9.5.17 Implemented (2026-10-09): a first real compile of pages of the catalog

Everything the measure of the catalog counts as emitted is Rust rustc has not seen. `tests/XamlCatalogFixture` (`xaml-catalog-fixture`) is the smallest build that changes that without converting the sample: a crate whose sources *are* files of the sample.

| Part | What it is |
|---|---|
| `build.rs` | `Build::from_env().type_system(TypeSystem::Model).default_compile_bindings(true)` with one group per page, `XamlGroup::new(module).documents(&[(path, text)]).class_document(path)`, the text read from `samples/ControlCatalog/<path>` |
| `pages.rs`, `view_models.rs` | `#[path = "../../samples/ControlCatalog/Pages/check_box_page.rs"] mod check_box_page;` and so on: the five page classes, `WrapPanelPageViewModel` with `WrapPanelItemList`, `Random`. The scanner follows `#[path]`, so the model of the fixture is the scan of the sample's files |
| `markup.rs` | the two macros the page classes take from `crate::markup`: `content_page_class!` as the sample has it, and `xaml_class!`, whose `initialize_component()` calls `<Class as CompiledMarkup>::populate(&self.to_ref())`, the `populate` of the module the build wrote |
| `lib.rs` | the assembly `ControlCatalog` and the namespaces of the sample, so that `x:Class="ControlCatalog.Pages.CheckBoxPage"` and `using:ControlCatalog.ViewModels` are what they are there; `include_compiled_xaml!()` |
| `tests/mod.rs` | each page populated by its compiled markup against the same class populated by the run-time loader from the same file |

It proves, for five documents (`CheckBoxPage`, `ProgressBarPage`, `ButtonSpinnerPage`, `WrapPanelPage`, `ImagePage`): the build emits them from the scan of the sample's own sources; rustc compiles the 207,653 bytes; four of them load to the tree the run-time loader builds, a handler is the method of the class, the compiled bindings deliver the typed list of the view model; the fifth fails to load as it does with the run-time loader, because no bitmap can be loaded in the tests. HANDOVER.md, section 19, has the exact list and what is not proven.

*(Superseded by 9.5.18 and 9.5.19: the fact is a registration, and the fixture is the whole sample.)* **It also found the first code rustc refuses.** `CanvasPage` and `SliderPage` are emitted and do not compile: the instance of a member a named collection inherits from the list it derives from is passed as `&value`, which is right only where the Rust type dereferences to the Rust type of the base (`RowDefinitions`), and that is not a fact either type system has (`Points`, `TickList`). The measure cannot see it. HANDOVER.md, section 19, "The first real compile: what rustc refuses", has the errors and the ways to give the emitter the fact; it is the first item of the next stage, before any conversion.

#### 9.5.18 Implemented (2026-10-09): what dereferences to its base is a registration

The rule of the emitter for the instance of an instance member (`Emitter::receiver`): a value of the declaring type is passed as `&value`; a value of a class that derives from the declaring class as `value.upcast_ref::<Base>()`; a value of a type with markup metadata whose declaration names the declaring type as a base was passed as `&value` too, on the assumption that the Rust type of the one dereferences to the Rust type of the other. The assumption holds for `RowDefinitions` and fails for `Points` and `TickList`, and neither type system could tell them apart (9.5.17).

The fact is now a registration (ruling 8 of HANDOVER.md):

```rust
// src/FerroUI.Controls/markup_types/plain.rs, next to the casts of the collections
ValueTypes::register_deref::<RowDefinitions, FerroList<Ref<RowDefinition>>>(|c| c);
```

| Part | What it does |
|---|---|
| `ValueTypes::register_deref::<From, To>(deref: fn(&From) -> &To)` (`ferroui-base`) | The argument is the coercion, written `|c| c`: the call compiles exactly when `&From` coerces to `&To`, so the fact cannot be stated of a pair it does not hold for. With the feature `compiler-metadata` the pair is recorded in the registry of the untyped value conversions (`ValueTypes::dereferences`, `dereference_count`); without it the call records nothing. It registers no conversion: the cast of the pair is `register_cast`, as before |
| The scanner | `deref` is one more name of `VALUE_REGISTRATIONS` (two type arguments): the call is read like the other registrations with stated types, into `AssemblyModel::value_types`, and a call whose types are not read is counted in `unread_value_types`. The format of the `.xamlmeta` is unchanged (a registration is a name and its types) |
| `EmitTypes::dereferences(from, to)` | `RuntimeEmitTypes`: the registry. `ModelEmitTypes`: the pairs of the `deref` registrations of the models; when a crate has such a call the scan did not read, a *no* is recorded as a question the models cannot answer and the document is refused (9.5.12) |
| `Emitter::receiver` | `&value` for a base only when `dereferences(handle of the value, handle of the base)`; otherwise no receiver, and the caller converts the instance as the run-time loader does (`rt::cast` of a clone of the value, through the cast the crate registers; `rt::instance` for the nullable form) |
| The drift test (`type_system_drift.rs`) | For every type both sides have and each base of it, the two hosts give the same answer; the pairs that dereference are as many as the scans read and as many as the registry holds |

Registered: `KeyFrames` and `Transitions` (base crate), `Controls`, `InlineCollection`, `DataTemplates`, `RowDefinitions`, `ColumnDefinitions` (controls): the named collections that implement `Deref` down to their list. Not registered, because the coercion does not exist: the collections that hold their list (`Points`, `GradientStops`, `PathFigures`, `PathSegments`, `GeometryCollection`, `Transforms`, `DrawingCollection`, `FontFeatureCollection`, `TextDecorationCollection`) and `TickList` (it dereferences to `FerroList<f64>`, and the base its declaration names is `MediaCollection<f64>`). For these the instance is converted by the registered cast, which yields the same list, so `Add` and `Capacity` reach the collection as they do in the run-time loader.

No existing output changed (the corpus, both themes, the dialogs, the fixtures: their byte checks). The corpus has `list_text_held_collection.xaml` (`Points` of a polygon and of a polyline, `Ticks` of a slider, and `RowDefinitions` in one document: the converted instance and the borrowed one side by side), emitted the same by both hosts and equal to the run-time loader's tree; `CanvasPage` and `SliderPage` are in the fixture of 9.5.17, compile, and load to the run-time loader's trees with the points and the ticks of their texts.

#### 9.5.19 Implemented (2026-10-09): everything the compiler emits for the catalog is compiled by rustc and compared

The fixture of 9.5.17 is now the whole sample. It is still not a copy and not a conversion: `samples/ControlCatalog` is unchanged but for the export of five typed lists (below) and loads its documents with the run-time loader.

| Part | What it is |
|---|---|
| `lib.rs` | The modules of the sample by `#[path]`: `Controls/mod.rs`, `Converter/mod.rs`, `Models/mod.rs`, `Pages/mod.rs`, `ViewModels/mod.rs`, `Views/mod.rs` and the root files (`app.rs`, `main_view.rs`, `main_window.rs`, `decorated_window.rs`, `transparent_styles.rs`, `icons.rs`, `page_assets.rs`, `smoke.rs`). The scanner follows them: 308 files, 295 types, 0 declarations not read, in about a second. `extern crate self as xaml_catalog_fixture;` (see the errors below) |
| `markup.rs` | The module of the sample with one difference: `load_component` (what `initialize_component()` of every class calls) populates the instance from the compiled markup of the document when the build compiled it (`populate_compiled`, over the table `COMPILED` the build script writes from the model of the build: document, class, `populate`), and with the run-time loader otherwise, as the sample does |
| `register_types.rs`, `assets.rs` | The files of the sample with the name of this crate in the namespace table, over the tables the build script writes (the documents and the assets of the sample, as the files are) |
| `documents.rs` | `PAGES` (the seven pages a build without the feature compiles), `REFUSED` (the documents the compiler refuses, each with its reason), `NOT_LOADED` and `NOT_RUN` (compiled documents the tests cannot compare, each with its reason: both are empty) |
| `build.rs` | Exports the models of the three crates that do not export theirs yet (the colour picker, the OpenGL controls, `mini-mvvm`) into its output directory, then `Build::new(..).type_system(TypeSystem::Model).default_compile_bindings(true)` with one group per compiled document, as the measure builds them. With the feature `catalog`: every document but `REFUSED`; without it: `PAGES`. A refused document that is not listed fails the build |
| `tests/mod.rs` | One test per compiled document with a class, written by the build script: `compare(path, application)` creates the class twice without the body of its constructor, populates one by its compiled markup and one by the run-time loader from the same document, and compares the dumps (every object with its class, the registered properties set on it with values and priorities, its name, its logical children), each dumped before the other instance exists. The application is the one the tests of the sample start for the document (`test_applications.txt`): the test services of the catalog (the Skia render interface and font manager, the HarfBuzz shaper, the assets of the sample) with the Simple theme and the resources of `CustomThemes.xaml`, or the application class of the sample |

**What it found.** The build with the feature emits the documents the measure counts and rustc compiles all of them.

| | Emitted | Compiled by rustc | The tree of the run-time loader | Emitted Rust |
|---|---|---|---|---|
| The stage before (9.5.17) | 195 | 5 | 4 (the fifth fails the same in both back ends) | 12,908,248 bytes |
| With the dereference fact (9.5.18) and the lists exported | 199 | 199 | 199 of 199 with a class | 13,185,447 bytes, 138,082 lines |
| With the type of an untyped value (9.4.7) | 200 | 200 | 200 of 200 | |
| With the declared type of a property (9.4.8) | 206 | 206 | 205 of 205 with a class; `CustomThemes.xaml` has none and is compared by the number of its resources | 13,800,499 bytes, 144,673 lines |

rustc errors, by cause, over the whole of the emitted code:

| Errors | Code | Cause | Fix |
|---|---|---|---|
| 16 (2 documents) | `E0308` | The instance of a member a named collection inherits passed as `&value` (9.5.17) | 9.5.18: the fact is a registration; the emitter converts where it is not registered |
| 54 (1 document) | `E0433` | A document without a class (`CustomThemes.xaml`) names the classes of its own crate by the name of the crate (`::xaml_catalog_fixture::controls::SamplePage`), which a crate cannot say of itself; the file of a class names them through `crate` (`generate_class_file_with` rewrites the prefix) | The crate declares `extern crate self as <crate>;`, as the include fixture and the XAML test crate do. The emitter is unchanged: writing `crate::` in the file of a group too is the better form, and changes the output of the include fixture, so it is left for the stage that converts the sample |

Nothing else: no other document of the 206 has an error or a warning of its own under rustc.

Every compiled document with a class loads in both back ends and to the same tree. Three things the tests had to do to say so, each a fact about the sample and not about the compiler: a page with bitmaps needs the assets and a render interface that decodes them (the fixture embeds the assets of the sample and uses the Skia services of the sample's tests; `ImagePage` now loads its bitmaps by compiled markup and the images have the sizes of the run-time loader's); pages that read the resources of the application need them (`CustomThemes.xaml`, and for `SettingsPage` the application class); and two instances of one page are not independent (the radio buttons of a group outside a visual tree are one group), so each tree is dumped before the other exists.

**Found and not fixed: a registration only the run-time loader makes.** With the application of a test merging the *compiled* `CustomThemes.xaml`, no run-time load precedes the first compiled page, and 19 comparisons differ in how one value prints: `DataValidationErrors.Errors` is `Option<Vec<BoxedValue>>`, and `Option<Vec<T>>` is registered as the nullable form of `Vec<T>` by the type system of the run-time loader when it is first created (`RuntimeArray::register_element`, `runtime/type_system/values.rs`). The trees are the same; the untyped value conversions of the process are not, until the run-time loader has loaded something. A process with compiled markup only (the converted sample) never gets that registration. The fixture loads `CustomThemes.xaml` with the run-time loader at the start of each test, as the tests of the sample do, and says why. The conversion of the sample has to find which registrations of `RuntimeTypeSystem::new` generated code relies on and move them to where a crate registers its types.

**The lists of the sample.** Compiled markup is a module of its own, so a type it names needs a public path. Five typed lists (`ferro_markup_list!`) were `pub` in private modules: `StandardCursorList`, `CountryList`, `NodeList`, `WrapPanelItemList` and `FormatObjectList` are now in the `pub use` of their modules (`ViewModels/mod.rs`, `Pages/mod.rs`). It is the one change to the sample.

**Cost.** On the machine of the author (the dependencies built, debug profile without debug information): the build script and the test binary without the feature 85 s, with it 165 s; the tests 1 s and 10 s. The build script itself: the three exports and the scan about 2 s, the 206 documents about 11 s. The feature is off by default, so `cargo test --workspace` (CI) builds the seven pages, each with a test, and the other classes populated by the run-time loader; the whole set is `cargo test -p xaml-catalog-fixture --lib --features catalog`.

**The measure and the fixture cannot drift.** `emitter::catalog_measure` reads `documents.rs` of the fixture and fails when the documents it finds refused are not exactly `REFUSED`; the build of the fixture fails when a document outside the list is refused.

### 9.6 Build integration

#### 9.6.1 Entry points

Crate `ferroui-build` (`src/FerroUI.Build.Tasks`), a build-dependency. It depends on the loader without the `runtime` feature, on `xamlx`, on `ferroui-base` and `ferroui-markup` (the loader already links both for the compile-time intrinsics and grammars; this settles open question 8 in favour of "reuse Base", which requires Base to stay free of native link dependencies), and on `syn` and `serde_json`. It never links `ferroui-controls` or `ferroui-markup-xaml` (decision 10); controls-typed intrinsics go through `IXamlCompileTimeValueParser` with a build-time implementation that evaluates only what the loader's own parsers cover and otherwise leaves `from_xaml_str` to run time.

```rust
// build.rs of an application or control library
fn main() {
    ferroui_build::Build::from_env()      // reads CARGO_MANIFEST_DIR, OUT_DIR, DEP_*_XAMLMETA, [package.metadata.ferroui]
        .export_metadata()                // scan src/**/*.rs, write $OUT_DIR/<crate>.xamlmeta, print cargo::metadata=xamlmeta=<path>
        .embed_assets()                   // patterns from [package.metadata.ferroui] assets = ["Assets/**"]
        .compile_xaml()                   // every *.xaml under the XAML root (default: src/)
        .run();                           // prints cargo:: directives; exits non-zero after reporting all errors
}
```

The three free functions of section 5 (`compile_xaml()`, `embed_assets()`, `export_metadata()`) are thin wrappers that run one step with defaults. A framework crate without XAML calls only `export_metadata()`.

`[package.metadata.ferroui]` keys: `assembly` (assembly name, default: package name in PascalCase), `xaml-root`, `assets`, `compile-bindings-default`, `create-source-info`, `warnings-as-errors`, `no-warn` (`XamlCompilerDiagnosticsFilter` port), `links-name`.

#### 9.6.2 Inputs, outputs, `OUT_DIR` layout

```
$OUT_DIR/
  <crate>.xamlmeta                       exported model (types, registered properties, documents)
  xaml/
    <relative path>.xaml.rs             one per document: class documents are included by the class's module,
                                         classless documents by compiled_xaml.rs
    <relative path>.xaml.map.json       generated line -> XAML position
    compiled_xaml.rs                     crate-level: modules of classless documents, try_load table, registration fn
    assets.rs                            include_bytes! index (embed_assets)
  xaml-diagnostics.json                  all diagnostics, for editors
  xaml-cache/<hash>.json                 per-document parse/transform cache keyed by content hash + model hash
```

User-side inclusion:

| Where | Form | Expands to |
|---|---|---|
| module of an `x:Class` type | `include_xaml!("views/MainWindow.xaml");` (path relative to the XAML root) | `include!(concat!(env!("OUT_DIR"), "/xaml/views/MainWindow.xaml.rs"));` |
| crate root | `ferroui::include_compiled_xaml!();` | `#[doc(hidden)] pub mod compiled_xaml { include!(concat!(env!("OUT_DIR"), "/xaml/compiled_xaml.rs")); }` |

Both are `macro_rules!` **(new)** in `ferroui-markup-xaml`. A document with `x:Class` whose class module has no `include_xaml!` shows up as "method `initialize_component` not found" at the user's call; to make this readable `compiled_xaml.rs` contains `const _: () = { let _ = <path>::<Class>::xaml_populate; };` preceded by a marker comment naming the missing `include_xaml!`.

Files are written only when their content changed. A classless document is a module body:

```rust
// $OUT_DIR/xaml/styles/Controls.xaml.rs  (mod styles__controls_xaml inside compiled_xaml)
pub fn build(sp: &Rc<dyn IServiceProvider>) -> Result<Ref<Styles>, XamlLoadException> { .. }       // "Build:/styles/Controls.xaml"
pub fn populate(sp: &Rc<dyn IServiceProvider>, target: &Ref<Styles>) -> Result<(), XamlLoadException> { .. }
```

`build` follows the contract of `ferro_xaml_il_compiler.rs`: it exists only when the root is a `XamlAstNewClrObjectNode` whose argument types equal the constructor's parameter types; when the single argument is the service provider the context is created before the constructor call.

#### 9.6.3 Dependencies between crates

- `Cargo.toml` of every crate that exports metadata: `links = "<crate>_xaml"` and a `build.rs`. `export_metadata()` prints `cargo::metadata=xamlmeta=<abs path>`; a direct dependent reads `DEP_<CRATE>_XAML_XAMLMETA`.
- `DEP_*` variables exist only for **direct** dependencies. Each `.xamlmeta` therefore lists the absolute paths of the files of its own dependencies; the tool loads the transitive closure. A facade crate (`ferroui`) must itself have `links` + `export_metadata()` so that applications depending only on the facade see the framework.
- The model of a dependency is trusted as is (no re-scan). Its `documents[]` list is what makes cross-crate includes direct calls (9.7.3).

#### 9.6.4 Change tracking

Printing any `cargo::rerun-if-changed` disables Cargo's default "re-run when any package file changes", so the list must be complete:

- `cargo::rerun-if-changed=build.rs`, `Cargo.toml`;
- the XAML root directory and the source root directory (`src`): Cargo scans a directory recursively, which covers new, removed and edited `.xaml` and `.rs` files with one line each;
- every asset directory;
- every dependency `.xamlmeta` path (they change only when their content changes, because they too are written on change);
- `cargo::rerun-if-env-changed` for `FERROUI_*` switches.

The script therefore runs on every source edit and must be fast: the scan result is cached per file by content hash, documents by (content hash, hash of the part of the model they resolved against), group transformers re-run only for documents whose include targets changed; unchanged outputs are not rewritten, so rustc's incremental cache stays valid. Target: under 100 ms for a no-op run on a mid-sized application.

#### 9.6.5 Diagnostics

- XAML errors: `cargo::error=src/views/MainWindow.xaml(12,34): error FRN2000: <message>`; warnings: `cargo::warning=...(12,34): warning FRN2105: <message>`. Codes are the constants of `ferro_xaml_diagnostic_codes.rs`; the severity mapping is `XamlCompilerDiagnosticsFilter`.
- All documents are parsed and transformed before failing; the script exits non-zero once, after printing every error (upstream behaviour: continue past the first error).
- On error the previous generated files are left in place and a `compile_error!` is written into `compiled_xaml.rs`, so that a stale build cannot succeed silently when Cargo is run with `--keep-going`.
- Scanner diagnostics (unreadable declaration, unresolved type name) use the Rust file position and codes in a separate range (`FRN9xxx`).

**Implemented (2026-10-09)** for the model host of `Build` (`src/FerroUI.Build.Tasks/diagnostics.rs`), as `XamlCompilerTaskExecutor` does it:

| Upstream | Here |
|---|---|
| `HandleDiagnostic = d => { newSeverity = diagnosticsFilter.Handle(d); diagnostics.Add(d with Severity); return newSeverity; }` | the diagnostic handler of every transform of a build (`TransformOptions::diagnostic_handler`): the filter states the severity, the transform continues with it, the diagnostic is kept |
| `AnalyzerConfigFiles` of the task | `Build::analyzer_config_files(&[..])`, relative to the crate directory: entries `ferro_xaml_diagnostic.<code>.severity = error / warning / default / anything else (silent)`; each file is an input of the build; one that does not exist is skipped |
| `ReportDiagnostics` (at most 100 of a document), `LogDiagnostic` | after each group: a warning is `cargo::warning=<document>(<line>,<position>): warning <code>: <title>`, an error the same text with `error` as an error of the build, a silenced one nothing |
| `TreatWarningsAsErrors`, left to the build system | `Build::warnings_as_errors(true)`: a warning the filter leaves a warning is reported as an error; it fails the build and not the transform, so every document is still compiled |
| `LogError(TransformError, "", e)` for a group that does not transform | one error for the group, `the group <module>: error FRN2000: ..` with the module in backquotes, (it was one line per document), and none when the diagnostics of the transform already reported an error; for a class group `<class document>: error FRN2000: ..` (the class not found in the models, the group not transformed) |
| `LogError(EmitError, res.FilePath, e)` | `<document>: error FRN3000: <reason>` for each document the emitter refuses, with the node and its position in the reason |
| `XamlLoaderUnreachable` through `diagnosticsFilter.Handle(Warning, code)` | the warning of a class without a constructor the loader table can call: `FRN3001`, through the filter by its code |

Tests: `diagnostics::tests` in `ferroui-build` (the handler, the report, the EditorConfig severities, warnings as errors, the cap) and `emitter::build_diagnostics` in `ferroui-markup-xaml-tests`, which runs builds over the scanned models of the framework crates with real documents: the warning `Views/List.xaml(4,14): warning FRN2208` (an item container in an item template), the same as an error under warnings as errors, silenced and raised by an EditorConfig file, a group with an unknown type (one `FRN2000`), and two documents whose functions collide (`FRN3000`).

Not done: the `compile_error!` of the third point (a failed build script already fails the build of the crate); positions for `FRN3000` as a `(line,position)` pair (they are in the text of the reason); the EditorConfig files found without a call (they come with the manifest keys of 9.6.1); the run-time host of `Build` reports as before.

#### 9.6.6 Group transformers

One `compile_xaml()` call is one group: all documents of the crate are parsed, transformed individually, then `transform_group` runs `XamlMergeResourceGroupTransformer` and `FerroXamlIncludeTransformer` over the set, exactly like `FerroXamlIlRuntimeCompiler` does for a runtime group. The build tool implements `IXamlDocumentResource` / `IXamlDocumentTypeBuilderProvider` with methods whose payload (`EmitBacking`) is the Rust path of the function the emitter will write (`crate::compiled_xaml::styles__controls_xaml::build`, `crate::views::main_window::MainWindow::xaml_populate`). Document usage (`XamlDocumentUsage::Merged`) suppresses the loader-table entry of non-public merged documents as upstream.

#### 9.6.7 Proc macro versus `build.rs` (restated)

| | `build.rs` (decided) | proc macro |
|---|---|---|
| Sees all documents of the crate (group transformers, loader table) | yes | no |
| File dependency tracking | `rerun-if-changed` | none that is stable |
| Can export `.xamlmeta` to dependents and embed assets | yes | no |
| Diagnostics with XAML positions | `cargo::error=` | spans point at the macro invocation |
| Runs in the IDE on every keystroke | no | yes |
| Cost | script runs on source edits (cached) | — |

Decision stands (section 1.1, decision 3): `build.rs` drives; `include_xaml!` / `include_compiled_xaml!` are declarative one-liners.

#### 9.6.8 Implemented (E5 step 3, 2026-10-08): the build script as a host of the emitter

Sections 9.6.1 to 9.6.6 assume the build-time type system of 9.5 (the source scanner, `ModelTypeSystem`). It does not exist: the emitter transforms against the run-time type system, that is, against the types the process registered. What is implemented is the build integration over that type system, and it reaches as far as that type system allows.

**The crate.** `ferroui-build` (`src/FerroUI.Build.Tasks`), a build-dependency. It depends on the loader with the `runtime` and `emitter` features, on `ferroui-base` and on `ferroui-markup-xaml`; it does not depend on `ferroui-controls`. `serde_json` is not used (the `.xamlmeta` reader and writer are the emitter's own, and the model's `json.rs`). Since the scanner stage (9.5.6) the crate also depends on `syn` (`full`, `parsing`, `printing`, `visit`; no default features), `proc-macro2` (`span-locations`, for the lines of the scanner's diagnostics) and `quote`; `Build` itself does not use them.

```rust
// build.rs
fn main() {
    ferroui_controls::register_types();                  // the types the documents name; see "Type metadata"
    ferroui_build::Build::from_env()                     // CARGO_MANIFEST_DIR, OUT_DIR, CARGO_PKG_NAME, DEP_*_XAMLMETA
        .assembly(&ASSEMBLY)                             // the MarkupAssembly the crate registers
        .embed_assets(&["Assets"])                       // every file below the directories
        .compile_xaml()                                  // every *.xaml below the XAML root, one group
        .run();                                          // writes the files, prints the cargo:: lines, fails once
}
// lib.rs
ferroui_markup_xaml::include_compiled_xaml!();           // pub mod compiled_xaml, pub mod compiled_markup
```

| Design (9.6.1 to 9.6.6) | Implemented |
|---|---|
| `export_metadata()` scans `src/**/*.rs` | Since 9.5.7: `Build::export_metadata()` scans the sources and `run()` writes the model into the `.xamlmeta` (format 2) next to `documents[]`. Without the call, as before: no scan, `run()` writes `$OUT_DIR/<crate>.xamlmeta` with `documents[]` and the absolute paths of the files of the direct dependencies, and prints `cargo::metadata=xamlmeta=<path>`. It also prints `cargo::rustc-env=FERROUI_XAMLMETA=<path>`, so the crate's own tests read the file. No framework crate calls `export_metadata()` yet (9.5.7 says what the switch needs). |
| `[package.metadata.ferroui]` keys | Not read (no TOML reader in the build). The builder states them: `assembly`, `xaml_root`, `embed_assets`, `compile_group(XamlGroup)` with `create_source_info`. Diagnostics filtering (`warnings-as-errors`, `no-warn`) is not implemented. |
| One file per document, `include_xaml!` per class | One file per group: `$OUT_DIR/xaml/<module>.rs` is the unchanged text of `rust_emitter::generate_file`, with `<module>.map.json`. `$OUT_DIR/xaml/mod.rs` declares the module of every group (`include!` of the file) and the module `compiled_markup`: `ASSETS`, `try_load` (the loader table of the crate: each table in turn) and `register()` (R8: `register_assets` and `register_compiled_xaml`). `include_compiled_xaml!()` includes `mod.rs` at the crate root. |
| `assets.rs` | The asset table is `compiled_markup::ASSETS` in `mod.rs`: `(rooted path, include_bytes!)`. A document a group compiled is left out, as upstream removes a compiled resource (`res.Remove()`). |
| Change tracking, cache | `rerun-if-changed` for `build.rs`, `Cargo.toml`, the XAML root and every asset directory, every file the script states with `input()`, every `.xamlmeta` read (the transitive ones too). Files are written only when their text changed. No cache: every run compiles every group. `rerun-if-changed=src` is not printed, because the sources are not read. |
| Diagnostics | Every group is compiled before the build fails. A document that is not eligible is one `cargo::error=<document>: <reason>` line; the script exits with a failure once. No codes, no `xaml-diagnostics.json`, no `compile_error!` file. |
| Group transformers | As designed: one group is one `generate_file` call (9.7.3). |
| Generated `register_types` (9.7.4) | Not generated: it needs the scan. The crate writes `register_types()` and calls `compiled_markup::register()` in it. |

Since 9.5.10 the compiler takes the type system from its host (`rust_emitter::EmitterHost`): `Build` calls `generate_file`, which is `generate_file_with` with `EmitterHost::runtime`, the run-time type system this section describes. Everything below still holds. `ferroui-build` itself links the controls through `ferroui-markup-xaml` (9.5.10, "What the build crate links").

Since 9.5.13 `Build` has a second host, the build-time type system (`Build::type_system(TypeSystem::Model)`): the limit below is the limit of the default host only. The include fixture and the Simple theme use the models; what follows describes the run-time host, which the Fluent theme's consumers and every other caller of `Build` still get.

**Type metadata (the limit).** A build script runs on the host and links what its `[build-dependencies]` name. The compiler reads types from the registries of the process, so:

1. The build script takes the crates whose types its documents name as build-dependencies and calls their `register_types()` before `run()`. It also starts the application services the transform runs with (the fixture starts `UnitTestApplication`, as the generator tests did). `ferroui-build` itself registers the runtime library and the assembly of the crate.
2. Those crates are built for the host. With the host equal to the target and equal features Cargo can share the units; otherwise the framework is built twice. This is the cost decision 10 and section 2.4 rejected linking for, and it is why the themes are not converted (below).
3. A build script cannot link the crate it builds. A document that names a type of its own crate cannot be compiled by the build script: the document of a class of the crate (`x:Class`), or a document that uses a control or a view model of the crate. Such a document stays generated by a test and checked in (`generate_class_file`, drift-tested). `Build::loader(path)` puts the `try_load` of the checked-in file first in the loader table of the crate, and `Build::checked_in_metadata(path)` adds its documents to the `.xamlmeta` of the crate.

Point 3 is the general obstacle: an application's documents are mostly `x:Class` documents. The build integration of 9.6 for them needs the scanner of 9.5 (E1 of 9.10.1), which is the next piece of work on this path, not a larger build script.

**Transport of `.xamlmeta` (9.6.3), implemented as designed.** A crate with compiled markup has `links = "<crate>_xaml"` and prints `cargo::metadata=xamlmeta=<path>`; the build script of a direct dependent reads `DEP_<CRATE>_XAML_XAMLMETA`. `Build::from_env()` takes every `DEP_*_XAMLMETA` variable, in the order of the variable names, and reads the files they name transitively. A crate whose compiled markup is checked in exports its checked-in file the same way without `ferroui-build`: the two themes have the `links` key and print the line for `compiled_xaml.xamlmeta` in their existing `build.rs`.

**The fixture (`tests/XamlIncludeFixture`).** *(As this step left it. Since 9.5.13 both crates compile against the type models: the document of `StyleWithServiceProvider` is compiled by the build script of the library, no file of the fixture is checked in, `assembly.rs` is gone and the build scripts link only `ferroui-build`.)*

- The library compiles its documents without a class (two groups: `compiled_xaml`, and `compiled_xaml_source_info` with `CreateSourceInfo`, which is `internal()`: not in the loader table, not in the `.xamlmeta`) in `build.rs`. The document of `StyleWithServiceProvider`, a class of the crate, stays checked in (`compiled_style_with_service_provider.rs` and `.xamlmeta`), joined through `loader()` and `checked_in_metadata()`.
- The application compiles all of its documents in `build.rs`, with the `.xamlmeta` of the library and of both themes from `DEP_*`. Its build script links the three crates and registers their types. `LocaleCollection`, which a document of the application uses, moved to the library for point 3.
- The documents are text constants of `documents.rs`, which both `lib.rs` and `build.rs` include; the assembly is `assembly.rs`, included by both.
- The differential test of each crate (`build_script_output_is_the_emitters`) compares the files the build script wrote with the output of the emitter run in the test, where every type of the crate is registered. It checks that the build script's type environment gives the same text. The upstream include tests run against the modules of `OUT_DIR`.

**Not converted: the two themes.** *(Since 9.5.13 the Simple theme is compiled by its build script against the type models, with its checked-in file as the reference of a test; reason 1 below is gone for both themes, reason 2 is not, see 9.5.13, "Found", item 4.)* Their compiled markup stays checked in, for three reasons that were settled by reading and not by a build:

1. The document of each theme is the document of a class of the crate (`FluentTheme`, `SimpleTheme`), and the group of the Fluent theme names further types of its crate (`ColorPaletteResources`, `SystemAccentColors`, `DensityStyle`). Point 3 applies to the whole group, because the documents of a theme are one group.
2. A build script of a theme would link `ferroui-controls` and `ferroui-dialogs` for the host with `compiler-metadata`. The browser build (`wasm32`) would then build the framework for the host as well as for the target on every clean build.
3. The checked-in files are what the size and start-up measurements of `browser-platform.md` were taken with, and their differential test already runs the emitter with every type registered.

### 9.7 Registration of compiled XAML

#### 9.7.1 The per-crate table

Real signatures (`ferro_xaml_loader.rs`):

```rust
pub type CompiledXamlLoader = fn(service_provider: Option<&Rc<dyn IServiceProvider>>, uri: &str) -> Option<BoxedValue>;
impl FerroXamlLoader { pub fn register_compiled_xaml(assembly_name: &str, try_load: CompiledXamlLoader); }
```

Generated in `compiled_xaml.rs` (the counterpart of `CompiledAvaloniaXaml.!XamlLoader.TryLoad`):

```rust
pub const ASSEMBLY_NAME: &str = "MyApp";

pub fn try_load(sp: Option<&Rc<dyn IServiceProvider>>, uri: &str) -> Option<BoxedValue> {
    // class with a public parameterless constructor: `new T()` (its constructor populates it)
    if rt::uri_equals(uri, "avares://MyApp/views/MainWindow.xaml") {
        return into_markup_value(crate::views::main_window::MainWindow::new());
    }
    // classless document: Build(CreateRootServiceProviderV3(sp))
    if rt::uri_equals(uri, "avares://MyApp/styles/Controls.xaml") {
        let sp = XamlIlRuntimeHelpers::create_root_service_provider_v3(sp.cloned());
        return into_markup_value(rt::throw(styles__controls_xaml::build(&sp)));
    }
    None
}

/// Called from the crate's `register_types()`.
pub fn register() {
    FerroXamlLoader::register_compiled_xaml(ASSEMBLY_NAME, try_load);
    rt::register_embedded_assets(ASSEMBLY_NAME, &super::ASSETS);          // (new) R8, when embed_assets ran
}
```

Entry rules, as upstream: a document gets an entry when it is public (`x:ClassModifier`, usage not `Merged`-and-private) and has a build method, or its class has a public parameterless constructor (`new:` in `ferro_class_info!`), or a constructor taking the service provider; otherwise warning `XamlLoaderUnreachable`. `rt::uri_equals` **(new)** is `StringComparison.OrdinalIgnoreCase`.

`CompiledXamlLoader` has no error channel; the table uses `rt::throw`. Changing the type to return `Result<Option<BoxedValue>, XamlLoadException>` would let `FerroXamlLoader::try_load_with_service_provider` report build failures of compiled documents as errors instead of panics (open question Q6).

#### 9.7.2 URI rules

- The URI of a document is `avares://<assembly name>/<path relative to the package root, forward slashes>`; it is also the document's base URI (`DocumentInfo::base_uri`, served by `IUriContext`).
- `FerroXamlLoader::try_load_with_service_provider` (existing code): resolves a relative URI against the base URI (`Uri::combine`), asks `IAssetLoader::get_assembly(uri, base_uri)` for the owning assembly, finds its loader by `AssetAssembly::name()`, calls it with `absolute_uri.absolute_uri()`; if that returns `None` it falls back to the runtime loader when one is registered. Consequence: an assembly must be known to the asset loader for its compiled XAML to be found, even when it embeds no assets; `register()` above takes care of that (R8).
- `resm:` is not generated (section 8, question 12).

#### 9.7.3 Includes

| Markup | Same crate | Other crate, target listed in its `.xamlmeta` `documents[]` | Otherwise |
|---|---|---|---|
| `StyleInclude Source=..` / `ResourceInclude Source=..` | `FerroXamlIncludeTransformer` replaces the include with a call node of the target's build method with a `NewServiceProviderNode` argument, or with a `new` of its class; emitted: `crate::compiled_xaml::<doc>::build(&XamlIlRuntimeHelpers::create_root_service_provider_v3(Some(ctx_sp.clone())))?` | the transformer finds the method `Build:<rooted path>` on the synthetic type `CompiledFerroXaml.!FerroResources` of that assembly (`COMPILED_RESOURCES_TYPE_NAME`), which `ModelTypeSystem` synthesises from `documents[]`; emitted: `::other_crate::compiled_xaml::<doc>::build(..)?` (requires the function to be `pub`, which it is for public documents) | the include object stays; at run time `StyleInclude::loaded` / `ResourceInclude` call `FerroXamlLoader::load` with the context base URI, which reaches the other crate's registered table |
| `MergeResourceInclude Source=..` | `XamlMergeResourceGroupTransformer` inlines the target's resources at build time; nothing is emitted for the include itself | error, as upstream (merging works within one assembly) | error |

Implemented (E5 step 2, 2026-10-08), for the checked-in generated files of the interim integration (9.10.1):

- The model is `rust_emitter::XamlMetadata` (`AssemblyModel` with `name`, `crate_name`, `documents[DocumentModel]` and `dependencies`, JSON). `GeneratedFile::metadata` and `ClassFile::metadata` write it next to the generated file (`compiled_xaml.xamlmeta`, drift-tested with it); `XamlMetadata::read` reads a file and, transitively, the files its `dependencies` name (relative to it, or absolute). Since E5 step 3 the file travels through Cargo `links` metadata (9.6.8): the build script of a dependent crate reads `DEP_<CRATE>_XAML_XAMLMETA`, also for the themes, whose file is still checked in.
- `ModelTypeSystem` does not exist yet, so the type system is `CompiledMarkupTypeSystem`: the run-time type system the emitter already transforms with, plus, on the assembly of each dependency, the type `CompiledFerroXaml.!FerroResources` with one static `Build:<rooted path>` method per document of `documents[]` with a build function (public when the document is public, returning its root type, taking the service provider). `XamlIncludeGroupTransformer` finds it there exactly as upstream finds the method; the emitter calls the function the method stands for (`::other_crate::compiled_xaml::build_<document>(..)?`). A class document of another crate is found as upstream finds it, as the type named after the path of the document, and created with its class.
- `compile_documents`, `generate_file` and `generate_class_file` take the dependencies; `generate_class_file` leaves the documents of a dependency out of its group (an include of one calls it in its crate). The emitter's transform has no run-time include fallback (decision 22): an include of a document of an assembly without `.xamlmeta` is upstream's error.
- Deviation: the port's `Uri` lower-cases the host (the assembly name) where upstream's registered resource URI parser keeps its case, so the name upstream looks the class of a document up by (`Path.GetFileNameWithoutExtension(assetPath.Replace('/', '.'))`) can differ from the class in case. The class of a document listed in `documents[]` is found by that name compared without regard to case (`compiled_resources.rs`).
- Fixture: `tests/XamlIncludeFixture` (9.10.1).

#### 9.7.4 Chaining from `register_types()`

Registration is explicit (decision 6). A crate with compiled XAML adds one line to its `register_types()`:

```rust
pub fn register_types() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        ferroui_controls::register_types();          // crates below first
        TypeInfo::register_namespaces(NAMESPACES);
        TypeInfo::register_all(TYPES);
        MarkupType::register_all(MARKUP_TYPES);
        MarkupAssembly::register(&ASSEMBLY);
        crate::compiled_xaml::register();            // compiled documents + embedded assets
    });
}
```

For applications, `export_metadata()` can generate the whole function (`$OUT_DIR/register_types.rs`, included by `ferroui::include_register_types!()`) from the scan: namespace table, class list, markup types, assembly, dependency chain, compiled XAML. The generated list is also what the existing "list equals sources" test of `ferroui-base` checks by hand today.

### 9.8 Node-by-node emit plan

`t` is the current target, `sp` the context as `Rc<dyn IServiceProvider>` (`ctx_sp` in the example), `ctx` the `XamlIlContext`. "Interpreter" names the function that defines the semantics.

#### 9.8.1 `xamlx` core nodes (`runtime/interpreter/evaluators.rs`)

| Node | Interpreter | Emitted Rust | Open issues |
|---|---|---|---|
| `XamlAstNewClrObjectNode` | evaluate arguments as parameter types, `call_constructor` | `let x: Ref<T> = T::new();` / named constructor path (form B) / `rt::construct(MARKUP, &[..])?` (form C) | constructor overloads are distinguished by parameter types only; `.xamlmeta` must keep declaration order |
| `XamlAstTextNode` | boxed `String` | `String::from("..")` or `".."` | escape through `{:?}` formatting |
| `XamlStaticOrTargetedReturnMethodCallNode`, `XamlNoReturnMethodCallNode` (method call base) | target or first argument as `this`, arguments as parameter types, `call_wrapped_method`; error if a value is expected from `void` | call by form A/B/C; result bound to a `let` when used | — |
| `XamlPropertyAssignmentNode` | `assignment_plan`, single setter or dynamic dispatch | 9.3.3 | plan code must be shared (E0) |
| `XamlPropertyValueManipulationNode` | getter on target, manipulate result | `{ let v = <getter>(&t); <manipulation on v> }` | — |
| `XamlManipulationGroupNode` | children in order on the same target | statements in order | — |
| `XamlValueWithManipulationNode` | evaluate value, manipulate unless the group is empty | `let x = ..;` + manipulation | — |
| `XamlMarkupExtensionNode` | 9.3.4 | 9.3.4 | — |
| `XamlObjectInitializationNode` | BeginInit, optional push, manipulation, pop, EndInit | 9.3.2 | `needs_parent_stack` shared (E0) |
| `XamlNullExtensionNode` | null of pseudo type | `None` | — |
| `XamlTypeExtensionNode` | `RuntimeTypeValue`, converted at the call site (decision 16) | `T::TYPE` or `ValueType::of::<T>()` by receiving member; boxed per decision 16 for `object`; `ValueType::of::<Option<BoxedValue>>()` for `System.Object` (9.4.7) | generic type arguments on the build path |
| `XamlStaticExtensionNode` | static property getter or field value | enum variant path, `Owner::name_property().as_property()`, `Owner::name_event()`, declared static getter (B/C) | literal fields of core types |
| `XamlConstantNode` | `constant_value` | literal with suffix (`20.0_f64`, `5_i32`, `true`, `'c'`); enum from numeric value → variant path, flag combination → `A \| B` or `rt::enum_from_value::<T>(n)?` | numeric value that is no member: build error (decision 12) |
| `XamlRootObjectNode` | `root_object_field()` | the `target` parameter / `rt::cast::<Ref<T>>(&ctx.root_object())?` inside deferred builders | — |
| `XamlIntermediateRootObjectNode` | `intermediate_root_object()` | the local holding it, or `ctx.intermediate_root_object()` cast | — |
| `XamlLoadMethodDelegateNode` | `MarkupDelegate` over the method | `rt::method_delegate(&context, n, \|this, arguments\| Class::__markup_Name_k(this, arguments.next()?, ..))`: the typed function of the declared method on the weakly held root object of the context (9.4.4) | O1 |
| `XamlAstCompilerLocalNode` | read local | the `let` name | — |
| `XamlAstLocalInitializationNodeEmitter` | evaluate, store, return | `let name = ..;` | — |
| `XamlValueNodeWithBeginInit` | evaluate, BeginInit | `let x = ..; x.begin_init();` | — |
| `XamlAstManipulationImperativeNode` | execute imperative, ignore target | the statement | — |
| `XamlAstImperativeValueManipulation` | evaluate value, manipulate it | `let v = ..;` + manipulation on `v` | — |
| `XamlAstContextLocalNode` | context as service provider / type descriptor context | `rt::service_provider(&context)` / `rt::type_descriptor_context(&context)` (as implemented, 9.4.6) | `ITypeDescriptorContext` lives in `ferroui_markup_xaml::converters` |
| `XamlAstRuntimeCastNode` | evaluate as object, checked cast | `let cast_n = rt::cast_checked(value, <handle of T>, "T", line, position)?;` for a reference type; the member that takes it converts it (`rt::exact`). A value type is refused (as implemented, 9.4.6) | — |
| `XamlAstNeedsParentStackValueNode` | verify stack, evaluate | the inner expression; the parents are pushed by the initialisation of the objects around it (as implemented, 9.4.6) | — |
| `XamlDeferredContentNode` | `DeferredContentFactory` + customisation method | `rt::defer(<handle of T>, &context, <function>_deferred_<n>, line, position)?` and the free function `<function>_deferred_<n>` | captured outer locals are not possible: the body is a function of its own, rustc rejects a use of an outer local |
| `XamlDeferredContentInitializeIntermediateRootNode` | evaluate, store as intermediate root | `let x = ..; ctx.set_intermediate_root_object(into_markup_value(x.clone()));` | — |
| `XamlDirectCallPropertySetter` (setter) | call method with target + arguments | form A/B/C call | — |
| `AdderSetter` (setter) | getter first, then arguments, then adder | `let c = <getter>(&t); <adder>(&c, v);` | `Add` overloads per item type |
| `XamlWrappedMethod`, `XamlWrappedMethodWithCasts` | plain call; cast arguments from the first differing parameter | call; `rt::cast` on the differing arguments | — |

#### 9.8.2 Framework nodes (`runtime/framework/`, table of `compiler_extensions/mod.rs`)

| Node / setter | Interpreter | Emitted Rust | Open issues |
|---|---|---|---|
| `FerroXamlIlVectorLikeConstantAstNode` | constructor with constant doubles | `Thickness::new(8.0, 4.0, 8.0, 4.0)` (constructor path from the node's constructor) | constructors of value types need form B paths in metadata |
| `FerroXamlIlGridLengthAstNode` | `GridLength(value, unit)` | `GridLength::new(1.0, GridUnitType::Star)` | controls type: path from the model only |
| `FerroXamlIlFontFamilyAstNode` | `FontFamily(context.BaseUri, text)` | `FontFamily::with_base_uri(ctx.base_uri().as_ref(), "..")` | — |
| `FerroXamlIlArrayConstantAstNode` | list of element values | `vec![a, b, c]` (typed) / `Vec<MarkupValue>` when the element type is `object` | array representation per receiving member |
| `FerroXamlIlFerroListConstantAstNode` | new list, capacity, `Add` each | `let l = FerroList::<T>::new(); l.add(a); ..` | no capacity setter was found on `FerroList`; the interpreter calls the method the node carries (`list_set_capacity_method`), the emitter does the same through form C or omits it if it is a pure hint (to be decided with the node's owner) |
| `XamlIlFerroPropertyNode`, `XamlIlFerroPropertyFieldNode` | load the property definition | `Owner::name_property().as_property()` (or without `.as_property()` where the typed definition is wanted) | — |
| `XamlIlFerroClassProperty` | `GetClassProperty(className)` | `ClassBindingManager::get_class_property("name")` | — |
| `BindingSetter`, `BindingWithPrioritySetter` | `bind_binding` | 9.3.3 | binding value form: decision 17 (`rt::binding_of`) |
| `SetValueWithPrioritySetter` | `set_value_untyped(.., priority)` | `set_value_with_priority` | — |
| `UnsetValueSetter` | set the unset marker | 9.3.3 | — |
| `try_get_provide_value_target` | registered property / property info / name | 9.3.4 step 2 | `PropertyInfoValue` for plain properties needs a property info: form C `rt::property_info(MARKUP, "Name")` |
| `FerroXamlIlContextNameScopeField` | `FerroNameScopeField` filled from the parent provider | `ctx.name_scope()` (field of `XamlIlContext`, filled in `new`) | — |
| `FerroXamlIlContextEagerParentStackProvider` | `IFerroXamlIlEagerParentStackProvider` on the context | implemented by `XamlIlContext` | — |
| `XamlIlSelectorInitialNode` | `None` | `None` | — |
| `XamlIlTypeSelector` | `Selectors::of_type_info` / `is_type_info` | same calls with `T::TYPE` | — |
| `XamlIlStringSelector` | `Selectors::class` / `name` | same | — |
| `XamlIlCombinatorSelector` | `Selectors::child` / `descendant` / `template` | same | — |
| `XamlIlNotSelector` | `Selectors::not(previous, argument)` | same | null argument is a build error |
| `XamlIlNthChildSelector` | `Selectors::nth_child` / `nth_last_child` | same | — |
| `XamlIlPropertyEqualsSelector`, `XamlIlAttachedPropertyEqualsSelector` | `Selectors::property_equals_untyped(previous, property, exact value)` | `Selectors::property_equals(previous, Owner::p_property(), v)` when the value is typed, else the untyped form with `rt::exact_property_value` | typed form's parameter list *(unverified)* |
| `XamlIlOrSelectorNode` | single alternative or `Selectors::or(list)` | `Selectors::or([a, b])` | — |
| `XamlIlNestingSelector` | `Selectors::nesting(previous)` | same | — |
| `XamlIlQueryInitialNode` | `None` | `None` | — |
| `XamlIlWidthQuery`, `XamlIlHeightQuery` | `StyleQueries::width` / `height(previous, operator, value)` | same | operator enum path |
| `XamlIlOrQueryNode`, `XamlIlAndQueryNode` | single member or `StyleQueries::or` / `and` | same | upstream `and` quirk preserved (decision 15) |
| `XamlIlTypeQuery`, `XamlIlStringQuery`, `XamlIlCombinatorQuery` | builder method called through the type system | form C call of the same builder method | never created upstream; emit kept for table completeness |
| `XamlIlDirectCallPropertySetter` | `setter.set_Value(object)` | form B/C call with `into_markup_value(v)` | — |
| `FerroNameScopeRegistrationXamlIlNode` | scope must exist and not be completed; `register` | `rt::register_name(ctx.name_scope(), name, &t)?` | — |
| `ClassValueSetter` | `target.Classes.Set(name, value)` | as implemented: `let classes_n = StyledElement::__markup_get_Classes(..); Classes::__markup_Set_3(&classes_n, name, value);`, the two typed functions of the declared members the interpreter calls | — |
| `ClassBindingSetter` | `BindClass(target, name, binding, null)` | as implemented: `let _ = ClassBindingManager::__markup_BindClass_0(target, name, binding, None);`, the typed function of the declared method; the subscription it returns is dropped | — |
| `FerroAttachedInstancePropertySetterMethod` | `set_value_untyped(Field, value, LocalValue)` | typed `t.set_value(Owner::p_property(), v)` when the value type is exact, else untyped | setter arity quirk (decision 15) |
| `FerroAttachedInstancePropertyGetterMethod` | `GetValue(Field)` | `t.get_value(Owner::p_property())` | — |
| `XamlDirectCallAddHandler` | `AddHandler(event, handler, Direct \| Bubble, false)` | `rt::add_handler(&t, &Owner::__markup_field_NameEvent(), delegate);` (`add_handler_untyped` with `Direct \| Bubble`, `false`; 9.4.4) | a choice of the setter at run time is refused |
| `InjectServiceProviderNode` | context as service provider | `sp.clone()` | — |
| `OptionsMarkupExtensionMethod` | first branch whose condition holds, else default; only tested options and the chosen value are evaluated | `if <cond>(&sp, opt1) { v1 } else if .. { .. } else { default }` as an expression; `OnPlatform` branches may be folded to `cfg!` later | conversion of the branch value to the return type per branch |
| `ResourceAdderSetter` | dictionary (optional getter), key, value, `Add*`, optional source info | `let d = ..; let k = ..; d.add_deferred(k, value); XamlSourceInfo::set_xaml_source_info_for_key(..)` | key forms (`&str`, `&'static TypeInfo`, `ValueType`) → `ResourceKey` |
| `EnsureCapacityNode` | if it is a `ResourceDictionary`: `ensure_capacity(count + n)` | `d.ensure_capacity(d.count() + n);` behind `rt::as_resource_dictionary` when the static type is not the class | — |
| `HandleRootObjectScopeNode` | `SetNameScope` for styled elements, `complete()` | as in the example; the `is StyledElement` test is resolved statically when the root type is known | — |
| `XamlSourceInfoValueManipulation` | `SetXamlSourceInfo(target, new XamlSourceInfo(line, pos, doc))` | `XamlSourceInfo::set_xaml_source_info(&into(t), Some(XamlSourceInfo::new(l, c, Some(SOURCE))));` | only when `CreateSourceInfo` (decision 21) |
| `NewServiceProviderNode` | `CreateRootServiceProviderV3(context)` | `XamlIlRuntimeHelpers::create_root_service_provider_v3(Some(sp.clone()))` | — |
| `XamlIlBindingPathNode` | builder, transform elements then elements, `build()` | `CompiledBindingPathBuilder::new()...build()` | typed emission (`try_enable_typed_emission`) → `typed_property` |
| `XamlIlNotPathElementNode` | `.not()` | `.not()` | — |
| `XamlIlStreamObservablePathElementNode`, `XamlIlStreamTaskPathElementNode` | `.stream_observable()` / `.stream_task()` | same | element type argument unused |
| `SelfPathElementNode` | `.self_()` | same | — |
| `FindAncestorPathElementNode`, `FindVisualAncestorPathElementNode` | `.ancestor(Some(class), level)` / `.visual_ancestor(..)` | `.ancestor(Some(T::TYPE), n)` | — |
| `ElementNamePathElementNode` | `.element_name(NameScopeRef(scope), name)` | `.element_name(rt::required_name_scope(&ctx)?, "name")` | — |
| `TemplatedParentPathElementNode` | `.templated_parent()` | same | — |
| `XamlIlFerroPropertyPropertyPathElementNode` | `.ferro_property(p)` / `.ferro_property_with(p, true)` | same with `Owner::p_property().as_property()` | — |
| `XamlIlClrPropertyPathElementNode` | `.property(info, inpc factory)` with a metadata-backed `RuntimePropertyInfo` | `.notifying_property::<O, K>("Name", \|o\| o.name(), \|o, v\| o.set_name(v))` / `notifying_read_only_property` (decision 13); `.typed_property` for the single-element typed case; `.property(rt::property_info(MARKUP, "Name"), ..)` when the accessors are form C | owner without `INotifyPropertyChanged` needs a non-notifying builder entry point; `accepts_null` variant for the typed forms |
| `XamlIlClrMethodPathElementNode` | not supported at run time (`method_untyped` missing) | closure-based builder call **(new)** | builder has no entry point yet (R9) |
| `XamlIlClrMethodAsCommandPathElementNode` | not supported at run time | `.command::<O>("Name", \|o, p\| o.method(..), can_execute, &["Dep"])` / `.notifying_command` | parameter conversion from `Option<&BoxedValue>`; the interpreter gap stays (R9) |
| `XamlIlClrIndexerPathElementNode` | `.property(info, indexer or INPC factory)` | `.indexer_property::<O, K>(index, get, set)` or `.property(..)` with form C | non-integer indexers |
| `XamlIlArrayIndexerPathElementNode` | `.array_element(&values)` | same | — |
| `TypeCastPathElementNode` | `.type_cast_value(CastTarget::Class(..) / Value(..))` | same with `T::TYPE` / `ValueType::of::<T>()` | — |
| `XamlIlClrPropertyInfoEmitter`, `XamlIlPropertyInfoAccessorFactoryEmitter`, `XamlIlTrampolineBuilder` | absorbed by binding-path evaluation | absorbed by the builder calls above; the per-assembly cache of property infos becomes a `thread_local!` `OnceCell` per distinct property in `compiled_xaml.rs` (optional optimisation) | — |

Document-level methods (`runtime/framework/methods.rs`): `DocumentBuildMethod` / `DocumentPopulateMethod` → calls of the generated `build` / `populate` functions by path; `DeferredTransformationFactoryMethod` → the direct `deferred_transformation_factory_v3::<T>` call.

### 9.9 Needs from core / runtime library

Runtime library (`ferroui-markup-xaml`):

| # | Item | Why |
|---|---|---|
| R1 | `xaml_il::runtime::XamlIlContext`: the context of compiled documents, with the members and service resolution order of the interpreter's `RuntimeContext` (inner provider, own services, static providers, parent provider), implementing `IServiceProvider`, `IRootObjectProvider`, `IProvideValueTarget`, `IUriContext`, `ITypeDescriptorContext`, `IFerroXamlIlParentStackProvider`, `IFerroXamlIlEagerParentStackProvider`, with the name scope field and the inner service provider (`create_inner_service_provider_v1`) set up in `new`. Preferably the interpreter's `RuntimeContext` is *moved* here and the interpreter uses it, so that one implementation serves both back ends | no context type exists outside the loader's `runtime` feature |
| R2 | `compiled::DocumentInfo` (base URI, static namespace table) providing `IFerroXamlIlXmlNamespaceInfoProvider` | upstream `NamespaceInfo:<name>` type |
| R3 | `compiled::{cast, is_instance, exact_property_value, binding_of, no_setter, at, throw, uri_equals, parent_root_object, register_name, complete_name_scope, required_name_scope, as_resource_dictionary, type_descriptor_context}`: the helpers the interpreter has in `runtime/framework/helpers.rs` and inline, with identical error texts; `binding_of` and `throw` exist as `pub(crate)` | shared semantics of checked casts, dynamic setters, name scopes |
| R4 | `compiled::{set_property, get_property, call_method, construct, property_info, enum_from_value}` over `&'static MarkupType` by member name | call form C (9.5.3) |
| R5 | `FromXamlStr` (+ `from_xaml_str`, `set_from_xaml`), `MaybeSupportInitialize`; `FromXamlStr` implemented by the declaration macros for every type with `parse:` | trait-directed fallbacks (9.5.4) |
| R6 | `FerroXamlLoader::{set_populate_override, populate_override}` keyed by `&'static TypeInfo`, behind feature `hot-reload` | `!XamlIlPopulateOverride` |
| R7 | `include_xaml!`, `include_compiled_xaml!`, optional `include_register_types!` (`macro_rules!`) | inclusion forms |
| R8 | registration of an assembly and its embedded asset index with the asset loader (`StandardAssetLoader` has only `new(assembly)` today), so that `IAssetLoader::get_assembly` resolves `avares://<Assembly>/..` for crates with compiled XAML | URI dispatch (9.7.2), `embed_assets` |
| R9 | `CompiledBindingPathBuilder`: entry points for a method as delegate, a non-notifying plain property with typed accessors, `accepts_null` on the typed forms; untyped `method`/`command` entry points for the interpreter | binding path rows of 9.8.2 |
| R10 | `CompiledXamlLoader` returning `Result` (optional, Q6) | error channel of the table |
| R11 | a canonical object-tree dump (`ferroui_markup_xaml::testing::dump_tree(root) -> String`): types, locally set registered properties with priority, bindings (kind, path text, mode), classes, name scope contents, resources (keys, deferred or not), styles (selector text, setters), templates (instantiated once), handlers per routed event (count). The existing `testing::dump_tree` of the loader dumps the AST, not objects | conformance (9.10.2) |

Core (`ferroui-base`, `ferroui-controls`):

| # | Item | Why |
|---|---|---|
| C1 | Metadata callables of framework types as `pub` paths (`Type::function`) wherever possible, so that form B applies; a `--report-invokers` run lists the rest | typed calls |
| C2 | `ferro_properties!` bodies stay in the scanner-readable shape (`FerroProperty::register*::<Owner[, Host], T>("Name", ..)` with builder calls on `StyledPropertyOptions`); a test per crate compares scanned registered properties with `TypeInfo::properties()` | registered properties are not in metadata (decision 3), the build tool reads them from source |
| C3 | Every enum and value type reachable from XAML has `ferro_markup_enum!` / `ferro_markup_type!` with `handles:` and, for constants the loader builds, a `pub` constructor path | constants as constructor calls |
| C4 | `register_types.rs` keeps `NAMESPACES`, `TYPES`, `MARKUP_TYPES`, `ASSEMBLY` as literal tables | scanner input |
| C5 | A fallible name registration (`INameScope` has no `try_register`; a duplicate panics) — same limitation as the interpreter notes | error parity with upstream's exception |
| C6 | Weak capture for handlers created from markup in the interpreter (`load_method_delegate`) | O1 |

Loader (`ferroui-markup-xaml-loader`), phase E0:

| # | Item |
|---|---|
| L1 | Move `assignment_plan`, the conversion decision of `EvalContext::convert`, `ParentStackVisitor` and the build-method eligibility check of `Interpreter::build` out of `runtime/` into back-end-neutral functions returning decisions (enums), used by both back ends |
| L2 | `MarkupModel` + `ModelTypeSystem<B>` (9.5.5) outside the `runtime` feature; data tables of `core_types.rs` / `object_model.rs` shared |
| L3 | `FRAMEWORK_EMITTERS` table test next to `FRAMEWORK_EVALUATORS` |

### 9.10 Phasing and tests

#### 9.10.1 Phases

| Phase | Content | Exit test |
|---|---|---|
| **E0 Foundations** | L1, L3; R1 (context moved to the runtime library, interpreter re-seated on it); R3; R11 (dump) and the dump of all existing interpreter test documents recorded as golden files | interpreter test suite unchanged and green; golden dumps stable |
| **E1 Type system at build time** | scanner for the five declaration forms + `register_types.rs`; `MarkupModel`, `.xamlmeta` writer/reader, `ModelTypeSystem<EmitBacking>` (L2); `export_metadata()`; drift test against `RuntimeTypeSystem` for base, controls, markup-xaml | type-system dumps equal for the three framework crates |
| **E2 MVP through the emitter** (section 7.1 MVP) | `RustWriter`; core node emitters of 9.8.1 except deferred content; framework: constants, registered-property nodes and setters, `ClassValueSetter`, attached instance setter, `XamlDirectCallAddHandler` + delegate node, name scope registration, root object scope; parts struct, `initialize_component`, `include_xaml!`; `compile_xaml()` with diagnostics and change tracking; call forms A and C only | the MVP `MainWindow.xaml` builds through `build.rs`, named field populated, click handler runs, dump equals the interpreter's |
| **E3 Styles, resources, templates** | selectors and queries, setter transformer setter, `ResourceAdderSetter`, `EnsureCapacityNode`, deferred content, `XamlSourceInfoValueManipulation`, markup extensions with dynamic setters (`StaticResource`, `DynamicResource`), `OptionsMarkupExtensionMethod`, control themes; call form B | a styled, templated Button from XAML: equal dumps (phase 4 exit test of 7.1) |
| **E4 Bindings** | binding path node and all elements, typed emission, `ClassBindingSetter`, `BindingWithPrioritySetter`, template bindings; R9 | ported upstream compiled-binding XAML tests pass through the emitter |
| **E5 Includes and assets** | group transformers at build time, `compiled_xaml.rs`, loader table, cross-crate includes via `documents[]`, `embed_assets()` + R8, generated `register_types` | a two-crate fixture (theme library + application) loads `StyleInclude`, `ResourceInclude`, `MergeResourceInclude`; then the Fluent theme documents |
| **E6 Hot reload** | R6, feature `hot-reload`, previewer hook calling the interpreter's `populate` through the override; rule for when a rebuild is required (set of names/handlers changed) | editing a document of a running application swaps content and refills the parts struct |

E0 and E1 are independent and can run in parallel; E2 needs both.

**E5 status (2026-10-07).** Done: the loader table. `rust_emitter::generate_class_file` ends the generated file of a class with `try_load`, the `CompiledXamlLoader` of its documents (upstream's `!XamlLoader.TryLoad`, `XamlCompilerTaskExecutor`): one entry per public document of the group, in group order, the URI compared with `rt::uri_equals` (`OrdinalIgnoreCase`). Whether a document is public is read from its `x:ClassModifier` as upstream reads it (`Public`; `NotPublic` or `Internal`; absent means public; another value is the upstream error; a class document whose modifier is not public is the upstream error "XAML file x:ClassModifier doesn't match the x:Class type modifiers."). The document of the class is created with its constructor, a classless public document is built by its build function with `CreateRootServiceProviderV3(serviceProvider)`; a public document the group merged into another one is still emitted as a function of its own so that the table can answer it, and a merged document that is not public is not emitted, as upstream removes it without compiling it. The themes register the table in `register_types()` and no longer register their compiled documents as assets; the feature `remove-compiled-documents` is gone (browser-platform.md, section 20, item 4; DEVIATIONS.md, "Corrected divergences").

Interim seam: upstream's compiler picks the constructor of the class from the assembly (a public parameterless constructor, else a public constructor whose single parameter is `IServiceProvider`). The port has no build-time type system of the crate being compiled yet (9.5), and the markup metadata of a class lists both `new()` and the service-provider form where upstream has the one constructor with an optional parameter, so the caller of `generate_class_file` states the constructor upstream's class has (`ClassConstructor::ServiceProvider("with_service_provider")` for `FluentTheme(IServiceProvider? sp = null)` and `SimpleTheme(IServiceProvider? sp = null)`). The choice moves into the compiler with `compile_xaml()` of 9.6.

**E5 status (2026-10-08).** Done: includes across crates (pull request #53). A crate with compiled markup describes its documents in `compiled_xaml.xamlmeta`, and `StyleInclude`, `ResourceInclude` and `MergeResourceInclude` of the documents of another crate are resolved at build time through it (9.7.3): a style or resource include of a public document calls the document's build function in its crate, an include of a class document creates the class, and a merge include of a document of another crate is upstream's error (merging works within one compilation). `XamlIncludeGroupTransformer` and `XamlMergeResourceGroupTransformer` were compared with upstream line by line; they differ only by the run-time include fallback of decision 22, which the emitter's transform does not install. Also in this step: `generate_file` links the documents of its group (until now only the class files did), source information (`CreateSourceInfo`) is emitted, build function names are snake case for any document name, and `RuntimeTypeSystem::find_assembly` compares names as upstream's `SreTypeSystem` does (it matched substrings). The themes write their `.xamlmeta`.

The fixture is `tests/XamlIncludeFixture`: the library `xaml-include-fixture-theme` (the assembly `Tests` of upstream's test URIs) and the application `xaml-include-fixture-application` (the assembly `アセンブリ` of upstream's non-Latin test), which includes documents of the library and of both themes. The application runs upstream's `ResourceIncludeTests`, `StyleIncludeTests` and `MergeResourceIncludeTests` against compiled documents under their names, with the run-time loader not registered; a document a test loads that upstream places in the assembly `Tests` (a relative include, a merge) is compiled with the library. Where two of upstream's tests give one URI to different documents, the documents of the later test are under a folder named after it (the library's `documents.rs`).

Seam, unchanged from step 1: the constructor of a class is the one the markup metadata declares; the themes declare a parameterless constructor and one that takes the service provider, where upstream's class has one with an optional service provider, so an include of a theme's class document creates it with `new()`, where upstream passes the service provider.

**E5 status (2026-10-08, step 3).** Done: build integration over the run-time type system (9.6.8). `ferroui-build` (`Build::from_env()`, `compile_xaml()`, `embed_assets()`, `run()`) compiles the documents of a crate in its build script, writes them to `OUT_DIR` with `register()` (R8) and the `.xamlmeta`, and the `.xamlmeta` travels through Cargo `links` metadata. The include fixture is converted (the library except the document of its class, the application whole); the themes export their checked-in `.xamlmeta` through `links` and are otherwise unchanged.

The seam of step 1 is closed in the compiler: `generate_class_file(class, None, ..)` picks the constructor as `XamlCompilerTaskExecutor` does, from the constructors the type system projects for the class (`ClassConstructor::of`): a public parameterless constructor (`T::new()`), else a public constructor whose single parameter is the service provider (the typed function of the declared constructor, `T::__markup_new_<n>`); a class with neither has no entry and upstream's `XamlLoaderUnreachable` warning (`ClassFile::warnings`). The two themes still state the constructor (`Some(ClassConstructor::ServiceProvider("with_service_provider"))`): their markup metadata declares `new()` next to the constructor that takes the service provider, where upstream's class has the one constructor with an optional parameter, so the rule would pick `new()`. Removing `new:` from the metadata of the themes closes that, and needs a build to check that `<FluentTheme/>` in markup then goes through `FerroXamlIlConstructorServiceProviderTransformer`.

Not done, and why (9.6.8): a build script cannot compile a document that names a type of its own crate, so `x:Class` documents stay generated by a test and checked in; the generated `register_types` of 9.7.4 needs the source scan. Both wait for the scanner and `ModelTypeSystem` of 9.5.

`XamlIlTests`: the two tests that create a class with compiled markup of the test assembly (`Parser_Should_Override_Precompiled_Xaml`, `Custom_Properties_Should_Work_With_XClass`) are ported and not ignored; the constructors of `XamlIlClassWithPrecompiledXaml` and `XamlIlClassWithCustomProperty` populate the instance through the run-time loader (`FerroXamlLoader::load_object`). Compiling the two documents is the checked-in path (`generate_class_file` in a test of the XAML test crate, as the fixture does for `StyleWithServiceProvider`), not the build script, for the reason above. It was not done in this step: the first generated file has to exist before the crate compiles, which takes a build.

**E1 status (2026-10-09, the scanner, first stage).** Written, not built by its author: the model and its file (`.xamlmeta` format 2), and the source scanner (9.5.6). Tested by a fixture source tree (`src/FerroUI.Build.Tasks/tests/fixtures/scanner`, never compiled) with exact models, diagnostics and counts, and by a scan of `src/FerroUI.Base` and `src/FerroUI.Controls` as files that checks that no invocation of a declaration macro is skipped and prints the numbers of each scan. Not in this stage: `ModelTypeSystem`, the call forms, the manifest keys, and any change to `Build` or the emitter.

**E1 status (2026-10-09, dependency models and the `ModelTypeSystem`, second stage).** Written, not built by its author (9.5.7): the export table of a crate in its model, the models of several crates as a set, the scan with the models of the dependencies, `Build::export_metadata()`; the closed table of runtime library types as data both type systems define their types from; `ModelTypeSystem` over the models; the drift test against `RuntimeTypeSystem` for the base and the controls crates. Stages 1 to 3 of the list below are done as far as they can be without a build: stage 2 without the build scripts of the framework crates (9.5.7 says what that switch needs), stage 3 with the drift test written and not yet run, so its exit ("the dumps equal") is the first work of the validating session and of the next stage. `markup-xaml` is not in the drift test yet (add its scan and its registered types to `CRATES` once the two crates agree).

**E1 status (2026-10-09, the two type systems aligned, third stage).** Built and run by its author with the three commands of the stage (9.5.8): the drift test over the base and the controls crates has nothing left in a kind that must be empty and is not ignored; a class its crate does not register and a declaration under a `cfg` condition are the known kinds. The model states the type an accessor is a function of, the classes a crate leaves out of its list, the registered handles and casts and the type aliases; the values of all enumeration members of the two crates are evaluated. The exit test of E1 ("the dumps equal") holds for base and controls; `markup-xaml` is still to be added to `CRATES` of the drift test (its scan needs the models of both crates, and its `register_types()` has a `TYPES` list of the third form the scanner reads).

**E1 status (2026-10-09, the emitter against the models).** Built and run by its author with the seven commands of the stage. Done (9.5.11): `markup-xaml` is in the drift test and nothing differs over the three framework crates (893 types; every kind that must be empty and both open kinds at nothing; the known kinds are the one type under `cfg` and the five classes that are not registered); the transform of the corpus agrees for all 109 documents, asserted. Done (9.5.12): `EmitTypes` over the models (`ModelEmitTypes` in `ferroui-build`), with the registrations of the untyped value conversions read by the scanner; the corpus emitted through both implementations is the same byte for byte for all 109 documents and as a whole file, and a question the models cannot answer refuses the document instead of changing its code (shown by a test). The exit test of E1 holds for the three framework crates, and stage 4 of the list below is done for the corpus. Not done: `compile_xaml()` on the model (stage 5; 9.5.12 says what it involves, in order), so `StyleWithServiceProvider` of the include fixture stays checked in and the run-time type system is the only host of the build; the themes and the fixture through the model path are not measured; the compile-time value parser of the build is not written.

**E1 status (2026-10-09, the call forms; the emitter's seam; fourth stage and the start of the fifth).** Built and run by its author with the seven commands of the stage. Done (9.5.9): the call form of every callable is chosen at the end of the scan and written into the model, with the public functions of a crate for the crates built on it; the test of the real crates prints the numbers per form. Done (9.5.10): the emitter reads the type system through `EmitTypes` and transforms with `transform_group` against any type system; the run-time type system is the one implementation and the one host, and every checked-in output (the corpus, both themes, the include fixture) is regenerated without a difference. Not done: `EmitTypes` over the models, and with it the differential of the two paths; `compile_xaml()` on the model (stage 5 of the list below); `markup-xaml` in the drift test, which the first measurement of the transform against the models shows is the next step (0 of 109 documents: the configuration of the language stops at a constructor of `XamlSourceInfo`). `StyleWithServiceProvider` of the include fixture stays checked in.

**E1 status (2026-10-09, `compile_xaml()` on the model).** Built and run by its author with the commands of the stage (9.5.13). Done: the model, the scanner and the export in a leaf crate (`ferroui-build-scan`) that the framework crates take as a build dependency; `ferroui-base`, `ferroui-controls`, `ferroui-markup-xaml` and `ferroui-dialogs` export their models from their build scripts, the Fluent theme its model with its checked-in documents; `Build::type_system(TypeSystem::Model)` compiles the groups of a crate, the group of a class of the crate among them, through `ModelTypeSystem` and `ModelEmitTypes`; the include fixture is whole on it (the checked-in class document is gone) and the Simple theme compiles its 82 documents in its build script, each output the same byte for byte as the emitter's against the run-time type system, no document refused. Stage 5 of the list below is done for those consumers and stage 6 for the framework crates and one theme. Not done: the Fluent theme, the dialogs and the catalog on the models; the emitter out of the `runtime` feature (a build script that uses `ferroui-build` still builds the controls for the host); the compile-time value parser, the manifest keys, diagnostics with codes, the cache.

**E1 status (2026-10-09, the compiler without the framework; the Fluent theme; diagnostics).** Built and run by its author with the commands of the stage (9.5.14, 9.6.5). Done: the transform and the emitter are the `compiler` feature of the loader, which links neither the XAML runtime library nor the controls (what they shared with the interpreter is `back_end`); `ferroui-build` takes it alone and its run-time host is the feature `runtime-host`, which no crate enables; every emitted output is unchanged (the Simple theme's 82 documents, the corpus 109 of 109, the fixtures). The Fluent theme is compiled by its build script: 86 documents, all identical to the checked-in reference, none refused. The diagnostics of a build have upstream's codes and go through the filter of upstream's task. Stopped: the class document of the dialogs is refused for its event handler (`Click="Button_OnClick"`, 9.4.4), which the emitter has no rule for; the dialogs still link the run-time loader for that document.

**E5 status (2026-10-09, event handlers; the dialogs).** Built and run by its author with the commands of the stage. Done (9.4.4): the emitter's rule for a method of the root object named in markup, in both hosts: a routed event, an event that is not a routed event, a property of a delegate type, a handler inside a template, with the method found and checked by the transform as at run time; a method that is not found is one diagnostic that names the document, the member and the method. Done (9.5.15): the dialogs are compiled by their build script and no longer link the run-time loader. Every output that existed is unchanged (the corpus, both themes, the fixtures). Measured, not converted: the ControlCatalog against the models (HANDOVER.md, section 18): of its 219 documents the emitter writes 84 today and 156 once four registrations of the colour picker are read by the scanner (emitted, not yet compiled by rustc); the reasons of the others, with counts, are the work list of item 2 below.

**E5 status (2026-10-09, the work list of the catalog).** Built and run by its author with the commands of the stage (HANDOVER.md, section 19). Done: the scanner reads the registrations a macro makes for each type it is invoked with and what a macro of another crate declares (9.5.16); the emitter's rule for a text a type converter converts when the document is loaded (9.4.6) and for a class set in markup (9.8.2), in both hosts, with ten corpus documents more (119, all the same bytes by both hosts and the same trees as the run-time loader's). The measure of the catalog: 195 of 219 documents emitted (84). A first real compile (9.5.17): five pages of the sample, with their classes, compiled by a build and by rustc and compared with the run-time loader. It found emitted code rustc refuses, for one cause, which the next stage starts with. Every output that existed is unchanged.

**E5 status (2026-10-09, the catalog compiled by rustc).** Built and run by its author with the commands of the stage (HANDOVER.md, section 20). Done: what dereferences to its base is a registration both hosts read (9.5.18); the fixture compiles everything the compiler emits for the catalog, rustc accepts all of it, and every compiled document with a class is the tree of the run-time loader (9.5.19: 206 of 219 documents, 205 of 205 trees); the type of an untyped value as a value (9.4.7) and a compiled binding path over a property whose type generated code cannot name (9.4.8), in both hosts, with corpus documents (122, all the same bytes by both hosts and the same trees as the run-time loader's). Refused: 13 documents (lists and arrays created in markup 5, a method as a command 3, container queries 2, the `PipsPager` case 1, `App.xaml` for the colour picker 1, the class that is not ported 1). Not started: those refusals and the colour picker compiled by its build.

Remaining for E5, in order:

1. The rest of the build-time type system (9.5), in stages that each build and test on their own:
   1. **Validate the scanner on the real crates.** Run the test of 9.5.6 and read its output: the `FRN9010` list is the list of forms the readers do not know (each is either a reader to extend or a declaration to rewrite), the unresolved paths are mostly names behind `use ferroui_base::*` (replace the glob by a list in those files, or resolve globs of dependencies in step 2), `FRN9024` shows where the scanner's public paths differ from `rust_paths.rs` (when none differ, the generated `rust_paths.rs` and `scripts/rust_paths.py` can go). Record the numbers in 9.5.6.
   2. **Dependency models.** `export_metadata()`: `Build` scans the crate, writes the model into the `.xamlmeta` of format 2 (next to the documents it writes today) and reads the models of the dependencies (`AssemblyModel::parse`, the transport of 9.6.3 as it is). With them: canonical paths for types of other crates (each model's types by `rust_path` and by every public path: the scanner has the export table, the model needs it written out), the names of properties whose owner is added across crates, the values of enumerations of other crates, glob imports of other crates. Scan the framework crates in their `build.rs` (they need `links` and a build script; the themes have both).
   3. **`ModelTypeSystem<EmitBacking>` (9.5.5)** in the loader, outside the `runtime` feature: `IXamlType` over `AssemblyModel` (the model moves to the loader or the loader takes it as a trait; `ferroui-build` already depends on the loader). 9.5.2 steps 2 to 4 (the handle table, the structural rules, opaque types), the projection rules of 9.5.5 shared with `runtime/type_system` (`core_types.rs`, `object_model.rs` as data). Exit test: the drift test, the dumps of `RuntimeTypeSystem` and of the scanned `ModelTypeSystem` equal for base, controls and markup-xaml.
   4. **Call forms (9.5.3) and the emitter.** *Done for the corpus (9.5.9, 9.5.11, 9.5.12): the call forms, the seam, the transform, `markup-xaml` in the drift test, `ModelEmitTypes` and the differential of the two hosts (109 of 109 the same). What is left of this stage: the compile-time value parser of the build (9.6.1), the themes and the include fixture through the model path, and a drift test of the descriptions `EmitTypes` hands out. As the stage was planned when it started:* *The call forms are done (9.5.9); of the emitter, the seam and the transform are done and the implementation over the models is not (9.5.10: the table of what `EmitTypes` asks and what the model lacks for each answer is the work list). In order: `markup-xaml` in the drift test; the registrations of `ValueTypes` in the scanner (`register_interface`, `register_upcast`, `register_reference`, `register_nullable`, `register_object`, `register_element_ref`) and one statement of `is_assignable` for both sides; the Rust types of parameters and results, the kind of a declared member, the type of a value and the nullable form of a type on the members and types of `ModelTypeSystem`; `ModelEmitTypes` in `ferroui-build`; the compile-time value parser of the build (9.6.1); then the differential test of the two hosts over the corpus, with `model_transform.rs` as its transform half.* As planned before the stage: The drift test is at nothing in every kind that must be empty and in the open kinds (9.5.8), and is the guard from here on; add `markup-xaml` to it first. Then the scanner (or a pass over the model) fills `call`: A from the declaration, B when `CallableModel::resolved` names a `pub` function of `Scan::functions` with the declared signature, else the typed function (`typed_function`, today's form with `markup-functions`) or C. The emitter reads `MemberSource` of the members of `ModelTypeSystem` (the `EmitBacking` of 9.5.5) instead of `MarkupEmit`, `DeclaredMember` and the registered Rust paths: every place of `rust_emitter` that downcasts to a `Runtime*` member is one to give a second arm, or a trait both member families implement. `Scan::functions` has to be in the model (or the choice made at scan time) for the members of dependencies.
   5. **`compile_xaml()` on the model** *(done as an option of `Build`, 9.5.13: the include fixture and the Simple theme compile on it; the run-time host is still the default. Left of this stage: the compile-time value parser, the manifest keys, `include_xaml!` per class, the generated `register_types`, the diagnostics and the cache. As it stood before:)* *(not started; nothing in the emitter blocks it any more. 9.5.12 lists what it involves: the models of the framework crates in a build script, the class file from files, the compiled documents of dependencies over `ModelTypeSystem`, the option; 9.5.10 says what the build crate links today)*: `Build` builds the `ModelTypeSystem` from its own scan and the models of the dependencies (a crate being compiled has no list of registered classes unless it writes one: the generated `register_types` of 9.7.4 registers every class, so none is marked), wraps it in `CompiledMarkupTypeSystem` for the documents of the dependencies, and the transform and the emitter run against it. The build script stops linking the crates of its documents and compiles `x:Class` documents of its own crate. The intrinsics the transformers evaluate at compile time through the run-time registries (`IXamlCompileTimeValueParser`, 9.6.1) need a build-time implementation. Then the manifest keys (`[package.metadata.ferroui]`: a TOML reader, or the keys stay builder calls), `include_xaml!` per class, the generated `register_types`, the diagnostics and the cache of 9.6.4 and 9.6.5.
   6. **The framework crates export their models and the themes compile in their build scripts** *(done for the base crate, the controls, the XAML runtime library, the dialogs and both themes, 9.5.13 and 9.5.14: the emitter is out of the `runtime` feature of the loader and the Fluent theme compiles in its build script; left: the manifest keys)*: the leaf crate of 9.5.7 (`ferroui-build-scan`: `json`, `model`, `model_set`, `scanner` and the export, with the two text constants and `DocumentModel` owned there, so that the base crate can scan itself), `links` and `build.rs` in the framework crates, the manifest keys of 9.6.1 (`[package.metadata.ferroui]`), then the two themes on `compile_xaml()` with their checked-in files as the differential (the three reasons of 9.6.8 against converting them fall with stage 5: the group no longer needs the theme crate linked, and nothing is built for the host).

   Before step 5, the two `XamlIlTests` documents and the dialogs can use the checked-in path.
2. The ControlCatalog: its documents compiled, so that it does not link the run-time loader (browser-platform.md, section 20, item 3). *(The dialogs are done: 9.4.4 and 9.5.15. The catalog was measured against the models without converting it, and the list was worked down to 24 refused documents of 219; five of its pages compile with rustc and run (9.5.17); then the whole of what is emitted was compiled by rustc and compared, 206 of 219 documents (9.5.19). Sections 18 to 20 of HANDOVER.md have the tables. Before a conversion: the 13 refused documents, the colour picker compiled by its build, and the registrations only the run-time loader makes (9.5.19).)* Measure a few pages first against the estimate there (+6 to +10 MB raw, +0.5 to +1.2 MB gzip on the module); above it, the owner decides.

#### 9.10.2 Test strategy

1. **Emit unit tests** (loader or build crate, no rustc): for small documents over the test type system (`testing/`), the emitted text of a node is compared with a golden string. Fast, pins formatting and ordering.
2. **Compile-and-run conformance**: a fixture crate `tests/xaml-conformance` with a `build.rs` that compiles every document under `fixtures/` and a test per document that (a) builds it with the generated `build`/`populate`, (b) loads the same text with `FerroRuntimeXamlLoader`, (c) compares `dump_tree` of both. The fixture list starts from the documents already embedded in the interpreter's tests (`runtime/interpreter/tests`, `testing/{objects,styles,bindings}.rs`) and the ported upstream XAML tests.
3. **Error parity**: for documents that fail at load time under the interpreter (no matching setter, missing resource, bad cast), the generated function must return an `XamlLoadException` whose message and position equal the interpreter's.
4. **Diagnostics tests**: documents with build-time errors run through `compile_xaml()` in-process; the `cargo::error=` lines are compared with golden files (codes, positions).
5. **Type-system drift test** (E1) in every framework crate.
6. **`trybuild`-style tests** for the rustc-checked places: a handler with a wrong signature, a missing `xaml` field, a missing `include_xaml!`: the test asserts that the error line carries the expected marker comment.
7. **Incremental build test**: touch one `.xaml` of a fixture workspace, assert which generated files changed.
8. **No-upstream-name check** on generated output and on the build crate (`grep -rni` as in the porting guide).

### 9.11 Open issues and questions for the owner

Open technical issues found while writing this note:

- **O1. Handler closures capture the root strongly in the interpreter** (`load_method_delegate` clones the instance into the `MarkupDelegate`), which forms a cycle root → child → handler → root. The emitter design captures weakly; the interpreter should follow. Needs a decision on what a handler does after the root is gone (proposed: nothing).
- **O2. Metadata callables are module-relative** (private helper functions, closures), so typed calls from generated code are possible only for the subset described as form B; everything else goes through the invoker (form C). Acceptable for correctness; the cost shows in hot paths such as large resource dictionaries.
- **O3. No compiled context type exists** in the runtime library; the interpreter's context is behind the loader's `runtime` feature (R1).
- **O4. `IAssetLoader::get_assembly` must know assemblies that only have compiled XAML** (R8); otherwise `FerroXamlLoader::load(uri)` never reaches the registered table.
- **O5. Binding to methods (delegate, command)** is unsupported in the interpreter today (`method_untyped` / `command_untyped` missing); conformance for those documents can only be tested once R9 lands for both back ends.
- **O6. `OrdinalIgnoreCase` URI comparison** in the loader table needs a helper with the same case folding as the group transformers' `invariant_ignore_case_equals`.

Questions for the owner:

1. **Code-behind shape**: confirm option A (parts struct + `include_xaml!` in the class's module, accessor methods `self.xaml.greeting()`), with the fixed field name `xaml`. Should the field name be configurable (`include_xaml!("..", field = ui)`)?
2. **Named-element accessors**: return `Ref<T>` and panic when unset (proposed, closest to a C# field), or `Option<Ref<T>>`?
3. **Handler discovery**: is scanning the inherent `impl` blocks of the `x:Class` type (9.4.4 rule 2) acceptable, or must every handler be declared in `markup: { methods: [..] }` (needed for the interpreter anyway)? If scanning is accepted, should the build tool also *generate* the metadata entries (`$OUT_DIR/..handlers.rs`) so that the interpreter and hot reload see the same handlers without hand-written declarations?
4. **Handler signatures**: is `fn(&self, sender: &Interactive, e: &TArgs)` the one accepted form for routed events, or should `fn(&self)` and `fn(&self, e: &TArgs)` be accepted as conveniences (upstream accepts only the delegate signature)?
5. **Call form C** (untyped invoker from generated code) as the general fallback: acceptable permanently, or should the declaration macros be extended so that every member has a typed, publicly addressable entry point (larger change to `ferro_class_info!` / `ferro_markup_type!`)?
6. **`CompiledXamlLoader` signature**: keep `-> Option<BoxedValue>` (generated table panics on a failing build) or change to `-> Result<Option<BoxedValue>, XamlLoadException>`?
7. **Context type**: move the interpreter's `RuntimeContext` into `ferroui-markup-xaml` as `XamlIlContext` (one implementation, recommended) or keep two and test them against each other?
8. **Type-system unification**: is re-seating `RuntimeTypeSystem` on the shared `MarkupModel` wanted (step 3 of 9.5.5), or should the two projections stay separate with the drift test as the only link?
9. **`build.rs` re-run cost**: `rerun-if-changed=src` re-runs the script on every Rust edit (cached, target under 100 ms). Acceptable, or should `x:Class`/view-model types be required to live in listed files to narrow the trigger?
10. **`links` key in every crate that exports metadata** (including a facade crate): acceptable? It makes two versions of the same crate in one dependency graph a Cargo error.
11. **Generated `register_types()`** for applications (9.7.4): wanted, or stays hand-written?
12. **Hot reload gating**: Cargo feature `hot-reload` on `ferroui-markup-xaml` (proposed) versus always-on override lookup (one thread-local map lookup per `initialize_component`).
13. **`x:FieldModifier="protected"`**: map to `pub(crate)` with a warning (proposed) or reject?
14. **Position mapping for rustc errors**: is the marker comment + `.map.json` enough for the first release, or is a `cargo ferroui` wrapper that rewrites rustc diagnostics to XAML positions in scope?

### 9.12 Provisional rulings on the questions of 9.11 (coordinator, 2026-10-05; the owner may override)

| # | Ruling |
|---|---|
| 1 | Code-behind option A, with the fixed field name `xaml`; not configurable. |
| 2 | Named-element accessors return `Ref<T>` and panic when unset (mirrors upstream's non-null fields). |
| 3 | Scanning the inherent `impl` blocks of the `x:Class` type for handlers is acceptable; the build tool generates the corresponding metadata entries so the interpreter sees the same handlers. |
| 4 | Handler signatures: exactly what upstream's compiler accepts for the event's delegate. Whether shorter forms are accepted is to be verified in the upstream compiler extensions and tests and mirrored, no more. |
| 5 | Call form C (untyped invoker from generated code) is acceptable as the permanent fallback; typed public paths are preferred wherever the declaration allows. |
| 6 | `CompiledXamlLoader` returns `Result`. |
| 7 | `RuntimeContext` moves into the runtime library as the single `XamlIlContext` used by both back ends. |
| 8 | `RuntimeTypeSystem` is re-seated on the shared `MarkupModel`: one projection, not two with a drift test. |
| 9 | `rerun-if-changed=src` is acceptable if cached and fast. |
| 10 | A `links` key per metadata-exporting crate is acceptable. |
| 11 | `register_types()` is generated for applications. |
| 12 | Hot reload is gated behind a `hot-reload` feature. |
| 13 | `x:FieldModifier`: mirror upstream's accepted values; `protected` maps to `pub(crate)` with a warning. |
| 14 | Marker comments and `.map.json` are enough for the first release; no cargo wrapper. |

Status of the open technical issues: O1 (strong capture of the root by handler delegates in the interpreter) is fixed in the interpreter as part of stage 2 (weak capture, leak test); the fallible name registration the note asks for exists (`INameScope::try_register`). Emitter implementation starts after stage 2 (runtime loader) is delivered and green.

### 9.13 Asset scheme and file extension (owner ruling, 2026-10-05)

The asset URI scheme of the port is `ferres://` everywhere (asset loader, includes, group transformers, error messages); the upstream scheme is not accepted. Markup files of the port have the extension `.xaml` only: the upstream-derived extension is not used or recognised anywhere (loader, build integration, asset and URI handling, tests, documentation). Files brought over from upstream are renamed. Earlier sections of this document were updated accordingly.
