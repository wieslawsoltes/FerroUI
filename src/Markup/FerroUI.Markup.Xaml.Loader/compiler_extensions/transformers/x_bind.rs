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
//! - [`parse_path`] (from `FerroXamlIlBindingPathParser`): the path is parsed in the grammar of
//!   WinUI's compiled bindings ([`super::x_bind_path`]), not in the one of `{Binding}`.
//! - [`transform_path`] (from `FerroXamlIlBindingPathTransformer`): the path is resolved against
//!   the `DataType`, or else against the type of the root of the document, element by element
//!   with the resolution of `{CompiledBinding}`: a first name that is an `x:Name` of the name
//!   scope is that element; static members, indexers, attached properties and interface
//!   members, casts, and a function at the end of the path with its arguments and its
//!   `BindBack` (step X2). The forms WinUI rejects, the modes and update triggers it rejects, a
//!   function that does not match its call or its target, and a `TwoWay` path that cannot be
//!   written are errors.
//!
//! The run-time loader and the Rust emitter run the same transformers: a diagnostic is a load
//! error of the one and a compile error of the other, with the position of the binding.

use std::cell::{Cell, RefCell};
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
use xamlx::type_system::{IXamlMethod, IXamlType, TypeSystemHelpers, XamlValue};
use xamlx::XamlNamespaces;

use super::x_bind_path::{self, XBindArgument, XBindFunctionOwner, XBindPath};
use super::{
    BindingExpressionNode, FerroXamlIlWellKnownTypesExtensions, ParsedBindingPathNode, XamlBindingsTransformException,
};
use crate::compiler_extensions::{
    TypeCastPathElementNode, XBindBindBack, XBindFunctionArgument, XBindFunctionPathElementNode, XBindLiteral,
    XamlIlBindingPathElementNode, XamlIlBindingPathHelper, XamlIlBindingPathNode, XamlIlClrPropertyPathElementNode,
    XamlIlFerroPropertyPropertyPathElementNode,
};

/// The namespace of `XBindExtension` (`ferroui_markup_xaml::markup_extensions`).
pub const X_BIND_NAMESPACE: &str = "FerroUI.Markup.Xaml.MarkupExtensions";

/// The type `{x:Bind}` is.
pub const X_BIND_EXTENSION: &str = "XBindExtension";

const DATA_TYPE: &str = "DataType";
const PATH: &str = "Path";
const MODE: &str = "Mode";
const UPDATE_SOURCE_TRIGGER: &str = "UpdateSourceTrigger";
const BIND_BACK: &str = "BindBack";

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

/// Parses the path of an `{x:Bind}` (from `FerroXamlIlBindingPathParser`, in place of the
/// grammar of `{Binding}`): one node holding the syntax tree, which [`transform_path`] resolves.
pub fn parse_path(text: &Rc<XamlAstTextNode>) -> XamlResult<Vec<BindingExpressionNode>> {
    let path_text = text.text();
    let path = x_bind_path::parse(&path_text)
        .map_err(|message| error(format!("Invalid binding path '{path_text}' : {message}"), &**text))?;
    Ok(vec![BindingExpressionNode::XBind(Rc::new(path))])
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
    let resolver = Resolver {
        context,
        line_info: &**binding,
        start_type,
        in_template: template_type.is_some(),
        path_type: context.try_get_ferro_types()?.compiled_binding_path.clone(),
    };

    let mut elements = Vec::new();
    if let Some(parsed) = parsed_path(binding, &assignments) {
        let syntax = match parsed.path().as_slice() {
            [BindingExpressionNode::XBind(syntax)] => syntax.clone(),
            _ => return Err(XamlError::invalid_operation("The path of an x:Bind is not an x:Bind path")),
        };
        elements = resolver.resolve(&syntax)?.elements;
        let resolved: Rc<dyn IXamlAstValueNode> = resolver.path_node(elements.clone());
        replace_parsed_path(binding, &assignments, resolved);
    }
    check_binding(&resolver, &assignments, &elements, &**binding)
}

