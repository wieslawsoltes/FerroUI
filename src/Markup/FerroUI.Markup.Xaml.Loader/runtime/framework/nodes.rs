//! The evaluation of the value and manipulation nodes of the framework
//! language: what the `Emit` methods of the nodes of the managed original
//! generate, node by node (each node type documents its IL).

use std::rc::Rc;

use ferroui_base::controls::{NameScope, NameScopeRef};
use ferroui_base::metadata::{from_markup_value, MarkupValue};
use ferroui_base::styling::{Selector, Selectors, StyleQueries, StyleQuery};
use ferroui_base::utilities::Uri;
use ferroui_base::{BoxedValue, StyledElement, TypeInfo};
use xamlx::ast::XamlAstExtensions as _;
use xamlx::ast::XamlAstNodeExtensions as _;
use xamlx::ast::{IXamlAstNode, IXamlAstValueNode, IXamlLineInfo};
use xamlx::exceptions::{XamlError, XamlResult};
use xamlx::type_system::{IXamlMethod, IXamlType};

use crate::compiler_extensions::ast_nodes::{
    FerroXamlIlArrayConstantAstNode, FerroXamlIlFerroListConstantAstNode, FerroXamlIlFontFamilyAstNode,
    FerroXamlIlGridLengthAstNode, FerroXamlIlVectorLikeConstantAstNode,
};
use crate::compiler_extensions::group_transformers::NewServiceProviderNode;
use crate::compiler_extensions::transformers::{
    CombinatorSelectorType, EnsureCapacityNode, FerroNameScopeRegistrationXamlIlNode, HandleRootObjectScopeNode,
    InjectServiceProviderNode, XamlIlAndQueryNode, XamlIlAttachedPropertyEqualsSelector, XamlIlCombinatorQuery,
    XamlIlCombinatorSelector, XamlIlHeightQuery, XamlIlNestingSelector, XamlIlNotSelector, XamlIlNthChildSelector,
    XamlIlNthChildSelectorType, XamlIlOrQueryNode, XamlIlOrSelectorNode, XamlIlPropertyEqualsSelector,
    XamlIlQueryInitialNode, XamlIlQueryNode, XamlIlSelectorInitialNode, XamlIlSelectorNode, XamlIlStringQuery,
    XamlIlStringSelector, XamlIlStringSelectorType, XamlIlTypeQuery, XamlIlTypeSelector, XamlIlWidthQuery,
    XamlSourceInfoValueManipulation,
};
use crate::compiler_extensions::{
    XamlIlBindingPathNode, XamlIlFerroClassProperty, XamlIlFerroPropertyFieldNode, XamlIlFerroPropertyHelper,
    XamlIlFerroPropertyNode,
};
use crate::runtime::interpreter::{runtime_error, EvalContext, EvalResult, IXamlNodeEvaluator};
use crate::runtime::type_system::RuntimeType;

use super::binding_path;
use super::helpers::{
    boxed, enum_constant, evaluate_as, evaluate_as_own_type, exact_property_value, ferro_types, load_field,
    load_property, object_of,
};
use super::services::name_scope_of;

/// Evaluates the nodes of the framework language. See
/// [`FRAMEWORK_EVALUATORS`](super::FRAMEWORK_EVALUATORS) for the list.
pub struct FrameworkNodeEvaluator;

