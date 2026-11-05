//! The directives around `{x:Bind}` in the compiler (step X5 of docs/porting/x-bind.md):
//! `x:DefaultBindMode`, `x:Phase`, `x:Load` and `x:DeferLoadStrategy`.
//!
//! Not a port of an upstream file. The semantics and the messages are those of WinUI's XAML
//! compiler at `7e4b0c2096be402f51cc8f7d203ebf67cff183d0` (the NOTICE.md of `ferroui-markup-xaml`
//! lists the sources: `XamlDomValidator.cs`, `XamlHarvester.cs`, `PhaseAssignment.cs`,
//! `BoundLoadAssignment.cs`, `DeferLoadStrategy.cs`, `DefaultBindMode.cs` and
//! `Properties/XamlCompilerResources.resx`). [`XBindDirectivesTransformer`] runs after the types
//! are resolved and before the properties are, so that what it writes is resolved as markup
//! written by hand would be:
//!
//! - `x:DefaultBindMode` is checked and left on its element; the binding path parser gives each
//!   `x:Bind` without a `Mode` the mode of the nearest one around it
//!   ([`super::x_bind::apply_default_mode`]), and the compiled bindings metadata remover takes
//!   the directives out once the bindings are resolved ([`remove_default_bind_mode`]).
//! - `x:Phase` becomes the `Phase` of every `x:Bind` of its element; it must be inside a
//!   `DataTemplate`, on an element with an `x:Bind`, and below `25`.
//! - `x:Load="False"`, `x:DeferLoadStrategy="Lazy"` and a bound `x:Load` replace the element by a
//!   `DeferredElement` (`ferroui_markup_xaml::markup_extensions`) whose `Content` is the element,
//!   built as the deferred content of a template is, whose `ElementName` is its name and whose
//!   `Load` is the value of `x:Load` (a literal or the `x:Bind`). `x:Load="True"` and
//!   `x:DeferLoadStrategy="None"` create the element with its document.

use std::cell::RefCell;
use std::rc::Rc;

use xamlx::ast::{
    IXamlAstNode, IXamlAstTypeReference, IXamlAstValueNode, IXamlLineInfo, XamlAstClrTypeReference,
    XamlAstNamePropertyReference, XamlAstNodeExtensions, XamlAstObjectNode, XamlAstTextNode,
    XamlAstXamlPropertyValueNode, XamlAstXmlDirective,
};
use xamlx::exceptions::{XamlError, XamlResult};
use xamlx::transform::{AstTransformationContext, IXamlAstTransformer};
use xamlx::type_system::IXamlType;
use xamlx::XamlNamespaces;

use super::{FerroXamlIlWellKnownTypesExtensions, XamlBindingsTransformException};

/// `x:DefaultBindMode`.
pub const DEFAULT_BIND_MODE: &str = "DefaultBindMode";
const PHASE: &str = "Phase";
const LOAD: &str = "Load";
const DEFER_LOAD_STRATEGY: &str = "DeferLoadStrategy";
const NAME: &str = "Name";

/// The placeholder of an element with `x:Load` (`ferroui_markup_xaml::markup_extensions`).
pub const DEFERRED_ELEMENT: &str = "FerroUI.Markup.Xaml.MarkupExtensions.DeferredElement";

/// The phases of `x:Phase` are below this (`XamlCompiler_InvalidValueForPhase`).
const MAX_PHASE: i32 = 25;

const LOAD_NOT_SUPPORTED: &str = "x:Load cannot be used on the root of a Page, User Control or DataTemplate, or direct children of a Resource Dictionary, and can only be used on elements of type UIElement or FlyoutBase";

fn error(message: impl Into<String>, line_info: &dyn IXamlLineInfo) -> XamlError {
    XamlBindingsTransformException::new(message, line_info, None)
}

/// The directive `x:name` of `children`, the children of an object node.
pub fn find_directive(children: &RefCell<Vec<Rc<dyn IXamlAstNode>>>, name: &str) -> Option<Rc<XamlAstXmlDirective>> {
    children.borrow().iter().filter_map(|child| child.cast::<XamlAstXmlDirective>()).find(|directive| {
        directive.namespace.borrow().as_deref() == Some(XamlNamespaces::XAML2006) && *directive.name.borrow() == name
    })
}

