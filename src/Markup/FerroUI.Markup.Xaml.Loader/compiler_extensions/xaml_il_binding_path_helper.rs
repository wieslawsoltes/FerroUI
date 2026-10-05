//! Port of `CompilerExtensions/XamlIlBindingPathHelper.cs`: resolution of a parsed compiled
//! binding path against the type system, and the resolved path model.
//!
//! Upstream the resolved path node and its elements generate the IL that builds a
//! `CompiledBindingPath` through `CompiledBindingPathBuilder`. There is no IL back end here:
//! [`XamlIlBindingPathNode`] and the path element types carry their data, and their doc
//! comments state the builder call each one emits, next to the method of the Rust
//! `ferroui_base::data::CompiledBindingPathBuilder` that does the same.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use ferroui_base::data::core::parsers::Node;
use xamlx::ast::{
    visit_node, IXamlAstNode, IXamlAstTypeReference, IXamlAstValueNode, IXamlAstVisitor,
    IXamlLineInfo, XamlAstCast, XamlAstClrTypeReference, XamlAstConstructableObjectNode,
    XamlAstExtensions, XamlAstNode, XamlAstNodeExtensions, XamlAstTextNode,
    XamlPropertyAssignmentNode,
};
use xamlx::extensions::query_node_interface;
use xamlx::exceptions::{XamlError, XamlResult};
use xamlx::transform::transformers::TypeReferenceResolver;
use xamlx::transform::{AstTransformationContext, TransformerConfiguration, XamlTransformHelpers};
use xamlx::type_system::{
    FindMethodMethodSignature, IXamlField, IXamlMethod, IXamlProperty, IXamlType, XamlPseudoType,
    XamlValue,
};
use xamlx::{xaml_ast_node_members, xaml_line_info_impl, xaml_query_interface};

use super::transformers::{
    BindingExpressionNode, FerroNameScopeRegistrationXamlIlNode,
    FerroXamlIlDataContextTypeMetadataNode, FerroXamlIlTargetTypeMetadataNode,
    FerroXamlIlWellKnownTypesExtensions, NestedScopeMetadataNode, ParsedBindingPathNode,
    ScopeTypes,
};
use super::{
    IXamlIlFerroPropertyNode, XamlIlClrPropertyInfo, XamlIlClrPropertyInfoEmitter,
    XamlIlCommandCanExecuteTrampoline, XamlIlCommandExecuteTrampoline, XamlIlFerroPropertyHelper,
    XamlIlPropertyAccessorFactory, XamlIlPropertyInfoAccessorFactoryEmitter,
    XamlIlTrampolineBuilder, XamlIlTypedClrPropertyInfo,
};

/// `Func<IXamlType> startTypeResolver`: yields the type a binding path starts from. It is only
/// called when the path needs it (a path that starts at `$self`, `#name`, ... never does) and
/// fails when no start type is known.
pub type StartTypeResolver<'a> = &'a dyn Fn() -> XamlResult<Rc<dyn IXamlType>>;

pub struct XamlIlBindingPathHelper;

impl XamlIlBindingPathHelper {
    /// `UpdateCompiledBindingExtension(context, binding, startTypeResolver, selfType)`:
    /// resolves the parsed path of the compiled binding extension `binding` (its first
    /// constructor argument or its `Path` assignment), replaces it with the resolved
    /// [`XamlIlBindingPathNode`] and returns the type of the value the binding produces. A
    /// binding without a path produces a value of its start type.
    pub fn update_compiled_binding_extension(
        context: &AstTransformationContext,
        binding: &Rc<XamlAstConstructableObjectNode>,
        start_type_resolver: StartTypeResolver<'_>,
        self_type: &Rc<dyn IXamlType>,
    ) -> XamlResult<Rc<dyn IXamlType>> {
        let first_argument = binding.arguments.borrow().first().cloned();
        if let Some(binding_path) = first_argument
            .as_ref()
            .and_then(|a| a.cast::<ParsedBindingPathNode>())
        {
            let transformed = Self::transform_binding_path(
                context,
                &*binding_path,
                start_type_resolver,
                self_type,
                &binding_path.path(),
            )?;

            let transformed = Self::transform_for_target_typing(transformed, context)?;

            let binding_result_type = transformed.binding_result_type();
            binding.arguments.borrow_mut()[0] = transformed;
            return Ok(binding_result_type);
        }

        if let Some(already_transformed) = first_argument
            .as_ref()
            .and_then(|a| a.cast::<XamlIlBindingPathNode>())
        {
            return Ok(already_transformed.binding_result_type());
        }

        let binding_path_assignment = binding
            .children
            .borrow()
            .iter()
            .filter_map(|c| c.cast::<XamlPropertyAssignmentNode>())
            .find(|v| v.property.name() == "Path");

        let Some(binding_path_assignment) = binding_path_assignment else {
            return start_type_resolver();
        };

        let path_value = binding_path_assignment.values.borrow().first().cloned();
        if let Some(path_node) = path_value
            .as_ref()
            .and_then(|v| v.cast::<XamlIlBindingPathNode>())
        {
            return Ok(path_node.binding_result_type());
        }

        if let Some(binding_path_node) = path_value
            .as_ref()
            .and_then(|v| v.cast::<ParsedBindingPathNode>())
        {
            let transformed = Self::transform_binding_path(
                context,
                &*binding_path_node,
                start_type_resolver,
                self_type,
                &binding_path_node.path(),
            )?;

            let transformed = Self::transform_for_target_typing(transformed, context)?;

            let binding_result_type = transformed.binding_result_type();
            binding_path_assignment.values.borrow_mut()[0] = transformed;
            return Ok(binding_result_type);
        }

        Err(XamlError::invalid_operation(
            "Invalid state of Path property",
        ))
    }

    /// When the binding is assigned to a property of type `ICommand` and the path ends in a
    /// method, the method element is replaced with a method-as-command element.
    fn transform_for_target_typing(
        transformed: Rc<XamlIlBindingPathNode>,
        context: &AstTransformationContext,
    ) -> XamlResult<Rc<XamlIlBindingPathNode>> {
        let parent_node = context
            .parent_nodes()
            .into_iter()
            .find_map(|n| n.cast::<XamlPropertyAssignmentNode>());

        let Some(parent_node) = parent_node else {
            return Ok(transformed);
        };

        let types = context.try_get_ferro_types()?;
        let last_element = transformed.elements.borrow().last().cloned();

        let is_command = Self::get_property_type(context, &parent_node)?
            .is_some_and(|t| t.equals(&*types.i_command));
        if let (true, Some(XamlIlBindingPathElementNode::ClrMethod(method_path_element))) =
            (is_command, last_element)
        {
            let well_known_types = context.configuration().well_known_types();
            let execute_method = method_path_element.method.clone();
            let can_execute_method = execute_method.declaring_type().find_method_by_signature(
                &FindMethodMethodSignature::new(
                    format!("Can{}", execute_method.name()),
                    well_known_types.boolean.clone(),
                    vec![well_known_types.object.clone()],
                ),
            );

            let mut depends_on_properties: Vec<String> = Vec::new();
            if let Some(can_execute_method) = &can_execute_method {
                for attr in can_execute_method.custom_attributes() {
                    if attr.type_().equals(&*types.depends_on_attribute) {
                        // `(string)attr.Parameters[0]!`
                        match attr.parameters().into_iter().next() {
                            Some(XamlValue::String(name)) => depends_on_properties.push(name),
                            Some(other) => {
                                return Err(XamlError::invalid_cast(format!(
                                    "Unable to cast object of type '{}' to type 'System.String'.",
                                    other.type_name()
                                )))
                            }
                            None => {
                                return Err(XamlError::internal(
                                    "ArgumentOutOfRangeException",
                                    "Index was out of range. Must be non-negative and less than the size of the collection. (Parameter 'index')",
                                ))
                            }
                        }
                    }
                }
            }

            let mut elements = transformed.elements.borrow_mut();
            elements.pop();
            elements.push(XamlIlBindingPathElementNode::ClrMethodAsCommand(Rc::new(
                XamlIlClrMethodAsCommandPathElementNode {
                    type_: types.i_command.clone(),
                    execute_method,
                    can_execute_method,
                    depends_on_properties,
                },
            )));
        }

        Ok(transformed)
    }

