//! Port of `CompilerExtensions/Transformers/FerroXamlIlBindingPathParser.cs`.

use std::cell::RefCell;
use std::rc::Rc;

use ferroui_base::data::core::parsers::{BindingExpressionGrammar, Node};
use xamlx::ast::{
    IXamlAstNode, IXamlAstTypeReference, IXamlAstValueNode, IXamlLineInfo, XamlAstClrProperty,
    XamlAstClrTypeReference, XamlAstExtensions, XamlAstNode, XamlAstNodeExtensions,
    XamlAstObjectNode, XamlAstPropertyReferenceExtensions, XamlAstTextNode,
    XamlAstXamlPropertyValueNode, XamlMarkupExtensionNode, XamlTypeExtensionNode,
};
use xamlx::exceptions::{XamlError, XamlResult};
use xamlx::transform::transformers::TypeReferenceResolver;
use xamlx::transform::{AstTransformationContext, IXamlAstTransformer};
use xamlx::type_system::IXamlType;
use xamlx::{xaml_ast_node_members, xaml_line_info_impl};

use super::{
    FerroSyntheticCompiledBindingProperty, FerroXamlIlTargetTypeMetadataNode,
    FerroXamlIlWellKnownTypesExtensions, ScopeTypes, SyntheticCompiledBindingPropertyName,
    XamlBindingsTransformException,
};

/// `BindingExpressionGrammar.INode`: a node of a parsed binding path.
///
/// Upstream the grammar nodes and the three node classes this file adds implement one
/// interface; the grammar of the binding engine produces a closed enum
/// ([`ferroui_base::data::core::parsers::Node`]), so the interface is this enum over the
/// grammar's nodes and the added ones.
#[derive(Clone)]
pub enum BindingExpressionNode {
    /// A node produced by `BindingExpressionGrammar`.
    Grammar(Node),
    VisualAncestor(VisualAncestorBindingExpressionNode),
    LogicalAncestor(LogicalAncestorBindingExpressionNode),
    TemplatedParent(TemplatedParentBindingExpressionNode),
}

impl BindingExpressionNode {
    /// `node is BindingExpressionGrammar.ITransformNode`.
    pub fn is_transform_node(&self) -> bool {
        matches!(self, BindingExpressionNode::Grammar(node) if node.is_transform_node())
    }

    /// `node is BindingExpressionGrammar.EmptyExpressionNode`.
    pub fn is_empty_expression_node(&self) -> bool {
        matches!(self, BindingExpressionNode::Grammar(Node::EmptyExpression))
    }
}

/// Parses the path of every compiled binding extension (its first constructor argument or its
/// `Path` property) and merges the long-form source properties (`ElementName`,
/// `RelativeSource`) into it, leaving a [`ParsedBindingPathNode`] in place of the path text.
pub struct FerroXamlIlBindingPathParser;

