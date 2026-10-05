//! Port of `CompilerExtensions/Transformers/FerroXamlIlSelectorTransformer.cs`.
//!
//! The transformer turns the text of a `Style.Selector` property into a chain of typed selector
//! nodes and wraps the style in a target-type scope. The selector nodes generate IL upstream;
//! here they carry their data and document what a back end has to do with it.
//!
//! # What a back end has to do with a selector node (upstream IL)
//!
//! Every selector node is a value node typed `Selector` ([`IXamlAstValueNode::type_`]). Emitting
//! a node pushes exactly one value of that type:
//!
//! 1. when the node has a [`XamlIlSelectorNode::previous`] node, emit it first, converted to the
//!    selector type (it becomes the first argument, `previous`, of the builder call below);
//! 2. perform the node's own step, documented on each node type. Except for the initial node
//!    (which pushes `null`) the step pushes the node's further arguments and calls one static
//!    method of the `Selectors` builder class (`FerroXamlIlWellKnownTypes::selectors`), which
//!    pops `previous` and the arguments and pushes the resulting selector.
//!
//! The builder method is looked up by shape, not by signature: the first method of `Selectors`
//! (declared methods first, then base types, then interfaces) that is static, has at least one
//! parameter and satisfies the predicate of the node. Each node exposes that lookup as
//! `builder_method`; it fails with a `XamlTypeSystemException` when there is no such method.

use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;

use ferroui_markup::markup::parsers::{SelectorGrammar, SelectorSyntax};
use xamlx::ast::{
    IXamlAstNode, IXamlAstTypeReference, IXamlAstValueNode, IXamlLineInfo, XamlAstCast,
    XamlAstClrTypeReference, XamlAstExtensions, XamlAstNode, XamlAstNodeExtensions,
    XamlAstObjectNode, XamlAstPropertyReferenceExtensions, XamlAstTextNode,
    XamlAstXamlPropertyValueNode,
};
use xamlx::exceptions::{XamlError, XamlResult};
use xamlx::extensions::query_node_interface;
use xamlx::transform::transformers::TypeReferenceResolver;
use xamlx::transform::{AstTransformationContext, IXamlAstTransformer, XamlTransformHelpers};
use xamlx::type_system::{IXamlField, IXamlMethod, IXamlProperty, IXamlType, XamlPseudoType};
use xamlx::{xaml_ast_node_members, xaml_line_info_impl, xaml_query_interface};

use crate::compiler_extensions::transformers::{
    FerroXamlIlTargetTypeMetadataNode, FerroXamlIlWellKnownTypes,
    FerroXamlIlWellKnownTypesExtensions, ScopeTypes,
};
use crate::compiler_extensions::XamlIlFerroPropertyHelper;

/// `XamlSelectorsTransformException : XamlTransformException`.
///
/// Errors are values of the single [`XamlError`] type; the derived class is represented by a
/// `XamlError::Transform` tagged with the class name, which is what
/// `FerroXamlDiagnosticCodes::xaml_x_diagnostic_code_to_ferro` and [`Self::is`] look at.
pub struct XamlSelectorsTransformException;

impl XamlSelectorsTransformException {
    pub const TYPE_NAME: &'static str = "XamlSelectorsTransformException";

    /// `new XamlSelectorsTransformException(message, lineInfo, innerException)`.
    pub fn new(
        message: impl Into<String>,
        line_info: &dyn IXamlLineInfo,
        inner_exception: Option<XamlError>,
    ) -> XamlError {
        let error = match inner_exception {
            Some(inner) => {
                XamlError::transform_exception_with_inner(message, Some(line_info), inner)
            }
            None => XamlError::transform_exception(message, Some(line_info)),
        };
        error.with_derived_type_name(Self::TYPE_NAME)
    }

    /// `e is XamlSelectorsTransformException`.
    pub fn is(error: &XamlError) -> bool {
        error.is_derived_type(Self::TYPE_NAME)
    }
}

/// `new XamlAstClrTypeReference(lineInfo, type, false)` for a `type` that may be `null`.
///
/// Upstream stores a `null` type in the target-type scope of a style whose selector has no
/// target type (`.foo`) and of every container query. A CLR type reference always has a type
/// here, so `null` is represented by the `{x:Null}` pseudo type; read such a reference back
/// with [`get_nullable_clr_type`].
pub fn create_nullable_clr_type_reference(
    line_info: &dyn IXamlLineInfo,
    type_: Option<Rc<dyn IXamlType>>,
) -> Rc<XamlAstClrTypeReference> {
    XamlAstClrTypeReference::new(line_info, type_.unwrap_or_else(XamlPseudoType::null), false)
}

/// `reference.GetClrType()` for a reference created by
/// [`create_nullable_clr_type_reference`]: `None` is the upstream `null`.
pub fn get_nullable_clr_type(
    reference: &Rc<dyn IXamlAstTypeReference>,
) -> XamlResult<Option<Rc<dyn IXamlType>>> {
    let type_ = reference.get_clr_type()?;
    Ok(if XamlPseudoType::is_null(&*type_) {
        None
    } else {
        Some(type_)
    })
}

pub(crate) fn argument_out_of_range() -> XamlError {
    XamlError::internal(
        "ArgumentOutOfRangeException",
        "Index was out of range. Must be non-negative and less than the size of the collection.",
    )
}

pub(crate) fn null_reference() -> XamlError {
    XamlError::internal(
        "NullReferenceException",
        "Object reference not set to an instance of an object.",
    )
}

pub struct FerroXamlIlSelectorTransformer;