    fn get_property_type(
        context: &AstTransformationContext,
        node: &Rc<XamlPropertyAssignmentNode>,
    ) -> XamlResult<Option<Rc<dyn IXamlType>>> {
        let setter_type = context.try_get_ferro_types()?.setter.clone();

        if node.property.declaring_type().equals(&*setter_type) && node.property.name() == "Value"
        {
            // The property is a Setter.Value property. We need to get the type of the property that the Setter.Value property is setting.
            let candidate = context
                .parent_nodes()
                .into_iter()
                .skip_while(|x| !x.same_node(node))
                .find_map(|x| x.cast::<XamlAstConstructableObjectNode>());
            let setter = match candidate {
                Some(x) if x.type_().get_clr_type()?.equals(&*setter_type) => Some(x),
                _ => None,
            };
            let property_assignment = setter.and_then(|setter| {
                setter
                    .children
                    .borrow()
                    .iter()
                    .filter_map(|c| c.cast::<XamlPropertyAssignmentNode>())
                    .find(|x| x.property.name() == "Property")
            });
            let property = property_assignment
                .and_then(|assignment| assignment.values.borrow().first().cloned())
                .and_then(|value| value.cast::<dyn IXamlIlFerroPropertyNode>());

            if let Some(property) = property {
                return Ok(Some(property.ferro_property_type()));
            }
        }

        Ok(node.property.getter().map(|getter| getter.return_type()))
    }

