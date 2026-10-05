//! Port of `CompilerExtensions/Transformers/FerroXamlIlOptionMarkupExtensionTransformer.cs`.

use std::any::Any;
use std::cell::RefCell;
use std::rc::{Rc, Weak};

use xamlx::ast::{
    visit_cell, visit_list, visit_node, visit_optional_cell, IXamlAstNode, IXamlAstTypeReference,
    IXamlAstValueNode, IXamlAstVisitor, IXamlLineInfo, IXamlMarkupExtensionNodeExtension,
    XamlAstClrTypeReference, XamlAstExtensions, XamlAstNode, XamlAstNodeExtensions,
    XamlAstObjectNode, XamlAstPropertyReferenceExtensions, XamlAstTextNode,
    XamlAstXamlPropertyValueNode, XamlConstantNode, XamlMarkupExtensionNode,
    XamlTypeExtensionNode,
};
use xamlx::exceptions::{XamlError, XamlResult};
use xamlx::transform::{AstTransformationContext, IXamlAstTransformer, XamlTransformHelpers};
use xamlx::type_system::{
    AnonymousParameterInfo, IXamlCustomAttribute, IXamlMember, IXamlMethod, IXamlParameterInfo,
    IXamlType, TypeSystemHelpers, XamlValue,
};
use xamlx::{xaml_ast_node_members, xaml_line_info_impl, xaml_query_interface};

use super::FerroXamlIlWellKnownTypesExtensions;
use crate::compiler_extensions::IOptionsMarkupExtensionNode;

/// Turns an option markup extension (`OnPlatform`, `OnFormFactor`: a markup extension whose
/// type has public `bool ShouldProvideOption(option)` /
/// `bool ShouldProvideOption(IServiceProvider, option)` methods) into an
/// [`OptionsMarkupExtensionNode`]: one branch per option property that was set (or per
/// `<On Options="...">` child), plus an optional default value. Only the selected branch is
/// evaluated at run time.
pub struct FerroXamlIlOptionMarkupExtensionTransformer;

fn single<T>(mut items: Vec<T>) -> XamlResult<T> {
    match items.len() {
        0 => Err(XamlError::invalid_operation("Sequence contains no elements")),
        1 => Ok(items.remove(0)),
        _ => Err(XamlError::invalid_operation(
            "Sequence contains more than one element",
        )),
    }
}

/// `children.OfType<XamlAstXamlPropertyValueNode>().SingleOrDefault(v => v.Property.GetClrProperty().Name == name)`.
fn single_property_value(
    object: &XamlAstObjectNode,
    name: &str,
) -> XamlResult<Option<Rc<XamlAstXamlPropertyValueNode>>> {
    let mut found: Option<Rc<XamlAstXamlPropertyValueNode>> = None;
    let children = object.children.borrow().clone();
    for child in children {
        let Some(property_value) = child.cast::<XamlAstXamlPropertyValueNode>() else {
            continue;
        };
        if property_value.property().get_clr_property()?.name() == name {
            if found.is_some() {
                return Err(XamlError::invalid_operation(
                    "Sequence contains more than one matching element",
                ));
            }
            found = Some(property_value);
        }
    }
    Ok(found)
}

struct TransformState<'a> {
    context: &'a AstTransformationContext,
    methods: Vec<Rc<dyn IXamlMethod>>,
    option_attribute: Rc<dyn IXamlType>,
    default_option_attribute: Rc<dyn IXamlType>,
    type_argument: Option<Rc<dyn IXamlType>>,
    default_value: Option<Rc<dyn IXamlAstValueNode>>,
    values: Vec<Rc<OptionsMarkupExtensionBranch>>,
}

