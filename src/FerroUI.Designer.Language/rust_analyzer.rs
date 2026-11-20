//! A scoped rust-analyzer session for resolving compiler-generated XAML symbols.
//! Source transactions are still planned and validated by the language service.
mod session;
#[cfg(test)]
mod tests;
mod workspace_edit;

use crate::{
    FileMoveDraft, FileMoveRustResolution, GeneratedRustRename, LanguageError, Result, require,
};
use ferroui_designer_project::{canonical_file_uri, file_path, file_uri};
use serde_json::{Value, json};
use session::Session;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
    process::Command,
    sync::atomic::AtomicBool,
    time::Duration,
};

/// The Cargo workspace and build configuration that published an XAML module.
#[derive(Clone, Debug)]
pub struct RustAnalyzerProject {
    pub root: PathBuf,
    pub manifest: PathBuf,
    pub features: Vec<String>,
    pub release: bool,
}

/// Resolve the authored Rust edits from the declaration in the accepted Cargo
/// output. Call from a worker and submit the result to `complete_file_move`.
pub fn resolve_file_move(
    project: &RustAnalyzerProject,
    renames: &[GeneratedRustRename],
    drafts: &[FileMoveDraft],
    cancelled: &AtomicBool,
) -> Result<Vec<FileMoveRustResolution>> {
    if renames.is_empty() {
        return Ok(Vec::new());
    }
    let root = file_uri(&project.root)?;
    let package = project
        .manifest
        .parent()
        .ok_or_else(|| LanguageError("The Cargo package has no root directory".into()))?;
    let program =
        std::env::var_os("FERROUI_RUST_ANALYZER").unwrap_or_else(|| "rust-analyzer".into());
    let mut command = Command::new(program);
    command
        .current_dir(package)
        .env("CARGO_BUILD_JOBS", "1")
        .env("CARGO_NET_OFFLINE", "true");
    let mut server = Session::spawn(command, cancelled, Duration::from_secs(90))?;
    let initialized = server.request("initialize", json!({
        "processId": std::process::id(),
        "rootUri": root,
        "workspaceFolders": [{"uri": root, "name": project.root.file_name().and_then(|name|name.to_str()).unwrap_or("Cargo workspace")}],
        "capabilities": {
            "general": {"positionEncodings": ["utf-16"]},
            "workspace": {"workspaceEdit": {"documentChanges": true}},
            "experimental": {"serverStatusNotification": true}
        },
        "initializationOptions": project.options()
    }))?;
    require(
        initialized
            .pointer("/capabilities/positionEncoding")
            .and_then(Value::as_str)
            .unwrap_or("utf-16")
            == "utf-16",
        "The installed rust-analyzer did not select UTF-16 positions",
    )?;
    server.notify("initialized", json!({}))?;

    let mut versions = BTreeMap::new();
    for draft in drafts {
        let uri = canonical_file_uri(&draft.uri)?;
        if file_path(&uri)?.starts_with(&project.root) {
            server.notify(
                "textDocument/didOpen",
                json!({"textDocument": {
                    "uri": uri, "languageId": "rust", "version": 1, "text": draft.source,
                }}),
            )?;
            // Use LSP version 1 for each new session. Studio's own revisions are
            // wider and are checked again when the plan is completed or applied.
            versions.insert(uri, draft.version);
        }
    }
    server.wait_until_ready()?;
    let generated = renames
        .iter()
        .map(|rename| {
            file_path(&rename.uri).map(|path| path.parent().unwrap_or(&path).to_path_buf())
        })
        .collect::<std::result::Result<BTreeSet<_>, _>>()?;
    let mut result = Vec::with_capacity(renames.len());
    for rename in renames {
        let response = server
            .request(
                "textDocument/rename",
                json!({
                    "textDocument": {"uri": rename.uri},
                    "position": rename.position,
                    "newName": rename.new_name
                }),
            )
            .map_err(|error| {
                LanguageError(format!(
                    "Resolve {} in the selected Cargo build. {error}",
                    rename.old_name
                ))
            })?;
        result.push(workspace_edit::resolve(
            rename, response, &generated, &versions,
        )?);
    }
    server.finish();
    Ok(result)
}

impl RustAnalyzerProject {
    fn options(&self) -> Value {
        // Use Cargo's existing target location and inherited target selection.
        // The wrapper confines this check to the artifacts rust-analyzer needs.
        let mut check = vec![
            "cargo",
            "check",
            "--workspace",
            "--locked",
            "--offline",
            "--message-format=json",
        ];
        if self.release {
            check.push("--release");
        }
        let features = self.features.join(",");
        if !features.is_empty() {
            check.extend(["--features", &features]);
        }
        json!({
            "linkedProjects": [self.manifest],
            "checkOnSave": false,
            "numThreads": 1,
            "cachePriming": {"enable": false},
            "cargo": {
                "features": self.features,
                "extraArgs": ["--locked", "--offline"],
                "buildScripts": {"enable": true, "rebuildOnSave": false, "useRustcWrapper": true, "overrideCommand": check}
            }
        })
    }
}
