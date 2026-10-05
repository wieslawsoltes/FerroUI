//! Port of `Transform/Transformers/ContentConvertTransformer.cs`.

use std::rc::Rc;

use crate::ast::{
    IXamlAstManipulationNode, IXamlAstNode, IXamlAstValueNode, XamlAstExtensions,
    XamlAstNodeExtensions, XamlAstObjectNode, XamlAstTextNode, XamlAstXmlDirective,
    XamlManipulationGroupNode, XamlValueWithManipulationNode,
};
use crate::exceptions::{XamlError, XamlResult};
use crate::transform::{AstTransformationContext, IXamlAstTransformer, XamlTransformHelpers};

pub struct ContentConvertTransformer;

impl IXamlAstTransformer for ContentConvertTransformer {
    fn transform(
        &self,
        context: &AstTransformationContext,
        node: Rc<dyn IXamlAstNode>,
    ) -> XamlResult<Rc<dyn IXamlAstNode>> {
        let Some(on) = node.cast::<XamlAstObjectNode>() else {
            return Ok(node);
        };
        let children = on.children.borrow().clone();
        let non_directive_children: Vec<Rc<dyn IXamlAstNode>> = children
            .iter()
            .filter(|a| !a.is::<XamlAstXmlDirective>())
            .cloned()
            .collect();

        if !on.arguments.borrow().is_empty() || non_directive_children.len() != 1 {
            return Ok(node);
        }
        let Some(vn) = non_directive_children[0].cast::<dyn IXamlAstValueNode>() else {
            return Ok(node);
        };
        if !vn
            .type_()
            .get_clr_type()?
            .equals(&*context.configuration().well_known_types().string)
        {
            return Ok(node);
        }

        let on_type = on.type_.borrow().get_clr_type()?;
        if let Some(mut rv) =
            XamlTransformHelpers::try_get_correctly_typed_value(context, &vn, &on_type)?
        {
            if non_directive_children.len() != children.len() {
                let directives: Vec<Rc<dyn IXamlAstManipulationNode>> = children
                    .iter()
                    .filter_map(|c| c.cast::<XamlAstXmlDirective>())
                    .map(|d| d as Rc<dyn IXamlAstManipulationNode>)
                    .collect();
                rv = XamlValueWithManipulationNode::new(
                    &*rv.clone(),
                    rv.clone(),
                    Some(XamlManipulationGroupNode::new(&*rv, Some(directives))),
                );
            }
            return Ok(rv);
        }

        if on_type.is_value_type() {
            return Err(XamlError::load_exception(
                format!(
                    "Unable to convert value {}) to {}",
                    vn.cast::<XamlAstTextNode>()
                        .map(|t| t.text())
                        .unwrap_or_default(),
                    on_type.to_type_string()
                ),
                Some(&*vn),
            ));
        }

        // Parser not found, isn't a value type, probably a regular object creation node with text content
        Ok(node)
    }
}
