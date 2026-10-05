//! Metadata of types for markup and other untyped code: the counterpart of
//! the attribute classes of the managed original's `Metadata` namespace and
//! of the reflection data they annotate.

mod i_add_child;
mod markup_assembly;
mod markup_macros;
mod markup_type;
mod property_accessors;
mod service_provider;
pub mod typed_path;

pub use i_add_child::IAddChild;
pub use markup_assembly::{MarkupAssembly, XmlnsDefinition, XmlnsPrefix, FERRO_XML_NAMESPACE};
pub use markup_type::{
    attributes, from_markup_value, into_markup_value, markup_result, MarkupArguments, MarkupAttribute, MarkupAttributeValue,
    MarkupConstructor, MarkupDelegate, MarkupEmit, MarkupEnumMember, MarkupEvent, MarkupField, MarkupGeneric, MarkupIndexer, MarkupInvoke,
    MarkupInvokeError, MarkupLiteral, MarkupMethod, MarkupParameter, MarkupProperty, MarkupType, MarkupTypeKind, MarkupTyped,
    MarkupValue, TypeOf,
};
#[cfg(feature = "compiler-metadata")]
pub use property_accessors::{declared_property_accessors, property_accessors, record_property_accessor, PropertyAccessor};
pub use property_accessors::{register_type_rust_paths, rust_path_of_type};
pub use typed_path::TypedPathElement;
pub use service_provider::{service, EmptyServiceProvider, IServiceProvider};

#[cfg(test)]
mod markup_type_tests;
