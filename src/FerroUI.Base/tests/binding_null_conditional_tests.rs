//! Port of the upstream `NullConditionalBindingTests`: the null-conditional
//! operator in binding paths.
//!
//! The upstream tests load markup; here the bindings are created directly, a
//! compiled binding where the upstream test compiles its bindings and a
//! string-path binding where it does not. The stream test streams an
//! observable instead of a task, and the method tests reach the method
//! through a plain member (the member is never read: its owner is null).

use super::binding_test_support::*;
use super::*;
use crate::data::core::expression_nodes::CastTarget;
use crate::data::core::plugins::ObservableValue;
use crate::data::core::{Maybe, Value, ValueTypes};
use crate::data::model::ModelTypes;
use crate::data::{
    BindingBase, BindingChainException, BindingError, BindingValueType, CompiledBinding, ReflectionBinding,
};
use crate::logging::{ILogSink, LogArea, LogEventLevel, Logger};
use crate::{
    ferro_class, ferro_impl_classes, ferro_property, instantiate, AttachedProperty, FerroObject, FerroObjectImpl,
    FerroProperty, Ref, StaticType, StyledElement, StyledElementImpl, StyledProperty, StyledPropertyOptions,
};
use std::any::Any;
use std::fmt::Display;

fn register_types() {
    once_per_thread!({
        ValueTypes::register_object::<First>();
        ValueTypes::register_object::<Second>();
        ValueTypes::register_object::<Third>();
        ModelTypes::register::<Ref<First>>(|b| {
            b.property::<Maybe<Ref<Second>>>("Second", |o| o.second(), |o, v| o.set_second(v))
        });
        ModelTypes::register::<Ref<Second>>(|b| {
            b.property::<Maybe<Ref<Third>>>("Third", |o| o.third(), |o, v| o.set_third(v))
                .read_only::<Maybe<ObservableValue>>("Observable", |o| o.observable.borrow().clone())
        });
        ModelTypes::register::<Ref<Third>>(|b| {
            b.property::<Maybe<String>>("Final", |o| o.final_(), |o, v| o.set_final(v))
                .read_only::<Value<String>>("Greeting", |o| o.greeting())
        });
        // The properties are looked up by name by string paths.
        First::styled_second_property();
        Second::styled_third_property();
        Third::styled_final_property();
        Grid::row_property();
    });
}

#[repr(C)]
pub struct First {
    base: StyledElement,
    second: RefCell<Option<Ref<Second>>>,
}

ferro_class!(First: StyledElement);
ferro_impl_classes!(First: FerroObjectImpl, StyledElementImpl);

impl First {
    ferro_property!(pub fn styled_second_property() -> StyledProperty<Option<Ref<Second>>> {
        FerroProperty::register::<First, _>("StyledSecond", None)
    });

    pub fn new() -> Ref<Self> {
        register_types();
        instantiate(Self { base: StyledElement::construct(), second: RefCell::new(None) })
    }

    pub fn second(&self) -> Option<Ref<Second>> {
        self.second.borrow().clone()
    }

    pub fn set_second(&self, value: Option<Ref<Second>>) {
        self.second.replace(value);
    }

    pub fn set_styled_second(&self, value: Option<Ref<Second>>) {
        self.set_value(Self::styled_second_property(), value)
    }

    fn second_step() -> Step {
        plain_prop::<Ref<First>, Maybe<Ref<Second>>>("Second", Out::Object, |o| o.second(), |o, v| o.set_second(v))
    }

    fn styled_second_step() -> Step {
        Step::Ferro(Self::styled_second_property())
    }
}

#[repr(C)]
pub struct Second {
    base: StyledElement,
    third: RefCell<Option<Ref<Third>>>,
    observable: RefCell<Option<ObservableValue>>,
}

ferro_class!(Second: StyledElement);
ferro_impl_classes!(Second: FerroObjectImpl, StyledElementImpl);

impl Second {
    ferro_property!(pub fn styled_third_property() -> StyledProperty<Option<Ref<Third>>> {
        FerroProperty::register::<Second, _>("StyledThird", None)
    });

