use super::BrowserRenderTimer;
use ferroui_base::rendering::{IRenderLoop, IRenderTimer, RenderLoop};
use std::sync::{Arc, OnceLock};

/// The one render loop of the page: every view renders on the UI thread,
/// in the animation frames.
pub struct BrowserSharedRenderLoop;

static BROWSER_UI_RENDER_TIMER: OnceLock<Arc<BrowserRenderTimer>> = OnceLock::new();
static RENDER_LOOP: OnceLock<Arc<dyn IRenderLoop>> = OnceLock::new();

impl BrowserSharedRenderLoop {
    /// The render timer of the page.
    pub fn render_timer() -> Arc<BrowserRenderTimer> {
        BROWSER_UI_RENDER_TIMER.get_or_init(|| BrowserRenderTimer::new(false)).clone()
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
