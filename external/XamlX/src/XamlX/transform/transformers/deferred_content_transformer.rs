//! Port of `Transform/Transformers/DeferredContentTransformer.cs`.

use std::rc::Rc;

use crate::ast::{
    IXamlAstNode, XamlAstNodeExtensions, XamlDeferredContentInitializeIntermediateRootNode,
    XamlDeferredContentNode, XamlObjectInitializationNode, XamlPropertyAssignmentNode,
};
use crate::exceptions::{XamlError, XamlResult};
use crate::transform::{AstTransformationContext, IXamlAstTransformer};
use crate::type_system::{IXamlType, XamlValue};

pub struct DeferredContentTransformer;

impl IXamlAstTransformer for DeferredContentTransformer {
    fn transform(
        &self,
        context: &AstTransformationContext,
        node: Rc<dyn IXamlAstNode>,
    ) -> XamlResult<Rc<dyn IXamlAstNode>> {
        let Some(pa) = node.cast::<XamlPropertyAssignmentNode>() else {
            return Ok(node);
        };
        let type_mappings = &context.configuration().type_mappings;
        let deferred_attrs = &type_mappings.deferred_content_property_attributes;
        if deferred_attrs.is_empty() {
            return Ok(node);
        }
        let deferred_attr = pa
            .property
            .custom_attributes()
            .into_iter()
            .find(|ca| deferred_attrs.iter().any(|da| da.equals(&*ca.type_())));
        let Some(deferred_attr) = deferred_attr else {
            return Ok(node);
        };

        if pa.values.borrow().len() != 1 {
            return Err(XamlError::transform_exception(
                "Property with deferred content can have only one value",
                Some(&*node),
            ));
        }
        let content_node = pa.values.borrow()[0].clone();

        let manipulation = content_node
            .as_value_with_manipulation_node()
            .filter(|manipulation| {
                manipulation
                    .manipulation()
                    .is_some_and(|m| m.is::<XamlObjectInitializationNode>())
            });
        let Some(manipulation) = manipulation else {
            return Err(XamlError::transform_exception(
                "Unable to find the object initialization node inside deferred content, \
                 this shouldn't happen in default Xaml configuration, probably some AST transformer have broken the structure",
                Some(&*node),
            ));
        };
        let inner_value = manipulation.value();
        *manipulation.base.value.borrow_mut() =
            XamlDeferredContentInitializeIntermediateRootNode::new(inner_value);

        // Find the type param for the customizer.
        // A host framework typically stores it in a property of its template content attribute;
        // it is used to return somewhat strongly typed results from templates.
        let mut type_param: Option<Rc<dyn IXamlType>> = type_mappings
            .deferred_content_executor_customization_default_type_parameter
            .clone();
        let customization_type_param_property_names = &type_mappings
            .deferred_content_executor_customization_type_parameter_deferred_content_attribute_property_names;
        if !customization_type_param_property_names.is_empty() {
            let properties = deferred_attr.properties();
            for property_name in customization_type_param_property_names {
                if let Some(value) = properties.get(property_name) {
                    type_param = match value {
                        XamlValue::Type(t) => Some(t.clone()),
                        XamlValue::Null => None,
                        other => {
                            return Err(XamlError::invalid_cast(format!(
                                "Unable to cast object of type '{}' to type 'XamlX.TypeSystem.IXamlType'.",
                                other.type_name()
                            )))
                        }
                    };
                    break;
                }
            }
        }

        let deferred =
            XamlDeferredContentNode::new(content_node, type_param, context.configuration())?;
        pa.values.borrow_mut()[0] = deferred;
        Ok(node)
    }
}
