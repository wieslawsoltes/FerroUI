//! Port of `CompilerExtensions/Transformers/FerroXamlIlControlTemplatePriorityTransformer.cs`.

use std::rc::Rc;

use xamlx::ast::{
    IXamlAstNode, IXamlAstValueNode, IXamlPropertySetter, XamlAstNodeExtensions, XamlConstantNode,
    XamlPropertyAssignmentNode,
};
use xamlx::exceptions::XamlResult;
use xamlx::transform::{AstTransformationContext, IXamlAstTransformer};
use xamlx::type_system::XamlValue;

use super::ferro_xaml_il_selector_transformer::value_at;
use crate::compiler_extensions::transformers::{
    FerroXamlIlTargetTypeMetadataNode, FerroXamlIlWellKnownTypesExtensions, ScopeTypes,
};
use crate::compiler_extensions::XamlIlFerroProperty;

/// `(int)BindingPriority.Template`.
const BINDING_PRIORITY_TEMPLATE: i32 = 2;

/// Transforms property assignments within ControlTemplates to use Style priority where possible.
pub struct FerroXamlIlControlTemplatePriorityTransformer;

impl IXamlAstTransformer for FerroXamlIlControlTemplatePriorityTransformer {
    fn transform(
        &self,
        context: &AstTransformationContext,
        node: Rc<dyn IXamlAstNode>,
    ) -> XamlResult<Rc<dyn IXamlAstNode>> {
        let binding_priority_type = context.get_ferro_types().binding_priority.clone();

        // The node is a candidate for transformation if:
        // - It's a property assignment to a registered property
        // - There's a ControlTemplate ancestor
        // - The property has a single value
        let Some(prop) = node.cast::<XamlPropertyAssignmentNode>() else {
            return Ok(node);
        };
        if XamlIlFerroProperty::from_clr_property(&prop.property).is_none() {
            return Ok(node);
        }
        let ferro_property = &prop.property;
        if !context.parent_nodes().iter().any(|c| {
            c.cast::<FerroXamlIlTargetTypeMetadataNode>()
                .is_some_and(|n| n.scope_type == ScopeTypes::ControlTemplate)
        }) {
            return Ok(node);
        }
        if prop.values.borrow().len() != 1 {
            return Ok(node);
        }

        let mut priority_value_setters: Vec<Rc<dyn IXamlPropertySetter>> = Vec::new();

        // Iterate through the possible setters, trying to find a setter on the property
        // which has a BindingPriority parameter followed by the parameter of the existing
        // setter.
        let possible_setters = prop.possible_setters.borrow().clone();
        for setter in possible_setters {
            let mut found: Option<Rc<dyn IXamlPropertySetter>> = None;
            for x in ferro_property.setters() {
                let parameters = x.parameters();
                if value_at(&parameters, 0)?.equals(&*binding_priority_type)
                    && value_at(&parameters, 1)?.equals(&*value_at(&setter.parameters(), 0)?)
                {
                    found = Some(x);
                    break;
                }
            }
            if let Some(s) = found {
                priority_value_setters.push(s);
            }
        }

        // If any BindingPriority setters were found, use those.
        if !priority_value_setters.is_empty() {
            *prop.possible_setters.borrow_mut() = priority_value_setters;
            let priority: Rc<dyn IXamlAstValueNode> = XamlConstantNode::new(
                &*node,
                binding_priority_type,
                XamlValue::Int32(BINDING_PRIORITY_TEMPLATE),
            )?;
            prop.values.borrow_mut().insert(0, priority);
        }

        Ok(node)
    }
}
