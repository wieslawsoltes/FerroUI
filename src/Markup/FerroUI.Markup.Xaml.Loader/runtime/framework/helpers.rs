//! What the framework evaluators share: loading static fields, viewing
//! values as objects of the object model, exact value conversion.

use std::rc::Rc;

use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::data::BindingBase;
use ferroui_base::metadata::{from_markup_value, MarkupValue};
use ferroui_base::{BoxedValue, FerroObject, FerroProperty, Ref};
use xamlx::ast::XamlAstExtensions as _;
use xamlx::ast::XamlAstNodeExtensions as _;
use xamlx::ast::{IXamlAstValueNode, IXamlLineInfo};
use xamlx::exceptions::{XamlError, XamlResult};
use xamlx::type_system::{IXamlField, IXamlType};

use crate::compiler_extensions::transformers::{FerroXamlIlWellKnownTypes, FerroXamlIlWellKnownTypesExtensions};
use crate::runtime::interpreter::{constant_value, runtime_error, EvalContext};
use crate::runtime::type_system::{to_untyped, RuntimeField};

pub(crate) fn boxed<T: PartialEq + 'static>(value: T) -> MarkupValue {
    let value: BoxedValue = Rc::new(value);
    Some(value)
}

/// The well-known types of the configuration the evaluation runs over.
pub(crate) fn ferro_types(context: &EvalContext<'_>) -> XamlResult<Rc<FerroXamlIlWellKnownTypes>> {
    context.configuration().try_get_ferro_types()
}

/// `ldc.i4 value` passed where an enumeration is expected: the member (or
/// combination of flags) of an enumeration of the run-time type system; for
/// an enumeration the type system has no values of, the integer itself, as
/// the IL passes it.
pub(crate) fn enum_constant(
    type_: &Rc<dyn IXamlType>,
    value: i32,
    line_info: &dyn IXamlLineInfo,
) -> XamlResult<MarkupValue> {
    if type_.as_any().downcast_ref::<crate::runtime::type_system::RuntimeType>().is_none() {
        return Ok(boxed(value));
    }
    constant_value(type_, &xamlx::type_system::XamlValue::Int32(value), line_info)
}

/// `ldsfld field`: the value of a static field.
pub(crate) fn load_field(field: &Rc<dyn IXamlField>, line_info: &dyn IXamlLineInfo) -> XamlResult<MarkupValue> {
    if let Some(runtime) = field.as_any().downcast_ref::<RuntimeField>() {
        return runtime.get().map_err(|e| runtime_error("TargetInvocationException", e.to_string(), line_info));
    }
    if field.is_literal() {
        return constant_value(&field.field_type(), &field.get_literal_value()?, line_info);
    }
    Err(XamlError::load_exception(
        format!(
            "Unable to read field {}.{}: it is not a field of the run-time type system",
            field.declaring_type().full_name(),
            field.name()
        ),
        Some(line_info),
    ))
}

/// The registered property a static field holds.
pub(crate) fn load_property(
    field: &Rc<dyn IXamlField>,
    line_info: &dyn IXamlLineInfo,
) -> XamlResult<&'static FerroProperty> {
    if let Some(property) = field.as_any().downcast_ref::<RuntimeField>().and_then(RuntimeField::ferro_property) {
        return Ok(property);
    }
    let value = load_field(field, line_info)?;
    from_markup_value::<&'static FerroProperty>(&value).ok_or_else(|| {
        runtime_error(
            "InvalidCastException",
            format!(
                "Field {}.{} doesn't hold a registered property",
                field.declaring_type().full_name(),
                field.name()
            ),
            line_info,
        )
    })
}

/// The value as an object of the object model (`castclass FerroObject`).
pub(crate) fn object_of(value: &MarkupValue, line_info: &dyn IXamlLineInfo) -> XamlResult<Ref<FerroObject>> {
    match value {
        None => Err(runtime_error("NullReferenceException", "Object reference not set to an instance of an object.", line_info)),
        Some(value) => ValueTypes::as_object(&**value).ok_or_else(|| {
            runtime_error(
                "InvalidCastException",
                format!("Unable to cast object of type '{}' to type 'FerroUI.FerroObject'.", value.type_name()),
                line_info,
            )
        }),
    }
}

/// The value as a binding: a binding is held as its concrete object and
/// viewed through the binding contract.
pub(crate) fn binding_of(value: &MarkupValue, line_info: &dyn IXamlLineInfo) -> XamlResult<Rc<dyn BindingBase>> {
    from_markup_value::<Rc<dyn BindingBase>>(value).ok_or_else(|| match value {
        None => runtime_error("NullReferenceException", "The binding is null", line_info),
        Some(value) => runtime_error(
            "InvalidCastException",
            format!("Unable to cast object of type '{}' to type 'FerroUI.Data.BindingBase'.", value.type_name()),
            line_info,
        ),
    })
}

/// The value in exactly the value type of a registered property: what the
/// untyped setter of the property system takes.
pub(crate) fn exact_property_value(
    property: &'static FerroProperty,
    value: &MarkupValue,
    line_info: &dyn IXamlLineInfo,
) -> XamlResult<BoxedValue> {
    let target = ValueType::new(property.property_type(), property.property_type_name());
    crate::runtime::type_system::to_exact_value(value, target).map_err(|e| {
        runtime_error(
            "InvalidCastException",
            format!("Unable to set property {}.{}: {e}", property.owner_type().name(), property.name()),
            line_info,
        )
    })
}

/// The untyped value of a registered property of an object.
pub(crate) fn property_value(object: &Ref<FerroObject>, property: &'static FerroProperty) -> MarkupValue {
    to_untyped(object.get_value_untyped(property))
}

/// Evaluates a value node as a value of its own type.
pub(crate) fn evaluate_as_own_type(
    node: &Rc<dyn IXamlAstValueNode>,
    context: &mut EvalContext<'_>,
) -> XamlResult<MarkupValue> {
    let type_ = node.type_().get_clr_type()?;
    context.evaluate(&node.as_node(), Some(&type_))
}

/// Evaluates a value node as a value of `type_`.
pub(crate) fn evaluate_as(
    node: &Rc<dyn IXamlAstValueNode>,
    context: &mut EvalContext<'_>,
    type_: &Rc<dyn IXamlType>,
) -> XamlResult<MarkupValue> {
    context.evaluate(&node.as_node(), Some(type_))
}
