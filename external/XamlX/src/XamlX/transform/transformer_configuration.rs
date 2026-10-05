//! Port of `Transform/TransformerConfiguration.cs`.

use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::ast::IXamlAstValueNode;
use crate::exceptions::{XamlError, XamlResult};
use crate::type_system::{
    IXamlAssembly, IXamlCustomAttribute, IXamlProperty, IXamlType, IXamlTypeSystem, XamlTypeId,
    XamlTypeWellKnownTypes,
};

use super::{
    AstTransformationContext, GuidIdentifierGenerator, IXamlIdentifierGenerator,
    XamlDiagnosticsHandler, XamlLanguageTypeMappings, XamlXmlnsMappings,
};

/// `TransformerConfiguration.XamlValueConverter`: returns `Ok(Some(node))` when the host
/// converted `node` to `type_` (the C# `out` result), `Ok(None)` otherwise.
pub type XamlValueConverter = Rc<
    dyn Fn(
        &AstTransformationContext,
        &Rc<dyn IXamlAstValueNode>,
        Option<&[Rc<dyn IXamlCustomAttribute>]>,
        &Rc<dyn IXamlType>,
    ) -> XamlResult<Option<Rc<dyn IXamlAstValueNode>>>,
>;

pub struct TransformerConfiguration {
    extras: RefCell<HashMap<TypeId, Rc<dyn Any>>>,
    pub type_system: Rc<dyn IXamlTypeSystem>,
    pub default_assembly: Option<Rc<dyn IXamlAssembly>>,
    pub type_mappings: XamlLanguageTypeMappings,
    pub xmlns_mappings: XamlXmlnsMappings,
    pub custom_value_converter: Option<XamlValueConverter>,
    pub diagnostics_handler: XamlDiagnosticsHandler,
    pub identifier_generator: Rc<dyn IXamlIdentifierGenerator>,
    /// `(namespace, name)` pairs of the directives handled by `KnownDirectivesTransformer`.
    pub known_directives: RefCell<Vec<(String, String)>>,
    content_property_cache: RefCell<HashMap<XamlTypeId, Option<Rc<dyn IXamlProperty>>>>,
    whitespace_significant_collection_cache: RefCell<HashMap<XamlTypeId, bool>>,
    trim_surrounding_whitespace_cache: RefCell<HashMap<XamlTypeId, bool>>,
}

impl TransformerConfiguration {
    pub fn new(
        type_system: Rc<dyn IXamlTypeSystem>,
        default_assembly: Option<Rc<dyn IXamlAssembly>>,
        type_mappings: XamlLanguageTypeMappings,
        xmlns_mappings: Option<XamlXmlnsMappings>,
        custom_value_converter: Option<XamlValueConverter>,
        identifier_generator: Option<Rc<dyn IXamlIdentifierGenerator>>,
        diagnostics_handler: Option<XamlDiagnosticsHandler>,
    ) -> XamlResult<Self> {
        let xmlns_mappings = match xmlns_mappings {
            Some(m) => m,
            None => XamlXmlnsMappings::resolve(&*type_system, &type_mappings)?,
        };
        Ok(Self {
            extras: RefCell::new(HashMap::new()),
            type_system,
            default_assembly,
            type_mappings,
            xmlns_mappings,
            custom_value_converter,
            diagnostics_handler: diagnostics_handler.unwrap_or_default(),
            identifier_generator: identifier_generator
                .unwrap_or_else(|| Rc::new(GuidIdentifierGenerator)),
            known_directives: RefCell::new(Vec::new()),
            content_property_cache: RefCell::new(HashMap::new()),
            whitespace_significant_collection_cache: RefCell::new(HashMap::new()),
            trim_surrounding_whitespace_cache: RefCell::new(HashMap::new()),
        })
    }