/// The text of the only value of a directive.
pub fn directive_text(directive: &XamlAstXmlDirective) -> Option<String> {
    let values = directive.values.borrow();
    match values.as_slice() {
        [value] => value.cast::<XamlAstTextNode>().map(|text| text.text()),
        _ => None,
    }
}

/// Takes the `x:DefaultBindMode` directives out of `children` once the bindings are resolved
/// (from the compiled bindings metadata remover).
pub fn remove_default_bind_mode(children: &RefCell<Vec<Rc<dyn IXamlAstNode>>>) {
    children.borrow_mut().retain(|child| {
        child.cast::<XamlAstXmlDirective>().is_none_or(|directive| {
            directive.namespace.borrow().as_deref() != Some(XamlNamespaces::XAML2006)
                || *directive.name.borrow() != DEFAULT_BIND_MODE
        })
    });
}

fn remove_directive(object: &XamlAstObjectNode, directive: &Rc<XamlAstXmlDirective>) {
    object.children.borrow_mut().retain(|child| !child.same_node(directive));
}

/// The line information of the value of a directive (the attribute), else of the directive.
fn value_line_info(directive: &Rc<XamlAstXmlDirective>) -> Rc<dyn IXamlAstNode> {
    match directive.values.borrow().first() {
        Some(value) => value.as_node(),
        None => directive.as_node(),
    }
}

fn clr_type(object: &XamlAstObjectNode) -> Option<Rc<dyn IXamlType>> {
    object.type_.borrow().cast::<XamlAstClrTypeReference>().map(|type_| type_.type_.clone())
}

/// The `x:Bind` extensions that are values of the properties of `object` itself.
fn element_bindings(object: &XamlAstObjectNode) -> Vec<Rc<XamlAstObjectNode>> {
    let mut bindings = Vec::new();
    for child in object.children.borrow().iter() {
        let Some(value) = child.cast::<XamlAstXamlPropertyValueNode>() else {
            continue;
        };
        for value in value.values.borrow().iter() {
            let Some(binding) = value.cast::<XamlAstObjectNode>() else {
                continue;
            };
            if clr_type(&binding).is_some_and(|type_| super::x_bind::is_x_bind_extension(&type_)) {
                bindings.push(binding);
            }
        }
    }
    bindings
}

/// Whether `object` has a property value of the property `name`, as markup writes it.
fn has_property_value(object: &XamlAstObjectNode, name: &str) -> bool {
    object.children.borrow().iter().any(|child| {
        child
            .cast::<XamlAstXamlPropertyValueNode>()
            .and_then(|value| value.property().cast::<XamlAstNamePropertyReference>())
            .is_some_and(|reference| reference.name() == name)
    })
}

/// Adds the property value `name` = `value` to `object` as markup would write it, before the
/// properties are resolved.
fn add_property_value(
    object: &XamlAstObjectNode,
    type_: &Rc<dyn IXamlAstTypeReference>,
    name: &str,
    value: Rc<dyn IXamlAstValueNode>,
    is_attribute_syntax: bool,
) {
    let reference = XamlAstNamePropertyReference::new(object, type_.clone(), name, type_.clone());
    let assignment: Rc<dyn IXamlAstNode> = XamlAstXamlPropertyValueNode::new(object, reference, value, is_attribute_syntax);
    object.children.borrow_mut().push(assignment);
}

/// A text value of type `String`: the transformer runs after the types are resolved, so the
/// value is given the resolved type the parser's text would have had by then.
fn text_value(context: &AstTransformationContext, line_info: &dyn IXamlLineInfo, text: &str) -> Rc<dyn IXamlAstValueNode> {
    let string = context.configuration().well_known_types().string.clone();
    XamlAstTextNode::with_type(line_info, text, true, Some(string))
}

/// The name of an element: its `x:Name` (the `Name` property it became) or its `Name`.
fn element_name(object: &XamlAstObjectNode) -> Option<String> {
    if let Some(directive) = find_directive(&object.children, NAME) {
        return directive_text(&directive);
    }
    for child in object.children.borrow().iter() {
        let Some(value) = child.cast::<XamlAstXamlPropertyValueNode>() else {
            continue;
        };
        let named = value.property().cast::<XamlAstNamePropertyReference>().is_some_and(|reference| reference.name() == NAME);
        if named {
            return value.values.borrow().first().and_then(|value| value.cast::<XamlAstTextNode>()).map(|text| text.text());
        }
    }
    None
}