impl IXamlAstTransformer for FerroXamlIlSelectorTransformer {
    fn transform(
        &self,
        context: &AstTransformationContext,
        node: Rc<dyn IXamlAstNode>,
    ) -> XamlResult<Rc<dyn IXamlAstNode>> {
        let Some(on) = node.cast::<XamlAstObjectNode>() else {
            return Ok(node);
        };
        let types = context.get_ferro_types();
        if !types.style.is_assignable_from(&*on.type_().get_clr_type()?) {
            return Ok(node);
        }

        let mut pn: Option<Rc<XamlAstXamlPropertyValueNode>> = None;
        let children = on.children.borrow().clone();
        for child in children {
            if let Some(p) = child.cast::<XamlAstXamlPropertyValueNode>() {
                if p.property().get_clr_property()?.name() == "Selector" {
                    pn = Some(p);
                    break;
                }
            }
        }

        // Missing selector, use the object's target type if available
        let Some(pn) = pn else {
            // We already went through this node
            if let Some(metadata_node) = context
                .first_parent_node()
                .and_then(|p| p.cast::<FerroXamlIlTargetTypeMetadataNode>())
            {
                if metadata_node.value().same_node(&on) {
                    return Ok(node);
                }
            }

            if let Some(parent_object_node) = find_style_parent_object(&*on, context)? {
                let style_node: Rc<dyn IXamlAstValueNode> = on;
                return Ok(FerroXamlIlTargetTypeMetadataNode::new(
                    style_node,
                    XamlAstClrTypeReference::new(
                        &*node,
                        parent_object_node.type_().get_clr_type()?,
                        false,
                    ),
                    ScopeTypes::Style,
                ));
            }

            return Ok(node);
        };

        if pn.values.borrow().len() != 1 {
            return Err(XamlSelectorsTransformException::new(
                "Selector property should have exactly one value",
                &*node,
                None,
            ));
        }

        let value = pn.values.borrow()[0].clone();
        if value.is::<dyn XamlIlSelectorNode>() {
            // Deja vu. I've just been in this place before
            return Ok(node);
        }

        let Some(tn) = value.cast::<XamlAstTextNode>() else {
            return Err(XamlSelectorsTransformException::new(
                "Selector property should be a text node",
                &*node,
                None,
            ));
        };

        let selector_type = pn
            .property()
            .get_clr_property()?
            .getter()
            .ok_or_else(null_reference)?
            .return_type();
        let initial_node: Rc<dyn XamlIlSelectorNode> =
            XamlIlSelectorInitialNode::new(&*node, selector_type.clone());
        let factory = SelectorFactory {
            context,
            node: &node,
            selector_type,
            initial_node: initial_node.clone(),
            ferro_attached_property_t: types.ferro_attached_property_t.clone(),
            types: &types,
        };

        let parsed = match SelectorGrammar::parse(&tn.text()) {
            Ok(parsed) => parsed,
            Err(e) => {
                return Err(XamlSelectorsTransformException::new(
                    format!("Unable to parse selector: {}", e.message()),
                    &*node,
                    Some(XamlError::internal("ExpressionParseException", e.message())),
                ));
            }
        };

        // Selectors should resolve control types only.
        // isMarkupExtension = false to prevent resolving selector types to XExtension.
        let selector = factory.create(&parsed)?;
        {
            let selector_value: Rc<dyn IXamlAstValueNode> = selector.clone();
            pn.values.borrow_mut()[0] = selector_value;
        }

        let template_type = get_last_template_type_from_selector(Some(selector.clone()));

        // Empty selector, use the object's target type if available
        if selector.same_node(&initial_node) {
            if let Some(parent_object_node) = find_style_parent_object(&*on, context)? {
                let style_node: Rc<dyn IXamlAstValueNode> = on;
                return Ok(FerroXamlIlTargetTypeMetadataNode::new(
                    style_node,
                    XamlAstClrTypeReference::new(
                        &*node,
                        parent_object_node.type_().get_clr_type()?,
                        false,
                    ),
                    ScopeTypes::Style,
                ));
            }

            return Ok(node);
        }

        let style_value: Rc<dyn IXamlAstValueNode> = on;
        let style_node = FerroXamlIlTargetTypeMetadataNode::new(
            style_value,
            create_nullable_clr_type_reference(&*selector, selector.target_type()),
            ScopeTypes::Style,
        );

        Ok(match template_type {
            None => style_node,
            Some(template_type) => {
                let type_reference = XamlAstClrTypeReference::new(&*style_node, template_type, false);
                FerroXamlIlTargetTypeMetadataNode::new(
                    style_node,
                    type_reference,
                    ScopeTypes::ControlTemplate,
                )
            }
        })
    }
}

/// The local function `Create` of the upstream `Transform` with the variables it captures.
struct SelectorFactory<'a> {
    context: &'a AstTransformationContext,
    node: &'a Rc<dyn IXamlAstNode>,
    selector_type: Rc<dyn IXamlType>,
    initial_node: Rc<dyn XamlIlSelectorNode>,
    ferro_attached_property_t: Rc<dyn IXamlType>,
    types: &'a FerroXamlIlWellKnownTypes,
}

