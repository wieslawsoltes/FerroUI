//! `BindingConverters` (`ReactiveUI.Binding/Bindings/Converters/BindingConverters.cs`).

use crate::ConverterService;
use std::cell::RefCell;
use std::rc::Rc;

ferroui_system::ferro_thread_local! {
    /// The service set with `set_service`, or the one created on the first read (`_current`).
    static CURRENT: RefCell<Option<Rc<ConverterService>>> = const { RefCell::new(None) };
}

/// The converter service that the bindings use (`BindingConverters`). The builders of ReactiveUI
/// and ReactiveUI.Binding set it once they registered their converters.
// Deviation (DEVIATIONS.md, ReactiveUI): the service is per thread, as the bindings are
// (`docs/porting/reactive.md`, section 5.1).
pub struct BindingConverters;

impl BindingConverters {
    /// The current service (`Current`): until one is set, an empty service created on the first
    /// read.
    pub fn current() -> Rc<ConverterService> {
        CURRENT.with(|slot| slot.borrow_mut().get_or_insert_with(|| Rc::new(ConverterService::new())).clone())
    }

    /// Makes `service` the current one (`SetService`).
    pub fn set_service(service: Rc<ConverterService>) {
        CURRENT.with(|slot| *slot.borrow_mut() = Some(service));
    }
}
