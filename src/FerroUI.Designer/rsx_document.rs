//! Authored Rust text and its editable native projection have separate coordinates.
mod edits;
mod history;
mod modules;
mod templates;

use ferroui_designer_core::{
    ChangeSet, DesignerError, Diagnostic, Document, SourceHistory, SourceSpan, SourceUpdate,
    TextPatch,
};
use ferroui_designer_language::LineIndex;
use ferroui_designer_project::RustModuleSources;
use ferroui_designer_rsx::{RsxCatalog, RsxSourceMap};
use history::{update_markup, ProjectedIdentity, RustRevision};
use std::sync::Arc;

const MAX_SOURCE_BYTES: usize = 1024 * 1024;
const EMPTY_PREVIEW: &str = r#"<Canvas xmlns="https://github.com/ferroui" />"#;

#[derive(Clone)]
pub(crate) struct RustDocument {
    source: Arc<str>,
    saved: Arc<str>,
    caret: usize,
    catalog: Option<Arc<RsxCatalog>>,
    modules: Option<Arc<RustModuleSources>>,
    module_error: bool,
    map: Option<Arc<RsxSourceMap>>,
    scope_changed: bool,
    template_scopes: Vec<templates::TemplateScope>,
    diagnostics: Vec<Diagnostic>,
    history: SourceHistory<RustRevision>,
}

impl RustDocument {
    pub(crate) fn for_uri(uri: &str) -> bool {
        uri.split(['?', '#'])
            .next()
            .and_then(|path| path.rsplit_once('.'))
            .is_some_and(|(_, ext)| ext.eq_ignore_ascii_case("rs"))
    }

    pub(crate) fn open(
        source: String,
        catalog: Option<Arc<RsxCatalog>>,
    ) -> Result<(Document, Self), DesignerError> {
        let source: Arc<str> = source.into();
        let mut rust = Self {
            source: source.clone(),
            saved: source.clone(),
            caret: 0,
            catalog,
            modules: None,
            module_error: false,
            map: None,
            scope_changed: false,
            template_scopes: Vec::new(),
            diagnostics: Vec::new(),
            history: SourceHistory::default(),
        };
        let mut document = Document::new(EMPTY_PREVIEW)?;
        document.set_history_limit(0);
        rust.project_into(&mut document, source, 0, "Open Rust view", false, None)?;
        rust.scope_changed = false;
        Ok((document, rust))
    }

