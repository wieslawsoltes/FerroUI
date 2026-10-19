//! `RxBindingBuilder` (ReactiveUI.Binding, `Builder/RxBindingBuilder.cs`).

use super::ReactiveUIBindingBuilder;
use ferroui_splat::{AppBuilder, AppLocator, ModernDependencyResolver};
use std::cell::Cell;
use std::rc::Rc;

// Deviation (DEVIATIONS.md, ReactiveUI): a static of the process upstream; per thread here, as the
// service locator is.
ferroui_system::ferro_thread_local! {
    /// Whether ReactiveUI.Binding has been initialized (`_hasBeenInitialized`).
    static HAS_BEEN_INITIALIZED: Cell<bool> = const { Cell::new(false) };
}

/// Creates [`ReactiveUIBindingBuilder`]s and tracks whether one was built (`RxBindingBuilder`).
pub struct RxBindingBuilder;

impl RxBindingBuilder {
    /// A builder over the current resolver of [`AppLocator`] (`CreateReactiveUIBindingBuilder`).
    pub fn create_reactive_ui_binding_builder() -> Rc<ReactiveUIBindingBuilder> {
        ReactiveUIBindingBuilder::new(AppLocator::current_mutable(), Some(AppLocator::current()))
    }

    /// Returns when a builder's `build_app` has run, and panics with an `InvalidOperationException`
    /// message otherwise (`EnsureInitialized`).
    pub fn ensure_initialized() {
        if HAS_BEEN_INITIALIZED.with(Cell::get) {
            return;
        }
        panic!(
            "InvalidOperationException: ReactiveUI.Binding has not been initialized. You must initialize using the builder pattern.\n\n\
             Example:\n\
             RxBindingBuilder::create_reactive_ui_binding_builder()\n    .with_core_services()\n    .build_app();"
        );
    }

    /// Resets the state of the builders, sets a new resolver and marks ReactiveUI.Binding not
    /// initialized (`ResetForTesting`). For tests only.
    pub fn reset_for_testing() {
        AppBuilder::reset_builder_state_for_tests();
        AppLocator::set_locator(Rc::new(ModernDependencyResolver::new()));
        HAS_BEEN_INITIALIZED.with(|initialized| initialized.set(false));
    }

    /// Marks ReactiveUI.Binding initialized (`MarkAsInitialized`); `build_app` calls it.
    pub fn mark_as_initialized() {
        HAS_BEEN_INITIALIZED.with(|initialized| initialized.set(true));
    }
}
