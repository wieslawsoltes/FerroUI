//! Port of the upstream `MarkupExtensions/TestValueConverter`.

use ferroui_base::utilities::CultureInfo;
use std::cell::RefCell;
use std::rc::{Rc, Weak};

use ferroui_base::data::converters::IValueConverter;
use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::data::BindingError;
use ferroui_base::metadata::MarkupTyped;
use ferroui_base::{ferro_markup_type, BoxedValue};

use crate::support::TypeModule;

/// A converter that appends [`append`](Self::append) to the text of the
/// value.
pub struct TestValueConverter {
    this: Weak<TestValueConverter>,
    append: RefCell<Option<String>>,
}

crate::test_identity_eq!(TestValueConverter);

impl TestValueConverter {
    pub fn new() -> Rc<Self> {
        Rc::new_cyclic(|this| Self { this: this.clone(), append: RefCell::new(None) })
    }

    pub fn append(&self) -> Option<String> {
        self.append.borrow().clone()
    }

    pub fn set_append(&self, value: Option<String>) {
        self.append.replace(value);
    }

    /// The converter as the value converter contract.
    pub fn as_value_converter(&self) -> Rc<dyn IValueConverter> {
        self.this.upgrade().expect("the converter is alive while it is used")
    }
}

impl IValueConverter for TestValueConverter {
    fn convert(
        &self,
        value: Option<&BoxedValue>,
        _target_type: ValueType,
        _parameter: Option<&BoxedValue>,
        _culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        // `value + Append`: the text of a null operand is empty.
        let value = value.map(|value| ValueTypes::to_display_string(Some(value))).unwrap_or_default();
        Ok(Some(Rc::new(format!("{value}{}", self.append().unwrap_or_default()))))
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

ferro_markup_type!(class TestValueConverter {
    this: Rc<TestValueConverter>,
    handles: [TestValueConverter, Rc<TestValueConverter>, Option<Rc<TestValueConverter>>],
    interfaces: [Rc<dyn IValueConverter>],
    constructors: [() => TestValueConverter::new],
    properties: [
        Append: Option<String> { get: TestValueConverter::append, set: TestValueConverter::set_append },
    ],
});

/// The test types of this file.
pub(crate) const MODULE: TypeModule = TypeModule {
    types: &[],
    markup_types: &[<TestValueConverter as MarkupTyped>::MARKUP],
    value_types: || {
        ValueTypes::register_reference::<TestValueConverter>();
        ValueTypes::register_cast::<TestValueConverter, Rc<dyn IValueConverter>>(
            TestValueConverter::as_value_converter,
        );
    },
};
