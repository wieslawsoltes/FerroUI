use crate::rsx_document::RustDocument;
use crate::{DesignerSession, PreviewStatus};
use ferroui_designer_core::{ChangeSet, DesignerError, Diagnostic, Document, NodeId, SourceSpan, TextPatch};
use ferroui_designer_project::RustModuleSources;
use ferroui_designer_rsx::{RsxCatalog, RsxSourceMap};
use std::sync::Arc;

impl DesignerSession {
    pub fn uses_rust_source(&self) -> bool {
        self.rust.is_some()
    }

    /// The original editor buffer. `document()` provides the native XAML tree
    /// on which layout, inspection, and the animation engine operate.
    pub fn authored_source(&self) -> &str {
        self.rust
            .as_ref()
            .map_or_else(|| self.document.source(), RustDocument::source)
    }

    pub fn authored_diagnostics(&self) -> &[Diagnostic] {
        self.rust
            .as_ref()
            .map_or_else(|| self.document.diagnostics(), RustDocument::diagnostics)
    }

    pub fn authored_is_valid(&self) -> bool {
        self.authored_diagnostics().is_empty()
    }

    pub fn authored_is_dirty(&self) -> bool {
        self.rust
            .as_ref()
            .map_or_else(|| self.document.is_dirty(), RustDocument::dirty)
    }

    pub(crate) fn mark_saved_source(&mut self, source: String) {
        match self.rust.as_mut() {
            Some(rust) => rust.mark_saved(source),
            None => self.document.mark_saved_snapshot(source),
        }
    }

    pub(crate) fn rust_map(&self) -> Option<&RsxSourceMap> {
        self.rust.as_ref()?.map()
    }

    pub(crate) fn authored_span(&self, node: NodeId) -> Option<SourceSpan> {
        if !self.authored_is_valid() {
            return None;
        }
        if let Some(rust) = &self.rust {
            rust.source_span(&self.document, node)
        } else {
            self.document.ast().node(node).map(|element| element.span)
        }
    }

    pub(crate) fn rust_property_span(&self, node: NodeId, property: &str) -> Option<SourceSpan> {
        let offset = self.document.ast().node(node)?.span.start;
        self.rust_map()?.property(offset, property)
    }

    pub(crate) fn rust_state_group(&self, owner: NodeId, name: &str) -> Option<&ferroui_designer_rsx::RsxStateGroup> {
        if !self.authored_is_valid() { return None; }
        let offset = self.document.ast().node(owner)?.span.start;
        self.rust_map()?.state_group(offset, name)
    }

    pub(crate) fn open_rust_document(
        &mut self,
        source: String,
        uri: String,
        catalog: Option<Arc<RsxCatalog>>,
    ) -> Result<(), String> {
        let (document, rust) =
            RustDocument::open(source, catalog).map_err(|error| error.to_string())?;
        self.restore_authored_document(document, Some(rust), uri)
    }

    pub(crate) fn set_rust_catalog(
        &mut self,
        catalog: Option<Arc<RsxCatalog>>,
    ) -> Result<(), String> {
        let Some(rust) = &self.rust else {
            return Ok(());
        };
        let mut rust = rust.clone();
        let mut document = self.document.snapshot_without_history();
        if rust
            .set_catalog(&mut document, catalog)
            .map_err(|error| error.to_string())?
        {
            self.remember_playback_for_reload();
            self.rust = Some(rust);
            self.document = document;
            self.changed(true);
        }
        Ok(())
    }

    pub(crate) fn set_rust_modules(&mut self, modules: Option<Arc<RustModuleSources>>) -> Result<(), String> {
        let Some(rust) = &self.rust else { return Ok(()) };
        let mut rust = rust.clone();
        let mut document = self.document.snapshot_without_history();
        if rust.set_modules(&mut document, modules).map_err(|error| error.to_string())? {
            self.remember_playback_for_reload();
            self.rust = Some(rust);
            self.document = document;
            self.changed(true);
        }
        Ok(())
    }

    pub(crate) fn invalidate_rust_modules(&mut self) -> Result<(), String> {
        let Some(rust) = self.rust.as_mut() else { return Ok(()) };
        if rust.invalidate_modules(&mut self.document).map_err(|error| error.to_string())? {
            self.changed(true);
        }
        Ok(())
    }

