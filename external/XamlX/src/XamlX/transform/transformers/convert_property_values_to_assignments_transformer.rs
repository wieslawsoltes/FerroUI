//! Port of `Transform/Transformers/ConvertPropertyValuesToAssignmentsTransformer.cs`.

use std::cell::RefCell;
use std::rc::Rc;

use crate::ast::{
    IXamlAstManipulationNode, IXamlAstNode, IXamlAstValueNode, IXamlPropertySetter, XamlAstCast,
    XamlAstExtensions, XamlAstNodeExtensions, XamlAstObjectNode,
    XamlAstPropertyReferenceExtensions, XamlAstXamlPropertyValueNode, XamlAstXmlDirective,
    XamlManipulationGroupNode, XamlMarkupExtensionNode, XamlObjectInitializationNode,
    XamlPropertyAssignmentNode,
};
use crate::exceptions::{XamlError, XamlResult};
use crate::transform::{AstTransformationContext, IXamlAstTransformer, XamlTransformHelpers};
use crate::type_system::{IXamlType, XamlPseudoType};
use crate::xaml_namespaces::XamlNamespaces;

/// Converts from AST node `XamlAstXamlPropertyValueNode` to `XamlPropertyAssignmentNode`
/// for code generation.
pub struct ConvertPropertyValuesToAssignmentsTransformer;

