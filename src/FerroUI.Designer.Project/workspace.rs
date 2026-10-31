use crate::{
    file_path, file_uri, require, DocumentEntry, DocumentKind, ProjectError, Result, Solution,
};
use ferroui_designer_core::Document;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

mod storage;
use storage::atomic_create;
pub(crate) use storage::atomic_write;

#[derive(Clone)]
pub struct Buffer {
    uri: String,
    path: Option<PathBuf>,
    document: Document,
    saved_source: Option<String>,
    version: u64,
}
impl Buffer {
    pub fn uri(&self) -> &str {
        &self.uri
    }
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }
    pub fn document(&self) -> &Document {
        &self.document
    }
    pub fn source(&self) -> &str {
        self.document.source()
    }
    pub fn saved_source(&self) -> Option<&str> {
        self.saved_source.as_deref()
    }
    pub fn version(&self) -> u64 {
        self.version
    }
    pub fn is_dirty(&self) -> bool {
        self.saved_source.as_deref() != Some(self.source())
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SavePolicy {
    CheckExternalChanges,
    Overwrite,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BufferUpdate {
    pub uri: String,
    pub expected_version: u64,
    pub source: String,
}
#[derive(Clone)]
pub struct ProjectWorkspace {
    pub solution: Solution,
    buffers: BTreeMap<String, Buffer>,
    active: Option<String>,
    resource_paths: BTreeMap<String, PathBuf>,
}
impl ProjectWorkspace {
    pub fn new(solution: Solution) -> Self {
        Self {
            solution,
            buffers: BTreeMap::new(),
            active: None,
            resource_paths: BTreeMap::new(),
        }
    }
    pub fn buffers(&self) -> impl Iterator<Item = &Buffer> {
        self.buffers.values()
    }
    pub fn buffer(&self, uri: &str) -> Option<&Buffer> {
        self.buffers
            .get(uri)
            .or_else(|| self.buffers.get(&self.buffer_uri(uri).ok()?))
    }
    pub fn active_uri(&self) -> Option<&str> {
        self.active.as_deref()
    }
    pub fn activate(&mut self, uri: &str) -> Result<()> {
        let uri = self.buffer_uri(uri)?;
        require(self.buffers.contains_key(&uri), "Document is not open")?;
        self.active = Some(uri);
        Ok(())
    }
    pub fn open(&mut self, uri: &str) -> Result<&Buffer> {
        let path = self.resolve_path(uri)?;
        let uri = file_uri(&path)?;
        if !self.buffers.contains_key(&uri) {
            require(
                std::fs::metadata(&path)?.len() <= 16 * 1024 * 1024,
                "Document exceeds 16 MiB",
            )?;
            let source = std::fs::read_to_string(&path)?;
            self.buffers.insert(
                uri.clone(),
                Buffer {
                    uri: uri.clone(),
                    path: Some(path),
                    document: source_document(&source)?,
                    saved_source: Some(source),
                    version: 0,
                },
            );
        }
        self.active = Some(uri.clone());
        Ok(&self.buffers[&uri])
    }
    pub fn create(&mut self, uri: impl Into<String>, source: impl Into<String>) -> Result<&Buffer> {
        let mut uri = uri.into();
        require(
            uri.starts_with("untitled:") || uri.starts_with("file://"),
            "New documents need file or untitled URIs",
        )?;
        let path = if uri.starts_with("file://") {
            let p = self.checked_path(&file_path(&uri)?)?;
            require(!p.exists(), "File already exists; open it instead")?;
            uri = file_uri(&p)?;
            Some(p)
        } else {
            None
        };
        require(!self.buffers.contains_key(&uri), "Document is already open")?;
        let source = source.into();
        let document = source_document(&source)?;
        self.buffers.insert(
            uri.clone(),
            Buffer {
                uri: uri.clone(),
                path,
                document,
                saved_source: None,
                version: 0,
            },
        );
        self.active = Some(uri.clone());
        Ok(&self.buffers[&uri])
    }
    pub fn update_source(
        &mut self,
        uri: &str,
        expected_version: u64,
        source: impl Into<String>,
    ) -> Result<()> {
        self.apply_updates(vec![BufferUpdate {
            uri: uri.into(),
            expected_version,
            source: source.into(),
        }])
    }
    /// Validate all versions and candidates before committing any buffer in a refactoring.
    pub fn apply_updates(&mut self, updates: Vec<BufferUpdate>) -> Result<()> {
        let mut seen = BTreeSet::new();
        let mut staged = Vec::new();
        for update in updates {
            let uri = self.buffer_uri(&update.uri)?;
            require(
                seen.insert(uri.clone()),
                "Duplicate document in workspace edit",
            )?;
            require(
                update.source.len() <= 16 * 1024 * 1024,
                "Document exceeds 16 MiB",
            )?;
            let mut buffer = self
                .buffers
                .get(&uri)
                .ok_or_else(|| ProjectError("Document is not open".into()))?
                .clone();
            require(
                buffer.version == update.expected_version,
                "Buffer version changed",
            )?;
            buffer.version = buffer
                .version
                .checked_add(1)
                .ok_or_else(|| ProjectError("Buffer version exhausted".into()))?;
            buffer
                .document
                .update_source("Edit document", update.source);
            staged.push(buffer);
        }
        for buffer in staged {
            self.buffers.insert(buffer.uri.clone(), buffer);
        }
        Ok(())
    }
    /// Preserve the native designer's complete undo/redo history when storing an inactive tab.
    pub fn store_document(
        &mut self,
        uri: &str,
        expected_version: u64,
        document: Document,
    ) -> Result<()> {
        let uri = self.buffer_uri(uri)?;
        let buffer = self
            .buffers
            .get_mut(&uri)
            .ok_or_else(|| ProjectError("Document is not open".into()))?;
        require(buffer.version == expected_version, "Buffer version changed")?;
        let version = buffer
            .version
            .checked_add(1)
            .ok_or_else(|| ProjectError("Buffer version exhausted".into()))?;
        buffer.document = document;
        buffer.version = version;
        Ok(())
    }
    pub fn save(&mut self, uri: &str) -> Result<()> {
        self.save_with_policy(uri, SavePolicy::CheckExternalChanges)
    }
    /// Record a successful host/provider save without IO or resetting tab history. The
    /// saved snapshot may predate current authored text after an asynchronous write.
    pub fn acknowledge_saved(
        &mut self,
        uri: &str,
        expected_version: u64,
        saved_source: impl Into<String>,
    ) -> Result<()> {
        let uri = self.buffer_uri(uri)?;
        let saved_source = saved_source.into();
        require(
            saved_source.len() <= 16 * 1024 * 1024,
            "Document exceeds 16 MiB",
        )?;
        let buffer = self
            .buffers
            .get_mut(&uri)
            .ok_or_else(|| ProjectError("Document is not open".into()))?;
        require(buffer.version == expected_version, "Buffer version changed")?;
        buffer.document.mark_saved_snapshot(saved_source.clone());
        buffer.saved_source = Some(saved_source);
        Ok(())
    }
    pub fn save_with_policy(&mut self, uri: &str, policy: SavePolicy) -> Result<()> {
        let uri = self.buffer_uri(uri)?;
        let buffer = self
            .buffers
            .get(&uri)
            .ok_or_else(|| ProjectError("Document is not open".into()))?;
        let path = self.checked_path(
            buffer
                .path()
                .ok_or_else(|| ProjectError("Use save_as for an untitled document".into()))?,
        )?;
        if policy == SavePolicy::CheckExternalChanges {
            let current = match std::fs::read_to_string(&path) {
                Ok(text) => Some(text),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
                Err(e) => return Err(e.into()),
            };
            require(
                current == buffer.saved_source,
                "File changed outside the designer; reload or explicitly overwrite",
            )?;
        }
        if policy == SavePolicy::CheckExternalChanges && buffer.saved_source.is_none() {
            atomic_create(&path, buffer.source())?;
        } else {
            atomic_write(&path, buffer.source())?;
        }
        let buffer = self.buffers.get_mut(&uri).unwrap();
        buffer.saved_source = Some(buffer.source().into());
        buffer.document.mark_saved();
        Ok(())
    }
    pub fn save_as(&mut self, uri: &str, path: impl AsRef<Path>) -> Result<String> {
        let uri = self.buffer_uri(uri)?;
        let path = self.checked_path(path.as_ref())?;
        let next = file_uri(&path)?;
        require(
            !path.exists() && !self.buffers.contains_key(&next),
            "Save destination already exists",
        )?;
        let mut buffer = self
            .buffers
            .get(&uri)
            .ok_or_else(|| ProjectError("Document is not open".into()))?
            .clone();
        atomic_create(&path, buffer.source())?;
        buffer.uri = next.clone();
        buffer.path = Some(path);
        buffer.saved_source = Some(buffer.source().into());
        buffer.document.mark_saved();
        self.buffers.remove(&uri);
        self.buffers.insert(next.clone(), buffer);
        if self.active.as_deref() == Some(&uri) {
            self.active = Some(next.clone());
        }
        self.add_entry(&next)?;
        Ok(next)
    }
    pub fn close(&mut self, uri: &str, discard: bool) -> Result<()> {
        let uri = self.buffer_uri(uri)?;
        let buffer = self
            .buffers
            .get(&uri)
            .ok_or_else(|| ProjectError("Document is not open".into()))?;
        require(
            discard || !buffer.is_dirty(),
            "Document has unsaved changes",
        )?;
        self.buffers.remove(&uri);
        if self.active.as_deref() == Some(&uri) {
            self.active = self.buffers.keys().next().cloned();
        }
        Ok(())
    }
    pub fn reload(&mut self, uri: &str, discard: bool) -> Result<()> {
        let uri = self.buffer_uri(uri)?;
        let buffer = self
            .buffers
            .get(&uri)
            .ok_or_else(|| ProjectError("Document is not open".into()))?;
        require(
            discard || !buffer.is_dirty(),
            "Document has unsaved changes",
        )?;
        let path = self.checked_path(
            buffer
                .path()
                .ok_or_else(|| ProjectError("Untitled document has no disk source".into()))?,
        )?;
        let source = std::fs::read_to_string(path)?;
        let version = buffer.version();
        self.update_source(&uri, version, source.clone())?;
        let buffer = self.buffers.get_mut(&uri).unwrap();
        buffer.saved_source = Some(source);
        buffer.document.mark_saved();
        Ok(())
    }
    pub fn rename_document(&mut self, uri: &str, new_path: impl AsRef<Path>) -> Result<String> {
        let old = self.resolve_path(uri)?;
        let destination = self.checked_path(new_path.as_ref())?;
        let next = file_uri(&destination)?;
        require(
            !destination.exists() && !self.buffers.contains_key(&next),
            "Rename destination exists",
        )?;
        std::fs::rename(&old, &destination)?;
        let old_uri = file_uri(&old)?;
        if let Some(mut buffer) = self.buffers.remove(&old_uri) {
            buffer.uri = next.clone();
            buffer.path = Some(destination.clone());
            self.buffers.insert(next.clone(), buffer);
        }
        if self.active.as_deref() == Some(&old_uri) {
            self.active = Some(next.clone());
        }
        for project in &mut self.solution.projects {
            project.documents.retain(|entry| entry.uri != old_uri);
        }
        self.add_entry(&next)?;
        Ok(next)
    }
    /// Register a host-defined embedded-resource key without allowing it to bypass root checks.
    pub fn register_resource(
        &mut self,
        uri: impl Into<String>,
        path: impl AsRef<Path>,
    ) -> Result<()> {
        let uri = uri.into();
        require(
            uri.starts_with("resm:") || uri.starts_with("ferres://"),
            "Expected an embedded-resource URI",
        )?;
        let path = self.checked_path(path.as_ref())?;
        self.resource_paths.insert(uri, path);
        Ok(())
    }
    pub fn resolve_path(&self, uri: &str) -> Result<PathBuf> {
        if let Some(path) = self.resource_paths.get(uri) {
            return self.checked_path(path);
        }
        let path = if let Some(rest) = uri.strip_prefix("ferres://") {
            let (assembly, relative) = rest
                .split_once('/')
                .ok_or_else(|| ProjectError("Resource URI needs an assembly and path".into()))?;
            let project = self
                .solution
                .projects
                .iter()
                .find(|p| {
                    p.name == assembly
                        || p.name.replace('-', "_") == assembly
                        || p.targets.iter().any(|t| t.name == assembly)
                })
                .ok_or_else(|| ProjectError("Unknown resource assembly".into()))?;
            let path = self.checked_path(&project.root.join(crate::uri::decode(relative)?))?;
            require(
                path.starts_with(project.root.canonicalize()?),
                "Resource escapes its assembly root",
            )?;
            path
        } else if let Some(rest) = uri.strip_prefix("resm:") {
            let (key, query) = rest.split_once('?').unwrap_or((rest, ""));
            let key = crate::uri::decode(key)?;
            let assembly = query
                .split('&')
                .find_map(|part| part.strip_prefix("assembly="));
            let assembly = assembly.map(crate::uri::decode).transpose()?;
            let mut matches = Vec::new();
            for project in &self.solution.projects {
                let names: Vec<_> = std::iter::once(project.name.clone())
                    .chain(std::iter::once(project.name.replace('-', "_")))
                    .chain(project.targets.iter().map(|target| target.name.clone()))
                    .collect();
                if assembly
                    .as_ref()
                    .is_some_and(|assembly| !names.contains(assembly))
                {
                    continue;
                }
                for entry in &project.documents {
                    let relative = entry
                        .path
                        .strip_prefix(&project.root)
                        .map_err(|e| ProjectError(e.to_string()))?
                        .to_string_lossy()
                        .replace(['/', '\\'], ".");
                    if key == relative
                        || names.iter().any(|name| key == format!("{name}.{relative}"))
                    {
                        matches.push(entry.path.clone());
                    }
                }
            }
            matches.sort();
            matches.dedup();
            require(
                matches.len() == 1,
                "Embedded resource is missing or ambiguous; register its host mapping",
            )?;
            matches.remove(0)
        } else {
            file_path(uri)?
        };
        self.checked_path(&path)
    }
    /// Stored keys are canonical file URIs. Resolve aliases at every public
    /// boundary, not only open(), so callers can keep their original URI.
    fn buffer_uri(&self, uri: &str) -> Result<String> {
        if self.buffers.contains_key(uri) || uri.starts_with("untitled:") {
            Ok(uri.into())
        } else {
            file_uri(self.resolve_path(uri)?)
        }
    }
    fn checked_path(&self, path: &Path) -> Result<PathBuf> {
        let absolute = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.solution.root.join(path)
        };
        let path = crate::uri::canonical_path(&absolute)?;
        require(
            path.starts_with(self.solution.root.canonicalize()?)
                || self.solution.projects.iter().any(|p| {
                    p.root
                        .canonicalize()
                        .is_ok_and(|root| path.starts_with(root))
                }),
            "Path is outside the solution roots",
        )?;
        Ok(path)
    }
    fn add_entry(&mut self, uri: &str) -> Result<()> {
        let path = file_path(uri)?;
        if let Some(project) = self
            .solution
            .projects
            .iter_mut()
            .filter(|p| path.starts_with(&p.root))
            .max_by_key(|p| p.root.components().count())
        {
            let kind = if path
                .extension()
                .is_some_and(|x| x.eq_ignore_ascii_case("xaml"))
            {
                DocumentKind::Xaml
            } else {
                DocumentKind::Asset
            };
            project.documents.push(DocumentEntry {
                uri: uri.into(),
                path,
                kind,
            });
            project.documents.sort_by(|a, b| a.path.cmp(&b.path));
        }
        Ok(())
    }
}
fn source_document(source: &str) -> Result<Document> {
    require(source.len() <= 16 * 1024 * 1024, "Document exceeds 16 MiB")?;
    let mut document = match Document::new(source) {
        Ok(document) => document,
        Err(_) => {
            let mut document = Document::new("<DesignDocument/>")?;
            document.update_source("Open source", source);
            document.set_history_limit(0);
            document.set_history_limit(256);
            document
        }
    };
    document.mark_saved();
    Ok(document)
}
