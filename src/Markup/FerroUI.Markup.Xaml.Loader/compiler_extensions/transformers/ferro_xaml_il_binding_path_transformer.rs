//! Port of `CompilerExtensions/Transformers/FerroXamlIlBindingPathTransformer.cs`.

use std::rc::Rc;

use xamlx::ast::{
    IXamlAstNode, IXamlAstValueNode, XamlAstConstructableObjectNode, XamlAstExtensions,
    XamlAstNodeExtensions, XamlAstTextNode, XamlManipulationGroupNode, XamlMarkupExtensionNode,
    XamlPropertyAssignmentNode, XamlStaticExtensionNode, XamlTypeExtensionNode,
};
use xamlx::exceptions::{XamlError, XamlResult};
use xamlx::transform::transformers::TypeReferenceResolver;
use xamlx::transform::{AstTransformationContext, IXamlAstTransformer};
use xamlx::type_system::IXamlType;

use super::{
    FerroXamlIlDataContextTypeMetadataNode, FerroXamlIlTargetTypeMetadataNode,
    FerroXamlIlWellKnownTypesExtensions, XamlBindingsTransformException,
};
use crate::compiler_extensions::XamlIlBindingPathHelper;

/// Resolves the parsed path of every compiled binding extension against its start type (the
/// type of its `Source`, its `DataType` or the data context type in scope) and replaces it with
/// the resolved path node.
pub struct FerroXamlIlBindingPathTransformer;

/// `Enumerable.First()` on an empty sequence.
fn sequence_contains_no_elements() -> XamlError {
    XamlError::invalid_operation("Sequence contains no elements")
}

impl FerroXamlIlBindingPathTransformer {
    /// The local function `matchProperty`.
    fn match_property(
        node: &Rc<dyn IXamlAstNode>,
        styled_element_type: &Rc<dyn IXamlType>,
        property_name: &str,
    ) -> bool {
        let matches = |p: &Rc<XamlPropertyAssignmentNode>| {
            p.property.declaring_type().equals(&**styled_element_type)
                && p.property.name() == property_name
        };
        if let Some(p) = node.cast::<XamlPropertyAssignmentNode>() {
            return matches(&p);
        }
        if let Some(m) = node.cast::<XamlManipulationGroupNode>() {
            let first = m.children.borrow().first().cloned();
            if let Some(pm) = first.and_then(|c| c.cast::<XamlPropertyAssignmentNode>()) {
                return matches(&pm);
            }
        }
        false
    }

    /// The local function `getResourceValue_xKey`.
    fn get_resource_value_x_key(node: &XamlPropertyAssignmentNode) -> String {
        let values = node.values.borrow();
        if values.len() == 2 {
            if let Some(t) = values[0].cast::<XamlAstTextNode>() {
                return t.text();
            }
        }
        String::new()
    }

    /// The local function `getResourceValue_Type`.
    fn get_resource_value_type(
        node: &XamlPropertyAssignmentNode,
        xaml_type: Option<Rc<dyn IXamlType>>,
    ) -> XamlResult<Option<Rc<dyn IXamlType>>> {
        let values = node.values.borrow().clone();
        if values.len() == 2 {
            Ok(Some(values[1].type_().get_clr_type()?))
        } else {
            Ok(xaml_type)
        }
    }

    /// The local iterator `getResourceValues`, searched for the first resource whose key is
    /// `key` (`getResourceValues(node).FirstOrDefault(r => getResourceValue_xKey(r) == key)`).
    fn find_resource_value(
        node: &Rc<dyn IXamlAstNode>,
        key: &str,
    ) -> XamlResult<Option<Rc<XamlPropertyAssignmentNode>>> {
        if let Some(property_node) = node.cast::<XamlPropertyAssignmentNode>() {
            let values = property_node.values.borrow().clone();
            let dictionary = if values.len() == 1 {
                values[0].cast::<XamlAstConstructableObjectNode>()
            } else {
                None
            };
            let dictionary = match dictionary {
                Some(obj) if obj.type_().get_clr_type()?.is("FerroUI.Controls", "ResourceDictionary") => {
                    Some(obj)
                }
                _ => None,
            };
            if let Some(obj) = dictionary {
                let children = obj.children.borrow().clone();
                for c in &children {
                    if let Some(r) = Self::find_resource_value(c, key)? {
                        return Ok(Some(r));
                    }
                }
            } else if Self::get_resource_value_x_key(&property_node) == key {
                return Ok(Some(property_node));
            }
        } else if let Some(m) = node.cast::<XamlManipulationGroupNode>() {
            let children = m.children.borrow().clone();
            for r in children
                .iter()
                .filter_map(|c| c.cast::<XamlPropertyAssignmentNode>())
            {
                if Self::get_resource_value_x_key(&r) == key {
                    return Ok(Some(r));
                }
            }
        }
        Ok(None)
    }
}

