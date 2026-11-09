//! The host of a scene: drives the frames from the animation frames of its top level.

use crate::clock::Clock;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_overrides, instantiate, FerroObjectImpl, Ref, StyledElementImpl, Visual,
    VisualImpl, VisualImplExt, VisualTreeAttachmentEventArgs,
};
use ferroui_controls::{Border, Control, ControlImpl, TopLevel};
use std::cell::Cell;

/// Shows a scene and, from its attachment to a top level on, advances the [`Clock`] once per
/// animation frame and requests the next one. Prints `PERF first_frame` on the first frame and,
/// with `PERF_REPORT=1`, `PERF fps N` once a second.
pub struct FrameHost {
    base: Border,
    first_frame_logged: Cell<bool>,
    running: Cell<bool>,
}

ferro_class!(FrameHost: Border);
ferro_impl_classes!(
    FrameHost: FerroObjectImpl,
    StyledElementImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl
);

ferro_overrides! { impl VisualImpl for FrameHost {
    fn on_attached_to_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, e);
        this.running.set(true);
        this.request_frame();
    }

    fn on_detached_from_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_detached_from_visual_tree(this, e);
        this.running.set(false);
    }
} }

impl FrameHost {
    pub fn new(scene: Ref<Control>) -> Ref<Self> {
        let this = instantiate(Self { base: Border::construct(), first_frame_logged: Cell::new(false), running: Cell::new(false) });
        this.set_child(&scene);
        this
    }

    fn request_frame(&self) {
        let visual: &Visual = self;
        if let Some(top_level) = TopLevel::get_top_level(Some(visual)) {
            let host = self.to_ref();
            top_level.request_animation_frame(move |_| host.on_frame());
        }
    }

    fn on_frame(&self) {
        if !self.running.get() {
            return;
        }
        if !self.first_frame_logged.replace(true) {
            println!("PERF first_frame");
        }
        Clock::advance();
        report_frame_rate();
        self.request_frame();
    }
}

/// With `PERF_REPORT=1`, prints `PERF fps N` once a second.
fn report_frame_rate() {
    ferroui_base::ferro_thread_local! {
        static REPORT: Option<(Cell<std::time::Instant>, Cell<u32>)> =
            std::env::var_os("PERF_REPORT").map(|_| (Cell::new(std::time::Instant::now()), Cell::new(0)));
    }
    REPORT.with(|report| {
        let Some((since, frames)) = report else { return };
        frames.set(frames.get() + 1);
        let elapsed = since.get().elapsed();
        if elapsed.as_secs_f64() >= 1.0 {
            println!("PERF fps {:.1}", f64::from(frames.get()) / elapsed.as_secs_f64());
            since.set(std::time::Instant::now());
            frames.set(0);
        }
    });
}
