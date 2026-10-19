//! `...ViewThreadInvokerTests` (the tests of ReactiveUI.Binding's integration).

use super::binding_support::{start_app, TestObject};
use crate::{FerroObjectHandles, FerroViewThreadInvoker};
use ferroui_reactiveui::IViewThreadInvoker;
use std::any::Any;
use std::rc::Rc;

#[test]
fn instance_is_shared() {
    assert!(Rc::ptr_eq(&FerroViewThreadInvoker::instance(), &FerroViewThreadInvoker::instance()));
}

#[test]
fn claims_with_a_ferro_object_claims_it() {
    let _app = start_app();
    let target = FerroObjectHandles::handle_of(&TestObject::new());

    assert!(FerroViewThreadInvoker::instance().claims(&target));
}

#[test]
fn claims_with_something_else_claims_nothing() {
    let target: Rc<dyn Any> = Rc::new(0_i32);

    assert!(!FerroViewThreadInvoker::instance().claims(&target));
}

#[test]
fn check_access_on_the_owning_thread_is_true() {
    let _app = start_app();
    let target = FerroObjectHandles::handle_of(&TestObject::new());

    assert!(FerroViewThreadInvoker::instance().check_access(&target));
}

// Not ported: CheckAccess_OnAnotherThread_IsFalse - real threads (an object of FerroUI cannot be
// handed to another thread)
// Not ported: Post_FromAnotherThread_RunsTheCallbackOnTheOwningThread - real threads (an object of
// FerroUI cannot be handed to another thread)
// Not ported: Post_WithANullCallback_Throws - argument validation