impl TransformState<'_> {
    fn transform_node(
        &self,
        values: &[Rc<dyn IXamlAstValueNode>],
        suggested_type: Option<&Rc<dyn IXamlType>>,
        line: &dyn IXamlLineInfo,
    ) -> XamlResult<Rc<dyn IXamlAstValueNode>> {
        let mut values: Vec<Rc<dyn IXamlAstValueNode>> = values.to_vec();
        if let Some(suggested_type) = suggested_type {
            let mut converted_values = Vec::with_capacity(values.len());
            for v in &values {
                converted_values.push(
                    match XamlTransformHelpers::try_get_correctly_typed_value(
                        self.context,
                        v,
                        suggested_type,
                    )? {
                        Some(converted) => converted,
                        None => v.clone(),
                    },
                );
            }
            values = converted_values;
        }

        if values.len() > 1 {
            return Err(XamlError::transform_exception(
                "Options markup extension supports only a singular value",
                Some(line),
            ));
        }

        single(values)
    }

    fn add_branch_node(
        &mut self,
        value_nodes: &[Rc<dyn IXamlAstValueNode>],
        prop_attributes: &[Rc<dyn IXamlCustomAttribute>],
        li: &dyn IXamlLineInfo,
    ) -> XamlResult<bool> {
        let transformed = self.transform_node(value_nodes, self.type_argument.as_ref(), li)?;
        if prop_attributes
            .iter()
            .any(|a| a.type_().equals(&*self.default_option_attribute))
        {
            self.default_value = Some(transformed);
            return Ok(true);
        }

        let Some(opt_attr) = prop_attributes
            .iter()
            .find(|a| a.type_().equals(&*self.option_attribute))
        else {
            return Ok(false);
        };

        let option = single(opt_attr.parameters())?;
        if option.is_null() {
            return Err(XamlError::transform_exception(
                "MarkupExtension option must not be null",
                Some(li),
            ));
        }

        let option_as_string = option.to_string();
        for method in &self.methods {
            let option_node = match self.try_create_option_node(method, &option, &option_as_string, li) {
                Ok(option_node) => option_node,
                // try next method overload
                Err(e) if e.type_name() == "FormatException" => None,
                Err(e) => return Err(e),
            };

            if let Some(option_node) = option_node {
                self.values.push(OptionsMarkupExtensionBranch::new(
                    option_node,
                    transformed,
                    method.clone(),
                ));
                return Ok(true);
            }
        }

        Err(XamlError::transform_exception(
            format!(
                "Option value \"{option_as_string}\" is not assignable to any of existing ShouldProvideOption methods"
            ),
            Some(li),
        ))
    }

    fn try_create_option_node(
        &self,
        method: &Rc<dyn IXamlMethod>,
        option: &XamlValue,
        option_as_string: &str,
        li: &dyn IXamlLineInfo,
    ) -> XamlResult<Option<Rc<dyn IXamlAstValueNode>>> {
        let target_type = method.parameters().into_iter().next_back().ok_or_else(|| {
            XamlError::invalid_operation("Sequence contains no elements")
        })?;
        if target_type.is("System", "Type") {
            if let XamlValue::Type(type_option) = option {
                return Ok(Some(XamlTypeExtensionNode::new(
                    li,
                    XamlAstClrTypeReference::new(li, type_option.clone(), false),
                    target_type,
                )));
            }
        } else if target_type.equals(&*self.context.configuration().well_known_types().string) {
            return Ok(Some(XamlConstantNode::new(
                li,
                target_type,
                XamlValue::String(option_as_string.to_string()),
            )?));
        } else if target_type.is_enum() {
            if let Some(enum_constant_node) =
                TypeSystemHelpers::try_get_enum_value_node(&target_type, option_as_string, li, false)?
            {
                return Ok(Some(enum_constant_node));
            }
        } else if let Some(constant_node) =
            TypeSystemHelpers::parse_constant_if_type_allows(option_as_string, &target_type, li)?
        {
            return Ok(Some(constant_node));
        }
        Ok(None)
    }
}

