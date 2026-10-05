//! Port of `Transform/XamlLanguageTypeMappings.cs`.

use std::rc::Rc;

use crate::exceptions::{XamlError, XamlResult};
use crate::type_system::{
    get_type, IXamlCustomAttribute, IXamlMethod, IXamlProperty, IXamlType, IXamlTypeSystem,
};

#[derive(Default)]
pub struct XamlLanguageTypeMappings {
    pub xmlns_attributes: Vec<Rc<dyn IXamlType>>,
    pub usable_during_initialization_attributes: Vec<Rc<dyn IXamlType>>,
    pub content_attributes: Vec<Rc<dyn IXamlType>>,
    pub whitespace_significant_collection_attributes: Vec<Rc<dyn IXamlType>>,
    pub trim_surrounding_whitespace_attributes: Vec<Rc<dyn IXamlType>>,
    pub type_converter_attributes: Vec<Rc<dyn IXamlType>>,
    /// Required; upstream declares it non-nullable and initializes it lazily.
    pub service_provider: Option<Rc<dyn IXamlType>>,
    pub type_descriptor_context: Option<Rc<dyn IXamlType>>,
    pub support_initialize: Option<Rc<dyn IXamlType>>,
    pub provide_value_target: Option<Rc<dyn IXamlType>>,
    pub root_object_provider: Option<Rc<dyn IXamlType>>,
    pub parent_stack_provider: Option<Rc<dyn IXamlType>>,
    pub xml_namespace_info_provider: Option<Rc<dyn IXamlType>>,
    pub uri_context_provider: Option<Rc<dyn IXamlType>>,
    pub i_add_child: Option<Rc<dyn IXamlType>>,
    pub i_add_child_of_t: Option<Rc<dyn IXamlType>>,
    pub custom_attribute_resolver: Option<Rc<dyn IXamlCustomAttributeResolver>>,

    /// Expected signature:
    /// `static IServiceProvider InnerServiceProviderFactory(IServiceProvider self);`
    pub inner_service_provider_factory_method: Option<Rc<dyn IXamlMethod>>,
    /// Expected signature:
    /// `static *any-non-void* DeferredTransformationFactory(Func<IServiceProvider, object> builder, IServiceProvider provider);`
    /// or
    /// `static *any-non-void* DeferredTransformationFactory(delegate*<IServiceProvider, object>, IServiceProvider provider);`
    pub deferred_content_executor_customization: Option<Rc<dyn IXamlMethod>>,
    pub deferred_content_property_attributes: Vec<Rc<dyn IXamlType>>,
    pub deferred_content_executor_customization_default_type_parameter: Option<Rc<dyn IXamlType>>,
    pub deferred_content_executor_customization_type_parameter_deferred_content_attribute_property_names:
        Vec<String>,
    pub root_object_provider_intermediate_root_property_name: Option<String>,
}

impl XamlLanguageTypeMappings {
    /// `XamlLanguageTypeMappings(IXamlTypeSystem)`: uses the default mappings.
    pub fn new<S: IXamlTypeSystem + ?Sized>(type_system: &S) -> XamlResult<Self> {
        Self::with_defaults(type_system, true)
    }

    /// `XamlLanguageTypeMappings(IXamlTypeSystem, bool useDefault)`.
    pub fn with_defaults<S: IXamlTypeSystem + ?Sized>(
        type_system: &S,
        use_default: bool,
    ) -> XamlResult<Self> {
        let mut rv = Self::default();
        if use_default {
            rv.service_provider = Some(get_type(type_system, "System.IServiceProvider")?);
            rv.type_descriptor_context = Some(get_type(
                type_system,
                "System.ComponentModel.ITypeDescriptorContext",
            )?);
            rv.support_initialize = Some(get_type(
                type_system,
                "System.ComponentModel.ISupportInitialize",
            )?);
            rv.type_converter_attributes.push(get_type(
                type_system,
                "System.ComponentModel.TypeConverterAttribute",
            )?);
        }
        Ok(rv)
    }

    /// The (required) `ServiceProvider` type.
    pub fn service_provider(&self) -> XamlResult<Rc<dyn IXamlType>> {
        self.service_provider.clone().ok_or_else(|| {
            XamlError::internal(
                "NullReferenceException",
                "XamlLanguageTypeMappings.ServiceProvider is not set",
            )
        })
    }
}

pub trait IXamlCustomAttributeResolver {
    /// `GetCustomAttribute(IXamlType type, IXamlType attributeType)`.
    fn get_custom_attribute_for_type(
        &self,
        type_: &dyn IXamlType,
        attribute_type: &dyn IXamlType,
    ) -> Option<Rc<dyn IXamlCustomAttribute>>;
    /// `GetCustomAttribute(IXamlProperty property, IXamlType attributeType)`.
    fn get_custom_attribute_for_property(
        &self,
        property: &dyn IXamlProperty,
        attribute_type: &dyn IXamlType,
    ) -> Option<Rc<dyn IXamlCustomAttribute>>;
}
