//! Keep Rust analysis off the Studio dispatcher during a type rename review.
use super::{reviewed_sources::SourceReviewAnchor, rust_review::RustReview, DesignerWorkspace};
use ferroui_designer_language::{RegisteredTypePlan, RustRenameResolution};
use std::rc::Rc;

#[derive(PartialEq, Eq)]
pub(super) struct TypeRenameTarget {
    anchor: SourceReviewAnchor,
    name: String,
}

pub(super) type TypeRenameReview = RustReview<TypeRenameTarget, RustRenameResolution>;

impl DesignerWorkspace {
    #[cfg(feature = "mcp")]
    pub(crate) fn registered_type_rename(
        self: &Rc<Self>,
        offset: usize,
        name: &str,
        review: Option<&str>,
        allow_analysis: bool,
        cancel: bool,
    ) -> Result<serde_json::Value, String> {
        use serde_json::json;
        let anchor = self.source_review_anchor(offset)?;
        if let Some(review) = review {
            return Ok(json!(self.apply_registered_type(&anchor, name, review)?));
        }
        if cancel {
            self.cancel_registered_type(&anchor, name);
            return Ok(json!({"status": "cancelled"}));
        }
        Ok(
            match self.review_registered_type(&anchor, name, allow_analysis)? {
                Some(plan) => json!(plan),
                None => json!({"status": "resolving"}),
            },
        )
    }

    pub(super) fn review_registered_type(
        &self,
        anchor: &SourceReviewAnchor,
        name: &str,
        allow_analysis: bool,
    ) -> Result<Option<RegisteredTypePlan>, String> {
        self.require_source_review_anchor(anchor)?;
        let target = TypeRenameTarget {
            anchor: anchor.clone(),
            name: name.into(),
        };
        let drafts = self.refresh_move_drafts(&anchor.uri)?;
        let existing = {
            let mut state = self.project.borrow_mut();
            state
                .type_rename
                .take()
                .filter(|review| review.matches(&target))
        };
        if let Some(mut review) = existing {
            let plan = if let Some((base, resolution)) = review.poll()? {
                Some(
                    self.try_language_service()?
                        .complete_registered_type_rename(
                            &anchor.uri,
                            &anchor.request,
                            name,
                            &drafts,
                            base,
                            resolution,
                        )
                        .map_err(|error| error.to_string())?,
                )
            } else {
                None
            };
            self.project.borrow_mut().type_rename = Some(review);
            return Ok(plan);
        }
        let plan = self
            .try_language_service()?
            .plan_registered_type_rename(&anchor.uri, &anchor.request, name, &drafts)
            .map_err(|error| error.to_string())?;
        let Some(rename) = plan.rust_rename else {
            return Ok(Some(plan));
        };
        if !allow_analysis {
            return Err("Start the type review in Studio or use allow_input to resolve Rust references through MCP".into());
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let project = self.rust_analysis_project(&rename.uri)?;
            let task = TypeRenameReview::spawn(
                target,
                plan.review,
                "FerroUI Rust type review",
                move |cancelled| {
                    ferroui_designer_language::rust_analyzer::resolve_registered_type(
                        &project, &rename, &drafts, cancelled,
                    )
                    .map_err(|error| error.to_string())
                },
            )?;
            self.project.borrow_mut().type_rename = Some(task);
            Ok(None)
        }
        #[cfg(target_arch = "wasm32")]
        {
            let _ = rename;
            Err("Resolve the Rust type in desktop Studio or VS Code".into())
        }
    }

    pub(super) fn apply_registered_type(
        self: &Rc<Self>,
        anchor: &SourceReviewAnchor,
        name: &str,
        review: &str,
    ) -> Result<RegisteredTypePlan, String> {
        let plan = self
            .review_registered_type(anchor, name, false)?
            .ok_or("Wait for the Rust type reference review before applying")?;
        if plan.review != review {
            return Err(
                "The type rename changed. Review the current source patches before applying it"
                    .into(),
            );
        }
        self.apply_reviewed_source_edits(anchor, "Rename registered type", &plan.documents)?;
        self.cancel_registered_type(anchor, name);
        Ok(plan)
    }

    pub(super) fn cancel_registered_type(&self, anchor: &SourceReviewAnchor, name: &str) {
        let target = TypeRenameTarget {
            anchor: anchor.clone(),
            name: name.into(),
        };
        let mut state = self.project.borrow_mut();
        if state
            .type_rename
            .as_ref()
            .is_some_and(|review| review.matches(&target))
        {
            state.type_rename = None;
        }
    }
}
