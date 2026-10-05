//! Port of `DynamicSettersTests.cs`: the choice of a property setter by the
//! run-time type of the value, with a setter the test language adds.

use std::any::Any;
use std::rc::Rc;

use ferroui_base::animation::TimeSpan;
use ferroui_base::metadata::MarkupValue;
use ferroui_base::utilities::Uri;
use xamlx::ast::{
    IXamlAstNode, IXamlPropertySetter, PropertySetterBinderParameters, XamlAstClrProperty, XamlAstNodeExtensions,
};
use xamlx::exceptions::XamlResult;
use xamlx::transform::{AstTransformationContext, IXamlAstTransformer};
use xamlx::type_system::{IXamlCustomAttribute, IXamlMethod, IXamlType};
use xamlx::xaml_query_interface;

use crate::runtime::interpreter::{EvalContext, IXamlEvaluablePropertySetter};

use super::classes::*;
use super::{HostOptions, TestHost, X};

/// Sets a property value either on `Value` or on `SpecialHandler`,
/// depending on the type of the value.
struct SpecialTransformer;

impl IXamlAstTransformer for SpecialTransformer {
    fn transform(
        &self,
        _context: &AstTransformationContext,
        node: Rc<dyn IXamlAstNode>,
    ) -> XamlResult<Rc<dyn IXamlAstNode>> {
        if let Some(property) = node.cast::<XamlAstClrProperty>() {
            let declaring_type = property.declaring_type();
            if property.name() == "Value" && declaring_type.name() == "DynamicSettersClass`2" {
                let get_special_handler = declaring_type.get_method(|m| m.name() == "get_SpecialHandler")?;
                let handle = get_special_handler.return_type().get_method(|m| m.name() == "Handle")?;
                let already = property.setters().iter().any(|s| s.as_any().is::<SpecialHandlerPropertySetter>());
                if !already {
                    property.setters.borrow_mut().push(Rc::new(SpecialHandlerPropertySetter::new(
                        get_special_handler,
                        handle,
                    )));
                }
            }
        }
        Ok(node)
    }
}

struct SpecialHandlerPropertySetter {
    target_type: Rc<dyn IXamlType>,
    parameters: Vec<Rc<dyn IXamlType>>,
    binder_parameters: PropertySetterBinderParameters,
    get_special_handler: Rc<dyn IXamlMethod>,
    handle: Rc<dyn IXamlMethod>,
}

impl SpecialHandlerPropertySetter {
    fn new(get_special_handler: Rc<dyn IXamlMethod>, handle: Rc<dyn IXamlMethod>) -> Self {
        let property_type = handle.parameters()[0].clone();
        let binder_parameters = PropertySetterBinderParameters::default();
        let allow_null = property_type.accepts_null();
        binder_parameters.allow_multiple.set(false);
        binder_parameters.allow_x_null.set(allow_null);
        binder_parameters.allow_runtime_null.set(allow_null);
        Self {
            target_type: handle.declaring_type(),
            parameters: vec![property_type],
            binder_parameters,
            get_special_handler,
            handle,
        }
    }
}

impl IXamlPropertySetter for SpecialHandlerPropertySetter {
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
        Vec::new()
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn type_name(&self) -> &'static str {
        "SpecialHandlerPropertySetter"
    }
    fn query_interface(self: Rc<Self>, slot: &mut dyn Any) -> bool {
        xaml_query_interface!(self, slot, dyn IXamlEvaluablePropertySetter)
    }
}

impl IXamlEvaluablePropertySetter for SpecialHandlerPropertySetter {
    /// `target.SpecialHandler.Handle(value)`.
    fn evaluate(&self, context: &mut EvalContext<'_>, target: &MarkupValue, arguments: &[MarkupValue]) -> XamlResult<()> {
        let line_info = xamlx::ast::XamlLineInfo::new(1, 1);
        let handler = context.call_method(&self.get_special_handler, std::slice::from_ref(target), &line_info)?;
        context.call_method(&self.handle, &[handler, arguments[0].clone()], &line_info).map(|_| ())
    }
}

fn host() -> TestHost {
    TestHost::with_options(HostOptions { transformer: Some(|| Box::new(SpecialTransformer)), ..HostOptions::default() })
}

fn xaml(type_arguments: &str, provided: &str) -> String {
    format!(
        "
<DynamicSettersClass
    x:TypeArguments='{type_arguments}'
    xmlns='clr-namespace:RtXamlParserTests'
    xmlns:x='{X}'
    xmlns:sys='clr-namespace:System;assembly=netstandard'
    Value='{{DynamicProvider ProvidedValue={provided}}}' />
"
    )
}

type Class<T1, T2> = Rc<DynamicSettersClass<T1, T2>>;

fn run<T1: 'static, T2: 'static>(type_arguments: &str, provided: &str) -> Class<T1, T2> {
    host().run::<Class<T1, T2>>(&xaml(type_arguments, provided))
}

