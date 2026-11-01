//! Host-owned asynchronous QML resource loading. Transport callbacks deal only in
//! bytes and URLs. Parsing, linking, status changes and handlers run on the owner.
use super::{dynamic::Template, ir, Error, Instance, ObjectId, TypeRegistry};
use std::{
    collections::{BTreeMap, HashMap, HashSet, VecDeque},
    rc::Rc,
    sync::{Arc, Mutex, Weak},
};
pub use url::Url;

#[cfg(all(feature = "http", not(target_arch = "wasm32")))]
#[path = "network_http.rs"]
mod http;
#[cfg(all(feature = "http", not(target_arch = "wasm32")))]
pub use http::HttpComponentTransport;

pub const MAX_DOCUMENT_BYTES: usize = 2 * 1024 * 1024;
const MAX_GRAPH_BYTES: usize = 8 * 1024 * 1024;
const MAX_DOCUMENTS: usize = 64;
const MAX_LOADS: usize = 16;

#[derive(Debug)]
pub enum ResourceError {
    NotFound,
    Failed(String),
}

/// Start a nonblocking read. The Rust host decides which origins are reachable.
/// Complete the reply exactly once. A reply may be moved to another thread.
pub trait ComponentTransport {
    fn request(&self, url: &Url, reply: ComponentReply);
}

type Wake = Arc<dyn Fn() + Send + Sync>;
type Response = Result<Vec<u8>, ResourceError>;

#[derive(Default)]
struct Inbox {
    waiting: HashSet<ObjectId>,
    ready: VecDeque<(ObjectId, Response)>,
    wake: Option<Wake>,
}

pub struct ComponentReply {
    id: ObjectId,
    inbox: Weak<Mutex<Inbox>>,
}

impl ComponentReply {
    /// False after the factory or instance has been destroyed.
    pub fn is_pending(&self) -> bool {
        self.inbox
            .upgrade()
            .is_some_and(|inbox| inbox.lock().unwrap().waiting.contains(&self.id))
    }

    pub fn complete(mut self, result: Result<Vec<u8>, ResourceError>) {
        self.send(result);
    }

    fn send(&mut self, mut result: Response) {
        let Some(inbox) = std::mem::take(&mut self.inbox).upgrade() else {
            return;
        };
        let wake = {
            let mut inbox = inbox.lock().unwrap();
            if !inbox.waiting.remove(&self.id) {
                return;
            }
            if matches!(&result, Ok(bytes) if bytes.len() > MAX_DOCUMENT_BYTES) {
                result = Err(ResourceError::Failed(
                    "remote QML file exceeds the size limit".into(),
                ));
            }
            inbox.ready.push_back((self.id, result));
            inbox.wake.clone()
        };
        if let Some(wake) = wake {
            wake()
        }
    }
}

impl Drop for ComponentReply {
    fn drop(&mut self) {
        if self.inbox.strong_count() > 0 {
            self.send(Err(ResourceError::Failed(
                "transport dropped the request".into(),
            )));
        }
    }
}

#[derive(Default)]
pub(crate) struct Network {
    inbox: Arc<Mutex<Inbox>>,
    graphs: BTreeMap<ObjectId, Graph>,
}

pub(crate) struct Graph {
    entry: Url,
    pending: Url,
    files: HashMap<Url, Option<Rc<ir::Document>>>,
    bytes: usize,
}

enum Progress {
    Fetch(Url),
    Ready(Template),
}

impl Network {
    pub(crate) fn retire(&mut self, id: ObjectId) {
        if self.graphs.remove(&id).is_some() {
            let mut inbox = self.inbox.lock().unwrap();
            inbox.waiting.remove(&id);
            inbox.ready.retain(|(owner, _)| *owner != id);
        }
    }
    pub(crate) fn count(&self) -> usize {
        self.graphs.len()
    }
    pub(crate) fn start(&mut self, transport: &dyn ComponentTransport, id: ObjectId, url: Url) {
        self.graphs.insert(
            id,
            Graph {
                entry: url.clone(),
                pending: url.clone(),
                files: HashMap::new(),
                bytes: 0,
            },
        );
        self.request(transport, id, &url);
    }
    fn request(&self, transport: &dyn ComponentTransport, id: ObjectId, url: &Url) {
        self.inbox.lock().unwrap().waiting.insert(id);
        transport.request(
            url,
            ComponentReply {
                id,
                inbox: Arc::downgrade(&self.inbox),
            },
        );
    }
}

impl Instance {
    pub fn has_pending_component_loads(&self) -> bool {
        self.network.count() != 0
    }

    pub fn set_component_waker(&mut self, wake: Option<Arc<dyn Fn() + Send + Sync>>) {
        let ready = {
            let mut inbox = self.network.inbox.lock().unwrap();
            inbox.wake = wake.clone();
            !inbox.ready.is_empty()
        };
        if ready {
            if let Some(wake) = wake {
                wake()
            }
        }
    }

    /// Drain completed transport operations. The returned count includes
    /// dependency reads. A terminal loading error is exposed on the Component.
    pub fn poll_component_loads(&mut self, limit: usize) -> Result<usize, Error> {
        let mut count = 0;
        while count < limit {
            let Some((id, result)) = self.network.inbox.lock().unwrap().ready.pop_front() else {
                break;
            };
            count += 1;
            let graph = self
                .network
                .graphs
                .get_mut(&id)
                .expect("live component request");
            let types = self.dynamic.registry.as_ref().expect("type registry");
            let step = graph.receive(result, types, self.javascript.engine.is_some());
            match step {
                Ok(Progress::Fetch(url)) => {
                    graph.pending = url.clone();
                    self.network.request(
                        types
                            .component_transport
                            .as_deref()
                            .expect("component transport"),
                        id,
                        &url,
                    );
                }
                done => {
                    self.network.retire(id);
                    match done {
                        Ok(Progress::Ready(template)) => {
                            self.dynamic.file_errors.remove(&id);
                            self.dynamic.templates.insert(id, template);
                        }
                        Err(error) => {
                            self.dynamic.file_errors.insert(id, error);
                        }
                        _ => unreachable!(),
                    }
                    if let Err(error) = self.transaction(|tx| tx.finish_factory(id)) {
                        self.dynamic.templates.remove(&id);
                        self.dynamic.file_errors.insert(id, error.to_string());
                        self.publish_then_notify(|tx| tx.finish_factory(id))?;
                        return Err(error);
                    }
                }
            }
        }
        Ok(count)
    }
}

