use super::{DesignerWorkspace, project::document_identity};
use ferroui_designer_core::{NodeId, TextPatch};
use ferroui_designer_language::{
    DictionaryExtractionPlan, DictionaryExtractionRequest, DictionaryExtractionTarget,
};
use ferroui_designer_project::CreatedSourceFile;
use std::rc::Rc;

#[path = "workspace_dictionary_review.rs"]
mod review;
#[cfg(all(test, feature = "mcp"))]
#[path = "workspace_dictionary_extraction_tests.rs"]
mod tests;

pub(super) struct ExtractionSite {
    uri: String,
    revision: u64,
    request: DictionaryExtractionRequest,
}

#[derive(Clone)]
pub(super) struct CreatedDictionary {
    pub owner: String,
    pub file: CreatedSourceFile,
}

impl DesignerWorkspace {
    #[cfg(feature = "mcp")]
    pub(crate) fn inspect_dictionary_extraction(
        self: &Rc<Self>,
        node: NodeId,
        file_name: Option<&str>,
    ) -> Result<serde_json::Value, String> {
        let site = self.extraction_site(Some(node))?;
        Ok(if let Some(file_name) = file_name {
            serde_json::json!(self.plan_extraction(&site, file_name)?)
        } else {
            serde_json::json!(self.extraction_target(&site)?)
        })
    }

    #[cfg(feature = "mcp")]
    pub(crate) fn extract_reviewed_dictionary(
        self: &Rc<Self>,
        node: NodeId,
        file_name: &str,
        review: &str,
    ) -> Result<DictionaryExtractionPlan, String> {
        let site = self.extraction_site(Some(node))?;
        self.apply_extraction(&site, file_name, review)
    }

    pub(super) fn extraction_site(
        self: &Rc<Self>,
        node: Option<NodeId>,
    ) -> Result<ExtractionSite, String> {
        if self.pending_source.get() {
            return Err("Apply the current XAML draft before extracting its resources".into());
        }
        self.synchronize_language_service()?;
        let session = self.session.borrow();
        let document = session.document();
        if session.uses_rust_source() || !document.is_valid() {
            return Err("Choose an authored XAML dictionary".into());
        }
        let offset = match node {
            Some(id) => {
                document
                    .ast()
                    .node(id)
                    .ok_or("Choose an authored resource or its owner")?
                    .name_span
                    .start
            }
            None => {
                super::source_tools::byte_from_utf16(document.source(), self.source.caret_index())
            }
        };
        let uri = session.document_uri().to_owned();
        let service = self.try_language_service()?;
        let version = service
            .version(&uri)
            .ok_or("Index the active XAML buffer first")?;
        let position = service
            .index(&uri)
            .and_then(|index| index.position(offset))
            .map_err(|error| error.to_string())?;
        Ok(ExtractionSite {
            uri,
            revision: document.revision(),
            request: DictionaryExtractionRequest { version, position },
        })
    }

    pub(super) fn extraction_target(
        &self,
        site: &ExtractionSite,
    ) -> Result<DictionaryExtractionTarget, String> {
        self.check_extraction_site(site)?;
        self.try_language_service()?
            .dictionary_extraction_target(&site.uri, site.request)
            .map_err(|e| e.to_string())
    }

    pub(super) fn plan_extraction(
        &self,
        site: &ExtractionSite,
        file_name: &str,
    ) -> Result<DictionaryExtractionPlan, String> {
        self.check_extraction_site(site)?;
        self.try_language_service()?
            .plan_dictionary_extraction(&site.uri, site.request, file_name)
            .map_err(|e| e.to_string())
    }

