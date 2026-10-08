use super::{BrowserRenderTimer, RenderWorker};
use ferroui_base::rendering::{IRenderLoop, IRenderTimer, RenderLoop};
use std::sync::{Arc, OnceLock};

/// The one render loop of the page. Every view renders in the animation
/// frames of one thread: the thread of the user interface, or the render
/// thread when the platform started one
/// ([`start_render_thread`](Self::start_render_thread)).
pub struct BrowserSharedRenderLoop;

static BROWSER_UI_RENDER_TIMER: OnceLock<Arc<BrowserRenderTimer>> = OnceLock::new();
static RENDER_LOOP: OnceLock<Arc<dyn IRenderLoop>> = OnceLock::new();

impl BrowserSharedRenderLoop {
    /// The render timer of the page. Upstream creates it with
    /// `RunsInBackground` false in both of its modes; here it runs in the
    /// background when it was created for the render thread
    /// ([`start_render_thread`](Self::start_render_thread)).
    pub fn render_timer() -> Arc<BrowserRenderTimer> {
        BROWSER_UI_RENDER_TIMER.get_or_init(|| BrowserRenderTimer::new(false)).clone()
    }

    /// Starts the render thread of the page and makes the render timer one
    /// that ticks there, in the background. Returns whether the views of
    /// the page are rendered by the render thread from here on.
    ///
    /// The platform calls it once, when it is registered, in a module built
    /// with threads: before the first view is created and before anything
    /// asks for the render timer or the render loop. The thread of the user
    /// interface does not wait for the thread
    /// ([`RenderWorker::start`]); a view that is created before the thread
    /// has reported itself is not ready until it has.
    ///
    /// The answer is `false`, and the page renders on the thread of the
    /// user interface as it does without threads, when the module was built
    /// without threads, when the thread cannot be created, when the render
    /// timer exists already (the order above was not kept) and when a
    /// render thread was started by somebody else, with a frame loop of
    /// their own.
    pub fn start_render_thread() -> bool {
        if !cfg!(all(target_os = "emscripten", target_feature = "atomics")) {
            return false;
        }
        if BROWSER_UI_RENDER_TIMER.get().is_some() {
            return Self::renders_on_render_thread();
        }
        // Created in its final state before the first compositor is: a
        // compositor reads where its loop runs once, when it is created.
        let timer = BrowserRenderTimer::new(true);
        match RenderWorker::start(Some(timer.clone()), None) {
            Ok(true) => BROWSER_UI_RENDER_TIMER.set(timer).is_ok(),
            // Started before, with another frame loop or none; or not
            // started at all. The timer made above is dropped unused.
            Ok(false) | Err(_) => false,
        }
    }

    /// Whether the render loop of the page ticks on the render thread: the
    /// compositor of a view is then confined to that thread, and the canvas
    /// of a view is created for it.
    pub fn renders_on_render_thread() -> bool {
        BROWSER_UI_RENDER_TIMER.get().is_some_and(|timer| timer.runs_in_background())
    }

    /// The render loop driven by [`render_timer`](Self::render_timer).
    pub fn render_loop() -> Arc<dyn IRenderLoop> {
        RENDER_LOOP
            .get_or_init(|| {
                let timer: Arc<dyn IRenderTimer> = Self::render_timer();
                RenderLoop::from_timer(timer)
            })
            .clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn without_threads_no_render_thread_is_started_and_the_loop_stays_on_the_page() {
        assert!(!BrowserSharedRenderLoop::start_render_thread());
        assert!(!RenderWorker::exists());
        assert!(!BrowserSharedRenderLoop::renders_on_render_thread());

        // The timer and the loop of the page, as before there were threads.
        assert!(!BrowserSharedRenderLoop::render_timer().runs_in_background());
        assert!(!BrowserSharedRenderLoop::render_loop().runs_in_background());

        // Asked again, with the timer in place, the answer is the same.
        assert!(!BrowserSharedRenderLoop::start_render_thread());
        assert!(!BrowserSharedRenderLoop::renders_on_render_thread());
    }
}
