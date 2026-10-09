//! Port of `IsolationTests.cs` of the upstream unit test project of the
//! headless platform.
//!
//! The statics of the original are values of the session thread, where the
//! application and the references to it live. A weak reference is dead as
//! soon as the last reference is dropped, so the forced collections of the
//! original have no counterpart.

use super::test_application::{ferro_theory, TestApplication};
use crate::FerroTestIsolationLevel;
use ferroui_base::threading::Dispatcher;
use ferroui_base::WeakRef;
use ferroui_controls::Application;
use std::cell::RefCell;
use std::sync::{Arc, Weak};

thread_local! {
    static PREVIOUS_APP_REF: RefCell<Option<WeakRef<Application>>> = const { RefCell::new(None) };
    static PREVIOUS_DISPATCHER_REF: RefCell<Option<Weak<Dispatcher>>> = const { RefCell::new(None) };
}

// The parameter is used to run the test several times with the proper isolation level.
fn application_instance_should_match_isolation_level(_run_index: i32) {
    let current_app = Application::current().expect("the application of the test");
    let current_dispatcher = Dispatcher::ui_thread();

    let previous_app_ref = PREVIOUS_APP_REF.with(|previous| previous.borrow().clone());
    let previous_dispatcher_ref = PREVIOUS_DISPATCHER_REF.with(|previous| previous.borrow().clone());
    if let (Some(previous_app_ref), Some(previous_dispatcher_ref)) = (previous_app_ref, previous_dispatcher_ref) {
        let isolation_level = TestApplication::isolation_level();

        match isolation_level {
            FerroTestIsolationLevel::PerTest => {
                let previous_app = previous_app_ref.upgrade();
                let previous_dispatcher = previous_dispatcher_ref.upgrade();
                assert!(previous_app.is_none(), "Previous Application instance should have been collected.");
                assert!(previous_dispatcher.is_none(), "Previous Dispatcher instance should have been collected.");

                assert!(!previous_app.is_some_and(|previous_app| previous_app.ptr_eq(&current_app)));
                assert!(!previous_dispatcher
                    .is_some_and(|previous_dispatcher| Arc::ptr_eq(&previous_dispatcher, &current_dispatcher)));
            }
            FerroTestIsolationLevel::PerAssembly => {
                let previous_app = previous_app_ref.upgrade();
                let previous_dispatcher = previous_dispatcher_ref.upgrade();
                assert!(previous_app.is_some(), "Previous Application instance should still be alive.");
                assert!(previous_dispatcher.is_some(), "Previous Dispatcher instance should still be alive.");

                assert!(previous_app.is_some_and(|previous_app| previous_app.ptr_eq(&current_app)));
                assert!(previous_dispatcher
                    .is_some_and(|previous_dispatcher| Arc::ptr_eq(&previous_dispatcher, &current_dispatcher)));
            }
        }
    }

    PREVIOUS_APP_REF.with(|previous| *previous.borrow_mut() = Some(current_app.downgrade()));
    PREVIOUS_DISPATCHER_REF.with(|previous| *previous.borrow_mut() = Some(Arc::downgrade(&current_dispatcher)));
}
ferro_theory!(application_instance_should_match_isolation_level {
    run_1: (1),
    run_2: (2),
    run_3: (3),
});