    pub(super) fn apply_extraction(
        self: &Rc<Self>,
        site: &ExtractionSite,
        file_name: &str,
        review: &str,
    ) -> Result<DictionaryExtractionPlan, String> {
        let plan = self.plan_extraction(site, file_name)?;
        if plan.review != review {
            return Err(
                "The extraction changed. Review the current patch before applying it".into(),
            );
        }
        let [edit] = plan.edit.edits.as_slice() else {
            return Err("Expected one checked source patch".into());
        };
        let span = self
            .try_language_service()?
            .index(&site.uri)
            .and_then(|index| index.span(edit.range))
            .map_err(|e| e.to_string())?;
        let (mut workspace, label) = {
            let mut state = self.project.borrow_mut();
            let workspace = state
                .workspace
                .clone()
                .ok_or("Open the owning project first")?;
            if state.tabs.contains_key(&plan.file.uri) {
                return Err("The dictionary destination is already open".into());
            }
            (
                workspace,
                state.linked_history.label("Extract resource dictionary")?,
            )
        };
        let patch = TextPatch {
            span,
            replacement: edit.new_text.clone(),
        };
        let file = self
            .session
            .borrow_mut()
            .commit_workspace_xaml(|document| {
                document
                    .apply_text_patches(site.revision, &label, vec![patch])
                    .map_err(|e| e.to_string())?;
                workspace
                    .publish_source_file(&plan.file.uri, &plan.file.source)
                    .map_err(|e| e.to_string())
            })?;
        let uri = file.uri().to_owned();
        {
            let mut state = self.project.borrow_mut();
            state.workspace = Some(workspace);
            state.linked_history.link_created(
                label,
                CreatedDictionary {
                    owner: plan.edit.uri.clone(),
                    file,
                },
            );
        }
        self.refresh_dictionary_file(&uri);
        Ok(plan)
    }

    fn check_extraction_site(&self, site: &ExtractionSite) -> Result<(), String> {
        let session = self.session.borrow();
        if self.pending_source.get()
            || session.document_uri() != site.uri
            || session.document().revision() != site.revision
        {
            return Err("The source changed. Open a new extraction review".into());
        }
        Ok(())
    }

    pub(super) fn step_dictionary_history(
        self: &Rc<Self>,
        label: &str,
        mut created: CreatedDictionary,
        redo: bool,
    ) -> Result<(), String> {
        let uri = created.file.uri().to_owned();
        let mut workspace = {
            let state = self.project.borrow();
            if let Some(tab) = state.tabs.get(&uri) {
                if tab.source() != created.file.source() {
                    return Err("Undo newer edits in the extracted dictionary first".into());
                }
            }
            state
                .workspace
                .clone()
                .ok_or("Open the owning project first")?
        };
        let active = document_identity(self.session.borrow().document_uri())?;
        if active != created.owner {
            return Err("Open the source view to change the extraction history".into());
        }
        let replacement = self
            .session
            .borrow_mut()
            .commit_workspace_xaml(|document| {
                if redo {
                    document.redo()
                } else {
                    document.undo()
                }
                .map_err(|error| error.to_string())?;
                if redo {
                    workspace
                        .publish_source_file(&uri, created.file.source())
                        .map(Some)
                } else {
                    workspace.undo_source_file(&created.file).map(|_| None)
                }
                .map_err(|error| error.to_string())
            })?;
        {
            let mut state = self.project.borrow_mut();
            state.workspace = Some(workspace);
            if let Some(new_file) = replacement {
                created.file = new_file;
                state.linked_history.link_created(label.into(), created);
            } else {
                state.tabs.remove(&uri);
            }
        }
        if !redo {
            self.close_language_document(&uri)?;
        }
        self.refresh_dictionary_file(&uri);
        Ok(())
    }

    fn refresh_dictionary_file(self: &Rc<Self>, uri: &str) {
        let result = self.try_language_service().and_then(|mut language| {
            language
                .refresh_project_file(uri)
                .map(|_| ())
                .map_err(|e| e.to_string())
        });
        self.rebuild_project_tree();
        self.rebuild_document_tabs();
        self.refresh(true);
        self.report(result);
    }
}
