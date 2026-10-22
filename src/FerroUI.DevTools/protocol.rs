use crate::input::Input;
use base64::Engine;
use ferroui_base::input::InputElement;
use ferroui_base::interactivity::RoutingStrategies;
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::{Point, Rect, Ref, StyledElement};
use ferroui_controls::automation::peers::{AutomationPeer, ControlAutomationPeer};
use ferroui_controls::{Control, TopLevel};
use ferroui_inspect::target::{Inspector, Node, NodeId, TreeKind};
use serde_json::{json, Value};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

pub(crate) type Result<T> = std::result::Result<T, Error>;
#[derive(Debug)]
pub(crate) struct Error {
    pub code: i32,
    pub message: String,
}
impl Error {
    pub fn params(message: impl Into<String>) -> Self {
        Self {
            code: -32602,
            message: message.into(),
        }
    }
    pub fn server(message: impl Into<String>) -> Self {
        Self {
            code: -32000,
            message: message.into(),
        }
    }
    pub fn method(method: &str) -> Self {
        Self {
            code: -32601,
            message: format!("Unsupported method: {method}"),
        }
    }
}
impl From<String> for Error {
    fn from(value: String) -> Self {
        Self::server(value)
    }
}
pub(crate) fn string<'a>(p: &'a Value, key: &str) -> Result<&'a str> {
    p[key]
        .as_str()
        .ok_or_else(|| Error::params(format!("{key} must be a string")))
}
pub(crate) fn number(p: &Value, key: &str) -> Result<f64> {
    p[key]
        .as_f64()
        .filter(|n| n.is_finite())
        .ok_or_else(|| Error::params(format!("{key} must be a finite number")))
}
fn node_id(p: &Value) -> Result<NodeId> {
    p["nodeId"]
        .as_u64()
        .or_else(|| p["backendNodeId"].as_u64())
        .filter(|n| *n > 0)
        .ok_or_else(|| Error::params("nodeId is required"))
}
fn depth(p: &Value, default: i64) -> Result<i64> {
    let d = p
        .get("depth")
        .map(|d| {
            d.as_i64()
                .ok_or_else(|| Error::params("depth must be an integer"))
        })
        .transpose()?
        .unwrap_or(default);
    if d < -1 || d == 0 || d > 256 {
        Err(Error::params("depth must be -1 or 1..256"))
    } else {
        Ok(if d == -1 { 256 } else { d })
    }
}
fn quad(r: Rect) -> Value {
    json!([
        r.x,
        r.y,
        r.x + r.width,
        r.y,
        r.x + r.width,
        r.y + r.height,
        r.x,
        r.y + r.height
    ])
}
fn rect(r: Rect) -> Value {
    json!({"x":r.x,"y":r.y,"width":r.width,"height":r.height})
}
fn style(properties: Vec<Value>) -> Value {
    json!({"cssProperties":properties,"shorthandEntries":[]})
}
fn dom_node(id: NodeId, nodes: &HashMap<NodeId, &Node>, depth: i64) -> Value {
    let n = nodes[&id];
    let mut value = json!({"nodeId":id,"backendNodeId":id,"nodeType":1,"nodeName":n.name,"localName":n.name,"nodeValue":"","childNodeCount":n.children.len(),"attributes":n.attributes});
    if let Some(parent) = n.parent {
        value["parentId"] = json!(parent);
    }
    if depth > 0 {
        value["children"] = n
            .children
            .iter()
            .filter(|id| nodes.contains_key(id))
            .map(|id| dom_node(*id, nodes, depth - 1))
            .collect();
    }
    value
}