    /// `TransformBindingPath(context, lineInfo, startTypeResolver, selfType, bindingExpression)`:
    /// resolves the nodes of a parsed binding path, in order, into path elements. Each element
    /// is looked up on the type of the element before it; the first one on the start type.
    pub fn transform_binding_path(
        context: &AstTransformationContext,
        line_info: &dyn IXamlLineInfo,
        start_type_resolver: StartTypeResolver<'_>,
        self_type: &Rc<dyn IXamlType>,
        binding_expression: &[BindingExpressionNode],
    ) -> XamlResult<Rc<XamlIlBindingPathNode>> {
        let types = context.try_get_ferro_types()?;
        let well_known_types = context.configuration().well_known_types();
        let mut transform_nodes: Vec<XamlIlBindingPathElementNode> = Vec::new();
        let mut nodes: Vec<XamlIlBindingPathElementNode> = Vec::new();

        let get_type = |ns: Option<&str>, name: Option<&str>| -> XamlResult<Rc<dyn IXamlType>> {
            Ok(TypeReferenceResolver::resolve_type_by_xml_name(
                context,
                &format!("{}:{}", ns.unwrap_or_default(), name.unwrap_or_default()),
                false,
                line_info,
                true,
            )?
            .type_
            .clone())
        };

        for ast_node in binding_expression {
            let target_type_resolver = |nodes: &[XamlIlBindingPathElementNode]| match nodes.last()
            {
                None => start_type_resolver(),
                Some(last) => Ok(last.type_()),
            };

            match ast_node {
                BindingExpressionNode::Grammar(Node::EmptyExpression) => {}
                BindingExpressionNode::Grammar(Node::Not) => {
                    transform_nodes.push(XamlIlBindingPathElementNode::Not(Rc::new(
                        XamlIlNotPathElementNode {
                            type_: well_known_types.boolean.clone(),
                        },
                    )));
                }
                BindingExpressionNode::Grammar(Node::Stream) => {
                    let target_type = target_type_resolver(&nodes)?;
                    let observable_of_t = &types.i_observable_of_t;
                    let is_observable = |t: &Rc<dyn IXamlType>| {
                        t.generic_type_definition()
                            .is_some_and(|d| d.equals(&**observable_of_t))
                    };
                    let observable_type = if is_observable(&target_type) {
                        Some(target_type.clone())
                    } else {
                        target_type
                            .get_all_interfaces()
                            .into_iter()
                            .find(|i| is_observable(i))
                    };

                    if let Some(item_type) =
                        observable_type.and_then(|t| t.generic_arguments().into_iter().next())
                    {
                        nodes.push(XamlIlBindingPathElementNode::StreamObservable(Rc::new(
                            XamlIlStreamObservablePathElementNode { type_: item_type },
                        )));
                        continue;
                    }

                    let mut found_task = false;
                    let task_type = &types.task_of_t;

                    let mut current_type = Some(target_type.clone());
                    while let Some(current) = current_type {
                        if current
                            .generic_type_definition()
                            .is_some_and(|d| d.equals(&**task_type))
                        {
                            if let Some(result_type) = current.generic_arguments().into_iter().next()
                            {
                                found_task = true;
                                nodes.push(XamlIlBindingPathElementNode::StreamTask(Rc::new(
                                    XamlIlStreamTaskPathElementNode { type_: result_type },
                                )));
                                break;
                            }
                        }
                        current_type = current.base_type();
                    }
                    if found_task {
                        continue;
                    }
                    return Err(XamlError::transform_exception(
                        format!(
                            "Compiled bindings do not support stream bindings for objects of type {}.",
                            target_type.full_name()
                        ),
                        Some(line_info),
                    ));
                }
                BindingExpressionNode::Grammar(Node::PropertyName {
                    accepts_null,
                    property_name,
                }) => {
                    let target_type = target_type_resolver(&nodes)?;
                    let ferro_property_field_name_maybe = format!("{property_name}Property");
                    let ferro_property_field_maybe =
                        target_type.get_all_fields().into_iter().find(|f| {
                            f.is_static()
                                && f.is_public()
                                && f.name() == ferro_property_field_name_maybe
                        });

                    if let Some(ferro_property_field) = ferro_property_field_maybe {
                        let is_data_context_property = ferro_property_field.name()
                            == "DataContextProperty"
                            && ferro_property_field
                                .declaring_type()
                                .equals(&*types.styled_element);
                        let property_type = if is_data_context_property {
                            nodes.last().and_then(|last| last.data_context_type())
                        } else {
                            None
                        };
                        let property_type = match property_type {
                            Some(property_type) => property_type,
                            None => XamlIlFerroPropertyHelper::get_ferro_property_type(
                                &*ferro_property_field,
                                &types,
                                line_info,
                            )?,
                        };

                        nodes.push(XamlIlBindingPathElementNode::FerroProperty(Rc::new(
                            XamlIlFerroPropertyPropertyPathElementNode {
                                field: ferro_property_field,
                                type_: property_type,
                                accepts_null: *accepts_null,
                            },
                        )));
                    } else if let Some(clr_property) = get_all_defined_properties(&target_type)
                        .into_iter()
                        .find(|p| p.name() == *property_name)
                    {
                        nodes.push(XamlIlBindingPathElementNode::ClrProperty(Rc::new(
                            XamlIlClrPropertyPathElementNode {
                                property: clr_property,
                                accepts_null: *accepts_null,
                                emit_typed: Cell::new(false),
                            },
                        )));
                    } else {
                        let method_candidates: Vec<Rc<dyn IXamlMethod>> =
                            get_all_defined_methods(&target_type)
                                .into_iter()
                                .filter(|p| p.name() == *property_name)
                                .collect();
                        if !method_candidates.is_empty() {
                            let candidate = get_best_command_method(
                                &method_candidates,
                                property_name,
                                &target_type,
                                line_info,
                            )?;
                            nodes.push(XamlIlBindingPathElementNode::ClrMethod(Rc::new(
                                XamlIlClrMethodPathElementNode {
                                    method: candidate,
                                    type_: well_known_types.delegate.clone(),
                                    accepts_null: *accepts_null,
                                },
                            )));
                        } else {
                            return Err(XamlError::transform_exception(
                                format!(
                                    "Unable to resolve property or method of name '{property_name}' on type '{}'.",
                                    target_type.to_type_string()
                                ),
                                Some(line_info),
                            ));
                        }
                    }
                }
                BindingExpressionNode::Grammar(Node::Indexer { arguments }) => {
                    let target_type = target_type_resolver(&nodes)?;
                    if target_type.is_array() {
                        nodes.push(XamlIlBindingPathElementNode::ArrayIndexer(Rc::new(
                            XamlIlArrayIndexerPathElementNode::new(
                                target_type,
                                arguments,
                                line_info,
                            )?,
                        )));
                        continue;
                    }

                    let mut property: Option<Rc<dyn IXamlProperty>> = None;
                    for current_type in traverse_type_hierarchy(&target_type) {
                        let default_member_attribute =
                            current_type.custom_attributes().into_iter().find(|x| {
                                let attribute_type = x.type_();
                                attribute_type.namespace().as_deref() == Some("System.Reflection")
                                    && attribute_type.name() == "DefaultMemberAttribute"
                            });
                        if let Some(default_member_attribute) = default_member_attribute {
                            // `(string)defaultMemberAttribute.Parameters[0]!`
                            let member_name =
                                match default_member_attribute.parameters().into_iter().next() {
                                    Some(XamlValue::String(name)) => name,
                                    Some(other) => {
                                        return Err(XamlError::invalid_cast(format!(
                                            "Unable to cast object of type '{}' to type 'System.String'.",
                                            other.type_name()
                                        )))
                                    }
                                    None => {
                                        return Err(XamlError::internal(
                                            "ArgumentOutOfRangeException",
                                            "Index was out of range. Must be non-negative and less than the size of the collection. (Parameter 'index')",
                                        ))
                                    }
                                };
                            property = current_type
                                .get_all_properties()
                                .into_iter()
                                .find(|x| x.name() == member_name);
                            break;
                        }
                    }
                    let Some(property) = property else {
                        return Err(XamlError::transform_exception(
                            format!(
                                "The type '${}' does not have an indexer.",
                                target_type.to_type_string()
                            ),
                            Some(line_info),
                        ));
                    };

                    let parameters = property.indexer_parameters();

                    let mut values: Vec<Rc<dyn IXamlAstValueNode>> = Vec::new();
                    for (current_param_index, param) in parameters.iter().enumerate() {
                        let Some(argument) = arguments.get(current_param_index) else {
                            return Err(XamlError::internal(
                                "ArgumentOutOfRangeException",
                                "Index was out of range. Must be non-negative and less than the size of the collection. (Parameter 'index')",
                            ));
                        };
                        let text_node: Rc<dyn IXamlAstValueNode> = XamlAstTextNode::with_type(
                            line_info,
                            argument,
                            false,
                            Some(well_known_types.string.clone()),
                        );
                        let Some(converted) =
                            XamlTransformHelpers::try_get_correctly_typed_value_with_attributes(
                                context,
                                &text_node,
                                Some(&property.custom_attributes()),
                                param,
                            )?
                        else {
                            return Err(XamlError::transform_exception(
                                format!(
                                    "Unable to convert indexer parameter value of '{argument}' to {}",
                                    param.get_fqn()
                                ),
                                Some(&*text_node),
                            ));
                        };

                        values.push(converted);
                    }

                    let is_notifying_collection =
                        target_type.get_all_interfaces().iter().any(|i| {
                            i.is("System.Collections.Specialized", "INotifyCollectionChanged")
                        });

                    nodes.push(XamlIlBindingPathElementNode::ClrIndexer(Rc::new(
                        XamlIlClrIndexerPathElementNode {
                            property,
                            values,
                            indexer_key: arguments.join(","),
                            is_notifying_collection,
                        },
                    )));
                }
                BindingExpressionNode::Grammar(Node::AttachedPropertyName {
                    accepts_null,
                    namespace,
                    type_name,
                    property_name,
                }) => {
                    let ferro_property_field_name = format!("{property_name}Property");
                    let property_owner_type = get_type(Some(namespace), Some(type_name))?;
                    let ferro_property_field =
                        property_owner_type.get_all_fields().into_iter().find(|f| {
                            f.is_static() && f.is_public() && f.name() == ferro_property_field_name
                        });

                    let Some(ferro_property_field) = ferro_property_field else {
                        return Err(XamlError::transform_exception(
                            format!(
                                "Unable to find {ferro_property_field_name} field on type {}",
                                property_owner_type.get_full_name()
                            ),
                            Some(line_info),
                        ));
                    };

                    let property_type = XamlIlFerroPropertyHelper::get_ferro_property_type(
                        &*ferro_property_field,
                        &types,
                        line_info,
                    )?;
                    nodes.push(XamlIlBindingPathElementNode::FerroProperty(Rc::new(
                        XamlIlFerroPropertyPropertyPathElementNode {
                            field: ferro_property_field,
                            type_: property_type,
                            accepts_null: *accepts_null,
                        },
                    )));
                }
                BindingExpressionNode::Grammar(Node::SelfNode) => {
                    nodes.push(XamlIlBindingPathElementNode::SelfElement(Rc::new(
                        SelfPathElementNode {
                            type_: self_type.clone(),
                        },
                    )));
                }
                BindingExpressionNode::VisualAncestor(visual_ancestor) => {
                    nodes.push(XamlIlBindingPathElementNode::FindVisualAncestor(Rc::new(
                        FindVisualAncestorPathElementNode {
                            type_: visual_ancestor.type_.clone(),
                            level: visual_ancestor.level,
                        },
                    )));
                }
                BindingExpressionNode::TemplatedParent(_) => {
                    let templated_parent_type = context
                        .parent_nodes()
                        .into_iter()
                        .filter_map(|n| n.cast::<FerroXamlIlTargetTypeMetadataNode>())
                        .find(|x| x.scope_type == ScopeTypes::ControlTemplate)
                        .map(|x| x.target_type());

                    let Some(templated_parent_type) = templated_parent_type else {
                        return Err(XamlError::transform_exception(
                            "A binding with a TemplatedParent RelativeSource has to be in a ControlTemplate.",
                            Some(line_info),
                        ));
                    };

                    nodes.push(XamlIlBindingPathElementNode::TemplatedParent(Rc::new(
                        TemplatedParentPathElementNode {
                            type_: templated_parent_type.get_clr_type()?,
                        },
                    )));
                }
                BindingExpressionNode::Grammar(Node::Ancestor {
                    namespace,
                    type_name,
                    level,
                }) => {
                    let styled_element = &types.styled_element;
                    let ancestor_type_filter = if !(namespace.is_none() && type_name.is_none()) {
                        Some(get_type(namespace.as_deref(), type_name.as_deref())?)
                    } else {
                        None
                    };

                    let parent_nodes = context.parent_nodes();
                    let mut ancestor_node: Option<Rc<XamlAstConstructableObjectNode>> = None;
                    {
                        let mut styled_elements_seen = 0usize;
                        let mut matching_seen = 0i64;
                        for x in parent_nodes
                            .iter()
                            .filter_map(|n| n.cast::<XamlAstConstructableObjectNode>())
                        {
                            let x_type = x.type_().get_clr_type()?;
                            if !styled_element.is_assignable_from(&*x_type) {
                                continue;
                            }
                            // `.Skip(1)`
                            styled_elements_seen += 1;
                            if styled_elements_seen == 1 {
                                continue;
                            }
                            if let Some(filter) = &ancestor_type_filter {
                                if !filter.is_assignable_from(&*x_type) {
                                    continue;
                                }
                            }
                            // `.ElementAtOrDefault(ancestor.Level)`
                            if matching_seen == i64::from(*level) {
                                ancestor_node = Some(x);
                                break;
                            }
                            matching_seen += 1;
                        }
                    }

                    let mut data_context_type: Option<Rc<dyn IXamlType>> = None;
                    if let Some(ancestor_node) = &ancestor_node {
                        let mut is_skipping = true;
                        for node in &parent_nodes {
                            if node.same_node(ancestor_node) {
                                is_skipping = false;
                            }
                            if node.is::<FerroNameScopeRegistrationXamlIlNode>() {
                                break;
                            }
                            if !is_skipping {
                                if let Some(metadata_node) =
                                    node.cast::<FerroXamlIlDataContextTypeMetadataNode>()
                                {
                                    data_context_type =
                                        Some(metadata_node.data_context_type.clone());
                                    break;
                                }
                            }
                        }
                    }

                    // We need actual ancestor for a correct DataContextType,
                    // but since in current design bindings do a double-work by enumerating the tree,
                    // we want to keep original ancestor type filter, if it was present.
                    let binding_ancestor_type = match ancestor_type_filter {
                        Some(filter) => Some(filter),
                        None => match &ancestor_node {
                            Some(ancestor_node) => Some(ancestor_node.type_().get_clr_type()?),
                            None => None,
                        },
                    };

                    let Some(binding_ancestor_type) = binding_ancestor_type else {
                        return Err(XamlError::transform_exception(
                            "Unable to resolve implicit ancestor type based on XAML tree.",
                            Some(line_info),
                        ));
                    };

                    nodes.push(XamlIlBindingPathElementNode::FindAncestor(Rc::new(
                        FindAncestorPathElementNode {
                            type_: binding_ancestor_type,
                            level: *level,
                            data_context_type,
                        },
                    )));
                }
                BindingExpressionNode::Grammar(Node::Name { name }) => {
                    let parent_nodes = context.parent_nodes();
                    let Some(namescope_root) = parent_nodes.last() else {
                        return Err(XamlError::invalid_operation(
                            "Sequence contains no elements",
                        ));
                    };
                    let mut found = ScopeRegistrationFinder::get_target_type(namescope_root, name)?;

                    for deferred_content in parent_nodes
                        .iter()
                        .filter(|n| n.is::<NestedScopeMetadataNode>())
                    {
                        if found.is_some() {
                            break;
                        }

                        found = ScopeRegistrationFinder::get_target_type(deferred_content, name)?;
                        if found.is_some() {
                            break;
                        }
                    }

                    let Some((element_type, data_type)) = found else {
                        return Err(XamlError::transform_exception(
                            format!(
                                "Unable to find element '{name}' in the current namescope. Unable to use a compiled binding with a name binding if the name cannot be found at compile time."
                            ),
                            Some(line_info),
                        ));
                    };
                    nodes.push(XamlIlBindingPathElementNode::ElementName(Rc::new(
                        ElementNamePathElementNode {
                            name: name.clone(),
                            type_: element_type,
                            data_context_type: data_type,
                        },
                    )));
                }
                BindingExpressionNode::Grammar(Node::TypeCast {
                    namespace,
                    type_name,
                }) => {
                    // Upstream also reports "Unable to resolve cast to type ..." for a null
                    // type; the resolution never returns null (it throws).
                    let cast_type = get_type(Some(namespace), Some(type_name))?;

                    nodes.push(XamlIlBindingPathElementNode::TypeCast(Rc::new(
                        TypeCastPathElementNode { type_: cast_type },
                    )));
                }
                // As upstream, the switch has no case for the logical ancestor node of a
                // long-form `RelativeSource` with `Tree=Logical`: it adds no element.
                BindingExpressionNode::LogicalAncestor(_) => {}
            }
        }

        Ok(XamlIlBindingPathNode::new(
            line_info,
            types.compiled_binding_path.clone(),
            transform_nodes,
            nodes,
        ))
    }
}

