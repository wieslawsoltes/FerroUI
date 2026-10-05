use crate::primitives::{RangeBase, TemplatedControlImpl, Thumb, Track};
use crate::templates::{FuncControlTemplate, FuncTemplateNameScopeExtensions};
use crate::test_support::{test_scope, TestRoot};
use crate::ControlImpl;
use ferroui_base::data::core::Value;
use ferroui_base::data::model::{Event, INotifyPropertyChanged, Model};
use ferroui_base::data::{BindingMode, BindingPriority, ReflectionBinding, TemplateBinding};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{LayoutableImpl, Orientation};
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_model, instantiate, BoxedValue, FerroObject, FerroObjectImpl, Rect, Ref,
    Size, StyledElementImpl, VisualImpl,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[repr(C)]
struct TestRange {
    base: RangeBase,
}

ferro_class!(TestRange: RangeBase);
ferro_impl_classes!(
    TestRange: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl
);

impl TestRange {
    fn new() -> Ref<Self> {
        instantiate(Self { base: RangeBase::construct() })
    }
}

struct TestStackOverflowViewModel {
    setter_invoked_count: Cell<i32>,
    value: Cell<f64>,
    property_changed: Event<str>,
}

impl TestStackOverflowViewModel {
    const MAX_INVOKED_COUNT: i32 = 1000;

    fn new(value: f64) -> Rc<Self> {
        let result = Model::new_model(Self {
            setter_invoked_count: Cell::new(0),
            value: Cell::new(0.0),
            property_changed: Event::new(),
        });
        result.set_value(value);
        result
    }

    fn value(&self) -> f64 {
        self.value.get()
    }

    fn set_value(&self, value: f64) {
        if self.value.get() != value {
            self.setter_invoked_count.set(self.setter_invoked_count.get() + 1);
            if self.setter_invoked_count.get() < Self::MAX_INVOKED_COUNT {
                let mut v = value.trunc();
                if v > 75.0 {
                    v = 75.0;
                }
                if v < 25.0 {
                    v = 25.0;
                }
                self.value.set(v);
            } else {
                self.value.set(value);
            }

            self.property_changed.raise("Value");
        }
    }
}

impl INotifyPropertyChanged for TestStackOverflowViewModel {
    fn property_changed(&self) -> &Event<str> {
        &self.property_changed
    }
}

ferro_model!(TestStackOverflowViewModel, |b| b
    .notify_property_changed()
    .property::<Value<f64>>("Value", |vm| vm.value(), |vm, v| vm.set_value(v)));

struct RangeTestViewModel {
    minimum: Cell<f64>,
    maximum: Cell<f64>,
    value: Cell<f64>,
}

ferro_model!(RangeTestViewModel, |b| b
    .property::<Value<f64>>("Minimum", |vm| vm.minimum.get(), |vm, v| vm.minimum.set(v))
    .property::<Value<f64>>("Maximum", |vm| vm.maximum.get(), |vm, v| vm.maximum.set(v))
    .property::<Value<f64>>("Value", |vm| vm.value.get(), |vm, v| vm.value.set(v)));

#[test]
fn maximum_should_be_coerced_to_minimum() {
    let _scope = test_scope();
    let target = TestRange::new();
    target.set_minimum(100.0);
    target.set_maximum(50.0);
    let _root = TestRoot::with_child(target.clone());

    assert_eq!(100.0, target.minimum());
    assert_eq!(100.0, target.maximum());
}

#[test]
fn changing_data_context_should_not_change_old_data_context() {
    let _scope = test_scope();
    let view_model = Model::new_model(RangeTestViewModel {
        minimum: Cell::new(-5000.0),
        maximum: Cell::new(5000.0),
        value: Cell::new(4000.0),
    });

    let target = TestRange::new();
    target.bind_binding(RangeBase::minimum_property().as_property(), &ReflectionBinding::new("Minimum"));
    target.bind_binding(RangeBase::maximum_property().as_property(), &ReflectionBinding::new("Maximum"));
    target.bind_binding(RangeBase::value_property().as_property(), &ReflectionBinding::new("Value"));

    let _root = TestRoot::with_child(target.clone());
    target.set_data_context(Some(view_model.clone()));
    target.set_data_context(None);

    assert_eq!(4000.0, view_model.value.get());
    assert_eq!(-5000.0, view_model.minimum.get());
    assert_eq!(5000.0, view_model.maximum.get());
}

#[test]
fn value_should_be_coerced_to_range() {
    let _scope = test_scope();
    let target = TestRange::new();
    target.set_minimum(0.0);
    target.set_maximum(50.0);
    target.set_range_value(100.0);
    let _root = TestRoot::with_child(target.clone());

    assert_eq!(0.0, target.minimum());
    assert_eq!(50.0, target.maximum());
    assert_eq!(50.0, target.value());
}