impl SelectorFactory<'_> {
    /// The `typeResolver` argument: `TypeReferenceResolver.ResolveType(context, $"{p}:{n}",
    /// false, node, true)`.
    fn resolve_type(&self, xmlns: &str, name: &str) -> XamlResult<Rc<XamlAstClrTypeReference>> {
        TypeReferenceResolver::resolve_type_by_xml_name(
            self.context,
            &format!("{xmlns}:{name}"),
            false,
            &**self.node,
            true,
        )
    }

    fn text_node(&self, text: &str) -> Rc<dyn IXamlAstValueNode> {
        XamlAstTextNode::with_type(
            &**self.node,
            text,
            false,
            Some(self.context.configuration().well_known_types().string.clone()),
        )
    }

    fn create(&self, syntax: &[SelectorSyntax]) -> XamlResult<Rc<dyn XamlIlSelectorNode>> {
        let node = &**self.node;
        let mut result: Rc<dyn XamlIlSelectorNode> = self.initial_node.clone();
        let mut results: Option<Rc<XamlIlOrSelectorNode>> = None;
        for i in syntax {
            match i {
                SelectorSyntax::OfType { type_name, xmlns } => {
                    let type_ = self.resolve_type(xmlns, type_name)?.type_.clone();
                    result = XamlIlTypeSelector::new(result, type_, true);
                }
                SelectorSyntax::Is { type_name, xmlns } => {
                    let type_ = self.resolve_type(xmlns, type_name)?.type_.clone();
                    result = XamlIlTypeSelector::new(result, type_, false);
                }
                SelectorSyntax::Class { class } => {
                    result = XamlIlStringSelector::new(result, XamlIlStringSelectorType::Class, class);
                }
                SelectorSyntax::Name { name } => {
                    result = XamlIlStringSelector::new(result, XamlIlStringSelectorType::Name, name);
                }
                SelectorSyntax::Property { property, value } => {
                    let Some(type_) = result.target_type() else {
                        return Err(XamlError::transform_exception(
                            "Property selectors must be applied to a type.",
                            Some(node),
                        ));
                    };

                    let target_property = type_
                        .get_all_properties()
                        .into_iter()
                        .find(|p| p.name() == *property);

                    let Some(target_property) = target_property else {
                        return Err(XamlError::transform_exception(
                            format!("Cannot find '{property}' on '{}", type_.to_type_string()),
                            Some(node),
                        ));
                    };

                    let Some(typed_value) =
                        XamlTransformHelpers::try_get_correctly_typed_value_for_property(
                            self.context,
                            &self.text_node(value),
                            &*target_property,
                        )?
                    else {
                        return Err(XamlError::transform_exception(
                            format!(
                                "Cannot convert '{value}' to '{}",
                                target_property.property_type().get_fqn()
                            ),
                            Some(node),
                        ));
                    };

                    result = XamlIlPropertyEqualsSelector::new(result, target_property, typed_value);
                }
                SelectorSyntax::AttachedProperty {
                    xmlns,
                    type_name,
                    property,
                    value,
                } => {
                    if result.target_type().is_none() {
                        return Err(XamlError::transform_exception(
                            "Attached Property selectors must be applied to a type.",
                            Some(node),
                        ));
                    }
                    // As upstream the owner type comes from the type resolver, which fails
                    // instead of returning null; the upstream `Cannot find 'ns:Type` error for
                    // a null owner type is therefore unreachable and not ported.
                    let attached_property_owner_type =
                        self.resolve_type(xmlns, type_name)?.type_.clone();

                    let attached_property_name = format!("{property}Property");

                    let target_property_field = attached_property_owner_type
                        .get_all_fields()
                        .into_iter()
                        .find(|f| {
                            f.is_static()
                                && f.is_public()
                                && f.name() == attached_property_name
                                && f.field_type()
                                    .generic_type_definition()
                                    .is_some_and(|d| d.equals(&*self.ferro_attached_property_t))
                        });

                    let Some(target_property_field) = target_property_field else {
                        return Err(XamlError::transform_exception(
                            format!(
                                "Cannot find '{property}' on '{}",
                                attached_property_owner_type.get_fqn()
                            ),
                            Some(node),
                        ));
                    };

                    let target_property_type = XamlIlFerroPropertyHelper::get_ferro_property_type(
                        &*target_property_field,
                        self.types,
                        node,
                    )?;

                    let Some(typed_value) = XamlTransformHelpers::try_get_correctly_typed_value(
                        self.context,
                        &self.text_node(value),
                        &target_property_type,
                    )?
                    else {
                        return Err(XamlError::transform_exception(
                            format!(
                                "Cannot convert '{value}' to '{}",
                                target_property_type.get_fqn()
                            ),
                            Some(node),
                        ));
                    };

                    result = XamlIlAttachedPropertyEqualsSelector::new(
                        result,
                        target_property_field,
                        typed_value,
                    );
                }
                SelectorSyntax::Child => {
                    result = XamlIlCombinatorSelector::new(result, CombinatorSelectorType::Child);
                }
                SelectorSyntax::Descendant => {
                    result =
                        XamlIlCombinatorSelector::new(result, CombinatorSelectorType::Descendant);
                }
                SelectorSyntax::Template => {
                    result = XamlIlCombinatorSelector::new(result, CombinatorSelectorType::Template);
                }
                SelectorSyntax::Not { argument } => {
                    let argument = self.create(argument)?;
                    result = XamlIlNotSelector::new(result, argument);
                }
                SelectorSyntax::NthChild { offset, step } => {
                    result = XamlIlNthChildSelector::new(
                        result,
                        *step,
                        *offset,
                        XamlIlNthChildSelectorType::NthChild,
                    );
                }
                SelectorSyntax::NthLastChild { offset, step } => {
                    result = XamlIlNthChildSelector::new(
                        result,
                        *step,
                        *offset,
                        XamlIlNthChildSelectorType::NthLastChild,
                    );
                }
                SelectorSyntax::Comma => {
                    let results = results.get_or_insert_with(|| {
                        XamlIlOrSelectorNode::new(node, self.selector_type.clone())
                    });
                    results.add(result);
                    result = self.initial_node.clone();
                }
                SelectorSyntax::Nesting => {
                    let parent_target_type = self
                        .context
                        .parent_nodes()
                        .into_iter()
                        .find_map(|n| n.cast::<FerroXamlIlTargetTypeMetadataNode>());

                    let Some(parent_target_type) = parent_target_type else {
                        return Err(XamlError::transform_exception(
                            "Cannot find parent style for nested selector.",
                            Some(node),
                        ));
                    };

                    result = XamlIlNestingSelector::new(
                        result,
                        get_nullable_clr_type(&parent_target_type.target_type())?,
                    );
                }
                // The upstream `default` branch ("Unsupported selector grammar") is unreachable:
                // the syntax is a closed enum.
            }
        }

        if let Some(results) = &results {
            results.add(result.clone());
        }

        match results {
            Some(results) => Ok(results),
            None => Ok(result),
        }
    }
}

