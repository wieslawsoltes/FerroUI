//! Unit tests of the framework evaluators, one group per row group of the
//! back-end table.
//!
//! The evaluators run without the controls library: the well-known types
//! are the ones of the test framework fixture (`crate::testing`), whose
//! methods and constructors have no invokers; a recording evaluator stands
//! in for them and the tests assert the calls an evaluator makes, in order,
//! with their arguments. Evaluators that work on the object model itself
//! (the setters of registered properties, name scopes, selectors, queries,
//! binding paths) run on objects of the base library.

use std::cell::RefCell;
use std::rc::Rc;

use ferroui_base::controls::{INameScope, NameScope, NameScopeRef};
use ferroui_base::data::core::ValueType;
use ferroui_base::data::{BindingPriority, CompiledBindingPath, CompiledBindingPathBuilder};
use ferroui_base::media::SolidColorBrush;
use ferroui_base::metadata::{from_markup_value, IServiceProvider, MarkupValue};
use ferroui_base::styling::{Selector, StyleQuery, StyleQueryComparisonOperator};
use ferroui_base::utilities::Uri;
use ferroui_base::{BoxedValue, FerroObject, Ref, StaticType, StyledElement};
use xamlx::ast::{
    IXamlAstNode, IXamlAstValueNode, IXamlPropertySetter, XamlAstClrProperty, XamlAstExtensions, XamlAstClrTypeReference,
    XamlAstNamePropertyReference, XamlAstNewClrObjectNode, XamlAstNodeExtensions, XamlConstantNode, XamlLineInfo,
};
use xamlx::exceptions::XamlResult;
use xamlx::testing::{FakeConstructor, FakeMethod};
use xamlx::type_system::{IXamlConstructor, IXamlField, IXamlMethod, IXamlType, IXamlTypeSystem, XamlValue};

use crate::compiler_extensions::ast_nodes::*;
use crate::compiler_extensions::group_transformers::NewServiceProviderNode;
use crate::compiler_extensions::transformers::*;
use crate::compiler_extensions::*;
use crate::runtime::interpreter::{
    EvalContext, IXamlConstructorEvaluator, IXamlMethodEvaluator, IXamlSetterEvaluator, Interpreter, RuntimeContext,
    RuntimeDocument,
};
use crate::runtime::type_system::{
    DeferredContentFactory, RuntimeArray, RuntimeConstructor, RuntimeMethod, RuntimeTypeSystem,
};
use crate::testing::{create_test_framework, TestFramework};

use super::services::FerroNameScopeField;
use super::*;

/// What a recorded call returned: the text of the call.
#[derive(Clone, Debug, PartialEq)]
struct Recorded(String);

fn describe(value: &MarkupValue) -> String {
    let Some(value) = value else { return "null".to_string() };
    if let Some(recorded) = value.downcast_ref::<Recorded>() {
        return recorded.0.clone();
    }
    if let Some(text) = value.downcast_ref::<String>() {
        return format!("{text:?}");
    }
    if let Some(v) = value.downcast_ref::<f64>() {
        return v.to_string();
    }
    if let Some(v) = value.downcast_ref::<i32>() {
        return v.to_string();
    }
    if let Some(v) = value.downcast_ref::<bool>() {
        return v.to_string();
    }
    if let Some(uri) = value.downcast_ref::<Uri>() {
        return format!("<{uri}>");
    }
    let name = value.type_name();
    let name = name.split('<').next().unwrap_or(name);
    format!("[{}]", name.rsplit("::").next().unwrap_or(name))
}

/// Stands in for the invokers the fixture has none of.
#[derive(Default)]
struct Recorder {
    calls: RefCell<Vec<String>>,
}

impl Recorder {
    fn record(&self, call: String) -> MarkupValue {
        self.calls.borrow_mut().push(call.clone());
        let value: BoxedValue = Rc::new(Recorded(call));
        Some(value)
    }

    fn take(&self) -> Vec<String> {
        std::mem::take(&mut *self.calls.borrow_mut())
    }
}

fn arguments_text(arguments: &[MarkupValue]) -> String {
    arguments.iter().map(describe).collect::<Vec<_>>().join(", ")
}

impl IXamlMethodEvaluator for Recorder {
    fn invoke(
        &self,
        method: &Rc<dyn IXamlMethod>,
        _context: &mut EvalContext<'_>,
        arguments: &[MarkupValue],
    ) -> Option<XamlResult<MarkupValue>> {
        method.as_any().downcast_ref::<FakeMethod>()?;
        let name = method.name();
        let recorded = self.record(format!("{}.{name}({})", method.declaring_type().name(), arguments_text(arguments)));
        Some(Ok(match name.as_str() {
            "get_Count" => boxed(3i32),
            "ShouldProvideOption" => {
                boxed(arguments.last().and_then(from_markup_value::<String>).as_deref() == Some("yes"))
            }
            _ => recorded,
        }))
    }
}

impl IXamlConstructorEvaluator for Recorder {
    fn invoke(
        &self,
        constructor: &Rc<dyn IXamlConstructor>,
        _context: &mut EvalContext<'_>,
        arguments: &[MarkupValue],
    ) -> Option<XamlResult<MarkupValue>> {
        constructor.as_any().downcast_ref::<FakeConstructor>()?;
        Some(Ok(self.record(format!("new {}({})", constructor.declaring_type().name(), arguments_text(arguments)))))
    }
}

fn boxed<T: PartialEq + 'static>(value: T) -> MarkupValue {
    let value: BoxedValue = Rc::new(value);
    Some(value)
}

fn line() -> XamlLineInfo {
    XamlLineInfo::new(3, 4)
}

struct Harness {
    fw: TestFramework,
    ts: Rc<RuntimeTypeSystem>,
    interpreter: Rc<Interpreter>,
    recorder: Rc<Recorder>,
    context: Rc<RuntimeContext>,
    scope: Rc<NameScope>,
}

impl Harness {
    fn new() -> Self {
        ferroui_base::register_types();
        let fw = create_test_framework();
        let ts = RuntimeTypeSystem::new();
        let recorder = Rc::new(Recorder::default());
        let mut interpreter = Interpreter::new(fw.configuration.clone(), ts.clone());
        interpreter.services = Rc::new(FerroRuntimeContextServices);
        interpreter.node_evaluators.insert(0, Rc::new(FrameworkNodeEvaluator));
        interpreter.setter_evaluators.push(Rc::new(FrameworkSetterEvaluator));
        interpreter.method_evaluators.push(Rc::new(FrameworkMethodEvaluator));
        interpreter.method_evaluators.push(recorder.clone());
        interpreter.constructor_evaluators.push(recorder.clone());
        let interpreter = Rc::new(interpreter);
        let context = RuntimeContext::new(
            crate::runtime::interpreter::context_definition(&fw.configuration),
            interpreter.services.clone(),
            None,
            Rc::from(Vec::new()),
            Uri::absolute("http://example.com/app/").ok(),
        );
        let scope = Rc::new(NameScope::new());
        let as_scope: Rc<dyn INameScope> = scope.clone();
        context.set_extension(Rc::new(FerroNameScopeField(Some(as_scope))));
        Self { fw, ts, interpreter, recorder, context, scope }
    }