fn error(type_arguments: &str, provided: &str) -> &'static str {
    host().error(&xaml(type_arguments, provided), None).type_name()
}

fn time_span() -> TimeSpan {
    TimeSpan::from_seconds(45296.789)
}

#[test]
fn dynamic_setter_with_reference_types() {
    let arguments = "sys:String,sys:Uri";
    // A null argument matches the first setter.
    let result = run::<Option<String>, Option<Uri>>(arguments, "Null");
    assert!(result.is_value_set.get());
    assert!(result.value.borrow().is_none());
    assert!(!result.special_handler.called.get());

    let result = run::<Option<String>, Option<Uri>>(arguments, "String");
    assert!(result.is_value_set.get());
    assert_eq!(result.value.borrow().as_deref(), Some("foo"));
    assert!(!result.special_handler.called.get());

    let result = run::<Option<String>, Option<Uri>>(arguments, "Uri");
    assert!(!result.is_value_set.get());
    assert!(result.value.borrow().is_none());
    assert!(result.special_handler.called.get());
    assert_eq!(*result.special_handler.value.borrow(), Uri::absolute("https://ferroui.net/").ok());
}

#[test]
fn dynamic_setter_with_value_types() {
    let arguments = "sys:Int32,sys:TimeSpan";
    assert_eq!(error(arguments, "Null"), "NullReferenceException");

    let result = run::<i32, TimeSpan>(arguments, "Int32");
    assert!(result.is_value_set.get());
    assert_eq!(*result.value.borrow(), 1234);
    assert!(!result.special_handler.called.get());

    let result = run::<i32, TimeSpan>(arguments, "TimeSpan");
    assert!(!result.is_value_set.get());
    assert_eq!(*result.value.borrow(), 0);
    assert!(result.special_handler.called.get());
    assert_eq!(*result.special_handler.value.borrow(), time_span());
}

#[test]
fn dynamic_setter_with_nullable_value_types() {
    let arguments = "sys:Nullable(sys:Int32),sys:Nullable(sys:TimeSpan)";
    // A null argument matches the first setter.
    let result = run::<Option<i32>, Option<TimeSpan>>(arguments, "Null");
    assert!(result.is_value_set.get());
    assert!(result.value.borrow().is_none());
    assert!(!result.special_handler.called.get());

    let result = run::<Option<i32>, Option<TimeSpan>>(arguments, "Int32");
    assert!(result.is_value_set.get());
    assert_eq!(*result.value.borrow(), Some(1234));
    assert!(!result.special_handler.called.get());

    let result = run::<Option<i32>, Option<TimeSpan>>(arguments, "TimeSpan");
    assert!(!result.is_value_set.get());
    assert!(result.value.borrow().is_none());
    assert!(result.special_handler.called.get());
    assert_eq!(*result.special_handler.value.borrow(), Some(time_span()));
}

#[test]
fn dynamic_setter_with_value_type_and_reference_type() {
    let arguments = "sys:Int32,sys:String";
    // A null argument matches the reference type.
    let result = run::<i32, Option<String>>(arguments, "Null");
    assert!(!result.is_value_set.get());
    assert!(result.special_handler.called.get());
    assert!(result.special_handler.value.borrow().is_none());

    let result = run::<i32, Option<String>>(arguments, "Int32");
    assert!(result.is_value_set.get());
    assert_eq!(*result.value.borrow(), 1234);
    assert!(!result.special_handler.called.get());
}

#[test]
fn dynamic_setter_with_nullable_value_type_and_reference_type() {
    let arguments = "sys:Nullable(sys:Int32),sys:String";
    // A null argument matches the first setter.
    let result = run::<Option<i32>, Option<String>>(arguments, "Null");
    assert!(result.is_value_set.get());
    assert!(result.value.borrow().is_none());
    assert!(!result.special_handler.called.get());

    let result = run::<Option<i32>, Option<String>>(arguments, "Int32");
    assert!(result.is_value_set.get());
    assert_eq!(*result.value.borrow(), Some(1234));
    assert!(!result.special_handler.called.get());

    let result = run::<Option<i32>, Option<String>>(arguments, "String");
    assert!(!result.is_value_set.get());
    assert!(result.value.borrow().is_none());
    assert!(result.special_handler.called.get());
    assert_eq!(result.special_handler.value.borrow().as_deref(), Some("foo"));
}

#[test]
fn dynamic_setter_with_mismatched_argument_should_throw_invalid_cast_exception() {
    for arguments in [
        "sys:Int32,sys:TimeSpan",
        "sys:String,sys:Uri",
        "sys:Nullable(sys:Int32),sys:Nullable(sys:TimeSpan)",
        "sys:Int32,sys:String",
        "sys:Nullable(sys:Int32),sys:String",
    ] {
        assert_eq!(error(arguments, "DateTime"), "InvalidCastException", "{arguments}");
    }
}