/// The local function `GetAllDefinedProperties`.
fn get_all_defined_properties(type_: &Rc<dyn IXamlType>) -> Vec<Rc<dyn IXamlProperty>> {
    traverse_type_hierarchy(type_)
        .into_iter()
        .flat_map(|t| t.properties())
        .collect()
}

/// The local function `GetAllDefinedMethods`.
fn get_all_defined_methods(type_: &Rc<dyn IXamlType>) -> Vec<Rc<dyn IXamlMethod>> {
    traverse_type_hierarchy(type_)
        .into_iter()
        .flat_map(|t| t.methods())
        .collect()
}

/// The local function `TraverseTypeHierarchy`: an interface followed by the interfaces it
/// extends (depth first), or a class followed by its base classes.
fn traverse_type_hierarchy(type_: &Rc<dyn IXamlType>) -> Vec<Rc<dyn IXamlType>> {
    let mut rv = Vec::new();
    if type_.is_interface() {
        rv.push(type_.clone());
        for iface in type_.interfaces() {
            rv.extend(traverse_type_hierarchy(&iface));
        }
    } else {
        let mut current_type = Some(type_.clone());
        while let Some(current) = current_type {
            current_type = current.base_type();
            rv.push(current);
        }
    }
    rv
}

/// The local function `GetBestCommandMethod`.
///
/// Priority:
///  1. One parameter method
///    1a. Object parameter (amongst several overloads)
///    1b. Single method with one parameter
///  2. Zero parameters method
fn get_best_command_method(
    candidates: &[Rc<dyn IXamlMethod>],
    name: &str,
    target_type: &Rc<dyn IXamlType>,
    line_info: &dyn IXamlLineInfo,
) -> XamlResult<Rc<dyn IXamlMethod>> {
    let mut zero_param_candidate: Option<Rc<dyn IXamlMethod>> = None;
    // `HashSet<IXamlMethod>(SingleParameterTypeXamlTypeComparer.Instance)`, in insertion order.
    let mut one_param_candidates: Vec<(Rc<dyn IXamlType>, Rc<dyn IXamlMethod>)> = Vec::new();

    for candidate in candidates {
        let parameters = candidate.parameters();
        match parameters.as_slice() {
            [] => {
                if zero_param_candidate.is_none() {
                    zero_param_candidate = Some(candidate.clone());
                }
            }
            [parameter] => {
                // Object parameter always wins
                if parameter.is("System", "Object") {
                    return Ok(candidate.clone());
                }

                // Our candidates are ordered with the most derived class first:
                // By adding the first candidate for a given parameter type, we automatically handle overridden or hidden methods.
                if !one_param_candidates
                    .iter()
                    .any(|(existing, _)| existing.equals(&**parameter))
                {
                    one_param_candidates.push((parameter.clone(), candidate.clone()));
                }
            }
            _ => {}
        }
    }

    if !one_param_candidates.is_empty() {
        if one_param_candidates.len() == 1 {
            return Ok(one_param_candidates.remove(0).1);
        }

        let mut parameter_types: Vec<String> = one_param_candidates
            .iter()
            .map(|(parameter, _)| format!("'{}'", parameter.full_name()))
            .collect();
        parameter_types.sort();

        return Err(XamlError::transform_exception(
            format!(
                "Unable to resolve method of name '{name}' on type '{}'. Found {} overloads accepting one parameter: {}. Expected either a single overload with one parameter, or an overload accepting System.Object.",
                target_type.to_type_string(),
                parameter_types.len(),
                parameter_types.join(", ")
            ),
            Some(line_info),
        ));
    }

    zero_param_candidate.ok_or_else(|| {
        XamlError::transform_exception(
            format!(
                "Unable to resolve method of name '{name}' on type '{}'. Found {} overloads accepting more than one parameter. Expected a method with zero or one parameter.",
                target_type.to_type_string(),
                candidates.len()
            ),
            Some(line_info),
        )
    })
}

/// Finds the registration of a name in one name scope: the type of the named element and the
/// data context type in scope where it is registered.
struct ScopeRegistrationFinder {
    stack: Vec<Rc<dyn IXamlAstNode>>,
    child_scopes_stack: Vec<Rc<dyn IXamlAstNode>>,
    name: String,
    target_type: Option<Rc<dyn IXamlType>>,
    data_context_type: Option<Rc<dyn IXamlType>>,
}

impl ScopeRegistrationFinder {
    fn get_target_type(
        namescope_root: &Rc<dyn IXamlAstNode>,
        name: &str,
    ) -> XamlResult<Option<(Rc<dyn IXamlType>, Option<Rc<dyn IXamlType>>)>> {
        // If we start from the nested scope - skip it.
        let namescope_root = match namescope_root.cast::<NestedScopeMetadataNode>() {
            Some(scope) => scope.value().as_node(),
            None => namescope_root.clone(),
        };

        let mut finder = ScopeRegistrationFinder {
            stack: Vec::new(),
            child_scopes_stack: Vec::new(),
            name: name.to_string(),
            target_type: None,
            data_context_type: None,
        };
        visit_node(&namescope_root, &mut finder)?;
        Ok(finder
            .target_type
            .map(|target_type| (target_type, finder.data_context_type)))
    }
}

