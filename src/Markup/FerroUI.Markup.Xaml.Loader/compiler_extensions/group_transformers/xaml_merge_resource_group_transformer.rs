//! Port of `CompilerExtensions/GroupTransformers/XamlMergeResourceGroupTransformer.cs`.

use std::rc::Rc;

use xamlx::ast::{
    IXamlAstManipulationNode, IXamlAstNode, IXamlAstValueNode, XamlAstExtensions,
    XamlAstNewClrObjectNode, XamlAstNodeExtensions, XamlConstantNode, XamlManipulationGroupNode,
    XamlObjectInitializationNode, XamlPropertyAssignmentNode, XamlStaticExtensionNode,
};
use xamlx::exceptions::{XamlError, XamlResult};

use super::{
    AstGroupTransformationContext, FerroXamlIncludeTransformer, IXamlAstGroupTransformer, XamlRuntimeIncludeFallback,
};
use crate::compiler_extensions::transformers::{
    EnsureCapacityNode, FerroXamlIlEnsureResourceDictionaryCapacityTransformer,
    FerroXamlIlWellKnownTypesExtensions,
};
use crate::compiler_extensions::XamlDocumentUsage;

/// Inlines `MergeResourceInclude` entries of `ResourceDictionary.MergedDictionaries` at
/// compile time: the resources of the included dictionary (another document of the same
/// compilation) are added directly to the including dictionary.
pub struct XamlMergeResourceGroupTransformer;

/// `string.Equals(a, b, StringComparison.InvariantCultureIgnoreCase)`.
pub(super) fn invariant_ignore_case_equals(a: Option<&str>, b: &str) -> bool {
    a.is_some_and(|a| a.to_lowercase() == b.to_lowercase())
}

/// `value is XamlValueWithManipulationNode { Manipulation: XamlObjectInitializationNode {
/// Manipulation: XamlManipulationGroupNode group } }`.
fn theme_dictionary_value_group(
    value: &Rc<dyn IXamlAstValueNode>,
) -> Option<Rc<XamlManipulationGroupNode>> {
    value
        .as_value_with_manipulation_node()?
        .manipulation()?
        .cast::<XamlObjectInitializationNode>()?
        .manipulation()
        .cast::<XamlManipulationGroupNode>()
}

/// A `MergeResourceInclude` taken out of the merged dictionaries of a dictionary.
struct MergeSource {
    /// The assignment of its `Source`.
    source: Rc<XamlPropertyAssignmentNode>,
    /// The assignment that added the include to `MergedDictionaries`.
    assignment: Rc<XamlPropertyAssignmentNode>,
}

impl XamlMergeResourceGroupTransformer {
    fn process_xaml_property_assignment_node(
        context: &AstGroupTransformationContext,
        merge_resource_include_type: &Rc<dyn xamlx::type_system::IXamlType>,
        merge_source_nodes: &mut Vec<MergeSource>,
        should_exit: &mut bool,
        parent: &XamlManipulationGroupNode,
        assignment_node: &Rc<XamlPropertyAssignmentNode>,
    ) -> XamlResult<()> {
        if assignment_node.property.name() != "MergedDictionaries" {
            return Ok(());
        }
        let Some(first_value) = assignment_node.values.borrow().first().cloned() else {
            return Ok(());
        };
        let Some(value_node) = first_value.as_value_with_manipulation_node() else {
            return Ok(());
        };

        if first_value
            .type_()
            .get_clr_type()?
            .equals(&**merge_resource_include_type)
        {
            let source_assignment_node = value_node
                .manipulation()
                .and_then(|m| m.cast::<XamlObjectInitializationNode>())
                .and_then(|object_initialization| {
                    object_initialization
                        .manipulation()
                        .cast::<XamlPropertyAssignmentNode>()
                });
            match source_assignment_node {
                Some(source_assignment_node) => {
                    parent
                        .children
                        .borrow_mut()
                        .retain(|c| !c.same_node(assignment_node));
                    merge_source_nodes.push(MergeSource {
                        source: source_assignment_node,
                        assignment: assignment_node.clone(),
                    });
                }
                None => {
                    *should_exit = true;
                    context.report_transform_error_node(
                        "Invalid MergeResourceInclude node found. Make sure that Source property is set.",
                        first_value.as_node(),
                    )?;
                }
            }
        } else if !merge_source_nodes.is_empty() {
            *should_exit = true;
            context.report_transform_error_node(
                "MergeResourceInclude should always be included last when mixing with other dictionaries inside of the ResourceDictionary.MergedDictionaries.",
                first_value.as_node(),
            )?;
        }
        Ok(())
    }

