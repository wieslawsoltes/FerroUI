//! Port of
//! `CompilerExtensions/Transformers/FerroXamlIlResolveByNameMarkupExtensionReplacer.cs`.

use std::rc::Rc;

use xamlx::ast::{
    IXamlAstNode, IXamlAstValueNode, XamlAstClrProperty, XamlAstClrTypeReference,
    XamlAstNodeExtensions, XamlAstObjectNode, XamlAstPropertyReferenceExtensions,
    XamlAstTextNode, XamlAstXamlPropertyValueNode,
};
use xamlx::exceptions::XamlResult;
use xamlx::transform::{AstTransformationContext, IXamlAstTransformer, XamlTransformHelpers};

use super::FerroXamlIlWellKnownTypesExtensions;

/// Replaces the text value of a property marked with `[ResolveByName]` with a
/// `ResolveByNameExtension` markup extension taking the text as its argument, so that
/// `<Label Target="input"/>` resolves the named element.
pub struct FerroXamlIlResolveByNameMarkupExtensionReplacer;

impl IXamlAstTransformer for FerroXamlIlResolveByNameMarkupExtensionReplacer {
    fn transform(
        &self,
        context: &AstTransformationContext,
        node: Rc<dyn IXamlAstNode>,
    ) -> XamlResult<Rc<dyn IXamlAstNode>> {
        let Some(property_value_node) = node.cast::<XamlAstXamlPropertyValueNode>() else {
            return Ok(node);
        };

        let Some(reference_node) = property_value_node.property().cast::<XamlAstClrProperty>()
        else {
            return Ok(node);
        };

        let mut attributes = property_value_node
            .property()
            .get_clr_property()?
            .custom_attributes();

        if let Some(getter) = reference_node.getter() {
            attributes.extend(getter.custom_attributes());
        }

        if attributes
            .iter()
            .all(|attribute| !attribute.type_().is("FerroUI.Controls", "ResolveByNameAttribute"))
        {
            return Ok(node);
        }

        let first_value = {
            let values = property_value_node.values.borrow();
            if values.len() != 1 || !values[0].is::<XamlAstTextNode>() {
                return Ok(node);
            }
            values[0].clone()
        };

        let new_node = XamlAstObjectNode::new(
            &*first_value,
            XamlAstClrTypeReference::new(
                &*first_value,
                context.try_get_ferro_types()?.resolve_by_name_extension.clone(),
                true,
            ),
        );
        *new_node.arguments.borrow_mut() = vec![first_value];

        let new_node: Rc<dyn IXamlAstValueNode> = new_node;
        if let Some(extension_node) =
            XamlTransformHelpers::try_convert_markup_extension(context, &new_node)?
        {
            property_value_node.values.borrow_mut()[0] = extension_node;
        }

        Ok(node)
    }
}