/// Puts the resolved path where the parsed one is: the first constructor argument or the
/// value of `Path`.
fn replace_parsed_path(
    binding: &XamlAstConstructableObjectNode,
    assignments: &[Rc<XamlPropertyAssignmentNode>],
    resolved: Rc<dyn IXamlAstValueNode>,
) {
    let first = binding.arguments.borrow().first().cloned();
    if first.is_some_and(|argument| argument.cast::<ParsedBindingPathNode>().is_some()) {
        binding.arguments.borrow_mut()[0] = resolved;
        return;
    }
    if let Some(path) = assignments.iter().find(|a| a.property.name() == PATH) {
        let mut values = path.values.borrow_mut();
        if values.first().is_some_and(|value| value.cast::<ParsedBindingPathNode>().is_some()) {
            values[0] = resolved;
        }
    }
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

/// A resolved path (or the start of one): its elements and the type of its value.
struct Resolved {
    elements: Vec<XamlIlBindingPathElementNode>,
    type_: Rc<dyn IXamlType>,
}

impl Resolved {
    fn push(mut self, element: XamlIlBindingPathElementNode) -> Self {
        self.type_ = element.type_();
        self.elements.push(element);
        self
    }
}

/// Resolves the syntax of an `{x:Bind}` path ([`XBindPath`]) into the elements of a compiled
/// path. A step that a path of `{CompiledBinding}` also has is resolved by the resolution of
/// `{CompiledBinding}` (`XamlIlBindingPathHelper::transform_binding_path`, one grammar node on
/// the type of the step before it); the forms of WinUI's grammar it does not have are
/// resolved here: static members, string keys, members of an interface the value is cast to,
/// casts of a whole path, and functions with their arguments and their `BindBack`
/// (`BindPathListener.cs` of WinUI's XAML compiler).
struct Resolver<'c> {
    context: &'c AstTransformationContext,
    line_info: &'c dyn IXamlLineInfo,
    start_type: Rc<dyn IXamlType>,
    in_template: bool,
    path_type: Rc<dyn IXamlType>,
}