impl IXamlAstVisitor for ScopeRegistrationFinder {
    fn pop(&mut self) {
        if let Some(node) = self.stack.pop() {
            if self
                .child_scopes_stack
                .last()
                .is_some_and(|scope| scope.same_node(&node))
            {
                self.child_scopes_stack.pop();
            }
        }
    }

    fn push(&mut self, node: Rc<dyn IXamlAstNode>) {
        if node.is::<NestedScopeMetadataNode>() {
            self.child_scopes_stack.push(node.clone());
        }
        self.stack.push(node);
    }

    fn visit(&mut self, node: Rc<dyn IXamlAstNode>) -> XamlResult<Rc<dyn IXamlAstNode>> {
        // Ignore name registrations, if we are inside of the nested namescope.
        if self.child_scopes_stack.is_empty() && self.target_type.is_none() {
            if let Some(registration) = node.cast::<FerroNameScopeRegistrationXamlIlNode>() {
                if registration
                    .name()
                    .cast::<XamlAstTextNode>()
                    .is_some_and(|text| text.text() == self.name)
                {
                    self.target_type = registration.target_type.clone();
                    self.data_context_type = self
                        .stack
                        .iter()
                        .rev()
                        .find_map(|n| n.cast::<FerroXamlIlDataContextTypeMetadataNode>())
                        .map(|n| n.data_context_type.clone());
                }
            }
        }
        Ok(node)
    }
}

/// `IXamlIlBindingPathElementNode`: one resolved element of a compiled binding path.
///
/// Upstream every element class implements `Emit(context, codeGen)`, which is called with the
/// `CompiledBindingPathBuilder` on top of the evaluation stack and leaves the builder there
/// (every builder method returns the builder). The doc comment of each element type states the
/// builder call it emits.
#[derive(Clone)]
pub enum XamlIlBindingPathElementNode {
    Not(Rc<XamlIlNotPathElementNode>),
    StreamObservable(Rc<XamlIlStreamObservablePathElementNode>),
    StreamTask(Rc<XamlIlStreamTaskPathElementNode>),
    SelfElement(Rc<SelfPathElementNode>),
    FindAncestor(Rc<FindAncestorPathElementNode>),
    FindVisualAncestor(Rc<FindVisualAncestorPathElementNode>),
    ElementName(Rc<ElementNamePathElementNode>),
    TemplatedParent(Rc<TemplatedParentPathElementNode>),
    FerroProperty(Rc<XamlIlFerroPropertyPropertyPathElementNode>),
    ClrProperty(Rc<XamlIlClrPropertyPathElementNode>),
    ClrMethod(Rc<XamlIlClrMethodPathElementNode>),
    ClrMethodAsCommand(Rc<XamlIlClrMethodAsCommandPathElementNode>),
    ClrIndexer(Rc<XamlIlClrIndexerPathElementNode>),
    ArrayIndexer(Rc<XamlIlArrayIndexerPathElementNode>),
    TypeCast(Rc<TypeCastPathElementNode>),
}

impl XamlIlBindingPathElementNode {
    /// `IXamlIlBindingPathElementNode.Type`: the type of the value this element produces.
    pub fn type_(&self) -> Rc<dyn IXamlType> {
        match self {
            XamlIlBindingPathElementNode::Not(e) => e.type_.clone(),
            XamlIlBindingPathElementNode::StreamObservable(e) => e.type_.clone(),
            XamlIlBindingPathElementNode::StreamTask(e) => e.type_.clone(),
            XamlIlBindingPathElementNode::SelfElement(e) => e.type_.clone(),
            XamlIlBindingPathElementNode::FindAncestor(e) => e.type_.clone(),
            XamlIlBindingPathElementNode::FindVisualAncestor(e) => e.type_.clone(),
            XamlIlBindingPathElementNode::ElementName(e) => e.type_.clone(),
            XamlIlBindingPathElementNode::TemplatedParent(e) => e.type_.clone(),
            XamlIlBindingPathElementNode::FerroProperty(e) => e.type_.clone(),
            XamlIlBindingPathElementNode::ClrProperty(e) => e.type_(),
            XamlIlBindingPathElementNode::ClrMethod(e) => e.type_.clone(),
            XamlIlBindingPathElementNode::ClrMethodAsCommand(e) => e.type_.clone(),
            XamlIlBindingPathElementNode::ClrIndexer(e) => e.type_(),
            XamlIlBindingPathElementNode::ArrayIndexer(e) => e.type_(),
            XamlIlBindingPathElementNode::TypeCast(e) => e.type_.clone(),
        }
    }

    /// `(element as IXamlIlBindingPathNodeWithDataContextType)?.DataContextType`: the data
    /// context type of the element this path element selects, when the element is a source
    /// element that knows it (an ancestor or a named element).
    pub fn data_context_type(&self) -> Option<Rc<dyn IXamlType>> {
        match self {
            XamlIlBindingPathElementNode::FindAncestor(e) => e.data_context_type.clone(),
            XamlIlBindingPathElementNode::ElementName(e) => e.data_context_type.clone(),
            _ => None,
        }
    }

    /// `GetType().Name`.
    pub fn type_name(&self) -> &'static str {
        match self {
            XamlIlBindingPathElementNode::Not(_) => "XamlIlNotPathElementNode",
            XamlIlBindingPathElementNode::StreamObservable(_) => {
                "XamlIlStreamObservablePathElementNode"
            }
            XamlIlBindingPathElementNode::StreamTask(_) => "XamlIlStreamTaskPathElementNode",
            XamlIlBindingPathElementNode::SelfElement(_) => "SelfPathElementNode",
            XamlIlBindingPathElementNode::FindAncestor(_) => "FindAncestorPathElementNode",
            XamlIlBindingPathElementNode::FindVisualAncestor(_) => {
                "FindVisualAncestorPathElementNode"
            }
            XamlIlBindingPathElementNode::ElementName(_) => "ElementNamePathElementNode",
            XamlIlBindingPathElementNode::TemplatedParent(_) => "TemplatedParentPathElementNode",
            XamlIlBindingPathElementNode::FerroProperty(_) => {
                "XamlIlFerroPropertyPropertyPathElementNode"
            }
            XamlIlBindingPathElementNode::ClrProperty(_) => "XamlIlClrPropertyPathElementNode",
            XamlIlBindingPathElementNode::ClrMethod(_) => "XamlIlClrMethodPathElementNode",
            XamlIlBindingPathElementNode::ClrMethodAsCommand(_) => {
                "XamlIlClrMethodAsCommandPathElementNode"
            }
            XamlIlBindingPathElementNode::ClrIndexer(_) => "XamlIlClrIndexerPathElementNode",
            XamlIlBindingPathElementNode::ArrayIndexer(_) => "XamlIlArrayIndexerPathElementNode",
            XamlIlBindingPathElementNode::TypeCast(_) => "TypeCastPathElementNode",
        }
    }
}

/// `!`: negates the value of the rest of the path. The only transform element.
///
/// Emits `builder.Not()` (no arguments). Rust: `CompiledBindingPathBuilder::not()`.
pub struct XamlIlNotPathElementNode {
    /// `System.Boolean`.
    pub type_: Rc<dyn IXamlType>,
}

/// `^` on an `IObservable<T>`: the values the observable produces.
///
/// Emits `builder.StreamObservable<T>()` with `T` = [`type_`](Self::type_) (no arguments).
/// Rust: `CompiledBindingPathBuilder::stream_observable()`.
pub struct XamlIlStreamObservablePathElementNode {
    /// `T`: the type argument of the `IObservable<T>` the previous element produces.
    pub type_: Rc<dyn IXamlType>,
}

/// `^` on a `Task<T>`: the result of the task once it completes.
///
/// Emits `builder.StreamTask<T>()` with `T` = [`type_`](Self::type_) (no arguments).
/// Rust: `CompiledBindingPathBuilder::stream_task()`.
pub struct XamlIlStreamTaskPathElementNode {
    /// `T`: the type argument of the `Task<T>` the previous element produces (or derives from).
    pub type_: Rc<dyn IXamlType>,
}

