//! Studio's grouped history for a reviewed asset and its authored references.

use super::{
    check_saved, document_identity, file_uri, patched, DesignerWorkspace, FileMovePlan,
    MovedProjectFile, PreviewStageSourceMoves,
};
use ferroui_designer_project::MovedAssetFile;
use std::{path::Path, rc::Rc};

#[derive(Clone)]
pub(in crate::workspace) struct MovedAsset {
    file: MovedAssetFile,
    pub(super) participants: Vec<String>,
}

impl DesignerWorkspace {
    pub(crate) fn published_asset_source(&self, address: &str) -> Result<String, String> {
        let session = self.session.borrow();
        let catalog = session
            .preview
            .packaged_assets()
            .ok_or("Build the owning package before moving a published asset")?;
        let input = catalog
            .resolve(address)
            .map_err(|error| error.to_string())?
            .ok_or("Choose a published file in the current Assets view")?;
        file_uri(input.file).map_err(|error| error.to_string())
    }

    pub(crate) fn plan_project_asset_move(
        &self,
        address: &str,
        destination: &str,
    ) -> Result<FileMovePlan, String> {
        let from = self.published_asset_source(address)?;
        let destination = Path::new(destination);
        let relative = !destination.is_absolute()
            && destination
                .components()
                .all(|part| matches!(part, std::path::Component::Normal(_)));
        if !relative {
            return Err("Choose a destination relative to the workspace folder".into());
        }
        let to = {
            let project = self.project.borrow();
            let workspace = project
                .workspace
                .as_ref()
                .ok_or("Open the owning project first")?;
            file_uri(workspace.solution.root.join(destination))
                .map_err(|error| error.to_string())?
        };
        let active = document_identity(self.session.borrow().document_uri())?;
        let drafts = self.refresh_move_drafts(&active)?;
        self.try_language_service()?
            .plan_asset_move(&from, &to, &drafts)
            .map_err(|error| error.to_string())
    }

    pub(crate) fn apply_project_asset_move(
        self: &Rc<Self>,
        address: &str,
        destination: &str,
        review: &str,
    ) -> Result<FileMovePlan, String> {
        self.ensure_local_file_move()?;
        let plan = self.plan_project_asset_move(address, destination)?;
        if plan.review != review {
            return Err("The asset move changed. Review the current file and source patches before applying it".into());
        }
        let mut staged = self.stage_reviewed_sources()?;
        let label = self
            .project
            .borrow_mut()
            .linked_history
            .label("Move published asset")?;
        let mut participants = Vec::new();
        for change in &plan.documents {
            let uri = &change.edit.uri;
            let tab = staged.tab(self, uri)?;
            check_saved(&staged.workspace, uri, &tab)?;
            staged
                .tabs
                .insert(uri.clone(), patched(tab, change, &label)?);
            participants.push(uri.clone());
        }
        if !participants.contains(&staged.active) {
            let tab = staged
                .tabs
                .get_mut(&staged.active)
                .ok_or("Open an authored view before moving the asset")?;
            check_saved(&staged.workspace, &staged.active, tab)?;
            if let Some(rust) = tab.rust.as_mut() {
                rust.record_workspace_operation(&mut tab.document, label.clone())
            } else {
                tab.document
                    .record_workspace_operation(tab.document.revision(), label.clone())
            }
            .map_err(|error| error.to_string())?;
            participants.push(staged.active.clone());
        }
        staged.prepare_edits(&participants)?;
        staged
            .language
            .rename_uri(&plan.from, &plan.to)
            .map_err(|error| error.to_string())?;
        let stage_moves = PreviewStageSourceMoves::default();
        let digest = plan
            .asset_digest
            .as_deref()
            .ok_or("The review has no asset digest")?;
        let file = staged
            .workspace
            .move_asset_file(&plan.from, &plan.to, digest)
            .map_err(|error| error.to_string())?;
        self.project.borrow_mut().linked_history.moves.insert(
            label,
            MovedProjectFile::Asset(MovedAsset { file, participants }),
        );
        self.accept_file_move(staged, &plan.from, &plan.to, &stage_moves)?;
        Ok(plan)
    }

    pub(super) fn step_asset_move(
        self: &Rc<Self>,
        label: &str,
        moved: &MovedAsset,
        redo: bool,
    ) -> Result<(), String> {
        self.ensure_local_file_move()?;
        let (from, to) = if redo {
            (&moved.file.from, &moved.file.to)
        } else {
            (&moved.file.to, &moved.file.from)
        };
        let mut staged = self.stage_reviewed_sources()?;
        for uri in &moved.participants {
            let mut tab = staged.tab(self, uri)?;
            if tab.history_label(redo) != Some(label) {
                return Err("Finish newer history in the files changed by this move first".into());
            }
            check_saved(&staged.workspace, uri, &tab)?;
            tab.step_history(redo)?;
            staged.tabs.insert(uri.clone(), tab);
        }
        staged.prepare_edits(&moved.participants)?;
        staged
            .language
            .rename_uri(from, to)
            .map_err(|error| error.to_string())?;
        let stage_moves = PreviewStageSourceMoves::default();
        staged
            .workspace
            .replay_asset_move(&moved.file, redo)
            .map_err(|error| error.to_string())?;
        self.accept_file_move(staged, from, to, &stage_moves)
    }
}