/// A transport-independent CDP connection to a single top-level. All calls run on its UI thread.
/// `handle` returns the response followed by any events; call `poll` for asynchronous events.
pub struct CdpSession {
    pub(crate) inspector: Inspector,
    input: Input,
    pub(crate) enabled: HashSet<String>,
    target_id: String,
    inspect_subscriptions: Vec<Rc<dyn IDisposable>>,
    picked: Rc<RefCell<Vec<(Point, bool)>>>,
    events: Vec<Value>,
    last_tree: Option<Vec<Node>>,
    pub(crate) log_cursor: u64,
}
impl CdpSession {
    pub fn new(root: &Ref<TopLevel>, target_id: impl Into<String>) -> Self {
        Self {
            inspector: Inspector::new(root),
            input: Input::default(),
            enabled: HashSet::new(),
            target_id: target_id.into(),
            inspect_subscriptions: Vec::new(),
            picked: Rc::new(RefCell::new(Vec::new())),
            events: Vec::new(),
            last_tree: None,
            log_cursor: 0,
        }
    }
    pub fn handle(&mut self, message: &str) -> Vec<Value> {
        let request: Value = match serde_json::from_str(message) {
            Ok(v) => v,
            Err(e) => {
                return vec![json!({"id":null,"error":{"code":-32700,"message":e.to_string()}})]
            }
        };
        let id = request["id"].clone();
        let result = if !request["id"].is_u64() {
            Err(Error {
                code: -32600,
                message: "Request requires a nonnegative integer id".into(),
            })
        } else {
            string(&request, "method").and_then(|method| {
                self.command(method, request.get("params").unwrap_or(&json!({})))
            })
        };
        let response = response(id, result);
        if request["method"] == "DOM.requestChildNodes"
            || request["method"] == "DOM.pushNodesByBackendIdsToFrontend"
            || request["method"] == "DOM.pushNodeByPathToFrontend"
        {
            let mut messages = std::mem::take(&mut self.events);
            messages.push(response);
            messages
        } else {
            let mut messages = vec![response];
            messages.append(&mut self.events);
            messages
        }
    }
    pub(crate) fn command(&mut self, method: &str, p: &Value) -> Result<Value> {
        if !p.is_object() {
            return Err(Error::params("params must be an object"));
        }
        self.inspector.root()?;
        if let Some(domain) = method.strip_suffix(".enable").filter(|d| {
            matches!(
                *d,
                "DOM" | "CSS" | "Page" | "Runtime" | "Log" | "Accessibility" | "Overlay"
            )
        }) {
            let newly_enabled = self.enabled.insert(domain.into());
            if domain == "Runtime" && newly_enabled {
                self.events.push(json!({"method":"Runtime.executionContextCreated","params":{"context":{"id":1,"origin":"ferroui://","name":"FerroUI (no JavaScript engine)","uniqueId":format!("ferroui-{}",self.target_id),"auxData":{"isDefault":true,"frameId":self.target_id}}}}));
            }
            return Ok(json!({}));
        }
        if let Some(domain) = method.strip_suffix(".disable").filter(|d| {
            matches!(
                *d,
                "DOM" | "CSS" | "Page" | "Runtime" | "Log" | "Accessibility" | "Overlay"
            )
        }) {
            self.enabled.remove(domain);
            if domain == "Overlay" {
                self.stop_inspecting();
                self.inspector.hide_highlight();
            }
            return Ok(json!({}));
        }
        match method {
            "DOM.getDocument" => {
                self.enabled.insert("DOM".into());
                let depth = depth(p,1)?;
                let tree = match p["tree"].as_str() { None => self.inspector.tree_kind, Some("visual") => TreeKind::Visual, Some("logical") => TreeKind::Logical, _ => return Err(Error::params("tree must be visual or logical")) };
                self.inspector.tree_kind = tree;
                let nodes = self.inspector.snapshot()?;
                self.last_tree = Some(nodes.clone());
                let map = nodes.iter().map(|n| (n.id,n)).collect();
                let children: Vec<_> = nodes.first().map(|n| dom_node(n.id,&map,depth-1)).into_iter().collect();
                Ok(json!({"root":{"nodeId":1,"backendNodeId":1,"nodeType":9,"nodeName":"#document","localName":"","nodeValue":"","documentURL":self.url(),"baseURL":self.url(),"xmlVersion":"1.0","childNodeCount":1,"children":children}}))
            }
            "DOM.describeNode" | "DOM.requestChildNodes" => {
                let id = node_id(p)?; let depth = depth(p,1)?;
                let nodes = self.inspector.snapshot()?;
                let map: HashMap<_,_> = nodes.iter().map(|n| (n.id,n)).collect();
                if id == 1 {
                    if method == "DOM.describeNode" { return self.command("DOM.getDocument",p).map(|v| json!({"node":v["root"]})); }
                    let children: Vec<_> = nodes.first().map(|n| dom_node(n.id,&map,depth-1)).into_iter().collect();
                    self.events.push(json!({"method":"DOM.setChildNodes","params":{"parentId":1,"nodes":children}}));
                } else {
                    self.inspector.element(id)?;
                    if method == "DOM.describeNode" { return Ok(json!({"node":dom_node(id,&map,depth)})); }
                    let children: Vec<_> = map[&id].children.iter().map(|id| dom_node(*id,&map,depth-1)).collect();
                    self.events.push(json!({"method":"DOM.setChildNodes","params":{"parentId":id,"nodes":children}}));
                }
                Ok(json!({}))
            }
            "DOM.pushNodeByPathToFrontend" => {
                let path = string(p, "path")?;
                let parts: Vec<_> = path.split(',').collect();
                if parts.is_empty() || parts.len() % 2 != 0 || parts.len() > 512 { return Err(Error::params("Invalid node path")); }
                let nodes = self.inspector.snapshot()?;
                let map: HashMap<_,_> = nodes.iter().map(|n|(n.id,n)).collect();
                let mut children: Vec<_> = nodes.first().map(|n|n.id).into_iter().collect();
                let mut id = 0;
                for pair in parts.as_chunks::<2>().0.iter() {
                    let index: usize = pair[0].parse().map_err(|_|Error::params("Invalid node path index"))?;
                    let Some(node) = children.get(index).and_then(|id|map.get(id)) else { return Ok(json!({"nodeId":0})); };
                    if node.name != pair[1] { return Ok(json!({"nodeId":0})); }
                    id = node.id; children = node.children.clone();
                }
                self.command("DOM.pushNodesByBackendIdsToFrontend", &json!({"backendNodeIds":[id]}))?;
                Ok(json!({"nodeId":id}))
            }
            "DOM.pushNodesByBackendIdsToFrontend" => {
                let requested = p["backendNodeIds"].as_array().ok_or_else(|| Error::params("backendNodeIds must be an array"))?;
                let ids: Vec<u64> = requested.iter().map(|id| id.as_u64().ok_or_else(||Error::params("Invalid backendNodeId"))).collect::<Result<_>>()?;
                let nodes = self.inspector.snapshot()?;
                let map: HashMap<_,_> = nodes.iter().map(|n|(n.id,n)).collect();
                let result: Vec<_> = ids.iter().map(|id| if *id == 1 || map.contains_key(id) { *id } else { 0 }).collect();
                let mut parents = HashSet::new();
                for id in &result {
                    let mut current = map.get(id).and_then(|n|n.parent);
                    while let Some(parent) = current { parents.insert(parent); current = map.get(&parent).and_then(|n|n.parent); }
                }
                if result.iter().any(|id|*id>1) {
                    let roots: Vec<_> = nodes.first().map(|n|dom_node(n.id,&map,0)).into_iter().collect();
                    self.events.push(json!({"method":"DOM.setChildNodes","params":{"parentId":1,"nodes":roots}}));
                    for n in nodes.iter().filter(|n|parents.contains(&n.id)) {
                        let children: Vec<_> = n.children.iter().map(|id|dom_node(*id,&map,0)).collect();
                        self.events.push(json!({"method":"DOM.setChildNodes","params":{"parentId":n.id,"nodes":children}}));
                    }
                }
                Ok(json!({"nodeIds":result}))
            }
            "DOM.getAttributes" => {
                let id = node_id(p)?;
                let nodes = self.inspector.snapshot()?;
                Ok(json!({"attributes":nodes.iter().find(|n|n.id == id).ok_or_else(||Error::server("Unknown node"))?.attributes}))
            }
            "DOM.querySelector" | "DOM.querySelectorAll" => {
                let scope = node_id(p)?; let selector = string(p,"selector")?;
                let ids = self.inspector.query(if scope==1 {None} else {Some(scope)},selector)?;
                Ok(if method.ends_with("All") { json!({"nodeIds":ids}) } else { json!({"nodeId":ids.first().copied().unwrap_or(0)}) })
            }
            "DOM.setAttributeValue" => { self.inspector.set_attribute(node_id(p)?,string(p,"name")?,string(p,"value")?)?; Ok(json!({})) }
            "DOM.getBoxModel" => {
                let b = self.inspector.box_model(node_id(p)?)?;
                Ok(json!({"model":{"content":quad(b.content),"padding":quad(b.padding),"border":quad(b.border),"margin":quad(b.margin),"width":b.border.width.round() as i64,"height":b.border.height.round() as i64}}))
            }
            "DOM.getContentQuads" => Ok(json!({"quads":[quad(self.inspector.box_model(node_id(p)?)?.content)]})),
            "DOM.getNodeForLocation" => {
                let root = self.inspector.root()?;
                let visual = root.get_visual_at(Point::new(number(p,"x")?,number(p,"y")?)).ok_or_else(||Error::server("No node at point"))?;
                let id = self.inspector.id_for(&visual.upcast::<StyledElement>());
                Ok(json!({"nodeId":id,"backendNodeId":id,"frameId":self.target_id}))
            }
            "DOM.focus" => { self.inspector.element(node_id(p)?)?.cast::<InputElement>().ok_or_else(||Error::server("Node cannot receive input"))?.focus(); Ok(json!({})) }
            "CSS.getComputedStyleForNode" => Ok(json!({"computedStyle":self.inspector.properties(node_id(p)?)?.iter().map(|p|json!({"name":p.name,"value":p.value})).collect::<Vec<_>>()})),
            "CSS.getInlineStylesForNode" => {
                let id = node_id(p)?;
                Ok(json!({"inlineStyle":style(self.inspector.properties(id)?.into_iter().filter(|p|p.priority=="LocalValue").map(|p|json!({"name":p.name,"value":p.value})).collect())}))
            }
            "CSS.getMatchedStylesForNode" => {
                let id = node_id(p)?;
                let rules: Vec<_> = self.inspector.matched_styles(id)?.into_iter().map(|s| json!({"rule":{"selectorList":{"selectors":[{"text":s.selector}],"text":s.selector},"origin":"regular","style":style(s.properties.into_iter().map(|(name,value)|json!({"name":name,"value":value})).collect())},"matchingSelectors":[0]})).collect();
                Ok(json!({"matchedCSSRules":rules,"inherited":[],"pseudoElements":[],"inlineStyle":style(self.inspector.properties(id)?.into_iter().filter(|p|p.priority=="LocalValue").map(|p|json!({"name":p.name,"value":p.value})).collect())}))
            }
            "Overlay.highlightNode" => { self.inspector.highlight(node_id(p)?)?; Ok(json!({})) }
            "Overlay.hideHighlight" => { self.inspector.hide_highlight(); Ok(json!({})) }
            "Overlay.setInspectMode" => { self.inspect_mode(string(p,"mode")?)?; Ok(json!({})) }
            "Page.captureScreenshot" => {
                if p["format"].as_str().is_some_and(|f|f!="png") || p.get("clip").is_some() { return Err(Error::params("Only full-window PNG screenshots are supported")); }
                Ok(json!({"data":base64::engine::general_purpose::STANDARD.encode(self.inspector.screenshot()?)}))
            }
            "Page.getLayoutMetrics" => {
                let bounds = Rect::from_size(self.inspector.root()?.bounds().size());
                let layout = json!({"pageX":0,"pageY":0,"clientWidth":bounds.width,"clientHeight":bounds.height});
                let visual = json!({"offsetX":0,"offsetY":0,"pageX":0,"pageY":0,"clientWidth":bounds.width,"clientHeight":bounds.height,"scale":1,"zoom":1});
                Ok(json!({"layoutViewport":layout,"visualViewport":visual,"contentSize":rect(bounds),"cssLayoutViewport":layout,"cssVisualViewport":visual,"cssContentSize":rect(bounds)}))
            }
            "Page.getFrameTree" | "Page.getResourceTree" => Ok(json!({"frameTree":{"resources":[],"frame":{"id":self.target_id,"loaderId":self.target_id,"url":self.url(),"domainAndRegistry":"","securityOrigin":"ferroui://","mimeType":"application/xml","secureContextType":"Secure","crossOriginIsolatedContextType":"NotIsolated","gatedAPIFeatures":[]}}})),
            "Accessibility.getFullAXTree" | "Accessibility.queryAXTree" => self.accessibility(p,method.ends_with("queryAXTree")),
            "Runtime.evaluate" | "Runtime.callFunctionOn" => Err(Error::server("FerroUI has no JavaScript runtime. Use DOM, CSS and Input commands through a CDP session.")),
            "Schema.getDomains" => {
                let schema: Value = serde_json::from_str(include_str!("protocol.json")).expect("embedded protocol");
                Ok(json!({"domains":schema["domains"].as_array().unwrap().iter().map(|d|json!({"name":d["domain"],"version":"1.3"})).collect::<Vec<_>>()}))
            }
            "Runtime.runIfWaitingForDebugger" => Ok(json!({})),
            // These client setup commands have no effect on a native, non-navigating target.
            "Page.setLifecycleEventsEnabled" | "Page.setInterceptFileChooserDialog" | "Emulation.setFocusEmulationEnabled" | "Emulation.setEmulatedMedia" | "Network.enable" | "Network.disable" | "Target.setAutoAttach" => Ok(json!({})),
            "Page.addScriptToEvaluateOnNewDocument" if p["source"].as_str() == Some("") => Ok(json!({"identifier":"empty-script"})),
            "Page.addScriptToEvaluateOnNewDocument" => Err(Error::server("FerroUI has no document scripts")),
            method if method.starts_with("Input.") => self.input.dispatch(&self.inspector.root()?,method,p),
            _ => Err(Error::method(method)),
        }
    }
    fn url(&self) -> String {
        format!("ferroui://{}", self.target_id)
    }
    fn stop_inspecting(&mut self) {
        for s in self.inspect_subscriptions.drain(..) {
            s.dispose();
        }
        self.picked.borrow_mut().clear();
    }
    fn inspect_mode(&mut self, mode: &str) -> Result<()> {
        if !matches!(mode, "none" | "searchForNode") {
            return Err(Error::params("mode must be none or searchForNode"));
        }
        self.stop_inspecting();
        self.inspector.hide_highlight();
        if mode == "none" {
            return Ok(());
        }
        let root = self.inspector.root()?;
        let queue = self.picked.clone();
        let weak = root.downgrade();
        let token = root.add_handler_with(
            InputElement::pointer_moved_event(),
            move |_, e| {
                if let Some(root) = weak.upgrade() {
                    let mut q = queue.borrow_mut();
                    if q.len() < 64 {
                        q.push((e.get_position(Some(&root)), false));
                    }
                }
            },
            RoutingStrategies::TUNNEL,
            true,
        );
        let weak = root.downgrade();
        self.inspect_subscriptions.push(Disposable::create(move || {
            if let Some(root) = weak.upgrade() {
                root.remove_handler(InputElement::pointer_moved_event(), token);
            }
        }));
        let queue = self.picked.clone();
        let weak = root.downgrade();
        let token = root.add_handler_with(
            InputElement::pointer_pressed_event(),
            move |_, e| {
                if let Some(root) = weak.upgrade() {
                    let mut q = queue.borrow_mut();
                    if q.len() < 64 {
                        q.push((e.get_position(Some(&root)), true));
                    }
                    e.set_handled(true);
                }
            },
            RoutingStrategies::TUNNEL,
            true,
        );
        let weak = root.downgrade();
        self.inspect_subscriptions.push(Disposable::create(move || {
            if let Some(root) = weak.upgrade() {
                root.remove_handler(InputElement::pointer_pressed_event(), token);
            }
        }));
        Ok(())
    }
    /// Poll on the UI thread, typically every 100 ms. Tree changes invalidate the document.
    pub fn poll(&mut self) -> Vec<Value> {
        let points = std::mem::take(&mut *self.picked.borrow_mut());
        for (point, pressed) in points {
            if let Ok(root) = self.inspector.root() {
                if let Some(visual) = root.get_visual_at(point) {
                    let id = self.inspector.id_for(&visual.upcast::<StyledElement>());
                    let _ = self.inspector.highlight(id);
                    self.events.push(if pressed { json!({"method":"Overlay.inspectNodeRequested","params":{"backendNodeId":id}}) } else { json!({"method":"Overlay.nodeHighlightRequested","params":{"nodeId":id}}) });
                }
            }
        }
        if self.enabled.contains("DOM") {
            if let Ok(nodes) = self.inspector.snapshot() {
                if let Some(previous) = &self.last_tree {
                    let topology_changed = previous.len() != nodes.len()
                        || previous.iter().zip(&nodes).any(|(a, b)| {
                            a.id != b.id
                                || a.parent != b.parent
                                || a.name != b.name
                                || a.children != b.children
                        });
                    if topology_changed {
                        self.events
                            .push(json!({"method":"DOM.documentUpdated","params":{}}));
                    } else {
                        for (before, after) in previous.iter().zip(&nodes) {
                            let old: HashMap<_, _> = before
                                .attributes
                                .as_chunks::<2>()
                                .0
                                .iter()
                                .map(|p| (&p[0], &p[1]))
                                .collect();
                            let new: HashMap<_, _> = after
                                .attributes
                                .as_chunks::<2>()
                                .0
                                .iter()
                                .map(|p| (&p[0], &p[1]))
                                .collect();
                            for (name, value) in &new {
                                if old.get(name) != Some(value) {
                                    self.events.push(json!({"method":"DOM.attributeModified","params":{"nodeId":after.id,"name":name,"value":value}}));
                                }
                            }
                            for name in old.keys().filter(|name| !new.contains_key(*name)) {
                                self.events.push(json!({"method":"DOM.attributeRemoved","params":{"nodeId":after.id,"name":name}}));
                            }
                        }
                    }
                }
                self.last_tree = Some(nodes);
            }
        }
        std::mem::take(&mut self.events)
    }
    fn accessibility(&mut self, p: &Value, query: bool) -> Result<Value> {
        let root: Ref<Control> =
            if query && (p.get("nodeId").is_some() || p.get("backendNodeId").is_some()) {
                self.inspector
                    .element(node_id(p)?)?
                    .cast::<Control>()
                    .ok_or_else(|| Error::server("Node has no automation peer"))?
            } else {
                self.inspector.root()?.upcast()
            };
        let peer = ControlAutomationPeer::create_peer_for_element(&root);
        let max_depth = if p.get("depth").is_some() {
            depth(p, 256)?
        } else {
            256
        };
        let mut pending: Vec<(Ref<AutomationPeer>, Option<String>, i64)> = vec![(peer, None, 0)];
        let mut nodes = Vec::new();
        let mut seen = HashSet::new();
        while let Some((peer, parent, d)) = pending.pop() {
            let Some(owner) = peer
                .cast::<ControlAutomationPeer>()
                .and_then(|p| p.try_owner())
            else {
                continue;
            };
            let id = self.inspector.id_for(&owner.upcast::<StyledElement>());
            if !seen.insert(id) {
                continue;
            }
            let children = if d < max_depth {
                peer.get_children().iter().cloned().collect::<Vec<_>>()
            } else {
                vec![]
            };
            let child_ids: Vec<_> = children
                .iter()
                .filter_map(|c| {
                    c.cast::<ControlAutomationPeer>()
                        .and_then(|p| p.try_owner())
                })
                .map(|c| {
                    self.inspector
                        .id_for(&c.upcast::<StyledElement>())
                        .to_string()
                })
                .collect();
            let role = format!("{:?}", peer.get_automation_control_type());
            let name = peer.get_name();
            if !query
                || (p["accessibleName"].as_str().is_none_or(|n| n == name)
                    && p["role"]
                        .as_str()
                        .is_none_or(|r| r.eq_ignore_ascii_case(&role)))
            {
                let mut node = json!({"nodeId":id.to_string(),"backendDOMNodeId":id,"ignored":false,"role":{"type":"role","value":role},"name":{"type":"computedString","value":name},"description":{"type":"computedString","value":peer.get_help_text()},"childIds":child_ids,"properties":[{"name":"disabled","value":{"type":"boolean","value":!peer.is_enabled()}},{"name":"focusable","value":{"type":"boolean","value":peer.is_keyboard_focusable()}},{"name":"focused","value":{"type":"boolean","value":peer.has_keyboard_focus()}}]});
                if let Some(parent) = parent {
                    node["parentId"] = json!(parent);
                }
                nodes.push(node);
            }
            pending.extend(
                children
                    .into_iter()
                    .rev()
                    .map(|c| (c, Some(id.to_string()), d + 1)),
            );
        }
        Ok(json!({"nodes":nodes}))
    }
}
impl Drop for CdpSession {
    fn drop(&mut self) {
        self.stop_inspecting();
    }
}
pub(crate) fn response(id: Value, result: Result<Value>) -> Value {
    match result {
        Ok(result) => json!({"id":id,"result":result}),
        Err(e) => json!({"id":id,"error":{"code":e.code,"message":e.message}}),
    }
}
