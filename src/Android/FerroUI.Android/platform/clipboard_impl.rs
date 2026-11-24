//! The clipboard of the system.

use super::clip_data_to_data_transfer_wrapper::ClipDataToDataTransferWrapper;
use super::specific::helpers::clip_data_helper::{ClipDataHelper, CLIP_DATA};
use crate::interop::java::{call_object, call_static_object, call_void, JavaClass, JavaObject, JavaRef, JavaValue};
use crate::interop::natives::sdk_int;
use ferroui_base::input::platform::{ClipboardError, IClipboardImpl, PlatformDataTransfer};
use ferroui_base::input::{IAsyncDataTransfer, LocalBoxFuture};
use std::panic::{catch_unwind, resume_unwind, AssertUnwindSafe};
use std::rc::Rc;

/// Runs `operation` now and returns its outcome as a completed future; a
/// panic (an exception of a call into the system) is raised when the
/// future is awaited, like a faulted task.
fn completed<T: 'static>(
    operation: impl FnOnce() -> Result<T, ClipboardError>,
) -> LocalBoxFuture<Result<T, ClipboardError>> {
    // Boundary (#567): a catch that raises the same panic again when the future is awaited,
    // not an error flow: the errors of the clipboard are values.
    match catch_unwind(AssertUnwindSafe(operation)) {
        Ok(value) => Box::pin(std::future::ready(value)),
        Err(payload) => Box::pin(async move { resume_unwind(payload) }),
    }
}

pub(crate) struct ClipboardImpl {
    clipboard_manager: Option<JavaObject>,
    context: Option<JavaObject>,
}

impl ClipboardImpl {
    /// The clipboard of `context` (`context.GetSystemService(Context.ClipboardService)`).
    pub fn new(context: &JavaObject) -> Rc<ClipboardImpl> {
        let clipboard_manager = call_object(
            context,
            "getSystemService",
            "(Ljava/lang/String;)Ljava/lang/Object;",
            &[JavaValue::String("clipboard")],
        )
        .map(|manager| manager.to_global());
        Rc::new(ClipboardImpl { clipboard_manager, context: Some(context.clone()) })
    }

    fn try_get_data(&self) -> Option<Rc<PlatformDataTransfer>> {
        let clip_data = call_object(
            self.clipboard_manager.as_ref()?,
            "getPrimaryClip",
            "()Landroid/content/ClipData;",
            &[],
        )?;
        Some(ClipDataToDataTransferWrapper::new(clip_data.to_global(), self.context.clone()))
    }

    async fn set_data(
        clipboard_manager: Option<JavaObject>,
        data_transfer: Rc<dyn IAsyncDataTransfer>,
    ) -> Result<(), ClipboardError> {
        let Some(clipboard_manager) = clipboard_manager else {
            return Ok(());
        };

        let Some(clip_data) = ClipDataHelper::get_clip_data(data_transfer).await? else {
            Self::clear(Some(&clipboard_manager));
            return Ok(());
        };

        call_void(
            &clipboard_manager,
            "setPrimaryClip",
            "(Landroid/content/ClipData;)V",
            &[JavaValue::Object(Some(&clip_data))],
        );
        Ok(())
    }

    fn clear(clipboard_manager: Option<&JavaObject>) {
        let Some(clipboard_manager) = clipboard_manager else {
            return;
        };

        if sdk_int() >= 28 {
            call_void(clipboard_manager, "clearPrimaryClip", "()V", &[]);
        } else {
            let empty = crate::interop::java::new_string("");
            let clip_data = call_static_object(
                &JavaClass::find(CLIP_DATA),
                "newPlainText",
                "(Ljava/lang/CharSequence;Ljava/lang/CharSequence;)Landroid/content/ClipData;",
                &[JavaValue::Object(None), JavaValue::Object(Some(&empty))],
            );
            call_void(
                clipboard_manager,
                "setPrimaryClip",
                "(Landroid/content/ClipData;)V",
                &[JavaValue::Object(clip_data.as_ref().map(|clip_data| clip_data as &dyn JavaRef))],
            );
        }
    }
}

impl IClipboardImpl for ClipboardImpl {
    fn try_get_data_async(&self) -> LocalBoxFuture<Result<Option<Rc<dyn IAsyncDataTransfer>>, ClipboardError>> {
        completed(|| Ok(self.try_get_data().map(|data| data as Rc<dyn IAsyncDataTransfer>)))
    }

    fn set_data_async(&self, data_transfer: Rc<dyn IAsyncDataTransfer>) -> LocalBoxFuture<Result<(), ClipboardError>> {
        Box::pin(Self::set_data(self.clipboard_manager.clone(), data_transfer))
    }

    fn clear_async(&self) -> LocalBoxFuture<Result<(), ClipboardError>> {
        completed(|| {
            Self::clear(self.clipboard_manager.as_ref());
            Ok(())
        })
    }
}
