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
/// animation frame and requests the next one. Prints `PERF first_frame` on the first frame,
/// `PERF second_frame` on the one after it and, with `PERF_REPORT=1`, `PERF fps N` once a second.
///
/// The first animation frame comes before the scene has been drawn: what it leaves to do (the first
/// commit of the composition tree, the first drawing, whatever the platform asks to be painted when
/// a window is shown) keeps the thread until the second one, which is therefore the time by which
/// the scene is on the screen and the application answers.
pub struct FrameHost {
    base: Border,
    frames_logged: Cell<u8>,
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
        let this = instantiate(Self { base: Border::construct(), frames_logged: Cell::new(0), running: Cell::new(false) });
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
        match self.frames_logged.get() {
            0 => println!("PERF first_frame"),
            1 => println!("PERF second_frame"),
            _ => {}
        }
        self.frames_logged.set(self.frames_logged.get().saturating_add(1).min(2));
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
