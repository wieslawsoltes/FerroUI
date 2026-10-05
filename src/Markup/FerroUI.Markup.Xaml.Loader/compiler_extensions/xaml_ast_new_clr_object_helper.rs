//! Port of `CompilerExtensions/XamlAstNewClrObjectHelper.cs`.

use std::rc::Rc;

use xamlx::ast::{
    IXamlAstValueNode, XamlAstCast, XamlAstNodeExtensions, XamlValueWithManipulationNode,
};

pub struct XamlAstNewClrObjectHelper;

impl XamlAstNewClrObjectHelper {
    /// Tries to resolve the underlying value of a [`XamlValueWithManipulationNode`],
    /// unwrapping any nested [`XamlValueWithManipulationNode`] instances.
    ///
    /// As upstream, a node "is" a `XamlValueWithManipulationNode` when it is one or derives
    /// from it (`IXamlAstNode::as_value_with_manipulation_node`).
    pub fn unwrap_value<TXamlAstValueNode: ?Sized + XamlAstCast>(
        node: &XamlValueWithManipulationNode,
    ) -> Option<Rc<TXamlAstValueNode>> {
        let mut current: Rc<dyn IXamlAstValueNode> = node.value();
        loop {
            let next = match current.as_value_with_manipulation_node() {
                Some(value_with_manipulation) => value_with_manipulation.value(),
                None => break,
            };
            current = next;
            if let Some(typed_value) = current.cast::<TXamlAstValueNode>() {
                return Some(typed_value);
            }
        }

        current.cast::<TXamlAstValueNode>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use xamlx::ast::{
        SkipXamlValueWithManipulationNode, XamlAstClrTypeReference, XamlAstObjectNode,
        XamlAstTextNode, XamlLineInfo,
    };

    use crate::compiler_extensions::transformers::NestedScopeMetadataNode;
    use crate::testing::create_test_framework;

    #[test]
    fn unwraps_nested_value_with_manipulation_nodes() {
        let fw = create_test_framework();
        let line_info = XamlLineInfo::new(1, 1);
        let object = XamlAstObjectNode::new(
            &line_info,
            XamlAstClrTypeReference::new(&line_info, fw.types.control.clone(), false),
        );
        let inner = XamlValueWithManipulationNode::new(&line_info, object.clone(), None);
        let middle = XamlValueWithManipulationNode::new(&line_info, inner.clone(), None);
        let outer = XamlValueWithManipulationNode::new(&line_info, middle.clone(), None);

        // Direct value.
        let direct = XamlAstNewClrObjectHelper::unwrap_value::<XamlAstObjectNode>(&inner)
            .expect("object");
        assert!(direct.same_node(&object));
        // Through nested wrappers.
        let nested = XamlAstNewClrObjectHelper::unwrap_value::<XamlAstObjectNode>(&outer)
            .expect("object");
        assert!(nested.same_node(&object));
        // Asking for the wrapper type itself stops at the first wrapper that is not followed
        // by another one.
        let wrapper =
            XamlAstNewClrObjectHelper::unwrap_value::<XamlValueWithManipulationNode>(&outer)
                .expect("wrapper");
        assert!(wrapper.same_node(&inner));
        // No value of the requested type.
        assert!(XamlAstNewClrObjectHelper::unwrap_value::<XamlAstTextNode>(&outer).is_none());
        // Interfaces work as type arguments.
        assert!(XamlAstNewClrObjectHelper::unwrap_value::<dyn IXamlAstValueNode>(&inner).is_some());

        // A derived wrapper is unwrapped too; other wrapping nodes are not looked through.
        let skip = SkipXamlValueWithManipulationNode::new(&line_info);
        let over_skip = XamlValueWithManipulationNode::new(&line_info, skip, None);
        assert!(XamlAstNewClrObjectHelper::unwrap_value::<XamlAstObjectNode>(&over_skip).is_none());
        let scope = NestedScopeMetadataNode::new(object);
        let over_scope = XamlValueWithManipulationNode::new(&line_info, scope.clone(), None);
        assert!(XamlAstNewClrObjectHelper::unwrap_value::<XamlAstObjectNode>(&over_scope).is_none());
        assert!(
            XamlAstNewClrObjectHelper::unwrap_value::<NestedScopeMetadataNode>(&over_scope)
                .expect("scope")
                .same_node(&scope)
        );
    }
}