impl IXamlAstTransformer for FerroXamlIlOptionMarkupExtensionTransformer {
    fn transform(
        &self,
        context: &AstTransformationContext,
        node: Rc<dyn IXamlAstNode>,
    ) -> XamlResult<Rc<dyn IXamlAstNode>> {
        let Some(markup_extension_node) = node.cast::<XamlMarkupExtensionNode>() else {
            return Ok(node);
        };
        let Some(object_node) = markup_extension_node.value().cast::<XamlAstObjectNode>() else {
            return Ok(node);
        };
        let Some(type_) = object_node
            .type_
            .borrow()
            .clone()
            .cast::<XamlAstClrTypeReference>()
            .map(|type_reference| type_reference.type_.clone())
        else {
            return Ok(node);
        };
        let boolean = context.configuration().well_known_types().boolean.clone();
        let methods = type_.find_methods(|m| {
            let parameter_count = m.parameters().len();
            m.is_public()
                && (parameter_count == 1 || parameter_count == 2)
                && m.return_type().equals(&*boolean)
                && m.name() == "ShouldProvideOption"
        });
        if methods.is_empty() {
            return Ok(node);
        }

        let ferro_types = context.try_get_ferro_types()?;
        let mut state = TransformState {
            context,
            methods,
            option_attribute: ferro_types.markup_extension_option_attribute.clone(),
            default_option_attribute: ferro_types
                .markup_extension_default_option_attribute
                .clone(),
            type_argument: type_.generic_arguments().into_iter().next(),
            default_value: None,
            values: Vec::new(),
        };

        let first_argument = object_node.arguments.borrow().first().cloned();
        if let Some(argument) = first_argument {
            let has_default_prop = type_.get_all_properties().iter().any(|p| {
                p.custom_attributes()
                    .iter()
                    .any(|a| a.type_().equals(&*state.default_option_attribute))
            });
            if has_default_prop {
                if object_node.arguments.borrow().len() > 1 {
                    return Err(XamlError::transform_exception(
                        "Options MarkupExtensions allow only single argument",
                        Some(&*object_node),
                    ));
                }

                state.default_value = Some(state.transform_node(
                    std::slice::from_ref(&argument),
                    state.type_argument.as_ref(),
                    &*object_node,
                )?);
                object_node
                    .arguments
                    .borrow_mut()
                    .retain(|a| !a.same_node(&argument));
            }
        }

        let ext_props: Vec<Rc<XamlAstXamlPropertyValueNode>> = object_node
            .children
            .borrow()
            .iter()
            .filter_map(|c| c.cast::<XamlAstXamlPropertyValueNode>())
            .collect();
        for ext_prop in ext_props {
            let ext_values = ext_prop.values.borrow().clone();
            if ext_values.is_empty() {
                continue;
            }

            let should_remove_prop;
            let mut on_objs: Vec<Rc<XamlAstObjectNode>> = Vec::new();
            for value in &ext_values {
                if let Some(o) = value.cast::<XamlAstObjectNode>() {
                    let object_type = o.type_.borrow().clone().get_clr_type()?;
                    if object_type.equals(&*ferro_types.on_extension_type) {
                        on_objs.push(o);
                    }
                }
            }
            if !on_objs.is_empty() {
                should_remove_prop = true;
                for on_obj in on_objs {
                    let options_prop_node = match single_property_value(&on_obj, "Options")? {
                        Some(options_prop) => Some(single(options_prop.values.borrow().clone())?),
                        None => None,
                    };
                    let options: Vec<String> = options_prop_node
                        .and_then(|n| n.cast::<XamlAstTextNode>())
                        .map(|text| {
                            text.text()
                                .split([',', ' '])
                                .filter(|o| !o.is_empty())
                                .map(str::to_string)
                                .collect()
                        })
                        .unwrap_or_default();
                    if options.is_empty() {
                        return Err(XamlError::transform_exception(
                            "On.Options string must be set",
                            Some(&*on_obj),
                        ));
                    }

                    let Some(content) = single_property_value(&on_obj, "Content")? else {
                        return Err(XamlError::transform_exception(
                            "On content object must be set",
                            Some(&*on_obj),
                        ));
                    };

                    let all_properties = type_.get_all_properties();
                    let mut properties_set = Vec::with_capacity(options.len());
                    for o in &options {
                        let property = all_properties
                            .iter()
                            .find(|p| p.name() == *o)
                            .cloned()
                            .ok_or_else(|| {
                                XamlError::transform_exception(
                                    format!(
                                        "Property \"{o}\" wasn't found on the \"{}\" type",
                                        type_.name()
                                    ),
                                    Some(&*on_obj),
                                )
                            })?;
                        properties_set.push(property);
                    }

                    for property_set in properties_set {
                        let content_values = content.values.borrow().clone();
                        state.add_branch_node(
                            &content_values,
                            &property_set.custom_attributes(),
                            &*content,
                        )?;
                    }
                }
            } else {
                let attributes = ext_prop.property().get_clr_property()?.custom_attributes();
                should_remove_prop = state.add_branch_node(&ext_values, &attributes, &*ext_prop)?;
            }

            if should_remove_prop {
                object_node
                    .children
                    .borrow_mut()
                    .retain(|c| !c.same_node(&ext_prop));
            }
        }

        if state.default_value.is_none() && state.values.is_empty() {
            return Err(XamlError::transform_exception(
                "Options markup extension requires at least one option to be set",
                Some(&*object_node),
            ));
        }

        Ok(OptionsMarkupExtensionNode::new(
            &markup_extension_node,
            state.values,
            state.default_value,
            context.configuration().type_mappings.service_provider()?,
        )?)
    }
}

