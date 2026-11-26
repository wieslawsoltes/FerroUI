//! Stage reviewed source changes before publishing editor and project buffers.
use super::{
    DesignerWorkspace,
    project::{Tab, document_identity},
};
use crate::rsx_document::RustDocument;
use ferroui_designer_core::{Edit, TextPatch};
use ferroui_designer_language::{LanguageService, LineIndex, ReviewedDocument, SourceReviewRequest};
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

#[derive(Clone, PartialEq, Eq)]
pub(super) struct SourceReviewAnchor {
    pub(super) uri: String,
    pub(super) revision: u64,
    pub(super) request: SourceReviewRequest,
}

impl DesignerWorkspace {
    pub(super) fn source_review_anchor(
        self: &Rc<Self>,
        offset: usize,
    ) -> Result<SourceReviewAnchor, String> {
        if self.pending_source.get() {
            return Err("Apply the current source draft before reviewing changes".into());
        }
        let uri = self.current_xaml_file()?;
        self.synchronize_language_service()?;
        let revision = self.session.borrow().document().revision();
        let tool = self.try_language_service()?;
        let request = SourceReviewRequest {
            version: tool
                .version(&uri)
                .ok_or("Index the active XAML file before reviewing")?,
            position: tool
                .index(&uri)
                .and_then(|index| index.position(offset))
                .map_err(|e| e.to_string())?,
        };
        Ok(SourceReviewAnchor {
            uri,
            request,
            revision,
        })
    }

    pub(super) fn require_source_review_anchor(&self, anchor: &SourceReviewAnchor) -> Result<(), String> {
        if self.pending_source.get()
            || self.current_xaml_file()? != anchor.uri
            || self.session.borrow().document().revision() != anchor.revision
        {
            return Err("The XAML source changed. Start a new source review".into());
        }
        Ok(())
    }

    pub(super) fn apply_reviewed_source_edits(
        self: &Rc<Self>,
        anchor: &SourceReviewAnchor,
        label: &str,
        documents: &[ReviewedDocument],
    ) -> Result<(), String> {
        self.require_source_review_anchor(anchor)?;
        {
            let session = self.session.borrow();
            session
                .ensure_writable()
                .map_err(|error| error.to_string())?;
            if session.collaboration_actor().is_some() {
                return Err(
                    "Open the local project for source edits spanning files".into(),
                );
            }
        }
        let mut staged = self.stage_reviewed_sources()?;
        let label = self.project.borrow_mut().linked_history.label(label)?;
        let mut participants = Vec::new();
        for change in documents {
            let uri = &change.edit.uri;
            let tab = staged.tab(self, uri)?;
            check_saved(&staged.workspace, uri, &tab)?;
            staged
                .tabs
                .insert(uri.clone(), patched(tab, change, &label)?);
            participants.push(uri.clone());
        }
        staged.prepare_edits(&participants)?;
        if let Some(source) = documents
            .iter()
            .find(|change| change.edit.uri == anchor.uri)
        {
            self.session
                .borrow_mut()
                .apply_at(
                    anchor.revision,
                    &label,
                    vec![Edit::ReplaceSource {
                        source: source.after.clone(),
                    }],
                )
                .map_err(|error| error.to_string())?;
        } else {
            self.session
                .borrow_mut()
                .record_workspace_operation(anchor.revision, label.clone())
                .map_err(|error| error.to_string())?;
            participants.push(anchor.uri.clone());
        }
        self.publish_reviewed_sources(staged, &participants)?;
        self.project
            .borrow_mut()
            .linked_history
            .link(label, participants);
        self.refresh(true);
        Ok(())
    }

    pub(super) fn sync_xaml_review_drafts(&self, active: &str) -> Result<(), String> {
        let project = self.project.borrow();
        let mut language = self.try_language_service()?;
        for (uri, tab) in &project.tabs {
            if uri.starts_with("file://") && uri != active && tab.rust.is_none() {
                sync(&mut language, uri, tab.source())?;
            }
        }
        let session = self.session.borrow();
        if !session.uses_rust_source() {
            sync(&mut language, active, session.document().source())?;
        }
        Ok(())
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
