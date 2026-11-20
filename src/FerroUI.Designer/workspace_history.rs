use super::DesignerWorkspace;
use super::dictionary_extraction::CreatedDictionary;
use super::project::{Tab, document_identity};
use std::collections::BTreeMap;
use std::rc::Rc;

/// Link simultaneous authored edits between existing per-tab history stacks.
/// A newer edit in either participating tab must be undone before the pair.
#[derive(Default)]
pub(super) struct WorkspaceHistory {
    serial: u64,
    pairs: BTreeMap<String, [String; 2]>,
    created: BTreeMap<String, CreatedDictionary>,
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

    pub(super) fn link(&mut self, label: String, first: String, second: String) {
        self.pairs.insert(label, [first, second]);
    }

    pub(super) fn link_created(&mut self, label: String, created: CreatedDictionary) {
        self.created.insert(label, created);
    }

    fn partner(&self, label: &str, active: &str) -> Option<&str> {
        let pair = self.pairs.get(label)?;
        if pair[0] == active {
            Some(&pair[1])
        } else if pair[1] == active {
            Some(&pair[0])
        } else {
            None
        }
    }

    pub(super) fn retain_closed(&mut self, uri: &str, tab: Tab) {
        if tab.source() == tab.saved
            && self
                .pairs
                .values()
                .any(|pair| pair.iter().any(|file| file == uri))
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
    fn history_label(&self, redo: bool) -> Option<&str> {
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

    fn step_history(&mut self, redo: bool) -> Result<(), String> {
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
        let companion = {
            let state = self.project.borrow();
            if let Some((label, uri)) = label.as_deref().and_then(|label| {
                state
                    .linked_history
                    .partner(label, &active)
                    .map(|uri| (label, uri))
            }) {
                let mut tab = state
                    .tabs
                    .get(uri)
                    .or_else(|| state.linked_history.closed(uri))
                    .cloned()
                    .ok_or("The other tab's source history is unavailable")?;
                if tab.history_label(redo) != Some(label) {
                    return Err("Finish newer history changes in the other tab first".into());
                }
                let workspace = state
                    .workspace
                    .as_ref()
                    .ok_or("Open the owning project first")?;
                if workspace
                    .read_source(uri)
                    .map_err(|error| error.to_string())?
                    != tab.saved
                {
                    return Err("The other file changed on disk. Reopen its current source before editing it".into());
                }
                tab.step_history(redo)?;
                Some((uri.to_owned(), tab))
            } else {
                None
            }
        };
        let mut session = self.session.borrow_mut();
        if redo { session.redo() } else { session.undo() }.map_err(|e| e.to_string())?;
        drop(session);
        if let Some((uri, tab)) = companion {
            let mut state = self.project.borrow_mut();
            state.linked_history.take_closed(&uri);
            state.tabs.insert(uri, tab);
            drop(state);
            self.rebuild_document_tabs();
        }
        Ok(())
    }
}
