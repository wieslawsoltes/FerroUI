//! Stage reviewed source changes before publishing editor and project buffers.
use super::{
    DesignerWorkspace,
    project::{Tab, document_identity},
};
use crate::rsx_document::RustDocument;
use ferroui_designer_core::TextPatch;
use ferroui_designer_language::{LanguageService, LineIndex, ReviewedDocument};
use ferroui_designer_project::ProjectWorkspace;
use std::collections::BTreeMap;
use std::rc::Rc;

#[path = "workspace_reviewed_sources_ui.rs"]
pub(super) mod ui;

pub(super) struct ReviewedSources {
    pub(super) workspace: ProjectWorkspace,
    pub(super) tabs: BTreeMap<String, Tab>,
    pub(super) language: LanguageService,
    pub(super) active: String,
}

impl DesignerWorkspace {
    pub(super) fn sync_xaml_review_drafts(&self, active: &str) -> Result<(), String> {
        let project = self.project.borrow();
        let mut language = self.try_language_service()?;
        for (uri, tab) in &project.tabs {
            if uri.starts_with("file://") && uri != active && tab.rust.is_none() {
                sync(&mut language, uri, tab.source())?;
            }
        }
        sync(&mut language, active, self.session.borrow().document().source())
    }

    pub(super) fn stage_reviewed_sources(&self) -> Result<ReviewedSources, String> {
        let active = document_identity(self.session.borrow().document_uri())?;
        self.capture_tab();
        let project = self.project.borrow();
        Ok(ReviewedSources {
            workspace: project
                .workspace
                .clone()
                .ok_or("Open the owning project first")?,
            tabs: project.tabs.clone(),
            language: self.try_language_service()?.clone(),
            active,
        })
    }

    pub(super) fn publish_reviewed_sources(
        self: &Rc<Self>,
        reviewed: ReviewedSources,
        participants: &[String],
    ) -> Result<(), String> {
        *self.try_language_service()? = reviewed.language;
        let mut project = self.project.borrow_mut();
        project.workspace = Some(reviewed.workspace);
        project.tabs = reviewed.tabs;
        for uri in participants {
            project.linked_history.take_closed(uri);
        }
        drop(project);
        self.capture_tab();
        self.rebuild_document_tabs();
        Ok(())
    }
}

impl ReviewedSources {
    pub(super) fn tab(&mut self, editor: &DesignerWorkspace, uri: &str) -> Result<Tab, String> {
        if let Some(tab) = self
            .tabs
            .get(uri)
            .cloned()
            .or_else(|| editor.project.borrow().linked_history.closed(uri).cloned())
        {
            return Ok(tab);
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

    pub(super) fn prepare_edits(&mut self, participants: &[String]) -> Result<(), String> {
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
        Ok(())
    }
}

pub(super) fn check_saved(
    workspace: &ProjectWorkspace,
    uri: &str,
    tab: &Tab,
) -> Result<(), String> {
    if workspace.read_source(uri).map_err(|e| e.to_string())? != tab.saved {
        return Err(format!(
            "Reopen the changed file before applying the reviewed source edits: {uri}"
        ));
    }
    Ok(())
}

pub(super) fn patched(mut tab: Tab, change: &ReviewedDocument, label: &str) -> Result<Tab, String> {
    if tab.source() != change.before {
        return Err("A participating file changed before applying the review".into());
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
        return Err("The source changed since it was reviewed".into());
    }
    Ok(tab)
}

pub(super) fn sync(language: &mut LanguageService, uri: &str, source: &str) -> Result<(), String> {
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
