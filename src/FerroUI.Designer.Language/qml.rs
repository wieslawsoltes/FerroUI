//! QML language service over open editor buffers. Analysis uses the production
//! frontend without loading components, executing JavaScript, or fetching imports.
mod model;
mod workspace;
use crate::lsp::{decode, integer, text, uri};
use crate::{require, LanguageError, LineIndex, Range, Result, TextChange};
use ferroui_qml::TypeRegistry;
use ferroui_qml_compiler::{parse_external, Diagnostic, Document, SourceSpan};
use model::{Model, Symbol};
use serde_json::{json, Value};
use std::cell::{Cell, OnceCell, RefCell};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::rc::Rc;
use url::Url;

pub(crate) fn capabilities() -> Value {
    json!({"serverInfo":{"name":"FerroUI QML","version":env!("CARGO_PKG_VERSION")},
        "capabilities":{"positionEncoding":"utf-16","textDocumentSync":{"openClose":true,"change":2},
        "completionProvider":{"triggerCharacters":["."]},"definitionProvider":true,"hoverProvider":true,
        "documentSymbolProvider":true,"foldingRangeProvider":true}})
}

struct Buffer {
    version: i64,
    index: LineIndex,
    parsed: std::result::Result<Cell<Option<Document>>, Diagnostic>,
    semantic: OnceCell<Model>,
}
impl Buffer {
    fn new(index: LineIndex, version: i64) -> Self {
        let source = index.source();
        let parsed = parse_external(source).map(|doc| Cell::new(Some(doc)));
        Self {
            version,
            index,
            parsed,
            semantic: OnceCell::new(),
        }
    }

    // Diagnostics only need the syntax tree. Consume it exactly once when an
    // editor request first needs reference resolution or the document outline.
    fn model(&self) -> Option<&Model> {
        let parsed = self.parsed.as_ref().ok()?;
        Some(self.semantic.get_or_init(|| {
            Model::new(
                self.index.source(),
                parsed.take().expect("first semantic query"),
            )
        }))
    }

    fn range(&self, span: SourceSpan) -> Result<Range> {
        self.index
            .range(ferroui_designer_core::SourceSpan::new(span.start, span.end))
    }
    fn publish(&self, uri: &str) -> Result<Value> {
        let diagnostics = match &self.parsed {
            Ok(_) => vec![],
            Err(e) => vec![
                json!({"range":self.range(e.span)?,"source":"FerroUI QML","severity":1,"message":e.message}),
            ],
        };
        Ok(
            json!({"jsonrpc":"2.0","method":"textDocument/publishDiagnostics","params":{"uri":uri,"version":self.version,"diagnostics":diagnostics}}),
        )
    }
    fn symbol(&self, symbol: &Symbol) -> Result<Value> {
        let children = symbol
            .children
            .iter()
            .map(|s| self.symbol(s))
            .collect::<Result<Vec<_>>>()?;
        Ok(
            json!({"name":symbol.name,"kind":symbol.kind,"detail":symbol.detail,"range":self.range(symbol.span)?,
            "selectionRange":self.range(symbol.selection)?,"children":children}),
        )
    }
    fn folds(&self, symbol: &Symbol, out: &mut Vec<Value>) -> Result<()> {
        let range = self.range(symbol.span)?;
        if range.end.line > range.start.line {
            out.push(json!({"startLine":range.start.line,"endLine":range.end.line,"startCharacter":range.start.character,
                "endCharacter":range.end.character,"kind":"region"}));
        }
        for child in &symbol.children {
            self.folds(child, out)?;
        }
        Ok(())
    }
}

pub(crate) struct QmlService {
    docs: BTreeMap<String, Rc<Buffer>>,
    disk: RefCell<VecDeque<(Url, Rc<Buffer>)>>,
    types: TypeRegistry,
    pub workspace: workspace::Workspace,
}
impl QmlService {
    pub fn new() -> Result<Self> {
        let mut registry = ferroui_qml_controls::ControlRegistry::standard()
            .map_err(|e| LanguageError(e.to_string()))?;
        Ok(Self {
            docs: BTreeMap::new(),
            disk: RefCell::default(),
            workspace: workspace::Workspace::default(),
            types: registry.types_mut().clone(),
        })
    }

