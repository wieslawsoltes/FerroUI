//! `StubSetMethodBindingConverter` (`TestModels/StubSetMethodBindingConverter.cs`).

use ferroui_reactiveui::{ConverterType, ISetMethodBindingConverter};
use std::any::Any;
use std::rc::Rc;

/// The affinity of the stub made with `new`.
const DEFAULT_AFFINITY: i32 = 10;

/// A set-method converter of a fixed affinity whose set returns the new value
/// (`StubSetMethodBindingConverter`).
pub struct StubSetMethodBindingConverter {
    affinity: i32,
}

#[allow(dead_code)]
impl StubSetMethodBindingConverter {
    /// The stub with an affinity of 10.
    pub fn new() -> Self {
        Self::with_affinity(DEFAULT_AFFINITY)
    }

    /// The stub with `affinity`.
    pub fn with_affinity(affinity: i32) -> Self {
        Self { affinity }
    }
}

impl ISetMethodBindingConverter for StubSetMethodBindingConverter {
    fn get_affinity_for_objects(&self, _from_type: Option<ConverterType>, _to_type: Option<ConverterType>) -> i32 {
        self.affinity
    }

    fn perform_set(
        &self,
        _to_target: Option<Rc<dyn Any>>,
        new_value: Option<Rc<dyn Any>>,
        _arguments: Option<&[Option<Rc<dyn Any>>]>,
    ) -> Option<Rc<dyn Any>> {
        new_value
    }
}