/// `OptionsMarkupExtensionNode : XamlMarkupExtensionNode`: a markup extension node whose
/// `ProvideValue` is an [`OptionsMarkupExtensionMethod`].
///
/// The derived class is represented by a `XamlMarkupExtensionNode` whose
/// [`XamlMarkupExtensionNode::extension`] is an instance of this type:
/// [`OptionsMarkupExtensionNode::new`] returns that node, `node is OptionsMarkupExtensionNode`
/// is [`OptionsMarkupExtensionNode::from_node`]. The node's type is recomputed from the
/// branches on every access, it visits the branch container before its value, and it answers
/// [`IOptionsMarkupExtensionNode`] interface queries.
pub struct OptionsMarkupExtensionNode {
    provide_value: Rc<OptionsMarkupExtensionMethod>,
    context_parameter: Rc<dyn IXamlType>,
    node: RefCell<Weak<XamlMarkupExtensionNode>>,
}

impl OptionsMarkupExtensionNode {
    /// `new OptionsMarkupExtensionNode(original, branches, defaultNode, contextParameter)`.
    ///
    /// Fails when the branches and the default value have no common base type (upstream: the
    /// base constructor reads `ProvideValue.ReturnType`).
    pub fn new(
        original: &XamlMarkupExtensionNode,
        branches: Vec<Rc<OptionsMarkupExtensionBranch>>,
        default_node: Option<Rc<dyn IXamlAstValueNode>>,
        context_parameter: Rc<dyn IXamlType>,
    ) -> XamlResult<Rc<XamlMarkupExtensionNode>> {
        let original_value = original.value();
        let container = OptionsMarkupExtensionNodesContainer::new(branches, default_node)?;
        let provide_value = OptionsMarkupExtensionMethod::new(
            container,
            original_value.type_().get_clr_type()?,
            context_parameter.clone(),
        )?;
        let node = XamlMarkupExtensionNode::new(
            &*original_value,
            provide_value.clone(),
            original_value.clone(),
        );
        node.set_extension(Rc::new(OptionsMarkupExtensionNode {
            provide_value,
            context_parameter,
            node: RefCell::new(Rc::downgrade(&node)),
        }));
        Ok(node)
    }

    /// `node as OptionsMarkupExtensionNode`.
    pub fn from_node(node: &XamlMarkupExtensionNode) -> Option<Rc<OptionsMarkupExtensionNode>> {
        node.extension_as::<OptionsMarkupExtensionNode>()
    }

    /// `ProvideValue`, typed.
    pub fn provide_value(&self) -> &Rc<OptionsMarkupExtensionMethod> {
        &self.provide_value
    }

