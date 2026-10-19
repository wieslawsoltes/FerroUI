//! `DynamicDataOptions` (`DynamicDataOptions.cs`): the defaults of the options of the binding
//! operators, which an application may change.
//!
//! Deviation (DEVIATIONS.md, DynamicData): upstream's static properties are global; the port's are
//! per thread, as its observables are (docs/porting/reactive.md, section 5.1).

use crate::binding::{BindingOptions, SortAndBindOptions};
use std::cell::{Cell, RefCell};

ferroui_system::ferro_thread_local! {
    static BINDING: Cell<BindingOptions> = Cell::new(BindingOptions::new(BindingOptions::DEFAULT_RESET_THRESHOLD));
    static SORT_AND_BIND: RefCell<SortAndBindOptions> = RefCell::new(SortAndBindOptions::default());
}

/// The defaults of the options of the binding operators (`DynamicDataOptions`).
pub struct DynamicDataOptions;

impl DynamicDataOptions {
    /// The options of `Bind` (`Binding`).
    pub fn binding() -> BindingOptions {
        BINDING.with(Cell::get)
    }

    /// Sets the options of `Bind` (`Binding { set; }`).
    pub fn set_binding(options: BindingOptions) {
        BINDING.with(|binding| binding.set(options));
    }

    /// The options of `SortAndBind` (`SortAndBind`).
    pub fn sort_and_bind() -> SortAndBindOptions {
        SORT_AND_BIND.with(|options| options.borrow().clone())
    }

    /// Sets the options of `SortAndBind` (`SortAndBind { set; }`).
    pub fn set_sort_and_bind(options: SortAndBindOptions) {
        SORT_AND_BIND.with(|current| *current.borrow_mut() = options);
    }
}
