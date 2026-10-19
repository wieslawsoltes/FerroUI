//! `AutoSuspendHelperTests` (`AutoSuspendHelperTests.cs`): the initialization of the suspension
//! helper and the wiring of its events.

use super::view_support::{start, ControlledLifetime, UnsupportedLifetime};
use crate::AutoSuspendHelper;
use ferroui_controls::application_lifetimes::{ClassicDesktopStyleApplicationLifetime, IApplicationLifetime};
use ferroui_reactive::Observer;
use ferroui_reactiveui::RxSuspension;
use ferroui_system::IDisposable;
use std::cell::Cell;
use std::rc::Rc;

#[test]
#[should_panic(expected = "ArgumentNullException")]
fn ctor_with_null_lifetime_throws() {
    let _app = start();
    let _ = AutoSuspendHelper::new(None);
}

#[test]
#[should_panic(expected = "NotSupportedException")]
fn ctor_with_unsupported_lifetime_throws() {
    let _app = start();
    let _ = AutoSuspendHelper::new(Some(&UnsupportedLifetime));
}

#[test]
fn ctor_with_desktop_lifetime_sets_should_persist_state() {
    let _app = start();
    let fixture = ControlledLifetime::new();
    let helper = AutoSuspendHelper::new(Some(&*fixture));

    let notified = Rc::new(Cell::new(false));
    let seen = notified.clone();
    let sub = RxSuspension::suspension_host().should_persist_state().subscribe(Observer::create_all(
        move |d: Rc<dyn IDisposable>| {
            seen.set(true);
            d.dispose();
        },
        |error| panic!("{error}"),
        || {},
    ));

    fixture.raise_exit();

    assert!(notified.get());
    sub.dispose();
    helper.dispose();
}

#[test]
fn on_framework_initialization_completed_pushes_is_launching_new() {
    let _app = start();
    let lifetime = ClassicDesktopStyleApplicationLifetime::new();
    let lifetime: &dyn IApplicationLifetime = &*lifetime;
    let helper = AutoSuspendHelper::new(Some(lifetime));

    let count = Rc::new(Cell::new(0));
    let counted = count.clone();
    let sub = RxSuspension::suspension_host()
        .is_launching_new()
        .subscribe(Observer::create_all(move |_| counted.set(counted.get() + 1), |error| panic!("{error}"), || {}));

    helper.on_framework_initialization_completed();

    assert_eq!(count.get(), 1);
    sub.dispose();
    helper.dispose();
}

#[test]
fn on_unhandled_exception_pushes_should_invalidate_state() {
    let _app = start();
    let lifetime = ClassicDesktopStyleApplicationLifetime::new();
    let lifetime: &dyn IApplicationLifetime = &*lifetime;
    let helper = AutoSuspendHelper::new(Some(lifetime));

    let count = Rc::new(Cell::new(0));
    let counted = count.clone();
    let sub = RxSuspension::suspension_host()
        .should_invalidate_state()
        .subscribe(Observer::create_all(move |_| counted.set(counted.get() + 1), |error| panic!("{error}"), || {}));

    helper.on_unhandled_exception();

    assert_eq!(count.get(), 1);
    sub.dispose();
    helper.dispose();
}