fn find_style_parent_object(
    style_node: &dyn IXamlLineInfo,
    context: &AstTransformationContext,
) -> XamlResult<Option<Rc<XamlAstObjectNode>>> {
    let ferro_types = context.get_ferro_types();

    let mut parent_node: Option<Rc<XamlAstObjectNode>> = None;
    for n in context.parent_nodes() {
        if let Some(n) = n.cast::<XamlAstObjectNode>() {
            if !ferro_types.styles.is_assignable_from(&*n.type_().get_clr_type()?) {
                parent_node = Some(n);
                break;
            }
        }
    }

    if let Some(parent_node) = parent_node {
        let parent_type = parent_node.type_().get_clr_type()?;

        if ferro_types.styled_element.is_assignable_from(&*parent_type) {
            return Ok(Some(parent_node));
        }

        if ferro_types.control_theme.is_assignable_from(&*parent_type) {
            return Err(XamlError::transform_exception(
                "Cannot add a Style without selector to a ControlTheme.",
                Some(style_node),
            ));
        }
    }

    Ok(None)
}

fn get_last_template_type_from_selector(
    mut node: Option<Rc<dyn XamlIlSelectorNode>>,
) -> Option<Rc<dyn IXamlType>> {
    while let Some(current) = node {
        if let Some(combinator) = current.as_any().downcast_ref::<XamlIlCombinatorSelector>() {
            if combinator.selector_type == CombinatorSelectorType::Template {
                return current.previous().and_then(|p| p.target_type());
            }
        }
        node = current.previous();
    }

    None
}

/// The upstream abstract class `XamlIlSelectorNode`: a step of a selector chain.
///
/// `node is XamlIlSelectorNode` is `node.cast::<dyn XamlIlSelectorNode>()`; the concrete node
/// type is reached with `as_any().downcast_ref::<T>()`. As upstream, selector nodes do not visit
/// any children. See the module documentation for what a back end has to do with a node.
pub trait XamlIlSelectorNode: IXamlAstValueNode {
    /// `Previous`: the node this step is applied to, `None` for the first node of a chain.
    fn previous(&self) -> Option<Rc<dyn XamlIlSelectorNode>>;

    /// `TargetType`: the control type the selector is known to match, if any.
    fn target_type(&self) -> Option<Rc<dyn IXamlType>>;
}

impl XamlAstCast for dyn XamlIlSelectorNode {
    fn kind_name() -> &'static str {
        "XamlIlSelectorNode"
    }
    fn cast_from(node: Rc<dyn IXamlAstNode>) -> Option<Rc<Self>> {
        query_node_interface::<dyn XamlIlSelectorNode>(&node)
    }
    fn upcast(this: Rc<Self>) -> Rc<dyn IXamlAstNode> {
        this
    }
}

/// The state of the upstream abstract class `XamlIlSelectorNode`.
pub struct XamlIlSelectorNodeBase {
    node: XamlAstNode,
    pub previous: Option<Rc<dyn XamlIlSelectorNode>>,
    /// `Type`: the selector type; the previous node's type reference unless the node was created
    /// with an explicit selector type.
    pub type_: Rc<dyn IXamlAstTypeReference>,
}

impl XamlIlSelectorNodeBase {
    /// `XamlIlSelectorNode(previous)`: line info and type are taken from `previous`.
    pub fn with_previous(previous: Rc<dyn XamlIlSelectorNode>) -> Self {
        Self {
            node: XamlAstNode::new(&*previous),
            type_: previous.type_(),
            previous: Some(previous),
        }
    }

    /// `XamlIlSelectorNode(null, info, selectorType)`.
    pub fn root(info: &dyn IXamlLineInfo, selector_type: Rc<dyn IXamlType>) -> Self {
        Self {
            node: XamlAstNode::new(info),
            previous: None,
            type_: XamlAstClrTypeReference::new(info, selector_type, false),
        }
    }

    /// The lookup of `EmitCall`: the first method of `Selectors` that is static, has at least
    /// one parameter and satisfies `method`; a `XamlTypeSystemException` when there is none.
    pub fn get_selectors_method(
        types: &FerroXamlIlWellKnownTypes,
        method: impl Fn(&dyn IXamlMethod) -> bool,
    ) -> XamlResult<Rc<dyn IXamlMethod>> {
        types
            .selectors
            .get_method(|m| m.is_static() && !m.parameters().is_empty() && method(m))
    }
}

macro_rules! xaml_il_selector_node {
    ($ty:ident, $name:literal) => {
        xaml_line_info_impl!($ty, base.node);

        impl IXamlAstNode for $ty {
            xaml_ast_node_members!($name, value);

            fn query_interface(self: Rc<Self>, slot: &mut dyn Any) -> bool {
                xaml_query_interface!(self, slot, dyn XamlIlSelectorNode)
            }
        }

        impl IXamlAstValueNode for $ty {
            fn type_(&self) -> Rc<dyn IXamlAstTypeReference> {
                self.base.type_.clone()
            }
        }
    };
}

