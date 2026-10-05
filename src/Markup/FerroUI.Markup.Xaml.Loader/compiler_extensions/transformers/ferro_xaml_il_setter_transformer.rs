//! Port of `CompilerExtensions/Transformers/FerroXamlIlSetterTransformer.cs`.

use std::any::Any;
use std::cell::Cell;
use std::rc::Rc;

use xamlx::ast::{
    IXamlAstClrPropertyExtension, IXamlAstNode, IXamlAstPropertyReference, IXamlAstValueNode,
    IXamlLineInfo, IXamlPropertySetter, PropertySetterBinderParameters, XamlAstClrProperty,
    XamlAstClrTypeReference, XamlAstExtensions, XamlAstNodeExtensions, XamlAstObjectNode,
    XamlAstPropertyReferenceExtensions, XamlAstTextNode, XamlAstXamlPropertyValueNode,
};
use xamlx::exceptions::{XamlError, XamlResult};
use xamlx::transform::{AstTransformationContext, IXamlAstTransformer, XamlTransformHelpers};
use xamlx::type_system::{IXamlCustomAttribute, IXamlMethod, IXamlType};

use super::ferro_xaml_il_selector_transformer::{argument_out_of_range, null_reference, value_at};
use crate::compiler_extensions::transformers::{
    get_nullable_clr_type, FerroXamlIlTargetTypeMetadataNode, FerroXamlIlWellKnownTypes,
    FerroXamlIlWellKnownTypesExtensions, ScopeTypes, XamlIlTypeSelector,
};
use crate::compiler_extensions::{IXamlIlFerroPropertyNode, XamlIlFerroPropertyHelper};

/// `XamlStyleTransformException : XamlTransformException`.
///
/// Errors are values of the single [`XamlError`] type; the derived class is represented by a
/// `XamlError::Transform` tagged with the class name, which is what
/// `FerroXamlDiagnosticCodes::xaml_x_diagnostic_code_to_ferro` and [`Self::is`] look at.
pub struct XamlStyleTransformException;

impl XamlStyleTransformException {
    pub const TYPE_NAME: &'static str = "XamlStyleTransformException";

    /// `new XamlStyleTransformException(message, lineInfo, innerException)`.
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

    /// `e is XamlStyleTransformException`.
    pub fn is(error: &XamlError) -> bool {
        error.is_derived_type(Self::TYPE_NAME)
    }
}

pub struct FerroXamlIlSetterTransformer;

