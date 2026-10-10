//! The results an activity receives: of an activity it started, and of a
//! request for permissions.

use std::rc::Rc;

/// An `android.content.Intent`. On another system than Android, where the
/// crate is its logic only, there is none.
#[cfg(target_os = "android")]
pub type Intent = crate::interop::java::JavaObject;
#[cfg(not(target_os = "android"))]
pub type Intent = std::convert::Infallible;

/// `Activity.RESULT_OK`, `RESULT_CANCELED` and `RESULT_FIRST_USER` (the
/// `Result` of the reference).
pub const RESULT_OK: i32 = -1;
pub const RESULT_CANCELED: i32 = 0;
pub const RESULT_FIRST_USER: i32 = 1;

/// `PackageManager.PERMISSION_GRANTED` and `PERMISSION_DENIED` (the
/// `Permission` of the reference).
pub const PERMISSION_GRANTED: i32 = 0;
pub const PERMISSION_DENIED: i32 = -1;

/// The request code, the result code and the intent of the result (an
/// `android.content.Intent`), if any.
pub type ActivityResultHandler = Rc<dyn Fn(i32, i32, Option<&Intent>)>;

/// The request code, the permissions and what was granted of each.
pub type RequestPermissionsResultHandler = Rc<dyn Fn(i32, &[String], &[i32])>;

pub trait IActivityResultHandler {
    fn activity_result(&self) -> Option<ActivityResultHandler>;

    fn set_activity_result(&self, value: Option<ActivityResultHandler>);

    fn request_permissions_result(&self) -> Option<RequestPermissionsResultHandler>;

    fn set_request_permissions_result(&self, value: Option<RequestPermissionsResultHandler>);
}
