//! `StubFallbackConverter` (`TestModels/StubFallbackConverter.cs`).

use ferroui_reactiveui::{ConverterType, IBindingFallbackConverter};
use std::any::Any;
use std::rc::Rc;

/// The affinity of the stub.
const STUB_AFFINITY: i32 = 5;

/// The function of the stub: the source type, the input, the target type and the conversion hint,
/// to the converted value.
type StubFallbackConversion = Box<dyn Fn(ConverterType, &dyn Any, ConverterType, Option<&dyn Any>) -> Option<Rc<dyn Any>>>;

/// A fallback converter whose conversion is a function of the test (`StubFallbackConverter`), with
/// an affinity of 5 for every pair of types.
pub struct StubFallbackConverter {
    try_convert: StubFallbackConversion,
}

impl StubFallbackConverter {
    pub fn new(
        try_convert: impl Fn(ConverterType, &dyn Any, ConverterType, Option<&dyn Any>) -> Option<Rc<dyn Any>> + 'static,
    ) -> Self {
        Self { try_convert: Box::new(try_convert) }
    }
}

impl IBindingFallbackConverter for StubFallbackConverter {
    fn get_affinity_for_objects(&self, _from_type: ConverterType, _to_type: ConverterType) -> i32 {
        STUB_AFFINITY
    }

    fn try_convert(
        &self,
        from_type: ConverterType,
        from: &dyn Any,
        to_type: ConverterType,
        conversion_hint: Option<&dyn Any>,
    ) -> Option<Rc<dyn Any>> {
        (self.try_convert)(from_type, from, to_type, conversion_hint)
    }
}