    /// Gets extension configuration section
    pub fn get_extra<T: 'static>(&self) -> XamlResult<Rc<T>> {
        let extra = self.extras.borrow().get(&TypeId::of::<T>()).cloned();
        extra.and_then(|e| e.downcast::<T>().ok()).ok_or_else(|| {
            XamlError::internal(
                "KeyNotFoundException",
                format!(
                    "The given key '{}' was not present in the dictionary.",
                    std::any::type_name::<T>()
                ),
            )
        })
    }

    /// Gets or create extension configuration section
    pub fn get_or_create_extra<T: Default + 'static>(&self) -> Rc<T> {
        if let Ok(found) = self.get_extra::<T>() {
            return found;
        }
        let created = Rc::new(T::default());
        self.extras
            .borrow_mut()
            .insert(TypeId::of::<T>(), created.clone());
        created
    }

    /// Adds extension configuration section
    pub fn add_extra<T: 'static>(&self, extra: Rc<T>) {
        self.extras.borrow_mut().insert(TypeId::of::<T>(), extra);
    }

    pub fn well_known_types(&self) -> Rc<XamlTypeWellKnownTypes> {
        self.type_system.well_known_types()
    }

    pub fn find_content_property(
        &self,
        type_: &Rc<dyn IXamlType>,
    ) -> XamlResult<Option<Rc<dyn IXamlProperty>>> {
        if self.type_mappings.content_attributes.is_empty() {
            return Ok(None);
        }
        let id = type_.id();
        if let Some(found) = self.content_property_cache.borrow().get(&id) {
            return Ok(found.clone());
        }

        let mut found: Option<Rc<dyn IXamlProperty>> = None;
        for content_attribute_on_type in
            self.get_custom_attributes_for_type(&**type_, &self.type_mappings.content_attributes)
        {
            let properties = content_attribute_on_type.properties();
            if properties.is_empty() {
                return Err(XamlError::type_system_exception(format!(
                    "The '{}' attribute must have a property name specified",
                    content_attribute_on_type.type_().to_type_string()
                )));
            }

            let name = properties.get("Name").ok_or_else(|| {
                XamlError::internal(
                    "KeyNotFoundException",
                    "The given key 'Name' was not present in the dictionary.",
                )
            })?;
            if let Some(property_name) = name.as_str() {
                let content_property = type_
                    .get_all_properties()
                    .into_iter()
                    .find(|prop| prop.name() == property_name);
                let Some(content_property) = content_property else {
                    return Err(XamlError::type_system_exception(format!(
                        "The property name '{property_name}' specified in the content property of {} does not exist.",
                        type_.get_fqn()
                    )));
                };
                if let Some(f) = &found {
                    if !content_property.equals(&**f) {
                        return Err(XamlError::type_system_exception(format!(
                            "Content attribute is declared on multiple properties of {}",
                            type_.get_fqn()
                        )));
                    }
                }
                found = Some(content_property);
            }
        }

        for p in type_.properties() {
            if !self
                .get_custom_attributes_for_property(&*p, &self.type_mappings.content_attributes)
                .is_empty()
            {
                if let Some(f) = &found {
                    if !p.equals(&**f) {
                        return Err(XamlError::type_system_exception(format!(
                            "Content attribute is declared on multiple properties of {}",
                            type_.get_fqn()
                        )));
                    }
                }
                found = Some(p);
            }
        }

        // Fall back to a Content attribute found on a base type
        if found.is_none() {
            if let Some(base_type) = type_.base_type() {
                found = self.find_content_property(&base_type)?;
            }
        }

        self.content_property_cache
            .borrow_mut()
            .insert(id, found.clone());
        Ok(found)
    }

    /// Checks whether the given type is annotated as a collection that treats whitespace as significant.
    pub fn is_whitespace_significant_collection(&self, type_: &dyn IXamlType) -> bool {
        self.is_attribute_present_in_type_hierarchy(
            type_,
            &self
                .type_mappings
                .whitespace_significant_collection_attributes,
            &self.whitespace_significant_collection_cache,
        )
    }

    /// Checks whether the given type is annotated to indicate that surrounding whitespace should be
    /// trimmed even if it is contained in a whitespace-significant collection. Note that this
    /// behavior is disabled in an xml:space="preserve" scope.
    pub fn is_trim_surrounding_whitespace_element(&self, type_: &dyn IXamlType) -> bool {
        self.is_attribute_present_in_type_hierarchy(
            type_,
            &self.type_mappings.trim_surrounding_whitespace_attributes,
            &self.trim_surrounding_whitespace_cache,
        )
    }

    fn is_attribute_present_in_type_hierarchy(
        &self,
        type_: &dyn IXamlType,
        attributes: &[Rc<dyn IXamlType>],
        cache: &RefCell<HashMap<XamlTypeId, bool>>,
    ) -> bool {
        if attributes.is_empty() {
            return false;
        }

        let id = type_.id();
        if let Some(result) = cache.borrow().get(&id) {
            return *result;
        }

        let mut result = false;
        // Check the base type first
        if let Some(base_type) = type_.base_type() {
            result = self.is_attribute_present_in_type_hierarchy(&*base_type, attributes, cache);
        }

        // Check if the current type has any of the configured attributes
        if !result {
            result = !self
                .get_custom_attributes_for_type(type_, attributes)
                .is_empty();
        }

        cache.borrow_mut().insert(id, result);
        result
    }

    /// `GetCustomAttribute(IXamlType type, IXamlType attributeType)`.
    pub fn get_custom_attribute_for_type(
        &self,
        type_: &dyn IXamlType,
        attribute_type: &dyn IXamlType,
    ) -> Vec<Rc<dyn IXamlCustomAttribute>> {
        let mut rv = Vec::new();
        if let Some(resolver) = &self.type_mappings.custom_attribute_resolver {
            if let Some(custom) = resolver.get_custom_attribute_for_type(type_, attribute_type) {
                rv.push(custom);
            }
        }
        rv.extend(
            type_
                .custom_attributes()
                .into_iter()
                .filter(|attr| attr.type_().equals(attribute_type)),
        );
        rv
    }

    /// `GetCustomAttribute(IXamlType type, IEnumerable<IXamlType> types)`.
    pub fn get_custom_attributes_for_type(
        &self,
        type_: &dyn IXamlType,
        types: &[Rc<dyn IXamlType>],
    ) -> Vec<Rc<dyn IXamlCustomAttribute>> {
        types
            .iter()
            .flat_map(|t| self.get_custom_attribute_for_type(type_, &**t))
            .collect()
    }

    /// `GetCustomAttribute(IXamlProperty prop, IXamlType attributeType)`.
    pub fn get_custom_attribute_for_property(
        &self,
        prop: &dyn IXamlProperty,
        attribute_type: &dyn IXamlType,
    ) -> Vec<Rc<dyn IXamlCustomAttribute>> {
        let mut rv = Vec::new();
        if let Some(resolver) = &self.type_mappings.custom_attribute_resolver {
            if let Some(custom) = resolver.get_custom_attribute_for_property(prop, attribute_type) {
                rv.push(custom);
            }
        }
        rv.extend(
            prop.custom_attributes()
                .into_iter()
                .filter(|attr| attr.type_().equals(attribute_type)),
        );
        rv
    }

    /// `GetCustomAttribute(IXamlProperty prop, IEnumerable<IXamlType> types)`.
    pub fn get_custom_attributes_for_property(
        &self,
        prop: &dyn IXamlProperty,
        types: &[Rc<dyn IXamlType>],
    ) -> Vec<Rc<dyn IXamlCustomAttribute>> {
        types
            .iter()
            .flat_map(|t| self.get_custom_attribute_for_property(prop, &**t))
            .collect()
    }
}
