use super::{BrowserSoftwareRenderTarget, BrowserWebGlRenderTarget};
use crate::interop::canvas_helper::{RENDER_TARGET_KIND_SOFTWARE, RENDER_TARGET_KIND_WEB_GL};
use crate::interop::{non_null, JsObject};
use ferroui_base::platform::IPlatformGraphicsContext;
use ferroui_base::PixelSize;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(raw_module = "./ferroui.js")]
extern "C" {
    /// A render target of the script side.
    pub type JsRenderTarget;

    #[wasm_bindgen(method, getter, js_name = renderTargetType)]
    fn render_target_type(this: &JsRenderTarget) -> String;

    #[wasm_bindgen(js_namespace = WebRenderTargetRegistry, js_name = getRenderTarget)]
    fn get_js_render_target(id: i32) -> JsObject;

    #[wasm_bindgen(js_namespace = WebRenderTarget, js_name = setSize)]
    fn set_js_size(target: &JsObject, w: i32, h: i32);

    #[wasm_bindgen(js_namespace = WebRenderTargetRegistry, js_name = initializeWorker)]
    fn initialize_js_worker();

    #[wasm_bindgen(js_namespace = WebRenderTargetRegistry, js_name = workerStarted)]
    fn js_worker_started(thread_id: i32);
}

/// Makes the worker of the calling thread take the canvases that are
/// transferred to it: installs the message handler of the registry of the
/// thread's script, which creates the render target of each canvas and
/// reports it
/// ([`on_render_target_registered`](crate::interop::canvas_helper::on_render_target_registered)).
///
/// Called once, by a thread other than the one of the page, before a canvas
/// is created with its id. The thread has to stay alive afterwards and
/// return to the event loop of its worker, where the messages arrive.
pub fn initialize_worker() {
    initialize_js_worker();
}

/// The id a canvas is created with for a render thread that has been started
/// and has not reported itself yet: the registry of the script side
/// transfers the control of the canvas at once and keeps the message for the
/// worker until [`worker_started`] names it.
///
/// Not from upstream, which waits for its render worker before it creates a
/// view. No thread has this id: the id of a thread is the address of its
/// descriptor.
pub const PENDING_RENDER_THREAD: i32 = -1;

/// Tells the registry of the script side of the calling thread, which has to
/// be the thread that creates the canvases, that the thread `thread_id` has
/// installed its handler ([`initialize_worker`]): the registry posts the
/// canvases it held back for it ([`PENDING_RENDER_THREAD`]) and posts the
/// later ones at once.
pub(crate) fn worker_started(thread_id: i32) {
    js_worker_started(thread_id);
}

/// The render target of a canvas, as the thread that draws to it holds it.
///
/// Upstream has an abstract class with the two kinds as subclasses; the two
/// kinds are closed here, so they are the variants of an enum. A value
/// belongs to its thread (it holds an object of the script side of that
/// thread) and is handed out by [`get_render_target`].
#[derive(Clone)]
pub enum BrowserRenderTarget {
    /// A canvas that is rendered to with WebGL.
    WebGl(Rc<BrowserWebGlRenderTarget>),
    /// A canvas that is rendered to in memory.
    Software(Rc<BrowserSoftwareRenderTarget>),
}

impl BrowserRenderTarget {
    /// The graphics context the target renders with; `None` for a target
    /// that is rendered to in software.
    pub fn platform_graphics_context(&self) -> Option<Rc<dyn IPlatformGraphicsContext>> {
        match self {
            Self::WebGl(target) => Some(target.gl_context()),
            Self::Software(_) => None,
        }
    }

    /// The kind of the target as a worker reports it and as
    /// [`BrowserSurfaceShared`](super::BrowserSurfaceShared) publishes it:
    /// [`RENDER_TARGET_KIND_WEB_GL`] or [`RENDER_TARGET_KIND_SOFTWARE`].
    pub fn kind(&self) -> i32 {
        match self {
            Self::WebGl(_) => RENDER_TARGET_KIND_WEB_GL,
            Self::Software(_) => RENDER_TARGET_KIND_SOFTWARE,
        }
    }
}

/// How a thread asks the registry of its script for a render target.
type ScriptRenderTargets = fn(i32) -> Option<BrowserRenderTarget>;

thread_local! {
    /// The render targets of this thread, by their id in the registry of the
    /// script side. Each thread has a table of its own: a render target
    /// exists on the thread whose script created it (the page, or the worker
    /// of a render thread a canvas was transferred to) and nowhere else.
    static RENDER_TARGETS: RefCell<HashMap<i32, BrowserRenderTarget>> = RefCell::new(HashMap::new());
    /// The registry of the script of this thread; a test puts a table of its
    /// own in its place.
    static SCRIPT_RENDER_TARGETS: Cell<ScriptRenderTargets> = Cell::new(wrap_script_render_target as ScriptRenderTargets);
}

/// The render target the script of the calling thread created under `id`,
/// wrapped for the framework.
///
/// # Panics
/// Panics when the target is of a kind the framework does not know.
fn wrap_script_render_target(id: i32) -> Option<BrowserRenderTarget> {
    let js = non_null(get_js_render_target(id))?;
    let type_ = JsCast::unchecked_ref::<JsRenderTarget>(&js).render_target_type();
    if type_ == "webgl" {
        return Some(BrowserRenderTarget::WebGl(BrowserWebGlRenderTarget::new(js)));
    }
    if type_ == "software" {
        return Some(BrowserRenderTarget::Software(BrowserSoftwareRenderTarget::new(js)));
    }
    panic!("{type_}");
}