/// `$self`: the binding target itself.
///
/// Emits `builder.Self()` (no arguments). Rust: `CompiledBindingPathBuilder::self_()`.
pub struct SelfPathElementNode {
    /// The type of the object the binding is set on (for a setter: the style's target type).
    pub type_: Rc<dyn IXamlType>,
}

/// `$parent[Type; level]`: an ancestor in the logical tree.
///
/// Emits `builder.Ancestor(typeof(Type), level)`: push the `System.Type` of
/// [`type_`](Self::type_) (`ldtoken` + `Type.GetTypeFromHandle`), push the `int`
/// [`level`](Self::level), call. Rust: `CompiledBindingPathBuilder::ancestor(Some(type), level)`.
pub struct FindAncestorPathElementNode {
    /// The ancestor type: the type filter of the path when it has one, else the type of the
    /// ancestor element found in the document.
    pub type_: Rc<dyn IXamlType>,
    /// `_level`: how many matching ancestors to skip (0 = the nearest).
    pub level: i32,
    /// The data context type in scope at the ancestor element found in the document, used to
    /// type a following `DataContext` element.
    pub data_context_type: Option<Rc<dyn IXamlType>>,
}

/// `RelativeSource` `FindAncestor` in the visual tree.
///
/// Emits `builder.VisualAncestor(typeof(Type), level)`: push the `System.Type` of
/// [`type_`](Self::type_), push the `int` [`level`](Self::level), call.
/// Rust: `CompiledBindingPathBuilder::visual_ancestor(Some(type), level)`.
pub struct FindVisualAncestorPathElementNode {
    pub type_: Rc<dyn IXamlType>,
    /// `_level`.
    pub level: i32,
}

/// `#name`: the element registered under a name in the name scope of the document (or of the
/// enclosing template).
///
/// Emits `builder.ElementName(nameScope, name)`: load the runtime context local, load its name
/// scope field (`FerroXamlIlLanguage::CONTEXT_NAME_SCOPE_FIELD_NAME`, an `INameScope`), push the
/// string [`name`](Self::name), call. The emitting context therefore needs its context local.
/// Rust: `CompiledBindingPathBuilder::element_name(name_scope, name)`.
pub struct ElementNamePathElementNode {
    /// `_name`.
    pub name: String,
    /// The type of the named element.
    pub type_: Rc<dyn IXamlType>,
    /// The data context type in scope where the element is registered, used to type a
    /// following `DataContext` element.
    pub data_context_type: Option<Rc<dyn IXamlType>>,
}

/// `$templatedParent` / `RelativeSource TemplatedParent`: the control a control template is
/// applied to.
///
/// Emits `builder.TemplatedParent()` (no arguments).
/// Rust: `CompiledBindingPathBuilder::templated_parent()`.
pub struct TemplatedParentPathElementNode {
    /// The target type of the enclosing control template.
    pub type_: Rc<dyn IXamlType>,
}

/// A registered property, by name on the previous element's type or as an attached property
/// `(Owner.Property)`.
///
/// Emits, in order:
/// 1. load the static field [`field`](Self::field) (`ldsfld`): the registered property;
/// 2. load the accessor factory
///    [`XamlIlPropertyAccessorFactory::FerroProperty`]
///    (`XamlIlPropertyInfoAccessorFactoryEmitter::emit_load_ferro_property_accessor_factory`);
/// 3. when [`accepts_null`](Self::accepts_null) (`?.`): push `true` and call the 3-parameter
///    `builder.Property(property, accessorFactory, acceptsNull)`; otherwise call the
///    2-parameter `builder.Property(property, accessorFactory)`.
///
/// Rust: `CompiledBindingPathBuilder::ferro_property(property)` /
/// `ferro_property_with(property, true)` (no accessor factory is needed).
pub struct XamlIlFerroPropertyPropertyPathElementNode {
    /// `_field`: the public static `<Name>Property` field.
    pub field: Rc<dyn IXamlField>,
    /// The value type of the registered property; for `StyledElement.DataContextProperty`
    /// after a source element with a known data context type, that data context type.
    pub type_: Rc<dyn IXamlType>,
    /// `_acceptsNull`.
    pub accepts_null: bool,
}

impl XamlIlFerroPropertyPropertyPathElementNode {
    /// The accessor factory step 2 loads.
    pub fn accessor_factory(&self) -> XamlIlPropertyAccessorFactory {
        XamlIlPropertyInfoAccessorFactoryEmitter::new().emit_load_ferro_property_accessor_factory()
    }
}

/// A plain (CLR) property of the previous element's type.
///
/// # Untyped emission ([`emit_typed`](Self::emit_typed) is `false`)
///
/// 1. load the property info of [`property`](Self::property):
///    `XamlIlClrPropertyInfoEmitter::emit(configuration, property, None, None)`
///    ([`Self::property_info`]);
/// 2. load the accessor factory [`XamlIlPropertyAccessorFactory::Inpc`];
/// 3. when [`accepts_null`](Self::accepts_null): push `true` and call the 3-parameter
///    `builder.Property(info, accessorFactory, acceptsNull)`; otherwise the 2-parameter
///    `builder.Property(info, accessorFactory)`.
///
/// Rust: `CompiledBindingPathBuilder::property(info, factory)` /
/// `property_with(info, factory, accepts_null)`, or the shortcuts `notifying_property` /
/// `notifying_read_only_property`.
///
/// # Typed emission ([`emit_typed`](Self::emit_typed) is `true`)
///
/// Enabled by [`XamlIlBindingPathNode::try_enable_typed_emission`] when the path is this single
/// element. `TSource` is the declaring type of the getter, `TValue` the property type.
///
/// 1. load the typed property info:
///    `XamlIlClrPropertyInfoEmitter::emit_typed(configuration, property)`
///    ([`Self::typed_property_info`]);
/// 2. load the accessor factory [`XamlIlPropertyAccessorFactory::Inpc`];
/// 3. push the boolean [`accepts_null`](Self::accepts_null);
/// 4. call the generic 3-parameter
///    `builder.Property<TSource, TValue>(info, accessorFactory, acceptsNull)`.
///
/// Rust: `CompiledBindingPathBuilder::typed_property(name, get, set)` /
/// `typed_property_info(info)`.
pub struct XamlIlClrPropertyPathElementNode {
    /// `_property` / `Property`.
    pub property: Rc<dyn IXamlProperty>,
    /// `_acceptsNull`.
    pub accepts_null: bool,
    /// `EmitTyped { get; set; }`.
    pub emit_typed: Cell<bool>,
}

impl XamlIlClrPropertyPathElementNode {
    /// `Type => _property.PropertyType`.
    pub fn type_(&self) -> Rc<dyn IXamlType> {
        self.property.property_type()
    }

    /// The property info step 1 of the untyped emission loads.
    pub fn property_info(
        &self,
        configuration: &TransformerConfiguration,
    ) -> XamlResult<Rc<XamlIlClrPropertyInfo>> {
        configuration
            .get_or_create_extra::<XamlIlClrPropertyInfoEmitter>()
            .emit(configuration, &self.property, None, None)
    }

    /// The property info step 1 of the typed emission loads.
    pub fn typed_property_info(
        &self,
        configuration: &TransformerConfiguration,
    ) -> XamlResult<Rc<XamlIlTypedClrPropertyInfo>> {
        configuration
            .get_or_create_extra::<XamlIlClrPropertyInfoEmitter>()
            .emit_typed(configuration, &self.property)
    }

    /// The accessor factory step 2 loads (both emissions).
    pub fn accessor_factory(&self) -> XamlIlPropertyAccessorFactory {
        XamlIlPropertyInfoAccessorFactoryEmitter::new().emit_load_inpc_property_accessor_factory()
    }
}

/// The delegate type a method element is bound as.
#[derive(Clone)]
pub enum XamlIlMethodDelegateType {
    /// `System.Action`, a constructed `System.Action<...>` or a constructed
    /// `System.Func<..., TResult>`.
    Existing(Rc<dyn IXamlType>),
    /// The method has more than 16 parameters: upstream defines a private delegate type nested
    /// in the type being compiled, named with a generated identifier part, with this signature.
    Custom {
        name: String,
        return_type: Rc<dyn IXamlType>,
        parameters: Vec<Rc<dyn IXamlType>>,
    },
}

