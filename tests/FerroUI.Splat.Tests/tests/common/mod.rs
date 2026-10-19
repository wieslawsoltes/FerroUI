//! The helpers the tests share, ported from Splat.Common.Test: the scopes that save and restore
//! the state of Splat, the test loggers, and the types that tests register.
//!
//! Not ported: `BitmapLoaderScope` and `PlatformModeDetectorScope` - Splat.Drawing is not ported.

#![allow(dead_code)]

mod app_builder_scope;
mod app_locator_scope;
mod dummy_object_class1;
mod dummy_object_class2;
mod dummy_object_class3;
mod format_helper;
mod i_dummy_interface;
mod i_mock_log_target;
mod i_screen;
mod i_view_for;
mod i_view_model_one;
mod internal_locator_scope;
mod mock_screen;
mod mode_detector_scope;
mod text_logger;
mod view_model_one;
mod view_model_two;
mod view_one;
mod view_that_should_not_load;
mod view_two;

pub use app_builder_scope::AppBuilderScope;
pub use app_locator_scope::AppLocatorScope;
pub use dummy_object_class1::DummyObjectClass1;
pub use dummy_object_class2::DummyObjectClass2;
pub use dummy_object_class3::DummyObjectClass3;
pub use format_helper::{FormatHelper, TestException};
pub use i_dummy_interface::IDummyInterface;
pub use i_mock_log_target::IMockLogTarget;
pub use i_screen::IScreen;
pub use i_view_for::{IViewFor, IViewForOf};
pub use i_view_model_one::IViewModelOne;
pub use internal_locator_scope::InternalLocatorScope;
pub use mode_detector_scope::ModeDetectorScope;
pub use text_logger::TextLogger;
pub use view_model_one::ViewModelOne;
pub use view_model_two::ViewModelTwo;

use std::rc::Rc;

/// `IsSameReferenceAs` for reference-counted values of any (possibly unsized) type.
pub fn same<A: ?Sized, B: ?Sized>(a: &Rc<A>, b: &Rc<B>) -> bool {
    std::ptr::addr_eq(Rc::as_ptr(a), Rc::as_ptr(b))
}

/// Whether `action` panics (`Throws<...>()`); the panic is caught.
pub fn panics(action: impl FnOnce()) -> bool {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(action)).is_err()
}

/// The payload of the panic of `action`, or `None` when it does not panic.
pub fn panic_payload(action: impl FnOnce()) -> Option<Box<dyn std::any::Any + Send>> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(action)).err()
}

/// The message of a panic payload (a `&str` or a `String`), or an empty string.
pub fn panic_message(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(message) = payload.downcast_ref::<&str>() {
        (*message).to_owned()
    } else if let Some(message) = payload.downcast_ref::<String>() {
        message.clone()
    } else {
        String::new()
    }
}