    pub fn construct() -> Self {
        Self { base: StyledElement::construct(), third: RefCell::new(None), observable: RefCell::new(None) }
    }

    pub fn new() -> Ref<Self> {
        register_types();
        instantiate(Self::construct())
    }

    pub fn third(&self) -> Option<Ref<Third>> {
        self.third.borrow().clone()
    }

    pub fn set_third(&self, value: Option<Ref<Third>>) {
        self.third.replace(value);
    }

    fn third_step() -> Step {
        plain_prop::<Ref<Second>, Maybe<Ref<Third>>>("Third", Out::Object, |o| o.third(), |o, v| o.set_third(v))
    }

    fn styled_third_step() -> Step {
        Step::Ferro(Self::styled_third_property())
    }

    fn observable_step() -> Step {
        plain_read_only_prop::<Ref<Second>, Maybe<ObservableValue>>("Observable", Out::Object, |o| {
            o.observable.borrow().clone()
        })
    }
}

/// A top-level type used to test casting in a binding path.
#[repr(C)]
pub struct DerivedSecond {
    base: Second,
}

ferro_class!(DerivedSecond: Second);
ferro_impl_classes!(DerivedSecond: FerroObjectImpl, StyledElementImpl);

#[repr(C)]
pub struct Third {
    base: StyledElement,
    final_: RefCell<Option<String>>,
}

ferro_class!(Third: StyledElement);
ferro_impl_classes!(Third: FerroObjectImpl, StyledElementImpl);

impl Third {
    ferro_property!(pub fn styled_final_property() -> StyledProperty<Option<String>> {
        FerroProperty::register::<Third, _>("StyledFinal", None)
    });

    pub fn final_(&self) -> Option<String> {
        self.final_.borrow().clone()
    }

    pub fn set_final(&self, value: Option<String>) {
        self.final_.replace(value);
    }

    pub fn greeting(&self) -> String {
        s("Hello!")
    }

    fn final_step() -> Step {
        plain_prop::<Ref<Third>, Maybe<String>>("Final", Out::String, |o| o.final_(), |o, v| o.set_final(v))
    }

    fn styled_final_step() -> Step {
        Step::Ferro(Self::styled_final_property()).with_out(Out::String)
    }

    fn greeting_step() -> Step {
        plain_read_only_prop::<Ref<Third>, Value<String>>("Greeting", Out::String, |o| o.greeting())
    }
}

static_type!(Grid);

impl Grid {
    ferro_property!(pub fn row_property() -> AttachedProperty<i32> {
        FerroProperty::register_attached::<Grid, FerroObject, _>("Row", 0)
    });

    fn row_step() -> Step {
        Step::Attached { owner: Grid::TYPE, property: Self::row_property() }
    }
}

/// A text box that records the data validation state of its text.
#[repr(C)]
pub struct ErrorCollectingTextBox {
    base: StyledElement,
    error: RefCell<Option<BindingError>>,
    error_state: Cell<BindingValueType>,
}

ferro_class!(ErrorCollectingTextBox: StyledElement);
ferro_impl_classes!(ErrorCollectingTextBox: StyledElementImpl);

impl FerroObjectImpl for ErrorCollectingTextBox {
    fn update_data_validation(
        this: &Self,
        property: &'static FerroProperty,
        state: BindingValueType,
        error: Option<&BindingError>,
    ) {
        if property.id() == Self::text_property().id() {
            this.error.replace(error.cloned());
            this.error_state.set(state);
        }
        Self::parent_update_data_validation(this, property, state, error);
    }
}

use crate::FerroObjectImplExt;

impl ErrorCollectingTextBox {
    ferro_property!(pub fn text_property() -> StyledProperty<Option<String>> {
        FerroProperty::register_with::<ErrorCollectingTextBox, _>("Text",
            StyledPropertyOptions::new(None).enable_data_validation(true))
    });

    pub fn new() -> Ref<Self> {
        instantiate(Self {
            base: StyledElement::construct(),
            error: RefCell::new(None),
            error_state: Cell::new(BindingValueType::VALUE),
        })
    }