    /// `ConvertToReturnType(context, type, out res)`: `Ok(Some(res))` is `true`.
    pub fn convert_to_return_type(
        &self,
        context: &AstTransformationContext,
        type_: &Rc<dyn IXamlType>,
    ) -> XamlResult<Option<Rc<XamlMarkupExtensionNode>>> {
        let container = &self.provide_value.extension_node_container;
        let mut converted_default_node: Option<Rc<dyn IXamlAstValueNode>> = None;
        if let Some(default_node) = container.default_node() {
            match XamlTransformHelpers::try_get_correctly_typed_value(context, &default_node, type_)?
            {
                Some(converted) => converted_default_node = Some(converted),
                None => return Ok(None),
            }
        }

        let mut converted_branches: Vec<Option<Rc<OptionsMarkupExtensionBranch>>> = Vec::new();
        for b in container.branches() {
            converted_branches.push(
                XamlTransformHelpers::try_get_correctly_typed_value(context, &b.value(), type_)?
                    .map(|converted_value| {
                        OptionsMarkupExtensionBranch::new(
                            b.option(),
                            converted_value,
                            b.condition_method.clone(),
                        )
                    }),
            );
        }

        let Some(converted_branches) = converted_branches.into_iter().collect::<Option<Vec<_>>>()
        else {
            return Ok(None);
        };

        let this = self.node.borrow().upgrade().ok_or_else(|| {
            XamlError::invalid_operation("The options markup extension node is no longer alive")
        })?;
        Ok(Some(OptionsMarkupExtensionNode::new(
            &this,
            converted_branches,
            converted_default_node,
            self.context_parameter.clone(),
        )?))
    }
}

impl IOptionsMarkupExtensionNode for OptionsMarkupExtensionNode {
    fn convert_to_return_type(
        &self,
        context: &AstTransformationContext,
        type_: &Rc<dyn IXamlType>,
    ) -> XamlResult<Option<Rc<dyn IXamlAstValueNode>>> {
        Ok(
            OptionsMarkupExtensionNode::convert_to_return_type(self, context, type_)?.map(|node| {
                let node: Rc<dyn IXamlAstValueNode> = node;
                node
            }),
        )
    }
}

impl IXamlMarkupExtensionNodeExtension for OptionsMarkupExtensionNode {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn into_any_rc(self: Rc<Self>) -> Rc<dyn Any> {
        self
    }
    fn type_name(&self) -> &'static str {
        "OptionsMarkupExtensionNode"
    }
    /// `IXamlAstValueNode.Type => new XamlAstClrTypeReference(this, ProvideValue.ReturnType, false)`.
    fn type_(&self, node: &XamlMarkupExtensionNode) -> Option<Rc<dyn IXamlAstTypeReference>> {
        Some(XamlAstClrTypeReference::new(
            node,
            self.provide_value.return_type(),
            false,
        ))
    }
    /// `ProvideValue.ExtensionNodeContainer.Visit(visitor)`; the result is discarded, as upstream.
    fn visit_children(&self, visitor: &mut dyn IXamlAstVisitor) -> XamlResult<()> {
        visit_node(
            &self.provide_value.extension_node_container.as_node(),
            visitor,
        )?;
        Ok(())
    }
    fn query_interface(self: Rc<Self>, slot: &mut dyn Any) -> bool {
        xaml_query_interface!(self, slot, dyn IOptionsMarkupExtensionNode)
    }
}

/// The branches and the default value of an options markup extension, as one AST node so
/// that transformers visit them.
pub struct OptionsMarkupExtensionNodesContainer {
    base: XamlAstNode,
    pub branches: RefCell<Vec<Rc<OptionsMarkupExtensionBranch>>>,
    pub default_node: RefCell<Option<Rc<dyn IXamlAstValueNode>>>,
}

impl OptionsMarkupExtensionNodesContainer {
    /// The line info is that of the first branch's value, or of the default value. Fails when
    /// there is neither (upstream dereferences `null`).
    pub fn new(
        branches: Vec<Rc<OptionsMarkupExtensionBranch>>,
        default_node: Option<Rc<dyn IXamlAstValueNode>>,
    ) -> XamlResult<Rc<Self>> {
        let line_info: Rc<dyn IXamlAstValueNode> = match branches.first() {
            Some(branch) => branch.value(),
            None => default_node.clone().ok_or_else(|| {
                XamlError::internal(
                    "NullReferenceException",
                    "An options markup extension needs a branch or a default value",
                )
            })?,
        };
        Ok(Rc::new(Self {
            base: XamlAstNode::new(&*line_info),
            branches: RefCell::new(branches),
            default_node: RefCell::new(default_node),
        }))
    }

    pub fn branches(&self) -> Vec<Rc<OptionsMarkupExtensionBranch>> {
        self.branches.borrow().clone()
    }