/// The start of every selector chain: no selector yet.
///
/// # What a back end has to do (upstream IL)
///
/// Push `null` (the `previous` argument of the first builder call). No builder method is called.
pub struct XamlIlSelectorInitialNode {
    pub base: XamlIlSelectorNodeBase,
}

impl XamlIlSelectorInitialNode {
    pub fn new(info: &dyn IXamlLineInfo, selector_type: Rc<dyn IXamlType>) -> Rc<Self> {
        Rc::new(Self {
            base: XamlIlSelectorNodeBase::root(info, selector_type),
        })
    }
}

xaml_il_selector_node!(XamlIlSelectorInitialNode, "XamlIlSelectorInitialNode");

impl XamlIlSelectorNode for XamlIlSelectorInitialNode {
    fn previous(&self) -> Option<Rc<dyn XamlIlSelectorNode>> {
        self.base.previous.clone()
    }
    fn target_type(&self) -> Option<Rc<dyn IXamlType>> {
        None
    }
}

/// Matches a control type: exactly (`Button`, `concrete`) or including derived types
/// (`:is(Button)`).
///
/// # What a back end has to do (upstream IL)
///
/// After `previous`: push the runtime type object of `target_type` (`ldtoken` +
/// `Type.GetTypeFromHandle`) and call [`XamlIlTypeSelector::builder_method`]:
/// `Selectors.OfType(Selector previous, Type type)` when `concrete`, otherwise
/// `Selectors.Is(Selector previous, Type type)`.
pub struct XamlIlTypeSelector {
    pub base: XamlIlSelectorNodeBase,
    pub target_type: Rc<dyn IXamlType>,
    pub concrete: bool,
}

impl XamlIlTypeSelector {
    pub fn new(
        previous: Rc<dyn XamlIlSelectorNode>,
        type_: Rc<dyn IXamlType>,
        concrete: bool,
    ) -> Rc<Self> {
        Rc::new(Self {
            base: XamlIlSelectorNodeBase::with_previous(previous),
            target_type: type_,
            concrete,
        })
    }

    /// The method named `OfType` (concrete) or `Is` with two parameters whose second parameter
    /// is `System.Type`.
    pub fn builder_method(&self, types: &FerroXamlIlWellKnownTypes) -> XamlResult<Rc<dyn IXamlMethod>> {
        let name = if self.concrete { "OfType" } else { "Is" };
        XamlIlSelectorNodeBase::get_selectors_method(types, |m| {
            let parameters = m.parameters();
            m.name() == name && parameters.len() == 2 && parameters[1].is("System", "Type")
        })
    }
}

xaml_il_selector_node!(XamlIlTypeSelector, "XamlIlTypeSelector");

impl XamlIlSelectorNode for XamlIlTypeSelector {
    fn previous(&self) -> Option<Rc<dyn XamlIlSelectorNode>> {
        self.base.previous.clone()
    }
    fn target_type(&self) -> Option<Rc<dyn IXamlType>> {
        Some(self.target_type.clone())
    }
}

/// `XamlIlStringSelector.SelectorType`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum XamlIlStringSelectorType {
    Class,
    Name,
}

impl XamlIlStringSelectorType {
    /// `Enum.ToString()`: the name of the builder method.
    pub fn name(self) -> &'static str {
        match self {
            XamlIlStringSelectorType::Class => "Class",
            XamlIlStringSelectorType::Name => "Name",
        }
    }
}

/// Matches a style class or pseudo-class (`.foo`, `:pointerover`; the pseudo-class keeps its
/// colon) or a control name (`#foo`).
///
/// # What a back end has to do (upstream IL)
///
/// After `previous`: push the string `string` and call
/// [`XamlIlStringSelector::builder_method`]: `Selectors.Class(Selector previous, string name)`
/// or `Selectors.Name(Selector previous, string name)`.
pub struct XamlIlStringSelector {
    pub base: XamlIlSelectorNodeBase,
    pub string: RefCell<String>,
    /// `_type` (private upstream).
    pub selector_type: XamlIlStringSelectorType,
}

impl XamlIlStringSelector {
    pub fn new(
        previous: Rc<dyn XamlIlSelectorNode>,
        type_: XamlIlStringSelectorType,
        s: &str,
    ) -> Rc<Self> {
        Rc::new(Self {
            base: XamlIlSelectorNodeBase::with_previous(previous),
            string: RefCell::new(s.to_string()),
            selector_type: type_,
        })
    }

    pub fn string(&self) -> String {
        self.string.borrow().clone()
    }

    /// The method named after the selector type with two parameters whose second parameter is
    /// `System.String`.
    pub fn builder_method(&self, types: &FerroXamlIlWellKnownTypes) -> XamlResult<Rc<dyn IXamlMethod>> {
        let name = self.selector_type.name();
        XamlIlSelectorNodeBase::get_selectors_method(types, |m| {
            let parameters = m.parameters();
            m.name() == name && parameters.len() == 2 && parameters[1].is("System", "String")
        })
    }
}

xaml_il_selector_node!(XamlIlStringSelector, "XamlIlStringSelector");

impl XamlIlSelectorNode for XamlIlStringSelector {
    fn previous(&self) -> Option<Rc<dyn XamlIlSelectorNode>> {
        self.base.previous.clone()
    }
    fn target_type(&self) -> Option<Rc<dyn IXamlType>> {
        self.base.previous.as_ref().and_then(|p| p.target_type())
    }
}

/// `XamlIlCombinatorSelector.CombinatorSelectorType`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CombinatorSelectorType {
    Child,
    Descendant,
    Template,
}

