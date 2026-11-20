//! Reuse a reviewed Rust resolution across the flyout, MCP and the apply check.
use super::DesignerWorkspace;
use ferroui_designer_language::{FileMoveDraft, FileMovePlan, FileMoveRustResolution};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
    mpsc::{Receiver, TryRecvError},
};

#[derive(PartialEq, Eq)]
pub(super) struct MoveTarget {
    pub(super) from: String,
    pub(super) to: String,
    pub(super) revision: u64,
}

#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
enum State {
    Running(Receiver<RustResolution>),
    Ready(Vec<FileMoveRustResolution>),
}

type RustResolution = Result<Vec<FileMoveRustResolution>, String>;

pub(in crate::workspace) struct RustMoveReview {
    target: MoveTarget,
    base: String,
    cancel: Arc<AtomicBool>,
    state: State,
}

impl RustMoveReview {
    #[cfg(any(test, not(target_arch = "wasm32")))]
    pub(super) fn pending(
        target: MoveTarget,
        base: String,
    ) -> (Self, std::sync::mpsc::Sender<RustResolution>) {
        let (sender, receiver) = std::sync::mpsc::channel();
        (
            Self {
                target,
                base,
                cancel: Arc::new(AtomicBool::new(false)),
                state: State::Running(receiver),
            },
            sender,
        )
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(super) fn start(
        workspace: &DesignerWorkspace,
        target: MoveTarget,
        plan: FileMovePlan,
        drafts: Vec<FileMoveDraft>,
    ) -> Result<Self, String> {
        use ferroui_designer_language::rust_analyzer::{RustAnalyzerProject, resolve_file_move};
        let from =
            ferroui_designer_project::file_path(&plan.from).map_err(|error| error.to_string())?;
        let package = from
            .ancestors()
            .skip(1)
            .find(|directory| directory.join("Cargo.toml").is_file())
            .ok_or("Open the Cargo package that generated the XAML module")?;
        let root = workspace
            .project
            .borrow()
            .workspace
            .as_ref()
            .map(|project| project.solution.root.clone())
            .ok_or("Open the owning project first")?;
        let cargo = workspace.cargo_analysis_configuration(package);
        let project = RustAnalyzerProject {
            root,
            manifest: package.join("Cargo.toml"),
            features: cargo.features,
            release: cargo.release,
        };
        let (review, sender) = Self::pending(target, plan.review);
        let cancelled = review.cancel.clone();
        std::thread::Builder::new()
            .name("FerroUI Rust move review".into())
            .spawn(move || {
                let result = resolve_file_move(&project, &plan.generated_rust, &drafts, &cancelled)
                    .map_err(|error| error.to_string());
                let _ = sender.send(result);
            })
            .map_err(|error| error.to_string())?;
        Ok(review)
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

    pub(super) fn matches(&self, target: &MoveTarget) -> bool {
        &self.target == target
    }
    pub(super) fn poll(&mut self) -> Result<Option<(&str, &[FileMoveRustResolution])>, String> {
        if let State::Running(receiver) = &self.state {
            match receiver.try_recv() {
                Ok(result) => self.state = State::Ready(result?),
                Err(TryRecvError::Empty) => return Ok(None),
                Err(TryRecvError::Disconnected) => {
                    return Err("The Rust reference review stopped".into());
                }
            }
        }
        match &self.state {
            State::Ready(resolutions) => Ok(Some((&self.base, resolutions))),
            State::Running(_) => unreachable!("running jobs return pending"),
        }
    }
}

impl Drop for RustMoveReview {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Release);
    }
}
