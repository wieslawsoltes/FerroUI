//! Project discovery, transactional document buffers and process tasks without a UI dependency.
mod discovery;
mod dictionary_file;
mod markup;
mod preview_sources;
mod compiled_resources;
pub use compiled_resources::{compiled_resource_key, CompiledResources};
mod tasks;
mod toolbox;
mod uri;
mod workspace;

pub use discovery::{CargoTarget, DocumentEntry, DocumentKind, ProjectDescriptor, Solution, workspace_documents};
pub use dictionary_file::{DictionaryDestination, dictionary_destination, suggested_dictionary_name};
use std::fmt;
pub use markup::{BuildMetadata, CompiledXamlOutput, MarkupAnimationProperty, MarkupAssemblySource, MarkupCatalog, MarkupControl, MarkupEventSignature, MarkupSource};
pub mod rsx_mapping;
pub use tasks::{BuildTask, OutputStream, TaskHandle, TaskKind, TaskOutput, TaskStatus};
pub use toolbox::{ToolboxSnippet, toolbox_snippets};
pub use uri::{canonical_file_uri, file_path, file_uri, relative_asset_path, relative_asset_uri, resolve_uri, same_loader_address};
mod file_move;
mod process;
pub use process::{spawn_project_process, stop_project_process};
pub use tasks::CargoBuildConfiguration;
pub use file_move::{ProjectFileMove, SavedDesignFile, XamlFileMove, relative_file_path, relative_file_reference};
mod rust_xaml;
mod rust_imports;
mod rust_modules;
pub use rust_imports::visit_rust_imports;
pub use rust_modules::{RustModuleProject, RustModuleSource, RustModuleSources};
pub use rust_xaml::{
    check_rust_rename_tokens, RustPathAnchor, RustXamlFileInput, RustXamlPath, RustXamlReferences,
    RustFilePath, RustIncludeSyntax, RustXamlRegistration, rust_file_includes, rust_string_like,
};
pub use preview_sources::{
    hosted_preview_sources, local_preview_sources, local_preview_sources_with_resources, LocalPreviewSources,
    PreviewInclude, PreviewIncludes, PreviewSourceBuffer, xaml_source_revision,
};
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