    pub fn text(&self) -> Option<String> {
        self.get_value(Self::text_property())
    }
}

struct TestLogger {
    messages: RefCell<Vec<String>>,
}

impl TestLogger {
    fn create() -> Rc<Self> {
        let result = Rc::new(Self { messages: RefCell::new(Vec::new()) });
        Logger::set_thread_sink(Some(result.clone()));
        result
    }

    fn messages(&self) -> Vec<String> {
        self.messages.borrow().clone()
    }
}

impl ILogSink for TestLogger {
    fn is_enabled(&self, level: LogEventLevel, area: &str) -> bool {
        level >= LogEventLevel::Warning && area == LogArea::BINDING
    }

    fn log(&self, _level: LogEventLevel, _area: &str, _source: Option<&dyn Any>, message_template: &str) {
        self.messages.borrow_mut().push(message_template.to_string());
    }

    fn log_with_values(
        &self,
        _level: LogEventLevel,
        _area: &str,
        _source: Option<&dyn Any>,
        message_template: &str,
        _property_values: &[&dyn Display],
    ) {
        self.messages.borrow_mut().push(message_template.to_string());
    }
}

/// Binds the text of a new text box to `path` (written `text` in markup) and
/// then sets the data context, as loading and showing the upstream window
/// does.
fn create_target(
    compile_bindings: bool,
    path: &Path,
    text: &str,
    target_null_value: Option<&str>,
    data: &Ref<First>,
) -> Ref<ErrorCollectingTextBox> {
    let target = ErrorCollectingTextBox::new();
    let binding: Rc<dyn BindingBase> = if compile_bindings {
        let binding = CompiledBinding::new(path.compiled());
        if let Some(value) = target_null_value {
            binding.set_target_null_value(Some(boxed(s(value))));
        }
        binding
    } else {
        let binding = ReflectionBinding::new(text);
        let resolver = path.type_resolver();
        binding.set_type_resolver(Some(Rc::new(move |namespace, name| {
            if name == "DerivedSecond" {
                Some(CastTarget::Class(DerivedSecond::TYPE))
            } else {
                resolver(namespace, name)
            }
        })));
        if let Some(value) = target_null_value {
            binding.set_target_null_value(Some(boxed(s(value))));
        }
        binding
    };
    target.bind_binding(ErrorCollectingTextBox::text_property(), &*binding);
    target.set_data_context(Some(boxed(data.clone())));
    target
}

#[track_caller]
fn assert_no_error(text_box: &ErrorCollectingTextBox, expected_text: Option<&str>, log: &TestLogger) {
    assert_eq!(text_box.text(), expected_text.map(s));
    assert!(text_box.error.borrow().is_none());
    assert_eq!(text_box.error_state.get(), BindingValueType::VALUE);
    assert!(log.messages().is_empty());
}

#[track_caller]
fn assert_chain_error(text_box: &ErrorCollectingTextBox, expression: &str, error_point: &str, log: &TestLogger) {
    let error = text_box.error.borrow().clone().expect("an error is reported");
    let error = error.inner().downcast_ref::<BindingChainException>().expect("a binding chain error").clone();
    let messages = log.messages();

    assert_eq!(messages.len(), 1);
    assert_eq!(text_box.text(), None);
    assert_eq!(error.expression(), Some(expression));
    assert_eq!(error.expression_error_point(), Some(error_point));
    assert_eq!(text_box.error_state.get(), BindingValueType::BINDING_ERROR);
    assert_eq!(
        messages[0],
        "An error occurred binding {Property} to {Expression} at {ExpressionErrorPoint}: {Message}"
    );
}

fn first_with_second() -> Ref<First> {
    let data = First::new();
    data.set_second(Some(Second::new()));
    data
}

fn first_with_styled_second() -> Ref<First> {
    let data = First::new();
    data.set_styled_second(Some(Second::new()));
    data
}