impl IXamlAstTransformer for FerroXamlIlBindingPathTransformer {
    fn transform(
        &self,
        context: &AstTransformationContext,
        node: Rc<dyn IXamlAstNode>,
    ) -> XamlResult<Rc<dyn IXamlAstNode>> {
        let Some(binding) = node.cast::<XamlAstConstructableObjectNode>() else {
            return Ok(node);
        };
        let types = context.try_get_ferro_types()?;
        if !binding
            .type_()
            .get_clr_type()?
            .equals(&*types.compiled_binding_extension)
        {
            return Ok(node);
        }

        let mut start_type: Option<Rc<dyn IXamlType>> = None;
        let assignments: Vec<Rc<XamlPropertyAssignmentNode>> = binding
            .children
            .borrow()
            .iter()
            .filter_map(|c| c.cast::<XamlPropertyAssignmentNode>())
            .collect();
        let source_property = assignments.iter().find(|c| c.property.name() == "Source");
        let data_type_property = assignments.iter().find(|c| c.property.name() == "DataType");

        let source_values = source_property.map(|p| p.values.borrow().clone());
        if let Some([source_value]) = source_values.as_deref() {
            if let Some(text_node) = source_value.cast::<XamlAstTextNode>() {
                start_type = Some(text_node.type_().get_clr_type()?);
            } else if let Some(extension) = source_value.cast::<XamlMarkupExtensionNode>() {
                start_type = Some(extension.type_().get_clr_type()?);

                //let's try to infer StaticResource type from parent resources in xaml
                let extension_value = extension.value();
                if extension_value
                    .type_()
                    .get_clr_type()?
                    .is("FerroUI.Markup.Xaml.MarkupExtensions", "StaticResourceExtension")
                {
                    let key_node = extension_value
                        .cast::<XamlAstConstructableObjectNode>()
                        .and_then(|cn| {
                            let arguments = cn.arguments.borrow();
                            if arguments.len() == 1 {
                                arguments[0].cast::<XamlAstTextNode>()
                            } else {
                                None
                            }
                        });
                    if let Some(key_node) = key_node {
                        let key = key_node.text();

                        let styled_element = &types.styled_element;
                        let mut resource = None;
                        for o in context
                            .parent_nodes()
                            .into_iter()
                            .filter_map(|n| n.cast::<XamlAstConstructableObjectNode>())
                        {
                            if !styled_element.is_assignable_from(&*o.type_().get_clr_type()?) {
                                continue;
                            }
                            let resources = o
                                .children
                                .borrow()
                                .iter()
                                .find(|p| Self::match_property(p, styled_element, "Resources"))
                                .cloned();
                            let Some(resources) = resources else {
                                continue;
                            };
                            if let Some(found) = Self::find_resource_value(&resources, &key)? {
                                resource = Some(found);
                                break;
                            }
                        }

                        if let Some(resource) = resource {
                            start_type = Self::get_resource_value_type(&resource, start_type)?;
                        }
                    }
                }
            } else if let Some(static_extension) = source_value.cast::<XamlStaticExtensionNode>()
            {
                start_type = Some(static_extension.type_().get_clr_type()?);
            }
        }

        let data_type_values = data_type_property.map(|p| p.values.borrow().clone());
        if let Some([data_type_value]) = data_type_values.as_deref() {
            if let Some(text) = data_type_value.cast::<XamlAstTextNode>() {
                start_type = Some(
                    TypeReferenceResolver::resolve_type_by_xml_name(
                        context,
                        &text.text(),
                        false,
                        &*text,
                        true,
                    )?
                    .type_
                    .clone(),
                );
            }

            if let Some(type_node) = data_type_value.cast::<XamlTypeExtensionNode>() {
                start_type = Some(type_node.value().get_clr_type()?);
            }
        }

        let start_type_resolver = || -> XamlResult<Rc<dyn IXamlType>> {
            if let Some(start_type) = &start_type {
                return Ok(start_type.clone());
            }

            let parent_data_context_node = context
                .parent_nodes()
                .into_iter()
                .find_map(|n| n.cast::<FerroXamlIlDataContextTypeMetadataNode>());
            match parent_data_context_node {
                None => Err(XamlBindingsTransformException::new(
                    "Cannot parse a compiled binding without an explicit x:DataType directive to give a starting data type for bindings.",
                    &*binding,
                    None,
                )),
                Some(parent_data_context_node) => {
                    Ok(parent_data_context_node.data_context_type.clone())
                }
            }
        };

        let parent_nodes = context.parent_nodes();
        let mut constructable_parents = parent_nodes
            .iter()
            .filter_map(|n| n.cast::<XamlAstConstructableObjectNode>());
        let mut self_type = constructable_parents
            .next()
            .ok_or_else(sequence_contains_no_elements)?
            .type_()
            .get_clr_type()?;

        if types.multi_binding.is_assignable_from(&*self_type) {
            self_type = constructable_parents
                .next()
                .ok_or_else(sequence_contains_no_elements)?
                .type_()
                .get_clr_type()?;
        }

        // When using self bindings with setters we need to change target type to resolved selector type.
        if types.setter_base.is_assignable_from(&*self_type) {
            self_type = parent_nodes
                .iter()
                .find_map(|n| n.cast::<FerroXamlIlTargetTypeMetadataNode>())
                .ok_or_else(sequence_contains_no_elements)?
                .target_type()
                .get_clr_type()?;
        }

        XamlIlBindingPathHelper::update_compiled_binding_extension(
            context,
            &binding,
            &start_type_resolver,
            &self_type,
        )?;

        Ok(node)
    }
}
