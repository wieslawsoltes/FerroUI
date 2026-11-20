use super::{
    project::{document_identity, Tab},
    DesignerWorkspace,
};
use crate::rsx_document::RustDocument;
use ferroui_designer_core::TextPatch;
use ferroui_designer_language::{
    FileMoveDocument, FileMoveDraft, FileMovePlan, LanguageService, LineIndex,
};
use ferroui_designer_project::{file_uri, MovedSourceFile, ProjectWorkspace};
use std::{collections::BTreeMap, path::Path, rc::Rc};

#[path = "workspace_file_move_review.rs"]
mod review;
#[cfg(all(test, feature = "mcp"))]
#[path = "workspace_file_move_tests.rs"]
mod tests;

#[derive(Clone)]
pub(super) struct MovedXaml {
    file: MovedSourceFile,
    participants: Vec<String>,
}

impl MovedXaml {
    pub(super) fn participates(&self, uri: &str) -> bool {
        uri == self.file.to || self.participants.iter().any(|file| file == uri)
    }
}

struct MoveState {
    workspace: ProjectWorkspace,
    tabs: BTreeMap<String, Tab>,
    language: LanguageService,
    active: String,
}

impl DesignerWorkspace {
    pub(crate) fn plan_project_file_move(
        self: &Rc<Self>,
        destination: &str,
    ) -> Result<FileMovePlan, String> {
        if self.pending_source.get() {
            return Err("Apply the current draft before moving its file".into());
        }
        let from = self.current_xaml_file()?;
        let state = self.project.borrow();
        let workspace = state
            .workspace
            .as_ref()
            .ok_or("Open the owning project first")?;
        let to = file_uri(workspace.solution.root.join(Path::new(destination)))
            .map_err(|e| e.to_string())?;
        let mut language = self.try_language_service()?;
        let mut rust_drafts = Vec::new();
        for (uri, tab) in &state.tabs {
            if !uri.starts_with("file://") || uri == &from {
                continue;
            }
            if tab.rust.is_some() {
                rust_drafts.push(FileMoveDraft {
                    uri: uri.clone(),
                    version: i64::try_from(tab.document.revision())
                        .map_err(|_| "Rust document revision exhausted")?,
                    source: tab.source().to_owned(),
                });
            } else {
                sync(&mut language, uri, tab.source())?;
            }
        }
        sync(
            &mut language,
            &from,
            self.session.borrow().document().source(),
        )?;
        language
            .plan_file_move_with_drafts(&from, &to, &rust_drafts)
            .map_err(|e| e.to_string())
    }

    pub(crate) fn apply_project_file_move(
        self: &Rc<Self>,
        destination: &str,
        review: &str,
    ) -> Result<FileMovePlan, String> {
        self.ensure_local_file_move()?;
        let plan = self.plan_project_file_move(destination)?;
        if plan.review != review {
            return Err(
                "The file move changed. Review the current source patches before applying it"
                    .into(),
            );
        }
        let mut staged = self.stage_file_move()?;
        let label = self
            .project
            .borrow_mut()
            .linked_history
            .label("Move XAML file")?;
        for change in &plan.documents {
            let tab = staged.tab(self, &change.edit.uri)?;
            check_saved(&staged.workspace, &change.edit.uri, &tab)?;
            staged
                .tabs
                .insert(change.edit.uri.clone(), patched(tab, change, &label)?);
        }
        let participants = plan
            .documents
            .iter()
            .map(|item| item.edit.uri.clone())
            .collect::<Vec<_>>();
        let source = staged.tabs[&plan.from].saved.clone();
        staged.prepare(&plan.from, &plan.to, &participants)?;
        let file = staged
            .workspace
            .move_source_file(&plan.from, &plan.to, &source)
            .map_err(|error| error.to_string())?;
        self.project
            .borrow_mut()
            .linked_history
            .moves
            .insert(label, MovedXaml { file, participants });
        self.accept_file_move(staged, &plan.from, &plan.to)?;
        Ok(plan)
    }

    pub(super) fn step_file_move(
        self: &Rc<Self>,
        label: &str,
        mut moved: MovedXaml,
        redo: bool,
    ) -> Result<(), String> {
        self.ensure_local_file_move()?;
        let (from, to) = if redo {
            (moved.file.from.clone(), moved.file.to.clone())
        } else {
            (moved.file.to.clone(), moved.file.from.clone())
        };
        let mut staged = self.stage_file_move()?;
        let participants = moved
            .participants
            .iter()
            .map(|uri| {
                if uri == &moved.file.from {
                    from.clone()
                } else {
                    uri.clone()
                }
            })
            .collect::<Vec<_>>();
        for uri in &participants {
            let mut tab = staged
                .tabs
                .get(uri)
                .cloned()
                .or_else(|| self.project.borrow().linked_history.closed(uri).cloned())
                .ok_or("Reopen all files in this move before applying its history")?;
            if tab.history_label(redo) != Some(label) {
                return Err("Finish newer history in the files changed by this move first".into());
            }
            check_saved(&staged.workspace, uri, &tab)?;
            tab.step_history(redo)?;
            staged.tabs.insert(uri.clone(), tab);
        }
        let saved = staged.tabs[&from].saved.clone();
        staged.prepare(&from, &to, &participants)?;
        staged
            .workspace
            .replay_source_move(&mut moved.file, redo, &saved)
            .map_err(|e| e.to_string())?;
        self.project
            .borrow_mut()
            .linked_history
            .moves
            .insert(label.into(), moved);
        self.accept_file_move(staged, &from, &to)
    }

