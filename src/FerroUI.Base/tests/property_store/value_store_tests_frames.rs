//! Port of the upstream value store frame tests.
//!
//! Upstream builds the frames from styles. The styling system is not part of
//! this test target, so a test frame holding constant values and bindings
//! stands in for the style instance.

use super::super::*;
use crate::data::{BindingPriority, BindingValue, BindingValueType};
use crate::property_store::{
    BindingEntry, BindingSource, FrameType, IValueEntry, ImmediateValueFrame, ValueFrame, ValueFrameBase,
};
use crate::*;
use std::any::Any;
use std::rc::Rc;

test_class!(Class1: FerroObject);

impl Class1 {
    ferro_property!(pub fn foo_property() -> StyledProperty<String> {
        FerroProperty::register::<Class1, _>("Foo", s("foodefault"))
    });

    ferro_property!(pub fn bar_property() -> StyledProperty<String> {
        FerroProperty::register_with::<Class1, _>("Bar", StyledPropertyOptions::new(s("bardefault")).inherits(true))
    });
}

/// The stand-in for a style instance.
struct TestFrame {
    base: ValueFrameBase,
}

impl ValueFrame for TestFrame {
    fn base(&self) -> &ValueFrameBase {
        &self.base
    }

    fn get_is_active(&self) -> (bool, bool) {
        (true, false)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// The stand-in for a setter with a constant value.
struct TestValueEntry {
    property: &'static StyledProperty<String>,
    value: String,
}

impl IValueEntry for TestValueEntry {
    fn property(&self) -> &'static FerroProperty {
        self.property
    }

    fn has_value(&self) -> bool {
        true
    }

    fn try_get_value(&self, out: &mut dyn Any) -> bool {
        match out.downcast_mut::<Option<String>>() {
            Some(out) => {
                *out = Some(self.value.clone());
                true
            }
            None => false,
        }
    }

    fn get_value_boxed(&self) -> Option<BoxedValue> {
        Some(Rc::new(self.value.clone()))
    }

    fn get_data_validation_state(&self) -> Option<(BindingValueType, Option<crate::data::BindingError>)> {
        None
    }

    fn unsubscribe(&self) {}
}

/// Instances a "style" setting `Foo` to "foo" and binding `Bar` to `source`.
fn instance_style(target: &Ref<Class1>, source: &Subject<String>) -> Rc<dyn ValueFrame> {
    // Register in declaration order.
    let (foo, bar) = (Class1::foo_property(), Class1::bar_property());
    let frame: Rc<dyn ValueFrame> =
        Rc::new(TestFrame { base: ValueFrameBase::new(BindingPriority::Style, FrameType::Style) });
    frame.base().add(Rc::new(TestValueEntry { property: foo, value: s("foo") }));
    frame.base().add(BindingEntry::new(target, Rc::downgrade(&frame), bar, BindingSource::Typed(source.observable())));
    frame
}

type PropertyChange = (&'static str, Option<String>, String);

fn record_changes(target: &Ref<Class1>) -> Recorder<PropertyChange> {
    let result: Recorder<PropertyChange> = Recorder::new();
    let r = result.clone();
    target.property_changed(move |e| r.push((e.property().name_static(), old_string(e), new_string(e))));
    result
}

trait NameStatic {
    fn name_static(&self) -> &'static str;
}

impl NameStatic for &'static FerroProperty {
    fn name_static(&self) -> &'static str {
        let property: &'static FerroProperty = self;
        property.name()
    }
}

#[test]
fn adding_frame_raises_property_changed() {
    let target = Class1::new();
    let subject = Subject::behavior(s("bar"));
    let frame = instance_style(&target, &subject);
    let result = record_changes(&target);

    target.values().add_frame(&target, frame);

    assert_eq!(
        vec![("Foo", Some(s("foodefault")), s("foo")), ("Bar", Some(s("bardefault")), s("bar"))],
        result.get()
    );
}

#[test]
fn removing_frame_raises_property_changed() {
    let target = Class1::new();
    let subject = Subject::behavior(s("bar"));
    let frame = instance_style(&target, &subject);

    target.values().add_frame(&target, frame.clone());

    let result = record_changes(&target);

    target.values().remove_frame(&target, &frame);

    assert_eq!(
        vec![("Bar", Some(s("bar")), s("bardefault")), ("Foo", Some(s("foo")), s("foodefault"))],
        result.get()
    );
}

#[test]
fn removing_frame_unsubscribes_binding() {
    let target = Class1::new();
    let obs = Subject::test(s("bar"));
    let frame = instance_style(&target, &obs);

    target.values().add_frame(&target, frame.clone());
    assert_eq!(1, obs.subscriber_count());

    target.values().remove_frame(&target, &frame);
    assert_eq!(0, obs.subscriber_count());
}

// Upstream binds with a binding object of style priority; an observable
// binding of style priority goes through the same immediate value frame.
#[test]
fn disposing_binding_removes_immediate_value_frame() {
    let target = Class1::new();
    let source: Subject<String> = Subject::new();
    let expression = target.bind(Class1::foo_property(), source.observable(), BindingPriority::Style);
    let value_store = target.values();

    assert_eq!(1, value_store.frames().len());
    assert!(value_store.frames()[0].as_any().is::<ImmediateValueFrame>());

    expression.dispose();

    assert_eq!(0, value_store.frames().len());
}

#[test]
fn completing_observable_removes_immediate_value_frame() {
    let target = Class1::new();
    let source: Subject<BindingValue<String>> = Subject::behavior(bv("foo"));

    target.bind_value(Class1::foo_property(), source.observable(), BindingPriority::Animation);

    let value_store = target.values();
    assert_eq!(1, value_store.frames().len());
    assert!(value_store.frames()[0].as_any().is::<ImmediateValueFrame>());

    source.on_completed();

    assert_eq!(0, value_store.frames().len());
}
