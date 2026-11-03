//! `{x:Bind}` in the compiler (docs/porting/x-bind.md): the WinUI markup extension mapped onto
//! `XBindExtension` of the XAML runtime library, its root source, and its diagnostics.
//!
//! Not a port of an upstream file. The semantics are those of WinUI's XAML compiler at
//! `7e4b0c2096be402f51cc8f7d203ebf67cff183d0` (the NOTICE.md of `ferroui-markup-xaml` lists the
//! sources); the compiled path is the one of `{CompiledBinding}`:
//!
//! - [`map_type_reference`] (from `FerroBindingExtensionTransformer`): `x:Bind`, a markup
//!   extension of the XAML language namespace, is `XBind` of the framework namespace.
//! - [`prepare_scope`] (from `FerroXamlIlBindingPathParser`, before the path is parsed): inside a
//!   `DataTemplate` the binding reads the item of the template, so the `x:DataType` of the
//!   template becomes the `DataType` of the extension; a template without one, a
//!   `ControlTemplate` and a style are errors.
//! - [`transform_path`] (from `FerroXamlIlBindingPathTransformer`): the path is resolved against
//!   the `DataType`, or else against the type of the root of the document; a first name that is
//!   an `x:Name` of the name scope is that element; the forms of later steps, the modes and
//!   update triggers WinUI rejects and a `TwoWay` path that cannot be written are errors.
//!
//! The run-time loader and the Rust emitter run the same transformers: a diagnostic is a load
//! error of the one and a compile error of the other, with the position of the binding.

use std::rc::Rc;

use ferroui_base::data::core::parsers::Node;
use ferroui_base::metadata::FERRO_XML_NAMESPACE;
use xamlx::ast::{
    IXamlAstNode, IXamlAstPropertyReference, IXamlAstValueNode, IXamlLineInfo, XamlAstClrProperty,
    XamlAstClrTypeReference, XamlAstConstructableObjectNode, XamlAstExtensions, XamlAstNamePropertyReference,
    XamlAstNodeExtensions, XamlAstObjectNode, XamlAstTextNode, XamlAstXamlPropertyValueNode, XamlAstXmlDirective,
    XamlAstXmlTypeReference, XamlConstantNode, XamlPropertyAssignmentNode, XamlTypeExtensionNode,
};
use xamlx::exceptions::{XamlError, XamlResult};
use xamlx::transform::transformers::TypeReferenceResolver;
use xamlx::transform::AstTransformationContext;
use xamlx::type_system::{IXamlType, TypeSystemHelpers, XamlValue};
use xamlx::XamlNamespaces;

use super::{
    BindingExpressionNode, FerroXamlIlWellKnownTypesExtensions, ParsedBindingPathNode, XamlBindingsTransformException,
};
use crate::compiler_extensions::{XamlIlBindingPathElementNode, XamlIlBindingPathHelper, XamlIlBindingPathNode};

/// The namespace of `XBindExtension` (`ferroui_markup_xaml::markup_extensions`).
pub const X_BIND_NAMESPACE: &str = "FerroUI.Markup.Xaml.MarkupExtensions";

/// The type `{x:Bind}` is.
pub const X_BIND_EXTENSION: &str = "XBindExtension";

const DATA_TYPE: &str = "DataType";
const PATH: &str = "Path";
const MODE: &str = "Mode";
const UPDATE_SOURCE_TRIGGER: &str = "UpdateSourceTrigger";

/// Whether `type_` is `XBindExtension`.
pub fn is_x_bind_extension(type_: &Rc<dyn IXamlType>) -> bool {
    (**type_).is(X_BIND_NAMESPACE, X_BIND_EXTENSION)
}

/// Whether `tref` is `x:Bind`: the name `Bind` in the XAML language namespace.
pub fn is_x_bind_reference(tref: &XamlAstXmlTypeReference) -> bool {
    tref.name() == "Bind" && tref.xml_namespace().as_deref() == Some(XamlNamespaces::XAML2006)
}

