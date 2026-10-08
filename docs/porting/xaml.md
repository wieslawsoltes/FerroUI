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

`Click="OnClick"` reaches the back end as `XamlDirectCallAddHandler` whose value is a `XamlLoadMethodDelegateNode` (created by `XamlTransformHelpers` when the root type has a method with that name and the signature of the delegate's `Invoke`). For this to happen on the build path the type system must list handler methods of the `x:Class` type:

1. Methods declared in metadata (`markup: { methods: [fn OnClick(Option<BoxedValue>, RoutedEventArgs) => ..] }`) are listed as declared. This form is what the interpreter needs anyway.
2. In addition the scanner lists the inherent methods of the class found in `impl <Class> { .. }` blocks of the file that contains its `ferro_class!`: for each `fn name(&self, a, b)` with two parameters it projects a method under both its own name and the PascalCase form of its name. Lookup therefore finds `OnClick` first by exact name (a method literally named `OnClick`) and then by `on_click`, which is the "exact name first, then snake_case" rule of section 3.8. Parameter types of such projected methods are *unknown* (9.5.4) and match any delegate signature; rustc checks the real signature at the emitted call.
3. An odd name is mapped explicitly: `#[doc(alias = "OnURLClicked")]` on the method is read by the scanner (no proc macro involved) *(proposal; alternatively only form 1)*.

Emitted call: the handler method is called by path on the upgraded weak root (`root.on_click(sender, e)`). Handler signatures accepted on the build path: `(&self, &Interactive, &TArgs)` for routed events; for plain events the parameter list of the event's `add` function. The diagnostic for a missing handler is the upstream one (`Unable to find suitable setter or adder for property Click ...`), raised by the transformer because the type system did not list the method.

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

### 9.5 Typing without the framework linked into the build tool

#### 9.5.1 Inputs of the build-time type system

| Input | Read from | Gives |
|---|---|---|
| `ferro_class!(X: Base [, virtuals ..])` | source scan | class, base class, Rust path (crate + module of the file + `X`) |
| `ferro_static_type!(X)` | source scan | static owner types of attached properties |
| `ferro_properties! { impl X [, also [..]] [, fn name] { .. } }` and `ferro_property!` | source scan | registered properties: accessor name (`child_property`), kind and exact value type from the return type (`StyledProperty<T>`, `AttachedProperty<T>`, `DirectProperty<O, T>`), XAML name and owner/host from the `FerroProperty::register*::<..>("Name", ..)` expression, `add_owner::<Y>()`, `.assign_binding(true)` / `.inherits(true)` builder calls |
| `ferro_class_info!(X { new, interfaces, markup: {..} })` | source scan | default constructor, interface handles, content property, constructors, plain properties, methods, fields (incl. routed events), events, attributes, `property_attributes`, `namespace` |
| `ferro_markup_type!(struct/class/interface/static ..)`, `ferro_markup_enum!` | source scan | everything else; `handles:`, `this:`, `parse:`, enum members and values |
| `register_types.rs` | source scan | module → dotted namespace table (`NAMESPACES`), the `MarkupAssembly` (assembly name, `XmlnsDefinition`s, metadata such as `CREATE_SOURCE_INFO`) |
| `impl X { fn .. }` blocks of `x:Class` types | source scan | handler candidates (9.4.4) |
| `<crate>.xamlmeta` of every dependency | Cargo `links` metadata (9.6.3) | the same model, serialised, for crates whose sources the tool does not read |
| closed `System.*` table | loader (`runtime/type_system/core_types.rs`, to be shared) | BCL types |

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

- The model is `rust_emitter::XamlMetadata` (`AssemblyModel` with `name`, `crate_name`, `documents[DocumentModel]` and `dependencies`, JSON). `GeneratedFile::metadata` and `ClassFile::metadata` write it next to the generated file (`compiled_xaml.xamlmeta`, drift-tested with it); `XamlMetadata::read` reads a file and, transitively, the files its `dependencies` name (relative to it). Until `export_metadata()` exists the generator of a dependent crate names the checked-in file of each dependency, where build integration will read `DEP_<CRATE>_XAML_XAMLMETA`.
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
| `XamlTypeExtensionNode` | `RuntimeTypeValue`, converted at the call site (decision 16) | `T::TYPE` or `ValueType::of::<T>()` by receiving member; boxed per decision 16 for `object` | generic type arguments on the build path |
| `XamlStaticExtensionNode` | static property getter or field value | enum variant path, `Owner::name_property().as_property()`, `Owner::name_event()`, declared static getter (B/C) | literal fields of core types |
| `XamlConstantNode` | `constant_value` | literal with suffix (`20.0_f64`, `5_i32`, `true`, `'c'`); enum from numeric value → variant path, flag combination → `A \| B` or `rt::enum_from_value::<T>(n)?` | numeric value that is no member: build error (decision 12) |
| `XamlRootObjectNode` | `root_object_field()` | the `target` parameter / `rt::cast::<Ref<T>>(&ctx.root_object())?` inside deferred builders | — |
| `XamlIntermediateRootObjectNode` | `intermediate_root_object()` | the local holding it, or `ctx.intermediate_root_object()` cast | — |
| `XamlLoadMethodDelegateNode` | `MarkupDelegate` over the method | closure calling the method on the (weakly captured) instance | O1 |
| `XamlAstCompilerLocalNode` | read local | the `let` name | — |
| `XamlAstLocalInitializationNodeEmitter` | evaluate, store, return | `let name = ..;` | — |
| `XamlValueNodeWithBeginInit` | evaluate, BeginInit | `let x = ..; x.begin_init();` | — |
| `XamlAstManipulationImperativeNode` | execute imperative, ignore target | the statement | — |
| `XamlAstImperativeValueManipulation` | evaluate value, manipulate it | `let v = ..;` + manipulation on `v` | — |
| `XamlAstContextLocalNode` | context as service provider / type descriptor context | `sp.clone()` / `rt::type_descriptor_context(&ctx)` | `ITypeDescriptorContext` lives in `ferroui_markup_xaml::converters` |
| `XamlAstRuntimeCastNode` | evaluate as object, checked cast | `rt::cast::<T>(&v)?` or `v.cast::<T>()` | — |
| `XamlAstNeedsParentStackValueNode` | verify stack, evaluate | the inner expression (the verification is a build-time assertion) | — |
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
| `ClassValueSetter` | `target.Classes.Set(name, value)` | `t.classes().set("name", v);` | — |
| `ClassBindingSetter` | `BindClass(target, name, binding, null)` | form C call of the `BindClass` method declared in `markup_types/well_known.rs` (no public typed function was found) | a `pub` typed entry point would allow form B |
| `FerroAttachedInstancePropertySetterMethod` | `set_value_untyped(Field, value, LocalValue)` | typed `t.set_value(Owner::p_property(), v)` when the value type is exact, else untyped | setter arity quirk (decision 15) |
| `FerroAttachedInstancePropertyGetterMethod` | `GetValue(Field)` | `t.get_value(Owner::p_property())` | — |
| `XamlDirectCallAddHandler` | `AddHandler(event, handler, Direct \| Bubble, false)` | `t.add_handler_with(&Owner::name_event(), closure, RoutingStrategies::DIRECT.union(RoutingStrategies::BUBBLE), false);` | args type narrower than the event's: `add_handler_as` |
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

Remaining for E5, in order:

1. `compile_xaml()` / `embed_assets()` of 9.6 in place of the checked-in files, with `register()` (R8) and the generated `register_types` of 9.7.4. The `.xamlmeta` of each crate then travels through Cargo `links` metadata (9.6.3) instead of the checked-in paths the generators name, and the choice of the constructor moves into the compiler.
2. The ControlCatalog: its documents and the `x:Class` documents of the dialogs compiled, so that neither links the run-time loader (browser-platform.md, section 20, item 3). Measure a few pages first against the estimate there (+6 to +10 MB raw, +0.5 to +1.2 MB gzip on the module); above it, the owner decides.

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