    fn current_xaml_file(&self) -> Result<String, String> {
        let session = self.session.borrow();
        let uri = session.document_uri();
        if session.uses_rust_source()
            || !uri.starts_with("file://")
            || !uri.to_ascii_lowercase().ends_with(".xaml")
        {
            return Err("Choose a project XAML file".into());
        }
        document_identity(uri)
    }

    fn ensure_local_file_move(&self) -> Result<(), String> {
        let session = self.session.borrow();
        session.ensure_writable().map_err(|e| e.to_string())?;
        if self.pending_source.get() || session.collaboration_actor().is_some() {
            return Err(
                "Apply the current draft and open the local project before moving its file".into(),
            );
        }
        if session.document().revision() >= i64::MAX as u64 - 2 {
            return Err("Document revision exhausted".into());
        }
        Ok(())
    }

    fn stage_file_move(&self) -> Result<MoveState, String> {
        let active = document_identity(self.session.borrow().document_uri())?;
        self.capture_tab();
        let project = self.project.borrow();
        Ok(MoveState {
            workspace: project
                .workspace
                .clone()
                .ok_or("Open the owning project first")?,
            tabs: project.tabs.clone(),
            language: self.try_language_service()?.clone(),
            active,
        })
    }

    fn accept_file_move(
        self: &Rc<Self>,
        staged: MoveState,
        from: &str,
        to: &str,
    ) -> Result<(), String> {
        let active = if staged.active == from {
            to
        } else {
            &staged.active
        };
        let tab = staged.tabs[active].clone();
        {
            let mut project = self.project.borrow_mut();
            project.workspace = Some(staged.workspace);
            project.tabs = staged.tabs;
            project.linked_history.take_closed(from);
            project.linked_history.take_closed(to);
        }
        *self.try_language_service()? = staged.language;
        self.present_project_tab(active, tab)?;
        self.rebuild_project_tree();
        Ok(())
    }
}

impl MoveState {
    fn tab(&mut self, editor: &DesignerWorkspace, uri: &str) -> Result<Tab, String> {
        if let Some(tab) = self.tabs.get(uri) {
            return Ok(tab.clone());
        }
        let (document, rust, saved) = if RustDocument::for_uri(uri) {
            let saved = self
                .workspace
                .read_source(uri)
                .map_err(|error| error.to_string())?;
            let (document, rust) = RustDocument::open(saved.clone(), editor.rust_catalog(uri))
                .map_err(|error| error.to_string())?;
            (document, Some(rust), saved)
        } else {
            let buffer = self
                .workspace
                .open(uri)
                .map_err(|error| error.to_string())?;
            (
                buffer.document().clone(),
                None,
                buffer.saved_source().unwrap_or_default().into(),
            )
        };
        Ok(Tab {
            preview_uri: uri.into(),
            document,
            rust,
            saved,
            file: None,
            layers: None,
            viewport: None,
            guides: None,
        })
    }

    fn prepare(&mut self, from: &str, to: &str, participants: &[String]) -> Result<(), String> {
        if self.tabs.contains_key(to) {
            return Err("The destination is already open".into());
        }
        for uri in participants {
            let tab = &self.tabs[uri];
            if tab.rust.is_some() {
                continue;
            }
            sync(&mut self.language, uri, tab.source())?;
            if let Some(buffer) = self.workspace.buffer(uri) {
                self.workspace
                    .store_document(uri, buffer.version(), tab.document.clone())
                    .map_err(|e| e.to_string())?;
            }
        }
        self.language
            .rename_uri(from, to)
            .map_err(|e| e.to_string())?;
        let mut tab = self
            .tabs
            .remove(from)
            .ok_or("The moved file is unavailable")?;
        tab.preview_uri = to.into();
        tab.file = None;
        self.tabs.insert(to.into(), tab);
        Ok(())
    }
}

fn check_saved(workspace: &ProjectWorkspace, uri: &str, tab: &Tab) -> Result<(), String> {
    if workspace.read_source(uri).map_err(|e| e.to_string())? != tab.saved {
        return Err(format!(
            "Reopen the changed file before moving its source: {uri}"
        ));
    }
    Ok(())
}

fn patched(mut tab: Tab, change: &FileMoveDocument, label: &str) -> Result<Tab, String> {
    if tab.source() != change.before {
        return Err("A participating file changed before applying the move".into());
    }
    let revision = tab.document.revision();
    if let Some(rust) = tab.rust.as_mut() {
        rust.update(&mut tab.document, change.after.clone().into(), label, false)
            .map_err(|error| error.to_string())?;
    } else if change.edit.edits.is_empty() {
        tab.document
            .record_workspace_operation(revision, label.into())
            .map_err(|e| e.to_string())?;
    } else {
        let index = LineIndex::new(tab.source());
        let patches = change
            .edit
            .edits
            .iter()
            .map(|edit| {
                Ok(TextPatch {
                    span: index.span(edit.range)?,
                    replacement: edit.new_text.clone(),
                })
            })
            .collect::<ferroui_designer_language::Result<_>>()
            .map_err(|e| e.to_string())?;
        tab.document
            .apply_text_patches(revision, label, patches)
            .map_err(|e| e.to_string())?;
    }
    if tab.source() != change.after {
        return Err("The source moved since it was reviewed".into());
    }
    Ok(tab)
}

fn sync(language: &mut LanguageService, uri: &str, source: &str) -> Result<(), String> {
    if language
        .document(uri)
        .is_some_and(|document| document.source() == source)
    {
        return Ok(());
    }
    let version = language
        .version(uri)
        .unwrap_or_default()
        .max(0)
        .checked_add(1)
        .ok_or("Language version exhausted")?;
    language
        .open(uri, source, version)
        .map_err(|e| e.to_string())
}
