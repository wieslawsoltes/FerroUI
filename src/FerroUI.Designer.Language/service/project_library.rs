use super::{LanguageService, State};
use crate::{require, Range, Result};
use crate::{LanguageError, LineIndex};
use ferroui_designer_core::Document;
use ferroui_designer_project::{canonical_file_uri, file_path, file_uri};
use ferroui_designer_tools::{ComponentCatalog, ComponentEntry, ComponentKind};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf, MAIN_SEPARATOR};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryDefinition {
    pub kind: ComponentKind,
    pub name: String,
    pub range: Range,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_type: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectLibraryEntry {
    pub folder: String,
    pub path: String,
    pub definition: LibraryDefinition,
}

#[derive(Debug, Serialize)]
pub struct ProjectLibrary {
    pub entries: Vec<ProjectLibraryEntry>,
    pub total: usize,
    pub offset: usize,
}

#[derive(Debug, Deserialize)]
pub struct ProjectLibraryRequest {
    pub folder: String,
    #[serde(default)]
    pub query: String,
    #[serde(default)]
    pub offset: usize,
    pub limit: usize,
}

#[derive(Debug, Deserialize)]
pub struct ProjectLibraryResolution {
    #[serde(flatten)]
    pub entry: ProjectLibraryEntry,
    pub uri: String,
    pub version: i64,
}

impl LanguageService {
    /// Search authoritative indexed snapshots, with open drafts overlaid on disk.
    pub fn project_library(&self, request: &ProjectLibraryRequest) -> Result<ProjectLibrary> {
        require(
            (1..=200).contains(&request.limit),
            "Library page size must be between 1 and 200",
        )?;
        require(
            request.query.len() <= 4096,
            "Library query exceeds 4096 bytes",
        )?;
        let root = self.library_root(&request.folder)?;
        let query = request.query.to_lowercase();
        let terms: Vec<_> = query.split_whitespace().collect();
        let mut page = ProjectLibrary {
            entries: Vec::new(),
            total: 0,
            offset: request.offset,
        };
        for (uri, state) in &self.documents {
            let Some(path) = self.library_path(&root, uri) else {
                continue;
            };
            let file = path.to_lowercase();
            for definition in state.library_definitions() {
                if !definition.matches(&file, &terms) {
                    continue;
                }
                if page.total >= page.offset && page.entries.len() < request.limit {
                    page.entries.push(ProjectLibraryEntry {
                        folder: request.folder.clone(),
                        path: path.clone(),
                        definition: definition.clone(),
                    });
                }
                page.total += 1;
            }
        }
        Ok(page)
    }

    /// Resolve a result after the editor opens its buffer, on the synchronized version.
    pub fn resolve_project_library(&self, request: &ProjectLibraryResolution) -> Result<()> {
        let root = self.library_root(&request.entry.folder)?;
        let uri = self.document_uri(&request.uri)?;
        require(
            self.library_path(&root, &uri).as_deref() == Some(request.entry.path.as_str()),
            "The definition is outside the selected Library folder",
        )?;
        let state = self.state(&uri)?;
        require(
            state.opened && state.version == request.version,
            "The Library source changed before navigation",
        )?;
        require(
            state
                .library_definitions()
                .contains(&request.entry.definition),
            "The Library definition changed. Search again for its current location",
        )
    }

    /// Resolve a project result to a local URI before Studio opens its source tab.
    pub fn project_library_uri(&self, entry: &ProjectLibraryEntry) -> Result<String> {
        let root = self.library_root(&entry.folder)?;
        let path = Path::new(&entry.path);
        require(path.is_relative(), "Use a relative path from the Library")?;
        let uri = canonical_file_uri(&file_uri(root.join(path))?)?;
        require(
            self.library_path(&root, &uri).as_deref() == Some(entry.path.as_str()),
            "The definition is outside the selected Library folder",
        )?;
        require(
            self.state(&uri)?
                .library_definitions()
                .contains(&entry.definition),
            "The Library definition changed. Search again for its current location",
        )?;
        Ok(uri)
    }

    fn library_root(&self, folder: &str) -> Result<PathBuf> {
        let requested = file_path(&canonical_file_uri(folder)?)?;
        require(
            self.workspaces.contains_key(&requested),
            "Choose an indexed workspace folder",
        )?;
        Ok(requested)
    }

    fn library_path(&self, root: &Path, uri: &str) -> Option<String> {
        let path = file_path(uri).ok()?;
        let relative = path.strip_prefix(root).ok()?.to_str()?;
        (self.workspace_root_for_path(&path)? == root)
            .then(|| relative.replace(MAIN_SEPARATOR, "/"))
    }
}

impl State {
    fn library_definitions(&self) -> &[LibraryDefinition] {
        self.library.get_or_init(|| {
            ComponentCatalog::project_definitions(&self.document)
                .unwrap_or_default()
                .into_iter()
                .filter_map(|entry| describe(&self.document, &self.index, entry))
                .collect()
        })
    }
}

/// Resolve an external mapping by exact key and optional authored scope, with
/// the same definition kinds and source ranges as the Project Library.
pub fn exact_library_definition(
    source: &str,
    key: Option<&str>,
    scope: Option<&str>,
) -> Result<LibraryDefinition> {
    let document = Document::new(source).map_err(|error| LanguageError(error.to_string()))?;
    let entries = ComponentCatalog::project_definitions(&document)
        .map_err(|error| LanguageError(error.to_string()))?;
    let mut matches = entries.into_iter().filter(|entry| {
        let same_key = match key {
            Some(key) => entry.key.as_deref() == Some(key),
            None => entry.node == document.ast().root(),
        };
        same_key
            && scope.is_none_or(|name| {
                let ast = document.ast();
                std::iter::successors(ast.node(entry.declaration_owner(&document)), |node| {
                    node.parent.and_then(|parent| ast.node(parent))
                })
                .any(|node| node.xaml_name() == Some(name))
            })
    });
    let entry = matches.next().ok_or_else(|| {
        LanguageError("The mapped definition is absent from this source scope".into())
    })?;
    require(
        matches.next().is_none(),
        "More than one definition matches this key. Specify its named scope",
    )?;
    describe(&document, &LineIndex::new(source), entry)
        .ok_or_else(|| LanguageError("The mapped definition has no source location".into()))
}

fn describe(
    document: &Document,
    index: &LineIndex,
    entry: ComponentEntry,
) -> Option<LibraryDefinition> {
    let node = document.ast().node(entry.node)?;
    Some(LibraryDefinition {
        kind: entry.kind,
        name: entry.display_name,
        range: index.range(node.name_span).ok()?,
        key: entry.key,
        target_type: entry.target_type,
    })
}

impl LibraryDefinition {
    fn matches(&self, file: &str, terms: &[&str]) -> bool {
        if terms.is_empty() {
            return true;
        }
        let kind = format!("{:?}", self.kind).to_ascii_lowercase();
        let name = self.name.to_lowercase();
        let target = self
            .target_type
            .as_deref()
            .unwrap_or_default()
            .to_lowercase();
        terms.iter().all(|term| {
            file.contains(term)
                || name.contains(term)
                || target.contains(term)
                || kind.contains(term)
        })
    }
}