/// Checks `x:DefaultBindMode`, `x:Phase`, `x:Load` and `x:DeferLoadStrategy` and writes what
/// they mean (see the module documentation).
pub struct XBindDirectivesTransformer;

impl IXamlAstTransformer for XBindDirectivesTransformer {
    fn transform(
        &self,
        context: &AstTransformationContext,
        node: Rc<dyn IXamlAstNode>,
    ) -> XamlResult<Rc<dyn IXamlAstNode>> {
        let Some(object) = node.cast::<XamlAstObjectNode>() else {
            return Ok(node);
        };
        default_bind_mode(context, &object)?;
        phase(context, &object)?;
        match load(context, &object)? {
            Some(placeholder) => Ok(placeholder),
            None => Ok(node),
        }
    }
}

/// `x:DefaultBindMode`: one of the modes of x:Bind (`XamlDomValidator`,
/// `XamlValidationError_DefaultBindModeInvalidValue`), written with the case of the mode.
fn default_bind_mode(context: &AstTransformationContext, object: &XamlAstObjectNode) -> XamlResult<()> {
    let Some(directive) = find_directive(&object.children, DEFAULT_BIND_MODE) else {
        return Ok(());
    };
    let line_info = value_line_info(&directive);
    let text = directive_text(&directive).unwrap_or_default();
    let Some(mode) = ["OneTime", "OneWay", "TwoWay"].into_iter().find(|mode| mode.eq_ignore_ascii_case(text.trim())) else {
        return Err(error(
            "Invalid value specified for attribute 'DefaultBindMode'.  Accepted values are 'OneWay', 'OneTime', or 'TwoWay'.",
            &*line_info,
        ));
    };
    *directive.values.borrow_mut() = vec![text_value(context, &*line_info, mode)];
    Ok(())
}

/// `x:Phase` (`PhaseAssignment`, `XamlCompiler_InvalidValueForPhase`,
/// `XamlCompiler_PhaseMustBeUsedWithinADataTemplate`, `XamlCompiler_PhaseMustHaveAssociatedBind`):
/// the phase of the bindings of its element.
fn phase(context: &AstTransformationContext, object: &XamlAstObjectNode) -> XamlResult<()> {
    let Some(directive) = find_directive(&object.children, PHASE) else {
        return Ok(());
    };
    let line_info = value_line_info(&directive);
    let phase = directive_text(&directive)
        .and_then(|text| text.trim().parse::<i32>().ok())
        .filter(|phase| (0..MAX_PHASE).contains(phase))
        .ok_or_else(|| error("Phase value must be a positive integer less than 25.", &*line_info))?;
    let types = context.try_get_ferro_types()?;
    let in_data_template = context.parent_nodes().iter().any(|parent| {
        parent
            .cast::<XamlAstObjectNode>()
            .and_then(|parent| clr_type(&parent))
            .is_some_and(|type_| types.i_data_template.is_assignable_from(&*type_))
    });
    if !in_data_template {
        return Err(error("Phase can only be used within a DataTemplate", &*line_info));
    }
    let bindings = element_bindings(object);
    if bindings.is_empty() {
        return Err(error("Phase must be used with Bind", &*line_info));
    }
    for binding in bindings {
        if has_property_value(&binding, PHASE) {
            continue;
        }
        let type_ = binding.type_.borrow().clone();
        add_property_value(&binding, &type_, PHASE, text_value(context, &*binding, &phase.to_string()), true);
    }
    remove_directive(object, &directive);
    Ok(())
}

