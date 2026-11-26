use super::RustModuleSources;
use crate::{PreviewSourceBuffer, ProjectDescriptor, file_path, uri::canonical_path};
use serde::Deserialize;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// The authored crate roots that Cargo reported for the selected package.
#[derive(Clone, Debug, Deserialize)]
pub struct RustModuleProject {
    pub manifest: PathBuf,
    pub targets: Vec<PathBuf>,
    /// Enabled features from the selected Cargo compiler artifact.
    #[serde(default)]
    pub features: Option<Vec<String>>,
}

impl From<&ProjectDescriptor> for RustModuleProject {
    fn from(project: &ProjectDescriptor) -> Self {
        Self {
            manifest: project.manifest_path.clone(),
            targets: project
                .targets
                .iter()
                .map(|target| target.source.clone())
                .collect(),
            features: None,
        }
    }
}

impl RustModuleProject {
    /// Resolve only when the editable source belongs to one actual target tree.
    /// Different crate roots can give the same file different `crate` bindings.
    pub fn capture(
        &self,
        workspace: &Path,
        active_uri: &str,
        source: &str,
        drafts: &[PreviewSourceBuffer],
    ) -> Option<RustModuleSources> {
        if !self.manifest.is_absolute() || self.manifest.file_name()? != "Cargo.toml" {
            return None;
        }
        let manifest = self.manifest.canonicalize().ok()?;
        let package = manifest.parent()?;
        let workspace = workspace.canonicalize().ok()?;
        let active = canonical_path(&file_path(active_uri).ok()?).ok()?;
        if !package.starts_with(&workspace)
            || !active.starts_with(package)
            || active
                .ancestors()
                .skip(1)
                .take_while(|dir| *dir != package)
                .any(|dir| dir.join("Cargo.toml").is_file())
        {
            return None;
        }

        let mut visited = BTreeSet::new();
        let mut chosen = None;
        for target in &self.targets {
            if !target.is_absolute() {
                continue;
            }
            let Ok(identity) = canonical_path(target) else {
                continue;
            };
            if !visited.insert(identity) {
                continue;
            }
            if let Some(sources) = RustModuleSources::capture_with_features(
                package, target, active_uri, source, drafts, self.features.as_deref(),
            )
            {
                if chosen.is_some() {
                    return None;
                }
                chosen = Some(sources);
            }
        }
        chosen
    }
}
