use super::{
    DesignerWorkspace,
    project::document_identity,
};
use super::reviewed_sources::{ReviewedSources, check_saved, patched, sync};
use crate::PreviewStageSourceMoves;
use ferroui_designer_language::{
    FileMoveDraft, FileMovePlan,
};
use ferroui_designer_project::{
    MovedSourceFile, SavedDesignFile, SavedReferenceEdit, file_path, file_uri,
};
use std::{path::Path, rc::Rc};

#[path = "workspace_file_move_resolution.rs"]
mod resolution;
#[path = "workspace_file_move_review.rs"]
mod review;
pub(super) use resolution::RustMoveReview;
#[cfg(all(test, feature = "mcp"))]
#[path = "workspace_file_move_tests.rs"]
mod tests;

#[derive(Clone)]
pub(super) struct MovedXaml {
    file: MovedSourceFile,
    participants: Vec<String>,
}

impl MovedXaml {
    pub(super) fn participates(&self, uri: &str) -> bool {
        uri == self.file.to || self.participants.iter().any(|file| file == uri)
    }
}

impl DesignerWorkspace {
    pub(crate) fn plan_project_file_move(
        self: &Rc<Self>,
        destination: &str,
        allow_analyzer: bool,
    ) -> Result<Option<FileMovePlan>, String> {
        let target = self.move_target(destination)?;
        let existing = {
            let mut project = self.project.borrow_mut();
            if project
                .rust_move
                .as_ref()
                .is_some_and(|review| review.matches(&target))
            {
                project.rust_move.take()
            } else {
                None
            }
        };
        if let Some(mut review) = existing {
            let plan = match review.poll()? {
                Some((base, resolutions)) => {
                    let rust_drafts = self.refresh_move_drafts(&target.from)?;
                    Some(
                        self.try_language_service()?
                            .complete_file_move(
                                &target.from,
                                &target.to,
                                &rust_drafts,
                                base,
                                resolutions,
                            )
                            .map_err(|error| error.to_string())?,
                    )
                }
                None => None,
            };
            self.project.borrow_mut().rust_move = Some(review);
            return Ok(plan);
        }
        let rust_drafts = self.refresh_move_drafts(&target.from)?;
        let plan = self
            .try_language_service()?
            .plan_file_move_with_drafts(&target.from, &target.to, &rust_drafts)
            .map_err(|error| error.to_string())?;
        if plan.generated_rust.is_empty() {
            return Ok(Some(plan));
        }
        if !allow_analyzer {
            return Err("Start a Rust move review in Studio or use allow_input to run rust-analyzer through MCP".into());
        }
        self.project.borrow_mut().rust_move = None;
        let review = RustMoveReview::start(self, target, plan, rust_drafts)?;
        self.project.borrow_mut().rust_move = Some(review);
        Ok(None)
    }

    fn move_target(&self, destination: &str) -> Result<resolution::MoveTarget, String> {
        if self.pending_source.get() {
            return Err("Apply the current draft before moving its file".into());
        }
        let from = self.current_xaml_file()?;
        let state = self.project.borrow();
        let workspace = state
            .workspace
            .as_ref()
            .ok_or("Open the owning project first")?;
        let to = file_uri(workspace.solution.root.join(Path::new(destination)))
            .map_err(|e| e.to_string())?;
        Ok(resolution::MoveTarget {
            from,
            to,
            revision: self.session.borrow().document().revision(),
        })
    }

    pub(super) fn refresh_move_drafts(&self, from: &str) -> Result<Vec<FileMoveDraft>, String> {
        let state = self.project.borrow();
        let mut language = self.try_language_service()?;
        let mut rust_drafts = Vec::new();
        for (uri, tab) in &state.tabs {
            if !uri.starts_with("file://") || uri == from {
                continue;
            }
            if tab.rust.is_some() {
                rust_drafts.push(FileMoveDraft {
                    uri: uri.clone(),
                    version: i64::try_from(tab.document.revision())
                        .map_err(|_| "Rust document revision exhausted")?,
                    source: tab.source().to_owned(),
                });
            } else {
                sync(&mut language, uri, tab.source())?;
            }
        }
        sync(
            &mut language,
            from,
            self.session.borrow().document().source(),
        )?;
        Ok(rust_drafts)
    }

    pub(crate) fn apply_project_file_move(
        self: &Rc<Self>,
        destination: &str,
        review: &str,
    ) -> Result<FileMovePlan, String> {
        self.ensure_local_file_move()?;
        let plan = self
            .plan_project_file_move(destination, false)?
            .ok_or("Wait for the Rust reference review before applying the move")?;
        if plan.review != review {
            return Err(
                "The file move changed. Review the current source patches before applying it"
                    .into(),
            );
        }
        let mut staged = self.stage_reviewed_sources()?;
        let label = self
            .project
            .borrow_mut()
            .linked_history
            .label("Move XAML file")?;
        let mut participants = Vec::new();
        let mut references = Vec::new();
        for change in &plan.documents {
            let path = file_path(&change.edit.uri).map_err(|error| error.to_string())?;
            if SavedDesignFile::for_path(&path).is_some() {
                references.push(SavedReferenceEdit {
                    uri: change.edit.uri.clone(),
                    before: change.before.clone(),
                    after: change.after.clone(),
                });
                continue;
            }
            let tab = staged.tab(self, &change.edit.uri)?;
            check_saved(&staged.workspace, &change.edit.uri, &tab)?;
            staged
                .tabs
                .insert(change.edit.uri.clone(), patched(tab, change, &label)?);
            participants.push(change.edit.uri.clone());
        }
        let source = staged.tabs[&plan.from].saved.clone();
        staged.prepare(&plan.from, &plan.to, &participants)?;
        let stage_moves = self.stage_source_moves(&plan.from, &plan.to)?;
        let file = staged
            .workspace
            .move_source_file_with_references(&plan.from, &plan.to, &source, references)
            .map_err(|error| error.to_string())?;
        self.project
            .borrow_mut()
            .linked_history
            .moves
            .insert(label, MovedXaml { file, participants });
        self.accept_file_move(staged, &plan.from, &plan.to, &stage_moves)?;
        self.project.borrow_mut().rust_move = None;
        Ok(plan)
    }

