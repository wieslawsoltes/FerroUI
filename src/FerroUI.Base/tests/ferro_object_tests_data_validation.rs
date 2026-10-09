//! Port of the observable-based parts of the upstream `DataValidation` object
//! tests. Each test runs against a direct property and a styled property;
//! `Bound_Validated_String_Property_Can_Be_Set_To_Null` is declared by both
//! upstream test classes with one body, which binds the direct string
//! property in both.

use super::binding_test_support::ViewModel;
use super::*;
use crate::data::{BindingError, BindingPriority, BindingValue, BindingValueType, ReflectionBinding};
use crate::*;
use std::cell::{Cell, RefCell};

type Notification = (BindingValueType, i32, Option<BindingError>);

#[repr(C)]
pub struct Class1 {
    base: FerroObject,
    non_validated_direct: Cell<i32>,
    direct_int: Cell<i32>,
    direct_string: RefCell<Option<String>>,
    notifications: RefCell<Vec<Notification>>,
}

ferro_class!(Class1: FerroObject);

impl FerroObjectImpl for Class1 {
    fn update_data_validation(
        this: &Self,
        property: &'static FerroProperty,
        state: BindingValueType,
        error: Option<&BindingError>,
    ) {
        // Upstream records the value as an object. The notifications that are asserted are of the integer
        // properties: the ones of the string property are not recorded.
        let value = this.get_value_untyped(property);
        let Some(value) = value.downcast_ref::<i32>() else { return };
        this.notifications.borrow_mut().push((state, *value, error.cloned()));
    }
}

impl Class1 {
    ferro_property!(pub fn non_validated_direct_int_property() -> DirectProperty<Class1, i32> {
        FerroProperty::register_direct::<Class1, _>(
            "NonValidatedDirectInt",
            |o| o.direct_int.get(),
            Some(|o, v| {
                o.set_and_raise_cell(Class1::non_validated_direct_int_property(), &o.non_validated_direct, v);
            }),
            0,
        )
    });

    ferro_property!(pub fn validated_direct_int_property() -> DirectProperty<Class1, i32> {
        FerroProperty::register_direct_with::<Class1, _>(
            "ValidatedDirectInt",
            |o| o.direct_int.get(),
            Some(|o, v| {
                o.set_and_raise_cell(Class1::validated_direct_int_property(), &o.direct_int, v);
            }),
            DirectPropertyMetadata::new(Some(0)).with_enable_data_validation(true),
        )
    });

    ferro_property!(pub fn validated_direct_string_property() -> DirectProperty<Class1, Option<String>> {
        FerroProperty::register_direct_with::<Class1, _>(
            "ValidatedDirectString",
            |o| o.direct_string.borrow().clone(),
            Some(|o, v| {
                o.set_and_raise(Class1::validated_direct_string_property(), &o.direct_string, v);
            }),
            DirectPropertyMetadata::new(Some(None)).with_enable_data_validation(true),
        )
    });

    ferro_property!(pub fn non_validated_styled_int_property() -> StyledProperty<i32> {
        FerroProperty::register::<Class1, _>("NonValidatedStyledInt", 0)
    });

    ferro_property!(pub fn validated_styled_int_property() -> StyledProperty<i32> {
        FerroProperty::register_with::<Class1, _>(
            "ValidatedStyledInt",
            StyledPropertyOptions::new(0).enable_data_validation(true),
        )
    });