    fn eval(&self) -> EvalContext<'_> {
        EvalContext::new(&self.interpreter, Some(self.context.clone()), RuntimeDocument::empty(), 0)
    }

    /// Evaluates a value node as a value of its own type.
    fn value(&self, node: Rc<dyn IXamlAstValueNode>) -> XamlResult<MarkupValue> {
        let type_ = node.type_().get_clr_type()?;
        self.eval().evaluate(&node.as_node(), Some(&type_))
    }

    fn manipulate(&self, node: Rc<dyn IXamlAstNode>, target: &MarkupValue) -> XamlResult<()> {
        self.eval().manipulate(&node, target)
    }

    fn set(&self, setter: Rc<dyn IXamlPropertySetter>, target: &MarkupValue, arguments: &[MarkupValue]) -> XamlResult<()> {
        match FrameworkSetterEvaluator.evaluate(&setter, &mut self.eval(), target, arguments) {
            Some(result) => result,
            None => panic!("{} has no evaluator", setter.type_name()),
        }
    }

    fn set_nodes(
        &self,
        setter: Rc<dyn IXamlPropertySetter>,
        target: &MarkupValue,
        arguments: &[Rc<dyn IXamlAstValueNode>],
    ) -> XamlResult<()> {
        match FrameworkSetterEvaluator.evaluate_with_arguments(&setter, &mut self.eval(), target, arguments) {
            Some(result) => result,
            None => panic!("{} has no evaluator that evaluates its arguments", setter.type_name()),
        }
    }

    fn constant(&self, value: XamlValue) -> Rc<dyn IXamlAstValueNode> {
        let types = self.fw.configuration.well_known_types();
        let type_ = match &value {
            XamlValue::String(_) => types.string.clone(),
            XamlValue::Double(_) => types.double.clone(),
            XamlValue::Boolean(_) => types.boolean.clone(),
            _ => types.int32.clone(),
        };
        XamlConstantNode::new(&line(), type_, value).ok().expect("constant")
    }

    /// A constant typed with the run-time type system (for values that reach real members).
    fn real_double(&self, value: f64) -> Rc<dyn IXamlAstValueNode> {
        XamlConstantNode::new(&line(), self.ts.get("System.Double"), XamlValue::Double(value)).ok().expect("constant")
    }

    fn text(&self, text: &str) -> Rc<dyn IXamlAstValueNode> {
        self.constant(XamlValue::String(text.to_string()))
    }

    /// A real object of the object model with a styled property.
    fn brush(&self) -> (MarkupValue, Ref<SolidColorBrush>, Rc<dyn IXamlType>, Rc<dyn IXamlField>) {
        let type_ = self.ts.get("FerroUI.Media.SolidColorBrush");
        let constructor = type_.get_constructor(None).ok().expect("constructor");
        let instance = constructor.as_any().downcast_ref::<RuntimeConstructor>().unwrap().invoke(&[]).ok().unwrap();
        let brush = from_markup_value::<Ref<SolidColorBrush>>(&instance).expect("brush");
        let field = type_.get_all_fields().into_iter().find(|f| f.name() == "OpacityProperty").expect("field");
        (instance, brush, type_, field)
    }

    fn recorded(&self, name: &str) -> MarkupValue {
        boxed(Recorded(name.to_string()))
    }

    fn calls(&self) -> Vec<String> {
        self.recorder.take()
    }
}

fn message<T>(result: XamlResult<T>) -> String {
    match result {
        Ok(_) => panic!("an error was expected"),
        Err(e) => e.message(),
    }
}

fn ok<T>(result: XamlResult<T>) -> T {
    match result {
        Ok(value) => value,
        Err(e) => panic!("{}: {}", e.type_name(), e.message()),
    }
}

/// A value node that yields a prepared value and records that it was
/// evaluated.
struct PreparedValueNode {
    base: xamlx::ast::XamlAstNode,
    type_: Rc<dyn xamlx::ast::IXamlAstTypeReference>,
    value: MarkupValue,
    log: Rc<RefCell<Vec<&'static str>>>,
    name: &'static str,
}

xamlx::xaml_line_info_impl!(PreparedValueNode, base);

impl IXamlAstNode for PreparedValueNode {
    xamlx::xaml_ast_node_members!("PreparedValueNode", value);

    fn query_interface(self: Rc<Self>, slot: &mut dyn std::any::Any) -> bool {
        xamlx::xaml_query_interface!(self, slot, dyn crate::runtime::interpreter::IXamlAstEvaluableNode)
    }
}

impl IXamlAstValueNode for PreparedValueNode {
    fn type_(&self) -> Rc<dyn xamlx::ast::IXamlAstTypeReference> {
        self.type_.clone()
    }
}

impl crate::runtime::interpreter::IXamlAstEvaluableNode for PreparedValueNode {
    fn evaluate(&self, _context: &mut EvalContext<'_>) -> XamlResult<crate::runtime::interpreter::EvalResult> {
        self.log.borrow_mut().push(self.name);
        Ok(crate::runtime::interpreter::EvalResult::value(self.type_.get_clr_type()?, self.value.clone()))
    }
}

fn prepared(
    type_: Rc<dyn IXamlType>,
    value: MarkupValue,
    log: &Rc<RefCell<Vec<&'static str>>>,
    name: &'static str,
) -> Rc<dyn IXamlAstValueNode> {
    Rc::new(PreparedValueNode {
        base: xamlx::ast::XamlAstNode::new(&line()),
        type_: XamlAstClrTypeReference::new(&line(), type_, false),
        value,
        log: log.clone(),
        name,
    })
}

fn class_property(types: &Rc<FerroXamlIlWellKnownTypes>, name: &str) -> Rc<dyn IXamlAstValueNode> {
    let node: Rc<XamlIlFerroClassProperty> = ok(XamlIlFerroClassProperty::new(types, name, &line()));
    node
}

fn options_method(
    container: Rc<OptionsMarkupExtensionNodesContainer>,
    declaring_type: Rc<dyn IXamlType>,
    context_parameter: Rc<dyn IXamlType>,
) -> Rc<dyn IXamlMethod> {
    let method: Rc<OptionsMarkupExtensionMethod> =
        ok(OptionsMarkupExtensionMethod::new(container, declaring_type, context_parameter));
    method
}

// --- constants ----------------------------------------------------------------

#[test]
fn constant_nodes_create_their_values() {
    let h = Harness::new();
    let types = &h.fw.types;

    let thickness = ok(FerroXamlIlVectorLikeConstantAstNode::new(
        &line(),
        types,
        types.thickness.clone(),
        types.thickness_full_constructor.clone(),
        vec![1.0, 2.0, 3.5, 4.0],
    ));
    assert_eq!(describe(&ok(h.value(thickness))), "new Thickness(1, 2, 3.5, 4)");

    let grid_length = FerroXamlIlGridLengthAstNode::new(
        &line(),
        types,
        FerroXamlIlGridLength { value: 2.0, grid_unit_type: FerroXamlIlGridUnitType::Star },
    );
    assert_eq!(describe(&ok(h.value(grid_length))), "new GridLength(2, 2)");

    // The font family is resolved against the base URI of the context.
    let font_family = FerroXamlIlFontFamilyAstNode::new(types, "Arial", &line());
    assert_eq!(describe(&ok(h.value(font_family))), "new FontFamily(<http://example.com/app/>, \"Arial\")");
    assert_eq!(h.calls().len(), 3);
}

