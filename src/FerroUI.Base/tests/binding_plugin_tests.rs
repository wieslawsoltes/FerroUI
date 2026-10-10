//! Port of the upstream binding plugin tests: `ExceptionValidationPluginTests`
//! and `IndeiValidationPluginTests`.
//!
//! Not ported: `DataAnnotationsValidationPluginTests` (validation attributes
//! are not available).

use super::*;
use crate::data::core::plugins::{
    ExceptionValidationPlugin, IDataValidationPlugin, IPropertyAccessorPlugin, IndeiValidationPlugin,
    InpcPropertyAccessorPlugin,
};
use crate::data::core::{Value, WeakValue};
use crate::data::model::{Event, INotifyDataErrorInfo, INotifyPropertyChanged, Model};
use crate::data::{BindingError, BindingErrorType, BindingNotification, BindingPriority, DataValidationException};
use crate::ferro_model;

/// The message of the managed "argument out of range" error for `value`.
const OUT_OF_RANGE: &str = "Specified argument was out of the range of valid values. (Parameter 'value')";

fn notification(value: &Option<BoxedValue>) -> &BindingNotification {
    value.as_ref().and_then(|v| v.downcast_ref::<BindingNotification>()).expect("a binding notification")
}

#[track_caller]
fn assert_notifications(actual: &[Option<BoxedValue>], expected: &[BindingNotification]) {
    assert_eq!(actual.len(), expected.len());
    for (i, (actual, expected)) in actual.iter().zip(expected).enumerate() {
        assert!(notification(actual) == expected, "notification {i}: {} != {}", notification(actual), expected);
    }
}

mod exception_validation_plugin_tests {
    use super::*;

    pub struct Data {
        must_be_positive: Cell<i32>,
        property_changed: Event<str>,
    }

    impl Data {
        fn must_be_positive(&self) -> i32 {
            self.must_be_positive.get()
        }

        fn set_must_be_positive(&self, value: i32) -> Result<(), BindingError> {
            if value <= 0 {
                return Err(BindingError::message(OUT_OF_RANGE));
            }
            if value != self.must_be_positive.get() {
                self.must_be_positive.set(value);
                self.property_changed.raise("MustBePositive");
            }
            Ok(())
        }
    }

    impl INotifyPropertyChanged for Data {
        fn property_changed(&self) -> &Event<str> {
            &self.property_changed
        }
    }

    ferro_model!(Data, |b| b.notify_property_changed().validated_property::<Value<i32>>(
        "MustBePositive",
        |o| o.must_be_positive(),
        |o, v| o.set_must_be_positive(v)
    ));

    #[test]
    fn produces_binding_notifications() {
        let inpc_accessor_plugin = InpcPropertyAccessorPlugin;
        let validator_plugin = ExceptionValidationPlugin;
        let data = Model::new_model(Data { must_be_positive: Cell::new(0), property_changed: Event::new() });
        let reference = WeakValue::new(&(data.clone() as BoxedValue));
        let accessor = inpc_accessor_plugin.start(&reference, "MustBePositive").expect("an accessor");
        let validator = validator_plugin.start(&reference, "MustBePositive", accessor);
        let result = Recorder::new();

        let recorder = result.clone();
        validator.subscribe(Rc::new(move |x| recorder.push(x)));
        validator.set_value(Some(&boxed(5)), BindingPriority::LocalValue).unwrap();
        validator.set_value(Some(&boxed(-2)), BindingPriority::LocalValue).unwrap();
        validator.set_value(Some(&boxed(6)), BindingPriority::LocalValue).unwrap();

        assert_notifications(
            &result.get(),
            &[
                BindingNotification::new(Some(boxed(0))),
                BindingNotification::new(Some(boxed(5))),
                BindingNotification::with_error(
                    BindingError::message(OUT_OF_RANGE),
                    BindingErrorType::DataValidationError,
                ),
                BindingNotification::new(Some(boxed(6))),
            ],
        );
    }
}

mod indei_validation_plugin_tests {
    use super::*;

    pub struct Data {
        value: Cell<i32>,
        maximum: Cell<i32>,
        error: RefCell<Option<String>>,
        property_changed: Event<str>,
        errors_changed: Event<str>,
    }

    impl Data {
        fn new(maximum: i32) -> Rc<Self> {
            let result = Model::new_model(Self {
                value: Cell::new(0),
                maximum: Cell::new(0),
                error: RefCell::new(None),
                property_changed: Event::new(),
                errors_changed: Event::new(),
            });
            result.set_maximum(maximum);
            result
        }

        fn errors_changed_subscription_count(&self) -> usize {
            self.errors_changed.handler_count()
        }

        fn value(&self) -> i32 {
            self.value.get()
        }

        fn set_value(&self, value: i32) {
            self.value.set(value);
            self.property_changed.raise("Value");
            self.update_error();
        }

        fn set_maximum(&self, value: i32) {
            self.maximum.set(value);
            self.update_error();
        }

