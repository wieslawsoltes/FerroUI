//! The test types declared by the upstream test file
//! `Converters/ValueConverterTests.cs`.

use ferroui_base::utilities::CultureInfo;
use std::rc::{Rc, Weak};

use ferroui_base::data::converters::IValueConverter;
use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::data::{BindingError, BindingOperations};
use ferroui_base::metadata::MarkupTyped;
use ferroui_base::{ferro_markup_type, BoxedValue, FerroProperty};

use crate::support::TypeModule;

/// A converter that returns the special values of bindings: text for a
/// positive number, the unset marker for zero and the do-nothing marker for
/// a negative number.
pub struct TestConverter {
    this: Weak<TestConverter>,
}

crate::test_identity_eq!(TestConverter);

impl TestConverter {
    pub fn new() -> Rc<TestConverter> {
        Rc::new_cyclic(|this| Self { this: this.clone() })
    }

    /// The shared instance.
    pub fn instance() -> Rc<TestConverter> {
        thread_local! {
            static INSTANCE: Rc<TestConverter> = TestConverter::new();
        }
        INSTANCE.with(Rc::clone)
    }

    /// The converter as the value converter contract.
    pub fn as_value_converter(&self) -> Rc<dyn IValueConverter> {
        self.this.upgrade().expect("the converter is alive while it is used")
    }
}

impl IValueConverter for TestConverter {
    fn convert(
        &self,
        value: Option<&BoxedValue>,
        _target_type: ValueType,
        _parameter: Option<&BoxedValue>,
        _culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        if let Some(i) = value.and_then(|value| value.downcast_ref::<i32>()).copied() {
            if i > 0 {
                return Ok(Some(Rc::new("foo".to_string())));
            }

            if i == 0 {
                return Ok(Some(FerroProperty::unset_value()));
            }

            return Ok(Some(BindingOperations::do_nothing()));
        }

        Ok(Some(Rc::new("(default)".to_string())))
    }

    fn convert_back(
        &self,
        _value: Option<&BoxedValue>,
        _target_type: ValueType,
        _parameter: Option<&BoxedValue>,
        _culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        Err(BindingError::message("The method or operation is not implemented."))
    }
}

ferro_markup_type!(class TestConverter {
    this: Rc<TestConverter>,
    handles: [TestConverter, Rc<TestConverter>, Option<Rc<TestConverter>>],
    namespace: "FerroUI.Markup.Xaml.UnitTests.Converters",
    interfaces: [Rc<dyn IValueConverter>],
    constructors: [() => TestConverter::new],
    fields: [Instance: Rc<TestConverter> => TestConverter::instance],
});

pub(crate) const MODULE: TypeModule = TypeModule {
    types: &[],
    markup_types: &[<TestConverter as MarkupTyped>::MARKUP],
    value_types: || {
        ValueTypes::register_reference::<TestConverter>();
        ValueTypes::register_cast::<TestConverter, Rc<dyn IValueConverter>>(TestConverter::as_value_converter);
    },
};