impl IXamlAstTransformer for FerroXamlIlBindingPathParser {
    fn transform(
        &self,
        context: &AstTransformationContext,
        node: Rc<dyn IXamlAstNode>,
    ) -> XamlResult<Rc<dyn IXamlAstNode>> {
        let Some(binding) = node.cast::<XamlAstObjectNode>() else {
            return Ok(node);
        };
        let types = context.try_get_ferro_types()?;
        if !binding
            .type_()
            .get_clr_type()?
            .equals(&*types.compiled_binding_extension)
        {
            return Ok(node);
        }

        let converted_node =
            Self::convert_long_form_properties_to_binding_expression_node(context, &binding)?;
        let mut found_path = false;

        let first_argument = binding.arguments.borrow().first().cloned();
        if let Some(binding_path_text) = first_argument.and_then(|a| a.cast::<XamlAstTextNode>())
        {
            let mut nodes = Self::get_binding_path(&binding_path_text)?;

            if let Some(converted_node) = &converted_node {
                let index = nodes.iter().take_while(|x| x.is_transform_node()).count();
                nodes.insert(index, converted_node.clone());
            }

            if nodes.len() == 1 && nodes[0].is_empty_expression_node() {
                binding.arguments.borrow_mut().remove(0);
            } else {
                let parsed: Rc<dyn IXamlAstValueNode> = ParsedBindingPathNode::new(
                    &*binding_path_text,
                    types.compiled_binding_path.clone(),
                    nodes,
                );
                binding.arguments.borrow_mut()[0] = parsed;
                found_path = true;
            }
        }

        if !found_path {
            let binding_path_assignment = Self::find_property_value(&binding, "Path")?;

            if let Some(binding_path_assignment) = binding_path_assignment {
                let first_value = binding_path_assignment.values.borrow().first().cloned();
                if let Some(path_value) = first_value.and_then(|v| v.cast::<XamlAstTextNode>()) {
                    let mut nodes = Self::get_binding_path(&path_value)?;

                    if nodes.len() == 1 && nodes[0].is_empty_expression_node() {
                        binding_path_assignment.values.borrow_mut().remove(0);
                    } else {
                        if let Some(converted_node) = &converted_node {
                            let index =
                                nodes.iter().take_while(|x| x.is_transform_node()).count();
                            nodes.insert(index, converted_node.clone());
                        }

                        let parsed: Rc<dyn IXamlAstValueNode> = ParsedBindingPathNode::new(
                            &*path_value,
                            types.compiled_binding_path.clone(),
                            nodes,
                        );
                        binding_path_assignment.values.borrow_mut()[0] = parsed;
                    }

                    found_path = true;
                }
            }
        }

        if !found_path {
            if let Some(converted_node) = converted_node {
                let nodes = vec![converted_node];
                let parsed: Rc<dyn IXamlAstValueNode> = ParsedBindingPathNode::new(
                    &*binding,
                    types.compiled_binding_path.clone(),
                    nodes,
                );
                binding.arguments.borrow_mut().push(parsed);
            }
        }

        Ok(node)
    }
}

impl FerroXamlIlBindingPathParser {
    fn get_binding_path(node: &Rc<XamlAstTextNode>) -> XamlResult<Vec<BindingExpressionNode>> {
        let text = node.text();
        match BindingExpressionGrammar::parse(&text) {
            Ok((nodes, _)) => Ok(nodes
                .into_iter()
                .map(BindingExpressionNode::Grammar)
                .collect()),
            Err(ex) => Err(XamlError::transform_exception_with_inner(
                format!("Failed to parse binding path '{text}': {}", ex.message()),
                Some(&**node),
                XamlError::internal("ExpressionParseException", ex.message()),
            )),
        }
    }

    /// `obj.Children.OfType<XamlAstXamlPropertyValueNode>()
    ///     .FirstOrDefault(x => x.Property.GetClrProperty().Name == name)`.
    fn find_property_value(
        obj: &XamlAstObjectNode,
        name: &str,
    ) -> XamlResult<Option<Rc<XamlAstXamlPropertyValueNode>>> {
        let property_values: Vec<Rc<XamlAstXamlPropertyValueNode>> = obj
            .children
            .borrow()
            .iter()
            .filter_map(|c| c.cast::<XamlAstXamlPropertyValueNode>())
            .collect();
        for value in property_values {
            if value.property().get_clr_property()?.name() == name {
                return Ok(Some(value));
            }
        }
        Ok(None)
    }

    /// `int.Parse(text)`.
    fn parse_int(text: &str) -> XamlResult<i32> {
        text.trim().parse::<i32>().map_err(|_| {
            XamlError::internal(
                "FormatException",
                format!("The input string '{text}' was not in a correct format."),
            )
        })
    }

