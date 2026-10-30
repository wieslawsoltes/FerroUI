//! Project discovery, transactional document buffers and process tasks without a UI dependency.
mod discovery;
mod tasks;
mod uri;
mod workspace;

pub use discovery::{CargoTarget, DocumentEntry, DocumentKind, ProjectDescriptor, Solution};
use std::fmt;
pub use tasks::{BuildTask, OutputStream, TaskHandle, TaskKind, TaskOutput, TaskStatus};
pub use uri::{canonical_file_uri, file_path, file_uri, resolve_uri};
pub use workspace::{Buffer, BufferUpdate, ProjectWorkspace, SavePolicy};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectError(pub String);
pub type Result<T> = std::result::Result<T, ProjectError>;
impl fmt::Display for ProjectError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for ProjectError {}
impl From<std::io::Error> for ProjectError {
    fn from(error: std::io::Error) -> Self {
        Self(error.to_string())
    }
}
impl From<serde_json::Error> for ProjectError {
    fn from(error: serde_json::Error) -> Self {
        Self(error.to_string())
    }
}
impl From<ferroui_designer_core::DesignerError> for ProjectError {
    fn from(error: ferroui_designer_core::DesignerError) -> Self {
        Self(error.to_string())
    }
}
pub(crate) fn require(value: bool, message: impl Into<String>) -> Result<()> {
    if value {
        Ok(())
    } else {
        Err(ProjectError(message.into()))
    }
}

#[cfg(test)]
mod tests;