impl IXamlAstTransformer for ConvertPropertyValuesToAssignmentsTransformer {
    fn transform(
        &self,
        context: &AstTransformationContext,
        node: Rc<dyn IXamlAstNode>,
    ) -> XamlResult<Rc<dyn IXamlAstNode>> {
        let Some(value_node) = node.cast::<XamlAstXamlPropertyValueNode>() else {
            return Ok(node);
        };

        let property = value_node.property().get_clr_property()?;
        let mut assignments: Vec<Rc<XamlPropertyAssignmentNode>> = Vec::new();
        let object_type = context.configuration().well_known_types().object.clone();

        let group = |assignments: &[Rc<XamlPropertyAssignmentNode>]| -> Rc<dyn IXamlAstNode> {
            XamlManipulationGroupNode::new(
                &*value_node,
                Some(
                    assignments
                        .iter()
                        .map(|a| a.clone() as Rc<dyn IXamlAstManipulationNode>)
                        .collect(),
                ),
            )
        };

        let values = value_node.values.borrow().clone();
        for v in &values {
            let key_node = find_and_remove_key(context, v)?;
            let mut arguments: Vec<Rc<dyn IXamlAstValueNode>> = Vec::new();

            if let Some(key_node) = &key_node {
                arguments.push(key_node.clone());
            }
            arguments.push(v.clone());

            // Pre-filter setters by non-last argument
            let mut filtered_setters: Vec<Rc<dyn IXamlPropertySetter>> = property
                .setters()
                .into_iter()
                .filter(|s| s.parameters().len() == arguments.len())
                .collect();

            if arguments.len() > 1 {
                for c in 0..arguments.len() - 1 {
                    let mut converted_to: Option<Rc<dyn IXamlType>> = None;
                    let mut s = 0;
                    while s < filtered_setters.len() {
                        let setter = filtered_setters[s].clone();
                        match &converted_to {
                            None => {
                                let converted = XamlTransformHelpers::try_get_correctly_typed_value_with_attributes(
                                    context,
                                    &arguments[c],
                                    Some(&setter.custom_attributes()),
                                    &setter.parameters()[c],
                                )?;
                                match converted {
                                    None => {
                                        // As upstream, the removed index is the argument index.
                                        if c >= filtered_setters.len() {
                                            return Err(XamlError::internal(
                                                "ArgumentOutOfRangeException",
                                                "Index was out of range. Must be non-negative and less than the size of the collection.",
                                            ));
                                        }
                                        filtered_setters.remove(c);
                                        continue;
                                    }
                                    Some(converted) => {
                                        converted_to = Some(converted.type_().get_clr_type()?);
                                        arguments[c] = converted;
                                    }
                                }
                            }
                            Some(converted_to) => {
                                if !setter.parameters()[c].is_assignable_from(&**converted_to) {
                                    return Err(XamlError::load_exception(
                                        format!(
                                            "Runtime setter selection is not supported for non-last setter arguments (e. g. x:Key) and can not downcast argument {c} of the setter from {} to {}",
                                            converted_to.to_type_string(),
                                            setter.parameters()[c].to_type_string()
                                        ),
                                        Some(&*arguments[c]),
                                    ));
                                }
                            }
                        }
                        s += 1;
                    }
                }
            }

            let mut create_assignment = || -> XamlResult<Option<Rc<XamlPropertyAssignmentNode>>> {
                let mut matched_setters: Vec<Rc<dyn IXamlPropertySetter>> = Vec::new();
                for setter in &filtered_setters {
                    let can_assign = |value: &Rc<dyn IXamlAstValueNode>,
                                      type_: &Rc<dyn IXamlType>|
                     -> XamlResult<bool> {
                        let binder_parameters = setter.binder_parameters();
                        if !binder_parameters.allow_attribute_syntax.get()
                            && value_node.is_attribute_syntax
                        {
                            return Ok(false);
                        }
                        let value_type = value.type_().get_clr_type()?;
                        // Don't allow x:Null
                        if !binder_parameters.allow_x_null.get()
                            && XamlPseudoType::is_null(&*value_type)
                        {
                            return Ok(false);
                        }

                        // Direct cast
                        if type_.is_assignable_from(&*value_type) {
                            return Ok(true);
                        }

                        // Upcast from System.Object
                        if value_type.equals(&*object_type) {
                            return Ok(true);
                        }

                        Ok(false)
                    };

                    let value_arg_index = arguments.len() - 1;
                    let value_arg = arguments[value_arg_index].clone();
                    let setter_type = setter.parameters()[value_arg_index].clone();

                    if can_assign(&value_arg, &setter_type)? {
                        matched_setters.push(setter.clone());
                    }
                    // Converted value have more priority than custom setters, so we just create a setter without an alternative
                    else if let Some(converted) = XamlTransformHelpers::try_convert_value(
                        context,
                        &value_arg,
                        Some(&setter.custom_attributes()),
                        &setter_type,
                        Some(&property),
                    )? {
                        arguments[value_arg_index] = converted;
                        return Ok(Some(XamlPropertyAssignmentNode::new(
                            &*value_node,
                            property.clone(),
                            vec![setter.clone()],
                            arguments.clone(),
                        )));
                    }
                }

                if !matched_setters.is_empty() {
                    return Ok(Some(XamlPropertyAssignmentNode::new(
                        &**v,
                        property.clone(),
                        matched_setters,
                        arguments.clone(),
                    )));
                }

                let v_type = v.type_().get_clr_type()?;
                // Current node was already skipped due an error, and it always will have unknown type, so ignore it.
                if XamlPseudoType::is_unknown(&*property.declaring_type())
                    || XamlPseudoType::is_unknown(&*v_type)
                {
                    return Ok(None);
                }

                let key_part = match &key_node {
                    Some(key_node) => {
                        format!(
                            " and x:Key of type {}",
                            key_node.type_().get_clr_type()?.to_type_string()
                        )
                    }
                    None => String::new(),
                };
                let parameter_lists: Vec<String> = filtered_setters
                    .iter()
                    .map(|setter| {
                        setter
                            .parameters()
                            .iter()
                            .map(|p| p.full_name())
                            .collect::<Vec<_>>()
                            .join(", ")
                    })
                    .collect();
                Err(XamlError::load_exception(
                    format!(
                        "Unable to find suitable setter or adder for property {} of type {} for argument {}{}, available setter parameter lists are:\n{}",
                        property.name(),
                        property.declaring_type().get_fqn(),
                        v_type.get_fqn(),
                        key_part,
                        parameter_lists.join("\n")
                    ),
                    Some(&**v),
                ))
            };

            match create_assignment()? {
                Some(assignment) => assignments.push(assignment),
                None => return Ok(group(&assignments)),
            }
        }

        if assignments.len() == 1 {
            return Ok(assignments[0].clone());
        }

        if assignments.len() > 1 {
            // Skip the first one, since we only care about further setters, e. g. the following is perfectly valid:
            // <Foo.Bar>
            //   <SomeList/>
            //   <ListItem/>
            //   <ListItem/>
            // </Foo.Bar>
            // <SomeList/> would be foo.Bar = new SomeList() and <ListItem/> would be foo.Bar.Add(new ListItem());
            for ass in assignments.iter().skip(1) {
                let mut possible_setters = ass.possible_setters.borrow_mut();
                possible_setters.retain(|s| s.binder_parameters().allow_multiple.get());
                if possible_setters.is_empty() {
                    return Err(XamlError::load_exception(
                        format!(
                            "Unable to find a setter that allows multiple assignments to the property {} of type {}",
                            ass.property.name(),
                            ass.property.declaring_type().get_fqn()
                        ),
                        Some(&*node),
                    ));
                }
            }
        }

        Ok(group(&assignments))
    }
}