    fn convert_long_form_properties_to_binding_expression_node(
        context: &AstTransformationContext,
        binding: &Rc<XamlAstObjectNode>,
    ) -> XamlResult<Option<BindingExpressionNode>> {
        let mut converted_node: Option<BindingExpressionNode> = None;

        let property_values: Vec<Rc<XamlAstXamlPropertyValueNode>> = binding
            .children
            .borrow()
            .iter()
            .filter_map(|c| c.cast::<XamlAstXamlPropertyValueNode>())
            .collect();

        let synthetic_name = |v: &Rc<XamlAstXamlPropertyValueNode>| {
            v.property()
                .cast::<FerroSyntheticCompiledBindingProperty>()
                .map(|p| p.name)
        };

        let element_name_property = property_values
            .iter()
            .find(|v| synthetic_name(v) == Some(SyntheticCompiledBindingPropertyName::ElementName))
            .cloned();

        let relative_source_property = property_values
            .iter()
            .find(|v| {
                synthetic_name(v) == Some(SyntheticCompiledBindingPropertyName::RelativeSource)
            })
            .cloned();

        let source_property = property_values
            .iter()
            .find(|v| {
                v.property()
                    .cast::<XamlAstClrProperty>()
                    .is_some_and(|prop| prop.name() == "Source")
            })
            .cloned();

        if let Some(element_name_property) = &element_name_property {
            let first_value = element_name_property.values.borrow().first().cloned();
            match first_value.as_ref().and_then(|v| v.cast::<XamlAstTextNode>()) {
                Some(element_name) => {
                    converted_node = Some(BindingExpressionNode::Grammar(Node::Name {
                        name: element_name.text(),
                    }));
                }
                None => {
                    let description = first_value
                        .as_ref()
                        .map(|v| v.to_node_string())
                        .unwrap_or_default();
                    let line_info: &dyn IXamlLineInfo = match &first_value {
                        Some(value) => &**value,
                        None => &**element_name_property,
                    };
                    return Err(XamlBindingsTransformException::new(
                        format!("Invalid ElementName '{description}'."),
                        line_info,
                        None,
                    ));
                }
            }
        }

        if source_property.is_some() && converted_node.is_some() {
            return Err(XamlBindingsTransformException::new(
                "Only one of ElementName, Source, or RelativeSource specified as a binding source. Only one property is allowed.",
                &**binding,
                None,
            ));
        }

        if let Some(relative_source_object) = Self::get_relative_source_object_from_assignment(
            context,
            relative_source_property.as_ref(),
        )? {
            if converted_node.is_some() {
                return Err(XamlBindingsTransformException::new(
                    "Only one of ElementName, Source, or RelativeSource specified as a binding source. Only one property is allowed.",
                    &**binding,
                    None,
                ));
            }

            let mode_property = Self::find_property_value(&relative_source_object, "Mode")?
                .and_then(|x| x.values.borrow().first().cloned())
                .and_then(|v| v.cast::<XamlAstTextNode>())
                .or_else(|| {
                    relative_source_object
                        .arguments
                        .borrow()
                        .iter()
                        .find_map(|a| a.cast::<XamlAstTextNode>())
                });

            let mut mode = mode_property.map(|m| m.text());
            if relative_source_object.arguments.borrow().is_empty() && mode.is_none() {
                mode = Some("FindAncestor".to_string());
            }

            match mode.as_deref() {
                Some("FindAncestor") => {
                    let ancestor_level = match Self::find_property_value(
                        &relative_source_object,
                        "FindAncestor",
                    )?
                    .and_then(|x| x.values.borrow().first().cloned())
                    .and_then(|v| v.cast::<XamlAstTextNode>())
                    {
                        Some(ancestor_level_text) => {
                            Self::parse_int(&ancestor_level_text.text())?.wrapping_sub(1)
                        }
                        None => 0,
                    };

                    let tree_type = match Self::find_property_value(&relative_source_object, "Tree")?
                        .and_then(|x| x.values.borrow().first().cloned())
                        .and_then(|v| v.cast::<XamlAstTextNode>())
                    {
                        Some(tree_type_value) => tree_type_value.text(),
                        None => "Visual".to_string(),
                    };

                    let ancestor_type_value =
                        Self::find_property_value(&relative_source_object, "AncestorType")?
                            .and_then(|x| x.values.borrow().first().cloned());
                    let mut ancestor_type: Option<Rc<dyn IXamlType>> = match ancestor_type_value {
                        None => None,
                        Some(value) => {
                            if let Some(text_node) = value.cast::<XamlAstTextNode>() {
                                Some(
                                    TypeReferenceResolver::resolve_type_by_xml_name(
                                        context,
                                        &text_node.text(),
                                        false,
                                        &*text_node,
                                        true,
                                    )?
                                    .type_
                                    .clone(),
                                )
                            } else if let Some(type_extension_node) =
                                value.cast::<XamlTypeExtensionNode>()
                            {
                                Some(type_extension_node.value().get_clr_type()?)
                            } else {
                                return Err(XamlBindingsTransformException::new(
                                    "Unsupported node for AncestorType property",
                                    &*relative_source_object,
                                    None,
                                ));
                            }
                        }
                    };

                    if ancestor_type.is_none() {
                        if tree_type == "Visual" {
                            return Err(XamlBindingsTransformException::new(
                                "AncestorType must be set for RelativeSourceMode.FindAncestor when searching the visual tree.",
                                &*relative_source_object,
                                None,
                            ));
                        } else if tree_type == "Logical" {
                            let types = types_of(context)?;
                            let styled_element_type = &types.styled_element;
                            let mut styled_parents = Vec::new();
                            for x in context
                                .parent_nodes()
                                .into_iter()
                                .filter_map(|n| n.cast::<XamlAstObjectNode>())
                            {
                                let parent_type = x.type_().get_clr_type()?;
                                if styled_element_type.is_assignable_from(&*parent_type) {
                                    styled_parents.push(parent_type);
                                }
                            }
                            ancestor_type = usize::try_from(ancestor_level)
                                .ok()
                                .and_then(|level| styled_parents.get(level).cloned());

                            if ancestor_type.is_none() {
                                return Err(XamlBindingsTransformException::new(
                                    "Unable to resolve implicit ancestor type based on XAML tree.",
                                    &*relative_source_object,
                                    None,
                                ));
                            }
                        }
                    }

                    match (tree_type.as_str(), ancestor_type) {
                        ("Visual", Some(ancestor_type)) => {
                            converted_node = Some(BindingExpressionNode::VisualAncestor(
                                VisualAncestorBindingExpressionNode {
                                    type_: ancestor_type,
                                    level: ancestor_level,
                                },
                            ));
                        }
                        ("Logical", Some(ancestor_type)) => {
                            converted_node = Some(BindingExpressionNode::LogicalAncestor(
                                LogicalAncestorBindingExpressionNode {
                                    type_: ancestor_type,
                                    level: ancestor_level,
                                },
                            ));
                        }
                        _ => {
                            return Err(XamlBindingsTransformException::new(
                                format!("Unknown tree type '{tree_type}'."),
                                &**binding,
                                None,
                            ));
                        }
                    }
                }
                Some("DataContext") => {
                    converted_node = None;
                }
                Some("Self") => {
                    converted_node = Some(BindingExpressionNode::Grammar(Node::SelfNode));
                }
                Some("TemplatedParent") => {
                    let content_template_node = context
                        .parent_nodes()
                        .into_iter()
                        .filter_map(|n| n.cast::<FerroXamlIlTargetTypeMetadataNode>())
                        .find(|x| x.scope_type == ScopeTypes::ControlTemplate);
                    let Some(content_template_node) = content_template_node else {
                        return Err(XamlBindingsTransformException::new(
                            "A binding with a TemplatedParent RelativeSource has to be in a ControlTemplate.",
                            &**binding,
                            None,
                        ));
                    };

                    // Upstream also reports "TargetType has to be set on ControlTemplate or it
                    // should be defined inside of a Style." for a null CLR type; `GetClrType`
                    // never returns null (it throws for a type reference that is not resolved).
                    let parent_type = content_template_node.target_type().get_clr_type()?;

                    converted_node = Some(BindingExpressionNode::TemplatedParent(
                        TemplatedParentBindingExpressionNode { type_: parent_type },
                    ));
                }
                _ => {
                    return Err(XamlBindingsTransformException::new(
                        format!(
                            "Unknown RelativeSource mode '{}'.",
                            mode.as_deref().unwrap_or_default()
                        ),
                        &**binding,
                        None,
                    ));
                }
            }
        }

        if let Some(element_name_property) = &element_name_property {
            remove_child(binding, element_name_property);
        }
        if let Some(relative_source_property) = &relative_source_property {
            remove_child(binding, relative_source_property);
        }

        Ok(converted_node)
    }

