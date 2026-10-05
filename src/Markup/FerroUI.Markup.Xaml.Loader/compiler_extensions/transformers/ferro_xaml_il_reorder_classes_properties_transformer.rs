//! Port of `CompilerExtensions/Transformers/FerroXamlIlReorderClassesPropertiesTransformer.cs`.

use std::rc::Rc;

use xamlx::ast::{
    IXamlAstNode, XamlAstClrProperty, XamlAstNodeExtensions, XamlAstObjectNode,
    XamlAstXamlPropertyValueNode,
};
use xamlx::exceptions::XamlResult;
use xamlx::transform::{AstTransformationContext, IXamlAstTransformer};

use super::FerroXamlIlWellKnownTypesExtensions;

/// Moves the `Classes` assignment of an object in front of its first `Classes.<name>`
/// assignment, so that the class list is assigned before single classes are toggled.
pub struct FerroXamlIlReorderClassesPropertiesTransformer;

impl IXamlAstTransformer for FerroXamlIlReorderClassesPropertiesTransformer {
    fn transform(
        &self,
        context: &AstTransformationContext,
        node: Rc<dyn IXamlAstNode>,
    ) -> XamlResult<Rc<dyn IXamlAstNode>> {
        if let Some(obj) = node.cast::<XamlAstObjectNode>() {
            let mut classes_node: Option<Rc<dyn IXamlAstNode>> = None;
            let mut first_single_class_node: Option<Rc<dyn IXamlAstNode>> = None;
            let types = context.try_get_ferro_types()?;
            for child in obj.children.borrow().iter() {
                let prop = child
                    .cast::<XamlAstXamlPropertyValueNode>()
                    .and_then(|prop_value| prop_value.property().cast::<XamlAstClrProperty>());
                if let Some(prop) = prop {
                    if prop.declaring_type().equals(&*types.classes) {
                        if first_single_class_node.is_none() {
                            first_single_class_node = Some(child.clone());
                        }
                    } else if prop.name() == "Classes"
                        && prop.declaring_type().equals(&*types.styled_element)
                    {
                        classes_node = Some(child.clone());
                    }
                }
            }

            if let (Some(classes_node), Some(first_single_class_node)) =
                (classes_node, first_single_class_node)
            {
                let mut children = obj.children.borrow_mut();
                if let Some(index) = children.iter().position(|c| c.same_node(&classes_node)) {
                    children.remove(index);
                }
                // `IndexOf` always finds the node: it was taken from this list and is not the
                // one that was just removed.
                let index = children
                    .iter()
                    .position(|c| c.same_node(&first_single_class_node))
                    .unwrap_or(0);
                children.insert(index, classes_node);
            }
        }
        Ok(node)
    }
}