impl CombinatorSelectorType {
    /// `Enum.ToString()`: the name of the builder method.
    pub fn name(self) -> &'static str {
        match self {
            CombinatorSelectorType::Child => "Child",
            CombinatorSelectorType::Descendant => "Descendant",
            CombinatorSelectorType::Template => "Template",
        }
    }
}

/// A combinator: child (`>`), descendant (white space) or template (`/template/`). The steps
/// after a combinator start without a target type.
///
/// # What a back end has to do (upstream IL)
///
/// After `previous`: call [`XamlIlCombinatorSelector::builder_method`] without further
/// arguments: `Selectors.Child(Selector previous)`, `Selectors.Descendant(Selector previous)`
/// or `Selectors.Template(Selector previous)`.
pub struct XamlIlCombinatorSelector {
    pub base: XamlIlSelectorNodeBase,
    /// `SelectorType` (`_type` upstream).
    pub selector_type: CombinatorSelectorType,
}

impl XamlIlCombinatorSelector {
    pub fn new(previous: Rc<dyn XamlIlSelectorNode>, type_: CombinatorSelectorType) -> Rc<Self> {
        Rc::new(Self {
            base: XamlIlSelectorNodeBase::with_previous(previous),
            selector_type: type_,
        })
    }

    /// The method named after the combinator type with exactly one parameter.
    pub fn builder_method(&self, types: &FerroXamlIlWellKnownTypes) -> XamlResult<Rc<dyn IXamlMethod>> {
        let name = self.selector_type.name();
        XamlIlSelectorNodeBase::get_selectors_method(types, |m| {
            m.name() == name && m.parameters().len() == 1
        })
    }
}

xaml_il_selector_node!(XamlIlCombinatorSelector, "XamlIlCombinatorSelector");

impl XamlIlSelectorNode for XamlIlCombinatorSelector {
    fn previous(&self) -> Option<Rc<dyn XamlIlSelectorNode>> {
        self.base.previous.clone()
    }
    fn target_type(&self) -> Option<Rc<dyn IXamlType>> {
        None
    }
}

/// Negation (`:not(...)`). The argument is a selector chain of its own that starts at the
/// initial node of the whole selector.
///
/// # What a back end has to do (upstream IL)
///
/// After `previous`: emit `argument` converted to the selector type, then call
/// [`XamlIlNotSelector::builder_method`]: `Selectors.Not(Selector previous, Selector argument)`.
pub struct XamlIlNotSelector {
    pub base: XamlIlSelectorNodeBase,
    pub argument: Rc<dyn XamlIlSelectorNode>,
}

impl XamlIlNotSelector {
    pub fn new(
        previous: Rc<dyn XamlIlSelectorNode>,
        argument: Rc<dyn XamlIlSelectorNode>,
    ) -> Rc<Self> {
        Rc::new(Self {
            base: XamlIlSelectorNodeBase::with_previous(previous),
            argument,
        })
    }

    /// The method named `Not` with two parameters whose second parameter is the selector type
    /// (not the overload taking a function).
    pub fn builder_method(&self, types: &FerroXamlIlWellKnownTypes) -> XamlResult<Rc<dyn IXamlMethod>> {
        let selector_type = self.base.type_.get_clr_type()?;
        XamlIlSelectorNodeBase::get_selectors_method(types, |m| {
            let parameters = m.parameters();
            m.name() == "Not" && parameters.len() == 2 && parameters[1].equals(&*selector_type)
        })
    }
}

xaml_il_selector_node!(XamlIlNotSelector, "XamlIlNotSelector");

impl XamlIlSelectorNode for XamlIlNotSelector {
    fn previous(&self) -> Option<Rc<dyn XamlIlSelectorNode>> {
        self.base.previous.clone()
    }
    fn target_type(&self) -> Option<Rc<dyn IXamlType>> {
        self.base.previous.as_ref().and_then(|p| p.target_type())
    }
}

/// `XamlIlNthChildSelector.SelectorType`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum XamlIlNthChildSelectorType {
    NthChild,
    NthLastChild,
}

impl XamlIlNthChildSelectorType {
    /// `Enum.ToString()`: the name of the builder method.
    pub fn name(self) -> &'static str {
        match self {
            XamlIlNthChildSelectorType::NthChild => "NthChild",
            XamlIlNthChildSelectorType::NthLastChild => "NthLastChild",
        }
    }
}

/// `:nth-child(step n + offset)` or `:nth-last-child(step n + offset)`.
///
/// # What a back end has to do (upstream IL)
///
/// After `previous`: push the 32-bit integers `step` and `offset` (in that order) and call
/// [`XamlIlNthChildSelector::builder_method`]:
/// `Selectors.NthChild(Selector previous, int step, int offset)` or
/// `Selectors.NthLastChild(Selector previous, int step, int offset)`.
pub struct XamlIlNthChildSelector {
    pub base: XamlIlSelectorNodeBase,
    /// `_step` (private upstream).
    pub step: i32,
    /// `_offset` (private upstream).
    pub offset: i32,
    /// `_type` (private upstream).
    pub selector_type: XamlIlNthChildSelectorType,
}

impl XamlIlNthChildSelector {
    pub fn new(
        previous: Rc<dyn XamlIlSelectorNode>,
        step: i32,
        offset: i32,
        type_: XamlIlNthChildSelectorType,
    ) -> Rc<Self> {
        Rc::new(Self {
            base: XamlIlSelectorNodeBase::with_previous(previous),
            step,
            offset,
            selector_type: type_,
        })
    }

    /// The method named after the selector type with exactly three parameters.
    pub fn builder_method(&self, types: &FerroXamlIlWellKnownTypes) -> XamlResult<Rc<dyn IXamlMethod>> {
        let name = self.selector_type.name();
        XamlIlSelectorNodeBase::get_selectors_method(types, |m| {
            m.name() == name && m.parameters().len() == 3
        })
    }
}

