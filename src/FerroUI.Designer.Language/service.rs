use crate::{
    require, FormattingOptions, LanguageError, LineIndex, Range, Result, Schema, TextChange,
    TextEdit,
};
use ferroui_designer_core::{Document, NodeKind, SourceSpan};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

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
#[derive(Clone, Debug, Serialize, Deserialize)]
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
#[derive(Clone)]
pub struct LanguageService {
    schema: Schema,
    documents: BTreeMap<String, State>,
    workspaces: BTreeMap<std::path::PathBuf, ferroui_designer_project::ProjectWorkspace>,
    declarations: BTreeMap<String, Vec<(String, Occurrence)>>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
enum SymbolKind {
    Name(u64),
    Resource,
}
#[derive(Clone)]
struct ResourceEdge {
    node: u64,
    source: Option<String>,
}
#[derive(Clone)]
struct Occurrence {
    name: String,
    span: SourceSpan,
    kind: SymbolKind,
    declaration: bool,
    node: u64,
    quote: Option<char>,
}
impl LanguageService {
    pub fn new(schema: Schema) -> Self {
        Self {
            schema,
            documents: BTreeMap::new(),
            workspaces: BTreeMap::new(),
            declarations: BTreeMap::new(),
        }
    }
    pub fn schema(&self) -> &Schema {
        &self.schema
    }
    pub fn schema_mut(&mut self) -> &mut Schema {
        &mut self.schema
    }
    pub fn document(&self, uri: &str) -> Option<&Document> {
        self.documents.get(uri).map(|s| &s.document)
    }
    pub fn version(&self, uri: &str) -> Option<i64> {
        self.documents.get(uri).map(|s| s.version)
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
    pub fn close(&mut self, uri: &str) -> Result<()> {
        require(self.documents.contains_key(uri), "Document is not indexed")?;
        let project_entry = self
            .workspaces
            .values()
            .flat_map(|workspace| &workspace.solution.projects)
            .flat_map(|project| &project.documents)
            .find(|entry| entry.uri == uri)
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
            self.open(uri, source, -1)?;
            self.documents.get_mut(uri).unwrap().opened = false;
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
        let mut state = self.state(uri)?.clone();
        require(version > state.version, "Document version must increase")?;
        require(changes.len() <= 4096, "Too many text changes")?;
        for change in changes {
            state.index.apply(change)?;
        }
        state
            .document
            .update_source("Incremental text edits", state.index.source());
        state.version = version;
        state.opened = true;
        state.symbols = occurrences(&state.document);
        state.resource_edges = resource_edges(&state.document);
        self.set_state(uri.into(), Some(state));
        Ok(())
    }
    pub fn index_solution(
        &mut self,
        solution: &ferroui_designer_project::Solution,
    ) -> Result<usize> {
        let mut added = 0;
        let mut staged = Vec::new();
        for entry in solution
            .projects
            .iter()
            .flat_map(|p| &p.documents)
            .filter(|d| d.kind == ferroui_designer_project::DocumentKind::Xaml)
        {
            if self.documents.get(&entry.uri).is_some_and(|s| s.opened) {
                continue;
            }
            require(
                std::fs::metadata(&entry.path)
                    .map_err(|e| LanguageError(e.to_string()))?
                    .len()
                    <= 16 * 1024 * 1024,
                "Indexed file exceeds 16 MiB",
            )?;
            let source =
                std::fs::read_to_string(&entry.path).map_err(|e| LanguageError(e.to_string()))?;
            staged.push((entry.uri.clone(), source));
        }
        for (uri, source) in staged {
            self.open(&uri, source, -1)?;
            self.documents.get_mut(&uri).unwrap().opened = false;
            added += 1;
        }
        self.workspaces.insert(
            solution.root.clone(),
            ferroui_designer_project::ProjectWorkspace::new(solution.clone()),
        );
        Ok(added)
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
        let retained: BTreeSet<_> = self
            .workspaces
            .values()
            .flat_map(|workspace| &workspace.solution.projects)
            .flat_map(|project| &project.documents)
            .map(|entry| entry.uri.as_str())
            .collect();
        let obsolete: BTreeSet<_> = removed
            .solution
            .projects
            .iter()
            .flat_map(|project| &project.documents)
            .filter(|entry| {
                !retained.contains(entry.uri.as_str())
                    && self
                        .documents
                        .get(&entry.uri)
                        .is_some_and(|state| !state.opened)
            })
            .map(|entry| entry.uri.clone())
            .collect();
        let count = obsolete.len();
        for uri in obsolete {
            self.set_state(uri, None);
        }
        count
    }
    pub fn diagnostics(&self, uri: &str) -> Result<Vec<Diagnostic>> {
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
                if let Some((owner, property)) = node.local_name().split_once('.') {
                    if self
                        .schema
                        .type_info(owner, node.namespace_uri.as_deref())
                        .is_some()
                        && !self
                            .schema
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
            let known = self
                .schema
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
                let expression =
                    attribute.value.starts_with('{') && !attribute.value.starts_with("{}");
                if expression {
                    if let Err(error) = attribute.markup() {
                        out.push(diagnostic_at(
                            state,
                            attribute.value_span,
                            1,
                            "markup",
                            &error,
                        )?);
                    }
                }
                if let Some(property) = self.schema.property(
                    &node.name,
                    attribute
                        .namespace_uri
                        .as_deref()
                        .or(node.namespace_uri.as_deref()),
                    &attribute.name,
                ) {
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
                } else if known && !attribute.name.contains(':') {
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
                        for item in self.schema.extensions.values() {
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
                    } else {
                        let name = extension
                            .split(|c: char| c.is_whitespace() || c == ',')
                            .next()
                            .unwrap_or("");
                        if let Some(info) = self.schema.extensions.get(name) {
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
                                            &self.schema,
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
                } else if let Some(info) =
                    self.schema
                        .property(tag_name, namespace.as_deref(), property)
                {
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
                for property in self
                    .schema
                    .properties(tag_name, namespace.as_deref())
                    .values()
                {
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
                for info in self.schema.types.values() {
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
            } else {
                for info in self.schema.types.values().filter(|t| {
                    t.constructible
                        && namespace.as_ref().is_none_or(|ns| {
                            ns == &t.namespace
                                || ns.starts_with("using:FerroUI")
                                || ns.starts_with("clr-namespace:FerroUI")
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
            for info in self.schema.types.values().filter(|t| t.constructible) {
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
        let state = self.state(uri)?;
        state.index.position(offset)?;
        if !state.document.is_valid() {
            return Ok(None);
        }
        if let Some((property, span)) = crate::binding::at(&self.schema, &state.document, offset) {
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
                    if let Some(info) = self.schema.property(
                        &node.name,
                        node.namespace_uri.as_deref(),
                        &attribute.name,
                    ) {
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
            if let Some(info) = self
                .schema
                .type_info(&node.name, node.namespace_uri.as_deref())
            {
                return Ok(Some(Hover {
                    contents: format!("{}\n\n{}", info.name, info.documentation),
                    range: state.index.range(node.name_span)?,
                }));
            }
        }
        Ok(None)
    }
    pub fn signature_help(&self, uri: &str, offset: usize) -> Result<Option<SignatureHelp>> {
        let state = self.state(uri)?;
        state.index.position(offset)?;
        let Some(prefix) = attribute_prefix(state, offset) else {
            return Ok(None);
        };
        let Some(active_extension) = crate::references::active_extension(prefix) else {
            return Ok(None);
        };
        let name = active_extension.name;
        let Some(info) = self.schema.extensions.get(&name) else {
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
    pub fn definitions(&self, uri: &str, offset: usize) -> Result<Vec<Location>> {
        let state = self.state(uri)?;
        state.index.position(offset)?;
        if let Some(occurrence) = state
            .symbols
            .iter()
            .find(|o| o.span.contains(offset) || o.span.end == offset)
            .cloned()
        {
            return self
                .resolve(uri, &occurrence)
                .into_iter()
                .map(|(uri, o)| self.location(&uri, o.span))
                .collect();
        }
        if state.document.is_valid() {
            if let Some((property, _)) = crate::binding::at(&self.schema, &state.document, offset) {
                return Ok(property.definition.into_iter().collect());
            }
            if let Some(node) = state
                .document
                .ast()
                .element_at(offset)
                .and_then(|id| state.document.ast().node(id))
            {
                if node.name_span.contains(offset) {
                    return Ok(self
                        .schema
                        .type_info(&node.name, node.namespace_uri.as_deref())
                        .and_then(|t| t.definition.clone())
                        .into_iter()
                        .collect());
                }
                for attribute in &node.attributes {
                    if attribute.name_span.contains(offset) {
                        return Ok(self
                            .schema
                            .property(
                                &node.name,
                                attribute
                                    .namespace_uri
                                    .as_deref()
                                    .or(node.namespace_uri.as_deref()),
                                &attribute.name,
                            )
                            .and_then(|property| property.definition)
                            .into_iter()
                            .collect());
                    }
                    if attribute.name == "Source" && attribute.value_span.contains(offset) {
                        if let Ok(target) = self.resource_uri(uri, &attribute.value) {
                            return Ok(vec![Location {
                                uri: target,
                                range: Range::default(),
                            }]);
                        }
                    }
                }
            }
        }
        Ok(Vec::new())
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
                if resolved.len() == 1
                    && resolved[0].0 == target.0
                    && resolved[0].1.span == target.1.span
                {
                    result.push(self.location(candidate, occurrence.span)?);
                }
            }
        }
        Ok(result)
    }
    pub fn rename(&self, uri: &str, offset: usize, new_name: &str) -> Result<WorkspaceEdit> {
        let target = self
            .target(uri, offset)?
            .ok_or_else(|| LanguageError("No unambiguous symbol at the caret".into()))?;
        require(
            match target.1.kind {
                SymbolKind::Name(_) => crate::references::identifier(new_name),
                SymbolKind::Resource => {
                    !new_name.is_empty()
                        && new_name.chars().all(|c| {
                            matches!(c, '\t' | '\n' | '\r')
                                || c >= ' ' && c != '\u{fffe}' && c != '\u{ffff}'
                        })
                }
            },
            "Names must be XAML identifiers; resource keys must be nonempty XML text",
        )?;
        let target_state = self.state(&target.0)?;
        require(
            !target_state.symbols.iter().any(|o| {
                o.declaration
                    && o.name == new_name
                    && o.kind == target.1.kind
                    && o.span != target.1.span
                    && (o.kind != SymbolKind::Resource
                        || resource_scope(&target_state.document, o.node)
                            == resource_scope(&target_state.document, target.1.node))
            }),
            "A declaration with this name already exists in scope",
        )?;
        let mut grouped = BTreeMap::<String, Vec<TextEdit>>::new();
        for (uri, state) in &self.documents {
            for occurrence in &state.symbols {
                let resolved = self.resolve(uri, occurrence);
                if resolved.len() != 1
                    || resolved[0].0 != target.0
                    || resolved[0].1.span != target.1.span
                {
                    continue;
                }
                let replacement =
                    if occurrence.kind == SymbolKind::Resource && !occurrence.declaration {
                        let quote = occurrence.quote.or_else(|| {
                            new_name
                                .chars()
                                .any(|c| {
                                    c.is_whitespace()
                                        || matches!(c, '{' | '}' | ',' | '=' | '\'' | '"' | '\\')
                                })
                                .then_some('\'')
                        });
                        if let Some(quote) = quote {
                            let value = new_name
                                .replace('\\', "\\\\")
                                .replace(quote, &format!("\\{quote}"));
                            if occurrence.quote.is_some() {
                                value
                            } else {
                                format!("{quote}{value}{quote}")
                            }
                        } else {
                            new_name.into()
                        }
                    } else {
                        new_name.into()
                    };
                grouped.entry(uri.clone()).or_default().push(TextEdit {
                    range: state.index.range(occurrence.span)?,
                    new_text: escape_xml(&replacement),
                });
            }
        }
        let document_changes = grouped
            .into_iter()
            .map(|(uri, edits)| DocumentEdits {
                version: self.documents[&uri].version,
                uri,
                edits,
            })
            .collect();
        Ok(WorkspaceEdit { document_changes })
    }
    pub fn apply_workspace_edit(&mut self, edit: &WorkspaceEdit) -> Result<()> {
        let mut staged = Vec::new();
        let mut seen = BTreeSet::new();
        for changes in &edit.document_changes {
            require(seen.insert(changes.uri.clone()), "Duplicate document edit")?;
            let mut state = self.state(&changes.uri)?.clone();
            require(state.version == changes.version, "Workspace edit is stale")?;
            let source = apply_text_edits(state.index.source(), &changes.edits)?;
            state
                .document
                .update_source("Workspace refactoring", &source);
            state.index = LineIndex::new(&source);
            state.symbols = occurrences(&state.document);
            state.resource_edges = resource_edges(&state.document);
            state.version = state
                .version
                .checked_add(1)
                .ok_or_else(|| LanguageError("Version exhausted".into()))?;
            staged.push((changes.uri.clone(), state));
        }
        for (uri, state) in staged {
            self.set_state(uri, Some(state));
        }
        Ok(())
    }
    pub fn format(&self, uri: &str, options: FormattingOptions) -> Result<Vec<TextEdit>> {
        crate::formatting::format(&self.state(uri)?.document, options)
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
            for part in state.index.source()[span.range()].split_inclusive('\n') {
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
        for uri in self.documents.keys() {
            let mut pending = self.symbols(uri)?;
            while let Some(symbol) = pending.pop() {
                pending.extend(symbol.children.clone());
                if symbol.name.to_lowercase().contains(&query.to_lowercase()) {
                    out.push((uri.clone(), symbol));
                }
            }
        }
        Ok(out)
    }
    pub fn resource_uri(&self, base: &str, reference: &str) -> Result<String> {
        let resolved = ferroui_designer_project::resolve_uri(base, reference)?;
        if resolved.starts_with("ferres://") || resolved.starts_with("resm:") {
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
        Ok(resolved)
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
        let state = self
            .documents
            .get_mut(uri)
            .ok_or_else(|| LanguageError("Document is not indexed".into()))?;
        require(
            text.is_none_or(|text| text == state.document.source()),
            "Saved text differs from the synchronized document",
        )?;
        state.document.mark_saved();
        Ok(())
    }
    pub fn rename_uri(&mut self, old: &str, new: &str) -> Result<()> {
        require(
            !self.documents.contains_key(new),
            "Rename destination is already indexed",
        )?;
        if let Some(state) = self.documents.get(old).cloned() {
            self.set_state(old.into(), None);
            self.set_state(new.into(), Some(state));
        }
        if let Ok(path) = ferroui_designer_project::file_path(new) {
            for entry in self
                .workspaces
                .values_mut()
                .flat_map(|workspace| &mut workspace.solution.projects)
                .flat_map(|project| &mut project.documents)
            {
                if entry.uri == old {
                    entry.uri = new.into();
                    entry.path = path.clone();
                }
            }
        }
        Ok(())
    }
    pub fn file_rename_edits(&self, old: &str, new: &str) -> Result<WorkspaceEdit> {
        let mut result = WorkspaceEdit::default();
        for (uri, state) in &self.documents {
            if !state.document.is_valid() {
                continue;
            }
            let mut edits = Vec::new();
            for node in state.document.ast().elements() {
                for attribute in &node.attributes {
                    if attribute.name == "Source"
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
            .get(uri)
            .ok_or_else(|| LanguageError(format!("Document is not indexed: {uri}")))
    }
    fn location(&self, uri: &str, span: SourceSpan) -> Result<Location> {
        Ok(Location {
            uri: uri.into(),
            range: self.state(uri)?.index.range(span)?,
        })
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
        if reference.declaration {
            return vec![(uri.into(), reference.clone())];
        }
        let mut found: Vec<_> = self
            .declarations
            .get(&reference.name)
            .into_iter()
            .flatten()
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
            let mut seen = BTreeSet::new();
            if let Some(state) = self.documents.get(uri) {
                let mut current = Some(reference.node);
                while let Some(scope) = current {
                    let matches = self.resources_at(uri, scope, &reference.name, &mut seen);
                    if !matches.is_empty() {
                        return matches;
                    }
                    current = state
                        .document
                        .ast()
                        .node(scope)
                        .and_then(|node| node.parent);
                }
            }
            // Standalone dictionaries and application resources may provide global resources.
            // Never expose private resources belonging to another control/view as globals.
            let candidates: BTreeSet<_> = found
                .iter()
                .filter_map(|(candidate, _)| {
                    let state = self.documents.get(candidate)?;
                    let root = state.document.ast().node(state.document.ast().root())?;
                    (candidate != uri
                        && matches!(
                            root.local_name(),
                            "ResourceDictionary" | "Application" | "Styles" | "Theme"
                        ))
                    .then_some(candidate.clone())
                })
                .collect();
            found.clear();
            for candidate in candidates {
                let root = self.documents[&candidate].document.ast().root();
                found.extend(self.resources_at(&candidate, root, &reference.name, &mut seen));
            }
        }
        found
    }
    fn resources_at(
        &self,
        uri: &str,
        scope: u64,
        name: &str,
        seen: &mut BTreeSet<(String, u64)>,
    ) -> Vec<(String, Occurrence)> {
        if !seen.insert((uri.into(), scope)) {
            return Vec::new();
        }
        let Some(state) = self
            .documents
            .get(uri)
            .filter(|state| state.document.is_valid())
        else {
            return Vec::new();
        };
        let found: Vec<_> = self
            .declarations
            .get(name)
            .into_iter()
            .flatten()
            .filter(|(candidate, occurrence)| {
                candidate == uri
                    && occurrence.kind == SymbolKind::Resource
                    && resource_scope(&state.document, occurrence.node) == scope
            })
            .cloned()
            .collect();
        if !found.is_empty() {
            return found;
        }
        for edge in state.resource_edges.get(&scope).into_iter().flatten().rev() {
            let (target, next) = if let Some(source) = &edge.source {
                let Ok(target) = self.resource_uri(uri, source) else {
                    continue;
                };
                let Some(imported) = self.documents.get(&target) else {
                    continue;
                };
                (target, imported.document.ast().root())
            } else {
                (uri.into(), edge.node)
            };
            let found = self.resources_at(&target, next, name, seen);
            if !found.is_empty() {
                return found;
            }
        }
        Vec::new()
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
            quote: None,
        })
        .collect();
    for node in document.ast().elements() {
        for attribute in &node.attributes {
            if attribute.name.rsplit(':').next() == Some("Key")
                && attribute.namespace_uri.as_deref()
                    == Some("http://schemas.microsoft.com/winfx/2006/xaml")
            {
                out.push(Occurrence {
                    name: attribute.value.clone(),
                    span: attribute.value_span,
                    kind: SymbolKind::Resource,
                    declaration: true,
                    node: node.id,
                    quote: None,
                });
            }
            let raw = &document.source()[attribute.value_span.range()];
            if raw.starts_with("{}") {
                continue;
            }
            for argument in crate::references::markup_arguments(raw) {
                if matches!(
                    argument.extension.as_str(),
                    "StaticResource" | "DynamicResource"
                ) && argument
                    .name
                    .as_deref()
                    .is_none_or(|name| name == "ResourceKey")
                {
                    out.push(Occurrence {
                        name: argument.value,
                        span: SourceSpan::new(
                            attribute.value_span.start + argument.span.start,
                            attribute.value_span.start + argument.span.end,
                        ),
                        kind: SymbolKind::Resource,
                        declaration: false,
                        node: node.id,
                        quote: argument.quote,
                    });
                }
            }
        }
    }
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
        let source = match node.local_name() {
            "ResourceInclude" | "StyleInclude" => match node.attribute("Source") {
                Some(source) => Some(source.to_owned()),
                None => continue,
            },
            "ResourceDictionary"
                if node.id != document.ast().root()
                    && !node
                        .attributes
                        .iter()
                        .any(|attribute| attribute.name.ends_with(":Key"))
                    && node
                        .parent
                        .and_then(|id| document.ast().node(id))
                        .is_some_and(|parent| {
                            parent.name.ends_with(".Resources")
                                || parent.name.ends_with(".MergedDictionaries")
                        }) =>
            {
                None
            }
            _ => continue,
        };
        edges
            .entry(resource_scope(document, node.id))
            .or_default()
            .push(ResourceEdge {
                node: node.id,
                source,
            });
    }
    for children in edges.values_mut() {
        children.sort_by_key(|edge| document.ast().node(edge.node).unwrap().span.start);
    }
    edges
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
    let mut output = source.to_owned();
    for (span, text) in patches.into_iter().rev() {
        output.replace_range(span.range(), text);
    }
    require(output.len() <= 16 * 1024 * 1024, "Document exceeds 16 MiB")?;
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
