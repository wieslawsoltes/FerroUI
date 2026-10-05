//! Port of `Transform/Transformers/TextNodeMerger.cs`.

use std::rc::Rc;

use crate::ast::{IXamlAstNode, XamlAstNodeExtensions, XamlAstObjectNode, XamlAstTextNode};
use crate::exceptions::XamlResult;
use crate::transform::{AstTransformationContext, IXamlAstTransformer};

/// Merges adjacent text nodes
pub struct TextNodeMerger;

impl IXamlAstTransformer for TextNodeMerger {
    fn transform(
        &self,
        _context: &AstTransformationContext,
        node: Rc<dyn IXamlAstNode>,
    ) -> XamlResult<Rc<dyn IXamlAstNode>> {
        if let Some(object_node) = node.cast::<XamlAstObjectNode>() {
            let mut children = object_node.children.borrow_mut();
            let mut next_node_is_text_node = false;
            let mut i = children.len();
            while i > 0 {
                i -= 1;
                let child_node = children[i].clone();
                if let Some(text_node) = child_node.cast::<XamlAstTextNode>() {
                    // If childNode is the first node in a chain of text nodes, merge it with all subsequent
                    // text nodes, and remove them.
                    if next_node_is_text_node
                        && (i == 0 || !children[i - 1].is::<XamlAstTextNode>())
                    {
                        let mut new_text = text_node.text();
                        while i + 1 < children.len() {
                            let Some(next_text_node) = children[i + 1].cast::<XamlAstTextNode>()
                            else {
                                break;
                            };
                            new_text.push_str(&next_text_node.text.borrow());
                            children.remove(i + 1);
                        }

                        *text_node.text.borrow_mut() = new_text;
                    }

                    next_node_is_text_node = true;
                } else {
                    next_node_is_text_node = false;
                }
            }
        }

        Ok(node)
    }
}
