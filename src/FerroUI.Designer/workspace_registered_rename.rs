use super::{
    reviewed_sources::SourceReviewAnchor,
    DesignerWorkspace,
};
use ferroui_designer_language::{
    RegisteredPropertyPlan, RegisteredPropertyTarget,
};
use std::rc::Rc;

#[cfg(all(test, feature = "mcp"))]
#[path = "workspace_registered_rename_tests.rs"]
mod tests;
#[path = "workspace_registered_rename_ui.rs"]
mod ui;

impl DesignerWorkspace {
    fn registered_property_target(
        &self,
        anchor: &SourceReviewAnchor,
    ) -> Result<RegisteredPropertyTarget, String> {
        self.try_language_service()?
            .registered_property_target(&anchor.uri, &anchor.request)
            .map_err(|e| e.to_string())
    }

    fn review_registered_property(
        &self,
        anchor: &SourceReviewAnchor,
        new_name: &str,
    ) -> Result<RegisteredPropertyPlan, String> {
        self.require_source_review_anchor(anchor)?;
        let drafts = self.refresh_move_drafts(&anchor.uri)?;
        self.try_language_service()?
            .plan_registered_property_rename_with_drafts(&anchor.uri, &anchor.request, new_name, &drafts)
            .map_err(|error| error.to_string())
    }

    fn apply_registered_property(
        self: &Rc<Self>,
        anchor: &SourceReviewAnchor,
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
        self.apply_reviewed_source_edits(anchor, "Rename registered property", &plan.documents)?;
        Ok(plan)
    }

    #[cfg(feature = "mcp")]
    pub(crate) fn registered_property_rename(
        self: &Rc<Self>,
        offset: usize,
        new_name: &str,
        review: Option<&str>,
    ) -> Result<RegisteredPropertyPlan, String> {
        let anchor = self.source_review_anchor(offset)?;
        match review {
            Some(review) => self.apply_registered_property(&anchor, new_name, review),
            None => self.review_registered_property(&anchor, new_name),
        }
    }
}