impl IXamlAstTransformer for FerroXamlIlSetterTransformer {
    fn transform(
        &self,
        context: &AstTransformationContext,
        node: Rc<dyn IXamlAstNode>,
    ) -> XamlResult<Rc<dyn IXamlAstNode>> {
        let Some(on) = node.cast::<XamlAstObjectNode>() else {
            return Ok(node);
        };
        if !on.type_().get_clr_type()?.is("FerroUI.Styling", "Setter") {
            return Ok(node);
        }

        let mut target_type: Option<Rc<dyn IXamlType>> = None;

        let ferro_types = context.get_ferro_types();

        let style_parent = context
            .parent_nodes()
            .into_iter()
            .filter_map(|n| n.cast::<FerroXamlIlTargetTypeMetadataNode>())
            .find(|x| x.scope_type == ScopeTypes::Style);

        if let Some(style_parent) = &style_parent {
            target_type = Some(get_nullable_clr_type(&style_parent.target_type())?.ok_or_else(|| {
                XamlStyleTransformException::new(
                    "Can not find parent Style Selector or ControlTemplate TargetType. If setter is not part of the style, you can set x:SetterTargetType directive on its parent.",
                    &*node,
                    None,
                )
            })?);
        }

        let (Some(target_type), Some(style_parent)) = (target_type, style_parent) else {
            return Err(XamlStyleTransformException::new(
                "Could not determine target type of Setter",
                &*node,
                None,
            ));
        };

        let property = find_property_value(&on, "Property")?;
        let prop_type: Rc<dyn IXamlType> = match property {
            Some(property) => {
                let values = property.values.borrow().clone();
                let mut ferro_property_node = values
                    .iter()
                    .find_map(|v| v.cast::<dyn IXamlIlFerroPropertyNode>());
                if ferro_property_node.is_none() {
                    let property_name = values
                        .iter()
                        .find_map(|v| v.cast::<XamlAstTextNode>())
                        .map(|t| t.text());
                    let Some(property_name) = property_name else {
                        return Err(XamlStyleTransformException::new(
                            "Setter.Property must be a string.",
                            &*node,
                            None,
                        ));
                    };

                    let created = XamlIlFerroPropertyHelper::create_node(
                        context,
                        &property_name,
                        XamlAstClrTypeReference::new(&*on, target_type.clone(), false),
                        &*value_at(&values, 0)?,
                    )?;

                    if created.is_xaml_il_ferro_class_property_node()
                        && has_complex_activator(&style_parent)
                    {
                        return Err(XamlStyleTransformException::new(
                            format!(
                                "Cannot set Classes Binding property '{property_name}' because the style has an activator."
                            ),
                            &*node,
                            None,
                        ));
                    }

                    let value: Rc<dyn IXamlAstValueNode> = created.clone();
                    *property.values.borrow_mut() = vec![value];
                    ferro_property_node = Some(created);
                }

                match ferro_property_node {
                    Some(ferro_property_node) => ferro_property_node.ferro_property_type(),
                    None => return Err(null_reference()),
                }
            }
            None => {
                let property_path = find_property_value(&on, "PropertyPath")?;
                return Err(match property_path {
                    None => XamlStyleTransformException::new(
                        "Setter without a property or property path is not valid",
                        &*node,
                        None,
                    ),
                    Some(_) => XamlStyleTransformException::new(
                        "Unable to get the property path property type",
                        &*node,
                        None,
                    ),
                });
            }
        };

        let value_property = find_property_value(&on, "Value")?;

        let mut text_value: Option<Rc<XamlAstTextNode>> = None;

        match &value_property {
            Some(value_property) => {
                let values = value_property.values.borrow();
                if values.len() == 1 {
                    text_value = values[0].cast::<XamlAstTextNode>();
                }
            }
            None => {
                let children = on.children.borrow();
                let non_property_children: Vec<&Rc<dyn IXamlAstNode>> = children
                    .iter()
                    .filter(|child| !child.is::<XamlAstXamlPropertyValueNode>())
                    .collect();

                if non_property_children.len() == 1 {
                    text_value = non_property_children[0].cast::<XamlAstTextNode>();
                }
            }
        }

        let mut converted = false;
        if let Some(text_value) = &text_value {
            let text_value_node: Rc<dyn IXamlAstValueNode> = text_value.clone();
            converted = XamlTransformHelpers::try_get_correctly_typed_value(
                context,
                &text_value_node,
                &prop_type,
            )?
            .is_some();
        }

        if let (Some(text_value), true) = (&text_value, converted) {
            let setter_type = on.type_().get_clr_type()?;
            let setter_value_property = match &value_property {
                Some(value_property) => SetterValueProperty::new(
                    &*value_property.property(),
                    setter_type,
                    prop_type.clone(),
                    &ferro_types,
                )?,
                None => SetterValueProperty::new(
                    &**text_value,
                    setter_type,
                    prop_type.clone(),
                    &ferro_types,
                )?,
            };
            let property_reference: Rc<dyn IXamlAstPropertyReference> = setter_value_property;
            match &value_property {
                Some(value_property) => {
                    *value_property.property.borrow_mut() = property_reference;
                }
                None => {
                    let text_value_node: Rc<dyn IXamlAstValueNode> = text_value.clone();
                    let replacement: Rc<dyn IXamlAstNode> = XamlAstXamlPropertyValueNode::new(
                        &**text_value,
                        property_reference,
                        text_value_node,
                        false,
                    );
                    let mut children = on.children.borrow_mut();
                    let index = children
                        .iter()
                        .position(|c| c.same_node(text_value))
                        .ok_or_else(argument_out_of_range)?;
                    children[index] = replacement;
                }
            }
        }
        // If we have `Value` property with plain text content that wasn't parsed, throw an exception.
        else if let (Some(_), Some(text_value)) = (&value_property, &text_value) {
            return Err(XamlStyleTransformException::new(
                format!("Unable to convert property value to {}", prop_type.get_fqn()),
                &**text_value,
                None,
            ));
        }

        // Handling a very specific case, when ITemplate value is used inside of Setter.Value,
        // Which then is materialized for a specific control, and usually would set TemplatedParent.
        // Note: this code is not always valid, as TemplatedParent might not be set,
        // but we have better validation in runtime for TemplatedBinding.
        // See Correctly_Resolve_TemplateBinding_In_Theme_Detached_Template test.
        if !ferro_types.i_template_of_control.is_assignable_from(&*prop_type) {
            let value_obj = on
                .children
                .borrow()
                .iter()
                .find_map(|c| c.cast::<XamlAstObjectNode>());
            if let Some(value_obj) = value_obj {
                if ferro_types
                    .i_template_of_control
                    .is_assignable_from(&*value_obj.type_().get_clr_type()?)
                {
                    let value_obj_node: Rc<dyn IXamlAstValueNode> = value_obj.clone();
                    let replacement: Rc<dyn IXamlAstNode> = FerroXamlIlTargetTypeMetadataNode::new(
                        value_obj_node,
                        XamlAstClrTypeReference::new(&*on, target_type, false),
                        ScopeTypes::ControlTemplate,
                    );
                    let mut children = on.children.borrow_mut();
                    let index = children
                        .iter()
                        .position(|c| c.same_node(&value_obj))
                        .ok_or_else(argument_out_of_range)?;
                    children[index] = replacement;
                }
            }
        }

        Ok(node)
    }
}

