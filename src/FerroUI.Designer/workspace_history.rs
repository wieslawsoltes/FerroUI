use super::DesignerWorkspace;
use super::dictionary_extraction::CreatedDictionary;
use super::file_move::MovedProjectFile;
use super::project::{Tab, document_identity};
use super::reviewed_sources::check_saved;
use std::collections::BTreeMap;
use std::rc::Rc;

/// Link simultaneous authored edits between existing per-tab history stacks.
/// A newer edit in a participating tab must be undone before the group.
#[derive(Default)]
pub(super) struct WorkspaceHistory {
    serial: u64,
    groups: BTreeMap<String, Vec<String>>,
    created: BTreeMap<String, CreatedDictionary>,
    pub(super) moves: BTreeMap<String, MovedProjectFile>,
    closed: BTreeMap<String, Tab>,
}

impl WorkspaceHistory {
    pub(super) fn label(&mut self, title: &str) -> Result<String, String> {
        self.serial = self
            .serial
            .checked_add(1)
            .ok_or("Workspace edit sequence exhausted")?;
        Ok(format!("{title} ({})", self.serial))
    }

    pub(super) fn link(&mut self, label: String, participants: Vec<String>) {
        self.groups.insert(label, participants);
    }

    pub(super) fn link_created(&mut self, label: String, created: CreatedDictionary) {
        self.created.insert(label, created);
    }

    fn companions(&self, label: &str, active: &str) -> Option<Vec<String>> {
        let group = self.groups.get(label)?;
        group.iter().any(|uri| uri == active).then(|| {
            group.iter().filter(|uri| *uri != active).cloned().collect()
        })
    }

    pub(super) fn retain_closed(&mut self, uri: &str, tab: Tab) {
        if tab.source() == tab.saved
            && (self.groups.values().any(|group| group.iter().any(|file| file == uri))
                || self.moves.values().any(|movement| movement.participates(uri)))
        {
            self.closed.insert(uri.into(), tab);
        }
    }

    pub(super) fn take_closed(&mut self, uri: &str) -> Option<Tab> {
        self.closed.remove(uri)
    }

    pub(super) fn closed(&self, uri: &str) -> Option<&Tab> {
        self.closed.get(uri)
    }
}

impl Tab {
    pub(super) fn history_label(&self, redo: bool) -> Option<&str> {
        self.rust.as_ref().map_or_else(
            || {
                if redo {
                    self.document.redo_label()
                } else {
                    self.document.undo_label()
                }
            },
            |source| source.history_label(redo),
        )
    }

    pub(super) fn step_history(&mut self, redo: bool) -> Result<(), String> {
        if let Some(rust) = self.rust.as_mut() {
            rust.restore_history(&mut self.document, redo)
        } else if redo {
            self.document.redo()
        } else {
            self.document.undo()
        }
        .map(|_| ())
        .map_err(|error| error.to_string())
    }
}

impl DesignerWorkspace {
    pub(crate) fn source_history(self: &Rc<Self>, redo: bool) -> Result<(), String> {
        let (active, label) = {
            let session = self.session.borrow();
            let active = document_identity(session.document_uri())?;
            let label = if session.collaboration_actor().is_some() {
                None
            } else if let Some(rust) = &session.rust {
                rust.history_label(redo)
            } else if redo {
                session.document().redo_label()
            } else {
                session.document().undo_label()
            };
            (active, label.map(str::to_owned))
        };
        let created = label.as_deref().and_then(|label| {
            self.project.borrow().linked_history.created.get(label).cloned().map(|created| (label, created))
        });
        if let Some((label, created)) = created {
            return self.step_dictionary_history(label, created, redo);
        }
        let moved = label.as_deref().and_then(|label| {
            self.project.borrow().linked_history.moves.get(label).cloned().map(|movement| (label, movement))
        });
        if let Some((label, movement)) = moved {
            return self.step_file_move(label, movement, redo);
        }
        let companions = label.as_deref().and_then(|label| {
            self.project.borrow().linked_history.companions(label, &active).map(|uris| (label, uris))
        });
        if let Some((label, companions)) = companions {
            let mut staged = self.stage_reviewed_sources()?;
            for uri in &companions {
                let mut tab = staged.tab(self, uri)?;
                if tab.history_label(redo) != Some(label) {
                    return Err("Finish newer history changes in the other participating tabs first".into());
                }
                check_saved(&staged.workspace, uri, &tab)?;
                tab.step_history(redo)?;
                staged.tabs.insert(uri.clone(), tab);
            }
            staged.prepare_edits(&companions)?;
            self.step_active_history(redo)?;
            self.publish_reviewed_sources(staged, &companions)?;
        } else {
            self.step_active_history(redo)?;
        }
        self.refresh_rust_modules()?;
        Ok(())
    }

    fn step_active_history(&self, redo: bool) -> Result<(), String> {
        let mut session = self.session.borrow_mut();
        if redo { session.redo() } else { session.undo() }.map(|_| ()).map_err(|e| e.to_string())
    }
}