    pub(crate) fn source(&self) -> &str {
        &self.source
    }
    pub(crate) fn dirty(&self) -> bool {
        self.source != self.saved
    }
    pub(crate) fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }
    pub(crate) fn map(&self) -> Option<&RsxSourceMap> {
        self.map.as_deref()
    }
    pub(crate) fn take_scope_change(&mut self) -> bool {
        std::mem::take(&mut self.scope_changed)
    }
    pub(crate) fn mark_saved(&mut self, source: String) {
        self.saved = source.into();
    }
    pub(crate) fn can_undo(&self) -> bool {
        self.history.undo_label().is_some()
    }
    pub(crate) fn can_redo(&self) -> bool {
        self.history.redo_label().is_some()
    }

    pub(crate) fn history_label(&self, redo: bool) -> Option<&str> {
        if redo { self.history.redo_label() } else { self.history.undo_label() }
    }

    fn catalog(&self) -> &RsxCatalog {
        self.catalog
            .as_deref()
            .unwrap_or_else(|| RsxCatalog::built_in())
    }

    pub(crate) fn projection(&self) -> Result<ferroui_designer_rsx::RsxProjection<'_>, String> {
        let map = self.map().ok_or("Repair the Rust buffer before editing its layout")?;
        self.catalog().project_with_modules(&self.source, map.invocation.start, self.modules.as_deref())
    }

    pub(crate) fn update(
        &mut self,
        document: &mut Document,
        source: Arc<str>,
        label: &str,
        require_valid: bool,
    ) -> Result<SourceUpdate, DesignerError> {
        if source == self.source {
            return Ok(SourceUpdate {
                change: self.unchanged(document.revision(), label),
                valid: self.map.is_some(),
                diagnostics: self.diagnostics.clone(),
            });
        }
        let before = self.history_revision(document);
        let (caret, anchor_replaced) = carry_caret(&self.source, &source, self.caret);
        let update = self.project_into(document, source, caret, label, require_valid, None)?;
        self.scope_changed |= anchor_replaced;
        self.history.commit(label.to_owned(), before, self.history_revision(document));
        Ok(update)
    }

    pub(crate) fn record_workspace_operation(&mut self, document: &mut Document, label: String) -> Result<(), DesignerError> {
        let before = self.history_revision(document);
        document.record_workspace_operation(document.revision(), label.clone())?;
        self.history.commit(label, before, self.history_revision(document));
        Ok(())
    }

    pub(crate) fn select_invocation(
        &mut self,
        document: &mut Document,
        start: usize,
    ) -> Result<(), DesignerError> {
        if self.map().is_some_and(|map| map.invocation.start == start) {
            return Ok(());
        }
        self.navigate_to_invocation(document, start)?;
        self.template_scopes.clear();
        Ok(())
    }

    fn navigate_to_invocation(
        &mut self,
        document: &mut Document,
        start: usize,
    ) -> Result<(), DesignerError> {
        let map = self
            .map()
            .ok_or_else(|| invalid("Repair the Rust buffer before switching views"))?;
        if map.invocation.start == start {
            return Ok(());
        }
        if !map.blocks.iter().any(|block| block.start == start) {
            return Err(invalid("Choose an RSX invocation in the current file"));
        }
        self.project_into(
            document,
            self.source.clone(),
            start,
            "Select Rust view",
            true,
            None,
        )?;
        Ok(())
    }

    pub(crate) fn set_catalog(
        &mut self,
        document: &mut Document,
        catalog: Option<Arc<RsxCatalog>>,
    ) -> Result<bool, DesignerError> {
        if match (&catalog, &self.catalog) {
            (Some(current), Some(previous)) => Arc::ptr_eq(current, previous),
            (None, None) => true,
            _ => false,
        } {
            return Ok(false);
        }
        let previous = std::mem::replace(&mut self.catalog, catalog);
        if let Err(error) = self.project_into(
            document,
            self.source.clone(),
            self.caret,
            "Refresh Rust registry",
            false,
            None,
        ) {
            self.catalog = previous;
            return Err(error);
        }
        Ok(true)
    }

    fn project_into(
        &mut self,
        document: &mut Document,
        source: Arc<str>,
        caret: usize,
        label: &str,
        require_valid: bool,
        identity: Option<ProjectedIdentity<'_>>,
    ) -> Result<SourceUpdate, DesignerError> {
        let previous = document.revision();
        let next = previous
            .checked_add(1)
            .ok_or_else(|| invalid("Document revision exhausted"))?;
        let mut candidate = document.snapshot_without_history();
        let modules = self.modules_for(&source);
        let projection = if source.len() <= MAX_SOURCE_BYTES {
            self.catalog()
                .project_with_modules(&source, caret, modules.as_deref())
                .map(|projection| projection.mapped_markup())
        } else {
            Err("The RSX designer supports up to 1 MiB of source".into())
        };
        let (map, diagnostics, mut change) = match projection {
            Ok((markup, map)) if !require_valid || map.invocation.start == caret => {
                let change = match identity {
                    Some(identity) => identity.apply(&mut candidate, label, markup)?,
                    None => update_markup(&mut candidate, label, markup)?,
                };
                (Some(Arc::new(map)), Vec::new(), change)
            }
            Ok(_) => return Err(invalid("The edit would change the selected RSX invocation")),
            Err(error) if require_valid => return Err(invalid(error)),
            Err(message) => {
                let caret = caret.min(source.len());
                let position = LineIndex::new(&source).position(caret).ok();
                let diagnostic = Diagnostic {
                    message,
                    line: position.map_or(1, |value| value.line + 1),
                    column: position.map_or(1, |value| value.character + 1),
                    span: SourceSpan::new(caret, caret),
                };
                (None, vec![diagnostic], self.unchanged(previous, label))
            }
        };
        // Fence the previous native frame even when only Rust comments changed.
        candidate.advance_revision_to(next)?;
        self.caret = map.as_ref().map_or(caret, |map| map.invocation.start);
        self.scope_changed |= map.is_some() && self.caret != caret;
        self.map = map;
        if modules.is_some() {
            self.modules = modules;
        }
        self.diagnostics = diagnostics;
        self.module_error = false;
        change.previous_revision = previous;
        change.revision = candidate.revision();
        change.patches = TextPatch::between(&self.source, &source)
            .into_iter()
            .collect();
        if let Some(patch) = change.patches.first() {
            self.carry_template_scopes(patch);
        }
        change.valid = self.map.is_some();
        self.source = source;
        *document = candidate;
        Ok(SourceUpdate {
            valid: change.valid,
            diagnostics: self.diagnostics.clone(),
            change,
        })
    }

    fn unchanged(&self, revision: u64, label: &str) -> ChangeSet {
        ChangeSet {
            label: label.into(),
            previous_revision: revision,
            revision,
            inserted: Vec::new(),
            removed: Vec::new(),
            changed: Vec::new(),
            patches: Vec::new(),
            valid: self.map.is_some(),
        }
    }
}

fn carry_caret(before: &str, after: &str, caret: usize) -> (usize, bool) {
    let Some(patch) = TextPatch::between(before, after) else {
        return (caret, false);
    };
    carry_anchor(&patch, caret)
}

fn carry_anchor(patch: &TextPatch, caret: usize) -> (usize, bool) {
    if caret >= patch.span.end {
        (caret - patch.span.len() + patch.replacement.len(), false)
    } else {
        (caret.min(patch.span.start), caret >= patch.span.start)
    }
}

fn invalid(message: impl Into<String>) -> DesignerError {
    DesignerError::InvalidEdit(message.into())
}