/// `x:Bind` is `XBind` of the framework namespace (the type `XBindExtension`).
pub fn map_type_reference(tref: &XamlAstXmlTypeReference) {
    tref.is_markup_extension.set(true);
    *tref.xml_namespace.borrow_mut() = Some(FERRO_XML_NAMESPACE.to_string());
    *tref.name.borrow_mut() = "XBind".to_string();
}

fn error(message: impl Into<String>, line_info: &dyn IXamlLineInfo) -> XamlError {
    XamlBindingsTransformException::new(message, line_info, None)
}

/// The name of a property reference, resolved or not.
fn property_name(property: &Rc<dyn IXamlAstPropertyReference>) -> Option<String> {
    if let Some(clr) = property.cast::<XamlAstClrProperty>() {
        return Some(clr.name());
    }
    property.cast::<XamlAstNamePropertyReference>().map(|reference| reference.name())
}

/// The values of the property value `name` of `object`, before property values are assignments.
fn property_values(object: &XamlAstObjectNode, name: &str) -> Option<Vec<Rc<dyn IXamlAstValueNode>>> {
    let children = object.children.borrow().clone();
    for child in children {
        if let Some(value) = child.cast::<XamlAstXamlPropertyValueNode>() {
            if property_name(&value.property()).as_deref() == Some(name) {
                return Some(value.values.borrow().clone());
            }
        }
    }
    None
}

/// The `x:DataType` of a template object: the directive, or the `DataType` property the
/// directive became (`XDataTypeTransformer`).
fn template_data_type(template: &XamlAstObjectNode) -> Option<Rc<dyn IXamlAstValueNode>> {
    let children = template.children.borrow().clone();
    for child in children {
        if let Some(directive) = child.cast::<XamlAstXmlDirective>() {
            if directive.namespace.borrow().as_deref() == Some(XamlNamespaces::XAML2006)
                && *directive.name.borrow() == DATA_TYPE
            {
                return directive.values.borrow().first().cloned();
            }
        }
    }
    property_values(template, DATA_TYPE).and_then(|values| values.first().cloned())
}

/// The scope of an `x:Bind` object node, before its path is parsed (WinUI's
/// `XamlHarvester`): inside a `DataTemplate` the root source is the item of the template, of
/// its `x:DataType`, which is given to the extension as its `DataType`.
pub fn prepare_scope(context: &AstTransformationContext, binding: &Rc<XamlAstObjectNode>) -> XamlResult<()> {
    let types = context.try_get_ferro_types()?;
    for parent in context.parent_nodes() {
        let Some(object) = parent.cast::<XamlAstObjectNode>() else {
            continue;
        };
        let Some(type_) = object.type_.borrow().cast::<XamlAstClrTypeReference>().map(|t| t.type_.clone()) else {
            continue;
        };
        if types.i_data_template.is_assignable_from(&*type_) {
            return set_template_data_type(context, binding, &object);
        }
        if types.control_template.is_assignable_from(&*type_) {
            return Err(error(
                "x:Bind inside a ControlTemplate is not supported yet (docs/porting/x-bind.md, step X5); use TemplateBinding",
                &**binding,
            ));
        }
        if types.setter_base.is_assignable_from(&*type_)
            || types.style.is_assignable_from(&*type_)
            || types.control_theme.is_assignable_from(&*type_)
        {
            return Err(error("Cannot use x:Bind in Styles", &**binding));
        }
    }
    Ok(())
}

