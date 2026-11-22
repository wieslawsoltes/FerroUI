//! Local design-system mappings and reviewed, exclusive asset transfers.
mod assets;
mod paths;

use crate::{require, ProjectError, Result};
pub use assets::{Asset, AssetCopy};
use paths::{confined_path, existing_file, read_bounded, relative_path};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    hash::{DefaultHasher, Hash, Hasher},
    path::{Path, PathBuf},
};

const MAX_MANIFEST_BYTES: u64 = 1024 * 1024;
const MAX_MAPPINGS: usize = 256;

/// Explicit UI selection grants access to the chosen export and its sibling
/// files. Workspace callers, including MCP, stay within the open project.
#[derive(Clone, Copy)]
pub enum Access {
    Workspace,
    SelectedFile,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub version: u32,
    pub mappings: Vec<Mapping>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Mapping {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub target: Target,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum Target {
    Control {
        toolbox: String,
    },
    Definition {
        path: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        key: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        scope: Option<String>,
    },
    Asset {
        path: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        transfer: Option<Transfer>,
    },
    PublishedAsset {
        address: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        export: Option<String>,
    },
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "direction", rename_all = "camelCase", deny_unknown_fields)]
pub enum Transfer {
    Import { file: String },
    Export { file: String },
}

pub struct ManifestFile {
    workspace: PathBuf,
    path: PathBuf,
    directory: PathBuf,
    fingerprint: String,
    pub manifest: Manifest,
}

impl ManifestFile {
    pub fn in_workspace(workspace: &Path, relative: &str) -> Result<PathBuf> {
        let root = workspace.canonicalize()?;
        confined_path(&root, relative_path(relative)?)
    }

    pub fn read(workspace: &Path, path: &Path, access: Access) -> Result<Self> {
        let workspace = workspace.canonicalize()?;
        require(
            workspace.is_dir(),
            "Open a local workspace for design handoff",
        )?;
        let path = existing_file(path)?;
        require(
            matches!(access, Access::SelectedFile) || path.starts_with(&workspace),
            "Choose a handoff file in the open workspace",
        )?;
        let directory = path
            .parent()
            .ok_or_else(|| ProjectError("The handoff file has no parent directory".into()))?
            .to_owned();
        let bytes = read_bounded(&path, MAX_MANIFEST_BYTES)?;
        let manifest: Manifest =
            serde_json::from_slice(bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(&bytes))?;
        manifest.validate()?;
        Ok(Self {
            workspace,
            path,
            directory,
            fingerprint: fingerprint(&bytes),
            manifest,
        })
    }

    pub fn workspace(&self) -> &Path {
        &self.workspace
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn directory(&self) -> &Path {
        &self.directory
    }

    pub fn fingerprint(&self) -> &str {
        &self.fingerprint
    }

    fn identity(&self) -> String {
        fingerprint(&(&self.workspace, &self.path, &self.fingerprint))
    }

    pub fn project_path(&self, name: &str) -> Result<PathBuf> {
        confined_path(&self.workspace, relative_path(name)?)
    }

    pub fn project_text(&self, name: &str) -> Result<String> {
        let file = self.project_path(name)?;
        let bytes = read_bounded(&file, MAX_MANIFEST_BYTES)?;
        String::from_utf8(bytes).map_err(|error| ProjectError(error.to_string()))
    }

    /// Recheck the reviewed manifest before publishing a transfer.
    pub fn verify(&self) -> Result<()> {
        require(
            existing_file(&self.path)? == self.path,
            "The handoff file was redirected",
        )?;
        let current = read_bounded(&self.path, MAX_MANIFEST_BYTES)?;
        require(
            fingerprint(&current) == self.fingerprint,
            "The handoff file changed. Refresh the review",
        )
    }
}

impl Manifest {
    fn validate(&self) -> Result<()> {
        require(
            self.version == 1,
            "Unsupported FerroUI handoff file version",
        )?;
        require(
            self.mappings.len() <= MAX_MAPPINGS,
            "A handoff file can contain up to 256 mappings",
        )?;
        let mut identifiers = BTreeSet::new();
        for mapping in &self.mappings {
            require(
                !mapping.id.trim().is_empty() && !mapping.id.chars().any(char::is_control),
                "Every mapping needs an external ID",
            )?;
            require(
                identifiers.insert(&mapping.id),
                format!("Duplicate external ID: {}", mapping.id),
            )?;
        }
        Ok(())
    }
}

/// Process-local review fingerprint. The host re-resolves every action against
/// current bytes and project identity before using a reviewed mapping.
pub fn fingerprint(value: &impl Hash) -> String {
    let mut hash = DefaultHasher::new();
    value.hash(&mut hash);
    format!("{:016x}", hash.finish())
}

#[cfg(test)]
mod tests;