/// The render target with the id `id` on the calling thread, when the
/// script of this thread has created it.
///
/// The first call that finds the target wraps it and keeps it in the table
/// of the thread; later calls answer from the table. A thread whose script
/// does not have the target (the thread of the page, for a canvas that was
/// transferred to a worker) gets `None` each time.
///
/// # Panics
/// Panics when the target is of a kind the framework does not know.
pub fn get_render_target(id: i32) -> Option<BrowserRenderTarget> {
    if let Some(target) = RENDER_TARGETS.with(|targets| targets.borrow().get(&id).cloned()) {
        return Some(target);
    }
    // Outside the borrow: wrapping a WebGL target calls into the script and
    // into the module.
    let script = SCRIPT_RENDER_TARGETS.with(|script| script.get());
    let target = script(id)?;
    RENDER_TARGETS.with(|targets| targets.borrow_mut().insert(id, target.clone()));
    Some(target)
}

/// Takes the render target with the id `id` out of the table of the calling
/// thread; returns whether it was there. What still draws to the target (a
/// render target a backend created from it) keeps it alive until it is
/// released, on this thread.
///
/// For the thread that renders, when a view is closed.
pub fn remove_render_target(id: i32) -> bool {
    RENDER_TARGETS.with(|targets| targets.borrow_mut().remove(&id)).is_some()
}

/// Replaces the registry of the script for the calling thread.
#[cfg(test)]
pub(crate) fn set_script_render_targets_for_unit_tests(script: fn(i32) -> Option<BrowserRenderTarget>) {
    SCRIPT_RENDER_TARGETS.with(|current| current.set(script));
}

/// Sets the size of the canvas behind a render target.
pub(crate) fn update_size(js: &JsObject, size: PixelSize) {
    set_js_size(js, size.width, size.height);
}

#[cfg(test)]
mod tests {
    use super::*;

    thread_local! {
        /// How often the thread of a test asked its script for a target.
        static ASKED: Cell<u32> = const { Cell::new(0) };
    }

    fn asked() -> u32 {
        ASKED.with(|asked| asked.get())
    }

    /// A script that has the software targets 41 and 42 and no other.
    fn script(id: i32) -> Option<BrowserRenderTarget> {
        ASKED.with(|asked| asked.set(asked.get() + 1));
        (id == 41 || id == 42).then(|| BrowserRenderTarget::Software(BrowserSoftwareRenderTarget::new(JsObject::NULL)))
    }

    /// A script without a target, as the thread of the page has for a canvas
    /// it transferred.
    fn empty_script(_id: i32) -> Option<BrowserRenderTarget> {
        ASKED.with(|asked| asked.set(asked.get() + 1));
        None
    }

    fn same(a: &BrowserRenderTarget, b: &BrowserRenderTarget) -> bool {
        match (a, b) {
            (BrowserRenderTarget::Software(a), BrowserRenderTarget::Software(b)) => Rc::ptr_eq(a, b),
            (BrowserRenderTarget::WebGl(a), BrowserRenderTarget::WebGl(b)) => Rc::ptr_eq(a, b),
            _ => false,
        }
    }

    #[test]
    fn a_target_is_wrapped_once_and_kept_for_its_thread() {
        set_script_render_targets_for_unit_tests(script);

        let first = get_render_target(41).expect("the script has the target");
        let again = get_render_target(41).expect("the table has the target");
        assert!(same(&first, &again));
        assert_eq!(1, asked());
        assert_eq!(RENDER_TARGET_KIND_SOFTWARE, first.kind());
        assert!(first.platform_graphics_context().is_none());

        // Another id is another target.
        let other = get_render_target(42).expect("the script has the target");
        assert!(!same(&first, &other));
        assert_eq!(2, asked());
    }

    #[test]
    fn a_target_the_script_does_not_have_is_asked_for_each_time() {
        set_script_render_targets_for_unit_tests(script);

        assert!(get_render_target(7).is_none());
        assert!(get_render_target(7).is_none());
        assert_eq!(2, asked());
    }

    #[test]
    fn each_thread_has_a_table_of_its_own() {
        set_script_render_targets_for_unit_tests(script);
        let here = get_render_target(41).expect("the script has the target");

        // The thread of the page, for a canvas it transferred: its script
        // has no target, and the table of the other thread is not its own.
        let there = std::thread::spawn(|| {
            set_script_render_targets_for_unit_tests(empty_script);
            let found = get_render_target(41).is_some();
            (found, asked())
        })
        .join()
        .unwrap();
        assert_eq!((false, 1), there);

        // A second thread with a script that has the target wraps its own.
        let wrapped = std::thread::spawn(|| {
            set_script_render_targets_for_unit_tests(script);
            get_render_target(41).is_some() && get_render_target(41).is_some() && asked() == 1
        })
        .join()
        .unwrap();
        assert!(wrapped);

        assert!(same(&here, &get_render_target(41).expect("the table has the target")));
        assert_eq!(1, asked());
    }

    #[test]
    fn a_removed_target_is_asked_for_again() {
        set_script_render_targets_for_unit_tests(script);
        let first = get_render_target(42).expect("the script has the target");

        assert!(remove_render_target(42));
        assert!(!remove_render_target(42));

        let second = get_render_target(42).expect("the script has the target");
        assert!(!same(&first, &second));
        assert_eq!(2, asked());
    }
}
