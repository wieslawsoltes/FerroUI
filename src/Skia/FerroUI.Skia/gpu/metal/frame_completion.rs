use ferroui_base::rendering::composition::frames::GpuCompletion;
use std::ffi::c_void;
extern "C" {
    fn ferro_frame_metal_complete(
        queue: *mut c_void,
        context: *mut c_void,
        finished: extern "C" fn(*mut c_void, i32),
    );
}
extern "C" fn finished(context: *mut c_void, success: i32) {
    // SAFETY: one Box transferred to exactly one native callback, including
    // command-buffer creation failure. This callback does not touch UI state.
    let completion = unsafe { Box::from_raw(context.cast::<GpuCompletion>()) };
    if success != 0 {
        completion.complete();
    } else {
        completion.mark_indeterminate();
    }
}
/// # Safety
/// `queue` is the live queue used for Skia's preceding submissions.
pub(super) unsafe fn schedule(queue: *mut c_void, completion: GpuCompletion) {
    let context = Box::into_raw(Box::new(completion)).cast();
    unsafe {
        ferro_frame_metal_complete(queue, context, finished);
    }
}