fn for_each_flavour(test: impl Fn(bool)) {
    for compile_bindings in [false, true] {
        test(compile_bindings);
        Logger::set_thread_sink(None);
    }
}

#[test]
fn should_report_error_without_null_conditional_operator_for_clr_property() {
    // Testing the baseline: should report a null error without null conditionals.
    for_each_flavour(|compile_bindings| {
        let log = TestLogger::create();
        let path = Path::of(First::second_step()).then(Second::third_step()).then(Third::final_step());
        let data = first_with_second();
        let text_box = create_target(compile_bindings, &path, "Second.Third.Final", None, &data);

        assert_chain_error(&text_box, "Second.Third.Final", "Third", &log);
    });
}

#[test]
fn should_report_error_without_null_conditional_operator_for_ferro_property() {
    // Testing the baseline: should report a null error without null conditionals.
    for_each_flavour(|compile_bindings| {
        let log = TestLogger::create();
        let path = Path::of(First::styled_second_step())
            .then(Second::styled_third_step())
            .then(Third::styled_final_step());
        let data = first_with_styled_second();
        let text_box = create_target(compile_bindings, &path, "StyledSecond.StyledThird.StyledFinal", None, &data);

        assert_chain_error(&text_box, "StyledSecond.StyledThird.StyledFinal", "StyledThird", &log);
    });
}

#[test]
fn should_not_report_error_with_null_conditional_operator_for_clr_property_1() {
    for_each_flavour(|compile_bindings| {
        let log = TestLogger::create();
        let path = Path::of(First::second_step())
            .then_null_conditional(Second::third_step())
            .then(Third::final_step());
        let data = First::new();
        let text_box = create_target(compile_bindings, &path, "Second?.Third.Final", None, &data);

        assert_no_error(&text_box, None, &log);
    });
}

#[test]
fn should_not_report_error_with_null_conditional_operator_for_clr_property_2() {
    for_each_flavour(|compile_bindings| {
        let log = TestLogger::create();
        let path = Path::of(First::second_step())
            .then(Second::third_step())
            .then_null_conditional(Third::final_step());
        let data = first_with_second();
        let text_box = create_target(compile_bindings, &path, "Second.Third?.Final", None, &data);

        assert_no_error(&text_box, None, &log);
    });
}

#[test]
fn should_not_report_error_with_null_conditional_operator_for_ferro_property() {
    for_each_flavour(|compile_bindings| {
        let log = TestLogger::create();
        let path = Path::of(First::styled_second_step())
            .then(Second::styled_third_step())
            .then_null_conditional(Third::styled_final_step());
        let data = first_with_styled_second();
        let text_box = create_target(compile_bindings, &path, "StyledSecond.StyledThird?.StyledFinal", None, &data);

        assert_no_error(&text_box, None, &log);
    });
}

#[test]
fn should_not_report_error_with_null_conditional_operator_for_stream() {
    for_each_flavour(|compile_bindings| {
        let log = TestLogger::create();
        let path = Path::of(First::second_step())
            .then_null_conditional(Second::observable_step())
            .then(Step::Stream(Out::String));
        let data = First::new();
        let text_box = create_target(compile_bindings, &path, "Second?.Observable^", None, &data);

        assert_no_error(&text_box, None, &log);
    });
}

#[test]
fn should_not_report_error_with_null_conditional_operator_for_attached_property() {
    for_each_flavour(|compile_bindings| {
        let log = TestLogger::create();
        let path = Path::of(First::second_step()).then_null_conditional(Grid::row_step());
        let data = First::new();
        let text_box = create_target(compile_bindings, &path, "Second?.(Grid.Row)", None, &data);

        assert_no_error(&text_box, None, &log);
    });
}

#[test]
fn should_not_report_error_with_null_conditional_operator_before_method_for_clr_property() {
    for_each_flavour(|compile_bindings| {
        let log = TestLogger::create();
        let path = Path::of(First::second_step())
            .then(Second::third_step())
            .then_null_conditional(Third::greeting_step());
        let data = first_with_second();
        let text_box = create_target(compile_bindings, &path, "Second.Third?.Greeting", None, &data);

        assert_no_error(&text_box, None, &log);
    });
}

