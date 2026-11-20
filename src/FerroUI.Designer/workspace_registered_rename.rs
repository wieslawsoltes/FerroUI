use super::{
    reviewed_sources::{check_saved, patched, sync},
    DesignerWorkspace,
};
use ferroui_designer_core::Edit;
use ferroui_designer_language::{
    HandlerDraft, RegisteredPropertyPlan, RegisteredPropertyRequest, RegisteredPropertyTarget,
    ReviewedDocument,
};
use std::rc::Rc;

#[cfg(all(test, feature = "mcp"))]
#[path = "workspace_registered_rename_tests.rs"]
mod tests;
#[path = "workspace_registered_rename_ui.rs"]
mod ui;

#[derive(Clone, PartialEq, Eq)]
pub(super) struct RenameAnchor {
    pub(super) uri: String,
    pub(super) revision: u64,
    pub(super) request: RegisteredPropertyRequest,
}

impl DesignerWorkspace {
    pub(super) fn registered_rename_anchor(
        self: &Rc<Self>,
        offset: usize,
    ) -> Result<RenameAnchor, String> {
        if self.pending_source.get() {
            return Err("Apply the current source draft before reviewing the rename".into());
        }
        let uri = self.current_xaml_file()?;
        self.synchronize_language_service()?;
        let revision = self.session.borrow().document().revision();
        let tool = self.try_language_service()?;
        let request = RegisteredPropertyRequest {
            version: tool
                .version(&uri)
                .ok_or("Index the active XAML file before reviewing")?,
            position: tool
                .index(&uri)
                .and_then(|index| index.position(offset))
                .map_err(|e| e.to_string())?,
        };
        Ok(RenameAnchor {
            uri,
            request,
            revision,
        })
    }

    pub(super) fn require_rename_anchor(&self, anchor: &RenameAnchor) -> Result<(), String> {
        if self.pending_source.get()
            || self.current_xaml_file()? != anchor.uri
            || self.session.borrow().document().revision() != anchor.revision
        {
            return Err("The XAML source changed. Prepare the rename review again".into());
        }
        Ok(())
    }

    fn registered_property_target(
        &self,
        anchor: &RenameAnchor,
    ) -> Result<RegisteredPropertyTarget, String> {
        self.try_language_service()?
            .registered_property_target(&anchor.uri, &anchor.request)
            .map_err(|e| e.to_string())
    }

    fn review_registered_property(
        &self,
        anchor: &RenameAnchor,
        new_name: &str,
    ) -> Result<RegisteredPropertyPlan, String> {
        self.require_rename_anchor(anchor)?;
        let project = self.project.borrow();
        let mut language = self.try_language_service()?;
        for (uri, tab) in &project.tabs {
            if uri != &anchor.uri && uri.starts_with("file://") && tab.rust.is_none() {
                sync(&mut language, uri, tab.source())?;
            }
        }
        let target = language
            .registered_property_target(&anchor.uri, &anchor.request)
            .map_err(|e| e.to_string())?;
        let draft = project.tabs.get(&target.rust_uri).map(|tab| HandlerDraft {
            uri: target.rust_uri,
            version: tab.document.revision() as i64,
            source: tab.source().to_owned(),
        });
        language
            .plan_registered_property_rename(&anchor.uri, &anchor.request, new_name, draft.as_ref())
            .map_err(|error| error.to_string())
    }

    fn apply_registered_property(
        self: &Rc<Self>,
        anchor: &RenameAnchor,
        new_name: &str,
        review: &str,
    ) -> Result<RegisteredPropertyPlan, String> {
        let plan = self.review_registered_property(anchor, new_name)?;
        if plan.review != review {
            return Err(
                "The property rename changed. Review the current source patches before applying it"
                    .into(),
            );
        }
        self.apply_reviewed_rename(anchor, "Rename registered property", &plan.documents)?;
        Ok(plan)
    }

    pub(super) fn apply_reviewed_rename(
        self: &Rc<Self>,
        anchor: &RenameAnchor,
        label: &str,
        documents: &[ReviewedDocument],
    ) -> Result<(), String> {
        self.require_rename_anchor(anchor)?;
        {
            let session = self.session.borrow();
            session
                .ensure_writable()
                .map_err(|error| error.to_string())?;
            if session.collaboration_actor().is_some() {
                return Err(
                    "Open the local project for source edits spanning XAML and Rust".into(),
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

    #[cfg(feature = "mcp")]
    pub(crate) fn registered_property_rename(
        self: &Rc<Self>,
        offset: usize,
        new_name: &str,
        review: Option<&str>,
    ) -> Result<RegisteredPropertyPlan, String> {
        let anchor = self.registered_rename_anchor(offset)?;
        match review {
            Some(review) => self.apply_registered_property(&anchor, new_name, review),
            None => self.review_registered_property(&anchor, new_name),
        }
    }
}
