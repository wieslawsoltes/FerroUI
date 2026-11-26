//! A cancellable background Rust resolution retained by the source review.
use super::DesignerWorkspace;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc::{Receiver, TryRecvError},
    Arc,
};

#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
enum State<R> {
    Running(Receiver<Result<R, String>>),
    Ready(R),
}

pub(super) struct RustReview<T, R> {
    target: T,
    base: String,
    cancel: Arc<AtomicBool>,
    state: State<R>,
}

impl<T: PartialEq, R: Send + 'static> RustReview<T, R> {
    #[cfg(any(test, not(target_arch = "wasm32")))]
    pub(super) fn pending(
        target: T,
        base: String,
    ) -> (Self, std::sync::mpsc::Sender<Result<R, String>>) {
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
    pub(super) fn spawn(
        target: T,
        base: String,
        name: &str,
        resolve: impl FnOnce(&AtomicBool) -> Result<R, String> + Send + 'static,
    ) -> Result<Self, String> {
        let (review, sender) = Self::pending(target, base);
        let cancel = review.cancel.clone();
        std::thread::Builder::new()
            .name(name.into())
            .spawn(move || {
                let _ = sender.send(resolve(&cancel));
            })
            .map_err(|error| error.to_string())?;
        Ok(review)
    }

    pub(super) fn matches(&self, target: &T) -> bool {
        &self.target == target
    }

    pub(super) fn poll(&mut self) -> Result<Option<(&str, &R)>, String> {
        if let State::Running(receiver) = &self.state {
            match receiver.try_recv() {
                Ok(result) => self.state = State::Ready(result?),
                Err(TryRecvError::Empty) => return Ok(None),
                Err(TryRecvError::Disconnected) => {
                    return Err("The Rust reference review stopped".into())
                }
            }
        }
        match &self.state {
            State::Ready(resolution) => Ok(Some((&self.base, resolution))),
            State::Running(_) => unreachable!("running jobs return pending"),
        }
    }
}

impl<T, R> Drop for RustReview<T, R> {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Release);
    }
}

impl DesignerWorkspace {
    #[cfg(not(target_arch = "wasm32"))]
    pub(super) fn rust_analysis_project(
        &self,
        uri: &str,
    ) -> Result<ferroui_designer_language::rust_analyzer::RustAnalyzerProject, String> {
        let source = ferroui_designer_project::file_path(uri).map_err(|error| error.to_string())?;
        let package = source
            .ancestors()
            .skip(1)
            .find(|directory| directory.join("Cargo.toml").is_file())
            .ok_or("Open the Cargo package that owns the Rust declaration")?;
        let root = self
            .project
            .borrow()
            .workspace
            .as_ref()
            .map(|project| project.solution.root.clone())
            .ok_or("Open the owning project first")?;
        let cargo = self.cargo_analysis_configuration(package);
        Ok(
            ferroui_designer_language::rust_analyzer::RustAnalyzerProject {
                root,
                manifest: package.join("Cargo.toml"),
                features: cargo.features,
                release: cargo.release,
            },
        )
    }
}