impl Graph {
    fn receive(
        &mut self,
        result: Response,
        types: &TypeRegistry,
        javascript: bool,
    ) -> Result<Progress, String> {
        let url = &self.pending;
        let document = match result {
            Err(ResourceError::NotFound) if url != &self.entry => None,
            Err(ResourceError::NotFound) => return Err(format!("{url}: resource not found")),
            Err(ResourceError::Failed(error)) => return Err(format!("{url}: {error}")),
            Ok(bytes) => {
                self.bytes += bytes.len();
                if self.bytes > MAX_GRAPH_BYTES {
                    return Err(format!(
                        "{url}: QML dependency graph exceeds the size limit"
                    ));
                }
                let text = std::str::from_utf8(&bytes).map_err(|e| format!("{url}: {e}"))?;
                Some(Rc::new(parse(url, text, javascript)?))
            }
        };
        self.files.insert(url.clone(), document);
        let root = self.files[&self.entry].clone().expect("root QML document");
        let mut resolver = Resolver {
            files: &self.files,
            types,
            missing: None,
        };
        let source = ir::ComponentSource {
            key: self.entry.to_string(),
            document: root,
        };
        let linked = ir::link(source, &mut resolver);
        if let Some(next) = resolver.missing {
            if self.files.len() >= MAX_DOCUMENTS {
                return Err(format!("{}: too many QML dependencies", self.entry));
            }
            return Ok(Progress::Fetch(next));
        }
        let doc = linked.map_err(|e| e.to_string())?;
        // Validate native contracts without instantiating the component or running user code.
        // Required root inputs are supplied later to createObject or Loader.setSource.
        types
            .check(&doc)
            .map_err(|e| format!("{}: {e}", self.entry))?;
        Template::from_object(doc.root, &doc.imports)
            .map(Progress::Ready)
            .map_err(|e| e.to_string())
    }
}

fn parse(url: &Url, source: &str, _javascript: bool) -> Result<ir::Document, String> {
    #[cfg(feature = "javascript")]
    let parsed = if _javascript {
        ir::parse_external(source)
    } else {
        ir::parse(source)
    };
    #[cfg(not(feature = "javascript"))]
    let parsed = ir::parse(source);
    let mut doc = parsed.map_err(|e| format!("{url}: {e}"))?;
    if doc.imports.iter().any(|i| i.is_script()) {
        return Err(format!(
            "{url}: remote JavaScript module imports require a compiled resource catalog"
        ));
    }
    doc.root.set_source_url(url.as_str());
    #[cfg(feature = "javascript")]
    if _javascript {
        ir::javascript::visit_scripts(&mut doc.root, &mut |script| {
            script.origin = url.to_string();
            Ok::<_, String>(())
        })?;
    }
    Ok(doc)
}

struct Resolver<'a> {
    files: &'a HashMap<Url, Option<Rc<ir::Document>>>,
    types: &'a TypeRegistry,
    missing: Option<Url>,
}

impl ir::ComponentResolver for Resolver<'_> {
    fn resolve(
        &mut self,
        from: &str,
        imports: &[ir::Import],
        name: &str,
    ) -> Result<Option<ir::ComponentSource>, String> {
        if self.types.resolve(imports, name).is_ok() {
            return Ok(None);
        }
        let (namespace, ty) = name
            .split_once('.')
            .map_or((None, name), |(ns, ty)| (Some(ns), ty));
        let base = Url::parse(from).map_err(|e| e.to_string())?;
        let mut candidates = Vec::new();
        if namespace.is_none() {
            candidates.push(base.join(&format!("{ty}.qml")).map_err(|e| e.to_string())?);
        }
        for import in imports
            .iter()
            .filter(|i| i.is_path && i.alias.as_deref() == namespace)
        {
            let path = format!("{}/{ty}.qml", import.uri.trim_end_matches('/'));
            candidates.push(base.join(&path).map_err(|e| e.to_string())?);
        }
        let mut found = None;
        for url in candidates {
            if !matches!(url.scheme(), "http" | "https") || url.origin() != base.origin() {
                return Err(format!("{from}: cross-origin component dependency {url}"));
            }
            match self.files.get(&url) {
                Some(Some(doc)) => {
                    if found
                        .as_ref()
                        .is_some_and(|s: &ir::ComponentSource| s.key != url.as_str())
                    {
                        return Err(format!("{from}: ambiguous component {name}"));
                    }
                    found = Some(ir::ComponentSource {
                        key: url.to_string(),
                        document: doc.clone(),
                    });
                }
                Some(None) => {}
                None => {
                    self.missing = Some(url);
                    return Err("pending network dependency".into());
                }
            }
        }
        found
            .ok_or_else(|| format!("{from}: unresolved QML type {name}"))
            .map(Some)
    }
}

pub(crate) fn allow_request(current: usize) -> Result<(), Error> {
    if current >= MAX_LOADS {
        Err(Error::new("too many pending component loads"))
    } else {
        Ok(())
    }
}
