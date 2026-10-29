//! The actions on the pointer of an input element (`InputElement/Actions/`).

mod capture_pointer_action;
mod release_pointer_capture_action;

pub use capture_pointer_action::CapturePointerAction;
pub use release_pointer_capture_action::ReleasePointerCaptureAction;
