//! Port of `CompilerExtensions/Transformers/FerroXamlIlControlThemeTransformer.cs`.

use std::rc::Rc;

use xamlx::ast::{
    IXamlAstNode, IXamlAstValueNode, XamlAstClrTypeReference, XamlAstExtensions,
    XamlAstNodeExtensions, XamlAstObjectNode, XamlAstPropertyReferenceExtensions, XamlAstTextNode,
    XamlAstXamlPropertyValueNode, XamlTypeExtensionNode,
};
use xamlx::exceptions::{XamlError, XamlResult};
use xamlx::transform::transformers::TypeReferenceResolver;
use xamlx::transform::{AstTransformationContext, IXamlAstTransformer};
use xamlx::type_system::IXamlType;

use super::ferro_xaml_il_selector_transformer::value_at;
use crate::compiler_extensions::transformers::{
    FerroXamlIlTargetTypeMetadataNode, FerroXamlIlWellKnownTypesExtensions, ScopeTypes,
};

/// Wraps a `ControlTheme` in a style target-type scope for its `TargetType`.
pub struct FerroXamlIlControlThemeTransformer;

impl IXamlAstTransformer for FerroXamlIlControlThemeTransformer {
    fn transform(
        &self,
        context: &AstTransformationContext,
        node: Rc<dyn IXamlAstNode>,
    ) -> XamlResult<Rc<dyn IXamlAstNode>> {
        let Some(on) = node.cast::<XamlAstObjectNode>() else {
            return Ok(node);
        };
        if !context
            .get_ferro_types()
            .control_theme
            .is_assignable_from(&*on.type_().get_clr_type()?)
        {
            return Ok(node);
        }

        // Check if we've already transformed this node.
        if context
            .first_parent_node()
            .is_some_and(|p| p.is::<FerroXamlIlTargetTypeMetadataNode>())
        {
            return Ok(node);
        }

        let mut target_type_node: Option<Rc<XamlAstXamlPropertyValueNode>> = None;
        let children = on.children.borrow().clone();
        for child in children {
            if let Some(p) = child.cast::<XamlAstXamlPropertyValueNode>() {
                if p.property().get_clr_property()?.name() == "TargetType" {
                    target_type_node = Some(p);
                    break;
                }
            }
        }
        let Some(target_type_node) = target_type_node else {
            return Err(XamlError::transform_exception(
                "ControlTheme must have a TargetType.",
                Some(&*node),
            ));
        };

        let first_value = value_at(&target_type_node.values.borrow(), 0)?;
        let target_type: Rc<dyn IXamlType> =
            if let Some(extension) = first_value.cast::<XamlTypeExtensionNode>() {
                extension.value().get_clr_type()?
            } else if let Some(text) = first_value.cast::<XamlAstTextNode>() {
                TypeReferenceResolver::resolve_type_by_xml_name(
                    context,
                    &text.text(),
                    false,
                    &*text,
                    true,
                )?
                .type_
                .clone()
            } else {
                return Err(XamlError::transform_exception(
                    "Could not determine TargetType for ControlTheme.",
                    Some(&*target_type_node),
                ));
            };

        let value: Rc<dyn IXamlAstValueNode> = on;
        Ok(FerroXamlIlTargetTypeMetadataNode::new(
            value,
            XamlAstClrTypeReference::new(&*target_type_node, target_type, false),
            ScopeTypes::Style,
        ))
    }
}
