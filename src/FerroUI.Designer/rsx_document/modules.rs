use super::RustDocument;
use ferroui_designer_core::{DesignerError, Diagnostic, Document, SourceSpan};
use ferroui_designer_language::LineIndex;
use ferroui_designer_project::RustModuleSources;
use std::sync::Arc;

impl RustDocument {
    pub(crate) fn module_sources(&self) -> Option<&Arc<RustModuleSources>> {
        self.modules.as_ref()
    }

    pub(super) fn modules_for(&self, source: &str) -> Option<Arc<RustModuleSources>> {
        let modules = self.modules.as_ref()?;
        if modules.active().source.as_ref() == source {
            return Some(Arc::clone(modules));
        }
        modules
            .with_file_text(&modules.active().uri, source)
            .map(Arc::new)
    }

    pub(crate) fn set_modules(
        &mut self,
        document: &mut Document,
        modules: Option<Arc<RustModuleSources>>,
    ) -> Result<bool, DesignerError> {
        if self.modules == modules && !self.module_error {
            return Ok(false);
        }
        let previous = std::mem::replace(&mut self.modules, modules);
        if let Err(error) = self.project_into(
            document,
            self.source.clone(),
            self.caret,
            "Refresh Rust module sources",
            false,
            None,
        ) {
            self.modules = previous;
            return Err(error);
        }
        Ok(true)
    }

    pub(crate) fn record_module_edit(&mut self, label: String, document: &Document) {
        let revision = self.history_revision(document);
        self.history.commit(label, revision.clone(), revision);
    }

    pub(crate) fn invalidate_modules(
        &mut self,
        document: &mut Document,
    ) -> Result<bool, DesignerError> {
        if self.module_error || self.map.is_none() {
            return Ok(false);
        }
        let revision = document
            .revision()
            .checked_add(1)
            .ok_or_else(|| super::invalid("Document revision exhausted"))?;
        document.advance_revision_to(revision)?;
        let position = LineIndex::new(&self.source).position(self.caret).ok();
        self.diagnostics.push(Diagnostic {
            message: "Repair the Rust module sources to update Studio".into(),
            line: position.map_or(1, |value| value.line + 1),
            column: position.map_or(1, |value| value.character + 1),
            span: SourceSpan::new(self.caret, self.caret),
        });
        self.module_error = true;
        Ok(true)
    }
}