impl IXamlNodeEvaluator for FrameworkNodeEvaluator {
    fn evaluate(&self, node: &Rc<dyn IXamlAstNode>, context: &mut EvalContext<'_>) -> Option<XamlResult<EvalResult>> {
        // Constants.
        if let Some(n) = node.cast::<FerroXamlIlVectorLikeConstantAstNode>() {
            return Some(vector_like_constant(&n, context));
        }
        if let Some(n) = node.cast::<FerroXamlIlGridLengthAstNode>() {
            return Some(grid_length(&n, context));
        }
        if let Some(n) = node.cast::<FerroXamlIlFontFamilyAstNode>() {
            return Some(font_family(&n, context));
        }
        if let Some(n) = node.cast::<FerroXamlIlArrayConstantAstNode>() {
            return Some(array_constant(&n, context));
        }
        if let Some(n) = node.cast::<FerroXamlIlFerroListConstantAstNode>() {
            return Some(list_constant(&n, context));
        }
        // Registered properties.
        if let Some(n) = node.cast::<XamlIlFerroPropertyNode>() {
            return Some((|| {
                let field = n.resolve_ferro_property_field()?;
                Ok(EvalResult::value(n.type_().get_clr_type()?, load_field(&field, &*n)?))
            })());
        }
        if let Some(n) = node.cast::<XamlIlFerroPropertyFieldNode>() {
            return Some(load_field(n.field(), &*n).map(|value| EvalResult::value(n.field().field_type(), value)));
        }
        if let Some(n) = node.cast::<XamlIlFerroClassProperty>() {
            return Some((|| {
                let value = context.call_method(n.method(), &[boxed(n.class_name().to_string())], &*n)?;
                Ok(EvalResult::value(n.type_().get_clr_type()?, value))
            })());
        }
        // Selectors and queries.
        if node.cast::<dyn XamlIlSelectorNode>().is_some() {
            return Some(selector(node, context).and_then(|value| {
                let value_node = node.cast::<dyn XamlIlSelectorNode>().ok_or_else(not_a_node)?;
                Ok(EvalResult::value(value_node.type_().get_clr_type()?, value.map(|s| Rc::new(s) as BoxedValue)))
            }));
        }
        if node.cast::<dyn XamlIlQueryNode>().is_some() {
            return Some(query(node, context).and_then(|value| {
                let value_node = node.cast::<dyn XamlIlQueryNode>().ok_or_else(not_a_node)?;
                Ok(EvalResult::value(value_node.type_().get_clr_type()?, value.map(|q| Rc::new(q) as BoxedValue)))
            }));
        }
        // Manipulations.
        if let Some(n) = node.cast::<FerroNameScopeRegistrationXamlIlNode>() {
            return Some(name_scope_registration(&n, context));
        }
        if let Some(n) = node.cast::<HandleRootObjectScopeNode>() {
            return Some(root_object_scope(&n, context));
        }
        if let Some(n) = node.cast::<XamlSourceInfoValueManipulation>() {
            return Some(source_info(&n, context));
        }
        if let Some(n) = node.cast::<EnsureCapacityNode>() {
            return Some(ensure_capacity(&n, context));
        }
        // Service providers.
        if let Some(n) = node.cast::<InjectServiceProviderNode>() {
            return Some((|| {
                let type_ = n.type_().get_clr_type()?;
                let value = context.context_value(&*type_)?;
                Ok(EvalResult::value(type_, value))
            })());
        }
        if let Some(n) = node.cast::<NewServiceProviderNode>() {
            return Some(new_service_provider(&n, context));
        }
        // Compiled binding paths.
        if let Some(n) = node.cast::<XamlIlBindingPathNode>() {
            return Some(binding_path::evaluate(&n, context));
        }
        None
    }
}

fn not_a_node() -> XamlError {
    XamlError::invalid_operation("The node changed its kind while it was evaluated")
}

fn vector_like_constant(
    node: &Rc<FerroXamlIlVectorLikeConstantAstNode>,
    context: &mut EvalContext<'_>,
) -> XamlResult<EvalResult> {
    let arguments: Vec<MarkupValue> = node.values().iter().map(|value| boxed(*value)).collect();
    let value = context.call_constructor(node.constructor(), &arguments, &**node)?;
    Ok(EvalResult::value(node.type_().get_clr_type()?, value))
}

fn grid_length(node: &Rc<FerroXamlIlGridLengthAstNode>, context: &mut EvalContext<'_>) -> XamlResult<EvalResult> {
    let types = node.types();
    let constructor = &types.grid_length_constructor_value_type;
    let unit_type = constructor.parameters().get(1).cloned().ok_or_else(|| {
        XamlError::load_exception("The grid length constructor doesn't take a unit", Some(&**node))
    })?;
    let grid_length = node.grid_length();
    let unit = enum_constant(&unit_type, grid_length.grid_unit_type as i32, &**node)?;
    let value = context.call_constructor(constructor, &[boxed(grid_length.value), unit], &**node)?;
    Ok(EvalResult::value(types.grid_length.clone(), value))
}

