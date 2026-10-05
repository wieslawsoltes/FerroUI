//! The run-time type system: the equivalent of the reflection based type
//! system of the managed original, over the process-wide registries of
//! classes, markup metadata and assemblies.
//!
//! # What metadata is projected to
//!
//! | Metadata | Projection |
//! |---|---|
//! | a registered property | the property `Name` (`get_Name` / `set_Name`) and the static field `NameProperty`; an attached property on the class that registered it: the static accessors `GetName` / `SetName` (`AttachedProperty<T>` field), on a class that added itself as an owner: an instance property (`StyledProperty<T>` field) |
//! | a declared static `GetName` / `SetName` of a registered attached property | replaces the plain accessor and carries the attributes of the property (`property_attributes`, `assign_binding`) |
//! | `properties:` | instance properties |
//! | `indexers:` | the property `Item` with index parameters (`get_Item` / `set_Item`) and `[DefaultMember("Item")]` on the type |
//! | `fields:` | static fields (static properties are declared with `static_properties:`) |
//! | `static_properties:` | static properties (`get_Name` / `set_Name` static accessors), no field |
//! | `constructors:` | public constructors; a parameter has the attributes its declaration states (`(name: T [Attribute(..)]) => ..`). FALLBACK: a constructor in the positional form gives a parameter the attributes of the property that names itself its `[ConstructorArgument]` (same type, in order) |
//! | an attribute argument `[a, b]` | an array-valued argument (`XamlValue::Array`) |
//! | a class without a constructor | one non-public parameterless constructor (the class cannot be created by markup, a root instance of it can be populated) |
//! | `base: FerroList<T>` | the base type, when the values can be cast to the list; the instantiations of the list implement `IList<T>`, `IReadOnlyList<T>`, `IList` and `INotifyCollectionChanged` |
//!
//! Runtime library collections (`List<T>`, `IList<T>`, `IReadOnlyList<T>`,
//! `Dictionary<K, V>`, `IDictionary<K, V>`) have `Count` and the indexer as
//! abstract members: a call is dispatched to the member of the same name the
//! run-time type of the instance declares. `System.String` and arrays have
//! `Length`.

mod core_types;
mod list_converter;
mod object_model;
mod runtime_type;
mod runtime_type_system;
mod values;

pub use runtime_type::{
    DeclaredMember, RuntimeAssembly, RuntimeConstructor, RuntimeCustomAttribute, RuntimeEvent, RuntimeField, RuntimeFieldValue,
    RuntimeInvoker, RuntimeMembers, RuntimeMethod, RuntimeProperty, RuntimeType, RuntimeTypeKind, RuntimeTypeOrigin,
};
pub use runtime_type_system::{
    attribute_type_name, RuntimeTypeSystem, CONTROLS_METADATA_NAMESPACE, METADATA_NAMESPACE, PROPERTY_NAMESPACE,
};
pub use list_converter::RuntimeListConverter;
pub use values::{
    box_object, normalize_object, register_bindable_array, to_untyped, DeferredContentFactory, ITypeDescriptorContext,
    RuntimeArray, RuntimeTypeValue,
};

/// Converts an untyped value to exactly the Rust type `target` with the
/// assignability casts of the untyped value conversions: what a member that
/// declares `target` receives.
pub fn to_exact_value(
    value: &ferroui_base::metadata::MarkupValue,
    target: ferroui_base::data::core::ValueType,
) -> Result<ferroui_base::BoxedValue, ferroui_base::metadata::MarkupInvokeError> {
    runtime_type::to_exact(value, target, 0)
}

/// Adds the TYPED element of a plain property to a compiled binding path under
/// construction: what the typed emission of a single-property path
/// (`builder.Property<TSource, TValue>(info, accessorFactory, acceptsNull)`) adds, from the
/// typed path hook of the property's metadata declaration
/// (`MarkupProperty::typed_path_element`). `None` if the property is not the projection of
/// a declared plain property with a typed form (the caller then adds the untyped
/// element); nothing is added to the path in that case.
pub fn typed_path_element(
    property: &dyn xamlx::type_system::IXamlProperty,
    builder: &ferroui_base::data::CompiledBindingPathBuilder,
    accepts_null: bool,
) -> Option<ferroui_base::data::CompiledBindingPathBuilder> {
    let hook = property.as_any().downcast_ref::<RuntimeProperty>()?.markup_property()?.typed_path_element?;
    hook(builder, accepts_null)
}

#[cfg(test)]
mod tests;