        fn update_error(&self) {
            if self.value.get() <= self.maximum.get() {
                if self.error.borrow().is_some() {
                    self.error.replace(None);
                    self.errors_changed.raise("Value");
                }
            } else if self.error.borrow().is_none() {
                self.error.replace(Some(s("Must be less than Maximum")));
                self.errors_changed.raise("Value");
            }
        }
    }

    impl INotifyPropertyChanged for Data {
        fn property_changed(&self) -> &Event<str> {
            &self.property_changed
        }
    }

    impl INotifyDataErrorInfo for Data {
        fn has_errors(&self) -> bool {
            self.error.borrow().is_some()
        }

        fn get_errors(&self, property_name: Option<&str>) -> Vec<BoxedValue> {
            match &*self.error.borrow() {
                Some(error) if property_name == Some("Value") => vec![boxed(error.clone())],
                _ => Vec::new(),
            }
        }

        fn errors_changed(&self) -> &Event<str> {
            &self.errors_changed
        }
    }

    ferro_model!(Data, |b| b
        .notify_property_changed()
        .notify_data_error_info()
        .property::<Value<i32>>("Value", |o| o.value(), |o, v| o.set_value(v)));

    fn validation_error() -> BindingNotification {
        BindingNotification::with_error_and_fallback(
            BindingError::new(DataValidationException::new(Some(boxed(s("Must be less than Maximum"))))),
            BindingErrorType::DataValidationError,
            Some(boxed(6)),
        )
    }

    #[test]
    fn produces_binding_notifications() {
        let inpc_accessor_plugin = InpcPropertyAccessorPlugin;
        let validator_plugin = IndeiValidationPlugin;
        let data = Data::new(5);
        let reference = WeakValue::new(&(data.clone() as BoxedValue));
        let accessor = inpc_accessor_plugin.start(&reference, "Value").expect("an accessor");
        let validator = validator_plugin.start(&reference, "Value", accessor);
        let result = Recorder::new();

        let recorder = result.clone();
        validator.subscribe(Rc::new(move |x| recorder.push(x)));
        validator.set_value(Some(&boxed(5)), BindingPriority::LocalValue).unwrap();
        validator.set_value(Some(&boxed(6)), BindingPriority::LocalValue).unwrap();
        data.set_maximum(10);
        data.set_maximum(5);

        assert_notifications(
            &result.get(),
            &[
                BindingNotification::new(Some(boxed(0))),
                BindingNotification::new(Some(boxed(5))),
                // The value is first signalled without an error as
                // validation hasn't been updated.
                BindingNotification::new(Some(boxed(6))),
                // Then the errors changed event is raised.
                validation_error(),
                // The maximum is changed to 10 so the value is now valid.
                BindingNotification::new(Some(boxed(6))),
                // And the maximum is changed back to 5.
                validation_error(),
            ],
        );
    }

    /// Not from upstream: a model object a binding read from a property typed with its
    /// handle is held as the handle (`Rc<T>` in the box); the validator finds the
    /// contract of the object behind it.
    #[test]
    fn validates_a_model_held_as_its_handle() {
        let inpc_accessor_plugin = InpcPropertyAccessorPlugin;
        let validator_plugin = IndeiValidationPlugin;
        let data = Data::new(5);
        let object = WeakValue::new(&(data.clone() as BoxedValue));
        let handle: BoxedValue = Rc::new(data.clone());
        let reference = WeakValue::new(&handle);
        assert!(validator_plugin.match_(&reference, "Value"));

        let accessor = inpc_accessor_plugin.start(&object, "Value").expect("an accessor");
        let validator = validator_plugin.start(&reference, "Value", accessor);
        let result = Recorder::new();
        let recorder = result.clone();
        validator.subscribe(Rc::new(move |x| recorder.push(x)));
        assert_eq!(data.errors_changed_subscription_count(), 1);
        validator.set_value(Some(&boxed(6)), BindingPriority::LocalValue).unwrap();
        assert_notifications(
            &result.get(),
            &[BindingNotification::new(Some(boxed(0))), BindingNotification::new(Some(boxed(6))), validation_error()],
        );
        validator.unsubscribe();
        assert_eq!(data.errors_changed_subscription_count(), 0);
    }

    #[test]
    fn subscribes_and_unsubscribes() {
        let inpc_accessor_plugin = InpcPropertyAccessorPlugin;
        let validator_plugin = IndeiValidationPlugin;
        let data = Data::new(5);
        let reference = WeakValue::new(&(data.clone() as BoxedValue));
        let accessor = inpc_accessor_plugin.start(&reference, "Value").expect("an accessor");
        let validator = validator_plugin.start(&reference, "Value", accessor);

        assert_eq!(data.errors_changed_subscription_count(), 0);
        validator.subscribe(Rc::new(|_| {}));
        assert_eq!(data.errors_changed_subscription_count(), 1);
        validator.unsubscribe();
        assert_eq!(data.errors_changed_subscription_count(), 0);
    }
}