/// `x:Load` and `x:DeferLoadStrategy` (`XamlDomValidator`, `CanHaveDeferLoadStrategyOrLoad`,
/// `BoundLoadAssignment`): the placeholder that takes the place of the element, or `None` when
/// the element is created with its document.
fn load(context: &AstTransformationContext, object: &Rc<XamlAstObjectNode>) -> XamlResult<Option<Rc<dyn IXamlAstNode>>> {
    let load = find_directive(&object.children, LOAD);
    let strategy = find_directive(&object.children, DEFER_LOAD_STRATEGY);
    let (directive, value) = match (load, strategy) {
        (Some(load), Some(_)) => {
            return Err(error("An element cannot have both x:Load and x:DeferLoadStrategy", &*value_line_info(&load)));
        }
        (None, None) => return Ok(None),
        (None, Some(strategy)) => {
            let line_info = value_line_info(&strategy);
            match directive_text(&strategy).as_deref().map(str::trim) {
                // `Enum.TryParse(strategy, false)`: the names of the strategy, with their case.
                Some("Lazy") => {
                    let value = text_value(context, &*line_info, "False");
                    (strategy, value)
                }
                Some("None") => {
                    remove_directive(object, &strategy);
                    return Ok(None);
                }
                _ => return Err(error("Invalid value specified for attribute 'DeferLoadStrategy'.", &*line_info)),
            }
        }
        (Some(load), None) => {
            let line_info = value_line_info(&load);
            let value = load.values.borrow().first().cloned();
            let bound = value.as_ref().and_then(|value| value.cast::<XamlAstObjectNode>()).filter(|binding| {
                clr_type(binding).is_some_and(|type_| super::x_bind::is_x_bind_extension(&type_))
            });
            match (bound, directive_text(&load).as_deref().map(str::trim)) {
                (Some(binding), _) => {
                    let value: Rc<dyn IXamlAstValueNode> = binding;
                    (load, value)
                }
                (None, Some(text)) if text.eq_ignore_ascii_case("true") => {
                    remove_directive(object, &load);
                    return Ok(None);
                }
                (None, Some(text)) if text.eq_ignore_ascii_case("false") => {
                    let value = text_value(context, &*line_info, "False");
                    (load, value)
                }
                _ => {
                    return Err(error(
                        "Invalid value specified for attribute 'Load'. x:Load is True, False or an x:Bind",
                        &*line_info,
                    ));
                }
            }
        }
    };
    let line_info = value_line_info(&directive);
    let attribute = format!("x:{}", directive.name.borrow());
    let Some(name) = element_name(object) else {
        return Err(error(format!("Element must have x:Name attribute specified since it uses {attribute}."), &*line_info));
    };
    check_load_place(context, object, &*line_info)?;
    remove_directive(object, &directive);

    let placeholder_type = context.configuration().type_system.get_type(DEFERRED_ELEMENT)?;
    let type_: Rc<dyn IXamlAstTypeReference> = XamlAstClrTypeReference::new(&**object, placeholder_type, false);
    let placeholder = XamlAstObjectNode::new(&**object, type_.clone());
    add_property_value(&placeholder, &type_, "ElementName", text_value(context, &**object, &name), true);
    let element: Rc<dyn IXamlAstValueNode> = object.clone();
    add_property_value(&placeholder, &type_, "Content", element, false);
    // Last: a bound value can load the element as soon as it is read.
    add_property_value(&placeholder, &type_, "Load", value, true);
    let placeholder: Rc<dyn IXamlAstNode> = placeholder;
    Ok(Some(placeholder))
}

/// Where an element with `x:Load` can be (`CanHaveDeferLoadStrategyOrLoad`): not the root of
/// its document or of a template, not a resource, and a control.
fn check_load_place(context: &AstTransformationContext, object: &XamlAstObjectNode, line_info: &dyn IXamlLineInfo) -> XamlResult<()> {
    let types = context.try_get_ferro_types()?;
    let parents = context.parent_nodes();
    let parent_object = parents.iter().find_map(|parent| parent.cast::<XamlAstObjectNode>());
    let Some(parent_object) = parent_object else {
        return Err(error(LOAD_NOT_SUPPORTED, line_info));
    };
    let in_resources = parents
        .iter()
        .take_while(|parent| parent.cast::<XamlAstObjectNode>().is_none())
        .filter_map(|parent| parent.cast::<XamlAstXamlPropertyValueNode>())
        .any(|value| {
            value.property().cast::<XamlAstNamePropertyReference>().is_some_and(|reference| reference.name() == "Resources")
        });
    let parent_type = clr_type(&parent_object);
    let template_or_dictionary = parent_type.is_some_and(|type_| {
        types.i_data_template.is_assignable_from(&*type_)
            || types.control_template.is_assignable_from(&*type_)
            || types.i_resource_dictionary.is_assignable_from(&*type_)
    });
    let is_control = clr_type(object).is_some_and(|type_| types.control.is_assignable_from(&*type_));
    if in_resources || template_or_dictionary || !is_control {
        return Err(error(LOAD_NOT_SUPPORTED, line_info));
    }
    Ok(())
}
