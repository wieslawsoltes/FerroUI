use crate::Client;
use ferroui_base::logging::LogEventLevel;
use ferroui_inspect::LogCapture;
use serde_json::{json, Value};
use std::{
    collections::{HashMap, VecDeque},
    sync::{
        atomic::{AtomicBool, AtomicU8, Ordering},
        Arc, Mutex,
    },
};
pub const VERSION: &str = "2025-11-25";
pub const VERSIONS: &[&str] = &[VERSION, "2025-06-18", "2025-03-26"];
/// The one version that has JSON-RPC batches; later versions removed them.
pub const BATCH_VERSION: &str = "2025-03-26";
pub const TREE_URI: &str = "ferroui://tree";
pub const LOG_URI: &str = "ferroui://logs";
/// How many cancellations of requests that have not arrived yet a session remembers.
const EARLY_CANCELLATIONS: usize = 64;
#[derive(Default)]
pub struct Session {
    state: AtomicU8,
    pub stream_open: AtomicBool,
    requests: Mutex<HashMap<String, Arc<AtomicBool>>>,
    /// Cancellations whose request had not been registered when they arrived: a client may
    /// cancel a request that is still on its way to a worker.
    cancelled: Mutex<VecDeque<String>>,
    version: Mutex<Option<String>>,
    pub log_level: Mutex<Option<String>>,
}
impl Session {
    pub fn close(&self) {
        self.state.store(3, Ordering::Release);
        for flag in self.requests.lock().unwrap().values() {
            flag.store(true, Ordering::Release);
        }
    }
    pub fn is_closed(&self) -> bool {
        self.state.load(Ordering::Acquire) == 3
    }
    /// Whether the negotiated version accepts JSON-RPC batches (arrays of messages).
    pub fn accepts_batches(&self) -> bool {
        self.version.lock().unwrap().as_deref() == Some(BATCH_VERSION)
    }
    /// A message or, for the version that has them, a batch: the responses of a batch are an
    /// array, and a batch of notifications has none.
    pub fn handle_message(&self, client: &Client, message: Value) -> Option<Value> {
        let Value::Array(batch) = message else {
            return self.handle(client, message);
        };
        if batch.is_empty() || !self.accepts_batches() {
            return Some(error(Value::Null, -32600, "Invalid Request"));
        }
        if batch.iter().any(|m| m["method"] == "initialize") {
            return Some(error(
                Value::Null,
                -32600,
                "initialize cannot be part of a batch",
            ));
        }
        let responses: Vec<_> = batch
            .into_iter()
            .filter_map(|m| self.handle(client, m))
            .collect();
        (!responses.is_empty()).then_some(Value::Array(responses))
    }
    pub fn handle(&self, client: &Client, message: Value) -> Option<Value> {
        if !message.is_object()
            || message["jsonrpc"] != "2.0"
            || !message["method"].is_string()
            || message
                .get("id")
                .is_some_and(|id| !id.is_string() && !id.is_number())
        {
            return Some(error(Value::Null, -32600, "Invalid Request"));
        }
        let method = message["method"].as_str().unwrap();
        let params = message.get("params").cloned().unwrap_or_else(|| json!({}));
        if method.starts_with("notifications/") {
            if let Some(id) = message.get("id") {
                return Some(error(id.clone(), -32600, "Notifications have no id"));
            }
        }
        if method == "notifications/cancelled" {
            let key = params["requestId"].to_string();
            let requests = self.requests.lock().unwrap();
            if let Some(flag) = requests.get(&key) {
                flag.store(true, Ordering::Release);
            } else {
                let mut cancelled = self.cancelled.lock().unwrap();
                if cancelled.len() == EARLY_CANCELLATIONS {
                    cancelled.pop_front();
                }
                cancelled.push_back(key);
            }
            return None;
        }
        if method == "notifications/initialized" {
            let _ = self
                .state
                .compare_exchange(1, 2, Ordering::AcqRel, Ordering::Acquire);
            return None;
        }
        let id = message.get("id")?.clone(); // Notifications never execute tools.
        if !params.is_object() {
            return Some(error(id, -32602, "params must be an object"));
        }
        if method != "initialize" && method != "ping" && self.state.load(Ordering::Acquire) != 2 {
            return Some(error(id, -32000, "Initialize the session first"));
        }
        let result = match method {
            "initialize" => {
                if !params["protocolVersion"].is_string() {
                    return Some(error(id, -32602, "protocolVersion required"));
                }
                if self
                    .state
                    .compare_exchange(0, 1, Ordering::AcqRel, Ordering::Acquire)
                    .is_err()
                {
                    return Some(error(id, -32600, "Session already initialized"));
                }
                let requested = params["protocolVersion"].as_str().unwrap();
                let version = if VERSIONS.contains(&requested) {
                    requested
                } else {
                    VERSION
                };
                *self.version.lock().unwrap() = Some(version.to_owned());
                json!({"protocolVersion":version,"capabilities":{"tools":{},"resources":{"subscribe":false,"listChanged":false},"logging":{}},"serverInfo":{"name":"ferroui-mcp","version":env!("CARGO_PKG_VERSION")},"instructions":"Inspect a running FerroUI application. Property writes and input require explicit application permissions. IDs expire when their objects are destroyed."})
            }
            "ping" => json!({}),
            "tools/list" => json!({"tools":ferroui_inspect::tools()}),
            "resources/list" => {
                json!({"resources":[{"uri":TREE_URI,"name":"Current visual tree","mimeType":"application/json"},{"uri":LOG_URI,"name":"Diagnostic log","mimeType":"application/json"}]})
            }
            "resources/templates/list" => json!({"resourceTemplates":[]}),
            "logging/setLevel" => {
                let Some(level) = params["level"].as_str().filter(|s| {
                    [
                        "debug",
                        "info",
                        "notice",
                        "warning",
                        "error",
                        "critical",
                        "alert",
                        "emergency",
                    ]
                    .contains(s)
                }) else {
                    return Some(error(id, -32602, "Invalid log level"));
                };
                // The capture records from Information up unless a client asks for more.
                if level == "debug" {
                    LogCapture::capture_from(LogEventLevel::Verbose);
                }
                *self.log_level.lock().unwrap() = Some(level.to_owned());
                json!({})
            }
            "tools/call" | "resources/read" => {
                let (tool, args, uri) = if method == "resources/read" {
                    let uri = params["uri"].as_str().unwrap_or("");
                    let tool = match uri {
                        TREE_URI => "get_tree",
                        LOG_URI => "get_logs",
                        _ => return Some(error(id, -32002, "Unknown resource")),
                    };
                    (tool, json!({}), Some(uri))
                } else {
                    let Some(tool) = params["name"].as_str() else {
                        return Some(error(id, -32602, "Tool name required"));
                    };
                    if !ferroui_inspect::tools()
                        .iter()
                        .any(|t| t["name"].as_str() == Some(tool))
                    {
                        return Some(error(id, -32602, &format!("Unknown tool: {tool}")));
                    }
                    let args = params
                        .get("arguments")
                        .cloned()
                        .unwrap_or_else(|| json!({}));
                    // Invalid arguments are a result of the tool, which the model can correct.
                    if let Err(e) = ferroui_inspect::validate(tool, &args) {
                        return Some(
                            json!({"jsonrpc":"2.0","id":id,"result":{"content":[{"type":"text","text":e}],"isError":true}}),
                        );
                    }
                    (tool, args, None)
                };
                let flag = Arc::new(AtomicBool::new(false));
                let key = id.to_string();
                {
                    let mut requests = self.requests.lock().unwrap();
                    if requests.contains_key(&key) {
                        return Some(error(id, -32600, "Duplicate in-flight request ID"));
                    }
                    let mut cancelled = self.cancelled.lock().unwrap();
                    if let Some(index) = cancelled.iter().position(|k| *k == key) {
                        cancelled.remove(index);
                        flag.store(true, Ordering::Release);
                    }
                    requests.insert(key.clone(), flag.clone());
                }
                let result = client.call(tool, args, &flag);
                self.requests.lock().unwrap().remove(&key);
                match result {
                    Ok(value) => {
                        if let Some(uri) = uri {
                            json!({"contents":[{"uri":uri,"mimeType":"application/json","text":value.to_string()}]})
                        } else if tool == "screenshot" {
                            json!({"content":[{"type":"image","mimeType":"image/png","data":value["data"]}],"isError":false})
                        } else {
                            json!({"content":[{"type":"text","text":value.to_string()}],"isError":false})
                        }
                    }
                    Err(e) => {
                        if uri.is_some() {
                            return Some(error(id, -32603, &e));
                        } else {
                            json!({"content":[{"type":"text","text":e}],"isError":true})
                        }
                    }
                }
            }
            _ => return Some(error(id, -32601, "Method not found")),
        };
        Some(json!({"jsonrpc":"2.0","id":id,"result":result}))
    }
    /// The `notifications/message` of the log entries after `cursor`, at the session's level.
    pub fn logs(&self, cursor: &mut u64) -> Vec<Value> {
        let entries = LogCapture::since(cursor);
        let level = self.log_level.lock().unwrap().clone();
        let Some(level) = level else {
            return vec![];
        };
        let rank = |s: &str| match s {
            "debug" => 0,
            "info" => 1,
            "notice" => 2,
            "warning" => 3,
            "error" => 4,
            _ => 5,
        };
        entries
            .iter()
            .filter_map(|entry| {
                let severity = match entry.level {
                    LogEventLevel::Verbose | LogEventLevel::Debug => "debug",
                    LogEventLevel::Information => "info",
                    LogEventLevel::Warning => "warning",
                    LogEventLevel::Error => "error",
                    _ => "critical",
                };
                (rank(severity) >= rank(&level)).then(|| {
                    json!({"jsonrpc":"2.0","method":"notifications/message","params":{"level":severity,"logger":entry.area,"data":entry.to_json()}})
                })
            })
            .collect()
    }
}
pub fn error(id: Value, code: i32, message: &str) -> Value {
    json!({"jsonrpc":"2.0","id":id,"error":{"code":code,"message":message}})
}
