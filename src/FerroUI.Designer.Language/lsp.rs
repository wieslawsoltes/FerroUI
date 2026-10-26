//! Framed JSON-RPC over stdio. Only this adapter uses LSP wire types; native callers use the
//! same portable service directly. No network listener or external language process is started.
use crate::{
    FormattingOptions, LanguageError, LanguageService, Position, Result, TextChange, WorkspaceEdit,
};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::io::{self, BufRead, Read, Write};
use std::sync::{mpsc, Arc, Mutex};

pub struct LspServer {
    pub service: LanguageService,
    initialized: bool,
    shutdown: bool,
    exit: bool,
    cancelled: Arc<Mutex<BTreeSet<String>>>,
}
impl Default for LspServer {
    fn default() -> Self {
        Self {
            service: LanguageService::default(),
            initialized: false,
            shutdown: false,
            exit: false,
            cancelled: Arc::new(Mutex::new(BTreeSet::new())),
        }
    }
}
impl LspServer {
    pub fn handle(&mut self, message: Value) -> Vec<Value> {
        let id = message.get("id").cloned();
        let method = message["method"].as_str().unwrap_or("");
        let params = message.get("params").cloned().unwrap_or_else(|| json!({}));
        if message["jsonrpc"] != "2.0"
            || method.is_empty()
            || id
                .as_ref()
                .is_some_and(|v| !v.is_string() && !v.is_number())
        {
            return vec![error(
                id.unwrap_or(Value::Null),
                -32600,
                "Invalid JSON-RPC request",
            )];
        }
        if method == "$/cancelRequest" {
            self.cancelled
                .lock()
                .unwrap()
                .insert(params["id"].to_string());
            return Vec::new();
        }
        if method == "exit" {
            self.exit = true;
            return Vec::new();
        }
        if method == "initialized" {
            return Vec::new();
        }
        if let Some(id) = &id {
            if self.cancelled.lock().unwrap().remove(&id.to_string()) {
                return vec![error(id.clone(), -32800, "Request cancelled")];
            }
        }
        if method != "initialize" && !self.initialized {
            return id
                .map(|id| vec![error(id, -32002, "Server not initialized")])
                .unwrap_or_default();
        }
        if self.shutdown {
            return id
                .map(|id| vec![error(id, -32600, "Server has shut down")])
                .unwrap_or_default();
        }
        let mut notifications = Vec::new();
        let result = self.dispatch(method, &params, &mut notifications);
        if let Some(id) = id {
            if self.cancelled.lock().unwrap().remove(&id.to_string()) {
                notifications.push(error(id, -32800, "Request cancelled"));
            } else {
                notifications.push(match result {
                    Ok(value) => json!({"jsonrpc":"2.0","id":id,"result":value}),
                    Err(e) => error(
                        id,
                        if e.0.starts_with("Unknown method") {
                            -32601
                        } else {
                            -32602
                        },
                        &e.0,
                    ),
                });
            }
        } else if let Err(e) = result {
            notifications.push(json!({"jsonrpc":"2.0","method":"window/logMessage","params":{"type":1,"message":e.0}}));
        }
        notifications
    }
    fn dispatch(
        &mut self,
        method: &str,
        params: &Value,
        notifications: &mut Vec<Value>,
    ) -> Result<Value> {
        match method {
            "initialize" => self.initialize(params, notifications),
            "shutdown" => {
                self.shutdown = true;
                Ok(Value::Null)
            }
            "textDocument/didOpen" => {
                let document = &params["textDocument"];
                let uri = text(document, "uri")?;
                self.service
                    .open(uri, text(document, "text")?, integer(document, "version")?)?;
                self.publish(uri, notifications)?;
                Ok(Value::Null)
            }
            "textDocument/didChange" => {
                let document = &params["textDocument"];
                let uri = text(document, "uri")?;
                let changes: Vec<TextChange> = decode(&params["contentChanges"])?;
                self.service
                    .change(uri, integer(document, "version")?, &changes)?;
                self.publish(uri, notifications)?;
                Ok(Value::Null)
            }
            "textDocument/didClose" => {
                let uri = uri(params)?;
                self.service.close(uri)?;
                notifications.push(json!({
                    "jsonrpc": "2.0", "method": "textDocument/publishDiagnostics",
                    "params": { "uri": uri, "diagnostics": [] }
                }));
                Ok(Value::Null)
            }
            "textDocument/didSave" => {
                let uri = uri(params)?;
                self.service.mark_saved(uri, params["text"].as_str())?;
                self.publish(uri, notifications)?;
                Ok(Value::Null)
            }
            "textDocument/completion" => {
                let (uri, offset) = self.position(params)?;
                let items: Vec<_> = self
                    .service
                    .completions(uri, offset)?
                    .into_iter()
                    .map(|item| {
                        json!({
                            "label": item.label, "detail": item.detail,
                            "documentation": { "kind": "markdown", "value": item.documentation },
                            "kind": item.kind, "textEdit": item.text_edit
                        })
                    })
                    .collect();
                Ok(json!(items))
            }
            "textDocument/hover" => {
                let (uri, offset) = self.position(params)?;
                Ok(self.service.hover(uri, offset)?.map_or(Value::Null, |hover| json!({
                    "contents": { "kind": "markdown", "value": hover.contents }, "range": hover.range
                })))
            }
            "textDocument/signatureHelp" => {
                let (uri, offset) = self.position(params)?;
                Ok(self.service.signature_help(uri, offset)?.map_or(Value::Null, |help| json!({
                    "activeSignature": help.active_signature, "activeParameter": help.active_parameter,
                    "signatures": help.signatures.into_iter().map(|signature| json!({
                        "label": signature.label, "documentation": signature.documentation,
                        "parameters": signature.parameters.into_iter().map(|label| json!({"label": label})).collect::<Vec<_>>()
                    })).collect::<Vec<_>>()
                })))
            }
            "textDocument/definition" => {
                let (uri, offset) = self.position(params)?;
                Ok(json!(self.service.definitions(uri, offset)?))
            }
            "textDocument/references" => {
                let (uri, offset) = self.position(params)?;
                let include = params["context"]["includeDeclaration"]
                    .as_bool()
                    .unwrap_or(false);
                Ok(json!(self
                    .service
                    .references_with_declarations(uri, offset, include)?))
            }
            "textDocument/prepareRename" => {
                let (uri, offset) = self.position(params)?;
                Ok(self.service.prepare_rename(uri, offset)?.map_or(
                    Value::Null,
                    |(range, name)| {
                        json!({
                            "range": range, "placeholder": name
                        })
                    },
                ))
            }
            "textDocument/rename" => {
                let (uri, offset) = self.position(params)?;
                Ok(workspace_edit(self.service.rename(
                    uri,
                    offset,
                    text(params, "newName")?,
                )?))
            }
            "textDocument/formatting" => {
                let options: FormattingOptions = decode(&params["options"])?;
                Ok(json!(self.service.format(uri(params)?, options)?))
            }
            "textDocument/documentSymbol" => Ok(json!(self.service.symbols(uri(params)?)?)),
            "textDocument/foldingRange" => Ok(json!(self.service.folding(uri(params)?)?)),
            "textDocument/semanticTokens/full" => {
                let mut data = Vec::new();
                let mut line = 0;
                let mut start = 0;
                for token in self.service.semantic_tokens(uri(params)?)? {
                    data.extend([
                        token.line - line,
                        if token.line == line {
                            token.start - start
                        } else {
                            token.start
                        },
                        token.length,
                        token.token_type,
                        token.modifiers,
                    ]);
                    line = token.line;
                    start = token.start;
                }
                Ok(json!({ "data": data }))
            }
            "workspace/symbol" => {
                let symbols: Vec<_> = self
                    .service
                    .workspace_symbols(text(params, "query")?)?
                    .into_iter()
                    .map(|(uri, symbol)| {
                        json!({
                            "name": symbol.name, "kind": symbol.kind,
                            "location": { "uri": uri, "range": symbol.selection_range }
                        })
                    })
                    .collect();
                Ok(json!(symbols))
            }
            "workspace/didChangeConfiguration" => {
                if let Some(types) = params["settings"]["ferroui"]["types"].as_array() {
                    let registrations = types.iter().map(decode).collect::<Result<Vec<_>>>()?;
                    for registration in registrations {
                        self.service.schema_mut().register(registration);
                    }
                }
                Ok(Value::Null)
            }
            "workspace/didChangeWorkspaceFolders" => {
                if let Some(folders) = params["event"]["removed"].as_array() {
                    for folder in folders {
                        self.service
                            .remove_solution(ferroui_designer_project::file_path(text(
                                folder, "uri",
                            )?)?);
                    }
                }
                if let Some(folders) = params["event"]["added"].as_array() {
                    for folder in folders {
                        let root = ferroui_designer_project::file_path(text(folder, "uri")?)?;
                        let solution = ferroui_designer_project::Solution::discover(root)?;
                        self.service.index_solution(&solution)?;
                    }
                }
                Ok(Value::Null)
            }
            "workspace/willRenameFiles" => self.will_rename_files(params),
            "workspace/didRenameFiles" => {
                for file in params["files"]
                    .as_array()
                    .ok_or_else(|| LanguageError("files array required".into()))?
                {
                    self.service
                        .rename_uri(text(file, "oldUri")?, text(file, "newUri")?)?;
                }
                Ok(Value::Null)
            }
            "$/setTrace" => Ok(Value::Null),
            _ => Err(LanguageError(format!("Unknown method: {method}"))),
        }
    }
    fn initialize(&mut self, params: &Value, notifications: &mut Vec<Value>) -> Result<Value> {
        if self.initialized {
            return Err(LanguageError("Already initialized".into()));
        }
        self.initialized = true;
        let roots: Vec<String> = params["workspaceFolders"]
            .as_array()
            .map(|folders| {
                folders
                    .iter()
                    .filter_map(|folder| folder["uri"].as_str().map(str::to_owned))
                    .collect()
            })
            .unwrap_or_else(|| {
                params["rootUri"]
                    .as_str()
                    .map(|uri| vec![uri.into()])
                    .unwrap_or_default()
            });
        for uri in roots {
            let result = ferroui_designer_project::file_path(&uri)
                .and_then(ferroui_designer_project::Solution::discover)
                .map_err(LanguageError::from)
                .and_then(|solution| self.service.index_solution(&solution));
            if let Err(error) = result {
                notifications.push(json!({
                    "jsonrpc": "2.0", "method": "window/logMessage",
                    "params": { "type": 2, "message": error.to_string() }
                }));
            }
        }
        Ok(json!({
            "capabilities": {
                "positionEncoding": "utf-16",
                "textDocumentSync": { "openClose": true, "change": 2, "save": { "includeText": true } },
                "completionProvider": { "triggerCharacters": ["<", " ", ":", ".", "{", "="] },
                "hoverProvider": true, "definitionProvider": true, "referencesProvider": true,
                "renameProvider": { "prepareProvider": true }, "documentFormattingProvider": true,
                "documentSymbolProvider": true, "foldingRangeProvider": true, "workspaceSymbolProvider": true,
                "signatureHelpProvider": { "triggerCharacters": ["{", ",", "="] },
                "semanticTokensProvider": {
                    "legend": { "tokenTypes": ["type", "property", "string", "comment", "variable", "keyword", "number", "namespace"], "tokenModifiers": [] },
                    "full": true
                },
                "workspace": {
                    "workspaceFolders": { "supported": true, "changeNotifications": true },
                    "fileOperations": {
                        "willRename": { "filters": [{ "pattern": { "glob": "**/*.xaml" } }] },
                        "didRename": { "filters": [{ "pattern": { "glob": "**/*.xaml" } }] }
                    }
                }
            },
            "serverInfo": { "name": "FerroUI XAML", "version": env!("CARGO_PKG_VERSION") }
        }))
    }
    fn will_rename_files(&self, params: &Value) -> Result<Value> {
        let mut grouped = BTreeMap::<String, crate::DocumentEdits>::new();
        for file in params["files"]
            .as_array()
            .ok_or_else(|| LanguageError("files array required".into()))?
        {
            for document in self
                .service
                .file_rename_edits(text(file, "oldUri")?, text(file, "newUri")?)?
                .document_changes
            {
                grouped
                    .entry(document.uri.clone())
                    .and_modify(|existing| {
                        existing.edits.extend(document.edits.clone());
                    })
                    .or_insert(document);
            }
        }
        for document in grouped.values_mut() {
            document.edits.sort_by_key(|edit| {
                (
                    edit.range.start.line,
                    edit.range.start.character,
                    edit.range.end.line,
                    edit.range.end.character,
                )
            });
            document.edits.dedup();
            crate::apply_text_edits(
                self.service.document(&document.uri).unwrap().source(),
                &document.edits,
            )?;
        }
        Ok(workspace_edit(WorkspaceEdit {
            document_changes: grouped.into_values().collect(),
        }))
    }
    fn position<'a>(&self, params: &'a Value) -> Result<(&'a str, usize)> {
        let uri = uri(params)?;
        let position: Position = decode(&params["position"])?;
        Ok((uri, self.service.index(uri)?.offset(position)?))
    }
    fn publish(&self, uri: &str, out: &mut Vec<Value>) -> Result<()> {
        out.push(json!({"jsonrpc":"2.0","method":"textDocument/publishDiagnostics","params":{"uri":uri,"version":self.service.version(uri),"diagnostics":self.service.diagnostics(uri)?.into_iter().map(|d|json!({"range":d.range,"severity":d.severity,"code":d.code,"source":"FerroUI","message":d.message})).collect::<Vec<_>>()}}));
        Ok(())
    }
}
fn text<'a>(value: &'a Value, key: &str) -> Result<&'a str> {
    value[key]
        .as_str()
        .ok_or_else(|| LanguageError(format!("{key} must be a string")))
}
fn integer(value: &Value, key: &str) -> Result<i64> {
    value[key]
        .as_i64()
        .ok_or_else(|| LanguageError(format!("{key} must be an integer")))
}
fn uri(params: &Value) -> Result<&str> {
    text(&params["textDocument"], "uri")
}
fn decode<T: serde::de::DeserializeOwned>(value: &Value) -> Result<T> {
    serde_json::from_value(value.clone()).map_err(|e| LanguageError(e.to_string()))
}
fn workspace_edit(edit: WorkspaceEdit) -> Value {
    json!({"documentChanges":edit.document_changes.into_iter().map(|document|json!({"textDocument":{"uri":document.uri,"version":if document.version<0{Value::Null}else{json!(document.version)}},"edits":document.edits})).collect::<Vec<_>>()})
}
fn error(id: Value, code: i64, message: &str) -> Value {
    json!({"jsonrpc":"2.0","id":id,"error":{"code":code,"message":message}})
}