impl Resolver<'_> {
    fn error(&self, message: impl Into<String>) -> XamlError {
        error(message, self.line_info)
    }

    /// The root source: no elements, of the start type.
    fn root(&self) -> Resolved {
        Resolved { elements: Vec::new(), type_: self.start_type.clone() }
    }

    /// A resolved path as a path node of its own (an argument, the owner of a `BindBack`).
    fn path_node(&self, elements: Vec<XamlIlBindingPathElementNode>) -> Rc<XamlIlBindingPathNode> {
        XamlIlBindingPathNode::new(self.line_info, self.path_type.clone(), Vec::new(), elements)
    }

    /// The type a type name of the path names (`ns:Type`, or `Type` in the default namespace).
    fn type_by_name(&self, type_name: &str) -> XamlResult<Rc<dyn IXamlType>> {
        let qualified = match type_name.contains(':') {
            true => type_name.to_string(),
            false => format!(":{type_name}"),
        };
        Ok(TypeReferenceResolver::resolve_type_by_xml_name(self.context, &qualified, false, self.line_info, true)?
            .type_
            .clone())
    }

    /// One node of the grammar of `{CompiledBinding}`, resolved on `owner`.
    fn grammar(&self, owner: &Rc<dyn IXamlType>, node: Node) -> XamlResult<XamlIlBindingPathElementNode> {
        let path = XamlIlBindingPathHelper::transform_binding_path(
            self.context,
            self.line_info,
            &|| Ok(owner.clone()),
            owner,
            &[BindingExpressionNode::Grammar(node)],
        )?;
        let element = path.elements.borrow().first().cloned();
        element.ok_or_else(|| XamlError::invalid_operation("A step of an x:Bind path resolved to no element"))
    }

    fn resolve(&self, path: &XBindPath) -> XamlResult<Resolved> {
        match path {
            XBindPath::Identifier(name) => self.first(name),
            XBindPath::Member(path, name) => {
                let owner = self.resolve(path)?;
                self.member(owner, name)
            }
            XBindPath::Static { type_name, member } => self.static_member(type_name, member),
            XBindPath::Index(path, index) => {
                let owner = self.resolve(path)?;
                let element = self.grammar(&owner.type_, Node::Indexer { arguments: vec![index.to_string()] })?;
                Ok(owner.push(element))
            }
            XBindPath::Key(path, key) => {
                let owner = self.resolve(path)?;
                let element = self.grammar(&owner.type_, Node::Indexer { arguments: vec![key.clone()] })?;
                Ok(owner.push(element))
            }
            XBindPath::Attached { path, type_name, property } => {
                let owner = match path {
                    Some(path) => self.resolve(path)?,
                    None => self.root(),
                };
                self.attached(owner, type_name, property)
            }
            XBindPath::Cast { type_name, path } => {
                let owner = match path {
                    Some(path) => self.resolve(path)?,
                    None => self.root(),
                };
                let target = self.type_by_name(type_name)?;
                self.cast(owner, target)
            }
            XBindPath::Function { owner, name, arguments } => self.function(owner, name, arguments),
        }
    }

    /// The first name of a path: a member of the root source, or the named element. At the root
    /// of a document the named element is used (WinUI generates a field for it); in a template
    /// a member of the data type wins, as WinUI assumes the data root field
    /// (`BindAssignment_FieldNameElementName_Conflict`).
    fn first(&self, name: &str) -> XamlResult<Resolved> {
        let member_wins = self.in_template && XamlIlBindingPathHelper::has_path_member(&self.start_type, name);
        if !member_wins && XamlIlBindingPathHelper::is_named_element(self.context, name)? {
            let element = self.grammar(&self.start_type, Node::Name { name: name.to_string() })?;
            return Ok(self.root().push(element));
        }
        self.member(self.root(), name)
    }

    /// A member of the value of `owner`.
    fn member(&self, owner: Resolved, name: &str) -> XamlResult<Resolved> {
        let element =
            self.grammar(&owner.type_, Node::PropertyName { accepts_null: false, property_name: name.to_string() })?;
        if matches!(element, XamlIlBindingPathElementNode::ClrMethod(_)) {
            return Err(self.error(format!(
                "x:Bind to the method '{name}' without a call is an event binding, which is not supported yet (docs/porting/x-bind.md, step X4); a function binding calls it: {name}()"
            )));
        }
        // `DataContext` after a named element has the data context type in scope where the
        // element is registered, as in a path of `{CompiledBinding}`.
        let data_context_type = owner.elements.last().and_then(|last| last.data_context_type());
        let typed_data_context = match (&element, data_context_type) {
            (XamlIlBindingPathElementNode::FerroProperty(property), Some(data_context_type))
                if property.field.name() == "DataContextProperty" =>
            {
                Some(XamlIlBindingPathElementNode::FerroProperty(Rc::new(XamlIlFerroPropertyPropertyPathElementNode {
                    field: property.field.clone(),
                    type_: data_context_type,
                    accepts_null: property.accepts_null,
                })))
            }
            _ => None,
        };
        Ok(owner.push(typed_data_context.unwrap_or(element)))
    }

    /// `ns:Type.Member`: a static property (WinUI's `StaticRootStep`). Its element reads the
    /// property whatever its source is; it is the first of the path, so its source is the
    /// root source.
    fn static_member(&self, type_name: &str, member: &str) -> XamlResult<Resolved> {
        let type_ = self.type_by_name(type_name)?;
        let properties = XamlIlBindingPathHelper::path_properties(&type_, member);
        let static_property = properties.iter().find(|p| p.getter().is_some_and(|getter| getter.is_static())).cloned();
        let Some(property) = static_property else {
            return Err(match properties.is_empty() {
                true => self.error(format!("Property '{member}' not found on type '{}'.", type_.full_name())),
                false => self.error(format!("Expecting '{member}' on type '{}' to be a static property.", type_.full_name())),
            });
        };
        let element = XamlIlBindingPathElementNode::ClrProperty(Rc::new(XamlIlClrPropertyPathElementNode {
            property,
            accepts_null: false,
            emit_typed: Cell::new(false),
        }));
        Ok(Resolved { elements: vec![element.clone()], type_: element.type_() })
    }

    /// `path.(ns:Owner.Property)`: an attached property of `Owner`, or else the member
    /// `Property` of the value cast to `Owner` (an interface member,
    /// `BindPathListener.ExitPathDotAttached`).
    fn attached(&self, owner: Resolved, type_name: &str, property: &str) -> XamlResult<Resolved> {
        let owner_type = self.type_by_name(type_name)?;
        let field_name = format!("{property}Property");
        let registered = owner_type
            .get_all_fields()
            .into_iter()
            .any(|field| field.is_static() && field.is_public() && field.name() == field_name);
        if registered {
            let (namespace, name) = match type_name.split_once(':') {
                Some((namespace, name)) => (namespace.to_string(), name.to_string()),
                None => (String::new(), type_name.to_string()),
            };
            let element = self.grammar(
                &owner.type_,
                Node::AttachedPropertyName { accepts_null: false, namespace, type_name: name, property_name: property.to_string() },
            )?;
            return Ok(owner.push(element));
        }
        // The member of an interface is read through the metadata of the interface: a cast to it
        // is a type test the values of a contract do not pass, so only a class is cast to.
        let owner = match owner_type.is_interface() {
            true => Resolved { elements: owner.elements, type_: owner_type.clone() },
            false => self.cast(owner, owner_type.clone())?,
        };
        self.member(owner, property)
    }

    /// A cast of the value of `owner` to `target` (WinUI's `CastStep`): the value when it is of
    /// the type, else null.
    fn cast(&self, owner: Resolved, target: Rc<dyn IXamlType>) -> XamlResult<Resolved> {
        let source = &owner.type_;
        let castable = target.is_assignable_from(&**source)
            || source.is_assignable_from(&*target)
            || source.is_interface()
            || target.is_interface()
            || source.is("System", "Object");
        if !castable {
            return Err(self.error(format!(
                "Unable to cast type '{}' to '{}'.  Use a converter or function binding to change the type",
                source.full_name(),
                target.full_name()
            )));
        }
        Ok(owner.push(XamlIlBindingPathElementNode::TypeCast(Rc::new(TypeCastPathElementNode { type_: target }))))
    }

    /// The methods named `name` of `owner_type`, and the one of `count` parameters
    /// (`BindPathListener.CreateFunctionStep`: the overload of the number of arguments).
    fn method(&self, owner_type: &Rc<dyn IXamlType>, name: &str, count: usize) -> XamlResult<Rc<dyn IXamlMethod>> {
        let methods = XamlIlBindingPathHelper::path_methods(owner_type, name);
        if methods.is_empty() {
            return Err(match XamlIlBindingPathHelper::has_path_member(owner_type, name) {
                true => self.error(format!("Expecting a method: '{name}' of type '{}' is not a function.", owner_type.full_name())),
                false => self.error(format!("Function '{name}' not found on type '{}'.", owner_type.full_name())),
            });
        }
        if let Some(method) = methods.iter().find(|method| method.parameters().len() == count) {
            return Ok(method.clone());
        }
        Err(match methods.as_slice() {
            [method] => self.error(format!(
                "Mismatched function parameter count: '{name}' takes {} parameters, the path passes {count}.",
                method.parameters().len()
            )),
            _ => self.error(format!("Cannot find an overload of '{name}' that takes {count} parameters.")),
        })
    }

    /// A function: the owner, the method, and its arguments checked against its parameters.
    fn function(&self, owner: &XBindFunctionOwner, name: &str, arguments: &[XBindArgument]) -> XamlResult<Resolved> {
        let (owner, owner_type, is_static_owner) = match owner {
            XBindFunctionOwner::Root => (self.root(), self.start_type.clone(), false),
            XBindFunctionOwner::Path(path) => {
                let owner = self.resolve(path)?;
                let owner_type = owner.type_.clone();
                (owner, owner_type, false)
            }
            XBindFunctionOwner::Static(type_name) => (self.root(), self.type_by_name(type_name)?, true),
        };
        let method = self.method(&owner_type, name, arguments.len())?;
        if is_static_owner && !method.is_static() {
            return Err(self.error(format!(
                "Expecting '{name}' on type '{}' to be a static function.",
                owner_type.full_name()
            )));
        }
        let well_known_types = self.context.configuration().well_known_types();
        if method.return_type().equals(&*well_known_types.void) {
            return Err(self.error(format!(
                "Function return type 'System.Void' must match binding target type: '{name}' returns nothing"
            )));
        }
        let parameters = method.parameters();
        let mut resolved_arguments = Vec::with_capacity(arguments.len());
        for (index, (argument, parameter)) in arguments.iter().zip(parameters.iter()).enumerate() {
            resolved_arguments.push(self.argument(argument, parameter, index + 1, name)?);
        }
        let element = XamlIlBindingPathElementNode::XBindFunction(Rc::new(XBindFunctionPathElementNode {
            method,
            owner_type,
            arguments: resolved_arguments,
            bind_back: RefCell::new(None),
        }));
        Ok(owner.push(element))
    }

    /// An argument of a function: a path read from the root source, of a type its parameter
    /// accepts, or a literal of the type of its parameter (`FunctionParam.SetParameterInfo`).
    fn argument(
        &self,
        argument: &XBindArgument,
        parameter: &Rc<dyn IXamlType>,
        position: usize,
        function: &str,
    ) -> XamlResult<XBindFunctionArgument> {
        let mismatch = |detail: String| {
            self.error(format!("Invalid or mismatched parameter at position '{position}' of '{function}': {detail}"))
        };
        if let XBindArgument::Path(path) = argument {
            let resolved = self.resolve(path)?;
            if !accepts(parameter, &resolved.type_) {
                return Err(mismatch(format!(
                    "the path '{path}' is a '{}', the parameter a '{}'",
                    resolved.type_.full_name(),
                    parameter.full_name()
                )));
            }
            return Ok(XBindFunctionArgument::Path(self.path_node(resolved.elements)));
        }
        match literal(argument, parameter) {
            Some(literal) => Ok(XBindFunctionArgument::Literal(literal)),
            None => Err(mismatch(format!("{argument} cannot be passed as a '{}'", parameter.full_name()))),
        }
    }

    /// The `BindBack` of a `TwoWay` binding through a function: a method of one parameter that
    /// accepts the value of the target (`BindAssignment.ValidateBindBackAssignment`).
    fn bind_back(&self, text: &str, target_type: Option<&Rc<dyn IXamlType>>) -> XamlResult<XBindBindBack> {
        let syntax = x_bind_path::parse(text)
            .map_err(|message| self.error(format!("Invalid BindBack path '{text}' : {message}")))?;
        let (owner, owner_type, is_static_owner, name) = match &syntax {
            XBindPath::Identifier(name) => (None, self.start_type.clone(), false, name.clone()),
            XBindPath::Member(path, name) => {
                let owner = self.resolve(path)?;
                let owner_type = owner.type_.clone();
                (Some(self.path_node(owner.elements)), owner_type, false, name.clone())
            }
            XBindPath::Static { type_name, member } => (None, self.type_by_name(type_name)?, true, member.clone()),
            _ => return Err(self.error("BindBack must point to a method")),
        };
        let methods = XamlIlBindingPathHelper::path_methods(&owner_type, &name);
        if methods.is_empty() {
            return Err(self.error(format!(
                "BindBack must point to a method: '{name}' is not a method of type '{}'",
                owner_type.full_name()
            )));
        }
        let accepted = |method: &Rc<dyn IXamlMethod>| match method.parameters().as_slice() {
            [parameter] => target_type.is_none_or(|target| accepts(parameter, target)),
            _ => false,
        };
        let Some(method) = methods.iter().find(|method| accepted(*method)).cloned() else {
            let target = target_type.map(|target| target.full_name()).unwrap_or_else(|| "System.Object".to_string());
            return Err(self.error(format!("BindBack must point to a method that takes one argument of type '{target}'")));
        };
        if is_static_owner && !method.is_static() {
            return Err(self.error(format!(
                "Expecting '{name}' on type '{}' to be a static function.",
                owner_type.full_name()
            )));
        }
        let value_type = method.parameters()[0].clone();
        // The owner of a static method is not read.
        let owner = if method.is_static() { None } else { owner };
        Ok(XBindBindBack { owner, method, value_type })
    }
}