    pub(crate) fn commit_rust_module_edit(&mut self, label: String, modules: Arc<RustModuleSources>) -> Result<(), String> {
        self.ensure_writable().map_err(|error| error.to_string())?;
        if !self.can_manipulate() || self.collaboration_actor().is_some() {
            return Err("Wait for an editable local Rust preview".into());
        }
        let mut rust = self.rust.clone().ok_or("Open a mapped Rust view")?;
        let mut document = self.document.snapshot_without_history();
        rust.set_modules(&mut document, Some(modules)).map_err(|error| error.to_string())?;
        rust.record_module_edit(label, &document);
        self.remember_playback_for_reload();
        self.rust = Some(rust);
        self.document = document;
        self.changed(true);
        Ok(())
    }

    pub(crate) fn select_rust_invocation(&mut self, start: usize) -> Result<(), String> {
        let rust = self
            .rust
            .as_mut()
            .ok_or("Open a Rust view to select an invocation")?;
        let previous = self.document.revision();
        rust.select_invocation(&mut self.document, start)
            .map_err(|error| error.to_string())?;
        if self.document.revision() != previous {
            self.reset_rust_view_state();
            self.changed(true);
        }
        Ok(())
    }

    pub(crate) fn reset_rust_view_state(&mut self) {
        self.animation.player.stop();
        self.animation.clear_program_state();
        self.selection.clear();
        self.layers.reset();
    }

    pub(crate) fn note_invalid_rust(&mut self) -> bool {
        if self.uses_rust_source() && !self.authored_is_valid() {
            self.preview.status = PreviewStatus::InvalidSource {
                revision: self.document.revision(),
            };
            true
        } else {
            false
        }
    }

    pub(crate) fn restore_rust_history(&mut self, redo: bool) -> Result<ChangeSet, DesignerError> {
        self.remember_playback_for_reload();
        let rust = self.rust.as_mut().expect("Rust source is active");
        let change = rust.restore_history(&mut self.document, redo)?;
        self.changed(true);
        Ok(change)
    }

    pub(crate) fn commit_rust_patches(
        &mut self,
        label: &str,
        patches: Vec<TextPatch>,
    ) -> Result<ChangeSet, DesignerError> {
        self.commit_scoped_rust_patches(label, patches, None)
    }

    pub(crate) fn commit_rust_style_plan(
        &mut self,
        plan: ferroui_designer_rsx::RsxStylePlan,
    ) -> Result<ChangeSet, DesignerError> {
        if plan.source_uri.is_some() {
            return Err(DesignerError::InvalidEdit("Apply this color edit to its authored Rust module".into()));
        }
        let label = if plan.shared_color.is_some() {
            "Edit shared Rust color"
        } else {
            "Edit Rust style"
        };
        self.commit_scoped_rust_patches(label, plan.patches, plan.shared_color)
    }

    fn commit_scoped_rust_patches(
        &mut self,
        label: &str,
        patches: Vec<TextPatch>,
        shared_color: Option<SourceSpan>,
    ) -> Result<ChangeSet, DesignerError> {
        self.commit_rust_edit(|rust, document| rust.apply(document, label, patches, shared_color))
    }

    pub(crate) fn commit_rust_child_edit(
        &mut self,
        label: &str,
        plan: ferroui_designer_rsx::RsxChildEdit,
        basis: &Document,
    ) -> Result<ChangeSet, DesignerError> {
        self.commit_rust_edit(|rust, document| rust.apply_child_edit(document, label, plan, basis))
    }

    fn commit_rust_edit(
        &mut self,
        update: impl FnOnce(&mut RustDocument, &mut Document) -> Result<ChangeSet, DesignerError>,
    ) -> Result<ChangeSet, DesignerError> {
        self.ensure_writable()?;
        if !self.can_manipulate() {
            return Err(DesignerError::InvalidEdit(
                "Wait for the current editable RSX preview".into(),
            ));
        }
        let mut rust = self.rust.clone().expect("Rust source is active");
        let mut document = self.document.snapshot_without_history();
        let change = update(&mut rust, &mut document)?;
        if !change.is_empty() {
            self.remember_playback_for_reload();
            self.rust = Some(rust);
            self.document = document;
        }
        Ok(change)
    }
}