fn font_family(node: &Rc<FerroXamlIlFontFamilyAstNode>, context: &mut EvalContext<'_>) -> XamlResult<EvalResult> {
    let types = node.types();
    if context.configuration().type_mappings.uri_context_provider.is_none() {
        return Err(XamlError::internal("NullReferenceException", "XamlLanguageTypeMappings.UriContextProvider is not set"));
    }
    let base_uri: MarkupValue = context.runtime_context()?.base_uri().map(|uri: Uri| Rc::new(uri) as BoxedValue);
    let value = context.call_constructor(
        &types.font_family_constructor_uri_name,
        &[base_uri, boxed(node.text().to_string())],
        &**node,
    )?;
    Ok(EvalResult::value(types.font_family.clone(), value))
}

/// An array evaluates to a `RuntimeArray` (element type and untyped
/// elements), converted where it is passed on.
fn array_constant(node: &Rc<FerroXamlIlArrayConstantAstNode>, context: &mut EvalContext<'_>) -> XamlResult<EvalResult> {
    let mut elements: Vec<MarkupValue> = Vec::with_capacity(node.values().len());
    for value in node.values() {
        elements.push(evaluate_as(value, context, node.element_type())?);
    }
    let array = crate::runtime::type_system::RuntimeArray::new(node.element_type().clone(), elements);
    Ok(EvalResult::value(node.type_().get_clr_type()?, boxed(array)))
}

fn list_constant(
    node: &Rc<FerroXamlIlFerroListConstantAstNode>,
    context: &mut EvalContext<'_>,
) -> XamlResult<EvalResult> {
    let list = context.call_constructor(node.constructor(), &[], &**node)?;
    let capacity = i32::try_from(node.values().len()).unwrap_or(i32::MAX);
    context.call_method(node.list_set_capacity_method(), &[list.clone(), boxed(capacity)], &**node)?;
    for value in node.values() {
        let value = evaluate_as(value, context, node.element_type())?;
        context.call_method(node.list_add_method(), &[list.clone(), value], &**node)?;
    }
    Ok(EvalResult::value(node.type_().get_clr_type()?, list))
}

// --- selectors ----------------------------------------------------------------

fn class_of(type_: &Rc<dyn IXamlType>, line_info: &dyn IXamlLineInfo) -> XamlResult<&'static TypeInfo> {
    type_.as_any().downcast_ref::<RuntimeType>().and_then(RuntimeType::type_info).ok_or_else(|| {
        XamlError::load_exception(
            format!("{} is not a class of the object model: it cannot be used as a control type", type_.get_full_name()),
            Some(line_info),
        )
    })
}

fn previous_selector(
    previous: Option<Rc<dyn XamlIlSelectorNode>>,
    context: &mut EvalContext<'_>,
) -> XamlResult<Option<Selector>> {
    match previous {
        Some(previous) => selector(&previous.as_node(), context),
        None => Ok(None),
    }
}

fn required(selector: Option<Selector>, line_info: &dyn IXamlLineInfo) -> XamlResult<Selector> {
    selector.ok_or_else(|| runtime_error("ArgumentNullException", "Value cannot be null. (Parameter 'previous')", line_info))
}