pub fn read_frame(reader: &mut impl BufRead) -> io::Result<Option<Value>> {
    let mut length = None;
    let mut total = 0;
    loop {
        let mut line = Vec::new();
        let count = reader.by_ref().take(8193).read_until(b'\n', &mut line)?;
        if count == 0 {
            return if total == 0 {
                Ok(None)
            } else {
                Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "Incomplete LSP header",
                ))
            };
        }
        total += count;
        if count > 8192 || total > 32768 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "LSP headers too large",
            ));
        }
        let line = std::str::from_utf8(&line)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Invalid LSP header"))?
            .trim();
        if line.is_empty() {
            break;
        }
        let (name, value) = line
            .split_once(':')
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Malformed LSP header"))?;
        if name.eq_ignore_ascii_case("Content-Length") {
            if length.is_some() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Duplicate Content-Length",
                ));
            }
            length = Some(value.trim().parse::<usize>().map_err(|_| {
                io::Error::new(io::ErrorKind::InvalidData, "Invalid Content-Length")
            })?);
        }
    }
    let length = length
        .filter(|length| *length <= 16 * 1024 * 1024)
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "Missing or excessive Content-Length",
            )
        })?;
    let mut bytes = vec![0; length];
    reader.read_exact(&mut bytes)?;
    serde_json::from_slice(&bytes)
        .map(Some)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}
