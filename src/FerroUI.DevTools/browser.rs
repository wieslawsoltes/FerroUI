use crate::protocol::{response, string, CdpSession, Error, Result};
use ferroui_base::logging::LogEventLevel;
use ferroui_base::reactive::IDisposable;
use ferroui_base::{Ref, WeakRef};
use ferroui_controls::{Application, TopLevel, Window};
use ferroui_inspect::{LogCapture, LogEntry, LogGuard};
use serde_json::{json, Value};
use std::cell::Cell;
use std::collections::{BTreeMap, HashSet};
use std::rc::Rc;

struct Target {
    root: WeakRef<TopLevel>,
    closed: Rc<Cell<bool>>,
    subscription: Rc<dyn IDisposable>,
}
impl Drop for Target {
    fn drop(&mut self) {
        self.subscription.dispose();
    }
}
struct Connection {
    direct: Option<String>,
    browser_sessions: HashSet<String>,
    parents: BTreeMap<String, String>,
    sessions: BTreeMap<String, (String, CdpSession)>,
    discover: bool,
    auto_attach: bool,
    known: HashSet<String>,
    events: Vec<Value>,
}

/// UI-thread target registry and flattened CDP session router, shared by the native server and
/// browser bridge. Window and popup discovery follows the desktop application lifetime.
pub struct Browser {
    targets: BTreeMap<String, Target>,
    connections: BTreeMap<u64, Connection>,
    next_target: u64,
    next_connection: u64,
    next_session: u64,
    /// The log capture shared with the other servers of the process.
    _logs: LogGuard,
}
impl Default for Browser {
    fn default() -> Self {
        Self::new()
    }
}
impl Browser {
    pub fn new() -> Self {
        Self {
            targets: BTreeMap::new(),
            connections: BTreeMap::new(),
            next_target: 1,
            next_connection: 1,
            next_session: 1,
            _logs: LogCapture::install(),
        }
    }
    /// Explicit registration also supports custom lifetimes. The registry retains only a weak root.
    pub fn register(&mut self, root: &Ref<TopLevel>) -> String {
        if let Some((id, _)) = self
            .targets
            .iter()
            .find(|(_, t)| t.root.upgrade().as_ref() == Some(root) && !t.closed.get())
        {
            return id.clone();
        }
        let id = self.next_target.to_string();
        self.next_target += 1;
        let closed = Rc::new(Cell::new(false));
        let flag = closed.clone();
        let subscription = root.closed(move || flag.set(true));
        self.targets.insert(
            id.clone(),
            Target {
                root: root.downgrade(),
                closed,
                subscription,
            },
        );
        id
    }
    pub fn refresh(&mut self) {
        if let Some(lifetime) = Application::current().and_then(|a| a.application_lifetime()) {
            if let Some(desktop) = lifetime.as_classic_desktop_style_application_lifetime() {
                for window in desktop.windows() {
                    self.register(&window.upcast());
                }
            }
        }
        let mut pending: Vec<_> = self
            .targets
            .values()
            .filter_map(|t| t.root.upgrade())
            .collect();
        let mut seen = Vec::new();
        while let Some(root) = pending.pop() {
            if seen.contains(&root) {
                continue;
            }
            seen.push(root.clone());
            for popup in root.opened_popups() {
                if let Some(top) = popup
                    .host()
                    .and_then(|h| h.as_popup_root())
                    .map(|r| r.upcast::<TopLevel>())
                {
                    self.register(&top);
                    pending.push(top);
                }
            }
        }
        self.targets
            .retain(|_, t| !t.closed.get() && t.root.upgrade().is_some());
    }
    fn info(&self, id: &str) -> Result<Value> {
        let root = self
            .targets
            .get(id)
            .and_then(|t| t.root.upgrade())
            .ok_or_else(|| Error::server("Unknown target"))?;
        let title = root
            .cast::<Window>()
            .and_then(|w| w.title())
            .unwrap_or_else(|| root.get_type().name().into());
        Ok(
            json!({"targetId":id,"type":"page","title":title,"url":format!("ferroui://{id}"),"attached":false,"canAccessOpener":false,"browserContextId":"default"}),
        )
    }
    pub fn discovery(&mut self, port: u16) -> Value {
        self.refresh();
        self.targets.keys().filter_map(|id|self.info(id).ok()).map(|info| {
            let id = info["targetId"].as_str().unwrap();
            json!({"id":id,"type":"page","title":info["title"],"description":"FerroUI native visual tree","url":info["url"],"webSocketDebuggerUrl":format!("ws://127.0.0.1:{port}/devtools/page/{id}"),"devtoolsFrontendUrl":format!("devtools://devtools/bundled/devtools_app.html?ws=127.0.0.1:{port}/devtools/page/{id}")})
        }).collect()
    }
    pub fn version(port: u16) -> Value {
        json!({"Browser":concat!("FerroUI/",env!("CARGO_PKG_VERSION")),"Protocol-Version":"1.3","User-Agent":"FerroUI","V8-Version":"0","WebKit-Version":"0","webSocketDebuggerUrl":format!("ws://127.0.0.1:{port}/devtools/browser")})
    }
    pub fn connect(&mut self, target: Option<&str>) -> std::result::Result<u64, String> {
        self.refresh();
        let mut connection = Connection {
            direct: target.map(str::to_owned),
            browser_sessions: HashSet::new(),
            parents: BTreeMap::new(),
            sessions: BTreeMap::new(),
            discover: false,
            auto_attach: false,
            known: HashSet::new(),
            events: Vec::new(),
        };
        if let Some(target) = target {
            let root = self
                .targets
                .get(target)
                .and_then(|t| t.root.upgrade())
                .ok_or("Unknown target")?;
            connection.sessions.insert(
                String::new(),
                (target.into(), CdpSession::new(&root, target)),
            );
        }
        let id = self.next_connection;
        self.next_connection += 1;
        self.connections.insert(id, connection);
        Ok(id)
    }
    pub fn disconnect(&mut self, connection: u64) {
        self.connections.remove(&connection);
    }
    pub fn handle(&mut self, connection: u64, message: &str) -> Vec<Value> {
        self.refresh();
        let Some(mut c) = self.connections.remove(&connection) else {
            return vec![response(
                Value::Null,
                Err(Error::server("Connection closed")),
            )];
        };
        let request: std::result::Result<Value, _> = serde_json::from_str(message);
        let events_first = request.as_ref().is_ok_and(|r| {
            r["method"] == "DOM.requestChildNodes"
                || r["method"] == "DOM.pushNodesByBackendIdsToFrontend"
                || r["method"] == "DOM.pushNodeByPathToFrontend"
        });
        let mut messages = match request {
            Err(e) => vec![response(
                Value::Null,
                Err(Error {
                    code: -32700,
                    message: e.to_string(),
                }),
            )],
            Ok(r) => {
                let session = r["sessionId"].as_str().unwrap_or("");
                let result = if !r["id"].is_u64() {
                    Err(Error {
                        code: -32600,
                        message: "Request requires a nonnegative integer id".into(),
                    })
                } else if let Some(method) = r["method"].as_str() {
                    let params = r.get("params").cloned().unwrap_or_else(|| json!({}));
                    if !params.is_object() {
                        Err(Error::params("params must be an object"))
                    } else if (!session.is_empty() && !c.browser_sessions.contains(session))
                        || (session.is_empty()
                            && c.direct.is_some()
                            && !method.starts_with("Target.")
                            && !method.starts_with("Browser."))
                    {
                        match c.sessions.get_mut(session) {
                            Some((target, s)) if self.targets.contains_key(target) => {
                                let result = s.command(method, &params);
                                for event in s.poll() {
                                    c.events.push(with_session(event, session));
                                }
                                result
                            }
                            _ => Err(Error::server("Unknown or closed session")),
                        }
                    } else {
                        self.browser_command(&mut c, method, &params, session)
                    }
                } else {
                    Err(Error::params("method must be a string"))
                };
                vec![with_session(response(r["id"].clone(), result), session)]
            }
        };
        if events_first {
            // DOM clients resolve getChildNodes when the response arrives, so populate their
            // model with setChildNodes first (as Chromium does).
            c.events.append(&mut messages);
            messages = std::mem::take(&mut c.events);
        } else {
            messages.append(&mut c.events);
        }
        self.connections.insert(connection, c);
        messages
    }
    fn attach(&mut self, c: &mut Connection, target: &str, parent: &str) -> Result<String> {
        if c.sessions.len() + c.browser_sessions.len() >= 64 {
            return Err(Error::server("Session limit reached"));
        }
        let root = self
            .targets
            .get(target)
            .and_then(|t| t.root.upgrade())
            .ok_or_else(|| Error::server("Unknown target"))?;
        let session = format!("session-{}", self.next_session);
        self.next_session += 1;
        c.sessions.insert(
            session.clone(),
            (target.into(), CdpSession::new(&root, target)),
        );
        c.parents.insert(session.clone(), parent.into());
        c.events.push(with_session(json!({"method":"Target.attachedToTarget","params":{"sessionId":session,"targetInfo":self.info(target)?,"waitingForDebugger":false}}),parent));
        Ok(session)
    }
    fn browser_command(
        &mut self,
        c: &mut Connection,
        method: &str,
        p: &Value,
        parent: &str,
    ) -> Result<Value> {
        match method {
            "Browser.getVersion" => Ok(
                json!({"protocolVersion":"1.3","product":concat!("FerroUI/",env!("CARGO_PKG_VERSION")),"revision":"0","userAgent":"FerroUI","jsVersion":"0"}),
            ),
            "Target.getBrowserContexts" => Ok(json!({"browserContextIds":[]})),
            "Target.getTargets" => Ok(
                json!({"targetInfos":self.targets.keys().filter_map(|id|self.info(id).ok()).collect::<Vec<_>>()}),
            ),
            "Target.getTargetInfo" => {
                if let Some(id) = p["targetId"].as_str().or(c.direct.as_deref()) {
                    Ok(json!({"targetInfo":self.info(id)?}))
                } else {
                    Ok(
                        json!({"targetInfo":{"targetId":"browser","type":"browser","title":"FerroUI","url":"","attached":true,"canAccessOpener":false}}),
                    )
                }
            }
            "Target.setDiscoverTargets" => {
                c.discover = p["discover"]
                    .as_bool()
                    .ok_or_else(|| Error::params("discover must be boolean"))?;
                c.known.clear();
                self.update_connection(c);
                Ok(json!({}))
            }
            "Target.setAutoAttach" => {
                let auto_attach = p["autoAttach"]
                    .as_bool()
                    .ok_or_else(|| Error::params("autoAttach must be boolean"))?;
                if auto_attach && p["flatten"] != true {
                    return Err(Error::params("Only flattened sessions are supported"));
                }
                // On the socket of one page, auto-attach is about that page's own sub-targets,
                // which a window does not have; only the browser socket attaches the windows.
                c.auto_attach = auto_attach && c.direct.is_none();
                self.update_connection(c);
                Ok(json!({}))
            }
            "Target.attachToBrowserTarget" => {
                if c.sessions.len() + c.browser_sessions.len() >= 64 {
                    return Err(Error::server("Session limit reached"));
                }
                let session = format!("browser-session-{}", self.next_session);
                self.next_session += 1;
                c.browser_sessions.insert(session.clone());
                Ok(json!({"sessionId":session}))
            }
            "Target.attachToTarget" => {
                if p["flatten"] != true {
                    return Err(Error::params("Only flattened sessions are supported"));
                }
                Ok(json!({"sessionId":self.attach(c,string(p,"targetId")?,parent)?}))
            }
            "Target.detachFromTarget" => {
                let id = string(p, "sessionId")?;
                if c.browser_sessions.remove(id) {
                    let children: Vec<_> = c
                        .parents
                        .iter()
                        .filter(|(_, p)| p.as_str() == id)
                        .map(|(s, _)| s.clone())
                        .collect();
                    for child in children {
                        c.sessions.remove(&child);
                        c.parents.remove(&child);
                    }
                    return Ok(json!({}));
                }
                let (target, _) = c
                    .sessions
                    .remove(id)
                    .ok_or_else(|| Error::server("Unknown session"))?;
                let parent = c.parents.remove(id).unwrap_or_default();
                c.events.push(with_session(json!({"method":"Target.detachedFromTarget","params":{"sessionId":id,"targetId":target}}),&parent));
                Ok(json!({}))
            }
            "Browser.setDownloadBehavior" => Ok(json!({})),
            _ => Err(Error::method(method)),
        }
    }
    fn update_connection(&mut self, c: &mut Connection) {
        let current: HashSet<_> = self.targets.keys().cloned().collect();
        if c.discover {
            for id in current.difference(&c.known) {
                if let Ok(info) = self.info(id) {
                    c.events.push(
                        json!({"method":"Target.targetCreated","params":{"targetInfo":info}}),
                    );
                }
            }
            for id in c.known.difference(&current) {
                c.events
                    .push(json!({"method":"Target.targetDestroyed","params":{"targetId":id}}));
            }
        }
        c.known = current;
        let removed: Vec<_> = c
            .sessions
            .iter()
            .filter(|(_, (id, _))| !self.targets.contains_key(id))
            .map(|(s, (t, _))| (s.clone(), t.clone()))
            .collect();
        for (s, t) in removed {
            c.sessions.remove(&s);
            let parent = c.parents.remove(&s).unwrap_or_default();
            c.events.push(with_session(
                json!({"method":"Target.detachedFromTarget","params":{"sessionId":s,"targetId":t}}),
                &parent,
            ));
        }
        if c.auto_attach {
            let ids: Vec<_> = self
                .targets
                .keys()
                .filter(|id| !c.sessions.values().any(|(t, _)| t == *id))
                .cloned()
                .collect();
            for id in ids {
                let _ = self.attach(c, &id, "");
            }
        }
    }
    pub fn poll(&mut self, connection: u64) -> Vec<Value> {
        self.refresh();
        let Some(mut c) = self.connections.remove(&connection) else {
            return vec![];
        };
        self.update_connection(&mut c);
        for (session, (_, s)) in &mut c.sessions {
            for event in s.poll() {
                c.events.push(with_session(event, session));
            }
            for entry in LogCapture::since(&mut s.log_cursor)
                .iter()
                .filter(|e| e.level >= LogEventLevel::Information)
                .map(log_entry)
            {
                if s.enabled.contains("Log") {
                    c.events.push(with_session(
                        json!({"method":"Log.entryAdded","params":{"entry":entry}}),
                        session,
                    ));
                }
                if s.enabled.contains("Runtime") {
                    c.events.push(with_session(json!({"method":"Runtime.consoleAPICalled","params":{"type":if entry["level"]=="warning" {"warning"} else if entry["level"]=="error" {"error"} else {"log"},"args":[{"type":"string","value":entry["text"]}],"executionContextId":1,"timestamp":entry["timestamp"]}}),session));
                }
            }
        }
        let events = std::mem::take(&mut c.events);
        self.connections.insert(connection, c);
        events
    }
}
/// A `Log.LogEntry` of CDP.
fn log_entry(entry: &LogEntry) -> Value {
    let level = match entry.level {
        LogEventLevel::Warning => "warning",
        LogEventLevel::Error | LogEventLevel::Fatal => "error",
        _ => "info",
    };
    json!({"source":"other","level":level,"text":format!("[{}] {}",entry.area,entry.message()),"timestamp":entry.timestamp})
}
fn with_session(mut value: Value, session: &str) -> Value {
    if !session.is_empty() {
        value["sessionId"] = json!(session);
    }
    value
}