/// `on.Children.OfType<XamlAstXamlPropertyValueNode>().FirstOrDefault(x =>
/// x.Property.GetClrProperty().Name == name)`.
fn find_property_value(
    on: &XamlAstObjectNode,
    name: &str,
) -> XamlResult<Option<Rc<XamlAstXamlPropertyValueNode>>> {
    let children = on.children.borrow().clone();
    for child in children {
        if let Some(p) = child.cast::<XamlAstXamlPropertyValueNode>() {
            if p.property().get_clr_property()?.name() == name {
                return Ok(Some(p));
            }
        }
    }
    Ok(None)
}

/// Check that the style has selector activator and complexity
fn has_complex_activator(style: &FerroXamlIlTargetTypeMetadataNode) -> bool {
    let Some(value_node) = style.value().cast::<XamlAstObjectNode>() else {
        return false;
    };
    let selector_node = value_node.children.borrow().iter().find_map(|n| {
        let property_value = n.cast::<XamlAstXamlPropertyValueNode>()?;
        let property = property_value.property().cast::<XamlAstClrProperty>()?;
        (property.name() == "Selector").then_some(property_value)
    });
    let Some(selector_node) = selector_node else {
        return false;
    };
    let values = selector_node.values.borrow();
    if values.is_empty() {
        return false;
    }
    values.len() > 1 || !values[0].as_any().is::<XamlIlTypeSelector>()
}

/// `SetterValueProperty : XamlAstClrProperty`: the `Setter.Value` property retyped to the type
/// of the registered property the setter targets, so that the standard value conversion parses
/// the text of the value at compile time.
///
/// The derived class is represented by a `XamlAstClrProperty` whose
/// [`XamlAstClrProperty::extension`] is an instance of this type; [`SetterValueProperty::new`]
/// returns that property node: name `Value`, declaring type the setter type, getter
/// `Setter.get_Value` and three [`XamlIlDirectCallPropertySetter`]s that all call
/// `Setter.set_Value(object)`, in this order:
///
/// 1. parameter `BindingBase`, null not allowed;
/// 2. parameter `UnsetValueType`, null not allowed;
/// 3. parameter the registered property's value type, null allowed when that type accepts null.
pub struct SetterValueProperty;