/// A method of the previous element's type, bound as a delegate (the element type is
/// `System.Delegate`). When the binding targets an `ICommand` property this element is replaced
/// with [`XamlIlClrMethodAsCommandPathElementNode`].
///
/// Emits, in order:
/// 1. push the runtime method handle of [`method`](Self::method) (`ldtoken method`);
/// 2. push the runtime type handle of the delegate type ([`Self::specific_delegate_type`]);
/// 3. when [`accepts_null`](Self::accepts_null): push `true` and call the 3-parameter
///    `builder.Method(methodHandle, delegateTypeHandle, acceptsNull)`; otherwise the 2-parameter
///    `builder.Method(methodHandle, delegateTypeHandle)`.
///
/// At run time the builder creates, per source object, a delegate of that type bound to the
/// source. Rust: the builder has no counterpart of `Method` at the time of this port; a back
/// end has to supply a closure over the method bound to the source object.
pub struct XamlIlClrMethodPathElementNode {
    /// `Method`.
    pub method: Rc<dyn IXamlMethod>,
    /// `System.Delegate`.
    pub type_: Rc<dyn IXamlType>,
    /// `_acceptsNull`.
    pub accepts_null: bool,
}

impl XamlIlClrMethodPathElementNode {
    /// The delegate type selection of `Emit`:
    /// `void M()` is `Action`; `void M(P1..Pn)` with `n <= 16` is `Action<P1..Pn>`; any other
    /// method with at most 16 parameters is `Func<P1..Pn, TReturn>`; a method with more
    /// parameters needs a custom delegate type.
    pub fn specific_delegate_type(
        &self,
        configuration: &TransformerConfiguration,
    ) -> XamlResult<XamlIlMethodDelegateType> {
        let well_known_types = configuration.well_known_types();
        let return_type = self.method.return_type();
        let parameters = self.method.parameters();
        let returns_void = return_type.equals(&*well_known_types.void);
        Ok(if returns_void && parameters.is_empty() {
            XamlIlMethodDelegateType::Existing(well_known_types.action.clone())
        } else if returns_void && parameters.len() <= 16 {
            XamlIlMethodDelegateType::Existing(
                well_known_types
                    .get_action_of_t(parameters.len())
                    .make_generic_type(&parameters)?,
            )
        } else if parameters.len() <= 16 {
            let mut generic_parameters = parameters.clone();
            generic_parameters.push(return_type);
            XamlIlMethodDelegateType::Existing(
                well_known_types
                    .get_func_of_t(parameters.len() + 1)
                    .make_generic_type(&generic_parameters)?,
            )
        } else {
            // In this case, we need to emit our own delegate type.
            XamlIlMethodDelegateType::Custom {
                name: configuration.identifier_generator.generate_identifier_part(),
                return_type,
                parameters,
            }
        })
    }
}

/// A method used as a command: the last element of a path bound to an `ICommand` property.
///
/// Emits, in order:
/// 1. push the string `execute_method.Name`;
/// 2. push an `Action<object, object>` over the execute trampoline
///    (`XamlIlTrampolineBuilder::emit_command_execute_trampoline`, [`Self::execute_trampoline`]):
///    `ldnull; ldftn trampoline; newobj Action<object, object>(object, IntPtr)`;
/// 3. push the can-execute delegate: null without a `Can<Name>(object)` method, otherwise a
///    `Func<object, object, bool>` over the can-execute trampoline
///    (`XamlIlTrampolineBuilder::emit_command_can_execute_trampoline`,
///    [`Self::can_execute_trampoline`]);
/// 4. push the names of the properties the command state depends on: null when
///    [`depends_on_properties`](Self::depends_on_properties) is empty, otherwise a new
///    `string[]` filled with them in order;
/// 5. call `builder.Command(methodName, execute, canExecute, dependsOnProperties)`.
///
/// Rust: `CompiledBindingPathBuilder::command::<Owner>(name, execute, can_execute, depends_on)`
/// or `notifying_command` when the owner raises property change notifications.
pub struct XamlIlClrMethodAsCommandPathElementNode {
    /// `System.Windows.Input.ICommand`.
    pub type_: Rc<dyn IXamlType>,
    /// `_executeMethod`: an instance method with zero or one parameter.
    pub execute_method: Rc<dyn IXamlMethod>,
    /// `_canExecuteMethod`: `bool Can<Name>(object)` on the declaring type of the execute
    /// method (or a base type), when there is one.
    pub can_execute_method: Option<Rc<dyn IXamlMethod>>,
    /// `_dependsOnProperties`: the arguments of the `DependsOn` attributes of the can-execute
    /// method, in declaration order.
    pub depends_on_properties: Vec<String>,
}

impl XamlIlClrMethodAsCommandPathElementNode {
    /// The trampoline step 2 wraps.
    pub fn execute_trampoline(
        &self,
        configuration: &TransformerConfiguration,
    ) -> Rc<XamlIlCommandExecuteTrampoline> {
        configuration
            .get_or_create_extra::<XamlIlTrampolineBuilder>()
            .emit_command_execute_trampoline(&self.execute_method)
    }

    /// The trampoline step 3 wraps, when there is a can-execute method.
    pub fn can_execute_trampoline(
        &self,
        configuration: &TransformerConfiguration,
    ) -> Option<Rc<XamlIlCommandCanExecuteTrampoline>> {
        self.can_execute_method.as_ref().map(|method| {
            configuration
                .get_or_create_extra::<XamlIlTrampolineBuilder>()
                .emit_command_can_execute_trampoline(method)
        })
    }
}

/// An indexer (`[a, b]`) on a type with a default member (`DefaultMemberAttribute`).
///
/// Emits, in order:
/// 1. load the property info of the indexer property applied to the constant arguments:
///    `XamlIlClrPropertyInfoEmitter::emit(configuration, property, Some(values), Some(indexer_key))`
///    ([`Self::property_info`]); its getter and setter pass [`values`](Self::values) as the
///    indexer arguments;
/// 2. load the accessor factory ([`Self::accessor_factory`]):
///    [`XamlIlPropertyAccessorFactory::Indexer`] over the single argument when the collection
///    raises collection change notifications and the indexer takes exactly one `System.Int32`
///    argument, [`XamlIlPropertyAccessorFactory::Inpc`] otherwise;
/// 3. call `builder.Property(info, accessorFactory)`. As upstream, the null-conditional
///    operator is not applied to indexers.
///
/// Rust: `CompiledBindingPathBuilder::property(info, factory)` with a property info named
/// `ferroui_base::data::core::INDEXER_NAME`, or the shortcuts `indexer_property`, `list_item`
/// and `dictionary_item`.
pub struct XamlIlClrIndexerPathElementNode {
    /// `_property`: the indexer property (the default member of the type).
    pub property: Rc<dyn IXamlProperty>,
    /// `_values`: the path's arguments converted to the indexer's parameter types.
    pub values: Vec<Rc<dyn IXamlAstValueNode>>,
    /// `_indexerKey`: the arguments as written, joined with `,`.
    pub indexer_key: String,
    /// `_isNotifyingCollection`: the indexed type implements `INotifyCollectionChanged`.
    pub is_notifying_collection: bool,
}

impl XamlIlClrIndexerPathElementNode {
    /// `Type => _property.PropertyType`.
    pub fn type_(&self) -> Rc<dyn IXamlType> {
        self.property.property_type()
    }

    /// The property info step 1 loads.
    pub fn property_info(
        &self,
        configuration: &TransformerConfiguration,
    ) -> XamlResult<Rc<XamlIlClrPropertyInfo>> {
        configuration
            .get_or_create_extra::<XamlIlClrPropertyInfoEmitter>()
            .emit(
                configuration,
                &self.property,
                Some(&self.values),
                Some(&self.indexer_key),
            )
    }

