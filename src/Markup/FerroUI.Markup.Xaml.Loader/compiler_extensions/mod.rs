//! Port of `Markup.Xaml.Loader/CompilerExtensions`: the framework's XAML language definition
//! on top of `xamlx`.
//!
//! # Nodes and setters the back ends must implement
//!
//! Upstream these types generate IL themselves (`IXamlAstILEmitableNode.Emit`,
//! `IXamlAstLocalsEmitableNode.Emit`, `IXamlILOptimizedEmitablePropertySetter.Emit` /
//! `EmitWithArguments`, callbacks of the emit mappings). Here they carry only data, all of it
//! public, and the two back ends (the run-time interpreter and the Rust source emitter)
//! implement them. The doc comment of each type states step by step what the upstream IL does.
//!
//! | Type | File | Kind | Semantics in one line |
//! |---|---|---|---|
//! | [`ast_nodes::FerroXamlIlVectorLikeConstantAstNode`] | `ast_nodes/ferro_xaml_il_vector_like_constant_ast_node.rs` | value node | `new T(double, ...)` from constant doubles |
//! | [`ast_nodes::FerroXamlIlGridLengthAstNode`] | `ast_nodes/ferro_xaml_il_grid_length_ast_node.rs` | value node | `new GridLength(value, unit)` from a constant |
//! | [`ast_nodes::FerroXamlIlFontFamilyAstNode`] | `ast_nodes/ferro_xaml_il_font_family_ast_node.rs` | value node | `new FontFamily(context.BaseUri, text)` |
//! | [`ast_nodes::FerroXamlIlArrayConstantAstNode`] | `ast_nodes/ferro_xaml_il_array_constant_ast_node.rs` | value node | new array filled with the value nodes |
//! | [`ast_nodes::FerroXamlIlFerroListConstantAstNode`] | `ast_nodes/ferro_xaml_il_ferro_list_constant_ast_node.rs` | value node | new list, `Capacity = n`, `Add` each value node |
//! | [`XamlIlFerroPropertyNode`] | `xaml_il_ferro_property_helper.rs` | value node | load the registered property field of a CLR property |
//! | [`XamlIlFerroPropertyFieldNode`] | `xaml_il_ferro_property_helper.rs` | value node | load a registered property static field |
//! | [`XamlIlFerroClassProperty`] | `xaml_il_ferro_property_helper.rs` | value node | `StyledElementExtensions.GetClassProperty(className)` |
//! | [`BindingSetter`] | `xaml_il_ferro_property_helper.rs` | property setter | `target.Bind(property, binding)` |
//! | [`BindingWithPrioritySetter`] | `xaml_il_ferro_property_helper.rs` | property setter | `target.Bind(property, binding)`, priority ignored |
//! | [`SetValueWithPrioritySetter`] | `xaml_il_ferro_property_helper.rs` | property setter | `target.SetValue<T>(property, value, priority)` |
//! | [`UnsetValueSetter`] | `xaml_il_ferro_property_helper.rs` | property setter | `target.SetValue(property, FerroProperty.UnsetValue, LocalValue)` |
//! | [`XamlIlFerroPropertyHelper::try_get_provide_value_target`] | `xaml_il_ferro_property_helper.rs` | emit mapping | what to load as `IProvideValueTarget.TargetProperty` |
//! | [`FerroXamlIlContextNameScopeField`] | `ferro_xaml_il_language.rs` | runtime context member | `FerroNameScope` field, filled from the parent service provider |
//! | [`FerroXamlIlContextEagerParentStackProvider`] | `ferro_xaml_il_language.rs` | runtime context member | eager parent stack provider implementation |
//! | [`transformers::XamlIlSelectorInitialNode`] | `transformers/ferro_xaml_il_selector_transformer.rs` | value node | push `null` (start of a selector chain) |
//! | [`transformers::XamlIlTypeSelector`] | `transformers/ferro_xaml_il_selector_transformer.rs` | value node | `Selectors.OfType(previous, type)` / `Selectors.Is(previous, type)` |
//! | [`transformers::XamlIlStringSelector`] | `transformers/ferro_xaml_il_selector_transformer.rs` | value node | `Selectors.Class(previous, name)` / `Selectors.Name(previous, name)` |
//! | [`transformers::XamlIlCombinatorSelector`] | `transformers/ferro_xaml_il_selector_transformer.rs` | value node | `Selectors.Child/Descendant/Template(previous)` |
//! | [`transformers::XamlIlNotSelector`] | `transformers/ferro_xaml_il_selector_transformer.rs` | value node | `Selectors.Not(previous, argument)` |
//! | [`transformers::XamlIlNthChildSelector`] | `transformers/ferro_xaml_il_selector_transformer.rs` | value node | `Selectors.NthChild/NthLastChild(previous, step, offset)` |
//! | [`transformers::XamlIlPropertyEqualsSelector`] | `transformers/ferro_xaml_il_selector_transformer.rs` | value node | `Selectors.PropertyEquals(previous, property field, (object) value)` |
//! | [`transformers::XamlIlAttachedPropertyEqualsSelector`] | `transformers/ferro_xaml_il_selector_transformer.rs` | value node | `Selectors.PropertyEquals(previous, attached property field, (object) value)` |
//! | [`transformers::XamlIlOrSelectorNode`] | `transformers/ferro_xaml_il_selector_transformer.rs` | value node | `Selectors.Or(new List<Selector> { ... })`, or the only alternative |
//! | [`transformers::XamlIlNestingSelector`] | `transformers/ferro_xaml_il_selector_transformer.rs` | value node | `Selectors.Nesting(previous)` |
//! | [`transformers::XamlIlQueryInitialNode`] | `transformers/ferro_xaml_il_query_transformer.rs` | value node | push `null` (start of a query chain) |
//! | [`transformers::XamlIlTypeQuery`] | `transformers/ferro_xaml_il_query_transformer.rs` | value node | `StyleQueries.OfType/Is(previous, type)` (never created upstream) |
//! | [`transformers::XamlIlStringQuery`] | `transformers/ferro_xaml_il_query_transformer.rs` | value node | `StyleQueries.Class/Name(previous, string)` (never created upstream) |
//! | [`transformers::XamlIlCombinatorQuery`] | `transformers/ferro_xaml_il_query_transformer.rs` | value node | `StyleQueries.Child/Descendant/Template(previous)` (never created upstream) |
//! | [`transformers::XamlIlWidthQuery`] | `transformers/ferro_xaml_il_query_transformer.rs` | value node | `StyleQueries.Width(previous, (int) operator, value)` |
//! | [`transformers::XamlIlHeightQuery`] | `transformers/ferro_xaml_il_query_transformer.rs` | value node | `StyleQueries.Height(previous, (int) operator, value)` |
//! | [`transformers::XamlIlOrQueryNode`] | `transformers/ferro_xaml_il_query_transformer.rs` | value node | `StyleQueries.Or(new List<StyleQuery> { ... })`, or the only member |
//! | [`transformers::XamlIlAndQueryNode`] | `transformers/ferro_xaml_il_query_transformer.rs` | value node | `StyleQueries.And(new List<StyleQuery> { ... })`, or the only member |
//! | [`transformers::XamlIlDirectCallPropertySetter`] | `transformers/ferro_xaml_il_setter_transformer.rs` | property setter | box the value if its type is a value type, then `setter.set_Value(object)` |
//! | [`transformers::FerroNameScopeRegistrationXamlIlNode`] | `transformers/add_name_scope_registration.rs` | manipulation node | `context.FerroNameScope.Register(name, target)` |
//! | [`transformers::ClassValueSetter`] | `transformers/ferro_xaml_il_classes_property_resolver.rs` | property setter | `target.Classes.Set(className, value)` |
//! | [`transformers::ClassBindingSetter`] | `transformers/ferro_xaml_il_classes_property_resolver.rs` | property setter | `StyledElementExtensions.BindClass(target, className, binding, null)` |
//! | [`transformers::FerroAttachedInstancePropertySetterMethod`] | `transformers/ferro_xaml_il_transform_instance_attached_properties.rs` | property setter | `target.SetValue(Field, (object) value, BindingPriority.LocalValue)` |
//! | [`transformers::FerroAttachedInstancePropertyGetterMethod`] | `transformers/ferro_xaml_il_transform_instance_attached_properties.rs` | pseudo method | `(T) target.GetValue(Field)` |
//! | [`transformers::XamlDirectCallAddHandler`] | `transformers/ferro_xaml_il_transform_routed_event.rs` | property setter | `target.AddHandler(EventField, handler, Direct or Bubble, false)` |
//! | [`transformers::InjectServiceProviderNode`] | `transformers/ferro_xaml_il_constructor_service_provider_transformer.rs` | value node | the runtime context as `IServiceProvider` |
//! | [`transformers::OptionsMarkupExtensionMethod`] | `transformers/ferro_xaml_il_option_markup_extension_transformer.rs` | pseudo method | first branch whose `ShouldProvideOption` holds, else the default |
//! | [`transformers::ResourceAdderSetter`] | `transformers/ferro_xaml_il_resource_transformer.rs` | property setter | `[target.Resources].Add*(key, value)` plus `XamlSourceInfo.SetXamlSourceInfo` |
//! | [`transformers::EnsureCapacityNode`] | `transformers/ferro_xaml_il_ensure_resource_dictionary_capacity_transformer.rs` | manipulation node | `if (x is ResourceDictionary d) d.EnsureCapacity(d.Count + n)` |
//! | [`transformers::HandleRootObjectScopeNode`] | `transformers/ferro_xaml_il_root_object_scope_transformer.rs` | manipulation node | `NameScope.SetNameScope(root, scope)` for styled elements, then `scope.Complete()` |
//! | [`transformers::XamlSourceInfoValueManipulation`] | `transformers/ferro_xaml_il_add_source_info_transformer.rs` | manipulation node | `XamlSourceInfo.SetXamlSourceInfo(target, new XamlSourceInfo(line, position, document))` |
//! | [`group_transformers::NewServiceProviderNode`] | `group_transformers/xaml_include_group_transformer.rs` | value node | `XamlIlRuntimeHelpers.CreateRootServiceProviderV3(context)` |
//! | [`XamlIlBindingPathNode`] | `xaml_il_binding_path_helper.rs` | value node | `new CompiledBindingPathBuilder()`, each element's builder call, `Build()` |
//! | [`XamlIlNotPathElementNode`] | `xaml_il_binding_path_helper.rs` | binding path element | `builder.Not()` |
//! | [`XamlIlStreamObservablePathElementNode`] | `xaml_il_binding_path_helper.rs` | binding path element | `builder.StreamObservable<T>()` |
//! | [`XamlIlStreamTaskPathElementNode`] | `xaml_il_binding_path_helper.rs` | binding path element | `builder.StreamTask<T>()` |
//! | [`SelfPathElementNode`] | `xaml_il_binding_path_helper.rs` | binding path element | `builder.Self()` |
//! | [`FindAncestorPathElementNode`] | `xaml_il_binding_path_helper.rs` | binding path element | `builder.Ancestor(typeof(T), level)` |
//! | [`FindVisualAncestorPathElementNode`] | `xaml_il_binding_path_helper.rs` | binding path element | `builder.VisualAncestor(typeof(T), level)` |
//! | [`ElementNamePathElementNode`] | `xaml_il_binding_path_helper.rs` | binding path element | `builder.ElementName(context name scope, name)` |
//! | [`TemplatedParentPathElementNode`] | `xaml_il_binding_path_helper.rs` | binding path element | `builder.TemplatedParent()` |
//! | [`XamlIlFerroPropertyPropertyPathElementNode`] | `xaml_il_binding_path_helper.rs` | binding path element | `builder.Property(registered property field, accessor factory[, true])` |
//! | [`XamlIlClrPropertyPathElementNode`] | `xaml_il_binding_path_helper.rs` | binding path element | `builder.Property(property info, INPC accessor factory[, true])`, or the typed overload |
//! | [`XamlIlClrMethodPathElementNode`] | `xaml_il_binding_path_helper.rs` | binding path element | `builder.Method(method handle, delegate type handle[, true])` |
//! | [`XamlIlClrMethodAsCommandPathElementNode`] | `xaml_il_binding_path_helper.rs` | binding path element | `builder.Command(name, execute, canExecute, dependsOnProperties)` |
//! | [`XamlIlClrIndexerPathElementNode`] | `xaml_il_binding_path_helper.rs` | binding path element | `builder.Property(indexer property info, indexer or INPC accessor factory)` |
//! | [`XamlIlArrayIndexerPathElementNode`] | `xaml_il_binding_path_helper.rs` | binding path element | `builder.ArrayElement(int[] indices, typeof(element))` |
//! | [`TypeCastPathElementNode`] | `xaml_il_binding_path_helper.rs` | binding path element | `builder.TypeCast<T>()` |
//! | [`XamlIlClrPropertyInfoEmitter`] | `xaml_il_clr_property_info_helper.rs` | emit helper | describes the cached `ClrPropertyInfo` (untyped or typed) of a CLR property |
//! | [`XamlIlPropertyInfoAccessorFactoryEmitter`] | `xaml_il_property_info_accessor_factory_emitter.rs` | emit helper | which `PropertyInfoAccessorFactory` method creates the accessor of a path element |
//! | [`XamlIlTrampolineBuilder`] | `xaml_il_trampoline_builder.rs` | emit helper | describes the execute / can-execute thunks of a method used as a command |
//!
//! Append rows when a ported transformer adds a node, setter or custom emit method of its own.

