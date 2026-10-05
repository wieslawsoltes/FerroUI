//! Port of `Transform/Transformers/TopDownInitializationTransformer.cs`.

use std::rc::Rc;

use crate::ast::{
    IXamlAstManipulationNode, IXamlAstNode, IXamlAstValueNode, XamlAstCompilerLocalNode,
    XamlAstExtensions, XamlAstImperativeValueManipulation, XamlAstLocalInitializationNodeEmitter,
    XamlAstManipulationImperativeNode, XamlAstNodeExtensions, XamlManipulationGroupNode,
    XamlNoReturnMethodCallNode, XamlObjectInitializationNode, XamlPropertyAssignmentNode,
    XamlValueNodeWithBeginInit,
};
use crate::exceptions::{XamlError, XamlResult};
use crate::transform::{AstTransformationContext, IXamlAstTransformer};
use crate::type_system::IXamlType;

pub struct TopDownInitializationTransformer;

fn usable_during_initialization(
    type_: &Rc<dyn IXamlType>,
    usable_attrs: &[Rc<dyn IXamlType>],
) -> bool {
    for attr in type_.custom_attributes() {
        let attr_type = attr.type_();
        for usable_attr_type in usable_attrs {
            if attr_type.equals(&**usable_attr_type) {
                let parameters = attr.parameters();
                return parameters.is_empty() || parameters[0].as_bool() == Some(true);
            }
        }
    }

    match type_.base_type() {
        Some(base_type) => usable_during_initialization(&base_type, usable_attrs),
        None => false,
    }
}

type Converted = (Rc<dyn IXamlAstValueNode>, Rc<dyn IXamlAstManipulationNode>);

fn try_convert(
    checked_node: &Rc<dyn IXamlAstValueNode>,
    usable_attrs: &[Rc<dyn IXamlType>],
) -> XamlResult<Option<Converted>> {
    let Some(manipulation) = checked_node.as_value_with_manipulation_node() else {
        return Ok(None);
    };
    let Some(initializer) = manipulation
        .manipulation()
        .and_then(|m| m.cast::<XamlObjectInitializationNode>())
    else {
        return Ok(None);
    };
    let manipulation_value = manipulation.value();
    if !usable_during_initialization(&manipulation_value.type_().get_clr_type()?, usable_attrs) {
        return Ok(None);
    }
    initializer.skip_begin_init.set(true);
    let local = XamlAstCompilerLocalNode::new(
        &*manipulation_value,
        manipulation_value.type_().get_clr_type_reference()?,
    );
    let value: Rc<dyn IXamlAstValueNode> = XamlValueNodeWithBeginInit::new(
        XamlAstLocalInitializationNodeEmitter::new(&*local, manipulation_value, local.clone()),
    );
    let deferred: Rc<dyn IXamlAstManipulationNode> = XamlAstManipulationImperativeNode::new(
        &*initializer,
        XamlAstImperativeValueManipulation::new(&*initializer, local, initializer.clone()),
    );
    Ok(Some((value, deferred)))
}

impl IXamlAstTransformer for TopDownInitializationTransformer {
    fn transform(
        &self,
        context: &AstTransformationContext,
        node: Rc<dyn IXamlAstNode>,
    ) -> XamlResult<Rc<dyn IXamlAstNode>> {
        let usable_attrs = &context
            .configuration()
            .type_mappings
            .usable_during_initialization_attributes;
        if usable_attrs.is_empty() {
            return Ok(node);
        }

        if let Some(assignment) = node.cast::<XamlPropertyAssignmentNode>() {
            let last = assignment
                .values
                .borrow()
                .last()
                .cloned()
                .ok_or_else(|| XamlError::invalid_operation("Sequence contains no elements"))?;
            let Some((nvalue, deferred)) = try_convert(&last, usable_attrs)? else {
                return Ok(node);
            };

            if let Some(slot) = assignment.values.borrow_mut().last_mut() {
                *slot = nvalue;
            }
            return Ok(XamlManipulationGroupNode::new(
                &*assignment,
                Some(vec![assignment.clone(), deferred]),
            ));
        } else if let Some(call) = node.cast::<XamlNoReturnMethodCallNode>() {
            let mut deferred_nodes: Vec<Rc<dyn IXamlAstManipulationNode>> = Vec::new();
            let count = call.base.arguments.borrow().len();
            for c in 0..count {
                let arg = call.base.arguments.borrow()[c].clone();
                if let Some((narg, deferred)) = try_convert(&arg, usable_attrs)? {
                    call.base.arguments.borrow_mut()[c] = narg;
                    deferred_nodes.push(deferred);
                }
            }

            if !deferred_nodes.is_empty() {
                let mut children: Vec<Rc<dyn IXamlAstManipulationNode>> = vec![call.clone()];
                children.extend(deferred_nodes);
                return Ok(XamlManipulationGroupNode::new(&*call, Some(children)));
            }
        }

        Ok(node)
    }
}