/// Whether a parameter of type `parameter` accepts a value of type `argument`: an object
/// parameter, an assignable type, or the value type of a nullable parameter.
fn accepts(parameter: &Rc<dyn IXamlType>, argument: &Rc<dyn IXamlType>) -> bool {
    if parameter.is("System", "Object") || parameter.is_assignable_from(&**argument) {
        return true;
    }
    is_nullable(parameter) && parameter.generic_arguments().first().is_some_and(|inner| inner.equals(&**argument))
}

fn is_nullable(type_: &Rc<dyn IXamlType>) -> bool {
    type_.generic_type_definition().is_some_and(|definition| definition.full_name() == "System.Nullable`1")
}

/// A literal argument as the value of the type of its parameter; `None` when the parameter
/// does not take it (WinUI: `InvalidParameter`).
fn literal(argument: &XBindArgument, parameter: &Rc<dyn IXamlType>) -> Option<XBindLiteral> {
    let is = |name: &str| parameter.is("System", name);
    let object = is("Object");
    match argument {
        XBindArgument::Path(_) => None,
        XBindArgument::Null => (!parameter.is_value_type() || is_nullable(parameter)).then_some(XBindLiteral::Null),
        XBindArgument::Bool(value) => (is("Boolean") || object).then_some(XBindLiteral::Boolean(*value)),
        XBindArgument::String(text) => (is("String") || object).then(|| XBindLiteral::String(text.clone())),
        XBindArgument::Number(text) => {
            let integral = !text.contains('.');
            if is("Int32") || (object && integral) {
                text.parse().ok().map(XBindLiteral::Int32)
            } else if is("Int64") {
                text.parse().ok().map(XBindLiteral::Int64)
            } else if is("Double") || object {
                text.parse().ok().map(XBindLiteral::Double)
            } else if is("Single") {
                text.parse().ok().map(XBindLiteral::Single)
            } else {
                None
            }
        }
    }
}

