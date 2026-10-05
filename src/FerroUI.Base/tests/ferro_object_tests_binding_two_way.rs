//! Port of the two-way binding parts of the upstream `Binding` object tests:
//! `SetValue_Should_Not_Cause_StackOverflow_And_Have_Correct_Values` and the
//! `TwoWay_Binding_*` tests.
//!
//! Not ported: the two `*_Indexer` tests (indexers are covered by the
//! indexer suite) and the three tests that upstream skips
//! (`TwoWay_Binding_Should_Not_Update_Source_When_Higher_Priority_*`,
//! `TwoWay_Style_Binding_Should_Not_Update_Source_When_StyleTrigger_Value_Set`).

use super::*;
use crate::data::core::Value;
use crate::data::model::{Event, INotifyPropertyChanged, Model};
use crate::data::{BindingMode, IndexerBinding, ReflectionBinding};
use crate::{
    ferro_class, ferro_impl_classes, ferro_model, ferro_property, instantiate, FerroObject, FerroObjectImpl,
    FerroProperty, StyledElement, StyledElementImpl, StyledProperty,
};

test_class!(Class1: FerroObject);

impl Class1 {
    ferro_property!(pub fn double_value_property() -> StyledProperty<f64> {
        FerroProperty::register::<Class1, _>("DoubleValue", 0.0)
    });

    fn double_value(&self) -> f64 {
        self.get_value(Self::double_value_property())
    }

    fn set_double_value(&self, value: f64) {
        self.set_value(Self::double_value_property(), value)
    }
}

/// The equivalent of the upstream text block: an element with a text
/// property.
#[repr(C)]
pub struct TextBlock {
    base: StyledElement,
}

ferro_class!(TextBlock: StyledElement);
ferro_impl_classes!(TextBlock: FerroObjectImpl, StyledElementImpl);

impl TextBlock {
    ferro_property!(pub fn text_property() -> StyledProperty<Option<String>> {
        FerroProperty::register::<TextBlock, _>("Text", None)
    });
}

struct TestStackOverflowViewModel {
    setter_invoked_count: Cell<i32>,
    value: Cell<f64>,
    property_changed: Event<str>,
}

impl TestStackOverflowViewModel {
    const MAX_INVOKED_COUNT: i32 = 1000;

    fn new() -> Rc<Self> {
        Model::new_model(Self { setter_invoked_count: Cell::new(0), value: Cell::new(0.0), property_changed: Event::new() })
    }

    fn value(&self) -> f64 {
        self.value.get()
    }

    fn set_value(&self, value: f64) {
        if self.value.get() != value {
            self.setter_invoked_count.set(self.setter_invoked_count.get() + 1);
            if self.setter_invoked_count.get() < Self::MAX_INVOKED_COUNT {
                let mut new_value = value.trunc();
                if new_value > 75.0 {
                    new_value = 75.0;
                }
                if new_value < 25.0 {
                    new_value = 25.0;
                }
                self.value.set(new_value);
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
    .property::<Value<f64>>("Value", |o| o.value(), |o, v| o.set_value(v)));

struct TestTwoWayBindingViewModel {
    value: Cell<f64>,
    setter_called: Cell<bool>,
}

impl TestTwoWayBindingViewModel {
    fn new() -> Rc<Self> {
        Model::new_model(Self { value: Cell::new(0.0), setter_called: Cell::new(false) })
    }

    fn value(&self) -> f64 {
        self.value.get()
    }

    fn set_value(&self, value: f64) {
        self.value.set(value);
        self.setter_called.set(true);
    }
}

ferro_model!(TestTwoWayBindingViewModel, |b| b.property::<Value<f64>>("Value", |o| o.value(), |o, v| o
    .set_value(v)));

fn two_way_binding(source: BoxedValue) -> Rc<ReflectionBinding> {
    let binding = ReflectionBinding::new("Value").with_mode(BindingMode::TwoWay);
    binding.set_source(Some(source));
    binding
}

#[test]
fn set_value_should_not_cause_stack_overflow_and_have_correct_values() {
    let view_model = TestStackOverflowViewModel::new();
    view_model.set_value(50.0);

    let target = Class1::new();

    target.bind_binding(Class1::double_value_property(), &two_way_binding(view_model.clone()));

    let child = Class1::new();

    child.bind_binding(
        Class1::double_value_property(),
        &IndexerBinding::new(target.clone().upcast(), Class1::double_value_property(), BindingMode::TwoWay),
    );

    assert_eq!(view_model.setter_invoked_count.get(), 1);

    // Issues #855 and #824 were causing a stack overflow at this point.
    target.set_double_value(51.001);

    assert_eq!(view_model.setter_invoked_count.get(), 2);

    let expected = 51.0;

    assert_eq!(view_model.value(), expected);
    assert_eq!(target.double_value(), expected);
    assert_eq!(child.double_value(), expected);
}

#[test]
fn two_way_binding_should_update_source() {
    let target = Class1::new();
    let source = TestTwoWayBindingViewModel::new();

    target.bind_binding(Class1::double_value_property(), &two_way_binding(source.clone()));

    target.set_double_value(123.4);

    assert!(source.setter_called.get());
    assert_eq!(source.value(), 123.4);
}

#[test]
fn two_way_binding_should_not_call_setter_on_creation() {
    let target = Class1::new();
    let source = TestTwoWayBindingViewModel::new();

    target.bind_binding(Class1::double_value_property(), &two_way_binding(source.clone()));

    assert!(!source.setter_called.get());
}

#[test]
fn two_way_binding_should_not_fail_with_null_data_context() {
    let target = instantiate(TextBlock { base: StyledElement::construct() });
    target.set_data_context(None);

    target.bind_binding(TextBlock::text_property(), &ReflectionBinding::new("Missing").with_mode(BindingMode::TwoWay));
}