    fn cancel_move_resolution(&self, target: &resolution::MoveTarget) {
        let mut project = self.project.borrow_mut();
        if project
            .rust_move
            .as_ref()
            .is_some_and(|review| review.matches(target))
        {
            project.rust_move = None;
        }
    }

    #[cfg(feature = "mcp")]
    pub(crate) fn cancel_project_file_move_review(&self, destination: &str) -> Result<(), String> {
        self.cancel_move_resolution(&self.move_target(destination)?);
        Ok(())
    }

    pub(super) fn step_file_move(
        self: &Rc<Self>,
        label: &str,
        mut moved: MovedXaml,
        redo: bool,
    ) -> Result<(), String> {
        self.ensure_local_file_move()?;
        let (from, to) = if redo {
            (moved.file.from.clone(), moved.file.to.clone())
        } else {
            (moved.file.to.clone(), moved.file.from.clone())
        };
        let mut staged = self.stage_reviewed_sources()?;
        let participants = moved
            .participants
            .iter()
            .map(|uri| {
                if uri == &moved.file.from {
                    from.clone()
                } else {
                    uri.clone()
                }
            })
            .collect::<Vec<_>>();
        for uri in &participants {
            let mut tab = staged
                .tabs
                .get(uri)
                .cloned()
                .or_else(|| self.project.borrow().linked_history.closed(uri).cloned())
                .ok_or("Reopen all files in this move before applying its history")?;
            if tab.history_label(redo) != Some(label) {
                return Err("Finish newer history in the files changed by this move first".into());
            }
            check_saved(&staged.workspace, uri, &tab)?;
            tab.step_history(redo)?;
            staged.tabs.insert(uri.clone(), tab);
        }
        let saved = staged.tabs[&from].saved.clone();
        staged.prepare(&from, &to, &participants)?;
        let stage_moves = self.stage_source_moves(&from, &to)?;
        staged
            .workspace
            .replay_source_move(&mut moved.file, redo, &saved)
            .map_err(|e| e.to_string())?;
        self.project
            .borrow_mut()
            .linked_history
            .moves
            .insert(label.into(), moved);
        self.accept_file_move(staged, &from, &to, &stage_moves)
    }

    pub(super) fn current_xaml_file(&self) -> Result<String, String> {
        let session = self.session.borrow();
        let uri = session.document_uri();
        if session.uses_rust_source()
            || !uri.starts_with("file://")
            || !uri.to_ascii_lowercase().ends_with(".xaml")
        {
            return Err("Choose a project XAML file".into());
        }
        document_identity(uri)
    }

    fn ensure_local_file_move(&self) -> Result<(), String> {
        let session = self.session.borrow();
        session.ensure_writable().map_err(|e| e.to_string())?;
        if self.pending_source.get() || session.collaboration_actor().is_some() {
            return Err(
                "Apply the current draft and open the local project before moving its file".into(),
            );
        }
        if session.document().revision() >= i64::MAX as u64 - 2 {
            return Err("Document revision exhausted".into());
        }
        Ok(())
    }

    fn stage_source_moves(&self, from: &str, to: &str) -> Result<PreviewStageSourceMoves, String> {
        let moves = self
            .preview_stages
            .board
            .borrow()
            .stages()
            .into_iter()
            .map(|stage| stage.configuration.source_uri)
            .filter(|uri| document_identity(uri).is_ok_and(|source| source == from))
            .map(|uri| (uri, to.to_owned()))
            .collect();
        PreviewStageSourceMoves::new(moves)
    }

    fn accept_file_move(
        self: &Rc<Self>,
        staged: ReviewedSources,
        from: &str,
        to: &str,
        stage_moves: &PreviewStageSourceMoves,
    ) -> Result<(), String> {
        let active = if staged.active == from {
            to
        } else {
            &staged.active
        };
        let tab = staged.tabs[active].clone();
        {
            let mut project = self.project.borrow_mut();
            project.workspace = Some(staged.workspace);
            project.tabs = staged.tabs;
            project.linked_history.take_closed(from);
            project.linked_history.take_closed(to);
        }
        *self.try_language_service()? = staged.language;
        self.preview_stages
            .board
            .borrow_mut()
            .retarget_sources(stage_moves);
        self.present_project_tab(active, tab)?;
        self.rebuild_project_tree();
        Ok(())
    }
}

impl ReviewedSources {
    fn prepare(&mut self, from: &str, to: &str, participants: &[String]) -> Result<(), String> {
        if self.tabs.contains_key(to) {
            return Err("The destination is already open".into());
        }
        self.prepare_edits(participants)?;
        self.language
            .rename_uri(from, to)
            .map_err(|e| e.to_string())?;
        let mut tab = self
            .tabs
            .remove(from)
            .ok_or("The moved file is unavailable")?;
        tab.preview_uri = to.into();
        tab.file = None;
        self.tabs.insert(to.into(), tab);
        Ok(())
    }
}
