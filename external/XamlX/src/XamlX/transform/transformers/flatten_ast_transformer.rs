//! Port of `Transform/Transformers/FlattenAstTransformer.cs`.

use std::rc::Rc;

use crate::ast::{IXamlAstNode, XamlAstNodeExtensions, XamlManipulationGroupNode};
use crate::exceptions::XamlResult;
use crate::transform::{AstTransformationContext, IXamlAstTransformer};

pub struct FlattenAstTransformer;

impl IXamlAstTransformer for FlattenAstTransformer {
    fn transform(
        &self,
        _context: &AstTransformationContext,
        node: Rc<dyn IXamlAstNode>,
    ) -> XamlResult<Rc<dyn IXamlAstNode>> {
        if let Some(group) = node.cast::<XamlManipulationGroupNode>() {
            let children = group.children.borrow();
            if children.len() == 1 {
                return Ok(children[0].clone());
            }
        }
        Ok(node)
    }
}
