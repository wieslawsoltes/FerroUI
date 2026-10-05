//! Port of `Transform/Transformers/ResolveContentPropertyTransformer.cs`.

use std::rc::Rc;

use crate::ast::{
    IXamlAstNode, IXamlAstValueNode, IXamlPropertySetter, XamlAstClrProperty, XamlAstExtensions,
    XamlAstNodeExtensions, XamlAstObjectNode, XamlAstXamlPropertyValueNode,
    XamlDirectCallPropertySetter,
};
use crate::exceptions::{XamlError, XamlResult};
use crate::transform::{
    AstTransformationContext, IXamlAstTransformer, WhitespaceNormalization, XamlTransformHelpers,
};

/// For object nodes, this transformer will collect all direct children that are value nodes and
/// wrap them into a property value node, which will be appended to the transformed node's
/// children. The property value node will refer to the target XAML type's content property.
pub struct ResolveContentPropertyTransformer;

impl IXamlAstTransformer for ResolveContentPropertyTransformer {
    fn transform(
        &self,
        context: &AstTransformationContext,
        node: Rc<dyn IXamlAstNode>,
    ) -> XamlResult<Rc<dyn IXamlAstNode>> {
        let Some(ni) = node.cast::<XamlAstObjectNode>() else {
            return Ok(node);
        };

        let mut property_node: Option<Rc<XamlAstXamlPropertyValueNode>> = None;

        let mut c = ni.children.borrow().len();
        while c > 0 {
            c -= 1;
            // The list may shrink while whitespace nodes are removed below.
            let Some(child) = ni.children.borrow().get(c).cloned() else {
                continue;
            };
            let Some(value_node) = child.cast::<dyn IXamlAstValueNode>() else {
                continue;
            };

            if property_node.is_none() {
                let ni_type = ni.type_.borrow().get_clr_type()?;
                let content_property = context.configuration().find_content_property(&ni_type)?;
                if let Some(content_property) = content_property {
                    property_node = Some(XamlAstXamlPropertyValueNode::with_values(
                        &*ni,
                        XamlAstClrProperty::from_property(
                            &*ni,
                            &content_property,
                            context.configuration(),
                        )?,
                        Vec::new(),
                        false,
                    ));
                } else {
                    let adders = XamlTransformHelpers::find_possible_adders(context, &ni_type)?;
                    if adders.is_empty() {
                        // If there's no content property, strip all whitespace-only nodes and continue
                        WhitespaceNormalization::remove_whitespace_nodes(
                            &mut ni.children.borrow_mut(),
                        );
                        if !ni.children.borrow().iter().any(|n| n.same_node(&child)) {
                            continue;
                        }

                        return Err(XamlError::transform_exception(
                            format!(
                                "No Content property or any Add methods found for type {}",
                                ni_type.get_fqn()
                            ),
                            Some(&*child),
                        ));
                    }

                    let mut setters: Vec<Rc<dyn IXamlPropertySetter>> =
                        Vec::with_capacity(adders.len());
                    for a in adders {
                        let setter = XamlDirectCallPropertySetter::new(a)?;
                        setter.binder_parameters().allow_multiple.set(true);
                        setters.push(setter);
                    }
                    property_node = Some(XamlAstXamlPropertyValueNode::with_values(
                        &*ni,
                        XamlAstClrProperty::with_setters(
                            &*ni,
                            "Content",
                            ni_type,
                            None,
                            Some(setters),
                            None,
                        ),
                        Vec::new(),
                        false,
                    ));
                }
            }

            // We are going in reverse order, so insert at the beginning
            if let Some(property_node) = &property_node {
                property_node.values.borrow_mut().insert(0, value_node);
            }
            ni.children.borrow_mut().remove(c);
        }

        if let Some(property_node) = property_node {
            ni.children.borrow_mut().push(property_node);
        }

        Ok(node)
    }
}