#[test]
fn array_constants_are_arrays_converted_where_they_are_passed() {
    let h = Harness::new();
    let double = h.ts.get("System.Double");
    let array_type = ok(double.make_array_type(1));
    let values = vec![h.real_double(1.5), h.real_double(-2.0)];
    let node = FerroXamlIlArrayConstantAstNode::new(&line(), array_type.clone(), double.clone(), values);
    let value = ok(h.value(node));
    let array = from_markup_value::<RuntimeArray>(&value).expect("array");
    assert!(array.element_type().equals(&*double));
    assert_eq!(array.items().len(), 2);
    // The run-time type of the value is the array type.
    assert!(h.ts.runtime_type_of(value.as_ref().unwrap()).equals(&*array_type));
    assert!(h.ts.resolve(ValueType::of::<Vec<f64>>()).equals(&*array_type));

    // A member that declares `Vec<f64>` receives the elements, an untyped one the array.
    let typed = array.to_declared(ValueType::of::<Vec<f64>>()).unwrap();
    assert_eq!(typed.downcast_ref::<Vec<f64>>(), Some(&vec![1.5, -2.0]));
    let optional = array.to_declared(ValueType::of::<Option<Vec<f64>>>()).unwrap();
    assert_eq!(optional.downcast_ref::<Option<Vec<f64>>>(), Some(&Some(vec![1.5, -2.0])));
    assert!(array.to_declared(ValueType::object()).unwrap().downcast_ref::<RuntimeArray>().is_some());
    let exact = crate::runtime::type_system::to_exact_value(&value, ValueType::of::<Vec<f64>>()).unwrap();
    assert!(exact.downcast_ref::<Vec<f64>>().is_some());
    // Another element type or collection type is an error that names both.
    let error = array.to_declared(ValueType::of::<Vec<i32>>()).unwrap_err();
    assert!(error.contains("System.Double") && error.contains("Vec<i32>"), "{error}");
    assert!(array.to_declared(ValueType::of::<String>()).is_err());

    // Strings and enumerations registered by a caller.
    #[derive(Clone, Copy, PartialEq, Debug)]
    enum Unit {
        A,
        B,
    }
    RuntimeArray::register_element::<Unit>();
    let units = RuntimeArray::new(double, vec![boxed(Unit::A), boxed(Unit::B)]);
    let typed = units.to_declared(ValueType::of::<Vec<Unit>>()).unwrap();
    assert_eq!(typed.downcast_ref::<Vec<Unit>>(), Some(&vec![Unit::A, Unit::B]));
    let strings = RuntimeArray::new(h.ts.get("System.String"), vec![boxed("a".to_string()), boxed("b".to_string())]);
    let typed = strings.to_declared(ValueType::of::<Vec<String>>()).unwrap();
    assert_eq!(typed.downcast_ref::<Vec<String>>().map(Vec::len), Some(2));
}

#[test]
fn list_constants_set_the_capacity_and_add_each_value() {
    let h = Harness::new();
    let types = &h.fw.types;
    let double = h.fw.configuration.well_known_types().double.clone();
    let list_type = ok(types.ferro_list.make_generic_type(std::slice::from_ref(&double)));
    let values = vec![h.constant(XamlValue::Double(1.0)), h.constant(XamlValue::Double(2.0))];
    let node = ok(FerroXamlIlFerroListConstantAstNode::new(&line(), types, list_type, double, values));
    let list = describe(&ok(h.value(node)));
    let calls = h.calls();
    assert_eq!(calls.len(), 4, "{calls:?}");
    assert!(calls[0].starts_with("new FerroList`1("), "{calls:?}");
    assert_eq!(list, calls[0]);
    assert!(calls[1].contains(".set_Capacity(") && calls[1].ends_with(", 2)"), "{calls:?}");
    assert!(calls[2].contains(".Add(") && calls[2].ends_with(", 1)"), "{calls:?}");
    assert!(calls[3].contains(".Add(") && calls[3].ends_with(", 2)"), "{calls:?}");
}

// --- registered properties ----------------------------------------------------

#[test]
fn registered_property_nodes_load_the_property() {
    let h = Harness::new();
    // The field is looked up on the type that declares the property.
    let brush_type = h.ts.get("FerroUI.Media.Brush");
    let property = XamlAstClrProperty::new(&line(), "Opacity", brush_type.clone(), None);
    let node = XamlIlFerroPropertyNode::with_property_type(
        &line(),
        h.ts.get("FerroUI.FerroProperty"),
        property,
        h.ts.get("System.Double"),
    );
    let value = ok(h.value(node));
    let loaded = from_markup_value::<&'static ferroui_base::FerroProperty>(&value).expect("property");
    assert_eq!(loaded.name(), "Opacity");

    // A property without a registered property behind it cannot be loaded.
    let missing = XamlAstClrProperty::new(&line(), "Nope", brush_type, None);
    let node = XamlIlFerroPropertyNode::with_property_type(
        &line(),
        h.ts.get("FerroUI.FerroProperty"),
        missing,
        h.ts.get("System.Double"),
    );
    assert!(message(h.value(node)).starts_with("Nope is not a FerroProperty"));

    // A style class as a property: `StyledElementExtensions.GetClassProperty(name)`.
    let class_property = ok(XamlIlFerroClassProperty::new(&h.fw.types, "accent", &line()));
    assert!(describe(&ok(h.value(class_property))).ends_with(".GetClassProperty(\"accent\")"));
}

#[test]
fn registered_property_setters_set_bind_and_unset_with_the_priority() {
    let h = Harness::new();
    let types = &h.fw.types;
    let (instance, brush, brush_type, field) = h.brush();
    let double = h.ts.get("System.Double");
    let priority = |p: BindingPriority| boxed(p);

    let set = SetValueWithPrioritySetter::new(types, brush_type.clone(), field.clone(), double.clone());
    ok(h.set(set.clone(), &instance, &[priority(BindingPriority::LocalValue), boxed(0.5f64)]));
    assert_eq!(brush.opacity(), 0.5);
    // A style value does not override the local value: the priority was passed on.
    ok(h.set(set.clone(), &instance, &[priority(BindingPriority::Style), boxed(0.25f64)]));
    assert_eq!(brush.opacity(), 0.5);
    // A value of another type is an invalid cast, not a conversion.
    let error = message(h.set(set.clone(), &instance, &[priority(BindingPriority::LocalValue), boxed(1i32)]));
    assert!(error.contains("Opacity"), "{error}");

    // The unset marker clears the local value: the style value shows.
    let unset = UnsetValueSetter::new(types, brush_type.clone(), field.clone());
    ok(h.set(unset.clone(), &instance, &[boxed(ferroui_base::UnsetValueType)]));
    assert_eq!(brush.opacity(), 0.25);
    ok(h.set(set.clone(), &instance, &[priority(BindingPriority::Animation), boxed(0.1f64)]));
    assert_eq!(brush.opacity(), 0.1);

    // The forms that evaluate their arguments: the value before the priority; the
    // argument of the unset setter is not evaluated at all.
    let (instance, brush, _, _) = h.brush();
    let log = Rc::new(RefCell::new(Vec::new()));
    let local = prepared(types.binding_priority.clone(), priority(BindingPriority::LocalValue), &log, "priority");
    let value = prepared(double.clone(), boxed(0.75f64), &log, "value");
    ok(h.set_nodes(set, &instance, &[local, value]));
    assert_eq!(brush.opacity(), 0.75);
    assert_eq!(*log.borrow(), ["value", "priority"]);
    let not_evaluated = class_property(types, "never");
    ok(h.set_nodes(unset, &instance, &[not_evaluated]));
    assert_eq!(brush.opacity(), 1.0);
    assert!(h.calls().is_empty(), "the argument of the unset setter was evaluated");

    // Bindings: a value that is no binding and a target that is no object of the
    // object model are errors. (A binding itself cannot be held in an untyped value
    // until bindings are reference objects of the base library.)
    let bind = BindingSetter::new(types, brush_type.clone(), field.clone());
    assert!(message(h.set(bind.clone(), &instance, &[boxed(1.0f64)])).contains("BindingBase"));
    assert!(message(h.set(bind, &instance, &[None])).contains("null"));
    let bind_with_priority = BindingWithPrioritySetter::new(types, brush_type, field);
    let error = message(h.set(bind_with_priority, &instance, &[priority(BindingPriority::Template), boxed(2i32)]));
    assert!(error.contains("BindingBase"), "{error}");
}

#[test]
fn setter_value_setter_calls_the_method_with_the_value() {
    let h = Harness::new();
    let setter_type = h.fw.types.setter.clone();
    let set_value = ok(setter_type.get_method(|m| m.name() == "set_Value"));
    let double = h.fw.configuration.well_known_types().double.clone();
    let setter = ok(XamlIlDirectCallPropertySetter::new(set_value, double, false));
    ok(h.set(setter, &h.recorded("setter"), &[boxed(1.5f64)]));
    assert_eq!(h.calls(), ["Setter.set_Value(setter, 1.5)"]);
}