/// The type of the property the binding is assigned to.
fn target_type(context: &AstTransformationContext) -> Option<Rc<dyn IXamlType>> {
    let assignment = context.parent_nodes().into_iter().find_map(|node| node.cast::<XamlPropertyAssignmentNode>())?;
    assignment.property.getter().map(|getter| getter.return_type())
}

/// The text of the assignment `property`: its text, or the string constant it became.
fn text_value(assignments: &[Rc<XamlPropertyAssignmentNode>], property: &str) -> XamlResult<Option<String>> {
    let Some(assignment) = assignments.iter().find(|a| a.property.name() == property) else {
        return Ok(None);
    };
    let value = assignment.values.borrow().first().cloned();
    if let Some(text) = value.as_ref().and_then(|value| value.cast::<XamlAstTextNode>()) {
        return Ok(Some(text.text()));
    }
    if let Some(constant) = value.as_ref().and_then(|value| value.cast::<XamlConstantNode>()) {
        if let XamlValue::String(text) = &constant.constant {
            return Ok(Some(text.clone()));
        }
    }
    Err(error(format!("x:Bind: {property} must be the name of a method"), &**assignment))
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

/// The checks of the binding after its path is resolved: the mode and the update trigger WinUI
/// accepts (`BindAssignment.ValidateUpdateSourceTrigger`), the `BindBack` of a `TwoWay`
/// function (`ValidateBindBackAssignment`), the return type of a function
/// (`ValidatePathAssignment`), a `TwoWay` path that ends in a cast (`ValidateTypeCasting`) and
/// one that ends in something it cannot write (`BindAssignment_TwoWaySourceNotWritable`).
fn check_binding(
    resolver: &Resolver<'_>,
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

    let function = match elements.last() {
        Some(XamlIlBindingPathElementNode::XBindFunction(function)) => Some(function.clone()),
        _ => None,
    };
    let target = target_type(resolver.context);
    match (&function, two_way, text_value(assignments, BIND_BACK)?) {
        (Some(function), true, Some(text)) => {
            let bind_back = resolver.bind_back(&text, target.as_ref())?;
            *function.bind_back.borrow_mut() = Some(Rc::new(bind_back));
        }
        (Some(_), true, None) => {
            return Err(error(
                "BindBack was expected, but not found: a TwoWay x:Bind through a function writes back through the method BindBack names",
                line_info,
            ));
        }
        (_, _, Some(_)) => {
            return Err(error(
                "Unexpected BindBack found: BindBack is only used by a TwoWay x:Bind whose path ends with a function",
                line_info,
            ));
        }
        _ => {}
    }
    if let (Some(function), Some(target)) = (&function, &target) {
        let returned = function.type_();
        let accepted = target.is_assignable_from(&*returned) || target.is("System", "String") || target.is("System", "Object");
        if !accepted {
            return Err(error(
                format!(
                    "Function return type '{}' must match binding target type '{}'",
                    returned.full_name(),
                    target.full_name()
                ),
                line_info,
            ));
        }
    }
    if !two_way || function.is_some() {
        return Ok(());
    }
    match elements.last() {
        None => Err(error("A TwoWay x:Bind needs a path that ends in a writable property", line_info)),
        Some(XamlIlBindingPathElementNode::TypeCast(_)) => {
            Err(error("TwoWay binding is invalid when the binding expression ends with a cast", line_info))
        }
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
