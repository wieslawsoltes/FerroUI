//! Port of `Transform/Transformers/StaticIntrinsicsPostProcessTransformer.cs`.

use std::rc::Rc;

use crate::ast::{IXamlAstNode, XamlAstNodeExtensions, XamlStaticExtensionNode};
use crate::exceptions::{XamlError, XamlResult};
use crate::transform::{AstTransformationContext, IXamlAstTransformer};

pub struct StaticIntrinsicsPostProcessTransformer;

impl IXamlAstTransformer for StaticIntrinsicsPostProcessTransformer {
    fn transform(
        &self,
        _context: &AstTransformationContext,
        node: Rc<dyn IXamlAstNode>,
    ) -> XamlResult<Rc<dyn IXamlAstNode>> {
        if let Some(static_extension) = node.cast::<XamlStaticExtensionNode>() {
            let member = static_extension.resolve_member(true)?;
            if member.is_none() {
                return Err(XamlError::invalid_operation(
                    "Operation is not valid due to the current state of the object.",
                ));
            }
        }

        Ok(node)
    }
}