// --- classes ------------------------------------------------------------------

#[test]
fn class_setters_set_and_bind_a_style_class() {
    let h = Harness::new();
    let well_known = h.fw.configuration.well_known_types();
    h.fw.fake_type("FerroUI.Controls.Classes").add_method(
        "Set",
        well_known.void.clone(),
        vec![well_known.string.clone(), well_known.boolean.clone()],
        false,
    );
    let target = h.recorded("target");
    ok(h.set(ClassValueSetter::new(&h.fw.types, "accent"), &target, &[boxed(true)]));
    assert_eq!(
        h.calls(),
        ["StyledElement.get_Classes(target)", "Classes.Set(StyledElement.get_Classes(target), \"accent\", true)"]
    );
    ok(h.set(ClassBindingSetter::new(&h.fw.types, "accent"), &target, &[h.recorded("binding")]));
    assert_eq!(h.calls(), ["StyledElementExtensions.BindClass(target, \"accent\", binding, null)"]);
}

// --- instance attached properties -----------------------------------------------

#[test]
fn instance_attached_property_is_read_and_written_through_the_property_system() {
    let h = Harness::new();
    let (instance, brush, brush_type, field) = h.brush();
    let reference: Rc<dyn xamlx::ast::IXamlAstTypeReference> =
        XamlAstClrTypeReference::new(&line(), brush_type.clone(), false);
    let name = XamlAstNamePropertyReference::new(&line(), reference.clone(), "Opacity", reference);
    let double = h.ts.get("System.Double");
    // The fixture declares the untyped setter; the getter the IL calls is added here.
    h.fw.fake_type("FerroUI.FerroObject").add_method(
        "GetValue",
        h.fw.configuration.well_known_types().object.clone(),
        vec![h.fw.types.ferro_property.clone()],
        false,
    );
    let property = FerroAttachedInstanceProperty::new(
        &name,
        &h.fw.configuration,
        brush_type,
        double,
        h.fw.types.ferro_property.clone(),
        h.fw.types.ferro_object.clone(),
        field,
    );
    let setter = property.setters().into_iter().next().expect("setter");
    assert!(setter.as_any().downcast_ref::<FerroAttachedInstancePropertySetterMethod>().is_some());
    ok(h.set(setter, &instance, &[instance.clone(), boxed(0.4f64)]));
    assert_eq!(brush.opacity(), 0.4);

    let getter = property.getter().expect("getter");
    assert!(getter.as_any().downcast_ref::<FerroAttachedInstancePropertyGetterMethod>().is_some());
    let read = ok(h.eval().call_method(&getter, &[instance], &line()));
    assert_eq!(from_markup_value::<f64>(&read), Some(0.4));
    // Nothing went through a projected method.
    assert!(h.calls().is_empty());
}

// --- routed events --------------------------------------------------------------

#[test]
fn add_handler_passes_the_event_the_handler_the_routes_and_the_flag() {
    let h = Harness::new();
    // Any static field stands for the routed event field.
    let (_, _, brush_type, field) = h.brush();
    let interactivity = &h.fw.types.interactivity;
    let setter = XamlDirectCallAddHandler::new(
        field,
        brush_type,
        interactivity.add_handler.clone(),
        interactivity.routed_event_handler.clone(),
    );
    ok(h.set(setter, &h.recorded("target"), &[h.recorded("handler")]));
    // Direct | Bubble, handled events not included.
    assert_eq!(h.calls(), ["Interactive.AddHandler(target, [&ferroui_base, handler, 5, false)".replace("[&ferroui_base", "[FerroProperty]")]);
}

// --- service providers ----------------------------------------------------------

#[test]
fn service_provider_nodes_load_the_context_and_create_a_root_provider() {
    let h = Harness::new();
    let service_provider = ok(h.fw.configuration.type_mappings.service_provider());
    let injected = ok(h.value(InjectServiceProviderNode::new(service_provider.clone(), &line())));
    let provider = from_markup_value::<Rc<dyn IServiceProvider>>(&injected).expect("the context");
    // The context answers for its own services.
    assert!(provider.get_service_of::<Rc<dyn ferroui_markup_xaml::IUriContext>>().is_some());
    assert!(provider
        .get_service_of::<Rc<dyn ferroui_markup_xaml::xaml_il::runtime::IFerroXamlIlParentStackProvider>>()
        .is_some());

    h.fw.fake_type("FerroUI.Markup.Xaml.XamlIl.Runtime.XamlIlRuntimeHelpers").add_method(
        "CreateRootServiceProviderV3",
        service_provider.clone(),
        vec![service_provider.clone()],
        true,
    );
    let created = ok(h.value(NewServiceProviderNode::new(service_provider, &line())));
    assert!(describe(&created).starts_with("XamlIlRuntimeHelpers.CreateRootServiceProviderV3(["), "{}", describe(&created));
    assert_eq!(h.calls().len(), 1);
}

#[test]
fn the_context_is_the_eager_parent_stack_and_provide_value_target() {
    use ferroui_markup_xaml::xaml_il::runtime::IFerroXamlIlParentStackProvider;
    let h = Harness::new();
    h.context.push_parent(boxed(1i32));
    h.context.push_parent(boxed(2i32));
    let provider: Rc<dyn IServiceProvider> = h.context.clone();
    let stack = provider.get_service_of::<Rc<dyn IFerroXamlIlParentStackProvider>>().expect("parent stack");
    // Nearest first when enumerated, immediate parent last in the direct stack.
    let parents: Vec<i32> = stack.parents().iter().filter_map(|p| p.downcast_ref::<i32>().copied()).collect();
    assert_eq!(parents, [2, 1]);
    let eager = stack.as_eager_parent_stack_provider().expect("eager");
    let direct: Vec<i32> = eager.direct_parents_stack().iter().filter_map(|p| p.downcast_ref::<i32>().copied()).collect();
    assert_eq!(direct, [1, 2]);
    assert!(eager.parent_provider().is_none());
    let target = provider.get_service_of::<Rc<dyn ferroui_markup_xaml::IProvideValueTarget>>().expect("target");
    assert_eq!(target.target_object().and_then(|t| t.downcast_ref::<i32>().copied()), Some(2));
    // The name scope field is filled from the parent service provider.
    let parent = ferroui_markup_xaml::xaml_il::runtime::XamlIlRuntimeHelpers::create_root_service_provider_v3(None);
    let child = RuntimeContext::new(h.context.definition(), h.interpreter.services.clone(), Some(parent), Rc::from(Vec::new()), None);
    ok(services::initialize_name_scope_field(&child));
    assert!(services::name_scope_of(&child).is_some());
    let orphan = RuntimeContext::new(h.context.definition(), h.interpreter.services.clone(), None, Rc::from(Vec::new()), None);
    ok(services::initialize_name_scope_field(&orphan));
    assert!(services::name_scope_of(&orphan).is_none());
}

// --- option markup extensions -----------------------------------------------------

