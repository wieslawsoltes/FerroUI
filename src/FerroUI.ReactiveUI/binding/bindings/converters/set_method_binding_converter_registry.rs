//! `SetMethodBindingConverterRegistry`
//! (`ReactiveUI.Binding/Bindings/Converters/SetMethodBindingConverterRegistry.cs`).

use crate::{ConverterType, ISetMethodBindingConverter};
use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

/// The converters that set values on binding targets in ways of their own
/// (`SetMethodBindingConverterRegistry`). The one with the highest positive affinity for a pair of
/// types wins; on a tie the first registered wins.
///
/// A lookup reads a snapshot of the registrations, so a converter may register others while it is
/// asked for its affinity.
// Deviation (DEVIATIONS.md, ReactiveUI): the registry is per thread (`docs/porting/reactive.md`,
// section 5.1) and keeps upstream's copy-on-write snapshot without its lock.
#[derive(Default)]
pub struct SetMethodBindingConverterRegistry {
    snapshot: RefCell<Option<Rc<Vec<Rc<dyn ISetMethodBindingConverter>>>>>,
}

impl SetMethodBindingConverterRegistry {
    /// An empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    fn current(&self) -> Option<Rc<Vec<Rc<dyn ISetMethodBindingConverter>>>> {
        self.snapshot.borrow().clone()
    }

    /// Registers `converter` (`Register`).
    pub fn register(&self, converter: Rc<dyn ISetMethodBindingConverter>) {
        let mut converters = self.current().map(|snapshot| snapshot.as_ref().clone()).unwrap_or_else(|| Vec::with_capacity(8));
        converters.push(converter);
        *self.snapshot.borrow_mut() = Some(Rc::new(converters));
    }

    /// The converter of the highest positive affinity for `from_type` and `to_type`, either of
    /// which may be unknown, or `None` (`TryGetConverter`).
    pub fn try_get_converter(
        &self,
        from_type: Option<ConverterType>,
        to_type: Option<ConverterType>,
    ) -> Option<Rc<dyn ISetMethodBindingConverter>> {
        let snapshot = self.current()?;
        let mut best: Option<&Rc<dyn ISetMethodBindingConverter>> = None;
        let mut best_score = -1;
        for converter in snapshot.iter() {
            let score = converter.get_affinity_for_objects(from_type, to_type);
            if score <= best_score || score <= 0 {
                continue;
            }
            best_score = score;
            best = Some(converter);
        }
        best.cloned()
    }

    /// Every registered converter, in the order of registration (`GetAllConverters`).
    pub fn get_all_converters(&self) -> Vec<Rc<dyn ISetMethodBindingConverter>> {
        self.current().map(|snapshot| snapshot.as_ref().clone()).unwrap_or_default()
    }
}

impl fmt::Debug for SetMethodBindingConverterRegistry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let count = self.current().map_or(0, |snapshot| snapshot.len());
        write!(f, "SetMethodBindingConverterRegistry: {count} set-method converters registered")
    }
}