/// Evaluates a selector node to the selector it builds (`None` is the null
/// of the start of a chain). The builders of the styling system are called
/// with the arguments the IL of each node passes to `Selectors.*`.
fn selector(node: &Rc<dyn IXamlAstNode>, context: &mut EvalContext<'_>) -> XamlResult<Option<Selector>> {
    if node.is::<XamlIlSelectorInitialNode>() {
        return Ok(None);
    }
    if let Some(n) = node.cast::<XamlIlTypeSelector>() {
        let previous = previous_selector(n.base.previous.clone(), context)?;
        let class = class_of(&n.target_type, &*n)?;
        return Ok(Some(match n.concrete {
            true => Selectors::of_type_info(previous, class),
            false => Selectors::is_type_info(previous, class),
        }));
    }
    if let Some(n) = node.cast::<XamlIlStringSelector>() {
        let previous = previous_selector(n.base.previous.clone(), context)?;
        let text = n.string();
        return Ok(Some(match n.selector_type {
            XamlIlStringSelectorType::Class => Selectors::class(previous, &text),
            XamlIlStringSelectorType::Name => Selectors::name(previous, &text),
        }));
    }
    if let Some(n) = node.cast::<XamlIlCombinatorSelector>() {
        let previous = previous_selector(n.base.previous.clone(), context)?;
        return Ok(Some(match n.selector_type {
            CombinatorSelectorType::Child => Selectors::child(previous),
            CombinatorSelectorType::Descendant => Selectors::descendant(previous),
            CombinatorSelectorType::Template => Selectors::template(previous),
        }));
    }
    if let Some(n) = node.cast::<XamlIlNotSelector>() {
        let previous = previous_selector(n.base.previous.clone(), context)?;
        let argument = selector(&n.argument.clone().as_node(), context)?;
        return Ok(Some(Selectors::not(previous, required(argument, &*n)?)));
    }
    if let Some(n) = node.cast::<XamlIlNthChildSelector>() {
        let previous = previous_selector(n.base.previous.clone(), context)?;
        return Ok(Some(match n.selector_type {
            XamlIlNthChildSelectorType::NthChild => Selectors::nth_child(previous, n.step, n.offset),
            XamlIlNthChildSelectorType::NthLastChild => Selectors::nth_last_child(previous, n.step, n.offset),
        }));
    }
    if let Some(n) = node.cast::<XamlIlPropertyEqualsSelector>() {
        let previous = previous_selector(n.base.previous.clone(), context)?;
        let field = n.resolve_ferro_property_field()?;
        let property = load_property(&field, &*n)?;
        let object = context.configuration().well_known_types().object.clone();
        let value = evaluate_as(&n.value(), context, &object)?;
        let value = exact_property_value(property, &value, &*n)?;
        return Ok(Some(Selectors::property_equals_untyped(previous, property, value)));
    }
    if let Some(n) = node.cast::<XamlIlAttachedPropertyEqualsSelector>() {
        let previous = previous_selector(n.base.previous.clone(), context)?;
        let property = load_property(&n.property_filed(), &*n)?;
        let object = context.configuration().well_known_types().object.clone();
        let value = evaluate_as(&n.value(), context, &object)?;
        let value = exact_property_value(property, &value, &*n)?;
        return Ok(Some(Selectors::property_equals_untyped(previous, property, value)));
    }
    if let Some(n) = node.cast::<XamlIlOrSelectorNode>() {
        let alternatives = n.selectors();
        return match alternatives.as_slice() {
            [] => Err(XamlError::load_exception("Invalid selector count", Some(&*n))),
            [only] => selector(&only.clone().as_node(), context),
            _ => {
                let mut list = Vec::with_capacity(alternatives.len());
                for alternative in &alternatives {
                    let alternative = selector(&alternative.clone().as_node(), context)?;
                    list.push(required(alternative, &*n)?);
                }
                Ok(Some(Selectors::or(list)))
            }
        };
    }
    if let Some(n) = node.cast::<XamlIlNestingSelector>() {
        let previous = previous_selector(n.base.previous.clone(), context)?;
        return Ok(Some(Selectors::nesting(previous)));
    }
    Err(XamlError::load_exception(
        format!("Unable to find evaluator for selector node type: {}", node.type_name()),
        Some(&**node),
    ))
}

// --- container queries --------------------------------------------------------

fn previous_query(
    previous: Option<Rc<dyn XamlIlQueryNode>>,
    context: &mut EvalContext<'_>,
) -> XamlResult<Option<StyleQuery>> {
    match previous {
        Some(previous) => query(&previous.as_node(), context),
        None => Ok(None),
    }
}

/// A query the builders of the styling system have no function for (the
/// node types the transformer of the managed original declares but never
/// creates): the builder method is looked up as the IL does and called
/// through the type system.
fn projected_query(
    node: &Rc<dyn IXamlAstNode>,
    method: XamlResult<Rc<dyn IXamlMethod>>,
    previous: Option<StyleQuery>,
    arguments: Vec<MarkupValue>,
    context: &mut EvalContext<'_>,
) -> XamlResult<Option<StyleQuery>> {
    let method = method?;
    let mut all: Vec<MarkupValue> = vec![previous.map(|q| Rc::new(q) as BoxedValue)];
    all.extend(arguments);
    let result = context.call_method(&method, &all, &**node)?;
    Ok(from_markup_value::<StyleQuery>(&result))
}