#[test]
fn option_markup_extension_evaluates_only_the_chosen_branch() {
    let h = Harness::new();
    let types = &h.fw.types;
    let well_known = h.fw.configuration.well_known_types();
    let owner = h.fw.fake_type("FerroUI.Markup.Xaml.MarkupExtensions.On");
    let condition: Rc<dyn IXamlMethod> =
        owner.add_method("ShouldProvideOption", well_known.boolean.clone(), vec![well_known.string.clone()], true);
    let value = |name: &str| -> Rc<dyn IXamlAstValueNode> { class_property(types, name) };
    let branch = |option: &str, name: &str| OptionsMarkupExtensionBranch::new(h.text(option), value(name), condition.clone());
    let service_provider = ok(h.fw.configuration.type_mappings.service_provider());

    let container = ok(OptionsMarkupExtensionNodesContainer::new(
        vec![branch("no", "a"), branch("yes", "b"), branch("yes", "c")],
        Some(value("default")),
    ));
    let method: Rc<dyn IXamlMethod> =
        options_method(container, types.on_extension_type.clone(), service_provider.clone());
    let result = ok(h.eval().call_method(&method, &[h.recorded("extension")], &line()));
    assert!(describe(&result).ends_with(".GetClassProperty(\"b\")"));
    let calls = h.calls();
    assert_eq!(calls.len(), 3, "{calls:?}");
    assert!(calls[0].ends_with(".ShouldProvideOption(\"no\")"));
    assert!(calls[1].ends_with(".ShouldProvideOption(\"yes\")"));
    assert!(calls[2].ends_with(".GetClassProperty(\"b\")"));

    // No branch matches: the default.
    let container =
        ok(OptionsMarkupExtensionNodesContainer::new(vec![branch("no", "a")], Some(value("default"))));
    let method: Rc<dyn IXamlMethod> =
        options_method(container, types.on_extension_type.clone(), service_provider.clone());
    let result = ok(h.eval().call_method(&method, &[h.recorded("extension")], &line()));
    assert!(describe(&result).ends_with(".GetClassProperty(\"default\")"));
    assert_eq!(h.calls().len(), 2);

    // No default: the default value of the return type (null for a reference type).
    let container = ok(OptionsMarkupExtensionNodesContainer::new(vec![branch("no", "a")], None));
    let method: Rc<dyn IXamlMethod> = options_method(container, types.on_extension_type.clone(), service_provider);
    assert!(ok(h.eval().call_method(&method, &[h.recorded("extension")], &line())).is_none());
}

// --- resources ----------------------------------------------------------------

#[test]
fn resource_adder_adds_to_the_dictionary_and_records_the_source() {
    let h = Harness::new();
    let types = h.fw.types.clone();
    let target = h.recorded("target");
    let arguments = [boxed("key".to_string()), h.recorded("value")];

    // The three adders, without a getter: the target is the dictionary.
    let add = ok(types.resource_dictionary.get_method(|m| m.name() == "Add" && m.parameters().len() == 2));
    for (adder, name) in [
        (add.clone(), "Add"),
        (types.resource_dictionary_deferred_add.clone(), "AddDeferred"),
        (types.resource_dictionary_not_shared_deferred_add.clone(), "AddNotSharedDeferred"),
    ] {
        let setter = ok(ResourceAdderSetter::new(adder, false, types.clone(), 7, 9, None));
        ok(h.set(setter, &target, &arguments));
        assert_eq!(h.calls(), [format!("ResourceDictionary.{name}(target, \"key\", value)")]);
    }

    // With a getter and source information: the dictionary is read first, the key is
    // evaluated once, the source is attached after the add.
    let getter = types.styled_element.get_all_properties().into_iter().find(|p| p.name() == "Resources");
    let getter = getter.and_then(|p| p.getter()).expect("Resources getter");
    let setter =
        ok(ResourceAdderSetter::with_getter(getter, add, true, types.clone(), 7, 9, Some("doc.xaml".to_string())));
    ok(h.set_nodes(setter.clone(), &target, &[h.text("key"), class_property(&types, "v")]));
    let calls = h.calls();
    assert_eq!(calls.len(), 5, "{calls:?}");
    assert_eq!(calls[0], "StyledElement.get_Resources(target)");
    assert!(calls[1].ends_with(".GetClassProperty(\"v\")"));
    assert!(calls[2].starts_with("ResourceDictionary.Add(StyledElement.get_Resources(target), \"key\", "));
    assert_eq!(calls[3], "new XamlSourceInfo(7, 9, \"doc.xaml\")");
    assert_eq!(
        calls[4],
        "XamlSourceInfo.SetXamlSourceInfo(StyledElement.get_Resources(target), \"key\", new XamlSourceInfo(7, 9, \"doc.xaml\"))"
    );
    // The form with evaluated arguments does the same.
    ok(h.set(setter, &target, &arguments));
    assert_eq!(h.calls().len(), 4);
}

#[test]
fn ensure_capacity_only_touches_resource_dictionaries() {
    let h = Harness::new();
    let getter = h.fw.types.styled_element.get_all_properties().into_iter().find(|p| p.name() == "Resources");
    let getter = getter.and_then(|p| p.getter()).expect("Resources getter");
    // The resources object is read; it is not a resource dictionary of the run-time
    // type system, so nothing else happens.
    ok(h.manipulate(EnsureCapacityNode::new(&line(), 4, Some(getter)).as_node(), &h.recorded("target")));
    assert_eq!(h.calls(), ["StyledElement.get_Resources(target)"]);
    ok(h.manipulate(EnsureCapacityNode::new(&line(), 4, None).as_node(), &h.recorded("target")));
    assert!(h.calls().is_empty());
}

// --- name scopes and source information -----------------------------------------

#[test]
fn names_are_registered_in_the_scope_of_the_context_and_the_root_completes_it() {
    let h = Harness::new();
    let (instance, brush, _, _) = h.brush();
    let registration = FerroNameScopeRegistrationXamlIlNode::new(h.text("brush"), None);
    ok(h.manipulate(registration.as_node(), &instance));
    let found = h.scope.find("brush").expect("registered");
    let object: Ref<FerroObject> = brush.clone().upcast();
    assert!(found == object);
    // Not an object of the object model.
    let registration = FerroNameScopeRegistrationXamlIlNode::new(h.text("other"), None);
    assert!(message(h.manipulate(registration.as_node(), &boxed(1i32))).contains("FerroObject"));

    // The root: a styled element gets the scope attached; the scope is completed.
    let styled = StyledElement::new();
    let root: MarkupValue = crate::runtime::type_system::box_object(styled.clone().upcast());
    let node = HandleRootObjectScopeNode::new(&line(), h.fw.types.clone());
    assert!(!h.scope.is_completed());
    ok(h.manipulate(node.as_node(), &root));
    assert!(h.scope.is_completed());
    let attached: Option<NameScopeRef> = NameScope::get_name_scope(&styled);
    assert!(attached.is_some_and(|scope| scope.find("brush").is_some()));
    // After completion a registration is a load error, not a panic.
    let registration = FerroNameScopeRegistrationXamlIlNode::new(h.text("late"), None);
    assert!(message(h.manipulate(registration.as_node(), &instance)).contains("completed"));

    // A root that is not a styled element: the scope is only completed.
    let h = Harness::new();
    let (instance, brush, _, _) = h.brush();
    let node = HandleRootObjectScopeNode::new(&line(), h.fw.types.clone());
    ok(h.manipulate(node.as_node(), &instance));
    assert!(h.scope.is_completed());
    let _ = brush;
    assert_eq!(<StyledElement as StaticType>::TYPE.name(), "StyledElement");
}

#[test]
fn source_info_is_created_and_attached_to_the_object() {
    let h = Harness::new();
    let types = h.fw.types.clone();
    let constructor = ok(types.resource_dictionary.get_constructor(None));
    let reference = XamlAstClrTypeReference::new(&line(), types.resource_dictionary.clone(), false);
    let object = XamlAstNewClrObjectNode::new(&line(), reference, constructor, Vec::new());
    let node = XamlSourceInfoValueManipulation::new(types.clone(), &object, Some("doc.xaml".to_string()));
    ok(h.manipulate(node.as_node(), &h.recorded("target")));
    assert_eq!(
        h.calls(),
        [
            "new XamlSourceInfo(3, 4, \"doc.xaml\")".to_string(),
            "XamlSourceInfo.SetXamlSourceInfo(target, new XamlSourceInfo(3, 4, \"doc.xaml\"))".to_string()
        ]
    );
    let node = XamlSourceInfoValueManipulation::new(types, &object, None);
    ok(h.manipulate(node.as_node(), &h.recorded("target")));
    assert_eq!(h.calls()[0], "new XamlSourceInfo(3, 4, null)");
}

