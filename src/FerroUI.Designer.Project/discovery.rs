use crate::{file_uri, require, ProjectError, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DocumentKind {
    Xaml,
    Rust,
    Manifest,
    Asset,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentEntry {
    pub uri: String,
    pub path: PathBuf,
    pub kind: DocumentKind,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CargoTarget {
    pub name: String,
    pub kind: Vec<String>,
    pub source: PathBuf,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectDescriptor {
    pub id: String,
    pub name: String,
    pub root: PathBuf,
    pub manifest_path: PathBuf,
    pub documents: Vec<DocumentEntry>,
    pub targets: Vec<CargoTarget>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Solution {
    pub format_version: u32,
    pub root: PathBuf,
    pub projects: Vec<ProjectDescriptor>,
}
impl Solution {
    /// Cargo is used as the manifest authority, including workspace membership and target paths.
    /// --locked/--offline prevent discovery from changing the dependency graph or downloading code.
    pub fn discover(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().canonicalize()?;
        let directory = if path.is_file() {
            path.parent().unwrap()
        } else {
            &path
        };
        let manifest = directory
            .ancestors()
            .map(|p| p.join("Cargo.toml"))
            .find(|p| p.is_file());
        if let Some(manifest) = manifest {
            let output = Command::new("cargo")
                .args([
                    "metadata",
                    "--locked",
                    "--offline",
                    "--no-deps",
                    "--format-version",
                    "1",
                    "--manifest-path",
                ])
                .arg(&manifest)
                .output()?;
            require(
                output.status.success(),
                format!(
                    "Cargo discovery failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                ),
            )?;
            Self::from_cargo_metadata(&serde_json::from_slice(&output.stdout)?)
        } else {
            let root = directory.to_path_buf();
            let name = root
                .file_name()
                .and_then(|x| x.to_str())
                .unwrap_or("Project")
                .to_owned();
            Ok(Self {
                format_version: 1,
                root: root.clone(),
                projects: vec![ProjectDescriptor {
                    id: file_uri(&root)?,
                    name,
                    manifest_path: root.join("Cargo.toml"),
                    documents: scan(&root)?,
                    targets: Vec::new(),
                    root,
                }],
            })
        }
    }
    pub fn from_cargo_metadata(metadata: &serde_json::Value) -> Result<Self> {
        let root = PathBuf::from(
            metadata["workspace_root"]
                .as_str()
                .ok_or_else(|| ProjectError("Cargo metadata has no workspace root".into()))?,
        )
        .canonicalize()?;
        let members: BTreeSet<_> = metadata["workspace_members"]
            .as_array()
            .ok_or_else(|| ProjectError("Cargo metadata has no members".into()))?
            .iter()
            .filter_map(|v| v.as_str())
            .collect();
        let mut projects = Vec::new();
        for package in metadata["packages"]
            .as_array()
            .ok_or_else(|| ProjectError("Cargo metadata has no packages".into()))?
        {
            let id = package["id"]
                .as_str()
                .ok_or_else(|| ProjectError("Package ID missing".into()))?;
            if !members.contains(id) {
                continue;
            }
            let manifest_path = PathBuf::from(
                package["manifest_path"]
                    .as_str()
                    .ok_or_else(|| ProjectError("Manifest path missing".into()))?,
            )
            .canonicalize()?;
            let package_root = manifest_path.parent().unwrap().to_path_buf();
            let targets = package["targets"]
                .as_array()
                .into_iter()
                .flatten()
                .map(|target| {
                    Ok(CargoTarget {
                        name: target["name"]
                            .as_str()
                            .ok_or_else(|| ProjectError("Target name missing".into()))?
                            .into(),
                        kind: target["kind"]
                            .as_array()
                            .into_iter()
                            .flatten()
                            .filter_map(|v| v.as_str().map(str::to_owned))
                            .collect(),
                        source: PathBuf::from(
                            target["src_path"]
                                .as_str()
                                .ok_or_else(|| ProjectError("Target source missing".into()))?,
                        ),
                    })
                })
                .collect::<Result<_>>()?;
            projects.push(ProjectDescriptor {
                id: id.into(),
                name: package["name"]
                    .as_str()
                    .ok_or_else(|| ProjectError("Package name missing".into()))?
                    .into(),
                root: package_root.clone(),
                manifest_path,
                documents: scan(&package_root)?,
                targets,
            });
        }
        projects.sort_by(|a, b| a.name.cmp(&b.name));
        require(
            !projects.is_empty(),
            "Solution contains no workspace projects",
        )?;
        Ok(Self {
            format_version: 1,
            root,
            projects,
        })
    }
    pub fn refresh(&mut self) -> Result<()> {
        let documents = self
            .projects
            .iter()
            .map(|p| scan(&p.root))
            .collect::<Result<Vec<_>>>()?;
        for (project, entries) in self.projects.iter_mut().zip(documents) {
            project.documents = entries;
        }
        Ok(())
    }
    pub fn save_descriptor(&self, path: impl AsRef<Path>) -> Result<()> {
        crate::workspace::atomic_write(path.as_ref(), &serde_json::to_string_pretty(self)?)
    }
    pub fn load_descriptor(path: impl AsRef<Path>) -> Result<Self> {
        let value: Self = serde_json::from_slice(&std::fs::read(path)?)?;
        require(
            value.format_version == 1,
            "Unsupported solution descriptor version",
        )?;
        require(
            value.root.is_absolute() && value.root.is_dir(),
            "Solution root must be an existing absolute directory",
        )?;
        for project in &value.projects {
            require(
                project.root.is_absolute() && project.root.is_dir(),
                "Project root must be an existing absolute directory",
            )?;
            require(
                project.documents.iter().all(|entry| {
                    entry.path.starts_with(&project.root)
                        && file_uri(&entry.path).ok().as_ref() == Some(&entry.uri)
                }),
                "Invalid project document path or URI",
            )?;
        }
        Ok(value)
    }
}
fn scan(root: &Path) -> Result<Vec<DocumentEntry>> {
    let mut pending = vec![(root.to_path_buf(), 0usize)];
    let mut entries = Vec::new();
    while let Some((directory, depth)) = pending.pop() {
        require(depth < 128, "Project tree exceeds 128 directory levels")?;
        for entry in std::fs::read_dir(directory)? {
            let entry = entry?;
            let ty = entry.file_type()?;
            let path = entry.path();
            if ty.is_symlink() {
                continue;
            }
            if ty.is_dir() {
                if !matches!(
                    entry.file_name().to_str(),
                    Some("target" | ".git" | "node_modules" | ".idea" | ".vs")
                ) {
                    pending.push((path, depth + 1));
                }
            } else if ty.is_file() {
                require(
                    entries.len() < 100_000,
                    "Project contains more than 100000 files",
                )?;
                let kind = match path
                    .extension()
                    .and_then(|s| s.to_str())
                    .map(str::to_ascii_lowercase)
                    .as_deref()
                {
                    Some("xaml") => DocumentKind::Xaml,
                    Some("rs") => DocumentKind::Rust,
                    Some("toml") => DocumentKind::Manifest,
                    _ => DocumentKind::Asset,
                };
                entries.push(DocumentEntry {
                    uri: file_uri(&path)?,
                    path,
                    kind,
                });
            }
        }
    }
    entries.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(entries)
}