xaml_il_selector_node!(XamlIlNthChildSelector, "XamlIlNthChildSelector");

impl XamlIlSelectorNode for XamlIlNthChildSelector {
    fn previous(&self) -> Option<Rc<dyn XamlIlSelectorNode>> {
        self.base.previous.clone()
    }
    fn target_type(&self) -> Option<Rc<dyn IXamlType>> {
        self.base.previous.as_ref().and_then(|p| p.target_type())
    }
}

/// The predicate shared by the two property selectors: the method named `PropertyEquals` with
/// three parameters, the second being the non-generic `FerroProperty` and the third `object`.
fn property_equals_method(types: &FerroXamlIlWellKnownTypes) -> XamlResult<Rc<dyn IXamlMethod>> {
    XamlIlSelectorNodeBase::get_selectors_method(types, |m| {
        let parameters = m.parameters();
        m.name() == "PropertyEquals"
            && parameters.len() == 3
            && parameters[1].is("FerroUI", "FerroProperty")
            && parameters[2].is("System", "Object")
    })
}

/// Matches a property value (`[IsEnabled=true]`). The value has already been converted to the
/// property's type.
///
/// # What a back end has to do (upstream IL)
///
/// After `previous`:
///
/// 1. load the static field returned by
///    [`XamlIlPropertyEqualsSelector::resolve_ferro_property_field`] (the registered property
///    behind `property`); when the declaring type of `property` declares no static
///    `<Name>Property` field, fail with the `XamlLoadException` that method returns;
/// 2. emit `value` converted to `object` (value types are boxed);
/// 3. call [`XamlIlPropertyEqualsSelector::builder_method`]:
///    `Selectors.PropertyEquals(Selector previous, FerroProperty property, object value)`.
pub struct XamlIlPropertyEqualsSelector {
    pub base: XamlIlSelectorNodeBase,
    pub property: RefCell<Rc<dyn IXamlProperty>>,
    pub value: RefCell<Rc<dyn IXamlAstValueNode>>,
}

impl XamlIlPropertyEqualsSelector {
    pub fn new(
        previous: Rc<dyn XamlIlSelectorNode>,
        property: Rc<dyn IXamlProperty>,
        value: Rc<dyn IXamlAstValueNode>,
    ) -> Rc<Self> {
        Rc::new(Self {
            base: XamlIlSelectorNodeBase::with_previous(previous),
            property: RefCell::new(property),
            value: RefCell::new(value),
        })
    }

    pub fn property(&self) -> Rc<dyn IXamlProperty> {
        self.property.borrow().clone()
    }

    pub fn value(&self) -> Rc<dyn IXamlAstValueNode> {
        self.value.borrow().clone()
    }

    /// The static field to load, or the `XamlLoadException` upstream throws while emitting.
    pub fn resolve_ferro_property_field(&self) -> XamlResult<Rc<dyn IXamlField>> {
        let property = self.property();
        XamlIlFerroPropertyHelper::try_get_ferro_property_field_for_property(&*property).ok_or_else(
            || {
                XamlError::load_exception(
                    format!(
                        "{} of {} doesn't seem to be an FerroProperty",
                        property.name(),
                        property.declaring_type().get_fqn()
                    ),
                    Some(self),
                )
            },
        )
    }

    pub fn builder_method(&self, types: &FerroXamlIlWellKnownTypes) -> XamlResult<Rc<dyn IXamlMethod>> {
        property_equals_method(types)
    }
}

xaml_il_selector_node!(XamlIlPropertyEqualsSelector, "XamlIlPropertyEqualsSelector");

impl XamlIlSelectorNode for XamlIlPropertyEqualsSelector {
    fn previous(&self) -> Option<Rc<dyn XamlIlSelectorNode>> {
        self.base.previous.clone()
    }
    fn target_type(&self) -> Option<Rc<dyn IXamlType>> {
        self.base.previous.as_ref().and_then(|p| p.target_type())
    }
}

/// Matches an attached property value (`[(Grid.Row)=1]`). The value has already been converted
/// to the property's type.
///
/// # What a back end has to do (upstream IL)
///
/// After `previous`: load the static field `property_filed` (the attached property), emit
/// `value` converted to `object` (value types are boxed) and call
/// [`XamlIlAttachedPropertyEqualsSelector::builder_method`]:
/// `Selectors.PropertyEquals(Selector previous, FerroProperty property, object value)`.
pub struct XamlIlAttachedPropertyEqualsSelector {
    pub base: XamlIlSelectorNodeBase,
    /// `PropertyFiled` (spelled as upstream): the static field holding the attached property.
    pub property_filed: RefCell<Rc<dyn IXamlField>>,
    pub value: RefCell<Rc<dyn IXamlAstValueNode>>,
}

impl XamlIlAttachedPropertyEqualsSelector {
    pub fn new(
        previous: Rc<dyn XamlIlSelectorNode>,
        property_filed: Rc<dyn IXamlField>,
        value: Rc<dyn IXamlAstValueNode>,
    ) -> Rc<Self> {
        Rc::new(Self {
            base: XamlIlSelectorNodeBase::with_previous(previous),
            property_filed: RefCell::new(property_filed),
            value: RefCell::new(value),
        })
    }

    pub fn property_filed(&self) -> Rc<dyn IXamlField> {
        self.property_filed.borrow().clone()
    }

    pub fn value(&self) -> Rc<dyn IXamlAstValueNode> {
        self.value.borrow().clone()
    }

    pub fn builder_method(&self, types: &FerroXamlIlWellKnownTypes) -> XamlResult<Rc<dyn IXamlMethod>> {
        property_equals_method(types)
    }
}