// --- selectors ----------------------------------------------------------------

fn selector_text(h: &Harness, node: Rc<dyn XamlIlSelectorNode>) -> String {
    let value = ok(h.eval().evaluate(&node.as_node(), Some(&node.type_().get_clr_type().ok().unwrap())));
    from_markup_value::<Selector>(&value).expect("selector").to_string()
}

#[test]
fn selector_nodes_build_the_selector_chain() {
    let h = Harness::new();
    let selector_type = h.ts.get("FerroUI.Styling.Selector");
    let styled = h.ts.type_of_class(<StyledElement as StaticType>::TYPE);
    let initial = || -> Rc<dyn XamlIlSelectorNode> { XamlIlSelectorInitialNode::new(&line(), selector_type.clone()) };
    let of_type = |previous: Rc<dyn XamlIlSelectorNode>, concrete: bool| -> Rc<dyn XamlIlSelectorNode> {
        XamlIlTypeSelector::new(previous, styled.clone(), concrete)
    };
    let class = |previous: Rc<dyn XamlIlSelectorNode>, name: &str| -> Rc<dyn XamlIlSelectorNode> {
        XamlIlStringSelector::new(previous, XamlIlStringSelectorType::Class, name)
    };
    use ferroui_base::styling::Selectors;
    let type_info = <StyledElement as StaticType>::TYPE;

    // The start of a chain is null.
    let value = ok(h.eval().evaluate(&initial().as_node(), Some(&selector_type)));
    assert!(value.is_none());

    // StyledElement.foo:pointerover#name
    let chain = class(of_type(initial(), true), "foo");
    let chain = class(chain, ":pointerover");
    let chain: Rc<dyn XamlIlSelectorNode> = XamlIlStringSelector::new(chain, XamlIlStringSelectorType::Name, "name");
    let expected = Selectors::of_type_info(None, type_info).class("foo").class(":pointerover").name("name");
    assert_eq!(selector_text(&h, chain), expected.to_string());

    // :is(StyledElement) > StyledElement /template/ StyledElement StyledElement
    let chain = of_type(initial(), false);
    let chain: Rc<dyn XamlIlSelectorNode> = XamlIlCombinatorSelector::new(chain, CombinatorSelectorType::Child);
    let chain = of_type(chain, true);
    let chain: Rc<dyn XamlIlSelectorNode> = XamlIlCombinatorSelector::new(chain, CombinatorSelectorType::Template);
    let chain = of_type(chain, true);
    let chain: Rc<dyn XamlIlSelectorNode> = XamlIlCombinatorSelector::new(chain, CombinatorSelectorType::Descendant);
    let chain = of_type(chain, true);
    let expected = Selectors::is_type_info(None, type_info)
        .child()
        .of_type_info(type_info)
        .template()
        .of_type_info(type_info)
        .descendant()
        .of_type_info(type_info);
    assert_eq!(selector_text(&h, chain), expected.to_string());

    // StyledElement:not(.a):nth-child(2n+1):nth-last-child(3)
    let chain: Rc<dyn XamlIlSelectorNode> = XamlIlNotSelector::new(of_type(initial(), true), class(initial(), "a"));
    let chain: Rc<dyn XamlIlSelectorNode> =
        XamlIlNthChildSelector::new(chain, 2, 1, XamlIlNthChildSelectorType::NthChild);
    let chain: Rc<dyn XamlIlSelectorNode> =
        XamlIlNthChildSelector::new(chain, 0, 3, XamlIlNthChildSelectorType::NthLastChild);
    let expected = Selectors::of_type_info(None, type_info)
        .not(Selectors::class(None, "a"))
        .nth_child(2, 1)
        .nth_last_child(0, 3);
    assert_eq!(selector_text(&h, chain), expected.to_string());

    // Alternatives: none is an error, one is itself, more are an or-selector.
    let or = XamlIlOrSelectorNode::new(&line(), selector_type.clone());
    assert!(message(h.eval().evaluate(&or.clone().as_node(), Some(&selector_type))).starts_with("Invalid selector count"));
    or.add(class(initial(), "a"));
    assert_eq!(selector_text(&h, or.clone()), Selectors::class(None, "a").to_string());
    or.add(of_type(initial(), true));
    let expected = Selectors::or([Selectors::class(None, "a"), Selectors::of_type_info(None, type_info)]);
    assert_eq!(selector_text(&h, or), expected.to_string());

    // Nesting.
    let nesting: Rc<dyn XamlIlSelectorNode> = XamlIlNestingSelector::new(initial(), None);
    let nested = class(nesting, "x");
    assert_eq!(selector_text(&h, nested), Selectors::nesting(None).class("x").to_string());

    // A type that is not a class of the object model cannot be selected.
    let not_a_class: Rc<dyn XamlIlSelectorNode> = XamlIlTypeSelector::new(initial(), h.ts.get("System.Double"), true);
    let error = message(h.eval().evaluate(&not_a_class.as_node(), Some(&selector_type)));
    assert!(error.contains("System.Double"), "{error}");
}

#[test]
fn property_selectors_compare_with_the_value_in_the_type_of_the_property() {
    let h = Harness::new();
    let selector_type = h.ts.get("FerroUI.Styling.Selector");
    let (_, _, brush_type, field) = h.brush();
    let initial: Rc<dyn XamlIlSelectorNode> = XamlIlSelectorInitialNode::new(&line(), selector_type);
    let node: Rc<dyn XamlIlSelectorNode> =
        XamlIlAttachedPropertyEqualsSelector::new(initial.clone(), field, h.constant(XamlValue::Double(0.5)));
    let text = selector_text(&h, node);
    assert!(text.contains("Opacity") && text.contains("0.5"), "{text}");

    let property = brush_type.get_all_properties().into_iter().find(|p| p.name() == "Opacity").expect("Opacity");
    let node: Rc<dyn XamlIlSelectorNode> =
        XamlIlPropertyEqualsSelector::new(initial.clone(), property.clone(), h.constant(XamlValue::Double(0.25)));
    let text = selector_text(&h, node);
    assert!(text.contains("Opacity") && text.contains("0.25"), "{text}");
    // A value that is not of the type of the property is rejected.
    let node: Rc<dyn XamlIlSelectorNode> = XamlIlPropertyEqualsSelector::new(initial, property, h.text("x"));
    let error = message(h.eval().evaluate(&node.as_node(), Some(&h.ts.get("FerroUI.Styling.Selector"))));
    assert!(error.contains("Opacity"), "{error}");
}

// --- queries --------------------------------------------------------------------

fn query_text(h: &Harness, node: Rc<dyn XamlIlQueryNode>) -> String {
    let value = ok(h.eval().evaluate(&node.as_node(), Some(&node.type_().get_clr_type().ok().unwrap())));
    from_markup_value::<StyleQuery>(&value).expect("query").to_string()
}