    /// Removes all existing EnsureCapacityNode (from the merged dictionaries) and adds a new one.
    fn fix_ensure_capacity_nodes(
        context: &AstGroupTransformationContext,
        manipulation: &Rc<XamlManipulationGroupNode>,
    ) -> XamlResult<()> {
        manipulation
            .children
            .borrow_mut()
            .retain(|c| !c.is::<EnsureCapacityNode>());
        FerroXamlIlEnsureResourceDictionaryCapacityTransformer::new().apply(context, manipulation)
    }

    pub fn theme_variant_node_equals(
        context: &AstGroupTransformationContext,
        left: &Rc<dyn IXamlAstValueNode>,
        right: &Rc<dyn IXamlAstValueNode>,
    ) -> XamlResult<bool> {
        if let (Some(left_const), Some(right_const)) = (
            left.cast::<XamlConstantNode>(),
            right.cast::<XamlConstantNode>(),
        ) {
            return Ok(left_const.constant == right_const.constant);
        }

        if let (Some(left_static_ext), Some(right_static_ext)) = (
            left.cast::<XamlStaticExtensionNode>(),
            right.cast::<XamlStaticExtensionNode>(),
        ) {
            return Ok(left.type_().get_clr_type()?.get_full_name()
                == right.type_().get_clr_type()?.get_full_name()
                && *left_static_ext.member.borrow() == *right_static_ext.member.borrow());
        }

        if let (Some(left_clr_object_node), Some(right_clr_object_node)) = (
            left.cast::<XamlAstNewClrObjectNode>(),
            right.cast::<XamlAstNewClrObjectNode>(),
        ) {
            let theme_variant = context.try_get_ferro_types()?.theme_variant.clone();
            let left_type = left_clr_object_node.type_.borrow().clone();
            let right_type = right_clr_object_node.type_.borrow().clone();
            // As upstream: the type references and the constructors are compared by reference,
            // and the argument of the left node is compared with itself.
            if !(left_type.get_clr_type()?.equals(&*theme_variant)
                && left_type.same_node(&right_type)
                && std::ptr::addr_eq(
                    Rc::as_ptr(&left_clr_object_node.constructor),
                    Rc::as_ptr(&right_clr_object_node.constructor),
                ))
            {
                return Ok(false);
            }
            let arguments = left_clr_object_node.arguments.borrow().clone();
            if arguments.len() != 1 {
                return Err(XamlError::invalid_operation(if arguments.is_empty() {
                    "Sequence contains no elements"
                } else {
                    "Sequence contains more than one element"
                }));
            }
            return Self::theme_variant_node_equals(context, &arguments[0], &arguments[0]);
        }

        Ok(false)
    }
}