    pub fn construct() -> Self {
        Self {
            base: FerroObject::construct(),
            non_validated_direct: Cell::new(0),
            direct_int: Cell::new(0),
            direct_string: RefCell::new(None),
            notifications: RefCell::new(Vec::new()),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    fn notifications(&self) -> Vec<Notification> {
        self.notifications.borrow().clone()
    }

    fn validated_direct_string(&self) -> Option<String> {
        self.get_direct_value(Self::validated_direct_string_property())
    }
}

#[repr(C)]
pub struct Class2 {
    base: Class1,
}

ferro_class!(Class2: Class1);
ferro_impl_classes!(Class2: FerroObjectImpl);

impl Class2 {
    fn class_init() {
        once_per_thread!({
            Class1::non_validated_direct_int_property()
                .override_metadata::<Class2>(DirectPropertyMetadata::new(None).with_enable_data_validation(true));
            Class1::non_validated_styled_int_property()
                .override_metadata::<Class2>(StyledPropertyMetadata::new(None).with_enable_data_validation(true));
        });
    }

    pub fn new() -> Ref<Self> {
        Self::class_init();
        instantiate(Self { base: Class1::construct() })
    }
}

macro_rules! data_validation_tests {
    ($module:ident, $property:ident, $non_validated_property:ident) => {
        mod $module {
            use super::*;

            #[test]
            fn bound_validated_string_property_can_be_set_to_null() {
                let source = ViewModel::new();
                source.set_string_value(Some("foo".to_string()));

                let target = Class1::new();
                let binding = ReflectionBinding::new("StringValue");
                binding.set_source(Some(source.clone()));
                target.bind_binding(Class1::validated_direct_string_property(), &binding);

                assert_eq!(Some("foo".to_string()), target.validated_direct_string());

                source.set_string_value(None);

                assert_eq!(None, target.validated_direct_string());
            }

            #[test]
            fn binding_non_validated_property_does_not_call_update_data_validation() {
                let target = Class1::new();
                let source: Subject<BindingValue<i32>> = Subject::new();
                let property = Class1::$non_validated_property();

                target.bind_typed_value(property, source.observable(), BindingPriority::LocalValue);
                source.on_next(BindingValue::new(6));
                source.on_next(BindingValue::binding_error(error("error")));
                source.on_next(BindingValue::data_validation_error(error("error")));
                source.on_next(BindingValue::new(6));

                assert!(target.notifications().is_empty());
            }

            #[test]
            fn binding_validated_property_calls_update_data_validation() {
                let target = Class1::new();
                let source: Subject<BindingValue<i32>> = Subject::new();
                let property = Class1::$property();
                let error1 = error("error1");
                let error2 = error("error2");

                target.bind_typed_value(property, source.observable(), BindingPriority::LocalValue);
                source.on_next(BindingValue::new(6));
                source.on_next(BindingValue::data_validation_error(error1.clone()));
                source.on_next(BindingValue::binding_error(error2.clone()));
                source.on_next(BindingValue::new(7));

                assert_eq!(
                    vec![
                        (BindingValueType::VALUE, 6, None),
                        (BindingValueType::DATA_VALIDATION_ERROR, 6, Some(error1)),
                        (BindingValueType::BINDING_ERROR, 0, Some(error2)),
                        (BindingValueType::VALUE, 7, None),
                    ],
                    target.notifications()
                );
            }

            // Upstream produces binding notification objects; the equivalent
            // untyped values here are boxed binding values.
            #[test]
            fn binding_validated_property_calls_update_data_validation_untyped() {
                let target = Class1::new();
                let source: Subject<BoxedValue> = Subject::new();
                let property = Class1::$property();
                let error1 = error("error1");
                let error2 = error("error2");

                target.bind_property_untyped(property.untyped(), source.observable(), BindingPriority::LocalValue);
                source.on_next(boxed(6));
                source.on_next(boxed(BindingValue::<i32>::data_validation_error(error1.clone())));
                source.on_next(boxed(BindingValue::<i32>::binding_error(error2.clone())));
                source.on_next(boxed(7));

                assert_eq!(
                    vec![
                        (BindingValueType::VALUE, 6, None),
                        (BindingValueType::DATA_VALIDATION_ERROR, 6, Some(error1)),
                        (BindingValueType::BINDING_ERROR, 0, Some(error2)),
                        (BindingValueType::VALUE, 7, None),
                    ],
                    target.notifications()
                );
            }

            // The reference form of the test above: the untyped source
            // produces binding notification objects.
            #[test]
            fn binding_validated_property_calls_update_data_validation_untyped_notification() {
                use crate::data::{BindingErrorType, BindingNotification};

                let target = Class1::new();
                let source: Subject<BoxedValue> = Subject::new();
                let property = Class1::$property();
                let error1 = error("error1");
                let error2 = error("error2");

                target.bind_property_untyped(property.untyped(), source.observable(), BindingPriority::LocalValue);
                source.on_next(boxed(6));
                source.on_next(Rc::new(BindingNotification::with_error(
                    error1.clone(),
                    BindingErrorType::DataValidationError,
                )));
                source.on_next(Rc::new(BindingNotification::with_error(error2.clone(), BindingErrorType::Error)));
                source.on_next(boxed(7));

                assert_eq!(
                    vec![
                        (BindingValueType::VALUE, 6, None),
                        (BindingValueType::DATA_VALIDATION_ERROR, 6, Some(error1)),
                        (BindingValueType::BINDING_ERROR, 0, Some(error2)),
                        (BindingValueType::VALUE, 7, None),
                    ],
                    target.notifications()
                );
            }

            #[test]
            fn binding_overridden_validated_property_calls_update_data_validation() {
                let target = Class2::new();
                let source: Subject<BindingValue<i32>> = Subject::new();
                let property = Class1::$non_validated_property();

                // Class2 overrides the non-validated property metadata to
                // enable data validation.
                target.bind_typed_value(property, source.observable(), BindingPriority::LocalValue);
                source.on_next(BindingValue::new(1));

                assert_eq!(1, target.notifications().len());
            }

            #[test]
            fn disposing_binding_subscription_clears_data_validation() {
                let target = Class1::new();
                let source: Subject<BindingValue<i32>> = Subject::new();
                let property = Class1::$property();
                let error1 = error("error");

                let sub = target.bind_typed_value(property, source.observable(), BindingPriority::LocalValue);
                source.on_next(BindingValue::new(6));
                source.on_next(BindingValue::data_validation_error(error1.clone()));
                sub.dispose();

                assert_eq!(
                    vec![
                        (BindingValueType::VALUE, 6, None),
                        (BindingValueType::DATA_VALIDATION_ERROR, 6, Some(error1)),
                        (BindingValueType::UNSET_VALUE, 6, None),
                    ],
                    target.notifications()
                );
            }

            #[test]
            fn completing_binding_clears_data_validation() {
                let target = Class1::new();
                let source: Subject<BindingValue<i32>> = Subject::new();
                let property = Class1::$property();
                let error1 = error("error");

                target.bind_typed_value(property, source.observable(), BindingPriority::LocalValue);
                source.on_next(BindingValue::new(6));
                source.on_next(BindingValue::data_validation_error(error1.clone()));
                source.on_completed();

                assert_eq!(
                    vec![
                        (BindingValueType::VALUE, 6, None),
                        (BindingValueType::DATA_VALIDATION_ERROR, 6, Some(error1)),
                        (BindingValueType::UNSET_VALUE, 6, None),
                    ],
                    target.notifications()
                );
            }
        }
    };
}

data_validation_tests!(direct_property_tests, validated_direct_int_property, non_validated_direct_int_property);
data_validation_tests!(styled_property_tests, validated_styled_int_property, non_validated_styled_int_property);