#[test]
fn changing_minimum_should_coerce_value_and_maximum() {
    let _scope = test_scope();
    let target = TestRange::new();
    target.set_minimum(0.0);
    target.set_maximum(100.0);
    target.set_range_value(50.0);
    let _root = TestRoot::with_child(target.clone());

    target.set_minimum(200.0);

    assert_eq!(200.0, target.minimum());
    assert_eq!(200.0, target.maximum());
    assert_eq!(200.0, target.value());
}

#[test]
fn changing_maximum_should_coerce_value() {
    let _scope = test_scope();
    let target = TestRange::new();
    target.set_minimum(0.0);
    target.set_maximum(100.0);
    target.set_range_value(100.0);
    let _root = TestRoot::with_child(target.clone());

    target.set_maximum(50.0);

    assert_eq!(0.0, target.minimum());
    assert_eq!(50.0, target.maximum());
    assert_eq!(50.0, target.value());
}

fn set_value_should_not_cause_stack_overflow(use_xaml_binding: bool) {
    let _scope = test_scope();
    let view_model = TestStackOverflowViewModel::new(50.0);

    let track: Rc<RefCell<Option<Ref<Track>>>> = Rc::new(RefCell::new(None));

    let target = TestRange::new();
    let track_slot = track.clone();
    target.set_template(Some(FuncControlTemplate::new(move |c, scope| {
        let track = Track::new();
        track.set_width(100.0);
        track.set_orientation(Orientation::Horizontal);
        track.bind_binding(
            Track::minimum_property().as_property(),
            &TemplateBinding::new(RangeBase::minimum_property().as_property()).with_mode(BindingMode::TwoWay),
        );
        track.bind_binding(
            Track::maximum_property().as_property(),
            &TemplateBinding::new(RangeBase::maximum_property().as_property()).with_mode(BindingMode::TwoWay),
        );

        track.set_name(Some("PART_Track".to_string()));
        track.set_thumb(Thumb::new());
        let track = track.register_in_name_scope(&**scope);

        if use_xaml_binding {
            let source: Ref<FerroObject> = c.clone().upcast();
            let binding = ReflectionBinding::new("Value").with_mode(BindingMode::TwoWay);
            binding.set_source(Some(Rc::new(source) as BoxedValue));
            binding.set_priority(BindingPriority::Style);
            track.bind_binding(Track::value_property().as_property(), &binding);
        } else {
            track.bind_binding(
                Track::value_property().as_property(),
                &TemplateBinding::new(RangeBase::value_property().as_property()).with_mode(BindingMode::TwoWay),
            );
        }

        *track_slot.borrow_mut() = Some(track.clone());
        track.upcast()
    })));
    target.set_minimum(0.0);
    target.set_maximum(100.0);
    target.set_data_context(Some(view_model.clone()));

    target.bind_binding(
        RangeBase::value_property().as_property(),
        &ReflectionBinding::new("Value").with_mode(BindingMode::TwoWay),
    );

    target.apply_template();
    let track = track.borrow().clone().unwrap();
    track.measure(Size::new(100.0, 0.0));
    track.arrange(Rect::new(0.0, 0.0, 100.0, 0.0));

    assert_eq!(1, view_model.setter_invoked_count.get());

    // A stack overflow was occurring at this point in the reference
    // implementation.
    target.set_range_value(51.001);

    assert_eq!(2, view_model.setter_invoked_count.get());

    let expected = 51.0;

    assert_eq!(expected, view_model.value());
    assert_eq!(expected, target.value());
    assert_eq!(expected, track.value());
}

#[test]
fn set_value_should_not_cause_stack_overflow_xaml_binding() {
    set_value_should_not_cause_stack_overflow(true);
}

#[test]
fn set_value_should_not_cause_stack_overflow_template_binding() {
    set_value_should_not_cause_stack_overflow(false);
}

#[test]
fn coercion_should_not_be_done_during_initialization() {
    let _scope = test_scope();
    let target = TestRange::new();

    target.begin_init();

    let _root = TestRoot::with_child(target.clone());
    target.set_minimum(1.0);
    assert_eq!(0.0, target.value());

    target.set_range_value(50.0);
    target.end_init();

    assert_eq!(50.0, target.value());
}

#[test]
fn coercion_should_be_done_after_initialization() {
    let _scope = test_scope();
    let target = TestRange::new();

    target.begin_init();

    let _root = TestRoot::with_child(target.clone());
    target.set_minimum(1.0);

    target.end_init();

    assert_eq!(1.0, target.value());
}