pub fn write_frame(writer: &mut impl Write, message: &Value) -> io::Result<()> {
    let bytes = serde_json::to_vec(message)?;
    write!(writer, "Content-Length: {}\r\n\r\n", bytes.len())?;
    writer.write_all(&bytes)?;
    writer.flush()
}
/// The reader remains responsive to cancellation while semantic work runs. Source notifications
/// and queries are processed in arrival order; a cancelled query's result is never published.
pub fn serve<R: BufRead + Send + 'static, W: Write>(
    mut reader: R,
    mut writer: W,
) -> io::Result<()> {
    let mut server = LspServer::default();
    let cancelled = server.cancelled.clone();
    let (send, receive) = mpsc::sync_channel(64);
    std::thread::spawn(move || loop {
        match read_frame(&mut reader) {
            Ok(Some(message)) => {
                if message["method"] == "$/cancelRequest" {
                    cancelled
                        .lock()
                        .unwrap()
                        .insert(message["params"]["id"].to_string());
                    continue;
                }
                if send.send(Ok(message)).is_err() {
                    break;
                }
            }
            Ok(None) => break,
            Err(error) => {
                let _ = send.send(Err(error));
                break;
            }
        }
    });
    for message in receive {
        for response in server.handle(message?) {
            write_frame(&mut writer, &response)?;
        }
        if server.exit {
            if !server.shutdown {
                return Err(io::Error::other("LSP exit received before shutdown"));
            }
            break;
        }
    }
    Ok(())
}
