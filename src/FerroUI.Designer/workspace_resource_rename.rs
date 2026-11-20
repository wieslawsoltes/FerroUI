use super::{registered_rename::RenameAnchor, DesignerWorkspace};
use ferroui_designer_language::ResourceRenamePlan;
use std::rc::Rc;

#[cfg(all(test, feature = "mcp"))]
#[path = "workspace_resource_rename_tests.rs"]
mod tests;

impl DesignerWorkspace {
    pub(super) fn review_resource_rename(
        &self,
        anchor: &RenameAnchor,
        new_key: &str,
    ) -> Result<ResourceRenamePlan, String> {
        self.require_rename_anchor(anchor)?;
        self.sync_xaml_review_drafts(&anchor.uri)?;
        self.try_language_service()?
            .plan_resource_rename(&anchor.uri, &anchor.request, new_key)
            .map_err(|error| error.to_string())
    }

    pub(super) fn apply_resource_rename(
        self: &Rc<Self>,
        anchor: &RenameAnchor,
        new_key: &str,
        review: &str,
    ) -> Result<ResourceRenamePlan, String> {
        let plan = self.review_resource_rename(anchor, new_key)?;
        if plan.review != review {
            return Err("The resource rename changed. Review the source patches again.".into());
        }
        self.apply_reviewed_rename(anchor, "Rename resource and references", &plan.documents)?;
        Ok(plan)
    }

    #[cfg(feature = "mcp")]
    pub(crate) fn resource_rename(
        self: &Rc<Self>,
        offset: usize,
        new_key: &str,
        review: Option<&str>,
    ) -> Result<ResourceRenamePlan, String> {
        let anchor = self.source_rename_anchor(offset)?;
        match review {
            Some(review) => self.apply_resource_rename(&anchor, new_key, review),
            None => self.review_resource_rename(&anchor, new_key),
        }
    }
}
