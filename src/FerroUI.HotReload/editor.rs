use crate::{ReloadOutcome, ReloadStatus};
use ferroui_base::{Ref, WeakRef, rendering::composition::{CompositionFrameStatus, Compositor}};
use ferroui_controls::Window;
use ferroui_reload_protocol::{self as protocol, FrameRequest, FrameStatus, Language, Status};
use std::{cell::RefCell, collections::BTreeMap, path::PathBuf, rc::Rc};

ferroui_base::ferro_thread_local! {
    static WINDOW: RefCell<Option<WeakRef<Window>>> = const { RefCell::new(None) };
}

/// Report that a manifest-backed XAML application has opened its main window.
/// Call after registering the live targets and before showing the window.
pub fn notify_editor_on_open(window: &Ref<Window>) {
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
                Some(frame) => frame.ready(Language::Xaml),
                None => protocol::ready(Language::Xaml),
            }
        }).detach();
    }
}

pub(crate) fn report(
    files: &BTreeMap<String, PathBuf>,
    sources: &BTreeMap<String, String>,
    outcome: &ReloadOutcome,
) {
    let status = match outcome.status {
        ReloadStatus::Applied => Status::Applied,
        ReloadStatus::Failed => Status::Failed,
        ReloadStatus::RestartRequired => Status::Rebuild,
        ReloadStatus::Superseded => return,
    };
    let compositor = (outcome.status == ReloadStatus::Applied).then(compositor).flatten();
    let mut frames = Vec::new();
    for uri in &outcome.documents {
        if let (Some(path), Some(text)) = (files.get(uri), sources.get(uri)) {
            if compositor.is_some() {
                frames.extend(protocol::source_with_frame(Language::Xaml, path, text, outcome.diagnostic.as_deref()));
            } else {
                protocol::source(Language::Xaml, status, path, text, outcome.diagnostic.as_deref());
            }
        }
    }
    if let Some(compositor) = compositor {
        observe(&compositor, frames);
    }
}

fn compositor() -> Option<Rc<Compositor>> {
    let window = WINDOW.with(|current| current.borrow().as_ref().and_then(WeakRef::upgrade))?;
    window.renderer().compositor()
}

fn observe(compositor: &Compositor, frames: Vec<FrameRequest>) {
    if frames.is_empty() {
        return;
    }
    compositor.observe_next_frame(move |batch, status| {
        let status = match status {
            CompositionFrameStatus::Rendered => FrameStatus::Rendered,
            CompositionFrameStatus::NoSceneUpdate => FrameStatus::NoSceneUpdate,
            CompositionFrameStatus::Discarded => FrameStatus::Discarded,
        };
        FrameRequest::complete_batch(frames, batch, status);
    });
}
