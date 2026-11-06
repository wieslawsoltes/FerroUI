use ferroui_base::rendering::composition::frames::GpuCompletion;
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
};
use wasm_bindgen::prelude::*;

#[wasm_bindgen(raw_module = "./ferroui.js")]
extern "C" {
    #[wasm_bindgen(catch,js_namespace=SharedGlFrames,js_name=complete)]
    fn complete(context: i32, callback: &JsValue) -> Result<bool, JsValue>;
}
ferroui_base::ferro_thread_local! {
    static NEXT:Cell<u64>=const {Cell::new(0)};
    static CALLBACKS:RefCell<HashMap<u64,Closure<dyn FnMut(i32)>>>=RefCell::new(HashMap::new());
}
/// Enqueues an actual WebGL2 fence, yielding between zero-timeout polls. A
/// failed/lost fence quarantines submitted storage; it never signals success
/// from render-job completion. Only page-owned single-thread profiles admit it.
pub fn schedule_webgl_completion(context: i32, completion: GpuCompletion) {
    let id = NEXT.with(|next| {
        let id = next
            .get()
            .checked_add(1)
            .expect("GPU callback identity exhausted");
        next.set(id);
        id
    });
    let signal = completion.clone();
    let callback = Closure::<dyn FnMut(i32)>::new(move |success: i32| {
        if success == 1 {
            signal.complete();
        } else {
            signal.mark_indeterminate();
        }
        // Remove after signaling, outside any map borrow; wakeups may submit
        // further work. wasm-bindgen defers freeing a closure while it runs.
        CALLBACKS.with(|callbacks| {
            callbacks.borrow_mut().remove(&id);
        });
    });
    let function = callback.as_ref().clone();
    CALLBACKS.with(|callbacks| {
        callbacks.borrow_mut().insert(id, callback);
    });
    if !matches!(complete(context, &function), Ok(true)) {
        CALLBACKS.with(|callbacks| {
            callbacks.borrow_mut().remove(&id);
        });
        completion.mark_indeterminate();
    }
}