fn set_template_data_type(
    context: &AstTransformationContext,
    binding: &Rc<XamlAstObjectNode>,
    template: &Rc<XamlAstObjectNode>,
) -> XamlResult<()> {
    if property_values(binding, DATA_TYPE).is_some() {
        return Ok(());
    }
    let Some(data_type) = template_data_type(template) else {
        return Err(error(
            "DataTemplates containing x:Bind need a DataType to be specified using 'x:DataType'",
            &**binding,
        ));
    };
    let value: Rc<dyn IXamlAstValueNode> = if let Some(text) = data_type.cast::<XamlAstTextNode>() {
        let string = context.configuration().well_known_types().string.clone();
        let node: Rc<dyn IXamlAstValueNode> = XamlAstTextNode::with_type(&**binding, &text.text(), true, Some(string));
        node
    } else if let Some(type_node) = data_type.cast::<XamlTypeExtensionNode>() {
        let node: Rc<dyn IXamlAstValueNode> =
            XamlTypeExtensionNode::new(&**binding, type_node.value(), type_node.system_type().clone());
        node
    } else {
        return Err(error(
            "x:Bind needs the x:DataType of its DataTemplate to be a type name",
            &**binding,
        ));
    };
    let binding_type = binding.type_.borrow().clone().get_clr_type()?;
    let property = binding_type
        .get_all_properties()
        .into_iter()
        .find(|p| p.name() == DATA_TYPE)
        .ok_or_else(|| XamlError::invalid_operation("XBindExtension has no DataType property"))?;
    let reference: Rc<dyn IXamlAstPropertyReference> =
        XamlAstClrProperty::from_property(&**binding, &property, context.configuration())?;
    let assignment: Rc<dyn IXamlAstNode> =
        XamlAstXamlPropertyValueNode::with_values(&**binding, reference, vec![value], true);
    binding.children.borrow_mut().push(assignment);
    Ok(())
}

/// Resolves the path of an `x:Bind` and checks the binding (from the binding path
/// transformer, in place of the resolution of `{CompiledBinding}`).
pub fn transform_path(context: &AstTransformationContext, binding: &Rc<XamlAstConstructableObjectNode>) -> XamlResult<()> {
    let assignments: Vec<Rc<XamlPropertyAssignmentNode>> = binding
        .children
        .borrow()
        .iter()
        .filter_map(|c| c.cast::<XamlPropertyAssignmentNode>())
        .collect();
    let template_type = match assignments.iter().find(|a| a.property.name() == DATA_TYPE) {
        Some(assignment) => Some(assigned_type(context, assignment)?),
        None => None,
    };
    let start_type = match &template_type {
        Some(template_type) => template_type.clone(),
        // WinUI binds to the class of the page; a document without a class binds to the type
        // of its root element.
        None => context.root_object()?.type_().get_clr_type()?,
    };

    if let Some(parsed) = parsed_path(binding, &assignments) {
        check_syntax(&parsed, &**binding)?;
        resolve_named_element(context, &parsed, &start_type, template_type.is_some())?;
    }
    XamlIlBindingPathHelper::update_compiled_binding_extension(
        context,
        binding,
        &|| Ok(start_type.clone()),
        &start_type,
    )?;

    let elements = resolved_elements(binding, &assignments);
    for element in &elements {
        if matches!(
            element,
            XamlIlBindingPathElementNode::ClrMethod(_) | XamlIlBindingPathElementNode::ClrMethodAsCommand(_)
        ) {
            return Err(error(
                "x:Bind to a method (a function binding or an event handler) is not supported yet (docs/porting/x-bind.md, steps X2 and X4)",
                &**binding,
            ));
        }
    }
    check_mode(&assignments, &elements, &**binding)
}

/// The type a `DataType` assignment names.
fn assigned_type(context: &AstTransformationContext, assignment: &XamlPropertyAssignmentNode) -> XamlResult<Rc<dyn IXamlType>> {
    let values = assignment.values.borrow().clone();
    if let [value] = values.as_slice() {
        if let Some(text) = value.cast::<XamlAstTextNode>() {
            return Ok(TypeReferenceResolver::resolve_type_by_xml_name(context, &text.text(), false, &*text, true)?
                .type_
                .clone());
        }
        if let Some(type_node) = value.cast::<XamlTypeExtensionNode>() {
            return type_node.value().get_clr_type();
        }
    }
    Err(error("x:Bind: the DataType must be a type name", assignment))
}

