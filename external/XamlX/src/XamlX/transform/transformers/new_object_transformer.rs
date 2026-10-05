//! Port of `Transform/Transformers/NewObjectTransformer.cs`.

use std::rc::Rc;

use crate::ast::{
    IXamlAstManipulationNode, IXamlAstNode, XamlAstConstructableObjectNode, XamlAstExtensions,
    XamlAstNewClrObjectNode, XamlAstNodeExtensions, XamlManipulationGroupNode,
    XamlObjectInitializationNode, XamlValueWithManipulationNode,
};
use crate::exceptions::{XamlError, XamlResult};
use crate::transform::{AstTransformationContext, IXamlAstTransformer};

pub struct NewObjectTransformer;

impl IXamlAstTransformer for NewObjectTransformer {
    fn transform(
        &self,
        _context: &AstTransformationContext,
        node: Rc<dyn IXamlAstNode>,
    ) -> XamlResult<Rc<dyn IXamlAstNode>> {
        if let Some(obj) = node.cast::<XamlAstConstructableObjectNode>() {
            let type_ = obj.type_.borrow().clone();
            let mut children: Vec<Rc<dyn IXamlAstManipulationNode>> = Vec::new();
            for child in obj.children.borrow().iter() {
                children.push(
                    child
                        .cast::<dyn IXamlAstManipulationNode>()
                        .ok_or_else(|| {
                            XamlError::invalid_cast(format!(
                        "Unable to cast object of type '{}' to type 'IXamlAstManipulationNode'.",
                        child.type_name()
                    ))
                        })?,
                );
            }
            return Ok(XamlValueWithManipulationNode::new(
                &*obj,
                XamlAstNewClrObjectNode::new(
                    &*obj,
                    type_.get_clr_type_reference()?,
                    obj.constructor.clone(),
                    obj.arguments.borrow().clone(),
                ),
                Some(XamlObjectInitializationNode::new(
                    &*obj,
                    XamlManipulationGroupNode::new(&*obj, Some(children)),
                    type_.get_clr_type()?,
                )),
            ));
        }

        Ok(node)
    }
}