    pub fn default_node(&self) -> Option<Rc<dyn IXamlAstValueNode>> {
        self.default_node.borrow().clone()
    }

    /// The common base class of the types of all branch values and the default value.
    pub fn get_return_type(&self) -> XamlResult<Rc<dyn IXamlType>> {
        let mut types = Vec::new();
        for b in self.branches.borrow().iter() {
            types.push(b.value().type_().get_clr_type()?);
        }
        if let Some(default_node) = self.default_node() {
            types.push(default_node.type_().get_clr_type()?);
        }
        XamlTransformHelpers::get_common_base_class(&types)
    }
}

xaml_line_info_impl!(OptionsMarkupExtensionNodesContainer, base);

impl IXamlAstNode for OptionsMarkupExtensionNodesContainer {
    xaml_ast_node_members!("OptionsMarkupExtensionNodesContainer");

    fn visit_children(&self, visitor: &mut dyn IXamlAstVisitor) -> XamlResult<()> {
        visit_list(&self.branches, visitor)?;
        visit_optional_cell(&self.default_node, visitor)
    }
}

/// One branch of an options markup extension: when `condition_method(option)` (or
/// `condition_method(serviceProvider, option)`) returns `true`, the extension provides `value`.
pub struct OptionsMarkupExtensionBranch {
    base: XamlAstNode,
    pub option: RefCell<Rc<dyn IXamlAstValueNode>>,
    pub value: RefCell<Rc<dyn IXamlAstValueNode>>,
    pub condition_method: Rc<dyn IXamlMethod>,
}

impl OptionsMarkupExtensionBranch {
    pub fn new(
        option: Rc<dyn IXamlAstValueNode>,
        value: Rc<dyn IXamlAstValueNode>,
        condition_method: Rc<dyn IXamlMethod>,
    ) -> Rc<Self> {
        Rc::new(Self {
            base: XamlAstNode::new(&*value),
            option: RefCell::new(option),
            value: RefCell::new(value),
            condition_method,
        })
    }

    pub fn option(&self) -> Rc<dyn IXamlAstValueNode> {
        self.option.borrow().clone()
    }

    pub fn value(&self) -> Rc<dyn IXamlAstValueNode> {
        self.value.borrow().clone()
    }

    /// Whether the condition method takes the service provider in front of the option.
    pub fn has_context(&self) -> bool {
        self.condition_method.parameters().len() > 1
    }
}

xaml_line_info_impl!(OptionsMarkupExtensionBranch, base);

impl IXamlAstNode for OptionsMarkupExtensionBranch {
    xaml_ast_node_members!("OptionsMarkupExtensionBranch");

    fn visit_children(&self, visitor: &mut dyn IXamlAstVisitor) -> XamlResult<()> {
        visit_cell(&self.option, visitor)?;
        visit_cell(&self.value, visitor)
    }
}

/// The pseudo `ProvideValue` method of an options markup extension: a public instance method
/// of the extension type that takes the service provider when one of the branch conditions
/// needs it and returns the common base class of the branch values. Methods compare by
/// identity.
///
/// # What a back end has to do (upstream IL)
///
/// `EmitCall` is invoked by the markup extension emitter in place of a real `ProvideValue`
/// call and shares the stack and the locals of the calling method. On entry the stack holds
/// the context (only when `parameters()` is not empty) below the markup extension instance;
/// on exit it holds the provided value.
///
/// 1. When `parameters()` is not empty, pop and discard the context argument (the method
///    reloads the context itself where it needs it).
/// 2. When any branch has an instance `condition_method`, store the extension instance in a
///    new local of the declaring type; otherwise pop and discard it.
/// 3. For each branch in order: when the branch `has_context()`, load the runtime context
///    local (the service provider argument); when its `condition_method` is not static, load
///    the stored extension instance; emit the branch `option` node as its own type; call
///    `condition_method`. When the result is `false` go on with the next branch, otherwise
///    emit the branch `value` node as its own type and finish.
/// 4. When no branch matched: emit the container's default node as its own type when there
///    is one, otherwise the default value (`default(T)`) of `return_type()`.
///
/// Only the option nodes of the tested branches and the value node of the selected branch
/// (or the default) are evaluated.
pub struct OptionsMarkupExtensionMethod {
    pub extension_node_container: Rc<OptionsMarkupExtensionNodesContainer>,
    declaring_type: Rc<dyn IXamlType>,
    parameters: Vec<Rc<dyn IXamlType>>,
    /// The last successfully computed return type; see `return_type`.
    last_return_type: RefCell<Rc<dyn IXamlType>>,
}