/// The parsed path: the first constructor argument or the `Path` property.
fn parsed_path(
    binding: &XamlAstConstructableObjectNode,
    assignments: &[Rc<XamlPropertyAssignmentNode>],
) -> Option<Rc<ParsedBindingPathNode>> {
    let first = binding.arguments.borrow().first().cloned();
    if let Some(parsed) = first.and_then(|argument| argument.cast::<ParsedBindingPathNode>()) {
        return Some(parsed);
    }
    let path = assignments.iter().find(|a| a.property.name() == PATH)?;
    let value = path.values.borrow().first().cloned();
    value.and_then(|value| value.cast::<ParsedBindingPathNode>())
}

/// The resolved path: what [`parsed_path`] became.
fn resolved_elements(
    binding: &XamlAstConstructableObjectNode,
    assignments: &[Rc<XamlPropertyAssignmentNode>],
) -> Vec<XamlIlBindingPathElementNode> {
    let first = binding.arguments.borrow().first().cloned();
    let path = match first.and_then(|argument| argument.cast::<XamlIlBindingPathNode>()) {
        Some(path) => Some(path),
        None => assignments.iter().find(|a| a.property.name() == PATH).and_then(|a| {
            let value = a.values.borrow().first().cloned();
            value.and_then(|value| value.cast::<XamlIlBindingPathNode>())
        }),
    };
    match path {
        Some(path) => path.elements.borrow().clone(),
        None => Vec::new(),
    }
}

/// The forms of a path that step X1 binds: properties, attached properties and named elements.
/// The binding path grammar of the framework also parses forms that are not x:Bind syntax.
fn check_syntax(parsed: &ParsedBindingPathNode, line_info: &dyn IXamlLineInfo) -> XamlResult<()> {
    for node in parsed.path() {
        let unsupported = match &node {
            BindingExpressionNode::Grammar(
                Node::EmptyExpression | Node::PropertyName { .. } | Node::AttachedPropertyName { .. },
            ) => None,
            BindingExpressionNode::Grammar(Node::TypeCast { .. }) => {
                Some("a cast is not supported yet (docs/porting/x-bind.md, step X2)")
            }
            BindingExpressionNode::Grammar(Node::Indexer { .. }) => {
                Some("an indexer is not supported yet (docs/porting/x-bind.md, step X2)")
            }
            BindingExpressionNode::Grammar(Node::Not) => Some("'!' is not x:Bind syntax; use a function binding"),
            BindingExpressionNode::Grammar(Node::Stream) => Some("the stream operator '^' is not x:Bind syntax"),
            BindingExpressionNode::Grammar(Node::SelfNode) => Some("'$self' is not x:Bind syntax"),
            BindingExpressionNode::Grammar(Node::Ancestor { .. }) => Some("'$parent' is not x:Bind syntax"),
            BindingExpressionNode::Grammar(Node::Name { .. }) => {
                Some("'#name' is not x:Bind syntax; x:Bind names an element without '#'")
            }
            _ => Some("a binding source of this kind is not x:Bind syntax"),
        };
        if let Some(message) = unsupported {
            return Err(error(format!("Invalid binding path: {message}"), line_info));
        }
    }
    Ok(())
}

/// A first name that is not a member of the root type and is an `x:Name` of the name scope is
/// the named element (WinUI generates a field for it). At the root of a document the named
/// element is used; in a template a member of the data type wins, as WinUI assumes the data
/// root field (`BindAssignment_FieldNameElementName_Conflict`).
fn resolve_named_element(
    context: &AstTransformationContext,
    parsed: &ParsedBindingPathNode,
    start_type: &Rc<dyn IXamlType>,
    in_template: bool,
) -> XamlResult<()> {
    let first = parsed.path.borrow().first().cloned();
    let Some(BindingExpressionNode::Grammar(Node::PropertyName { property_name, .. })) = first else {
        return Ok(());
    };
    if in_template && XamlIlBindingPathHelper::has_path_member(start_type, &property_name) {
        return Ok(());
    }
    if !XamlIlBindingPathHelper::is_named_element(context, &property_name)? {
        return Ok(());
    }
    parsed.path.borrow_mut()[0] = BindingExpressionNode::Grammar(Node::Name { name: property_name });
    Ok(())
}

