use crate::project_schema::ProjectSchema;
use ferroui_designer_project::{BuildMetadata, CargoTarget, Solution};
use crate::{
    FormattingOptions, LanguageError, LineIndex, Range, Result, Schema, TextChange, TextEdit,
    TypeInfo, require,
};
use ferroui_designer_core::{Document, MarkupValue, Node, NodeKind, SourceSpan};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};

mod definitions;
mod registered_rename;
mod registered_rsx;
mod registered_types;
mod style_selectors;
mod refactoring;
mod resource_occurrences;
mod source_review;
#[path = "handler_service.rs"]
mod handlers;
#[path = "dictionary_extraction.rs"]
mod extraction;
#[path = "file_move.rs"]
mod file_moves;
#[path = "file_move_literals.rs"]
mod file_move_literals;
pub use file_move_literals::relocate_xaml_fragment;
pub use extraction::{DictionaryExtractionPlan, DictionaryExtractionRequest, DictionaryExtractionTarget, ExtractedDictionary};
pub use file_moves::{FileMoveDocument, FileMoveDraft, FileMovePlan, FileMoveRustResolution, GeneratedRustRename};
pub use handlers::{HandlerDraft, HandlerPlan, HandlerRequest, HandlerTarget};
pub use registered_rename::{RegisteredPropertyPlan, RegisteredPropertyRequest, RegisteredPropertyTarget};
pub use registered_types::{RegisteredTypeTarget, RegisteredTypePlan};
pub use refactoring::resource_rename::{ResourceRenamePlan, ResourceRenameTarget};
pub use source_review::{ReviewedDocument, RustRenameResolution, RustSymbolRename, SourceRenamePlan, SourceReviewRequest};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Location {
    pub uri: String,
    pub range: Range,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Diagnostic {
    pub span: SourceSpan,
    pub range: Range,
    pub severity: u32,
    pub code: String,
    pub message: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Completion {
    pub label: String,
    pub detail: String,
    pub documentation: String,
    pub kind: u32,
    pub span: SourceSpan,
    pub insert_text: String,
    pub text_edit: TextEdit,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Hover {
    pub contents: String,
    pub range: Range,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentSymbol {
    pub name: String,
    pub detail: String,
    pub kind: u32,
    pub range: Range,
    pub selection_range: Range,
    pub children: Vec<DocumentSymbol>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FoldingRange {
    pub start_line: u32,
    pub start_character: u32,
    pub end_line: u32,
    pub end_character: u32,
    pub kind: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SemanticToken {
    pub line: u32,
    pub start: u32,
    pub length: u32,
    pub token_type: u32,
    pub modifiers: u32,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Signature {
    pub label: String,
    pub documentation: String,
    pub parameters: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SignatureHelp {
    pub signatures: Vec<Signature>,
    pub active_signature: u32,
    pub active_parameter: u32,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentEdits {
    pub uri: String,
    pub version: i64,
    pub edits: Vec<TextEdit>,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct WorkspaceEdit {
    pub document_changes: Vec<DocumentEdits>,
}

#[derive(Clone)]
struct State {
    document: Document,
    symbols: Vec<Occurrence>,
    resource_edges: BTreeMap<u64, Vec<ResourceEdge>>,
    index: LineIndex,
    version: i64,
    opened: bool,
}
impl State {
    /// Retain the document snapshot but rebuild semantic indexes from the new
    /// syntax. Cloning the old occurrence/edge collections would only allocate
    /// data that is immediately discarded after every keystroke.
    fn with_text(&self, index: LineIndex, version: i64, label: &str) -> Self {
        let mut document = self.document.clone();
        document.update_source(label, index.source());
        Self {
            symbols: occurrences(&document),
            resource_edges: resource_edges(&document),
            document,
            index,
            version,
            opened: self.opened,
        }
    }
}
#[derive(Clone)]
pub struct LanguageService {
    schema: Schema,
    overrides: Vec<TypeInfo>,
    configured_schema: Option<Schema>,
    package_schemas: BTreeMap<std::path::PathBuf, ProjectSchema>,
    documents: BTreeMap<String, State>,
    workspaces: BTreeMap<std::path::PathBuf, ferroui_designer_project::ProjectWorkspace>,
    declarations: DeclarationIndex,
    disk_revision: i64,
}
type DeclarationIndex = BTreeMap<String, Vec<(String, Occurrence)>>;
#[derive(Clone, Debug, PartialEq, Eq)]
enum SymbolKind {
    Name(u64),
    Resource,
}
#[derive(Clone)]
struct ResourceEdge {
    node: u64,
    source: Option<String>,
    // A theme dictionary is an alternative, not a later merged override. The
    // language service has no active runtime theme and must retain every outcome.
    kind: ResourceEdgeKind,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum ResourceEdgeKind {
    Collection,
    Theme,
    DefaultTheme,
}
#[derive(Clone)]
struct ResourceLookup {
    matches: Vec<(String, Occurrence)>,
    may_be_missing: bool,
    themed: bool,
}
impl ResourceLookup {
    fn missing() -> Self {
        Self {
            matches: Vec::new(),
            may_be_missing: true,
            themed: false,
        }
    }
    fn alternative(&mut self, other: Self) {
        self.may_be_missing |= other.may_be_missing;
        self.themed |= other.themed;
        for candidate in other.matches {
            if !self
                .matches
                .iter()
                .any(|existing| existing.0 == candidate.0 && existing.1.span == candidate.1.span)
            {
                self.matches.push(candidate);
            }
        }
    }
    fn fallback(&mut self, other: Self) {
        if self.may_be_missing {
            self.may_be_missing = false;
            self.alternative(other);
        }
    }
}
#[derive(Clone)]
struct Occurrence {
    name: String,
    span: SourceSpan,
    kind: SymbolKind,
    declaration: bool,
    node: u64,
    spelling: OccurrenceSpelling,
}
#[derive(Clone, Copy)]
enum OccurrenceSpelling {
    Attribute,
    MarkupArgument(Option<char>),
    Text,
    CData,
}
impl LanguageService {
    pub fn new(schema: Schema) -> Self {
        Self {
            schema,
            overrides: Vec::new(),
            configured_schema: None,
            package_schemas: BTreeMap::new(),
            documents: BTreeMap::new(),
            workspaces: BTreeMap::new(),
            declarations: BTreeMap::new(),
            disk_revision: 0,
        }
    }
    pub fn schema(&self) -> &Schema {
        &self.schema
    }
    pub fn schema_mut(&mut self) -> &mut Schema {
        &mut self.schema
    }
    pub fn schema_for(&self, uri: &str) -> &Schema {
        self.project_schema_for(uri).map_or_else(
            || self.configured_schema.as_ref().unwrap_or(&self.schema),
            |(_, project)| &project.effective,
        )
    }
    fn project_schema_for(&self, uri: &str) -> Option<(&std::path::Path, &ProjectSchema)> {
        if self.package_schemas.is_empty() { return None; }
        let path = ferroui_designer_project::file_path(&self.document_uri(uri).ok()?).ok()?;
        let root = path.ancestors().skip(1).find(|root| {
            self.package_schemas.contains_key(*root) || root.join("Cargo.toml").is_file()
        })?;
        self.package_schemas.get_key_value(root).map(|(root, schema)| (root.as_path(), schema))
    }
    fn cargo_package_targets(
        &self,
        root: &std::path::Path,
    ) -> Result<Option<(String, Vec<CargoTarget>)>> {
        if let Some(project) = self
            .workspaces
            .values()
            .flat_map(|workspace| &workspace.solution.projects)
            .find(|project| project.root == root && !project.targets.is_empty())
        {
            return Ok(Some((project.name.clone(), project.targets.clone())));
        }
        // An editor workspace can contain a nested Cargo package that has its own targets.
        Ok(Solution::discover(root)?
            .projects
            .into_iter()
            .find(|project| project.root == root)
            .map(|project| (project.name, project.targets)))
    }
    pub fn set_type_overrides(&mut self, overrides: Vec<TypeInfo>) {
        self.overrides = overrides;
        self.configured_schema = (!self.overrides.is_empty()).then(|| {
            let mut schema = self.schema.clone();
            for info in &self.overrides {
                schema.register(info.clone());
            }
            schema
        });
        for project in self.package_schemas.values_mut() {
            project.refresh(&self.schema, &self.overrides);
        }
    }
    pub fn set_project_metadata(
        &mut self,
        manifest: &std::path::Path,
        directories: &[std::path::PathBuf],
    ) -> Result<()> {
        self.set_build_metadata(&BuildMetadata {
            manifest: manifest.into(),
            directories: directories.to_vec(),
            package: None,
        })
    }

    pub fn set_build_metadata(&mut self, metadata: &BuildMetadata) -> Result<()> {
        let root = metadata.package_root()?;
        self.require_package_root(&root)?;
        self.publish_project_catalog(root, std::sync::Arc::new(metadata.catalog()?));
        Ok(())
    }

    pub fn set_project_catalog(&mut self, root: std::path::PathBuf, catalog: std::sync::Arc<ferroui_designer_project::MarkupCatalog>) -> Result<()> {
        self.require_package_root(&root)?;
        self.publish_project_catalog(root, catalog);
        Ok(())
    }
    fn require_package_root(&self, root: &std::path::Path) -> Result<()> {
        require(
            self.workspaces
                .values()
                .any(|workspace| workspace.solution.owns_package_root(root)),
            "Cargo metadata must belong to an indexed workspace",
        )
    }
    fn publish_project_catalog(&mut self, root: std::path::PathBuf, catalog: std::sync::Arc<ferroui_designer_project::MarkupCatalog>) {
        // The sidecar is optional for type completion. Move review validates it
        // separately when it needs authoritative compiled input positions.
        let resources = catalog.compiled_resources(&root).unwrap_or_default();
        let schema = ProjectSchema::new(&self.schema, &self.overrides, catalog, resources);
        self.package_schemas.insert(root, schema);
    }
    pub fn compiled_resources(&self, owner: &str) -> Option<std::sync::Arc<ferroui_designer_project::CompiledResources>> {
        self.project_schema_for(owner).map(|(_, schema)| schema.resources.clone())
    }
    pub fn document(&self, uri: &str) -> Option<&Document> {
        self.state(uri).ok().map(|s| &s.document)
    }
    pub fn version(&self, uri: &str) -> Option<i64> {
        self.state(uri).ok().map(|s| s.version)
    }
    pub fn index(&self, uri: &str) -> Result<&LineIndex> {
        Ok(&self.state(uri)?.index)
    }
    pub fn uris(&self) -> impl Iterator<Item = &str> {
        self.documents.keys().map(String::as_str)
    }
    pub fn open(
        &mut self,
        uri: impl Into<String>,
        source: impl Into<String>,
        version: i64,
    ) -> Result<()> {
        let uri = uri.into();
        let uri = self.document_uri(&uri)?.into_owned();
        let source = source.into();
        require(source.len() <= 16 * 1024 * 1024, "Document exceeds 16 MiB")?;
        if let Some(old) = self.documents.get(&uri) {
            require(
                !old.opened || version > old.version,
                "Document version must increase",
            )?;
        }
        let mut document = match Document::new(&source) {
            Ok(doc) => doc,
            Err(_) => {
                let mut doc = Document::new("<DesignDocument/>")?;
                doc.update_source("Open source", &source);
                doc.set_history_limit(0);
                doc
            }
        };
        document.set_history_limit(0);
        document.mark_saved();
        self.set_state(
            uri,
            Some(State {
                symbols: occurrences(&document),
                resource_edges: resource_edges(&document),
                document,
                index: LineIndex::new(&source),
                version,
                opened: true,
            }),
        );
        Ok(())
    }
    fn index_disk(&mut self, uri: &str, source: String) -> Result<()> {
        if self.documents.get(uri).is_some_and(|state| {
            !state.opened && state.index.source() == source
        }) {
            return Ok(());
        }
        let version = previous_disk_revision(self.disk_revision)?;
        self.open(uri, source, version)?;
        self.documents.get_mut(uri).expect("indexed snapshot").opened = false;
        self.disk_revision = version;
        Ok(())
    }
    pub fn close(&mut self, uri: &str) -> Result<()> {
        let uri = self.document_uri(uri)?.into_owned();
        let uri = uri.as_str();
        require(self.documents.contains_key(uri), "Document is not indexed")?;
        let project_entry = self
            .workspaces
            .values()
            .flat_map(|workspace| &workspace.solution.projects)
            .flat_map(|project| &project.documents)
            .find(|entry| {
                self.document_uri(&entry.uri)
                    .is_ok_and(|entry_uri| entry_uri == uri)
            })
            .cloned();
        let snapshot = project_entry
            .filter(|entry| entry.path.is_file())
            .map(|entry| {
                require(
                    std::fs::metadata(&entry.path)
                        .map_err(|e| LanguageError(e.to_string()))?
                        .len()
                        <= 16 * 1024 * 1024,
                    "Indexed file exceeds 16 MiB",
                )?;
                std::fs::read_to_string(entry.path).map_err(|e| LanguageError(e.to_string()))
            })
            .transpose()?;
        self.set_state(uri.into(), None);
        if let Some(source) = snapshot {
            self.index_disk(uri, source)?;
        }
        Ok(())
    }
    pub fn replace(&mut self, uri: &str, source: impl Into<String>, version: i64) -> Result<()> {
        self.change(
            uri,
            version,
            &[TextChange {
                range: None,
                range_length: None,
                text: source.into(),
            }],
        )
    }
    pub fn change(&mut self, uri: &str, version: i64, changes: &[TextChange]) -> Result<()> {
        let uri = self.document_uri(uri)?.into_owned();
        let uri = uri.as_str();
        let state = self.state(uri)?;
        require(version > state.version, "Document version must increase")?;
        require(changes.len() <= 4096, "Too many text changes")?;
        let mut index = state.index.clone();
        for change in changes {
            index.apply(change)?;
        }
        let mut state = state.with_text(index, version, "Incremental text edits");
        state.opened = true;
        self.set_state(uri.into(), Some(state));
        Ok(())
    }
    pub fn index_solution(
        &mut self,
        solution: &ferroui_designer_project::Solution,
    ) -> Result<usize> {
        let mut added = 0;
        let mut staged = BTreeMap::new();
        let root = solution
            .root
            .canonicalize()
            .map_err(|error| LanguageError(error.to_string()))?;
        let workspace = ferroui_designer_project::ProjectWorkspace::new(solution.clone());
        for entry in solution
            .projects
            .iter()
            .flat_map(|p| &p.documents)
            .filter(|d| d.kind == ferroui_designer_project::DocumentKind::Xaml)
        {
            let uri = self.document_uri(&entry.uri)?.into_owned();
            if staged.contains_key(&uri) || self.documents.get(&uri).is_some_and(|s| s.opened) {
                continue;
            }
            let path = workspace.resolve_path(&uri)?;
            require(
                std::fs::metadata(&path)
                    .map_err(|e| LanguageError(e.to_string()))?
                    .len()
                    <= 16 * 1024 * 1024,
                "Indexed file exceeds 16 MiB",
            )?;
            let source =
                std::fs::read_to_string(&path).map_err(|e| LanguageError(e.to_string()))?;
            require(
                source.len() <= 16 * 1024 * 1024,
                "Indexed file exceeds 16 MiB",
            )?;
            staged.insert(uri, source);
        }
        for (uri, source) in staged {
            self.index_disk(&uri, source)?;
            added += 1;
        }
        self.workspaces.insert(root, workspace);
        Ok(added)
    }

    /// Refresh one project snapshot without disturbing an editor's unsaved buffer.
    pub fn refresh_project_file(&mut self, uri: &str) -> Result<bool> {
        use ferroui_designer_project::{DocumentEntry, DocumentKind};

        let uri = self.document_uri(uri)?.into_owned();
        let path = ferroui_designer_project::file_path(&uri)?;
        if !path
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("xaml"))
        {
            return Ok(false);
        }
        let contains = |project: &ferroui_designer_project::ProjectDescriptor| {
            path.strip_prefix(&project.root).is_ok_and(|relative| {
                !relative.iter().any(|part| {
                    matches!(
                        part.to_str(),
                        Some(".git" | "target" | "node_modules" | "bin" | "obj")
                    )
                })
            })
        };
        if !self
            .workspaces
            .values()
            .flat_map(|workspace| &workspace.solution.projects)
            .any(contains)
        {
            return Ok(false);
        }

        let opened = self.documents.get(&uri).is_some_and(|state| state.opened);
        let present = path.is_file();
        if !opened {
            if present {
                use std::io::Read;
                const MAX: u64 = 16 * 1024 * 1024;
                let mut source = String::new();
                std::fs::File::open(&path)
                    .and_then(|file| file.take(MAX + 1).read_to_string(&mut source))
                    .map_err(|error| LanguageError(error.to_string()))?;
                require(source.len() as u64 <= MAX, "Indexed file exceeds 16 MiB")?;
                self.index_disk(&uri, source)?;
            } else {
                self.set_state(uri.clone(), None);
            }
        }
        for project in self
            .workspaces
            .values_mut()
            .flat_map(|workspace| &mut workspace.solution.projects)
        {
            if !contains(project) {
                continue;
            }
            project.documents.retain(|entry| entry.path != path);
            if present {
                project.documents.push(DocumentEntry {
                    uri: uri.clone(),
                    path: path.clone(),
                    kind: DocumentKind::Xaml,
                });
            }
        }
        Ok(true)
    }
    /// Remove a workspace's disk snapshots while preserving open buffers and shared roots.
    pub fn remove_solution(&mut self, root: impl AsRef<std::path::Path>) -> usize {
        let root = root
            .as_ref()
            .canonicalize()
            .unwrap_or_else(|_| root.as_ref().to_path_buf());
        let Some(removed) = self.workspaces.remove(&root) else {
            return 0;
        };
        self.package_schemas.retain(|root, _| {
            self.workspaces
                .values()
                .any(|workspace| workspace.solution.owns_package_root(root))
        });
        let retained: BTreeSet<_> = self
            .workspaces
            .values()
            .flat_map(|workspace| &workspace.solution.projects)
            .flat_map(|project| &project.documents)
            .filter_map(|entry| self.document_uri(&entry.uri).ok().map(Cow::into_owned))
            .collect();
        let obsolete: BTreeSet<_> = removed
            .solution
            .projects
            .iter()
            .flat_map(|project| &project.documents)
            .filter_map(|entry| self.document_uri(&entry.uri).ok().map(Cow::into_owned))
            .filter(|uri| {
                !retained.contains(uri.as_str())
                    && self.documents.get(uri).is_some_and(|state| !state.opened)
            })
            .collect();
        let count = obsolete.len();
        for uri in obsolete {
            self.set_state(uri, None);
        }
        count
    }
    pub fn diagnostics(&self, uri: &str) -> Result<Vec<Diagnostic>> {
        let schema = self.schema_for(uri);
        let state = self.state(uri)?;
        let mut out = Vec::new();
        for diagnostic in state.document.diagnostics() {
            out.push(diagnostic_at(
                state,
                diagnostic.span,
                1,
                "xml",
                &diagnostic.message,
            )?);
        }
        if !state.document.is_valid() {
            return Ok(out);
        }
        for node in state.document.ast().elements() {
            if node.kind == NodeKind::PropertyElement {
                if node.namespace_uri.as_deref() == Some(crate::design_support::DESIGN_NAMESPACE) {
                    if crate::design_support::property_element(schema, &state.document, node)
                        .is_none()
                    {
                        out.push(diagnostic_at(
                            state,
                            node.name_span,
                            2,
                            "unknown-property",
                            &format!("Unknown design-time property element {}", node.name),
                        )?);
                    }
                    continue;
                }
                if let Some((owner, property)) = node.local_name().split_once('.') {
                    if schema
                        .type_info(owner, node.namespace_uri.as_deref())
                        .is_some()
                        && !schema
                            .properties(owner, node.namespace_uri.as_deref())
                            .contains_key(property)
                    {
                        out.push(diagnostic_at(
                            state,
                            node.name_span,
                            2,
                            "unknown-property",
                            &format!("Unknown property element {}", node.name),
                        )?);
                    }
                }
                continue;
            }
            let known = schema
                .type_info(&node.name, node.namespace_uri.as_deref())
                .is_some();
            if !known
                && node.namespace_uri.as_deref()
                    != Some("http://schemas.microsoft.com/winfx/2006/xaml")
            {
                out.push(diagnostic_at(
                    state,
                    node.name_span,
                    2,
                    "unknown-type",
                    &format!(
                        "Type {} is not registered in the language schema",
                        node.name
                    ),
                )?);
            }
            for attribute in &node.attributes {
                if attribute.is_namespace()
                    || attribute.namespace_uri.as_deref()
                        == Some("http://schemas.microsoft.com/winfx/2006/xaml")
                    || attribute.name.starts_with("xml:")
                {
                    continue;
                }
                if ferroui_designer_core::design_fixtures::is_fixtures(attribute) {
                    let result = if node.id == state.document.ast().root() {
                        ferroui_designer_core::design_fixtures::DesignFixtures::parse(&attribute.value).map(|_| ())
                    } else {
                        Err("Design fixtures belong on the view root.".into())
                    };
                    if let Err(message) = result {
                        out.push(diagnostic_at(state, attribute.value_span, 1, "design-fixtures", &message)?);
                    }
                    continue;
                }
                let expression =
                    attribute.value.starts_with('{') && !attribute.value.starts_with("{}");
                if expression {
                    match attribute.markup() {
                        Err(error) => out.push(diagnostic_at(
                            state,
                            attribute.value_span,
                            1,
                            "markup",
                            &error,
                        )?),
                        Ok(MarkupValue::Extension(extension)) => {
                            if let Some(problem) = crate::binding::case_mismatch(
                                schema,
                                &state.document,
                                node,
                                attribute.value_span,
                                &extension,
                                |name| self.binding_named_element(uri, node.id, name),
                            ) {
                                out.push(diagnostic_at(
                                    state,
                                    problem.span,
                                    2,
                                    "binding-member-case",
                                    &format!(
                                        "Binding member {} has a case mismatch. The registered spelling is {}.",
                                        problem.authored, problem.registered,
                                    ),
                                )?);
                            }
                        }
                        Ok(MarkupValue::Literal(_)) => (),
                    }
                    if let Some(message) =
                        crate::design_support::sample_diagnostic(&state.document, node, attribute)
                    {
                        out.push(diagnostic_at(
                            state,
                            attribute.value_span,
                            1,
                            "sample-data",
                            &message,
                        )?);
                    }
                }
                if let Some(property) = schema.attribute_property(&state.document, node, attribute)
                {
                    if !expression
                        && !property.values.is_empty()
                        && !property
                            .values
                            .iter()
                            .any(|v| v.eq_ignore_ascii_case(&attribute.value))
                    {
                        out.push(diagnostic_at(
                            state,
                            attribute.value_span,
                            1,
                            "enum-value",
                            &format!("Expected one of {}", property.values.join(", ")),
                        )?);
                    }
                    if !expression
                        && matches!(property.type_name.as_str(), "f64" | "f32")
                        && !matches!(
                            attribute.value.as_str(),
                            "Auto" | "NaN" | "Infinity" | "-Infinity"
                        )
                        && attribute.value.parse::<f64>().is_err()
                    {
                        out.push(diagnostic_at(
                            state,
                            attribute.value_span,
                            1,
                            "numeric-value",
                            "Expected a numeric value",
                        )?);
                    }
                    if property.read_only {
                        out.push(diagnostic_at(
                            state,
                            attribute.name_span,
                            1,
                            "read-only",
                            "Property cannot be assigned",
                        )?);
                    }
                } else if known
                    && (!attribute.name.contains(':')
                        || attribute.namespace_uri.as_deref()
                            == Some(crate::design_support::DESIGN_NAMESPACE))
                {
                    out.push(diagnostic_at(
                        state,
                        attribute.name_span,
                        2,
                        "unknown-property",
                        &format!(
                            "Property {} is not registered on {}",
                            attribute.name, node.name
                        ),
                    )?);
                }
            }
        }
        let occurrences = &state.symbols;
        let mut declarations = BTreeSet::new();
        for occurrence in occurrences.iter().filter(|o| o.declaration) {
            let scope = match occurrence.kind {
                SymbolKind::Name(scope) => scope,
                SymbolKind::Resource => resource_scope(&state.document, occurrence.node),
            };
            if !declarations.insert((
                scope,
                occurrence.name.clone(),
                matches!(occurrence.kind, SymbolKind::Resource),
            )) {
                out.push(diagnostic_at(
                    state,
                    occurrence.span,
                    1,
                    "duplicate-symbol",
                    &format!("Duplicate declaration {} in this scope", occurrence.name),
                )?);
            }
        }
        for occurrence in occurrences.iter().filter(|o| !o.declaration) {
            if self.resolve(uri, occurrence).is_empty() {
                out.push(diagnostic_at(
                    state,
                    occurrence.span,
                    2,
                    "unresolved-reference",
                    &format!("No indexed declaration for {}", occurrence.name),
                )?);
            }
        }
        Ok(out)
    }
    pub fn completions(&self, uri: &str, offset: usize) -> Result<Vec<Completion>> {
        let schema = self.schema_for(uri);
        let uri = self.document_uri(uri)?;
        let uri = uri.as_ref();
        let state = self.state(uri)?;
        let source = state.index.source();
        state.index.position(offset)?;
        let mut start = source[..offset]
            .char_indices()
            .rev()
            .take_while(|(_, c)| c.is_alphanumeric() || matches!(c, '_' | ':' | '.' | '-'))
            .last()
            .map_or(offset, |(i, _)| i);
        let mut prefix = &source[start..offset];
        let mut span = SourceSpan::new(start, offset);
        let mut proposals = BTreeMap::<String, (String, String, u32, String)>::new();
        let open = source[..offset].rfind('<');
        let tag = open.map(|i| &source[i + 1..offset]);
        if let Some(tag) = tag {
            if tag.starts_with("!--") || tag.starts_with("![CDATA[") {
                return Ok(Vec::new());
            }
            let tag_name = tag
                .trim_start_matches('/')
                .split(|c: char| c.is_whitespace() || c == '/')
                .next()
                .unwrap_or("");
            let namespace = completion_namespace(state, tag_name, open.unwrap(), offset);
            let mut quote = None;
            let mut value_start = 0;
            for (index, ch) in tag.char_indices() {
                if Some(ch) == quote {
                    quote = None;
                } else if quote.is_none() && matches!(ch, '\'' | '"') {
                    quote = Some(ch);
                    value_start = index + 1;
                }
            }
            if quote.is_some() {
                let before = &tag[..value_start - 1];
                let property = before
                    .trim_end()
                    .trim_end_matches('=')
                    .trim_end()
                    .rsplit(|c: char| c.is_whitespace())
                    .next()
                    .unwrap_or("");
                let value = &tag[value_start..];
                if let Some(brace) =
                    crate::references::active_extension(value).map(|active| active.start)
                {
                    let extension = &value[brace + 1..];
                    if !extension.contains(|c: char| c.is_whitespace() || c == ',') {
                        for item in schema.extensions.values() {
                            proposals.insert(
                                item.name.clone(),
                                (
                                    "Markup extension".into(),
                                    item.documentation.clone(),
                                    3,
                                    item.name.clone(),
                                ),
                            );
                        }
                        if let Some((design_prefix, _)) = extension.split_once(':') {
                            let extension_namespace =
                                completion_namespace(state, extension, open.unwrap(), offset);
                            if let Some(info) = crate::design_support::extension(
                                "SampleData",
                                extension_namespace.as_deref(),
                            ) {
                                let name = format!("{design_prefix}:{}", info.name);
                                proposals.insert(
                                    name.clone(),
                                    (
                                        "Markup extension".into(),
                                        info.documentation.clone(),
                                        3,
                                        name,
                                    ),
                                );
                            }
                        }
                    } else {
                        let name = extension
                            .split(|c: char| c.is_whitespace() || c == ',')
                            .next()
                            .unwrap_or("");
                        let extension_namespace =
                            completion_namespace(state, name, open.unwrap(), offset);
                        if let Some(info) = schema
                            .extension(name, extension_namespace.as_deref())
                            .or_else(|| {
                                crate::design_support::extension(
                                    name,
                                    extension_namespace.as_deref(),
                                )
                            })
                        {
                            for parameter in &info.parameters {
                                proposals.insert(
                                    parameter.clone(),
                                    (
                                        "Parameter".into(),
                                        info.documentation.clone(),
                                        5,
                                        format!("{parameter}="),
                                    ),
                                );
                            }
                        }
                        if matches!(name, "Binding" | "CompiledBinding" | "ReflectionBinding") {
                            let argument = crate::references::markup_arguments(value)
                                .into_iter()
                                .rev()
                                .find(|argument| argument.span.end == value.trim_end().len());
                            let path = argument
                                .filter(|argument| {
                                    argument.name.as_deref().is_none_or(|name| name == "Path")
                                })
                                .map(|argument| argument.value)
                                .or_else(|| {
                                    extension[name.len()..].trim().is_empty().then(String::new)
                                });
                            if let Some(path) = path {
                                if let Some(node) = state.document.ast().element_at(open.unwrap()) {
                                    let context_matches = state.document.is_valid()
                                        || state.document.valid_source().get(..open.unwrap())
                                            == source.get(..open.unwrap());
                                    if context_matches {
                                        let expression = state
                                            .document
                                            .is_valid()
                                            .then(|| {
                                                state
                                                    .document
                                                    .ast()
                                                    .node(node)
                                                    .unwrap()
                                                    .attributes
                                                    .iter()
                                                    .find(|attribute| {
                                                        attribute.value_span.contains(offset)
                                                    })
                                            })
                                            .flatten()
                                            .map(|attribute| &source[attribute.value_span.range()])
                                            .unwrap_or(value);
                                        for property in crate::binding::members(
                                            schema,
                                            &state.document,
                                            node,
                                            expression,
                                            &path,
                                        )
                                        .into_values()
                                        {
                                            proposals.insert(
                                                property.name.clone(),
                                                (
                                                    property.type_name,
                                                    property.documentation,
                                                    5,
                                                    property.name,
                                                ),
                                            );
                                        }
                                        if let Some(dot) = prefix.rfind('.') {
                                            start += dot + 1;
                                            prefix = &source[start..offset];
                                            span = SourceSpan::new(start, offset);
                                        }
                                    }
                                }
                            }
                        }
                        for occurrence in self.all_declarations() {
                            if matches!(occurrence.1.kind, SymbolKind::Resource)
                                && matches!(name, "StaticResource" | "DynamicResource")
                                || matches!(occurrence.1.kind, SymbolKind::Name(_))
                                    && (name.ends_with("Reference")
                                        || extension.contains("ElementName"))
                            {
                                let name = occurrence.1.name;
                                proposals.insert(
                                    name.clone(),
                                    ("Declared symbol".into(), occurrence.0, 18, name),
                                );
                            }
                        }
                    }
                } else if let Some(info) = schema.property_in_context(
                    tag_name,
                    namespace.as_deref(),
                    property,
                    if property.contains(['.', ':']) {
                        completion_namespace(state, property, open.unwrap(), offset)
                    } else {
                        namespace.clone()
                    }
                    .as_deref(),
                ) {
                    for value in info.values {
                        proposals.insert(
                            value.clone(),
                            (
                                info.type_name.clone(),
                                info.documentation.clone(),
                                20,
                                value,
                            ),
                        );
                    }
                }
                if property == "Source" {
                    for candidate in self.documents.keys() {
                        if candidate != uri {
                            let label =
                                candidate.rsplit('/').next().unwrap_or(candidate).to_owned();
                            proposals.insert(
                                label.clone(),
                                ("Project document".into(), candidate.clone(), 17, label),
                            );
                        }
                    }
                }
            } else if tag.contains(char::is_whitespace) && !tag.ends_with('>') {
                for property in schema.properties(tag_name, namespace.as_deref()).values() {
                    if !property.read_only {
                        proposals.insert(
                            property.name.clone(),
                            (
                                property.type_name.clone(),
                                property.documentation.clone(),
                                if property.event { 23 } else { 10 },
                                format!("{}=\"\"", property.name),
                            ),
                        );
                    }
                }
                for info in schema.types.values() {
                    for property in info.properties.values().filter(|p| p.attached) {
                        let name = format!("{}.{}", info.name, property.name);
                        proposals.insert(
                            name.clone(),
                            (
                                property.type_name.clone(),
                                property.documentation.clone(),
                                10,
                                format!("{name}=\"\""),
                            ),
                        );
                    }
                }
                for name in ["x:Name", "x:Key", "x:DataType", "xmlns:x"] {
                    proposals.insert(
                        name.into(),
                        (
                            "XAML directive".into(),
                            String::new(),
                            14,
                            format!("{name}=\"\""),
                        ),
                    );
                }
                if let Some((member_prefix, _)) = prefix.split_once(':') {
                    let member_namespace =
                        completion_namespace(state, prefix, open.unwrap(), offset);
                    if member_namespace.as_deref() == Some(crate::design_support::DESIGN_NAMESPACE)
                    {
                        let mut properties = schema.properties(tag_name, namespace.as_deref());
                        if state.document.ast().element_at(open.unwrap()) == Some(state.document.ast().root()) {
                            let property = crate::design_support::fixtures_property();
                            properties.insert(property.name.clone(), property);
                        }
                        for (alias, native) in
                            [("DesignWidth", "Width"), ("DesignHeight", "Height")]
                        {
                            if let Some(mut property) = properties.get(native).cloned() {
                                property.name = alias.into();
                                properties.insert(alias.into(), property);
                            }
                        }
                        for member in ["PreviewWith", "DesignStyle"] {
                            if let Some(property) = crate::design_support::property(
                                schema,
                                tag_name,
                                namespace.as_deref(),
                                member,
                            ) {
                                properties.insert(member.into(), property);
                            }
                        }
                        for info in schema
                            .types
                            .values()
                            .filter(|info| info.namespace == crate::schema::FRAMEWORK_NAMESPACE)
                        {
                            for property in info
                                .properties
                                .values()
                                .filter(|property| property.attached)
                            {
                                let mut property = property.clone();
                                property.name = format!("{}.{}", info.name, property.name);
                                properties.insert(property.name.clone(), property);
                            }
                        }
                        for property in properties.values().filter(|property| {
                            !property.read_only
                                && !property.event
                                && !matches!(
                                    property.name.as_str(),
                                    "Name" | "Class" | "Key" | "Ignorable"
                                )
                        }) {
                            let label = format!("{member_prefix}:{}", property.name);
                            proposals.insert(
                                label.clone(),
                                (
                                    property.type_name.clone(),
                                    property.documentation.clone(),
                                    10,
                                    format!("{label}=\"\""),
                                ),
                            );
                        }
                    } else if member_namespace.as_deref()
                        == Some(crate::design_support::COMPATIBILITY_NAMESPACE)
                    {
                        let info = crate::design_support::ignorable_property();
                        let label = format!("{member_prefix}:Ignorable");
                        proposals.insert(
                            label.clone(),
                            (
                                info.type_name,
                                info.documentation,
                                14,
                                format!("{label}=\"\""),
                            ),
                        );
                    }
                }
            } else {
                if let Some((owner, _)) = tag_name.rsplit_once('.') {
                    let owner_namespace = if namespace.as_deref()
                        == Some(crate::design_support::DESIGN_NAMESPACE)
                    {
                        // A design property element on a custom control keeps that owner's
                        // namespace, while ordinary d:Control elements use native types.
                        state
                            .document
                            .ast()
                            .element_at(open.unwrap())
                            .and_then(|id| state.document.ast().node(id))
                            .and_then(|node| {
                                let parent = if node.kind == NodeKind::PropertyElement {
                                    state.document.ast().node(node.parent?)?
                                } else {
                                    node
                                };
                                (parent.local_name() == owner.rsplit(':').next().unwrap_or(owner))
                                    .then(|| parent.namespace_uri.clone())
                                    .flatten()
                            })
                            .or_else(|| namespace.clone())
                    } else {
                        namespace.clone()
                    };
                    for property in schema
                        .properties(owner, owner_namespace.as_deref())
                        .values()
                        .filter(|property| !property.read_only && !property.event)
                    {
                        let label = format!("{owner}.{}", property.name);
                        proposals.insert(
                            label.clone(),
                            (
                                property.type_name.clone(),
                                property.documentation.clone(),
                                10,
                                label,
                            ),
                        );
                    }
                }
                for info in schema.types.values().filter(|t| {
                    t.constructible
                        && namespace.as_deref().is_none_or(|ns| {
                            schema
                                .type_info(&t.name, Some(ns))
                                .is_some_and(|resolved| resolved.namespace == t.namespace)
                        })
                }) {
                    let label = tag_name.split_once(':').map_or_else(
                        || info.name.clone(),
                        |(prefix, _)| format!("{prefix}:{}", info.name),
                    );
                    proposals.insert(
                        label.clone(),
                        (
                            info.base.clone().unwrap_or_default(),
                            info.documentation.clone(),
                            7,
                            label,
                        ),
                    );
                }
            }
        } else {
            for info in schema.types.values().filter(|t| t.constructible) {
                proposals.insert(
                    info.name.clone(),
                    (
                        info.base.clone().unwrap_or_default(),
                        info.documentation.clone(),
                        7,
                        format!("<{} />", info.name),
                    ),
                );
            }
        }
        let mut result = Vec::new();
        for (label, (detail, documentation, kind, insert_text)) in proposals {
            if label.to_lowercase().starts_with(&prefix.to_lowercase()) {
                result.push(Completion {
                    label,
                    detail,
                    documentation,
                    kind,
                    span,
                    text_edit: TextEdit {
                        range: state.index.range(span)?,
                        new_text: insert_text.clone(),
                    },
                    insert_text,
                });
            }
        }
        Ok(result)
    }
    pub fn hover(&self, uri: &str, offset: usize) -> Result<Option<Hover>> {
        let schema = self.schema_for(uri);
        let state = self.state(uri)?;
        state.index.position(offset)?;
        if !state.document.is_valid() {
            return Ok(None);
        }
        if let Some((property, span)) = crate::binding::at(schema, &state.document, offset) {
            return Ok(Some(Hover {
                contents: format!(
                    "{}: {}\n\n{}",
                    property.name, property.type_name, property.documentation
                ),
                range: state.index.range(span)?,
            }));
        }
        if let Some(node) = state
            .document
            .ast()
            .element_at(offset)
            .and_then(|id| state.document.ast().node(id))
        {
            for attribute in &node.attributes {
                if attribute.span.contains(offset) {
                    if let Some(info) = schema.attribute_property(&state.document, node, attribute)
                    {
                        return Ok(Some(Hover {
                            contents: format!(
                                "{}: {}\n\n{}",
                                attribute.name, info.type_name, info.documentation
                            ),
                            range: state.index.range(attribute.name_span)?,
                        }));
                    }
                }
            }
            if let Some(info) = schema.type_info(&node.name, node.namespace_uri.as_deref()) {
                return Ok(Some(Hover {
                    contents: format!("{}\n\n{}", info.name, info.documentation),
                    range: state.index.range(node.name_span)?,
                }));
            }
        }
        Ok(None)
    }
    pub fn signature_help(&self, uri: &str, offset: usize) -> Result<Option<SignatureHelp>> {
        let schema = self.schema_for(uri);
        let state = self.state(uri)?;
        state.index.position(offset)?;
        let Some(prefix) = attribute_prefix(state, offset) else {
            return Ok(None);
        };
        let Some(active_extension) = crate::references::active_extension(prefix) else {
            return Ok(None);
        };
        let name = active_extension.name;
        let open = state.index.source()[..offset].rfind('<').unwrap_or(0);
        let namespace = completion_namespace(state, &name, open, offset);
        let Some(info) = schema
            .extension(&name, namespace.as_deref())
            .or_else(|| crate::design_support::extension(&name, namespace.as_deref()))
        else {
            return Ok(None);
        };
        let active = active_extension
            .argument
            .split_once('=')
            .and_then(|(name, _)| {
                info.parameters
                    .iter()
                    .position(|parameter| parameter == name.trim())
            })
            .unwrap_or(0);
        Ok(Some(SignatureHelp {
            signatures: vec![Signature {
                label: format!("{{{} {}}}", name, info.parameters.join(", ")),
                documentation: info.documentation.clone(),
                parameters: info.parameters.clone(),
            }],
            active_signature: 0,
            active_parameter: active as u32,
        }))
    }
    pub fn references(&self, uri: &str, offset: usize) -> Result<Vec<Location>> {
        self.references_with_declarations(uri, offset, true)
    }
    pub fn references_with_declarations(
        &self,
        uri: &str,
        offset: usize,
        include: bool,
    ) -> Result<Vec<Location>> {
        let target = self.target(uri, offset)?;
        let Some(target) = target else {
            return Ok(Vec::new());
        };
        let mut result = Vec::new();
        for (candidate, state) in &self.documents {
            for occurrence in state.symbols.iter().cloned() {
                if !include && occurrence.declaration {
                    continue;
                }
                let resolved = self.resolve(candidate, &occurrence);
                if resolved
                    .iter()
                    .any(|candidate| candidate.0 == target.0 && candidate.1.span == target.1.span)
                {
                    result.push(self.location(candidate, occurrence.span)?);
                }
            }
        }
        Ok(result)
    }
    pub fn apply_workspace_edit(&mut self, edit: &WorkspaceEdit) -> Result<()> {
        let mut staged = Vec::new();
        let mut seen = BTreeSet::new();
        let mut disk_revision = self.disk_revision;
        for changes in &edit.document_changes {
            let uri = self.document_uri(&changes.uri)?.into_owned();
            require(seen.insert(uri.clone()), "Duplicate document edit")?;
            let state = self.state(&uri)?;
            require(state.version == changes.version, "Workspace edit is stale")?;
            let source = apply_text_edits(state.index.source(), &changes.edits)?;
            let version = if state.opened {
                state.version.checked_add(1).ok_or_else(|| LanguageError("Version exhausted".into()))?
            } else {
                disk_revision = previous_disk_revision(disk_revision)?;
                disk_revision
            };
            let state = state.with_text(LineIndex::new(&source), version, "Workspace refactoring");
            staged.push((uri, state));
        }
        for (uri, state) in staged {
            self.set_state(uri, Some(state));
        }
        self.disk_revision = disk_revision;
        Ok(())
    }
    pub fn format(&self, uri: &str, options: FormattingOptions) -> Result<Vec<TextEdit>> {
        crate::formatting::format(&self.state(uri)?.document, options, None)
    }
    pub fn format_range(
        &self,
        uri: &str,
        range: Range,
        options: FormattingOptions,
    ) -> Result<Vec<TextEdit>> {
        let state = self.state(uri)?;
        crate::formatting::format(&state.document, options, Some(state.index.span(range)?))
    }
    pub fn symbols(&self, uri: &str) -> Result<Vec<DocumentSymbol>> {
        let state = self.state(uri)?;
        if !state.document.is_valid() {
            return Ok(Vec::new());
        }
        fn symbol(state: &State, id: u64) -> Result<DocumentSymbol> {
            let node = state.document.ast().node(id).unwrap();
            let name = node
                .xaml_name()
                .or_else(|| {
                    node.attributes
                        .iter()
                        .find(|a| a.name.ends_with(":Key"))
                        .map(|a| a.value.as_str())
                })
                .unwrap_or(&node.name);
            Ok(DocumentSymbol {
                name: name.into(),
                detail: node.name.clone(),
                kind: if node.kind == NodeKind::PropertyElement {
                    7
                } else {
                    5
                },
                range: state.index.range(node.span)?,
                selection_range: state.index.range(node.name_span)?,
                children: state
                    .document
                    .ast()
                    .element_children(id)
                    .into_iter()
                    .map(|id| symbol(state, id))
                    .collect::<Result<_>>()?,
            })
        }
        Ok(vec![symbol(state, state.document.ast().root())?])
    }
    pub fn folding(&self, uri: &str) -> Result<Vec<FoldingRange>> {
        let state = self.state(uri)?;
        if !state.document.is_valid() {
            return Ok(Vec::new());
        }
        let mut result = Vec::new();
        for node in state.document.ast().nodes() {
            if !matches!(
                node.kind,
                NodeKind::Element | NodeKind::PropertyElement | NodeKind::Comment | NodeKind::CData
            ) {
                continue;
            }
            let range = state.index.range(node.span)?;
            if range.end.line > range.start.line {
                result.push(FoldingRange {
                    start_line: range.start.line,
                    start_character: range.start.character,
                    end_line: range.end.line,
                    end_character: range.end.character,
                    kind: if node.kind == NodeKind::Comment {
                        "comment"
                    } else {
                        "region"
                    }
                    .into(),
                });
            }
        }
        Ok(result)
    }
    /// Token types: type, property, string, comment, variable, keyword, number, namespace.
    pub fn semantic_tokens(&self, uri: &str) -> Result<Vec<SemanticToken>> {
        let state = self.state(uri)?;
        if !state.document.is_valid() {
            return Ok(Vec::new());
        }
        let mut spans = Vec::new();
        for node in state.document.ast().nodes() {
            if node.is_element() {
                spans.push((node.name_span, 0));
                if let Some(closing) = node.closing_span {
                    let start = closing.start + 2;
                    spans.push((SourceSpan::new(start, start + node.name.len()), 0));
                }
                for attribute in &node.attributes {
                    spans.push((
                        attribute.name_span,
                        if attribute.is_namespace() { 7 } else { 1 },
                    ));
                    spans.push((
                        attribute.value_span,
                        if attribute.value.parse::<f64>().is_ok() {
                            6
                        } else {
                            2
                        },
                    ));
                }
            } else if matches!(node.kind, NodeKind::Comment | NodeKind::CData) {
                spans.push((node.span, 3));
            }
        }
        let mut tokens = Vec::new();
        for (span, kind) in spans {
            let mut start = span.start;
            for part in state.index.source()[span.range()].split_inclusive(['\r', '\n']) {
                let text = part.trim_end_matches(['\r', '\n']);
                if !text.is_empty() {
                    let position = state.index.position(start)?;
                    tokens.push(SemanticToken {
                        line: position.line,
                        start: position.character,
                        length: text.encode_utf16().count() as u32,
                        token_type: kind,
                        modifiers: 0,
                    });
                }
                start += part.len();
            }
        }
        tokens.sort_by_key(|t| (t.line, t.start));
        Ok(tokens)
    }
    pub fn workspace_symbols(&self, query: &str) -> Result<Vec<(String, DocumentSymbol)>> {
        let mut out = Vec::new();
        let query = query.to_lowercase();
        for uri in self.documents.keys() {
            let mut pending = self.symbols(uri)?;
            while let Some(symbol) = pending.pop() {
                pending.extend(symbol.children.clone());
                if symbol.name.to_lowercase().contains(&query) {
                    out.push((uri.clone(), symbol));
                }
            }
        }
        Ok(out)
    }
    pub fn resource_uri(&self, base: &str, reference: &str) -> Result<String> {
        if let Some((_, schema)) = self.project_schema_for(base) {
            let owner = self.document_uri(base)?;
            if let Some(uri) = schema.resources.resolve(&owner, reference)? {
                return Ok(uri.to_owned());
            }
        }
        let resolved = ferroui_designer_project::resolve_uri(base, reference)?;
        if resolved.starts_with("ferres://") || resolved.starts_with("resm:") {
            let owner = self.document_uri(base)?;
            if let Some(path) = self
                .workspace_root(&owner)
                .and_then(|root| self.workspaces[root].resolve_path(&resolved).ok())
            {
                return Ok(ferroui_designer_project::file_uri(path)?);
            }
            let mut matches = BTreeSet::new();
            for workspace in self.workspaces.values() {
                if let Ok(path) = workspace.resolve_path(&resolved) {
                    matches.insert(ferroui_designer_project::file_uri(path)?);
                }
            }
            require(
                matches.len() <= 1,
                "Resource URI is ambiguous across workspace roots",
            )?;
            if let Some(uri) = matches.into_iter().next() {
                return Ok(uri);
            }
        }
        Ok(self.document_uri(&resolved)?.into_owned())
    }
    pub fn prepare_rename(&self, uri: &str, offset: usize) -> Result<Option<(Range, String)>> {
        let state = self.state(uri)?;
        let target = self.target(uri, offset)?;
        let occurrence = state
            .symbols
            .iter()
            .find(|o| o.span.contains(offset) || o.span.end == offset)
            .cloned();
        match (target, occurrence) {
            (Some(_), Some(occurrence)) => {
                Ok(Some((state.index.range(occurrence.span)?, occurrence.name)))
            }
            _ => Ok(None),
        }
    }
    pub fn mark_saved(&mut self, uri: &str, text: Option<&str>) -> Result<()> {
        let uri = self.document_uri(uri)?.into_owned();
        let state = self
            .documents
            .get_mut(&uri)
            .ok_or_else(|| LanguageError("Document is not indexed".into()))?;
        require(
            text.is_none_or(|text| text == state.document.source()),
            "Saved text differs from the synchronized document",
        )?;
        state.document.mark_saved();
        Ok(())
    }
    pub fn rename_uri(&mut self, old: &str, new: &str) -> Result<()> {
        self.rename_uris(&[(old.into(), new.into())])
    }
    /// Validate an entire client rename notification before moving any indexed
    /// state. A swap is legal; overwriting an unrelated document is not.
    pub fn rename_uris(&mut self, renames: &[(String, String)]) -> Result<()> {
        let mut mapping = BTreeMap::new();
        let mut destinations = BTreeSet::new();
        for (old, new) in renames {
            let old = self.document_uri(old)?.into_owned();
            let new = self.document_uri(new)?.into_owned();
            require(
                mapping.insert(old, new.clone()).is_none(),
                "Duplicate rename source",
            )?;
            require(destinations.insert(new), "Duplicate rename destination")?;
        }
        for new in mapping.values() {
            require(
                !self.documents.contains_key(new) || mapping.contains_key(new),
                "Rename destination is already indexed",
            )?;
        }
        for schema in self.package_schemas.values_mut() {
            if mapping.keys().any(|from| schema.resources.contains_source(from)) {
                std::sync::Arc::make_mut(&mut schema.resources).retarget(&mapping);
            }
        }
        let staged: Vec<_> = mapping
            .iter()
            .filter_map(|(old, new)| {
                self.documents
                    .get(old)
                    .cloned()
                    .map(|state| (new.clone(), state))
            })
            .collect();
        for old in mapping.keys() {
            self.set_state(old.clone(), None);
        }
        for (new, state) in staged {
            self.set_state(new, Some(state));
        }
        for entry in self
            .workspaces
            .values_mut()
            .flat_map(|workspace| &mut workspace.solution.projects)
            .flat_map(|project| &mut project.documents)
        {
            let identity = ferroui_designer_project::canonical_file_uri(&entry.uri)
                .unwrap_or_else(|_| entry.uri.clone());
            if let Some(new) = mapping.get(&identity) {
                entry.uri = new.clone();
                if let Ok(path) = ferroui_designer_project::file_path(new) {
                    entry.path = path;
                }
            }
        }
        Ok(())
    }
    pub fn file_rename_edits(&self, old: &str, new: &str) -> Result<WorkspaceEdit> {
        let old = self.document_uri(old)?;
        let mut result = WorkspaceEdit::default();
        for (uri, state) in &self.documents {
            if !state.document.is_valid() {
                continue;
            }
            let resources = self.compiled_resources(uri);
            let mut edits = Vec::new();
            for node in state.document.ast().elements() {
                for attribute in &node.attributes {
                    if attribute.name == "Source"
                        && resources.as_ref().is_none_or(|resources| {
                            matches!(resources.resolve(uri, &attribute.value), Ok(None))
                        })
                        && self
                            .resource_uri(uri, &attribute.value)
                            .is_ok_and(|target| target == old)
                    {
                        edits.push(TextEdit {
                            range: state.index.range(attribute.value_span)?,
                            new_text: new
                                .replace('&', "&amp;")
                                .replace('"', "&quot;")
                                .replace('\'', "&apos;"),
                        });
                    }
                }
            }
            if !edits.is_empty() {
                result.document_changes.push(DocumentEdits {
                    uri: uri.clone(),
                    version: state.version,
                    edits,
                });
            }
        }
        Ok(result)
    }
    fn state(&self, uri: &str) -> Result<&State> {
        self.documents
            .get(self.document_uri(uri)?.as_ref())
            .ok_or_else(|| LanguageError(format!("Document is not indexed: {uri}")))
    }
    pub(crate) fn document_uri<'a>(&self, uri: &'a str) -> Result<Cow<'a, str>> {
        if self.documents.contains_key(uri) || !uri.starts_with("file://") {
            Ok(Cow::Borrowed(uri))
        } else {
            Ok(Cow::Owned(ferroui_designer_project::canonical_file_uri(
                uri,
            )?))
        }
    }
    fn location(&self, uri: &str, span: SourceSpan) -> Result<Location> {
        Ok(Location {
            uri: uri.into(),
            range: self.state(uri)?.index.range(span)?,
        })
    }
    fn workspace_root(&self, uri: &str) -> Option<&std::path::PathBuf> {
        let path = ferroui_designer_project::file_path(uri).ok()?;
        self.workspaces
            .keys()
            .filter(|root| path.starts_with(root))
            .max_by_key(|root| root.as_os_str().len())
    }
    pub(crate) fn binding_named_element(&self, uri: &str, anchor: u64, name: &str) -> Option<u64> {
        let uri = self.document_uri(uri).ok()?;
        let scope = self.documents.get(uri.as_ref())?.document.ast().name_scope(anchor);
        let mut targets = self.declarations.get(name)?.iter().filter(|(source, occurrence)| {
            source == uri.as_ref() && occurrence.kind == SymbolKind::Name(scope)
        });
        let node = targets.next()?.1.node;
        targets.next().is_none().then_some(node)
    }
    fn all_declarations(&self) -> Vec<(String, Occurrence)> {
        self.declarations.values().flatten().cloned().collect()
    }
    fn target(&self, uri: &str, offset: usize) -> Result<Option<(String, Occurrence)>> {
        let state = self.state(uri)?;
        state.index.position(offset)?;
        let occurrence = state
            .symbols
            .iter()
            .find(|o| o.span.contains(offset) || o.span.end == offset)
            .cloned();
        let Some(occurrence) = occurrence else {
            return Ok(None);
        };
        let targets = self.resolve(uri, &occurrence);
        Ok((targets.len() == 1).then(|| targets[0].clone()))
    }
    fn resolve(&self, uri: &str, reference: &Occurrence) -> Vec<(String, Occurrence)> {
        self.resolve_with_context(uri, reference).matches
    }
    fn resolve_with_context(&self, uri: &str, reference: &Occurrence) -> ResourceLookup {
        self.resolve_with_declarations(uri, reference, &DeclarationIndex::new())
    }
    fn declarations_for<'a>(
        &'a self,
        name: &str,
        overrides: &'a DeclarationIndex,
    ) -> &'a [(String, Occurrence)] {
        overrides
            .get(name)
            .or_else(|| self.declarations.get(name))
            .map_or(&[], Vec::as_slice)
    }
    fn resolve_with_declarations(
        &self,
        uri: &str,
        reference: &Occurrence,
        overrides: &DeclarationIndex,
    ) -> ResourceLookup {
        let Ok(uri) = self.document_uri(uri) else {
            return ResourceLookup::missing();
        };
        let uri = uri.as_ref();
        if reference.declaration {
            return ResourceLookup {
                matches: vec![(uri.into(), reference.clone())],
                may_be_missing: false,
                themed: false,
            };
        }
        let found: Vec<_> = self
            .declarations_for(&reference.name, overrides)
            .iter()
            .filter(|(candidate, o)| {
                o.name == reference.name
                    && match (&o.kind, &reference.kind) {
                        (SymbolKind::Name(a), SymbolKind::Name(b)) => candidate == uri && a == b,
                        (SymbolKind::Resource, SymbolKind::Resource) => true,
                        _ => false,
                    }
            })
            .cloned()
            .collect();
        if reference.kind == SymbolKind::Resource {
            let seen = BTreeSet::new();
            let mut result = ResourceLookup::missing();
            if let Some(state) = self.documents.get(uri) {
                let mut current = Some(reference.node);
                while let Some(scope) = current {
                    result.fallback(self.resources_at(uri, scope, &reference.name, &seen, overrides));
                    if !result.may_be_missing {
                        return result;
                    }
                    current = state
                        .document
                        .ast()
                        .node(scope)
                        .and_then(|node| node.parent);
                }
            }
            // Standalone dictionaries and application resources may provide global resources.
            // Implicit lookup stays in the owning workspace. Explicit imports have
            // already been resolved through the document's dictionary graph.
            let home = self.workspace_root(uri);
            let candidates: BTreeSet<_> = found
                .iter()
                .filter_map(|(candidate, _)| {
                    let state = self.documents.get(candidate)?;
                    let root = state.document.ast().node(state.document.ast().root())?;
                    (candidate != uri
                        && self.workspace_root(candidate) == home
                        && matches!(
                            root.local_name(),
                            "ResourceDictionary" | "Application" | "Styles" | "Theme"
                        ))
                    .then_some(candidate.clone())
                })
                .collect();
            let mut globals = ResourceLookup::missing();
            for candidate in candidates {
                let root = self.documents[&candidate].document.ast().root();
                globals.alternative(self.resources_at(
                    &candidate, root, &reference.name, &seen, overrides,
                ));
            }
            result.fallback(globals);
            return result;
        }
        ResourceLookup {
            may_be_missing: found.is_empty(),
            matches: found,
            themed: false,
        }
    }
    fn resources_at(
        &self,
        uri: &str,
        scope: u64,
        name: &str,
        seen: &BTreeSet<(String, u64)>,
        overrides: &DeclarationIndex,
    ) -> ResourceLookup {
        let mut seen = seen.clone();
        if !seen.insert((uri.into(), scope)) {
            return ResourceLookup::missing();
        }
        let Some(state) = self
            .documents
            .get(uri)
            .filter(|state| state.document.is_valid())
        else {
            return ResourceLookup::missing();
        };
        let found: Vec<_> = self
            .declarations_for(name, overrides)
            .iter()
            .filter(|(candidate, occurrence)| {
                candidate == uri
                    && occurrence.kind == SymbolKind::Resource
                    && resource_scope(&state.document, occurrence.node) == scope
            })
            .cloned()
            .collect();
        if !found.is_empty() {
            return ResourceLookup {
                matches: found,
                may_be_missing: false,
                themed: false,
            };
        }
        let edges = state.resource_edges.get(&scope).into_iter().flatten();
        let lookup_edge = |edge: &ResourceEdge| {
            let (target, next) = if let Some(source) = &edge.source {
                let Ok(target) = self.resource_uri(uri, source) else {
                    return ResourceLookup::missing();
                };
                let Some(imported) = self.documents.get(&target) else {
                    return ResourceLookup::missing();
                };
                (target, imported.document.ast().root())
            } else {
                (uri.into(), edge.node)
            };
            let mut result = self.resources_at(&target, next, name, &seen, overrides);
            if edge.kind != ResourceEdgeKind::Collection && !result.matches.is_empty() {
                result.themed = true;
            }
            result
        };
        // Unknown/custom runtime themes may miss every named variant. Default
        // covers that path, and also the fallback of a named variant without a key.
        let mut defaults: Option<ResourceLookup> = None;
        for edge in edges
            .clone()
            .filter(|edge| edge.kind == ResourceEdgeKind::DefaultTheme)
        {
            let found = lookup_edge(edge);
            if let Some(defaults) = &mut defaults {
                defaults.alternative(found);
            } else {
                defaults = Some(found);
            }
        }
        let defaults = defaults.unwrap_or_else(ResourceLookup::missing);
        let mut result = defaults.clone();
        for edge in edges
            .clone()
            .filter(|edge| edge.kind == ResourceEdgeKind::Theme)
        {
            let mut variant = lookup_edge(edge);
            variant.fallback(defaults.clone());
            result.alternative(variant);
        }
        // The runtime checks local values first, then themes, then merged
        // dictionaries from last to first. Conditional theme misses retain the
        // applicable merged/ancestor fallback instead of inventing one winner.
        for edge in edges
            .rev()
            .filter(|edge| edge.kind == ResourceEdgeKind::Collection)
        {
            if !result.may_be_missing {
                break;
            }
            result.fallback(lookup_edge(edge));
        }
        result
    }
    fn set_state(&mut self, uri: String, state: Option<State>) {
        if let Some(old) = self.documents.remove(&uri) {
            for occurrence in old.symbols.iter().filter(|o| o.declaration) {
                if let Some(entries) = self.declarations.get_mut(&occurrence.name) {
                    entries.retain(|(candidate, _)| candidate != &uri);
                    if entries.is_empty() {
                        self.declarations.remove(&occurrence.name);
                    }
                }
            }
        }
        if let Some(state) = state {
            for occurrence in state.symbols.iter().filter(|o| o.declaration) {
                self.declarations
                    .entry(occurrence.name.clone())
                    .or_default()
                    .push((uri.clone(), occurrence.clone()));
            }
            self.documents.insert(uri, state);
        }
    }
}
impl Default for LanguageService {
    fn default() -> Self {
        Self::new(Schema::ferroui())
    }
}
fn previous_disk_revision(revision: i64) -> Result<i64> {
    // Keep server-owned disk identities disjoint from editor versions. LSP
    // clients receive a null document version for these unopened snapshots.
    revision.checked_sub(1).ok_or_else(|| LanguageError("Disk revision exhausted".into()))
}
fn diagnostic_at(
    state: &State,
    span: SourceSpan,
    severity: u32,
    code: &str,
    message: &str,
) -> Result<Diagnostic> {
    Ok(Diagnostic {
        range: state.index.range(span)?,
        span,
        severity,
        code: code.into(),
        message: message.into(),
    })
}
fn occurrences(document: &Document) -> Vec<Occurrence> {
    if !document.is_valid() {
        return Vec::new();
    }
    let mut out: Vec<_> = crate::name_occurrences(document)
        .into_iter()
        .map(|o| Occurrence {
            name: o.name,
            span: o.span,
            kind: SymbolKind::Name(o.scope),
            declaration: o.declaration,
            node: o.node,
            spelling: OccurrenceSpelling::Attribute,
        })
        .collect();
    resource_occurrences::collect(document, &mut out);
    out
}
fn resource_scope(document: &Document, node: u64) -> u64 {
    let mut current = document.ast().node(node).and_then(|n| n.parent);
    while let Some(id) = current {
        let n = document.ast().node(id).unwrap();
        if n.local_name() == "ResourceDictionary" {
            return id;
        }
        if n.name.ends_with(".Resources") {
            return n.parent.unwrap_or(id);
        }
        current = n.parent;
    }
    document.ast().root()
}
fn resource_edges(document: &Document) -> BTreeMap<u64, Vec<ResourceEdge>> {
    let mut edges = BTreeMap::<u64, Vec<ResourceEdge>>::new();
    if !document.is_valid() {
        return edges;
    }
    for node in document.ast().elements() {
        let (source, kind) = match node.local_name() {
            "ResourceDictionary" | "ResourceInclude" if theme_provider(document, node) => (
                node.attribute("Source").map(str::to_owned),
                if default_theme_dictionary(document, node) {
                    ResourceEdgeKind::DefaultTheme
                } else {
                    ResourceEdgeKind::Theme
                },
            ),
            "ResourceInclude" | "StyleInclude"
                if native_resource_type(node.local_name(), node.namespace_uri.as_deref()) =>
            {
                match node.attribute("Source") {
                    Some(source) => (Some(source.to_owned()), ResourceEdgeKind::Collection),
                    None => continue,
                }
            }
            "ResourceDictionary"
                if node.id != document.ast().root()
                    && native_resource_type(
                        "ResourceDictionary",
                        node.namespace_uri.as_deref(),
                    )
                    && !node.attributes.iter().any(|attribute| {
                        attribute.name.rsplit(':').next() == Some("Key")
                            && attribute.namespace_uri.as_deref()
                                == Some("http://schemas.microsoft.com/winfx/2006/xaml")
                    })
                    && node
                        .parent
                        .and_then(|id| document.ast().node(id))
                        .is_some_and(|parent| {
                            parent.name.ends_with(".Resources")
                                || dictionary_property(document, parent, "MergedDictionaries")
                        }) =>
            {
                (None, ResourceEdgeKind::Collection)
            }
            _ => continue,
        };
        edges
            .entry(resource_scope(document, node.id))
            .or_default()
            .push(ResourceEdge {
                node: node.id,
                source,
                kind,
            });
    }
    for children in edges.values_mut() {
        children.sort_by_key(|edge| document.ast().node(edge.node).unwrap().span.start);
    }
    edges
}

fn native_resource_type(name: &str, namespace: Option<&str>) -> bool {
    static SCHEMA: std::sync::OnceLock<Schema> = std::sync::OnceLock::new();
    SCHEMA
        .get_or_init(Schema::ferroui)
        .type_info(name, namespace)
        .is_some_and(|info| {
            info.name == name && info.namespace == crate::schema::FRAMEWORK_NAMESPACE
        })
}

fn dictionary_property(document: &Document, node: &Node, property: &str) -> bool {
    node.kind == NodeKind::PropertyElement
        && node.local_name().split_once('.') == Some(("ResourceDictionary", property))
        && native_resource_type("ResourceDictionary", node.namespace_uri.as_deref())
        && node
            .parent
            .and_then(|parent| document.ast().node(parent))
            .is_some_and(|parent| {
                parent.local_name() == "ResourceDictionary"
                    && native_resource_type("ResourceDictionary", parent.namespace_uri.as_deref())
            })
}

fn theme_provider(document: &Document, node: &Node) -> bool {
    matches!(node.local_name(), "ResourceDictionary" | "ResourceInclude")
        && native_resource_type(node.local_name(), node.namespace_uri.as_deref())
        && node
            .parent
            .and_then(|parent| document.ast().node(parent))
            .is_some_and(|parent| dictionary_property(document, parent, "ThemeDictionaries"))
}

fn default_theme_dictionary(document: &Document, node: &Node) -> bool {
    let Some(attribute) = node.attributes.iter().find(|attribute| {
        attribute.name.rsplit(':').next() == Some("Key")
            && attribute.namespace_uri.as_deref()
                == Some("http://schemas.microsoft.com/winfx/2006/xaml")
    }) else {
        return false;
    };
    let Ok(value) = attribute.markup() else {
        return false;
    };
    let MarkupValue::Extension(extension) = value else {
        return attribute.value.trim() == "Default";
    };
    let namespaces = document.ast().namespaces(node.id);
    let (prefix, name) = extension
        .name
        .split_once(':')
        .unwrap_or(("", &extension.name));
    if namespaces.get(prefix).map(String::as_str)
        != Some("http://schemas.microsoft.com/winfx/2006/xaml")
        || name != "Static"
        || extension.arguments.len() != 1
    {
        return false;
    }
    let argument = &extension.arguments[0];
    if !matches!(argument.name.as_deref(), None | Some("Member")) {
        return false;
    }
    let MarkupValue::Literal(member) = &argument.value else {
        return false;
    };
    let Some((owner, "Default")) = member.rsplit_once('.') else {
        return false;
    };
    let (prefix, owner) = owner.split_once(':').unwrap_or(("", owner));
    owner == "ThemeVariant"
        && (prefix.is_empty() || namespaces.contains_key(prefix))
        && native_resource_type(owner, namespaces.get(prefix).map(String::as_str))
}
pub fn apply_text_edits(source: &str, edits: &[TextEdit]) -> Result<String> {
    let index = LineIndex::new(source);
    let mut patches = edits
        .iter()
        .map(|edit| Ok((index.span(edit.range)?, &edit.new_text)))
        .collect::<Result<Vec<_>>>()?;
    patches.sort_by_key(|(span, _)| (span.start, span.end));
    for pair in patches.windows(2) {
        require(
            pair[0].0.end <= pair[1].0.start
                && !(pair[0].0.is_empty()
                    && pair[1].0.is_empty()
                    && pair[0].0.start == pair[1].0.start),
            "Text edits overlap",
        )?;
    }
    let mut length = source.len();
    for (span, text) in &patches {
        length = length
            .checked_sub(span.len())
            .and_then(|length| length.checked_add(text.len()))
            .ok_or_else(|| LanguageError("Document size overflow".into()))?;
    }
    require(length <= 16 * 1024 * 1024, "Document exceeds 16 MiB")?;
    // Emit untouched slices once. Repeated replace_range calls shift the entire
    // remaining document for every edit and turn large renames into quadratic work.
    let mut output = String::with_capacity(length);
    let mut previous = 0;
    for (span, text) in patches {
        output.push_str(&source[previous..span.start]);
        output.push_str(text);
        previous = span.end;
    }
    output.push_str(&source[previous..]);
    Ok(output)
}
fn completion_namespace(state: &State, name: &str, open: usize, offset: usize) -> Option<String> {
    let prefix = name.split_once(':').map_or("", |(prefix, _)| prefix);
    if state.document.is_valid() {
        if let Some(node) = state.document.ast().element_at(open) {
            return state.document.ast().namespaces(node).get(prefix).cloned();
        }
    }
    let key = if prefix.is_empty() {
        "xmlns".into()
    } else {
        format!("xmlns:{prefix}")
    };
    for (start, _) in state.index.source()[..offset].rmatch_indices(&key) {
        let rest = state.index.source()[start + key.len()..offset].trim_start();
        let Some(rest) = rest.strip_prefix('=') else {
            continue;
        };
        let rest = rest.trim_start();
        let quote = rest.chars().next()?;
        if !matches!(quote, '\'' | '"') {
            continue;
        }
        let rest = &rest[quote.len_utf8()..];
        if let Some(end) = rest.find(quote) {
            return Some(rest[..end].into());
        }
    }
    None
}

fn escape_xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
        .replace('\r', "&#13;")
        .replace('\n', "&#10;")
        .replace('\t', "&#9;")
}

fn attribute_prefix(state: &State, offset: usize) -> Option<&str> {
    let source = state.index.source();
    if state.document.is_valid() {
        let node = state
            .document
            .ast()
            .node(state.document.ast().element_at(offset)?)?;
        return node
            .attributes
            .iter()
            .find(|attribute| {
                attribute.value_span.start <= offset && offset <= attribute.value_span.end
            })
            .map(|attribute| &source[attribute.value_span.start..offset]);
    }
    let open = source[..offset].rfind('<')?;
    let tag = &source[open + 1..offset];
    if tag.starts_with(['!', '?']) {
        return None;
    }
    let mut quote = None;
    let mut start = 0;
    for (index, ch) in tag.char_indices() {
        if quote == Some(ch) {
            quote = None;
        } else if quote.is_none() && matches!(ch, '\'' | '"') {
            quote = Some(ch);
            start = index + 1;
        }
    }
    quote.map(|_| &tag[start..])
}