    pub fn dispatch(&mut self, method: &str, p: &Value, out: &mut Vec<Value>) -> Result<Value> {
        let doc = &p["textDocument"];
        match method {
            "textDocument/didOpen" => {
                let uri = uri(p)?;
                require(!self.docs.contains_key(uri), "Document is already open")?;
                let source = text(doc, "text")?;
                require(source.len() <= 16 * 1024 * 1024, "Document exceeds 16 MiB")?;
                let buffer = Buffer::new(LineIndex::new(source), integer(doc, "version")?);
                out.push(buffer.publish(uri)?);
                self.docs.insert(uri.into(), Rc::new(buffer));
            }
            "textDocument/didChange" => {
                let uri = uri(p)?;
                let buffer = self.buffer(uri)?;
                let version = integer(doc, "version")?;
                require(version > buffer.version, "Document version must increase")?;
                let mut index = buffer.index.clone();
                for change in decode::<Vec<TextChange>>(&p["contentChanges"])? {
                    index.apply(&change)?;
                }
                let buffer = Buffer::new(index, version);
                out.push(buffer.publish(uri)?);
                self.docs.insert(uri.into(), Rc::new(buffer));
            }
            "textDocument/didClose" => {
                let uri = uri(p)?;
                require(self.docs.remove(uri).is_some(), "Document is not open")?;
                out.push(json!({"jsonrpc":"2.0","method":"textDocument/publishDiagnostics","params":{"uri":uri,"diagnostics":[]}}));
            }
            "textDocument/documentSymbol" | "textDocument/foldingRange" => {
                let buffer = self.buffer(uri(p)?)?;
                let mut items = vec![];
                if let Some(model) = buffer.model() {
                    if method == "textDocument/documentSymbol" {
                        items.push(buffer.symbol(&model.outline)?);
                    } else {
                        buffer.folds(&model.outline, &mut items)?;
                    }
                }
                return Ok(json!(items));
            }
            "textDocument/completion" | "textDocument/definition" | "textDocument/hover" => {
                let uri = uri(p)?;
                let buffer = self.buffer(uri)?;
                let at = buffer.index.offset(decode(&p["position"])?)?;
                if method == "textDocument/completion" {
                    return self.complete(uri, buffer, at);
                }
                return self.lookup(uri, buffer, at, method == "textDocument/hover");
            }
            _ if method.starts_with("$/") => {}
            _ => return Err(LanguageError(format!("Unknown method {method}"))),
        }
        Ok(Value::Null)
    }

    fn buffer(&self, uri: &str) -> Result<&Buffer> {
        self.docs
            .get(uri)
            .map(Rc::as_ref)
            .ok_or_else(|| LanguageError("Document is not open".into()))
    }

    fn component(&self, uri: &str, model: &Model, name: &str) -> Option<(String, Rc<Buffer>)> {
        let (alias, ty) = name
            .split_once('.')
            .map_or((None, name), |(ns, ty)| (Some(ns), ty));
        for dir in workspace::directories(uri, model, alias) {
            let Ok(url) = dir.join(&format!("{ty}.qml")) else {
                continue;
            };
            if let Some(doc) = self.docs.get(url.as_str()) {
                return Some((url.into(), doc.clone()));
            }
            if let Some(source) = self.workspace.read(&url) {
                // Read through the workspace boundary on each query. Comparing the
                // actual text observes same-size saves and atomic replacements even
                // on filesystems with coarse modification times.
                const MAX_CACHED_FILES: usize = 16;
                const MAX_CACHED_SOURCE_BYTES: usize = 2 * 1024 * 1024;
                let mut cache = self.disk.borrow_mut();
                if let Some(index) = cache.iter().position(|(key, _)| *key == url) {
                    let entry = cache.remove(index).expect("cached file");
                    if entry.1.index.source() == source {
                        let buffer = entry.1.clone();
                        cache.push_front(entry);
                        return Some((url.into(), buffer));
                    }
                }
                let buffer = Rc::new(Buffer::new(LineIndex::new(&source), -1));
                if source.len() <= MAX_CACHED_SOURCE_BYTES {
                    let mut bytes = source.len()
                        + cache
                            .iter()
                            .map(|(_, b)| b.index.source().len())
                            .sum::<usize>();
                    while cache.len() >= MAX_CACHED_FILES || bytes > MAX_CACHED_SOURCE_BYTES {
                        bytes -= cache
                            .pop_back()
                            .expect("cached file")
                            .1
                            .index
                            .source()
                            .len();
                    }
                    cache.push_front((url.clone(), buffer.clone()));
                }
                return Some((url.into(), buffer));
            }
        }
        None
    }