fn integer(value: &XamlValue) -> Option<i64> {
    match value {
        XamlValue::SByte(v) => Some(i64::from(*v)),
        XamlValue::Byte(v) => Some(i64::from(*v)),
        XamlValue::Int16(v) => Some(i64::from(*v)),
        XamlValue::UInt16(v) => Some(i64::from(*v)),
        XamlValue::Int32(v) => Some(i64::from(*v)),
        XamlValue::UInt32(v) => Some(i64::from(*v)),
        XamlValue::Int64(v) => Some(*v),
        XamlValue::UInt64(v) => i64::try_from(*v).ok(),
        _ => None,
    }
}

/// Which of `names` the enumeration value of the assignment `property` is: its text, or the
/// constant the value converter made of it. `None` when it is not assigned or not a literal.
fn enum_member(
    assignments: &[Rc<XamlPropertyAssignmentNode>],
    property: &str,
    names: &[&'static str],
) -> XamlResult<Option<&'static str>> {
    let Some(assignment) = assignments.iter().find(|a| a.property.name() == property) else {
        return Ok(None);
    };
    let value = assignment.values.borrow().first().cloned();
    let Some(value) = value else {
        return Ok(None);
    };
    if let Some(text) = value.cast::<XamlAstTextNode>() {
        let text = text.text();
        let text = text.trim();
        return Ok(names.iter().copied().find(|name| name.eq_ignore_ascii_case(text)));
    }
    if let Some(constant) = value.cast::<XamlConstantNode>() {
        let Some(actual) = integer(&constant.constant) else {
            return Ok(None);
        };
        let enum_type = constant.type_().get_clr_type()?;
        for name in names {
            if let Some(expected) = TypeSystemHelpers::try_get_enum_value(&enum_type, name, false)? {
                if integer(&expected) == Some(actual) {
                    return Ok(Some(*name));
                }
            }
        }
    }
    Ok(None)
}

/// The mode and the update trigger WinUI accepts (`BindAssignment.ValidateUpdateSourceTrigger`),
/// and a `TwoWay` path that ends in something it can write (`BindAssignment_TwoWaySourceNotWritable`).
fn check_mode(
    assignments: &[Rc<XamlPropertyAssignmentNode>],
    elements: &[XamlIlBindingPathElementNode],
    line_info: &dyn IXamlLineInfo,
) -> XamlResult<()> {
    let mode = enum_member(assignments, MODE, &["OneTime", "OneWay", "TwoWay", "Default", "OneWayToSource"])?;
    if matches!(mode, Some("Default" | "OneWayToSource")) {
        return Err(error("x:Bind Mode must be OneTime, OneWay or TwoWay", line_info));
    }
    let two_way = mode == Some("TwoWay");
    match enum_member(assignments, UPDATE_SOURCE_TRIGGER, &["Default", "PropertyChanged", "LostFocus", "Explicit"])? {
        Some("Explicit") => {
            return Err(error("'Explicit' is not a supported value for 'UpdateSourceTrigger'", line_info));
        }
        Some("PropertyChanged" | "LostFocus") if !two_way => {
            return Err(error("'UpdateSourceTrigger' may only be used with binding 'Mode=TwoWay'", line_info));
        }
        _ => {}
    }
    if !two_way {
        return Ok(());
    }
    match elements.last() {
        None => Err(error("A TwoWay x:Bind needs a path that ends in a writable property", line_info)),
        Some(XamlIlBindingPathElementNode::ElementName(element)) => Err(error(
            format!("TwoWay binding path cannot write to the named element '{}'", element.name),
            line_info,
        )),
        Some(XamlIlBindingPathElementNode::ClrProperty(element)) if element.property.setter().is_none() => {
            Err(error(
                format!(
                    "TwoWay binding path cannot write to read-only source member '{}'",
                    element.property.name()
                ),
                line_info,
            ))
        }
        Some(XamlIlBindingPathElementNode::ClrIndexer(element)) if element.property.setter().is_none() => {
            Err(error(
                format!(
                    "TwoWay binding path cannot write to read-only source member '{}'",
                    element.property.name()
                ),
                line_info,
            ))
        }
        _ => Ok(()),
    }
}
