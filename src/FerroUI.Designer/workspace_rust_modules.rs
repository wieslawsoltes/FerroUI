//! Studio's workspace adapter for Rust palette module transactions.
use super::{DesignerWorkspace, project::document_identity, reviewed_sources::check_saved};
use ferroui_designer_core::TextPatch;
use ferroui_designer_language::{LineIndex, Range};
use ferroui_designer_project::{
    PreviewSourceBuffer, RustModuleProject, RustModuleSources, file_path,
};
use ferroui_designer_rsx::{RsxStyleEdit, RsxStylePlan};
use std::{rc::Rc, sync::Arc};

impl DesignerWorkspace {
    pub(super) fn capture_rust_modules(
        &self,
        uri: &str,
        source: &str,
    ) -> Option<Arc<RustModuleSources>> {
        let active = file_path(&document_identity(uri).ok()?).ok()?;
        let project = self.project.borrow();
        let workspace = project.workspace.as_ref()?;
        let descriptor = workspace
            .solution
            .projects
            .iter()
            .filter(|descriptor| {
                active.starts_with(&descriptor.root) && !descriptor.targets.is_empty()
            })
            .max_by_key(|descriptor| descriptor.root.components().count())?;
        let drafts: Vec<_> = project
            .tabs
            .iter()
            .filter(|(_, tab)| tab.rust.is_some() && tab.source() != tab.saved)
            .map(|(uri, tab)| PreviewSourceBuffer {
                uri: uri.clone(),
                source: Some(tab.source().into()),
            })
            .collect();
        RustModuleProject::from(descriptor)
            .capture(&workspace.solution.root, uri, source, &drafts)
            .map(Arc::new)
    }

    pub(super) fn refresh_rust_modules(&self) -> Result<(), String> {
        let (modules, previously_mapped) = {
            let session = self.session.borrow();
            let Some(rust) = session.rust.as_ref() else {
                return Ok(());
            };
            if rust.map().is_none() {
                return Ok(());
            }
            let modules = self.capture_rust_modules(session.document_uri(), rust.source());
            (modules, rust.module_sources().is_some())
        };
        let mut session = self.session.borrow_mut();
        if modules.is_none() && previously_mapped {
            session.invalidate_rust_modules()
        } else {
            session.set_rust_modules(modules)
        }
    }

    pub(crate) fn edit_rust_style(self: &Rc<Self>, edit: &RsxStyleEdit) -> Result<(), String> {
        if self.pending_source.get() {
            return Err("Apply the current Rust draft before editing its styles".into());
        }
        let (plan, modules) = {
            let session = self.session.borrow();
            let projection = session.rust_style_projection()?;
            let modules = session
                .rust
                .as_ref()
                .and_then(|rust| rust.module_sources())
                .filter(|_| projection.module_sources().is_some())
                .cloned();
            if let Some(modules) = &modules {
                self.require_rust_module_snapshot(session.document_uri(), modules)?;
            }
            (projection.plan_style_source(edit)?, modules)
        };
        if plan.source_uri.is_some() && !plan.patches.is_empty() {
            self.apply_shared_module_color(
                plan,
                modules.ok_or("The Rust module source is unavailable")?,
            )?;
        } else {
            self.session.borrow_mut().apply_rust_style_plan(plan)?;
        }
        self.refresh(true);
        Ok(())
    }

    fn require_rust_module_snapshot(
        &self,
        active: &str,
        modules: &RustModuleSources,
    ) -> Result<(), String> {
        let draft = self.capture_rust_modules(active, &modules.active().source);
        if draft.as_deref() != Some(modules) {
            return Err(
                "The Rust module sources changed. Reopen Styles & Themes for the current preview"
                    .into(),
            );
        }
        Ok(())
    }

    fn apply_shared_module_color(
        self: &Rc<Self>,
        plan: RsxStylePlan,
        modules: Arc<RustModuleSources>,
    ) -> Result<(), String> {
        let uri = plan
            .source_uri
            .ok_or("Choose a shared color in an authored Rust module")?;
        let captured = modules
            .source(&uri)
            .ok_or("The supplying Rust module is outside the accepted snapshot")?;
        let mut staged = self.stage_reviewed_sources()?;
        let mut tab = staged.tab(self, &uri)?;
        check_saved(&staged.workspace, &uri, &tab)?;
        if tab.source() != captured.source.as_ref() {
            return Err("The supplying Rust buffer changed before applying the color".into());
        }
        let after =
            TextPatch::apply_all(tab.source(), plan.patches).map_err(|error| error.to_string())?;
        let updated = modules
            .with_file_text(&uri, &after)
            .ok_or("The color edit changes the module layout")?;
        let label = self
            .project
            .borrow_mut()
            .linked_history
            .label("Edit shared Rust color")?;
        let rust = tab
            .rust
            .as_mut()
            .ok_or("The supplying document is no longer Rust source")?;
        rust.update(&mut tab.document, after.into(), &label, false)
            .map_err(|error| error.to_string())?;
        staged.tabs.insert(uri.clone(), tab);
        let participants = vec![staged.active.clone(), uri];
        staged.prepare_edits(&participants)?;
        self.session
            .borrow_mut()
            .commit_rust_module_edit(label.clone(), Arc::new(updated))?;
        self.publish_reviewed_sources(staged, &participants)?;
        self.project
            .borrow_mut()
            .linked_history
            .link(label, participants);
        Ok(())
    }

    pub(crate) fn reveal_rust_style_source(
        self: &Rc<Self>,
        uri: &str,
        range: Range,
    ) -> Result<(), String> {
        self.visit_rust_module(uri, |source| {
            LineIndex::new(source)
                .span(range)
                .map_err(|error| error.to_string())
        })
    }

    pub(super) fn reveal_rust_style_module_span(
        self: &Rc<Self>,
        uri: &str,
        span: ferroui_designer_core::SourceSpan,
    ) -> Result<(), String> {
        self.visit_rust_module(uri, |source| {
            source
                .get(span.range())
                .map(|_| span)
                .ok_or("The Rust declaration changed".into())
        })
    }

    fn visit_rust_module(
        self: &Rc<Self>,
        uri: &str,
        locate: impl FnOnce(&str) -> Result<ferroui_designer_core::SourceSpan, String>,
    ) -> Result<(), String> {
        if self.pending_source.get() {
            return Err("Apply the current Rust draft before revealing a declaration".into());
        }
        let source = {
            let session = self.session.borrow();
            let projection = session.rust_style_projection()?;
            let modules = projection
                .module_sources()
                .ok_or("The Rust module source is unavailable")?;
            self.require_rust_module_snapshot(session.document_uri(), modules)?;
            modules
                .source(uri)
                .ok_or("The source is outside the accepted Rust module snapshot")?
                .source
                .clone()
        };
        let span = locate(&source)?;
        self.visit_source_document(uri, |workspace| {
            if workspace.session.borrow().authored_source() != source.as_ref() {
                return Err("The Rust source changed before opening its declaration".into());
            }
            workspace.select_source_span(span);
            Ok(())
        })
    }
}

#[cfg(all(test, feature = "mcp"))]
#[path = "workspace_rust_module_tests.rs"]
mod tests;