pub mod ast_nodes;
pub mod group_transformers;
pub mod transformers;
pub mod visitors;

mod ferro_xaml_diagnostic_codes;
mod ferro_xaml_il_compiler;
mod ferro_xaml_il_compiler_configuration;
mod ferro_xaml_il_language;
mod ferro_xaml_il_language_parse_intrinsics;
mod i_xaml_document_resource;
mod xaml_ast_new_clr_object_helper;
mod xaml_document_resource;
mod xaml_document_usage;
mod xaml_il_binding_path_helper;
mod xaml_il_clr_property_info_helper;
mod xaml_il_ferro_property_helper;
mod xaml_il_property_info_accessor_factory_emitter;
mod xaml_il_trampoline_builder;

pub use ferro_xaml_diagnostic_codes::*;
pub use ferro_xaml_il_compiler::*;
pub use ferro_xaml_il_compiler_configuration::*;
pub use ferro_xaml_il_language::*;
pub use ferro_xaml_il_language_parse_intrinsics::*;
pub use i_xaml_document_resource::*;
pub use xaml_ast_new_clr_object_helper::*;
pub use xaml_document_resource::*;
pub use xaml_document_usage::*;
pub use xaml_il_binding_path_helper::*;
pub use xaml_il_clr_property_info_helper::*;
pub use xaml_il_ferro_property_helper::*;
pub use xaml_il_property_info_accessor_factory_emitter::*;
pub use xaml_il_trampoline_builder::*;

#[cfg(test)]
mod ferro_xaml_il_compiler_tests;