    /// The accessor factory step 2 loads.
    pub fn accessor_factory(
        &self,
        configuration: &TransformerConfiguration,
    ) -> XamlResult<XamlIlPropertyAccessorFactory> {
        let int_type = configuration.type_system.well_known_types().int32.clone();
        let emitter = XamlIlPropertyInfoAccessorFactoryEmitter::new();
        if self.is_notifying_collection
            && self.values.len() == 1
            && self.values[0].type_().get_clr_type()?.equals(&*int_type)
        {
            Ok(emitter.emit_load_indexer_accessor_factory(&self.values[0]))
        } else {
            Ok(emitter.emit_load_inpc_property_accessor_factory())
        }
    }
}

/// An index into an array (`[i]`, `[i, j]`).
///
/// Emits, in order:
/// 1. create an `int[]` with [`values`](Self::values)`.len()` elements in a local and store
///    each index into it;
/// 2. push the array, push the `System.Type` of the element type ([`Self::type_`]);
/// 3. call `builder.ArrayElement(indices, elementType)`.
///
/// Rust: `CompiledBindingPathBuilder::array_element(&indices)`.
pub struct XamlIlArrayIndexerPathElementNode {
    /// `_arrayType`.
    pub array_type: Rc<dyn IXamlType>,
    /// `_values`: one index per dimension.
    pub values: Vec<i32>,
}

impl XamlIlArrayIndexerPathElementNode {
    /// The constructor: converts the path's arguments to integers.
    pub fn new(
        array_type: Rc<dyn IXamlType>,
        values: &[String],
        line_info: &dyn IXamlLineInfo,
    ) -> XamlResult<Self> {
        let mut indices = Vec::with_capacity(values.len());
        for item in values {
            // `int.TryParse(item, out var index)`
            let Ok(index) = item.trim().parse::<i32>() else {
                return Err(XamlError::transform_exception(
                    format!("Unable to convert '{item}' to an integer."),
                    Some(line_info),
                ));
            };
            indices.push(index);
        }
        Ok(Self {
            array_type,
            values: indices,
        })
    }

    /// `Type => _arrayType.ArrayElementType!`.
    pub fn type_(&self) -> Rc<dyn IXamlType> {
        self.array_type
            .array_element_type()
            .unwrap_or_else(XamlPseudoType::unknown)
    }
}

/// A cast (`(Type)` / `((ns:Type)Member)`): the value when it is of the type, null otherwise.
///
/// Emits `builder.TypeCast<T>()` with `T` = [`type_`](Self::type_) (no arguments).
/// Rust: `CompiledBindingPathBuilder::type_cast::<T>()` / `type_cast_value(target)`.
pub struct TypeCastPathElementNode {
    pub type_: Rc<dyn IXamlType>,
}

/// `IXamlIlBindingPathNode : IXamlAstValueNode`.
///
/// `node is IXamlIlBindingPathNode` is `node.cast::<dyn IXamlIlBindingPathNode>()`.
pub trait IXamlIlBindingPathNode: IXamlAstValueNode {
    /// The type of the value a binding with this path produces.
    fn binding_result_type(&self) -> Rc<dyn IXamlType>;
}

impl XamlAstCast for dyn IXamlIlBindingPathNode {
    fn kind_name() -> &'static str {
        "IXamlIlBindingPathNode"
    }
    fn cast_from(node: Rc<dyn IXamlAstNode>) -> Option<Rc<Self>> {
        query_node_interface::<dyn IXamlIlBindingPathNode>(&node)
    }
    fn upcast(this: Rc<Self>) -> Rc<dyn IXamlAstNode> {
        this
    }
}

/// The resolved path of a compiled binding: a value node of type `CompiledBindingPath`.
///
/// # What a back end has to do (upstream IL)
///
/// Stack effect: pushes one value, the built `CompiledBindingPath`. The emitting context needs
/// its locals (upstream the node is an `IXamlAstLocalsEmitableNode`): the element-name element
/// reads the runtime context local, the command and array elements use temporaries.
///
/// 1. Call [`XamlIlBindingPathNode::try_enable_typed_emission`].
/// 2. Create the builder: `new CompiledBindingPathBuilder()`
///    (Rust: `CompiledBindingPathBuilder::new()`).
/// 3. Emit every element of [`transform_elements`](Self::transform_elements), in order, then
///    every element of [`elements`](Self::elements), in order. Each element's emission is
///    documented on its type; it consumes the builder and leaves the builder.
/// 4. Call `builder.Build()` (Rust: `CompiledBindingPathBuilder::build()`).
pub struct XamlIlBindingPathNode {
    base: XamlAstNode,
    type_: Rc<dyn IXamlAstTypeReference>,
    /// `_transformElements`: the transform elements (`!`), outermost first.
    pub transform_elements: RefCell<Vec<XamlIlBindingPathElementNode>>,
    /// `Elements`: the source and member elements, in path order.
    pub elements: RefCell<Vec<XamlIlBindingPathElementNode>>,
}

impl XamlIlBindingPathNode {
    pub fn new(
        line_info: &dyn IXamlLineInfo,
        binding_path_type: Rc<dyn IXamlType>,
        transform_elements: Vec<XamlIlBindingPathElementNode>,
        elements: Vec<XamlIlBindingPathElementNode>,
    ) -> Rc<Self> {
        Rc::new(Self {
            base: XamlAstNode::new(line_info),
            type_: XamlAstClrTypeReference::new(line_info, binding_path_type, false),
            transform_elements: RefCell::new(transform_elements),
            elements: RefCell::new(elements),
        })
    }

    /// `BindingResultType`: the type of the first transform element, else of the last element,
    /// else the unknown pseudo type.
    pub fn binding_result_type(&self) -> Rc<dyn IXamlType> {
        if let Some(first) = self.transform_elements.borrow().first() {
            return first.type_();
        }
        if let Some(last) = self.elements.borrow().last() {
            return last.type_();
        }
        XamlPseudoType::unknown()
    }

    /// `TryEnableTypedEmission(transformElements, elements)`, the first step of `Emit`: a path
    /// that is a single CLR property with an instance getter declared by a reference type is
    /// emitted through the typed property overload, which lets the binding read and write
    /// values without boxing (`TypedBindingExpression<TSource, TValue>` requires
    /// `TSource : class`). Sets [`XamlIlClrPropertyPathElementNode::emit_typed`].
    pub fn try_enable_typed_emission(&self) {
        if !self.transform_elements.borrow().is_empty() {
            return;
        }
        let elements = self.elements.borrow();
        let [XamlIlBindingPathElementNode::ClrProperty(clr)] = elements.as_slice() else {
            return;
        };

        let property = &clr.property;
        let declaring_type = property
            .getter()
            .or_else(|| property.setter())
            .map(|accessor| accessor.declaring_type());

        // TypedBindingExpression<TSource, TValue> requires TSource : class.
        match declaring_type {
            None => return,
            Some(declaring_type) if declaring_type.is_value_type() => return,
            Some(_) => {}
        }

        // We need an instance getter so the source value can be read.
        match property.getter() {
            None => return,
            Some(getter) if getter.is_static() => return,
            Some(_) => {}
        }

        clr.emit_typed.set(true);
    }
}

xaml_line_info_impl!(XamlIlBindingPathNode, base);

impl IXamlAstNode for XamlIlBindingPathNode {
    xaml_ast_node_members!("XamlIlBindingPathNode", value);

    // `VisitChildren` upstream visits the elements that are AST nodes; no element is one.

    fn query_interface(self: Rc<Self>, slot: &mut dyn std::any::Any) -> bool {
        xaml_query_interface!(self, slot, dyn IXamlIlBindingPathNode)
    }
}

impl IXamlAstValueNode for XamlIlBindingPathNode {
    fn type_(&self) -> Rc<dyn IXamlAstTypeReference> {
        self.type_.clone()
    }
}

impl IXamlIlBindingPathNode for XamlIlBindingPathNode {
    fn binding_result_type(&self) -> Rc<dyn IXamlType> {
        XamlIlBindingPathNode::binding_result_type(self)
    }
}

#[cfg(test)]
#[path = "xaml_il_binding_path_helper_tests.rs"]
mod tests;
