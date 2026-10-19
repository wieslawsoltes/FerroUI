//! `AmountViewModel` (`TestModels/AmountViewModel.cs`).
//!
//! The amount is an `f64` where upstream has a `decimal`, which lives in `ferroui-base` until phase
//! 1b moves it.

use ferroui_system::component_model::INotifyPropertyChanged;
use ferroui_system::events::Event;
use std::cell::Cell;
use std::rc::Rc;

/// The writes after which a binding that keeps writing is stopped.
const WRITE_LIMIT: usize = 1_000;

/// A view model whose `Amount` counts its writes and panics past a thousand (`AmountViewModel`).
pub struct AmountViewModel {
    amount: Cell<f64>,
    writes: Cell<usize>,
    property_changed: Event<str>,
}

#[allow(dead_code)]
impl AmountViewModel {
    pub fn new() -> Rc<Self> {
        Rc::new(Self { amount: Cell::new(0.0), writes: Cell::new(0), property_changed: Event::new() })
    }

    /// The number of writes of `Amount` (`Writes`).
    pub fn writes(&self) -> usize {
        self.writes.get()
    }

    pub fn amount(&self) -> f64 {
        self.amount.get()
    }

    pub fn set_amount(&self, value: f64) {
        self.writes.set(self.writes.get() + 1);
        if self.writes.get() > WRITE_LIMIT {
            panic!("The binding kept writing: {value}");
        }
        self.amount.set(value);
        self.property_changed.raise("Amount");
    }
}

impl INotifyPropertyChanged for AmountViewModel {
    fn property_changed(&self) -> &Event<str> {
        &self.property_changed
    }
}