fn query(node: &Rc<dyn IXamlAstNode>, context: &mut EvalContext<'_>) -> XamlResult<Option<StyleQuery>> {
    if node.is::<XamlIlQueryInitialNode>() {
        return Ok(None);
    }
    if let Some(n) = node.cast::<XamlIlWidthQuery>() {
        let previous = previous_query(n.base.previous.clone(), context)?;
        return Ok(Some(StyleQueries::width(previous, n.operator, n.value)));
    }
    if let Some(n) = node.cast::<XamlIlHeightQuery>() {
        let previous = previous_query(n.base.previous.clone(), context)?;
        return Ok(Some(StyleQueries::height(previous, n.operator, n.value)));
    }
    if let Some(n) = node.cast::<XamlIlTypeQuery>() {
        let previous = previous_query(n.base.previous.clone(), context)?;
        let types = ferro_types(context)?;
        let type_value = Some(context.type_system().type_value(&n.target_type));
        return projected_query(node, n.builder_method(&types, &context.configuration().well_known_types()), previous, vec![type_value], context);
    }
    if let Some(n) = node.cast::<XamlIlStringQuery>() {
        let previous = previous_query(n.base.previous.clone(), context)?;
        let types = ferro_types(context)?;
        return projected_query(node, n.builder_method(&types, &context.configuration().well_known_types()), previous, vec![boxed(n.string())], context);
    }
    if let Some(n) = node.cast::<XamlIlCombinatorQuery>() {
        let previous = previous_query(n.base.previous.clone(), context)?;
        let types = ferro_types(context)?;
        return projected_query(node, n.builder_method(&types), previous, Vec::new(), context);
    }
    let members = if let Some(n) = node.cast::<XamlIlOrQueryNode>() {
        Some((n.queries(), true))
    } else {
        node.cast::<XamlIlAndQueryNode>().map(|n| (n.queries(), false))
    };
    if let Some((members, is_or)) = members {
        return match members.as_slice() {
            [] => Err(XamlError::load_exception("Invalid query count", Some(&**node))),
            [only] => query(&only.clone().as_node(), context),
            _ => {
                let mut list = Vec::with_capacity(members.len());
                for member in &members {
                    let member = query(&member.clone().as_node(), context)?;
                    list.push(member.ok_or_else(|| {
                        runtime_error("ArgumentNullException", "A member of a query list is null", &**node)
                    })?);
                }
                Ok(Some(if is_or { StyleQueries::or(list) } else { StyleQueries::and(list) }))
            }
        };
    }
    Err(XamlError::load_exception(
        format!("Unable to find evaluator for query node type: {}", node.type_name()),
        Some(&**node),
    ))
}

// --- manipulations ------------------------------------------------------------

/// `context.FerroNameScope.Register(name, target)`.
fn name_scope_registration(
    node: &Rc<FerroNameScopeRegistrationXamlIlNode>,
    context: &mut EvalContext<'_>,
) -> XamlResult<EvalResult> {
    let target = context.target()?;
    let scope = name_scope_of(context.runtime_context()?);
    let name = evaluate_as_own_type(&node.name(), context)?;
    let scope = scope.ok_or_else(|| {
        runtime_error("NullReferenceException", "The runtime context has no name scope to register a name in", &**node)
    })?;
    let name = from_markup_value::<String>(&name).ok_or_else(|| {
        runtime_error("ArgumentNullException", "Value cannot be null. (Parameter 'name')", &**node)
    })?;
    let element = object_of(&target, &**node)?;
    // A failed registration (a duplicate name, a completed scope) is the exception
    // of the managed original: a load error at the registration.
    scope.try_register(&name, element).map_err(|e| {
        let kind = match e {
            ferroui_base::controls::NameScopeError::Completed => "InvalidOperationException",
            ferroui_base::controls::NameScopeError::DuplicateName(_) => "ArgumentException",
        };
        runtime_error(kind, e.to_string(), &**node)
    })?;
    Ok(EvalResult::void())
}

/// `if (root is StyledElement s) NameScope.SetNameScope(s, scope); scope.Complete();`.
fn root_object_scope(node: &Rc<HandleRootObjectScopeNode>, context: &mut EvalContext<'_>) -> XamlResult<EvalResult> {
    let target = context.target()?;
    let scope = name_scope_of(context.runtime_context()?);
    let styled = target
        .as_ref()
        .and_then(|value| ferroui_base::data::core::ValueTypes::as_object(&**value))
        .and_then(|object| object.cast::<StyledElement>());
    if let Some(styled) = styled {
        NameScope::set_name_scope(&styled, scope.clone().map(NameScopeRef));
    }
    let scope = scope.ok_or_else(|| {
        runtime_error("NullReferenceException", "The runtime context has no name scope to complete", &**node)
    })?;
    scope.complete();
    Ok(EvalResult::void())
}