impl OptionsMarkupExtensionMethod {
    /// Fails when the return type cannot be computed (no common base class).
    pub fn new(
        extension_node_container: Rc<OptionsMarkupExtensionNodesContainer>,
        declaring_type: Rc<dyn IXamlType>,
        context_parameter: Rc<dyn IXamlType>,
    ) -> XamlResult<Rc<Self>> {
        let parameters = if extension_node_container
            .branches
            .borrow()
            .iter()
            .any(|c| c.has_context())
        {
            vec![context_parameter]
        } else {
            Vec::new()
        };
        let return_type = extension_node_container.get_return_type()?;
        Ok(Rc::new(Self {
            extension_node_container,
            declaring_type,
            parameters,
            last_return_type: RefCell::new(return_type),
        }))
    }
}

impl IXamlMember for OptionsMarkupExtensionMethod {
    fn name(&self) -> String {
        "ProvideValue".to_string()
    }
    fn declaring_type(&self) -> Rc<dyn IXamlType> {
        self.declaring_type.clone()
    }
}

impl IXamlMethod for OptionsMarkupExtensionMethod {
    fn is_public(&self) -> bool {
        true
    }
    fn is_private(&self) -> bool {
        false
    }
    fn is_family(&self) -> bool {
        false
    }
    fn is_static(&self) -> bool {
        false
    }
    fn contains_generic_parameters(&self) -> bool {
        false
    }
    fn is_generic_method(&self) -> bool {
        false
    }
    fn is_generic_method_definition(&self) -> bool {
        false
    }
    /// `ExtensionNodeContainer.GetReturnType()`, recomputed on every call because later
    /// transformers replace the branch values. Upstream throws when the values have no common
    /// base class; this accessor cannot fail, so it then answers the last type that could be
    /// computed (the constructor reports the failure for the initial values).
    fn return_type(&self) -> Rc<dyn IXamlType> {
        match self.extension_node_container.get_return_type() {
            Ok(return_type) => {
                *self.last_return_type.borrow_mut() = return_type.clone();
                return_type
            }
            Err(_) => self.last_return_type.borrow().clone(),
        }
    }
    fn parameters(&self) -> Vec<Rc<dyn IXamlType>> {
        self.parameters.clone()
    }
    fn make_generic_method(
        &self,
        _type_arguments: &[Rc<dyn IXamlType>],
    ) -> XamlResult<Rc<dyn IXamlMethod>> {
        Err(XamlError::internal(
            "NotImplementedException",
            "The method or operation is not implemented.",
        ))
    }
    fn custom_attributes(&self) -> Vec<Rc<dyn IXamlCustomAttribute>> {
        Vec::new()
    }
    fn get_parameter_info(&self, index: usize) -> XamlResult<Rc<dyn IXamlParameterInfo>> {
        let parameter = self.parameters.get(index).ok_or_else(|| {
            XamlError::internal(
                "ArgumentOutOfRangeException",
                "Index was out of range. Must be non-negative and less than the size of the collection.",
            )
        })?;
        Ok(Rc::new(AnonymousParameterInfo::with_index(
            parameter.clone(),
            index,
        )))
    }
    fn generic_parameters(&self) -> Vec<Rc<dyn IXamlType>> {
        Vec::new()
    }
    fn generic_arguments(&self) -> Vec<Rc<dyn IXamlType>> {
        Vec::new()
    }
    fn equals(&self, other: &dyn IXamlMethod) -> bool {
        std::ptr::addr_eq(
            self as *const Self,
            other.as_any() as *const dyn Any,
        )
    }
    fn get_hash_code(&self) -> u64 {
        self as *const Self as usize as u64
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}
