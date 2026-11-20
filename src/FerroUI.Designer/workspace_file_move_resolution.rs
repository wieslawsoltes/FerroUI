//! Reuse a reviewed Rust resolution across the flyout, MCP and the apply check.
use super::super::rust_review::RustReview;
use super::DesignerWorkspace;
use ferroui_designer_language::{FileMoveDraft, FileMovePlan, FileMoveRustResolution};

#[derive(PartialEq, Eq)]
pub(in crate::workspace) struct MoveTarget {
    pub(super) from: String,
    pub(super) to: String,
    pub(super) revision: u64,
}

pub(in crate::workspace) type RustMoveReview = RustReview<MoveTarget, Vec<FileMoveRustResolution>>;

impl RustMoveReview {
    #[cfg(not(target_arch = "wasm32"))]
    pub(super) fn start(
        workspace: &DesignerWorkspace,
        target: MoveTarget,
        plan: FileMovePlan,
        drafts: Vec<FileMoveDraft>,
    ) -> Result<Self, String> {
        use ferroui_designer_language::rust_analyzer::resolve_file_move;
        let project = workspace.rust_analysis_project(&plan.from)?;
        Self::spawn(
            target,
            plan.review,
            "FerroUI Rust move review",
            move |cancelled| {
                resolve_file_move(&project, &plan.generated_rust, &drafts, cancelled)
                    .map_err(|error| error.to_string())
            },
        )
    }

    #[cfg(target_arch = "wasm32")]
    pub(super) fn start(
        _: &DesignerWorkspace,
        _: MoveTarget,
        _: FileMovePlan,
        _: Vec<FileMoveDraft>,
    ) -> Result<Self, String> {
        Err("Review generated Rust references in desktop Studio or VS Code".into())
    }
}
