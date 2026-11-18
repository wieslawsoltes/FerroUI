use crate::{ReloadOutcome, ReloadStatus};
use ferroui_base::Ref;
use ferroui_controls::Window;
use ferroui_reload_protocol::{self as protocol, Language, Status};
use std::{collections::BTreeMap, path::PathBuf};

/// Report that a manifest-backed XAML application has opened its main window.
/// Call after registering the live targets and before showing the window.
pub fn notify_editor_on_open(window: &Ref<Window>) {
    if protocol::enabled() {
        window.opened(|| protocol::ready(Language::Xaml)).detach();
    }
}

pub(crate) fn report(files: &BTreeMap<String, PathBuf>, sources: &BTreeMap<String, String>, outcome: &ReloadOutcome) {
    let status = match outcome.status {
        ReloadStatus::Applied => Status::Applied,
        ReloadStatus::Failed => Status::Failed,
        ReloadStatus::RestartRequired => Status::Rebuild,
        ReloadStatus::Superseded => return,
    };
    for uri in &outcome.documents {
        if let (Some(path), Some(text)) = (files.get(uri), sources.get(uri)) {
            protocol::source(Language::Xaml, status, path, text, outcome.diagnostic.as_deref());
        }
    }
}