fn is_key_directive(node: &Rc<dyn IXamlAstNode>) -> Option<Rc<XamlAstXmlDirective>> {
    node.cast::<XamlAstXmlDirective>().filter(|d| {
        d.namespace.borrow().as_deref() == Some(XamlNamespaces::XAML2006)
            && *d.name.borrow() == "Key"
    })
}

fn process_directive(
    context: &AstTransformationContext,
    directive: &Rc<XamlAstXmlDirective>,
    key_node: &mut Option<Rc<dyn IXamlAstValueNode>>,
) -> XamlResult<()> {
    if directive.values.borrow().len() != 1 {
        context.report_transform_error(
            "Invalid number of arguments for x:Key directive",
            Some(&**directive),
            (),
        )?;
    }
    *key_node = directive.values.borrow().first().cloned();
    Ok(())
}

fn process_directive_candidate_list<K: ?Sized + XamlAstCast>(
    context: &AstTransformationContext,
    nodes: &RefCell<Vec<Rc<K>>>,
    key_node: &mut Option<Rc<dyn IXamlAstValueNode>>,
) -> XamlResult<()> {
    let found = nodes
        .borrow()
        .iter()
        .enumerate()
        .find_map(|(index, n)| is_key_directive(&n.as_node()).map(|d| (index, d)));
    if let Some((index, directive)) = found {
        process_directive(context, &directive, key_node)?;
        nodes.borrow_mut().remove(index);
    }
    Ok(())
}

fn visit_manipulation_node(
    context: &AstTransformationContext,
    man: Option<Rc<dyn IXamlAstManipulationNode>>,
    key_node: &mut Option<Rc<dyn IXamlAstValueNode>>,
) -> XamlResult<Option<Rc<dyn IXamlAstManipulationNode>>> {
    let Some(man) = man else { return Ok(None) };
    if let Some(directive) = is_key_directive(&man.as_node()) {
        process_directive(context, &directive, key_node)?;
        return Ok(Some(XamlManipulationGroupNode::new(&*man, None)));
    }
    if let Some(grp) = man.cast::<XamlManipulationGroupNode>() {
        process_directive_candidate_list(context, &grp.children, key_node)?;
    }
    if let Some(init) = man.cast::<XamlObjectInitializationNode>() {
        let inner = init.manipulation();
        if let Some(visited) = visit_manipulation_node(context, Some(inner), key_node)? {
            *init.manipulation.borrow_mut() = visited;
        }
    }
    Ok(Some(man))
}

fn find_and_remove_key(
    context: &AstTransformationContext,
    value: &Rc<dyn IXamlAstValueNode>,
) -> XamlResult<Option<Rc<dyn IXamlAstValueNode>>> {
    let mut key_node: Option<Rc<dyn IXamlAstValueNode>> = None;

    let probe = match value.as_value_with_side_effect_node_base() {
        Some(side) => side.value(),
        None => value.clone(),
    };

    if let Some(ast_object) = probe.cast::<XamlAstObjectNode>() {
        process_directive_candidate_list(context, &ast_object.children, &mut key_node)?;
    } else if let Some(vman) = value.as_value_with_manipulation_node() {
        let visited = visit_manipulation_node(context, vman.manipulation(), &mut key_node)?;
        *vman.manipulation.borrow_mut() = visited;
    } else if let Some(mext) = value.cast::<XamlMarkupExtensionNode>() {
        return find_and_remove_key(context, &mext.value());
    }

    Ok(key_node)
}