impl SetterValueProperty {
    pub fn new(
        line: &dyn IXamlLineInfo,
        setter_type: Rc<dyn IXamlType>,
        target_type: Rc<dyn IXamlType>,
        types: &FerroXamlIlWellKnownTypes,
    ) -> XamlResult<Rc<XamlAstClrProperty>> {
        let rv = XamlAstClrProperty::new(line, "Value", setter_type.clone(), None);
        let methods = setter_type.methods();
        let first = |name: &str| {
            methods
                .iter()
                .find(|m| m.name() == name)
                .cloned()
                .ok_or_else(|| XamlError::invalid_operation("Sequence contains no matching element"))
        };
        *rv.getter.borrow_mut() = Some(first("get_Value")?);
        let method = first("set_Value")?;
        {
            let mut setters = rv.setters.borrow_mut();
            setters.push(XamlIlDirectCallPropertySetter::new(
                method.clone(),
                types.binding_base.clone(),
                false,
            )?);
            setters.push(XamlIlDirectCallPropertySetter::new(
                method.clone(),
                types.unset_value_type.clone(),
                false,
            )?);
            let allow_null = target_type.accepts_null();
            setters.push(XamlIlDirectCallPropertySetter::new(
                method,
                target_type,
                allow_null,
            )?);
        }
        rv.set_extension(Rc::new(SetterValueProperty));
        Ok(rv)
    }

    /// `property is SetterValueProperty`.
    pub fn from_clr_property(property: &XamlAstClrProperty) -> Option<Rc<SetterValueProperty>> {
        property.extension_as::<SetterValueProperty>()
    }
}

impl IXamlAstClrPropertyExtension for SetterValueProperty {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn into_any_rc(self: Rc<Self>) -> Rc<dyn Any> {
        self
    }
    fn type_name(&self) -> &'static str {
        "SetterValueProperty"
    }
}

/// `SetterValueProperty.XamlIlDirectCallPropertySetter`: calls a method that takes `object`
/// with a value of the statically known type `type_`.
///
/// Two setters are equal when their methods and their types are equal.
///
/// # What a back end has to do (upstream IL)
///
/// `Emit` (stack: target object, value of type `type_`): when `type_` is a value type, box the
/// value as `type_`; then call `method` on the target object (virtual call when the method is
/// an instance method) and discard its result if it has one. The setter leaves nothing on the
/// stack. There is no `EmitWithArguments` form.
pub struct XamlIlDirectCallPropertySetter {
    /// `_method` (private upstream): `Setter.set_Value(object)`.
    pub method: Rc<dyn IXamlMethod>,
    /// `_type` (private upstream): the static type of the value.
    pub type_: Rc<dyn IXamlType>,
    pub target_type: Rc<dyn IXamlType>,
    pub binder_parameters: PropertySetterBinderParameters,
    pub parameters: Vec<Rc<dyn IXamlType>>,
}

impl XamlIlDirectCallPropertySetter {
    pub fn new(
        method: Rc<dyn IXamlMethod>,
        type_: Rc<dyn IXamlType>,
        allow_null: bool,
    ) -> XamlResult<Rc<Self>> {
        let target_type = method.this_or_first_parameter()?;
        Ok(Rc::new(Self {
            parameters: vec![type_.clone()],
            method,
            type_,
            target_type,
            binder_parameters: PropertySetterBinderParameters {
                allow_x_null: Cell::new(allow_null),
                allow_runtime_null: Cell::new(allow_null),
                ..PropertySetterBinderParameters::default()
            },
        }))
    }
}

impl IXamlPropertySetter for XamlIlDirectCallPropertySetter {
    fn target_type(&self) -> Rc<dyn IXamlType> {
        self.target_type.clone()
    }
    fn binder_parameters(&self) -> &PropertySetterBinderParameters {
        &self.binder_parameters
    }
    fn parameters(&self) -> Vec<Rc<dyn IXamlType>> {
        self.parameters.clone()
    }
    fn custom_attributes(&self) -> Vec<Rc<dyn IXamlCustomAttribute>> {
        self.method.custom_attributes()
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn type_name(&self) -> &'static str {
        "XamlIlDirectCallPropertySetter"
    }
    fn equals(&self, other: &dyn IXamlPropertySetter) -> bool {
        match other.as_any().downcast_ref::<XamlIlDirectCallPropertySetter>() {
            Some(other) => self.method.equals(&*other.method) && self.type_.equals(&*other.type_),
            None => false,
        }
    }
}
