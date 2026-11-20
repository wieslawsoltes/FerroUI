use ferroui_base::{Ref, WeakRef, rendering::composition::{CompositionFrameStatus, Compositor}};
use ferroui_controls::Window;
use ferroui_reload_protocol::{self as protocol, FrameRequest, FrameStatus, Language, Status};
use std::{cell::RefCell, path::Path, rc::Rc};

ferroui_base::ferro_thread_local! {
    static WINDOW: RefCell<Option<WeakRef<Window>>> = const { RefCell::new(None) };
}

pub(super) fn notify_on_open(window: &Ref<Window>) {
    if protocol::enabled() {
        let opened = window.downgrade();
        let startup = window.renderer().compositor().and_then(|compositor| {
            let frame = FrameRequest::startup()?;
            compositor.observe_first_scene(move |batch| frame.complete(batch, FrameStatus::Rendered));
            Some(frame)
        });
        window.opened(move || {
            WINDOW.with(|window| window.replace(Some(opened.clone())));
            match startup {
                Some(frame) => frame.ready(Language::Rsx),
                None => protocol::ready(Language::Rsx),
            }
        }).detach();
    }
}

pub(super) fn applied(path: &Path, source: &str) {
    if let Some(compositor) = compositor() {
        if let Some(frame) = protocol::source_with_frame(Language::Rsx, path, source, None) {
            observe(&compositor, frame);
        }
    } else {
        protocol::source(Language::Rsx, Status::Applied, path, source, None);
    }
}

fn compositor() -> Option<Rc<Compositor>> {
    let window = WINDOW.with(|current| current.borrow().as_ref().and_then(WeakRef::upgrade))?;
    window.renderer().compositor()
}

fn observe(compositor: &Compositor, frame: FrameRequest) {
    compositor.observe_next_frame(move |batch, status| {
        let status = match status {
            CompositionFrameStatus::Rendered => FrameStatus::Rendered,
            CompositionFrameStatus::NoSceneUpdate => FrameStatus::NoSceneUpdate,
            CompositionFrameStatus::Discarded => FrameStatus::Discarded,
        };
        frame.complete(batch, status);
    });
}
