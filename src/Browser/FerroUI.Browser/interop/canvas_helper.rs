use super::JsObject;
use crate::browser_top_level_impl::BrowserTopLevelImpl;
use ferroui_base::utilities::HandlerList;
use std::rc::Rc;
use wasm_bindgen::prelude::*;

/// What the script side knows about a WebGL context.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GlInfo {
    pub context_id: i32,
    pub fbo_id: u32,
    pub stencils: i32,
    pub samples: i32,
    pub depth: i32,
}

#[wasm_bindgen(raw_module = "./ferroui.js")]
extern "C" {
    /// A canvas and the render target created for it.
    pub type CanvasSurface;

    #[wasm_bindgen(method, getter, js_name = targetId)]
    pub fn target_id(this: &CanvasSurface) -> i32;

    #[wasm_bindgen(method, getter)]
    pub fn width(this: &CanvasSurface) -> f64;

    #[wasm_bindgen(method, getter)]
    pub fn height(this: &CanvasSurface) -> f64;

    #[wasm_bindgen(method, getter)]
    pub fn scaling(this: &CanvasSurface) -> f64;

    /// Creates a canvas in `container` and a render target of the first of
    /// `modes` the browser supports.
    ///
    /// `thread_id` is 0, or the id of the thread that renders (what
    /// `pthread_self` returns there). With an id the script transfers the
    /// control of the canvas to the worker of that thread and creates no
    /// render target here: the worker creates it and reports it with
    /// [`on_render_target_registered`]. The thread must have called
    /// [`initialize_worker`](crate::rendering::initialize_worker) before.
    ///
    /// For the render thread of the page the id is
    /// [`RenderWorker::canvas_thread_id`](crate::rendering::RenderWorker::canvas_thread_id),
    /// which is
    /// [`PENDING_RENDER_THREAD`](crate::rendering::PENDING_RENDER_THREAD)
    /// while that thread has not reported itself: the script then transfers
    /// the control of the canvas at once and posts it to the worker when the
    /// thread has.
    #[wasm_bindgen(static_method_of = CanvasSurface, js_name = create)]
    pub fn create_render_target_surface(
        container: &JsObject,
        modes: &[i32],
        top_level_id: i32,
        thread_id: i32,
    ) -> CanvasSurface;

    #[wasm_bindgen(static_method_of = CanvasSurface, js_name = destroy)]
    pub fn destroy(canvas_surface: &CanvasSurface);
}

/// The kind of a render target a worker reports as rendered in software.
pub const RENDER_TARGET_KIND_SOFTWARE: i32 = 1;

/// The kind of a render target a worker reports as rendered with WebGL.
pub const RENDER_TARGET_KIND_WEB_GL: i32 = 2;

thread_local! {
    // Per thread: the report arrives on the thread whose worker created the
    // render target, and that thread is the one that wants to hear of it.
    static RENDER_TARGET_REGISTERED: HandlerList<dyn Fn(i32, i32)> = HandlerList::new();
}

/// Subscribes this thread to the render targets its worker creates; the
/// arguments are the id of the target and its kind
/// ([`RENDER_TARGET_KIND_WEB_GL`] or [`RENDER_TARGET_KIND_SOFTWARE`]).
/// Returns the token of the subscription.
pub fn add_render_target_registered(handler: Rc<dyn Fn(i32, i32)>) -> u64 {
    RENDER_TARGET_REGISTERED.with(|handlers| handlers.add(handler))
}

/// Ends a subscription to the render targets of this thread.
pub fn remove_render_target_registered(token: u64) -> bool {
    RENDER_TARGET_REGISTERED.with(|handlers| handlers.remove(token))
}

/// The worker of this thread created the render target of a canvas whose
/// control was transferred to it.
///
/// Not from upstream, where the render thread asks its registry for the
/// target on each frame. See `docs/porting/browser-render-worker.md`,
/// section 4.
#[wasm_bindgen(js_name = CanvasHelper_OnRenderTargetRegistered)]
pub fn on_render_target_registered(target_id: i32, kind: i32) {
    let handlers = RENDER_TARGET_REGISTERED.with(|handlers| handlers.snapshot());
    for (_, handler) in handlers.iter() {
        handler(target_id, kind);
    }
}

thread_local! {
    // Per thread: the size of a canvas is observed on the thread of the page.
    static SIZE_CHANGED: HandlerList<dyn Fn(i32, f64, f64, f64)> = HandlerList::new();
}

/// Subscribes this thread to the size changes of every canvas; the arguments
/// are those of [`on_size_changed`]. Returns the token of the subscription.
///
/// Not from upstream. It is how a canvas that was created without a
/// top-level (the id it was created with names none) is followed: the page
/// that renders from a thread without the compositor writes the size into a
/// [`BrowserSurfaceShared`](crate::rendering::BrowserSurfaceShared) from
/// here.
pub fn add_size_changed(handler: Rc<dyn Fn(i32, f64, f64, f64)>) -> u64 {
    SIZE_CHANGED.with(|handlers| handlers.add(handler))
}

/// Ends a subscription to the size changes.
pub fn remove_size_changed(token: u64) -> bool {
    SIZE_CHANGED.with(|handlers| handlers.remove(token))
}

/// The canvas of a top-level changed its size or its scaling. `width` and
/// `height` are in device pixels; `dpr` is the device pixel ratio.
#[wasm_bindgen(js_name = CanvasHelper_OnSizeChanged)]
pub fn on_size_changed(top_level_id: i32, width: f64, height: f64, dpr: f64) {
    if let Some(surface) = BrowserTopLevelImpl::try_get_top_level(top_level_id).and_then(|top_level| top_level.surface())
    {
        surface.on_size_changed(width, height, dpr);
    }
    let handlers = SIZE_CHANGED.with(|handlers| handlers.snapshot());
    for (_, handler) in handlers.iter() {
        handler(top_level_id, width, height, dpr);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    #[test]
    fn a_registered_render_target_reaches_the_subscribers_of_its_thread() {
        let seen = Rc::new(RefCell::new(Vec::new()));
        let token = add_render_target_registered({
            let seen = seen.clone();
            Rc::new(move |target_id, kind| seen.borrow_mut().push((target_id, kind)))
        });

        on_render_target_registered(3, RENDER_TARGET_KIND_WEB_GL);
        // Another thread has subscribers of its own.
        std::thread::spawn(|| on_render_target_registered(4, RENDER_TARGET_KIND_SOFTWARE)).join().unwrap();
        assert!(remove_render_target_registered(token));
        on_render_target_registered(5, RENDER_TARGET_KIND_SOFTWARE);

        assert_eq!(vec![(3, RENDER_TARGET_KIND_WEB_GL)], *seen.borrow());
        assert!(!remove_render_target_registered(token));
    }

    #[test]
    fn a_size_change_of_a_canvas_without_a_top_level_reaches_the_subscribers() {
        let seen = Rc::new(RefCell::new(Vec::new()));
        let token = add_size_changed({
            let seen = seen.clone();
            Rc::new(move |top_level_id, width, height, dpr| seen.borrow_mut().push((top_level_id, width, height, dpr)))
        });

        // No top-level has the id 0.
        on_size_changed(0, 300.0, 180.0, 1.5);
        assert!(remove_size_changed(token));
        on_size_changed(0, 150.0, 90.0, 1.0);

        assert_eq!(vec![(0, 300.0, 180.0, 1.5)], *seen.borrow());
        assert!(!remove_size_changed(token));
    }
}