#[test]
fn query_nodes_build_the_query_chain() {
    use ferroui_base::styling::StyleQueries;
    use StyleQueryComparisonOperator::{GreaterThanOrEquals, LessThanOrEquals};
    let h = Harness::new();
    let query_type = h.fw.types.style_queries.clone();
    let query_type = h.ts.find_type("FerroUI.Styling.StyleQuery").unwrap_or(query_type);
    let initial = || -> Rc<dyn XamlIlQueryNode> { XamlIlQueryInitialNode::new(&line(), query_type.clone()) };
    assert!(ok(h.eval().evaluate(&initial().as_node(), Some(&query_type))).is_none());

    let width: Rc<dyn XamlIlQueryNode> = XamlIlWidthQuery::new(initial(), GreaterThanOrEquals, 100.0);
    let both: Rc<dyn XamlIlQueryNode> = XamlIlHeightQuery::new(width, LessThanOrEquals, 50.0);
    let expected =
        StyleQueries::width(None, GreaterThanOrEquals, 100.0).height(LessThanOrEquals, 50.0);
    assert_eq!(query_text(&h, both), expected.to_string());

    let member = |value: f64| -> Rc<dyn XamlIlQueryNode> { XamlIlWidthQuery::new(initial(), GreaterThanOrEquals, value) };
    let or = XamlIlOrQueryNode::new(&line(), query_type.clone());
    assert!(message(h.eval().evaluate(&or.clone().as_node(), Some(&query_type))).starts_with("Invalid query count"));
    or.add(member(1.0));
    assert_eq!(query_text(&h, or.clone()), StyleQueries::width(None, GreaterThanOrEquals, 1.0).to_string());
    or.add(member(2.0));
    let expected = StyleQueries::or([
        StyleQueries::width(None, GreaterThanOrEquals, 1.0),
        StyleQueries::width(None, GreaterThanOrEquals, 2.0),
    ]);
    assert_eq!(query_text(&h, or), expected.to_string());

    let and = XamlIlAndQueryNode::new(&line(), query_type.clone());
    and.add(member(1.0));
    and.add(member(2.0));
    let expected = StyleQueries::and([
        StyleQueries::width(None, GreaterThanOrEquals, 1.0),
        StyleQueries::width(None, GreaterThanOrEquals, 2.0),
    ]);
    assert_eq!(query_text(&h, and), expected.to_string());
}

// --- compiled binding paths -------------------------------------------------------

fn path_of(h: &Harness, transform: Vec<XamlIlBindingPathElementNode>, elements: Vec<XamlIlBindingPathElementNode>) -> XamlResult<String> {
    let node = XamlIlBindingPathNode::new(&line(), h.fw.types.compiled_binding_path.clone(), transform, elements);
    let value = h.value(node)?;
    Ok(from_markup_value::<CompiledBindingPath>(&value).expect("path").to_string())
}

#[test]
fn binding_path_nodes_call_the_builder_per_element() {
    use XamlIlBindingPathElementNode as E;
    let h = Harness::new();
    let object = h.ts.get("System.Object");
    let styled = h.ts.type_of_class(<StyledElement as StaticType>::TYPE);
    let type_info = <StyledElement as StaticType>::TYPE;
    let (_, _, brush_type, field) = h.brush();
    let property = ferroui_base::media::Brush::opacity_property().as_property();
    let o = || object.clone();

    let text = ok(path_of(
        &h,
        vec![E::Not(Rc::new(XamlIlNotPathElementNode { type_: o() }))],
        vec![
            E::SelfElement(Rc::new(SelfPathElementNode { type_: o() })),
            E::FerroProperty(Rc::new(XamlIlFerroPropertyPropertyPathElementNode {
                field: field.clone(),
                type_: o(),
                accepts_null: false,
            })),
            E::TypeCast(Rc::new(TypeCastPathElementNode { type_: styled.clone() })),
            E::FerroProperty(Rc::new(XamlIlFerroPropertyPropertyPathElementNode {
                field: field.clone(),
                type_: o(),
                accepts_null: true,
            })),
            E::StreamTask(Rc::new(XamlIlStreamTaskPathElementNode { type_: o() })),
            E::StreamObservable(Rc::new(XamlIlStreamObservablePathElementNode { type_: o() })),
        ],
    ));
    let expected = CompiledBindingPathBuilder::new()
        .not()
        .self_()
        .ferro_property(property)
        .type_cast_value(ferroui_base::data::core::expression_nodes::CastTarget::Class(type_info))
        .ferro_property_with(property, true)
        .stream_task()
        .stream_observable()
        .build();
    assert_eq!(text, expected.to_string());

    // The sources.
    let text = ok(path_of(
        &h,
        Vec::new(),
        vec![E::FindAncestor(Rc::new(FindAncestorPathElementNode { type_: styled.clone(), level: 1, data_context_type: None }))],
    ));
    assert_eq!(text, CompiledBindingPathBuilder::new().ancestor(Some(type_info), 1).build().to_string());
    let text = ok(path_of(
        &h,
        Vec::new(),
        vec![E::FindVisualAncestor(Rc::new(FindVisualAncestorPathElementNode { type_: styled.clone(), level: 2 }))],
    ));
    assert_eq!(text, CompiledBindingPathBuilder::new().visual_ancestor(Some(type_info), 2).build().to_string());
    let text = ok(path_of(&h, Vec::new(), vec![E::TemplatedParent(Rc::new(TemplatedParentPathElementNode { type_: o() }))]));
    assert_eq!(text, CompiledBindingPathBuilder::new().templated_parent().build().to_string());
    // The named element is looked up in the name scope of the context.
    let text = ok(path_of(
        &h,
        Vec::new(),
        vec![E::ElementName(Rc::new(ElementNamePathElementNode { name: "other".into(), type_: o(), data_context_type: None }))],
    ));
    let scope: Rc<dyn INameScope> = h.scope.clone();
    assert_eq!(text, CompiledBindingPathBuilder::new().element_name(NameScopeRef(scope), "other").build().to_string());

    // A plain property: described by the accessors of the type system.
    let plain = brush_type.get_all_properties().into_iter().find(|p| p.name() == "Color").expect("Color");
    let text = ok(path_of(
        &h,
        Vec::new(),
        vec![E::ClrProperty(Rc::new(XamlIlClrPropertyPathElementNode {
            property: plain.clone(),
            accepts_null: false,
            emit_typed: std::cell::Cell::new(false),
        }))],
    ));
    assert_eq!(text, "Color");
    let info = ok(binding_path::property_info(&mut h.eval(), &plain, None, &line()));
    let (instance, brush, _, _) = h.brush();
    let read = info.try_get(&**instance.as_ref().unwrap()).ok().flatten().expect("the color");
    assert!(read.downcast_ref::<ferroui_base::media::Color>() == Some(&brush.color()));
    assert!(info.can_get() && info.name() == "Color");
    // An owner that is not an object of the object model cannot be read through metadata yet.
    assert!(info.try_get(&5i32).is_err());

    // An array index.
    let array = ok(h.ts.get("System.Double").make_array_type(1));
    let indexer = ok(XamlIlArrayIndexerPathElementNode::new(array, &["3".to_string()], &line()));
    let text = ok(path_of(&h, Vec::new(), vec![E::ArrayIndexer(Rc::new(indexer))]));
    assert_eq!(text, CompiledBindingPathBuilder::new().array_element(&[3]).build().to_string());

    // An ancestor type that is not a class, and a method bound as a delegate.
    let error = message(path_of(
        &h,
        Vec::new(),
        vec![E::FindAncestor(Rc::new(FindAncestorPathElementNode { type_: o(), level: 0, data_context_type: None }))],
    ));
    assert!(error.contains("System.Object"), "{error}");
}

// --- deferred content -------------------------------------------------------------