/// `new XamlSourceInfo(line, position, document)`.
pub(crate) fn create_source_info(
    constructor: &Rc<dyn xamlx::type_system::IXamlConstructor>,
    line: i32,
    position: i32,
    document: Option<&str>,
    context: &mut EvalContext<'_>,
    line_info: &dyn IXamlLineInfo,
) -> XamlResult<MarkupValue> {
    let document: MarkupValue = document.map(|d| Rc::new(d.to_string()) as BoxedValue);
    context.call_constructor(constructor, &[boxed(line), boxed(position), document], line_info)
}

/// `XamlSourceInfo.SetXamlSourceInfo(target, new XamlSourceInfo(line, position, document))`.
fn source_info(node: &Rc<XamlSourceInfoValueManipulation>, context: &mut EvalContext<'_>) -> XamlResult<EvalResult> {
    let target = context.target()?;
    let info = create_source_info(
        &node.types.xaml_source_info_constructor,
        node.line(),
        node.position(),
        node.document.as_deref(),
        context,
        &**node,
    )?;
    context.call_method(&node.types.xaml_source_info_setter, &[target, info], &**node)?;
    Ok(EvalResult::void())
}

/// `if (x is ResourceDictionary d) d.EnsureCapacity(d.Count + n)`.
fn ensure_capacity(node: &Rc<EnsureCapacityNode>, context: &mut EvalContext<'_>) -> XamlResult<EvalResult> {
    let target = context.target()?;
    let types = ferro_types(context)?;
    let resources = match &node.resources_getter {
        Some(getter) => context.call_method(getter, &[target], &**node)?,
        None => target,
    };
    if !context.type_system().is_instance(&resources, &*types.resource_dictionary) {
        return Ok(EvalResult::void());
    }
    let count = context.call_method(&types.resource_dictionary_get_count, &[resources.clone()], &**node)?;
    let count = from_markup_value::<i32>(&count).ok_or_else(|| {
        runtime_error("InvalidCastException", "The count of a resource dictionary is not a 32-bit integer", &**node)
    })?;
    let capacity = count.wrapping_add(node.capacity);
    context.call_method(&types.resource_dictionary_ensure_capacity, &[resources, boxed(capacity)], &**node)?;
    Ok(EvalResult::void())
}

/// `XamlIlRuntimeHelpers.CreateRootServiceProviderV3(context)`.
fn new_service_provider(node: &Rc<NewServiceProviderNode>, context: &mut EvalContext<'_>) -> XamlResult<EvalResult> {
    let type_ = node.type_().get_clr_type()?;
    let types = ferro_types(context)?;
    let method = types
        .runtime_helpers
        .get_method(|m| m.name() == NewServiceProviderNode::CREATE_ROOT_SERVICE_PROVIDER_METHOD_NAME)?;
    let service_provider = context.configuration().type_mappings.service_provider()?;
    let context_value = context.context_value(&*service_provider)?;
    let value = context.call_method(&method, &[context_value], &**node)?;
    Ok(EvalResult::value(type_, value))
}

/// The provide-value target property of a property (the
/// `ProvideValueTargetPropertyEmitter` of the language): the registered
/// property behind it, or the description of a plain property the declaring
/// type declares. `None` leaves the name of the property.
pub fn provide_value_target_property(
    context: &mut EvalContext<'_>,
    property: &Rc<xamlx::ast::XamlAstClrProperty>,
) -> XamlResult<Option<MarkupValue>> {
    use crate::compiler_extensions::XamlIlProvideValueTargetProperty;
    match XamlIlFerroPropertyHelper::try_get_provide_value_target(property) {
        Some(XamlIlProvideValueTargetProperty::FerroProperty(field)) => {
            let property = load_property(&field, &**property)?;
            Ok(Some(boxed(property)))
        }
        Some(XamlIlProvideValueTargetProperty::ClrProperty(clr)) => {
            let info = binding_path::property_info(context, &clr, None, &**property)?;
            Ok(Some(boxed(info)))
        }
        None => Ok(None),
    }
}
