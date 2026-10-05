//! Port of `Transform/Transformers/MarkupExtensionTransformer.cs`.

use std::rc::Rc;

use crate::ast::{IXamlAstNode, IXamlAstValueNode, XamlAstNodeExtensions, XamlMarkupExtensionNode};
use crate::exceptions::XamlResult;
use crate::transform::{AstTransformationContext, IXamlAstTransformer, XamlTransformHelpers};

pub struct MarkupExtensionTransformer;

impl IXamlAstTransformer for MarkupExtensionTransformer {
    fn transform(
        &self,
        context: &AstTransformationContext,
        node: Rc<dyn IXamlAstNode>,
    ) -> XamlResult<Rc<dyn IXamlAstNode>> {
        if let Some(vn) = node.cast::<dyn IXamlAstValueNode>() {
            if context
                .first_parent_node()
                .is_some_and(|p| p.is::<XamlMarkupExtensionNode>())
            {
                return Ok(node);
            }

            if let Some(rv) = XamlTransformHelpers::try_convert_markup_extension(context, &vn)? {
                return Ok(rv);
            }
        }

        Ok(node)
    }
}