    /// `GetRelativeSourceObjectFromAssignment(context, relativeSourceProperty, out relativeSourceObject)`:
    /// `Ok(Some(object))` is `true`.
    fn get_relative_source_object_from_assignment(
        context: &AstTransformationContext,
        relative_source_property: Option<&Rc<XamlAstXamlPropertyValueNode>>,
    ) -> XamlResult<Option<Rc<XamlAstObjectNode>>> {
        let Some(relative_source_property) = relative_source_property else {
            return Ok(None);
        };
        let Some(value) = relative_source_property.values.borrow().first().cloned() else {
            return Ok(None);
        };
        let types = types_of(context)?;
        let relative_source = &types.relative_source;

        if let Some(me) = value.cast::<XamlMarkupExtensionNode>() {
            let me_type = me.type_().get_clr_type()?;
            if !me_type.equals(&**relative_source) {
                return Err(XamlBindingsTransformException::new(
                    format!(
                        "Expected an object of type 'FerroUI.Data.RelativeSource'. Found a object of type '{}'",
                        me_type.get_fqn()
                    ),
                    &*me,
                    None,
                ));
            }

            // `(XamlAstObjectNode)me.Value`
            let me_value = me.value();
            let Some(relative_source_object) = me_value.cast::<XamlAstObjectNode>() else {
                return Err(XamlError::invalid_cast(format!(
                    "Unable to cast object of type '{}' to type 'XamlAstObjectNode'.",
                    me_value.type_name()
                )));
            };
            return Ok(Some(relative_source_object));
        }

        if let Some(on) = value.cast::<XamlAstObjectNode>() {
            let on_type = on.type_().get_clr_type()?;
            if !on_type.equals(&**relative_source) {
                return Err(XamlBindingsTransformException::new(
                    format!(
                        "Expected an object of type 'FerroUI.Data.RelativeSource'. Found a object of type '{}'",
                        on_type.get_fqn()
                    ),
                    &*on,
                    None,
                ));
            }

            return Ok(Some(on));
        }

        Ok(None)
    }
}

