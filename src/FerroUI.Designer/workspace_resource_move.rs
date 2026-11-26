use super::{DesignerWorkspace, reviewed_sources::SourceReviewAnchor};
use ferroui_designer_language::{ResourceMovePlan, ResourceMoveTarget, SourceReviewRequest};
use std::rc::Rc;

#[cfg(all(test, feature = "mcp"))]
#[path = "workspace_resource_move_tests.rs"]
mod tests;
#[path = "workspace_resource_move_ui.rs"]
mod ui;

impl DesignerWorkspace {
    fn resource_move_target(
        &self,
        anchor: &SourceReviewAnchor,
    ) -> Result<ResourceMoveTarget, String> {
        self.require_source_review_anchor(anchor)?;
        self.sync_xaml_review_drafts(&anchor.uri)?;
        self.try_language_service()?
            .resource_move_target(&anchor.uri, &anchor.request)
            .map_err(|error| error.to_string())
    }

    fn review_resource_move(
        &self,
        anchor: &SourceReviewAnchor,
        uri: &str,
        request: &SourceReviewRequest,
    ) -> Result<ResourceMovePlan, String> {
        self.require_source_review_anchor(anchor)?;
        self.sync_xaml_review_drafts(&anchor.uri)?;
        self.try_language_service()?
            .plan_resource_move(&anchor.uri, &anchor.request, uri, request)
            .map_err(|error| error.to_string())
    }

    fn apply_resource_move(
        self: &Rc<Self>,
        anchor: &SourceReviewAnchor,
        uri: &str,
        request: &SourceReviewRequest,
        review: &str,
    ) -> Result<ResourceMovePlan, String> {
        let plan = self.review_resource_move(anchor, uri, request)?;
        if plan.review != review {
            return Err("The resource move changed. Review the current source patches.".into());
        }
        self.apply_reviewed_source_edits(anchor, "Move resource declaration", &plan.documents)?;
        Ok(plan)
    }

    #[cfg(feature = "mcp")]
    pub(crate) fn resource_move_targets_at(
        self: &Rc<Self>,
        offset: usize,
    ) -> Result<ResourceMoveTarget, String> {
        self.resource_move_target(&self.source_review_anchor(offset)?)
    }

    #[cfg(feature = "mcp")]
    pub(crate) fn resource_move(
        self: &Rc<Self>,
        offset: usize,
        uri: &str,
        request: &SourceReviewRequest,
        review: Option<&str>,
    ) -> Result<ResourceMovePlan, String> {
        let anchor = self.source_review_anchor(offset)?;
        match review {
            Some(review) => self.apply_resource_move(&anchor, uri, request, review),
            None => self.review_resource_move(&anchor, uri, request),
        }
    }
}
