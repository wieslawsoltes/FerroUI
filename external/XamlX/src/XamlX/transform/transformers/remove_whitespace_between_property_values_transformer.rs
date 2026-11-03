//! Port of `Transform/Transformers/RemoveWhitespaceBetweenPropertyValuesTransformer.cs`.

use std::rc::Rc;

use crate::ast::{
    IXamlAstNode, XamlAstObjectNode, XamlAstTextNode,
    XamlAstXamlPropertyValueNode,
};
use crate::exceptions::XamlResult;
use crate::transform::{AstTransformationContext, IXamlAstTransformer, WhitespaceNormalization};

/// This transformer drops insignificant whitespace before and between property value nodes
/// within object nodes.
pub struct RemoveWhitespaceBetweenPropertyValuesTransformer;

impl IXamlAstTransformer for RemoveWhitespaceBetweenPropertyValuesTransformer {
    fn transform(
        &self,
        _context: &AstTransformationContext,
        node: Rc<dyn IXamlAstNode>,
    ) -> XamlResult<Rc<dyn IXamlAstNode>> {
        let mut property_encountered = false;
        if let Some(ni) = node.as_any().downcast_ref::<XamlAstObjectNode>() {
            let mut children = ni.children.borrow_mut();
            let mut c = children.len();
            while c > 0 {
                c -= 1;
                let child = &children[c];
                if child.as_any().is::<XamlAstXamlPropertyValueNode>() {
                    property_encountered = true;
                } else if property_encountered {
                    if let Some(text_node) = child.as_any().downcast_ref::<XamlAstTextNode>() {
                        if WhitespaceNormalization::is_whitespace(&text_node.text.borrow()) {
                            children.remove(c);
                        }
                    }
                }
            }
        }

        Ok(node)
    }
}
