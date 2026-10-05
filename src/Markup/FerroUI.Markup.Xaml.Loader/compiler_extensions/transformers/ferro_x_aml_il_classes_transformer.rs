//! Port of `CompilerExtensions/Transformers/FerroXAmlIlClassesTransformer.cs`.

use std::rc::Rc;

use xamlx::ast::{
    IXamlAstNode, IXamlAstValueNode, XamlAstClrProperty, XamlAstNodeExtensions, XamlAstTextNode,
    XamlAstXamlPropertyValueNode,
};
use xamlx::exceptions::XamlResult;
use xamlx::transform::{AstTransformationContext, IXamlAstTransformer};

use super::FerroXamlIlWellKnownTypesExtensions;

/// Converts an attribute syntax property value assignment to a collection syntax property
/// assignment.
///
/// Converts the property assignment `Classes="foo bar"` to:
///
/// ```text
///     <StyledElement.Classes>
///         <x:String>foo</x:String>
///         <x:String>bar</x:String>
///     </StyledElement.Classes>
/// ```
pub struct FerroXamlIlClassesTransformer;

impl IXamlAstTransformer for FerroXamlIlClassesTransformer {
    fn transform(
        &self,
        context: &AstTransformationContext,
        node: Rc<dyn IXamlAstNode>,
    ) -> XamlResult<Rc<dyn IXamlAstNode>> {
        let types = context.try_get_ferro_types()?;
        let Some(property_value) = node.cast::<XamlAstXamlPropertyValueNode>() else {
            return Ok(node);
        };
        if !property_value.is_attribute_syntax {
            return Ok(node);
        }
        let Some(property) = property_value.property().cast::<XamlAstClrProperty>() else {
            return Ok(node);
        };
        if !property
            .getter()
            .is_some_and(|getter| getter.return_type().equals(&*types.classes))
        {
            return Ok(node);
        }
        let value = {
            let values = property_value.values.borrow();
            if values.len() != 1 {
                return Ok(node);
            }
            values[0].cast::<XamlAstTextNode>()
        };
        let Some(value) = value else {
            return Ok(node);
        };

        let text = value.text();
        let string_type = context.configuration().well_known_types().string.clone();
        // `string.Split(' ')`: empty entries are kept.
        let classes: Vec<Rc<dyn IXamlAstValueNode>> = text
            .split(' ')
            .map(|x| {
                let text_node: Rc<dyn IXamlAstValueNode> =
                    XamlAstTextNode::with_type(&*node, x, false, Some(string_type.clone()));
                text_node
            })
            .collect();
        Ok(XamlAstXamlPropertyValueNode::with_values(
            &*node, property, classes, false,
        ))
    }
}
