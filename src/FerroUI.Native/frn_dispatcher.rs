//! The dispatcher handed to native code: lets it post callbacks to the UI
//! thread from any thread.

use crate::interop::*;
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_microcom::ComPtr;

/// An owned reference to a native callback that is moved to the UI thread.
struct PostedCallback(*mut IFrnActionCallback);

// SAFETY: the reference is only taken on the posting thread and moved; the
// callback is run (and normally released) on the UI thread. Reference
// counting of native objects is atomic.
unsafe impl Send for PostedCallback {}

impl PostedCallback {
    fn run(mut self) {
        let raw = std::mem::replace(&mut self.0, std::ptr::null_mut());
        // SAFETY: `raw` carries the reference taken in `post`.
        if let Some(callback) = unsafe { ComPtr::from_raw(raw) } {
            callback.run();
        }
    }
}

impl Drop for PostedCallback {
    fn drop(&mut self) {
        // Not run (the dispatcher dropped the job): release the reference.
        // SAFETY: `self.0` is null or carries the reference taken in `post`.
        drop(unsafe { ComPtr::from_raw(self.0) });
    }
}

/// The `IFrnDispatcher` implementation.
pub(crate) struct FrnDispatcher;

impl IFrnDispatcherImpl for FrnDispatcher {
    fn post(&self, cb: Option<&IFrnActionCallback>) {
        let Some(cb) = cb else {
            return;
        };
        crate::callback_base::guard((), || {
            // Keep the callback alive until it has run.
            let callback = PostedCallback(ComPtr::from_ref(cb).into_raw());
            Dispatcher::ui_thread().post(move || callback.run(), DispatcherPriority::SEND);
        })
    }
}
