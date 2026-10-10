//! Listeners of views of the system whose handlers are closures.
//!
//! Not from the reference, where the binding of the managed runtime gives
//! every listener of the framework of the system as an event
//! (`button.Click += ...`). An application written in Rust that creates a
//! view of the system (a native control it embeds) sets its handler with
//! these functions; the listener is a class of the Java layer that calls
//! back with the number of the handler.

use super::java::{call_void, new_object, JavaClass, JavaRef, JavaValue};
use super::natives::{next_handle, NATIVE_CLICK_LISTENER};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

thread_local! {
    static CLICK_HANDLERS: RefCell<HashMap<i64, Rc<dyn Fn()>>> = RefCell::new(HashMap::new());
}

/// Sets the click listener of `view` (`View.setOnClickListener`). The
/// handler runs on the UI thread, and lives as long as the process: the
/// system does not tell when a view lets go of its listener.
pub fn set_on_click_listener(view: &dyn JavaRef, handler: Rc<dyn Fn()>) {
    let handle = next_handle();
    CLICK_HANDLERS.with(|handlers| handlers.borrow_mut().insert(handle, handler));
    let listener = new_object(&JavaClass::find(NATIVE_CLICK_LISTENER), "(J)V", &[JavaValue::Long(handle)]);
    call_void(
        view,
        "setOnClickListener",
        "(Landroid/view/View$OnClickListener;)V",
        &[JavaValue::Object(Some(&listener))],
    );
}

/// The listener with `handle` was clicked.
pub(crate) fn on_click(handle: i64) {
    let handler = CLICK_HANDLERS.with(|handlers| handlers.borrow().get(&handle).cloned());
    if let Some(handler) = handler {
        handler();
    }
}