impl IXamlAstGroupTransformer for XamlMergeResourceGroupTransformer {
    fn transform(
        &self,
        context: &AstGroupTransformationContext,
        node: Rc<dyn IXamlAstNode>,
    ) -> XamlResult<Rc<dyn IXamlAstNode>> {
        let ferro_types = context.try_get_ferro_types()?;
        let resource_dictionary_type = ferro_types.resource_dictionary.clone();
        let Some(resource_dictionary_node) = node.cast::<XamlObjectInitializationNode>() else {
            return Ok(node);
        };
        if !resource_dictionary_node
            .type_
            .borrow()
            .equals(&*resource_dictionary_type)
        {
            return Ok(node);
        }
        let Some(resource_dictionary_manipulation) = resource_dictionary_node
            .manipulation()
            .cast::<XamlManipulationGroupNode>()
        else {
            return Ok(node);
        };

        let merge_resource_include_type = ferro_types.merge_resource_include.clone();
        let mut merge_source_nodes: Vec<MergeSource> = Vec::new();
        // if any manipulation node has an error, we should stop processing further.
        let mut should_exit = false;
        let manipulation_nodes = resource_dictionary_manipulation.children.borrow().clone();
        for manipulation_node in manipulation_nodes {
            if let Some(single_value_assignment) =
                manipulation_node.cast::<XamlPropertyAssignmentNode>()
            {
                Self::process_xaml_property_assignment_node(
                    context,
                    &merge_resource_include_type,
                    &mut merge_source_nodes,
                    &mut should_exit,
                    &resource_dictionary_manipulation,
                    &single_value_assignment,
                )?;
            } else if let Some(group_node_values) =
                manipulation_node.cast::<XamlManipulationGroupNode>()
            {
                let group_values: Vec<Rc<XamlPropertyAssignmentNode>> = group_node_values
                    .children
                    .borrow()
                    .iter()
                    .filter_map(|c| c.cast::<XamlPropertyAssignmentNode>())
                    .collect();
                for group_node_value in group_values {
                    Self::process_xaml_property_assignment_node(
                        context,
                        &merge_resource_include_type,
                        &mut merge_source_nodes,
                        &mut should_exit,
                        &group_node_values,
                        &group_node_value,
                    )?;
                }
            }
        }

        if should_exit || merge_source_nodes.is_empty() {
            return Ok(node);
        }

        let mut manipulation_group: Vec<Rc<dyn IXamlAstManipulationNode>> = Vec::new();
        // The includes that stay run-time includes (`XamlRuntimeIncludeFallback`).
        let mut runtime_includes: Vec<Rc<dyn IXamlAstManipulationNode>> = Vec::new();
        for merge_source in &merge_source_nodes {
            let source_node = &merge_source.source;
            let (original_asset_path, property_node) =
                FerroXamlIncludeTransformer::resolve_source_from_xaml_include(
                    context,
                    "MergeResourceInclude",
                    source_node,
                    true,
                )?;
            let Some(original_asset_path) = original_asset_path else {
                return context.report_transform_error(
                    "Node MergeResourceInclude is unable to resolve \"\" path.",
                    property_node
                        .as_deref()
                        .map(|n| n as &dyn xamlx::ast::IXamlLineInfo),
                    node,
                );
            };

            let target_document = context
                .documents()
                .iter()
                .find(|d| invariant_ignore_case_equals(d.uri().as_deref(), &original_asset_path))
                .cloned();
            // A document outside of the compilation that can be loaded at run time: the
            // include stays in the merged dictionaries and loads as a `ResourceInclude`.
            if target_document.is_none() && XamlRuntimeIncludeFallback::applies(context, &original_asset_path) {
                runtime_includes.push(merge_source.assignment.clone());
                continue;
            }
            let target_document_root = match &target_document {
                Some(target_document) => Some(target_document.xaml_document().borrow().root()?),
                None => None,
            };
            let target_document_manipulation = target_document_root
                .as_ref()
                .and_then(|root| root.as_value_with_manipulation_node())
                .map(|root| root.manipulation());
            let (Some(target_document), Some(target_document_manipulation)) =
                (target_document, target_document_manipulation)
            else {
                return context.report_transform_error(
                    &format!(
                        "Node MergeResourceInclude is unable to resolve \"{original_asset_path}\" path."
                    ),
                    property_node
                        .as_deref()
                        .map(|n| n as &dyn xamlx::ast::IXamlLineInfo),
                    node,
                );
            };

            let root_group = target_document_manipulation
                .and_then(|m| m.cast::<XamlManipulationGroupNode>())
                .ok_or_else(|| {
                    XamlError::invalid_cast(
                        "Unable to cast the root manipulation to type 'XamlManipulationGroupNode'.",
                    )
                })?;
            let mut root_objects: Vec<Rc<XamlObjectInitializationNode>> = root_group
                .children
                .borrow()
                .iter()
                .filter_map(|c| c.cast::<XamlObjectInitializationNode>())
                .collect();
            if root_objects.len() != 1 {
                return Err(XamlError::invalid_operation(if root_objects.is_empty() {
                    "Sequence contains no elements"
                } else {
                    "Sequence contains more than one element"
                }));
            }
            let single_root_object = root_objects.remove(0);
            if !single_root_object
                .type_
                .borrow()
                .equals(&*resource_dictionary_type)
            {
                return context.report_transform_error(
                    "MergeResourceInclude can only include another ResourceDictionary",
                    property_node
                        .as_deref()
                        .map(|n| n as &dyn xamlx::ast::IXamlLineInfo),
                    node,
                );
            }

            manipulation_group.push(single_root_object.manipulation());
            if target_document.usage() == XamlDocumentUsage::Unknown {
                target_document.set_usage(XamlDocumentUsage::Merged);
            }
        }

        // Order of resources is defined by ResourceDictionary.TryGetResource.
        // It is read by following priority:
        // - own resources.
        // - own theme dictionaries.
        // - merged dictionaries.
        // We need to maintain this order when we inject "compiled merged" resources.
        // Doing this by injecting merged dictionaries in the beginning, so it can be overwritten by "own resources".
        // MergedDictionaries are read first, so we need ot inject our merged values in the beginning.
        let mut children = resource_dictionary_manipulation.children.borrow().clone();
        children.splice(0..0, manipulation_group);

        // Flatten resource assignments.
        let mut i = 0;
        while i < children.len() {
            if let Some(group) = children[i].cast::<XamlManipulationGroupNode>() {
                children.remove(i);
                children.extend(group.children.borrow().iter().cloned());
                // step back, so new items can be reiterated.
                continue;
            }
            i += 1;
        }

        // Merge "ThemeDictionaries" as well.
        let mut i = children.len();
        while i > 0 {
            i -= 1;
            let Some(assignment_node) = children[i].cast::<XamlPropertyAssignmentNode>() else {
                continue;
            };
            if assignment_node.property.name() != "ThemeDictionaries" {
                continue;
            }
            let values = assignment_node.values.borrow().clone();
            if values.len() != 2 {
                continue;
            }
            let key = values[0].clone();
            let Some(value_group) = theme_dictionary_value_group(&values[1]) else {
                continue;
            };

            let mut j = i;
            while j > 0 {
                j -= 1;
                let Some(same_key_prev_assignment_node) =
                    children[j].cast::<XamlPropertyAssignmentNode>()
                else {
                    continue;
                };
                if same_key_prev_assignment_node.property.name() != "ThemeDictionaries" {
                    continue;
                }
                let prev_values = same_key_prev_assignment_node.values.borrow().clone();
                if prev_values.len() != 2 {
                    continue;
                }
                let Some(same_key_prev_value_group) = theme_dictionary_value_group(&prev_values[1])
                else {
                    continue;
                };
                if Self::theme_variant_node_equals(context, &key, &prev_values[0])? {
                    let added = value_group.children.borrow().clone();
                    same_key_prev_value_group.children.borrow_mut().extend(added);
                    Self::fix_ensure_capacity_nodes(context, &same_key_prev_value_group)?;
                    children.remove(i);
                    break;
                }
            }
        }

        // The run-time includes go back to the merged dictionaries, in their order.
        children.extend(runtime_includes);

        *resource_dictionary_manipulation.children.borrow_mut() = children;
        Self::fix_ensure_capacity_nodes(context, &resource_dictionary_manipulation)?;

        Ok(node)
    }
}
