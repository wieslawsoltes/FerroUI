# StaticViewLocator port

The port of StaticViewLocator (<https://github.com/wieslawsoltes/StaticViewLocator>, MIT, Copyright (c) Wiesław Šoltés) to FerroUI: the source generator, its attributes and options, its tests and its samples. The reference is the checkout at `/Volumes/1TB-macOS/staticviewlocator-ref`, branch `main`, commit `fc362d57018c90d2aea18b642e3a071f493f9a9d` (2026-09-01), read only.

StaticViewLocator is a C# incremental source generator for Avalonia. A class annotated with `[StaticViewLocator]` gets, at compile time, a table that maps each view-model type to a factory of its view: by naming convention (`MyApp.ViewModels.SettingsViewModel` to `MyApp.Views.SettingsView`), by generic MVVM contracts that views implement, by ReactiveUI's `IViewFor<TViewModel>`, and by explicit `[StaticViewMapping]` attributes. Nothing is looked up by name at run time and no assembly is scanned, so the locator is trimming and AOT friendly. Optionally the generator also writes the complete `IDataTemplate` and ReactiveUI `IViewLocator` adapters.

## 1. What the reference contains

| Project | Files | Lines | Role |
|---|---|---|---|
| `StaticViewLocator` | 2 + props | 2,846 | The generator (`StaticViewLocatorGenerator.cs`, `StaticViewLocatorGenerator.Adapters.cs`), the attribute source it injects, eight diagnostics, `buildTransitive/StaticViewLocator.props` (the MSBuild properties exported as `CompilerVisibleProperty`) |
| `StaticViewLocator.Tests` | 13 + helper | 4,083 | 59 tests: snapshots of generated source, diagnostics, and runtime tests that compile the generated source and run it on the headless platform |
| `StaticViewLocatorDemo` | 22 | ~600 | A tabbed application: dashboard, test, settings and reports pages shown through the generated locator |
| `StaticViewLocatorReactiveUIDemo` (+ `.Reactive`) | 13 | ~500 | The same navigation shown side by side by a `ContentControl` with the generated `IDataTemplate` and by a ReactiveUI `ViewModelViewHost` with the generated `IViewLocator`; a context-wrapper fallback |

## 2. Crates and places

| Reference | Port | Path |
|---|---|---|
| The generator (`StaticViewLocatorGenerator`) | `ferroui-static-view-locator-build`, run by the build script of the crate that declares locators | `src/StaticViewLocator/FerroUI.StaticViewLocator.Build` |
| The injected attributes (`StaticViewLocatorAttribute`, `StaticViewMappingAttribute`, `ViewResolutionMode`) | `#[static_view_locator]`, `#[static_view_mapping]` (`ferroui-static-view-locator-macros`), `ViewResolutionMode` | `src/StaticViewLocator/FerroUI.StaticViewLocator.Macros`, `src/StaticViewLocator/FerroUI.StaticViewLocator` |
| The `Dictionary` fields and the framework types the generated source names | `ferroui-static-view-locator`: `ViewTable`, `MissingViewTable`, `NameTable`, `RuntimeType`, the helpers of the adapters, the feature `reactiveui` | `src/StaticViewLocator/FerroUI.StaticViewLocator` |
| `buildTransitive/StaticViewLocator.props` (MSBuild properties) | the setters of `StaticViewLocatorBuild` in `build.rs` | the build crate |
| `StaticViewLocator.Tests` (generation, diagnostics) | `tests/` module of the build crate | `src/StaticViewLocator/FerroUI.StaticViewLocator.Build/tests` |
| `StaticViewLocator.Tests` (runtime) | a crate whose build script generates locators for its fixture modules, one per runtime test | `tests/FerroUI.StaticViewLocator.Tests` |
| `StaticViewLocatorDemo`, `StaticViewLocatorReactiveUIDemo` | `static-view-locator-demo` | `samples/StaticViewLocatorDemo` |

No crate, module, type or item names the upstream framework; the default view base types are `ferroui_controls::UserControl` and `ferroui_controls::Window`.

## 3. Design

### 3.1 Where the generator runs

A C# source generator sees the whole compilation: every type of the project and of its references, with base types, interfaces and constructors. Rust has two places to generate code:

- A procedural macro sees only the item it is attached to. It cannot find the view models and views of the crate, so it cannot apply naming conventions or discover contracts.
- A build script sees the sources of the crate. FerroUI already reads them there: the scanner of `ferroui-build-scan` (the markup compiler's type model) reads every file from the crate root along its `mod` items. It resolves paths, `use` re-exports and glob imports, and reads the classes of the object model (`ferro_class!`) with their base classes and constructors. The `.xamlmeta` models of the dependencies (`DEP_<LINKS>_XAMLMETA`) give the base classes of other crates, such as `UserControl`, `Window`, `ReactiveUserControlBase` and `ReactiveWindowBase`.

The port therefore uses both:

1. **`ferroui-static-view-locator-build`** is the generator. The build script of the crate runs it (`StaticViewLocatorBuild::from_env().run()`):
   - it runs the scanner over the crate with the models of its dependencies;
   - it reads the items of the scanned files with `syn`: structs, enumerations and traits (the view-model candidates), `impl` blocks (the traits a type implements, the contracts a view implements, the members a locator declares), the visibility of modules, and the structs with `#[static_view_locator]`;
   - it writes `$OUT_DIR/static_view_locator/<Locator>.rs` for each locator.
2. **`#[static_view_locator(..)]`** checks its options, writes the struct, and includes that file. The file holds an `impl` block of the struct with the tables and the members the options ask for, followed by the trait `impl` blocks of the adapters. This is the generated `partial class` of the reference. `#[static_view_mapping(VM, View)]` is read by the generator; the macro only checks it.
3. **`ferroui-static-view-locator`** holds what the generated source names, and re-exports the attributes.

What the generator reads are the items of the sources. A class declared through a macro of the crate is known (the scanner expands the declaration macros of the crate), but a trait `impl` or a struct written by a macro is not: the `impl` blocks of contracts and of `IControlViewFor` are written as items.

The build script prints `cargo::rerun-if-changed` for every file it read, so the locators are generated again when a source file, the manifest or the model of a dependency changes. This is the counterpart of the incremental pipeline.

### 3.2 Types, keys and the run-time type of a view model

A C# `Type` is the key of the reference's dictionaries. In Rust, the key is the `TypeId` of the view-model type:

| Reference | Port |
|---|---|
| `Dictionary<Type, Func<Control>> s_views` with a collection initializer | `fn s_views() -> &'static ViewTable`: a static of constant entries (`TypeEntry::new(TypeId::of::<VM>, \|\| upcast(View::new()))`), indexed into a `HashMap` the first time it is read. `TypeId::of` is not a constant function, so an entry holds the function |
| `Dictionary<Type, string> s_missingViews` | `fn s_missing_views() -> &'static MissingViewTable` |
| `data.GetType()` | `RuntimeType::of_value(&BoxedValue)`. This is the type of the view model itself, of the object behind a registered reference handle (`Rc<T>`), of the class of an object of the class model (keyed as `Ref<T>`), or, with ReactiveUI, of the view model held by an `UntypedViewModel` |
| A class of view models with a base class | A class of the object model (`ferro_class!(Derived: Base)`), or a markup type with `base:`. A plain struct has no base class |
| An interface of view models | A trait. Its own entry is keyed by `dyn Trait` when the trait is dyn-compatible |
| `Func<Control>` | `fn() -> Ref<Control>` (`ViewFactory`) |
| `new View()` | the constructor without parameters: an inherent `fn new() -> Ref<Self>`, or the `new:` of `ferro_class_info!` |

### 3.3 Mapping sources and their order

Mapping sources are applied as in the reference, from the highest priority to the lowest:

1. `#[static_view_mapping(VM, View)]`: the final override. The types are named as in the module of the locator, where the generated file is included, so the generated source names them as written.
2. `view_model_mapping_contracts = [Contract, ..]`: the views of the crate that implement a contract trait, directly or through a base class. The contract has one view-model type: its single generic argument (`impl IViewFor<DashboardModel> for DashboardScreen`) or its single associated type (`impl FrameworkView for DashboardScreen { type ViewModel = DashboardModel; }`). The second form is how a Rust view states what the reference's generic base class (`FrameworkView<TViewModel>`) states.
3. With `generate_view_locator = true`: the views that implement `ferroui_reactiveui_controls::IControlViewFor` (`type ViewModel = ..`). This is the FerroUI form of a control that is ReactiveUI's `IViewFor<TViewModel>`.
4. The conventions: the module path of the view model with the namespace rules applied, and its name with the type name rules applied (and the interface prefix stripped for a trait).

A view must be a class of the object model that derives from `UserControl`, `Window` or an additional view base type. It must be visible from the module of the locator, not generic, and have an accessible constructor without parameters (SVL0004 otherwise).

## 4. Mapping of the features

### 4.1 Attribute options

| Reference (`[StaticViewLocator(..)]`) | Port (`#[static_view_locator(..)]`) | Default |
|---|---|---|
| `GenerateViewFactoryMethods` | `generate_view_factory_methods`: `fn try_create_view_exact(TypeId) -> Option<Ref<Control>>` | `false` |
| `GenerateRuntimeTypeFallbackMethods` | `generate_runtime_type_fallback_methods`: the legacy helpers `try_get_factory`, `try_get_factory_from_interfaces`, `try_get_missing_view`, `try_get_missing_view_from_interfaces` (and `..._for_type`) | `true` |
| `GenerateIViewLocator` | `generate_view_locator`: `impl IViewLocator` (`resolve_view_typed`, `resolve_view`, `resolve_view_unsafe`), `impl IEnableLogger`, and the inherent `resolve_view_for::<VM>(contract)` and `resolve_view_default::<VM>()`; needs the feature `reactiveui` | `false` |
| `GenerateIDataTemplate` | `generate_data_template`: `impl ITemplateWithParam` (`build`), `impl IDataTemplate` (`match_`) and the hooks | `false` |
| `GeneratedAdapterResolutionMode` | `generated_adapter_resolution_mode = ExactThenBaseTypesThenInterfaces` (the five modes) | `ExactThenBaseTypesThenInterfaces` |
| `ViewModelMappingContracts` | `view_model_mapping_contracts = [Trait, ..]` | empty |
| `DataTemplateMatchTypes` | `data_template_match_types = [Type, ..]` (classes or traits) | empty |
| none | `id = ".."`: the name of the generated file, for two locators of one name in a crate | the name of the struct |
| `[StaticViewMapping(typeof(VM), typeof(V))]`, repeatable | `#[static_view_mapping(VM, V)]`, repeatable | |

### 4.2 Build options (MSBuild properties)

| Reference | Port (`StaticViewLocatorBuild`) | Default |
|---|---|---|
| `StaticViewLocatorViewModelNamespacePrefixes` | `view_model_namespace_prefixes(["crate::view_models"])`: module paths | all modules |
| `StaticViewLocatorIncludeInternalViewModels` | `include_internal_view_models(bool)`: kept, without effect (see 6) | `false` |
| `StaticViewLocatorIncludeReferencedAssemblies` | `include_referenced_crates(bool)`: the view models of the models of the dependencies | `false` |
| `StaticViewLocatorAdditionalViewBaseTypes` | `additional_view_base_types(["crate::controls::ToolWindowBase"])` | none |
| `StaticViewLocatorNamespaceReplacementRules` | `namespace_replacement_rules([("view_models", "views")])` | `view_models=views`, `_view_model=_view` |
| `StaticViewLocatorTypeNameReplacementRules` | `type_name_replacement_rules([("ViewModel", "View")])` | `ViewModel=View` |
| `StaticViewLocatorStripGenericArityFromViewName` | `strip_generic_arity_from_view_name(bool)` | `true` |
| `StaticViewLocatorInterfacePrefixesToStrip` | `interface_prefixes_to_strip(["I"])` | `I` |
| The text of a property, commas between values | `property(name, text)` (both the snake-case names and the property names of the reference) | |

As in the reference, configured rules are applied first and the default rules after them, and a replacement target may be empty (`ViewModel=`).

### 4.3 Generated members

| Reference | Port |
|---|---|
| `s_views`, `s_missingViews` | `s_views()`, `s_missing_views()` |
| `TryCreateViewExact(Type, out Control?)` | `try_create_view_exact(TypeId) -> Option<Ref<Control>>`; a declared one is reused |
| `TryGetResolvedViewFactory`, `TryCreateResolvedView` | `try_get_resolved_view_factory(Option<&BoxedValue>) -> Option<ViewFactory>`, `try_create_resolved_view` |
| Legacy `Build(object?)` | `pub fn build(&self, Option<&BoxedValue>) -> Option<Ref<Control>>`; panics like the reference's exception for a type with neither a view nor a missing-view entry |
| `TryGetFactory`, `TryGetFactoryFromInterfaces`, `TryGetMissingView`, `TryGetMissingViewFromInterfaces`, `TryGetFactoryForType`, `TryGetMissingViewForType` | The same names in snake case. They walk tables that the generator flattens (see 5) |
| `IDataTemplate.Build`/`Match` | `ITemplateWithParam::build`, `IDataTemplate::match_` |
| `BuildInvalidView`, `BuildResolvedView`, `BuildFallbackView`, `BuildMissingView`, `MatchDataTemplate` | `build_invalid_view`, `build_resolved_view`, `build_fallback_view`, `build_missing_view(.., RuntimeType)`, `match_data_template`: private methods of the locator. A method of the name declared by the locator replaces the default one. A hook may return any type that converts into `Option<Ref<Control>>` |
| `IViewLocator.ResolveView` (four overloads) | `resolve_view_typed`, `resolve_view`, `resolve_view_unsafe` of FerroUI's `IViewLocator`, and `resolve_view_for::<VM>(contract)` / `resolve_view_default::<VM>()` |
| `IViewFor.ViewModel = instance` | `IViewFor::set_view_model` on a `ControlViewFor` of the control, for the views that implement `IControlViewFor` |

### 4.4 Diagnostics

| ID | Reference | Port |
|---|---|---|
| SVL0001 | An adapter contract type is not referenced | `generate_view_locator` without the feature `reactiveui` (`DEP_FERROUI_STATIC_VIEW_LOCATOR_REACTIVEUI`); `generate_data_template` always has its contract |
| SVL0002 | A source member collides with a generated member but cannot implement it | A hook, `try_create_view_exact` or `resolve_view_*` with another receiver or another number of parameters, and an inherent `build` or `match_` that would shadow the data template |
| SVL0003 | Several inferred views map to one view model | The same, for contracts and `IControlViewFor` |
| SVL0004 | A mapped view is inaccessible, abstract or has no constructor without arguments | Not visible from the module of the locator, generic, or without an accessible constructor without parameters |
| SVL0005 | The locator is nested, static, file-local or not partial | The attribute is on an item that is not a struct |
| SVL0006 | A contract is not an open generic with one type parameter | A contract that is a struct or an enumeration, has generic arguments, or is a trait of the crate without exactly one generic parameter or one associated type |
| SVL0007 | Mapped interfaces of a view model resolve to different views | The same, for traits |
| SVL0008 | An open generic adapter mapping has no compiled contract | Every open generic view model mapped in a generated adapter (see 6) |
| SVL0009 | none | Two locators of a crate generate one file (state distinct `id`s) |
| SVL0010 | none | An option of the attribute or of the build is not understood |

An error fails the build (`cargo::error=`), as an error of a source generator fails the compilation.

## 5. Resolution

The reference's generated adapters resolve a view model by an exact dictionary lookup. For the types the generator can enumerate, inherited and interface results are flattened into exact entries; for the other types they fall back to generated `data is MappedBase` and `data is IMappedContract` branches, in the order of the resolution mode.

Rust cannot test a value for a trait at run time, and a plain struct has no base class. So the port flattens the precedence into the table of the views for every type of the crate: every struct, enumeration and class that has no view of its own and has a mapped base class or a mapped trait gets the view the mode selects. A view model of the crate thus resolves as the reference resolves it, through one `TypeId` lookup. `match_` creates no view and allocates nothing (the runtime tests measure it).

The legacy fallback of the reference walks `BaseType` and `GetInterfaces()` at run time. The port flattens the same walks at build time into four tables (`s_base_views`, `s_interface_views`, `s_base_missing_views`, `s_interface_missing_views`) and looks them up by `TypeId`. The walk by generic type definition looks up `NameTable`s by the path of the definition: `std::any::type_name` of the value without its generic arguments, the Rust counterpart of `GetGenericTypeDefinition()` (only on the legacy path, as in the reference).

## 6. Deviations

Recorded in `docs/porting/DEVIATIONS.md`, "StaticViewLocator":

1. **Generator in the build script, attribute as a proc macro.** See 3.1. A locator must be a struct at the top level of a module the build script reads (not inside a function, not under `#[cfg(test)]`). Same-named locators in one crate state an `id` (SVL0009), where the reference qualifies the hint name with the namespace.
2. **Paths and conventions.** A namespace is a module path. The defaults add `_view_model=_view` to `view_models=views`, for the one-module-per-type layout of Rust (`settings_view_model.rs` beside `settings_view.rs`). Rust names carry no generic arity, so `strip_generic_arity_from_view_name(false)` requires a view with the generic parameters of the view model, which cannot be constructed.
3. **No run-time type tests in the generated adapters.** See 5. Types of other crates resolve exactly. An open generic view model mapped in a generated adapter is reported (SVL0008) unless its mapped traits are ambiguous first (SVL0007): a closed instance has a `TypeId` of its own, and the reference's inferred base or interface contract cannot be tested at run time. Map an instantiation explicitly (`#[static_view_mapping(WidgetViewModel<i32>, WidgetView)]`), through a contract of its view, or use the legacy locator.
4. **Traits as keys.** A trait view model is keyed by `dyn Trait` only when the trait is dyn-compatible; its mapping is flattened onto its implementors in any case.
5. **No subclasses of a locator.** Hooks are private methods. The reference's `protected virtual` and sealed `private` defaults are one form in Rust: a method of the name declared by the locator replaces the default.
6. **No overloads.** A member with the name of a generated member is that member. Members that cannot stand for it are reported (SVL0002), where the reference ignores overloads with other parameter kinds.
7. **Internal view models.** Rust has no `InternalsVisibleTo`: a view model of another crate is taken only when it is public, and `include_internal_view_models` has no effect.
8. **Referenced view models** are the types of the `.xamlmeta` models of the dependencies: the types those crates declare with the declaration macros of the framework.
9. **ReactiveUI views.** A view of ReactiveUI is a control class that implements `IControlViewFor`, shown through a `ControlViewFor` adapter. A mapped view that does not implement it gives no view of ReactiveUI, as `control as IViewFor` gives `null`. The generated map has no contracts: a resolution under a contract other than the default one (`None` or empty) finds nothing.
10. **One view model distribution of ReactiveUI.** The reference supports two mutually exclusive ReactiveUI package families. FerroUI has one ReactiveUI port, so there is nothing to choose.

## 7. Tests

The 59 tests of the reference are ported under their names in snake case:

- **The tests that check generated source and diagnostics** are in the build crate (`tests/`). Each writes its source text as the root file of a crate in a directory of its own and runs the generator. A test whose premise Rust does not have keeps its name and checks the Rust counterpart; its comment says how. This covers `RefKind`, `Dynamic`, `Sealed`, base-class contracts and run-time type tests.
- **The runtime tests** (`CreatesRegisteredViewInstances`, `ResolvesMultipleViewModelsAndRespectsOrdering`, `ResolvesGenericViewModelsUsingGenericTypeDefinition`, `ResolvesUsingBaseClassBeforeInterfaceFallback`, `CustomBuildCanCallGeneratedHelpers`, `ResolvesInterfaceMappingsByStrippingConfiguredPrefix`, `CombinedAdaptersBuildConventionOnlyViews`, `GeneratedReactiveUIAndDataTemplateAdaptersWorkAtRuntime`, and the run-time half of `GeneratedResolverCoversAllPathsWithoutAllocatingOrConstructingDuringMatch`) are modules of `tests/FerroUI.StaticViewLocator.Tests`. Its build script generates their locators, so the generated source of each adapter shape is compiled and run in the unit test application. Every locator of that crate sees every view model of the crate, as every locator of a C# compilation does. A fixture therefore names its types so that the conventions of the other fixtures do not map them.

## 8. Sample

`samples/StaticViewLocatorDemo` ports both demos into one application. The four pages of `StaticViewLocatorDemo` (dashboard with metric cards, test, settings, reports) are shown in a tab control whose content template is the generated data template of a convention-only locator. The ReactiveUI page of `StaticViewLocatorReactiveUIDemo` shows the same navigation through a content control and through a `ViewModelViewHost` whose view locator is the generated `IViewLocator`, with the context-host fallback of the reference.