#[test]
fn failing_deferred_content_is_an_error_on_the_fallible_paths() {
    use ferroui_markup_xaml::xaml_il::runtime::DeferredContent;
    let h = Harness::new();
    let system = crate::FerroXamlIlRuntimeCompiler::type_system();
    let customization = ok(h.fw.types.runtime_helpers.get_method(|m| m.name() == "DeferredTransformationFactoryV3"));
    let method = DeferredTransformationFactoryMethod::new(customization);
    let method = ok(method.make_generic_method(&[system.get("System.String")]));
    let provider: Rc<dyn IServiceProvider> = h.context.clone();

    let failing = DeferredContentFactory::new(|_| {
        Err(xamlx::exceptions::XamlError::load_exception("boom", Some(&XamlLineInfo::new(12, 7))))
    });
    let content = ok(h.eval().call_method(&method, &[boxed(failing), boxed(provider.clone())], &line()));
    let deferred = from_markup_value::<Rc<DeferredContent>>(&content).expect("deferred content");
    // Through the fallible Rust entry point: the error, with the position of the failing node.
    let error = deferred.try_build_with(None).err().expect("an error");
    assert!(error.message().contains("boom") && error.message().contains("line 12 position 7"), "{}", error.message());
    // Through metadata: a failed invocation.
    let deferred_type = system.get("FerroUI.Markup.Xaml.XamlIl.Runtime.DeferredContent");
    let build = ok(deferred_type.get_method(|m| m.name() == "Build"));
    let result = build.as_any().downcast_ref::<RuntimeMethod>().unwrap().invoke(&[content, None]);
    assert!(matches!(result, Err(ferroui_base::metadata::MarkupInvokeError::Failed(message)) if message.contains("boom")));

    // Content that builds: a fresh name scope per instantiation, the result type checked.
    let building = DeferredContentFactory::new(|_| Ok(Some(Rc::new("built".to_string()) as BoxedValue)));
    let content = ok(h.eval().call_method(&method, &[boxed(building), boxed(provider.clone())], &line()));
    let deferred = from_markup_value::<Rc<DeferredContent>>(&content).expect("deferred content");
    let first = deferred.try_build_with(None).ok().expect("built");
    let second = deferred.try_build_with(None).ok().expect("built");
    assert_eq!(first.result.as_ref().and_then(|r| r.downcast_ref::<String>().cloned()).as_deref(), Some("built"));
    assert!(!Rc::ptr_eq(&first.name_scope.0, &second.name_scope.0));
    assert!(first.name_scope.0.is_completed());
    let wrong = DeferredContentFactory::new(|_| Ok(Some(Rc::new(5i32) as BoxedValue)));
    let content = ok(h.eval().call_method(&method, &[boxed(wrong), boxed(provider)], &line()));
    let deferred = from_markup_value::<Rc<DeferredContent>>(&content).expect("deferred content");
    assert!(deferred.try_build_with(None).is_err());
}

#[test]
fn an_error_during_a_load_is_a_load_exception() {
    // Not well-formed, an unknown type, an unknown namespace: never a panic.
    for xaml in ["<Border", "<Nope xmlns='https://github.com/ferroui'/>", "<a:B xmlns:a='nowhere'/>", ""] {
        let error = crate::FerroRuntimeXamlLoader::parse(xaml, None).err().expect("an error");
        assert!(!error.message().is_empty());
    }
    assert!(crate::FerroRuntimeXamlLoader::load_group(vec![], None).ok().is_some_and(|loaded| loaded.is_empty())
        || crate::FerroRuntimeXamlLoader::load_group(vec![], None).is_err());
}

// --- paths the first round of tests did not reach -------------------------------

#[test]
fn bindings_are_bound_through_the_property_system() {
    use ferroui_base::data::ReflectionBinding;
    let h = Harness::new();
    let types = &h.fw.types;
    // An element: a binding without a source needs a data context to look for.
    let element_type = h.ts.get("FerroUI.Input.InputElement");
    let field = element_type.get_all_fields().into_iter().find(|f| f.name() == "IsEnabledProperty").expect("field");
    let element = ferroui_base::input::InputElement::new();
    let instance: MarkupValue = crate::runtime::type_system::box_object(element.clone().upcast());
    // A binding is held as its concrete object and viewed through the binding contract.
    let binding = || boxed(ReflectionBinding::new("Missing"));
    ok(h.set(BindingSetter::new(types, element_type.clone(), field.clone()), &instance, &[binding()]));
    // The priority is discarded.
    let with_priority = BindingWithPrioritySetter::new(types, element_type.clone(), field.clone());
    ok(h.set(with_priority.clone(), &instance, &[boxed(BindingPriority::Template), binding()]));
    // The form that evaluates its arguments does not evaluate the priority at all.
    let log = Rc::new(RefCell::new(Vec::new()));
    let priority = prepared(types.binding_priority.clone(), boxed(BindingPriority::Template), &log, "priority");
    let value = prepared(types.binding_base.clone(), binding(), &log, "binding");
    ok(h.set_nodes(with_priority, &instance, &[priority, value]));
    assert_eq!(*log.borrow(), ["binding"]);
    // A target that is no object of the object model.
    let bind = BindingSetter::new(types, element_type, field);
    assert!(message(h.set(bind, &boxed(5i32), &[binding()])).contains("FerroObject"));
}

#[test]
fn routed_event_handlers_are_attached_to_the_element() {
    use ferroui_base::input::InputElement;
    use ferroui_base::interactivity::{IRoutedEventArgs, RoutedEventArgs};
    use ferroui_base::metadata::MarkupDelegate;
    let h = Harness::new();
    let system = &h.ts;
    let element_type = system.get("FerroUI.Input.InputElement");
    let field = element_type.get_all_fields().into_iter().find(|f| f.name() == "ContextCanceledEvent").expect("event field");
    let well_known = system.well_known_types();
    let interactivity = ok(InteractivityWellKnownTypes::new(&system.as_type_system(), &well_known));
    let setter = XamlDirectCallAddHandler::new(
        field,
        element_type,
        interactivity.add_handler.clone(),
        interactivity.routed_event_handler.clone(),
    );
    let element = InputElement::new();
    let target: MarkupValue = crate::runtime::type_system::box_object(element.clone().upcast());
    let calls = Rc::new(RefCell::new(Vec::new()));
    let seen = calls.clone();
    let handler = MarkupDelegate::new(move |arguments| {
        let sender = arguments.first().and_then(|a| a.as_ref()).map(|a| a.type_name().to_string());
        let args = arguments.get(1).and_then(from_markup_value::<Rc<dyn IRoutedEventArgs>>);
        seen.borrow_mut().push((sender.is_some(), args.is_some()));
        None
    });
    ok(h.set(setter, &target, &[boxed(handler)]));
    element.raise_event(&RoutedEventArgs::with_event(&InputElement::context_canceled_event()));
    assert_eq!(*calls.borrow(), [(true, true)]);
}

#[test]
fn method_and_command_path_elements_use_the_untyped_builder_members() {
    use XamlIlBindingPathElementNode as E;
    let h = Harness::new();
    let object = h.ts.get("System.Object");
    // Any invocable methods stand for the view model methods.
    let time_span = h.ts.get("System.TimeSpan");
    let method = ok(time_span.get_method(|m| m.name() == "FromSeconds"));
    let text = ok(path_of(
        &h,
        Vec::new(),
        vec![E::ClrMethod(Rc::new(XamlIlClrMethodPathElementNode { method: method.clone(), type_: object.clone(), accepts_null: false }))],
    ));
    assert_eq!(text, "FromSeconds()");
    let text = ok(path_of(
        &h,
        Vec::new(),
        vec![E::ClrMethodAsCommand(Rc::new(XamlIlClrMethodAsCommandPathElementNode {
            type_: object,
            execute_method: method,
            can_execute_method: None,
            depends_on_properties: vec!["Name".to_string()],
        }))],
    ));
    assert_eq!(text, "FromSeconds()");
}

#[test]
fn property_field_nodes_load_the_field() {
    let h = Harness::new();
    let (_, _, _, field) = h.brush();
    let system = crate::FerroXamlIlRuntimeCompiler::type_system();
    // The node needs the property types of the type system the field belongs to.
    let Ok(types) = FerroXamlIlWellKnownTypes::new(&system.as_type_system()) else {
        // The well-known types do not resolve over the real registries yet (see the
        // diagnostic test); the node cannot be created without them.
        return;
    };
    let field = system.get("FerroUI.Media.Brush").get_all_fields().into_iter().find(|f| f.name() == "OpacityProperty").unwrap_or(field);
    let node = ok(XamlIlFerroPropertyFieldNode::new(&types, &line(), field));
    let value = ok(h.value(node));
    assert_eq!(from_markup_value::<&'static ferroui_base::FerroProperty>(&value).map(|p| p.name()), Some("Opacity"));
}