#[test]
fn should_not_report_error_with_null_conditional_operator_before_method_for_ferro_property() {
    for_each_flavour(|compile_bindings| {
        let log = TestLogger::create();
        let path = Path::of(First::styled_second_step())
            .then(Second::styled_third_step())
            .then_null_conditional(Third::greeting_step());
        let data = first_with_styled_second();
        let text_box = create_target(compile_bindings, &path, "StyledSecond.StyledThird?.Greeting", None, &data);

        assert_no_error(&text_box, None, &log);
    });
}

#[test]
fn should_use_target_null_value_with_null_conditional_operator_for_clr_property() {
    for_each_flavour(|compile_bindings| {
        let log = TestLogger::create();
        let path = Path::of(First::second_step())
            .then(Second::third_step())
            .then_null_conditional(Third::final_step());
        let data = first_with_second();
        let text_box = create_target(compile_bindings, &path, "Second.Third?.Final", Some("ItsNull"), &data);

        assert_no_error(&text_box, Some("ItsNull"), &log);
    });
}

#[test]
fn should_use_target_null_value_with_null_conditional_operator_for_ferro_property() {
    for_each_flavour(|compile_bindings| {
        let log = TestLogger::create();
        let path = Path::of(First::styled_second_step())
            .then(Second::styled_third_step())
            .then_null_conditional(Third::styled_final_step());
        let data = first_with_styled_second();
        let text_box =
            create_target(compile_bindings, &path, "StyledSecond.StyledThird?.StyledFinal", Some("ItsNull"), &data);

        assert_no_error(&text_box, Some("ItsNull"), &log);
    });
}

#[test]
fn should_use_target_null_value_with_short_circuited_null_conditional_operator_for_clr_property() {
    for_each_flavour(|compile_bindings| {
        let log = TestLogger::create();
        let path = Path::of(First::second_step())
            .then_null_conditional(Second::third_step())
            .then(Third::final_step());
        let data = First::new();
        let text_box = create_target(compile_bindings, &path, "Second?.Third.Final", Some("ItsNull"), &data);

        assert_no_error(&text_box, Some("ItsNull"), &log);
    });
}

#[test]
fn should_use_target_null_value_with_short_circuited_null_conditional_operator_for_ferro_property() {
    for_each_flavour(|compile_bindings| {
        let log = TestLogger::create();
        let path = Path::of(First::styled_second_step())
            .then_null_conditional(Second::styled_third_step())
            .then(Third::styled_final_step());
        let data = First::new();
        let text_box =
            create_target(compile_bindings, &path, "StyledSecond?.StyledThird.StyledFinal", Some("ItsNull"), &data);

        assert_no_error(&text_box, Some("ItsNull"), &log);
    });
}

#[test]
fn should_not_report_error_with_null_conditional_operator_after_cast() {
    for_each_flavour(|compile_bindings| {
        let log = TestLogger::create();
        let path = Path::of(First::second_step())
            .then(Step::Cast(CastTarget::Class(DerivedSecond::TYPE)))
            .then_null_conditional(Second::third_step())
            .then(Third::final_step());
        let data = First::new();
        let text_box =
            create_target(compile_bindings, &path, "((local:DerivedSecond)Second)?.Third.Final", None, &data);

        assert_no_error(&text_box, None, &log);
    });
}

#[test]
fn paths_match_their_text() {
    // The compiled paths of the tests above are the paths their text states.
    register_types();
    let path = Path::of(First::second_step()).then_null_conditional(Second::third_step()).then(Third::final_step());
    assert_eq!(path.text(), "Second?.Third.Final");
    let path = Path::of(First::second_step()).then_null_conditional(Grid::row_step());
    assert_eq!(path.text(), "Second?.(Grid.Row)");
    let path = Path::of(First::second_step())
        .then_null_conditional(Second::observable_step())
        .then(Step::Stream(Out::String));
    assert_eq!(path.text(), "Second?.Observable^");
}