    /// Walk source inheritance once per query, stopping on cycles or an invalid
    /// editor buffer. Derived declarations take precedence over their bases.
    fn members(
        &self,
        uri: &str,
        buffer: &Buffer,
        model: &Model,
        index: usize,
        requested: Option<&str>,
    ) -> Result<BTreeMap<String, MemberInfo>> {
        let mut result = BTreeMap::new();
        let mut visited = BTreeSet::new();
        let mut from = uri.to_owned();
        let mut next = Some((buffer, model, index));
        let mut base;
        while let Some((buffer, model, index)) = next.take() {
            let node = &model.nodes[index];
            if let Some(name) = requested {
                if let Some((detail, span)) = node.declarations.get(name) {
                    result.insert(
                        name.to_owned(),
                        MemberInfo {
                            detail: detail.clone(),
                            kind: kind(detail),
                            location: Some((from, buffer.range(*span)?)),
                        },
                    );
                    return Ok(result);
                }
            } else {
                // Completion does not expose source ranges. In particular this
                // also handles the temporary model of a repaired partial edit.
                for (name, (detail, _)) in &node.declarations {
                    result.entry(name.clone()).or_insert_with(|| MemberInfo {
                        detail: detail.clone(),
                        kind: kind(detail),
                        location: None,
                    });
                }
            }
            if visited.len() >= 64 {
                break;
            }
            if let Some((path, doc)) = self.component(&from, model, &node.ty) {
                if !visited.insert(path.clone()) {
                    break;
                }
                base = doc;
                from = path;
                next = base.model().map(|m| (base.as_ref(), m, 0));
                continue;
            }
            if let Ok((_, _, def)) = self.types.resolve(&model.imports, &node.ty) {
                let mut insert = |name: &str, k, detail: &str| {
                    if requested.is_some_and(|wanted| wanted != name) {
                        return;
                    }
                    result.entry(name.to_owned()).or_insert_with(|| MemberInfo {
                        detail: detail.into(),
                        kind: k,
                        location: None,
                    });
                };
                for (name, ty) in def.property_types() {
                    if !name.contains('.') && !name.starts_with('$') {
                        insert(name, 10, ty.trim_start_matches("$qml:"));
                    }
                }
                for name in def.signals() {
                    insert(name, 23, "signal");
                }
                for name in def.methods() {
                    insert(name, 2, "method");
                }
            }
        }
        Ok(result)
    }

    fn lookup(&self, uri: &str, buffer: &Buffer, at: usize, hover: bool) -> Result<Value> {
        let Some(model) = buffer.model() else {
            return Ok(Value::Null);
        };
        let Some(owner) = model.owner(at) else {
            return Ok(Value::Null);
        };
        let node = &model.nodes[owner];
        let selection = word(buffer.index.source(), at);
        let name = &buffer.index.source()[selection.start..selection.end];
        let result = if node.type_span.start <= at && at < node.type_span.end {
            if let Some((_, span)) = model.nodes[node.scope]
                .declarations
                .get(&node.ty)
                .filter(|(d, _)| d == "component")
            {
                Some((uri.to_owned(), buffer.range(*span)?, node.ty.clone()))
            } else if let Some((path, base)) = self.component(uri, model, &node.ty) {
                let Some(base_model) = base.model() else {
                    return Ok(Value::Null);
                };
                Some((
                    path,
                    base.range(base_model.outline.selection)?,
                    node.ty.clone(),
                ))
            } else {
                return Ok(if hover {
                    self.types.resolve(&model.imports, &node.ty).ok().map_or(Value::Null, |(module, ty, _)| {
                        json!({"contents":{"kind":"plaintext","value":format!("{module}.{ty}")},"range":buffer.range(node.type_span).ok()})
                    })
                } else {
                    Value::Null
                });
            }
        } else {
            let Some(target) = model.target(owner, name, at) else {
                return Ok(Value::Null);
            };
            match target {
                model::Target::Id(index) => {
                    let id = &model.nodes[index];
                    id.id
                        .as_ref()
                        .map(|(_, s)| {
                            buffer
                                .range(*s)
                                .map(|r| (uri.to_owned(), r, format!("{} {name}", id.ty)))
                        })
                        .transpose()?
                }
                model::Target::Member {
                    owner: index,
                    lexical,
                } => {
                    let mut info = self
                        .members(uri, buffer, model, index, Some(name))?
                        .remove(name);
                    let scope = model.nodes[index].scope;
                    if info.is_none() && lexical && index != scope {
                        info = self
                            .members(uri, buffer, model, scope, Some(name))?
                            .remove(name);
                    }
                    let Some(info) = info else {
                        return Ok(Value::Null);
                    };
                    let detail = format!("{} {name}", info.detail);
                    if hover {
                        return Ok(
                            json!({"contents":{"kind":"plaintext","value":detail},"range":buffer.range(selection)?}),
                        );
                    }
                    info.location.map(|(uri, r)| (uri, r, detail))
                }
            }
        };
        let Some((uri, range, detail)) = result else {
            return Ok(Value::Null);
        };
        Ok(if hover {
            json!({"contents":{"kind":"plaintext","value":detail},"range":buffer.range(selection)?})
        } else {
            json!({"uri":uri,"range":range})
        })
    }