xaml_il_selector_node!(
    XamlIlAttachedPropertyEqualsSelector,
    "XamlIlAttachedPropertyEqualsSelector"
);

impl XamlIlSelectorNode for XamlIlAttachedPropertyEqualsSelector {
    fn previous(&self) -> Option<Rc<dyn XamlIlSelectorNode>> {
        self.base.previous.clone()
    }
    fn target_type(&self) -> Option<Rc<dyn IXamlType>> {
        self.base.previous.as_ref().and_then(|p| p.target_type())
    }
}

/// Alternatives separated by commas (`Button, TextBlock.foo`). Each alternative is a selector
/// chain of its own that starts at the initial node of the whole selector; the node itself has
/// no previous node.
///
/// The target type is the most derived type all alternatives' target types are assignable to
/// (walking up the base types of the first alternative's target type), or none when any
/// alternative has no target type.
///
/// # What a back end has to do (upstream IL)
///
/// * no alternatives: fail with `XamlLoadException("Invalid selector count")` positioned at
///   this node;
/// * one alternative: emit that selector node and nothing else (no builder call);
/// * otherwise: create a new `System.Collections.Generic.List<Selector>` (parameterless
///   constructor; `Selector` is this node's type); for each alternative, in order, duplicate the
///   list reference, emit the alternative converted to the selector type and call
///   `List<Selector>.Add(Selector)`; finally call [`XamlIlOrSelectorNode::builder_method`]:
///   `Selectors.Or(IReadOnlyList<Selector> selectors)` with the list as its only argument.
pub struct XamlIlOrSelectorNode {
    pub base: XamlIlSelectorNodeBase,
    /// `_selectors` (private upstream).
    pub selectors: RefCell<Vec<Rc<dyn XamlIlSelectorNode>>>,
}

impl XamlIlOrSelectorNode {
    pub fn new(info: &dyn IXamlLineInfo, selector_type: Rc<dyn IXamlType>) -> Rc<Self> {
        Rc::new(Self {
            base: XamlIlSelectorNodeBase::root(info, selector_type),
            selectors: RefCell::new(Vec::new()),
        })
    }

    pub fn add(&self, node: Rc<dyn XamlIlSelectorNode>) {
        self.selectors.borrow_mut().push(node);
    }

    pub fn selectors(&self) -> Vec<Rc<dyn XamlIlSelectorNode>> {
        self.selectors.borrow().clone()
    }

    /// The method named `Or` with one parameter whose type name starts with `IReadOnlyList`
    /// (not the overload taking an array).
    pub fn builder_method(&self, types: &FerroXamlIlWellKnownTypes) -> XamlResult<Rc<dyn IXamlMethod>> {
        XamlIlSelectorNodeBase::get_selectors_method(types, |m| {
            let parameters = m.parameters();
            m.name() == "Or"
                && parameters.len() == 1
                && parameters[0].name().starts_with("IReadOnlyList")
        })
    }
}

xaml_il_selector_node!(XamlIlOrSelectorNode, "XamlIlOrSelectorNode");

impl XamlIlSelectorNode for XamlIlOrSelectorNode {
    fn previous(&self) -> Option<Rc<dyn XamlIlSelectorNode>> {
        self.base.previous.clone()
    }

    fn target_type(&self) -> Option<Rc<dyn IXamlType>> {
        let mut result: Option<Rc<dyn IXamlType>> = None;

        for selector in self.selectors.borrow().iter() {
            let target_type = selector.target_type()?;
            match result {
                None => result = Some(target_type),
                Some(_) => {
                    while let Some(current) = result.clone() {
                        if current.is_assignable_from(&*target_type) {
                            break;
                        }
                        result = current.base_type();
                    }
                }
            }
        }

        result
    }
}

/// The nesting selector (`^`): stands for the selector of the parent style.
///
/// # What a back end has to do (upstream IL)
///
/// After `previous`: call [`XamlIlNestingSelector::builder_method`] without further arguments:
/// `Selectors.Nesting(Selector previous)`.
pub struct XamlIlNestingSelector {
    pub base: XamlIlSelectorNodeBase,
    /// The target type of the parent style's scope; `None` when the parent selector has no
    /// target type (`null` upstream).
    pub target_type: Option<Rc<dyn IXamlType>>,
}

impl XamlIlNestingSelector {
    pub fn new(
        previous: Rc<dyn XamlIlSelectorNode>,
        target_type: Option<Rc<dyn IXamlType>>,
    ) -> Rc<Self> {
        Rc::new(Self {
            base: XamlIlSelectorNodeBase::with_previous(previous),
            target_type,
        })
    }

    /// The method named `Nesting` with exactly one parameter.
    pub fn builder_method(&self, types: &FerroXamlIlWellKnownTypes) -> XamlResult<Rc<dyn IXamlMethod>> {
        XamlIlSelectorNodeBase::get_selectors_method(types, |m| {
            m.name() == "Nesting" && m.parameters().len() == 1
        })
    }
}

xaml_il_selector_node!(XamlIlNestingSelector, "XamlIlNestingSelector");

impl XamlIlSelectorNode for XamlIlNestingSelector {
    fn previous(&self) -> Option<Rc<dyn XamlIlSelectorNode>> {
        self.base.previous.clone()
    }
    fn target_type(&self) -> Option<Rc<dyn IXamlType>> {
        self.target_type.clone()
    }
}

/// `values[index]` with the upstream out-of-range failure.
pub(crate) fn value_at<T: Clone>(values: &[T], index: usize) -> XamlResult<T> {
    values.get(index).cloned().ok_or_else(argument_out_of_range)
}
