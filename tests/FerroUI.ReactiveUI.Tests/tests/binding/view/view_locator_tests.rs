//! `ViewLocatorTests` (ReactiveUI.Binding.Tests, `View/ViewLocatorTests.cs`).

use super::initialize;
use ferroui_reactiveui::*;
use std::any::Any;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::Rc;

#[test]
fn get_current_when_registered_returns_locator() {
    initialize();

    // with_core_services registers a DefaultViewLocator as the IViewLocator.
    let locator = ViewLocator::current();

    assert!((locator as Rc<dyn Any>).is::<DefaultViewLocator>());
}

#[test]
fn get_current_not_registered_throws_view_locator_not_found_exception() {
    initialize();
    // Reset without rebuilding, so that no IViewLocator is registered.
    RxBindingBuilder::reset_for_testing();

    let payload = catch_unwind(AssertUnwindSafe(|| {
        let _ = ViewLocator::current();
    }))
    .expect_err("there is no view locator");

    assert!(payload.downcast_ref::<ViewLocatorNotFoundException>().is_some());
}
