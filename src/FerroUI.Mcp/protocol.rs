use crate::Client;
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, AtomicU8, Ordering},
        Arc, Mutex,
    },
};
pub const VERSION: &str = "2025-11-25";
pub const VERSIONS: &[&str] = &[VERSION, "2025-06-18", "2025-03-26"];
pub const TREE_URI: &str = "ferroui://tree";
pub const LOG_URI: &str = "ferroui://logs";
#[derive(Default)]
pub struct Session {
    state: AtomicU8,
    pub stream_open: AtomicBool,
    requests: Mutex<HashMap<String, Arc<AtomicBool>>>,
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
        if method == "notifications/cancelled" {
            if let Some(flag) = self
                .requests
                .lock()
                .unwrap()
                .get(&params["requestId"].to_string())
            {
                flag.store(true, Ordering::Release);
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
                let version = params["protocolVersion"].as_str().unwrap();
                json!({"protocolVersion":if VERSIONS.contains(&version){version}else{VERSION},"capabilities":{"tools":{},"resources":{"subscribe":false,"listChanged":false},"logging":{}},"serverInfo":{"name":"ferroui-mcp","version":env!("CARGO_PKG_VERSION")},"instructions":"Inspect a running FerroUI application. Property writes and input require explicit application permissions. IDs expire when their objects are destroyed."})
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
                    let args = params
                        .get("arguments")
                        .cloned()
                        .unwrap_or_else(|| json!({}));
                    if let Err(e) = ferroui_inspect::validate(tool, &args) {
                        return Some(error(id, -32602, &e));
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
    pub fn logs(&self, client: &Client, cursor: &mut u64) -> Vec<Value> {
        let snapshot = client.logs.snapshot(*cursor);
        *cursor = snapshot["cursor"].as_u64().unwrap_or(*cursor);
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
        snapshot["entries"].as_array().unwrap().iter().filter_map(|entry| {
            let severity=match entry["level"].as_str().unwrap_or("") {"Verbose"|"Debug"=>"debug","Information"=>"info","Warning"=>"warning","Error"=>"error",_=>"critical"};
            (rank(severity)>=rank(&level)).then(||json!({"jsonrpc":"2.0","method":"notifications/message","params":{"level":severity,"logger":entry["area"],"data":entry}}))
        }).collect()
    }
}
pub fn error(id: Value, code: i32, message: &str) -> Value {
    json!({"jsonrpc":"2.0","id":id,"error":{"code":code,"message":message}})
}