    fn complete(&self, uri: &str, buffer: &Buffer, at: usize) -> Result<Value> {
        let source = buffer.index.source();
        let line = source[..at]
            .rsplit(['\n', '\r'])
            .next()
            .unwrap_or("")
            .trim_start();
        let mut items = BTreeMap::new();
        if line
            .strip_prefix("import")
            .is_some_and(|tail| tail.starts_with(char::is_whitespace))
        {
            for module in self.types.modules() {
                item(&mut items, module, 9, "QML module");
            }
            return Ok(Value::Array(items.into_values().collect()));
        }
        let token = word(source, at);
        let prefix = &source[..token.start];
        let qualifier = prefix.strip_suffix('.').map(|before| {
            let id = word(before, before.len());
            &before[id.start..id.end]
        });
        // Complete an unfinished member declaration or JS/namespace "." by filling
        // the token under the cursor in a temporary parse. Keep the editor text intact.
        let repair = || {
            for stub in [
                "qmlCompletionPlaceholder",
                "qmlCompletionPlaceholder: null",
                "Item {}",
            ] {
                let mut fixed = source.to_owned();
                fixed.replace_range(token.start..token.end, stub);
                if let Ok(doc) = parse_external(&fixed) {
                    return Some(Model::new(&fixed, doc));
                }
            }
            None
        };
        let repaired;
        let model = if let Some(m) = buffer.model() {
            m
        } else {
            repaired = repair();
            let Some(m) = &repaired else {
                return Ok(json!([]));
            };
            m
        };
        let owner = model.owner(token.start);
        let mut native_owner = owner;
        if let (Some(receiver), Some(owner)) = (qualifier, owner) {
            native_owner = model.receiver(owner, receiver);
        }
        if let Some(index) = native_owner {
            let node = &model.nodes[index];
            for (name, info) in self.members(uri, buffer, model, index, None)? {
                item(&mut items, &name, info.kind, &info.detail);
            }
            if qualifier.is_none() {
                for id in model.nodes.iter().filter(|n| n.scope == node.scope) {
                    if let Some((name, _)) = &id.id {
                        item(&mut items, name, 6, &id.ty);
                    }
                }
            }
        }
        for import in &model.imports {
            if let Some(alias) = import.alias.as_deref() {
                if qualifier.is_none() {
                    item(&mut items, alias, 9, &import.uri);
                }
            }
            if import.alias.as_deref() != qualifier {
                continue;
            }
            for ty in self.types.exported_types(&import.uri) {
                item(&mut items, ty, 7, &import.uri);
            }
        }
        let dirs = workspace::directories(uri, model, qualifier);
        for dir in &dirs {
            for ty in self.workspace.components(dir) {
                item(&mut items, &ty, 7, "QML component");
            }
        }
        for path in self.docs.keys().filter_map(|p| Url::parse(p).ok()) {
            if path.join(".").ok().is_some_and(|dir| dirs.contains(&dir)) {
                if let Some(ty) = path
                    .path_segments()
                    .and_then(|mut s| s.next_back())
                    .and_then(|f| f.strip_suffix(".qml"))
                {
                    item(&mut items, ty, 7, "QML component");
                }
            }
        }
        Ok(Value::Array(items.into_values().collect()))
    }
}

struct MemberInfo {
    detail: String,
    kind: u32,
    location: Option<(String, Range)>,
}

fn kind(detail: &str) -> u32 {
    match detail {
        "function" => 2,
        "signal" => 23,
        "component" => 7,
        "enum" => 13,
        _ => 10,
    }
}
fn item(items: &mut BTreeMap<String, Value>, name: &str, kind: u32, detail: &str) {
    items.insert(
        name.into(),
        json!({"label":name,"kind":kind,"detail":detail}),
    );
}
fn word(source: &str, at: usize) -> SourceSpan {
    let ident = |c: char| c == '_' || c == '$' || c.is_alphanumeric();
    let start = source[..at].trim_end_matches(ident).len();
    let end = source.len() - source[at..].trim_start_matches(ident).len();
    SourceSpan { start, end }
}
