//! `StubBindingTypeConverter` (`TestModels/StubBindingTypeConverter.cs`), with
//! `StubConversionResult` (`TestModels/StubConversionResult.cs`): the result of the stub's
//! function is the converted value, or `None` for a failure, as the Rust signature of the
//! conversion has it.

use ferroui_reactiveui::{ConverterType, IBindingTypeConverter};
use std::any::Any;
use std::rc::Rc;

/// The affinity of the stub.
const STUB_AFFINITY: i32 = 10;

/// The function of the stub: the input and the conversion hint, to the converted value.
type StubConversion = Box<dyn Fn(Option<&dyn Any>, Option<&dyn Any>) -> Option<Rc<dyn Any>>>;

/// A typed converter of the given types whose conversion is a function of the test
/// (`StubBindingTypeConverter`), with an affinity of 10.
pub struct StubBindingTypeConverter {
    from_type: ConverterType,
    to_type: ConverterType,
    try_convert: StubConversion,
}

impl StubBindingTypeConverter {
    pub fn new(
        from_type: ConverterType,
        to_type: ConverterType,
        try_convert: impl Fn(Option<&dyn Any>, Option<&dyn Any>) -> Option<Rc<dyn Any>> + 'static,
    ) -> Self {
        Self { from_type, to_type, try_convert: Box::new(try_convert) }
    }
}

impl IBindingTypeConverter for StubBindingTypeConverter {
    fn from_type(&self) -> ConverterType {
        self.from_type
    }

    fn to_type(&self) -> ConverterType {
        self.to_type
    }

    fn get_affinity_for_objects(&self) -> i32 {
        STUB_AFFINITY
    }

    fn try_convert_typed(&self, from: Option<&dyn Any>, conversion_hint: Option<&dyn Any>) -> Option<Rc<dyn Any>> {
        (self.try_convert)(from, conversion_hint)
    }
}