fn types_of(
    context: &AstTransformationContext,
) -> XamlResult<Rc<super::FerroXamlIlWellKnownTypes>> {
    context.try_get_ferro_types()
}

/// `binding.Children.Remove(child)`.
fn remove_child(binding: &XamlAstObjectNode, child: &Rc<XamlAstXamlPropertyValueNode>) {
    let mut children = binding.children.borrow_mut();
    if let Some(index) = children.iter().position(|c| c.same_node(child)) {
        children.remove(index);
    }
}

/// The parsed, not yet resolved path of a compiled binding. It stands where the path text
/// stood (constructor argument or `Path` value) and is typed as `CompiledBindingPath` so that
/// the constructor and property resolution of the binding extension succeed;
/// `FerroXamlIlBindingPathTransformer` replaces it with the resolved path node.
pub struct ParsedBindingPathNode {
    base: XamlAstNode,
    type_: Rc<dyn IXamlAstTypeReference>,
    /// `Path`.
    pub path: RefCell<Vec<BindingExpressionNode>>,
}

impl ParsedBindingPathNode {
    pub fn new(
        line_info: &dyn IXamlLineInfo,
        compiled_binding_type: Rc<dyn IXamlType>,
        path: Vec<BindingExpressionNode>,
    ) -> Rc<Self> {
        Rc::new(Self {
            base: XamlAstNode::new(line_info),
            type_: XamlAstClrTypeReference::new(line_info, compiled_binding_type, false),
            path: RefCell::new(path),
        })
    }

    pub fn path(&self) -> Vec<BindingExpressionNode> {
        self.path.borrow().clone()
    }
}

xaml_line_info_impl!(ParsedBindingPathNode, base);

impl IXamlAstNode for ParsedBindingPathNode {
    xaml_ast_node_members!("ParsedBindingPathNode", value);

    // `VisitChildren` upstream visits the path nodes that are AST nodes; no path node is one.
}

impl IXamlAstValueNode for ParsedBindingPathNode {
    fn type_(&self) -> Rc<dyn IXamlAstTypeReference> {
        self.type_.clone()
    }
}

/// `RelativeSource` with `Mode=FindAncestor, Tree=Visual`.
#[derive(Clone)]
pub struct VisualAncestorBindingExpressionNode {
    pub type_: Rc<dyn IXamlType>,
    pub level: i32,
}

/// `RelativeSource` with `Mode=FindAncestor, Tree=Logical`.
#[derive(Clone)]
pub struct LogicalAncestorBindingExpressionNode {
    pub type_: Rc<dyn IXamlType>,
    pub level: i32,
}

/// `RelativeSource` with `Mode=TemplatedParent`.
#[derive(Clone)]
pub struct TemplatedParentBindingExpressionNode {
    pub type_: Rc<dyn IXamlType>,
}
