//! Project discovery, transactional document buffers and process tasks without a UI dependency.
mod discovery;
mod dictionary_file;
mod markup;
mod preview_sources;
mod tasks;
mod toolbox;
mod uri;
mod workspace;

pub use discovery::{CargoTarget, DocumentEntry, DocumentKind, ProjectDescriptor, Solution, workspace_documents};
pub use dictionary_file::{DictionaryDestination, dictionary_destination, suggested_dictionary_name};
use std::fmt;
pub use markup::{BuildMetadata, CompiledXamlOutput, MarkupAnimationProperty, MarkupCatalog, MarkupControl, MarkupEventSignature};
pub use tasks::{BuildTask, OutputStream, TaskHandle, TaskKind, TaskOutput, TaskStatus};
pub use toolbox::{ToolboxSnippet, toolbox_snippets};
pub use uri::{canonical_file_uri, file_path, file_uri, relative_asset_path, relative_asset_uri, resolve_uri};
mod file_move;
pub use file_move::{SavedDesignFile, XamlFileMove, relative_file_path, relative_file_reference};
mod rust_xaml;
pub use rust_xaml::{RustPathAnchor, RustXamlPath, RustXamlReferences, RustXamlRegistration, check_rust_rename_tokens};
pub use preview_sources::{LocalPreviewSources, PreviewSourceBuffer, local_preview_sources};
pub use workspace::{Buffer, BufferUpdate, CreatedSourceFile, MovedSourceFile, ProjectWorkspace, SavePolicy, SavedReferenceEdit};

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
