//! Port of `Transform/Transformers/ApplyWhitespaceNormalization.cs`.

use std::rc::Rc;

use crate::ast::{
    IXamlAstNode, IXamlAstValueNode, XamlAstClrProperty, XamlAstNodeExtensions,
    XamlAstPropertyReferenceExtensions, XamlAstXamlPropertyValueNode,
};
use crate::exceptions::XamlResult;
use crate::transform::{
    AstTransformationContext, IXamlAstTransformer, TransformerConfiguration,
    WhitespaceNormalization,
};

// See: https://docs.microsoft.com/en-us/dotnet/desktop/xaml-services/white-space-processing
// Must be applied after content has been transformed to a XamlAstXamlPropertyValueNode,
// and after ResolvePropertyValueAddersTransformer has resolved the Add methods for collection properties
pub struct ApplyWhitespaceNormalization;

impl IXamlAstTransformer for ApplyWhitespaceNormalization {
    fn transform(
        &self,
        context: &AstTransformationContext,
        node: Rc<dyn IXamlAstNode>,
    ) -> XamlResult<Rc<dyn IXamlAstNode>> {
        if let Some(property_node) = node.cast::<XamlAstXamlPropertyValueNode>() {
            let property = property_node.property().get_clr_property()?;
            let mut child_nodes = property_node.values.borrow_mut();

            WhitespaceNormalization::apply(&mut child_nodes, context.configuration())?;
            if !wants_whitespace_only_elements(context.configuration(), &property, &child_nodes) {
                WhitespaceNormalization::remove_whitespace_nodes(&mut child_nodes);
            }
        }

        Ok(node)
    }
}

fn wants_whitespace_only_elements(
    config: &TransformerConfiguration,
    property: &XamlAstClrProperty,
    child_nodes: &[Rc<dyn IXamlAstValueNode>],
) -> bool {
    let well_known_types = config.well_known_types();

    // A collection-like property will only receive whitespace-only nodes if the
    // property type can be deduced, and that type is annotated as whitespace significant
    if let Some(getter) = property.getter() {
        if config.is_whitespace_significant_collection(&*getter.return_type()) {
            return true;
        }
    }

    for setter in property.setters() {
        // Skip any dictionary-like setters
        let parameters = setter.parameters();
        if parameters.len() != 1 {
            continue;
        }

        let parameter_type = &parameters[0];
        if !setter.binder_parameters().allow_multiple.get() {
            if child_nodes.len() > 1 {
                return false;
            }

            // If the property can accept a scalar string, it'll get whitespace nodes by default
            if parameter_type.equals(&*well_known_types.string)
                || parameter_type.equals(&*well_known_types.object)
            {
                return true;
            }
        }
    }

    false
}
