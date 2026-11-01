//! Off-thread script actors exchange copied values with the QML owner thread.
//! Source changes and outgoing messages take effect only after their originating
//! transaction commits. The owner thread explicitly pumps incoming deliveries.
use super::*;
use crate::transaction::Transaction;
use serde::{Deserialize, Serialize};
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex, Weak},
};

const EVENT_LIMIT: usize = 1024;
const BYTE_LIMIT: usize = 32 * 1024 * 1024;
type Wake = Arc<dyn Fn() + Send + Sync>;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "t", content = "v")]
pub enum WorkerValue {
    Undefined,
    Null,
    Bool(bool),
    Number(String),
    String(String),
    Array(Vec<Self>),
    Object(BTreeMap<String, Self>),
}
impl WorkerValue {
    pub fn estimated_bytes(&self) -> usize {
        let body = match self {
            Self::String(s) | Self::Number(s) => s.len(),
            Self::Array(values) => values
                .iter()
                .fold(0usize, |n, v| n.saturating_add(v.estimated_bytes())),
            Self::Object(values) => values.iter().fold(0usize, |n, (k, v)| {
                n.saturating_add(k.len() + 32)
                    .saturating_add(v.estimated_bytes())
            }),
            _ => 0,
        };
        body.saturating_add(std::mem::size_of::<Self>())
    }
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "t", content = "v")]
pub enum WorkerEvent {
    Ready,
    Message(WorkerValue),
    Error(String),
    Stopped(String),
}
pub trait WorkerEndpoint {
    fn send(&self, value: WorkerValue) -> Result<(), Error>;
}

struct Entry {
    generation: u64,
    src: String,
    origin: String,
    endpoint: Option<Box<dyn WorkerEndpoint>>,
}
struct Event {
    id: ObjectId,
    generation: u64,
    event: WorkerEvent,
    bytes: usize,
}
#[derive(Default)]
struct Inbox {
    queue: VecDeque<Event>,
    bytes: usize,
    active: BTreeMap<ObjectId, u64>,
    stopped: BTreeSet<(ObjectId, u64)>,
    wake: Option<Wake>,
}
impl Inbox {
    fn replace(&mut self, id: ObjectId, next: Option<u64>) {
        if let Some(previous) = self.active.remove(&id) {
            self.stopped.remove(&(id, previous));
            self.queue.retain(|event| {
                if event.id == id {
                    self.bytes -= event.bytes;
                    false
                } else {
                    true
                }
            });
        }
        if let Some(generation) = next {
            self.active.insert(id, generation);
        }
    }
    fn pending(&self) -> bool {
        !self.queue.is_empty() || !self.stopped.is_empty()
    }
    fn pop(&mut self) -> Option<Event> {
        if let Some(event) = self.queue.pop_front() {
            self.bytes -= event.bytes;
            return Some(event);
        }
        self.stopped.pop_first().map(|(id, generation)| Event {
            id,
            generation,
            event: WorkerEvent::Stopped("worker output limit exceeded".into()),
            bytes: 0,
        })
    }
}

/// Generation-checked publication capability. Clones never retain the instance.
#[derive(Clone)]
pub struct WorkerDelivery {
    inbox: Weak<Mutex<Inbox>>,
    id: ObjectId,
    generation: u64,
}
impl WorkerDelivery {
    pub fn publish(&self, event: WorkerEvent) -> bool {
        let Some(inbox) = self.inbox.upgrade() else {
            return false;
        };
        let mut q = inbox.lock().unwrap_or_else(|e| e.into_inner());
        if q.active.get(&self.id) != Some(&self.generation)
            || q.stopped.contains(&(self.id, self.generation))
        {
            return false;
        }
        let bytes = std::mem::size_of::<Event>().saturating_add(match &event {
            WorkerEvent::Message(v) => v.estimated_bytes(),
            WorkerEvent::Error(s) | WorkerEvent::Stopped(s) => s.len(),
            WorkerEvent::Ready => 0,
        });
        let fits = q.queue.len() < EVENT_LIMIT && q.bytes.saturating_add(bytes) <= BYTE_LIMIT;
        if fits {
            q.bytes += bytes;
            q.queue.push_back(Event {
                id: self.id,
                generation: self.generation,
                event,
                bytes,
            });
        } else {
            q.stopped.insert((self.id, self.generation));
        }
        let wake = q.wake.clone();
        drop(q);
        if let Some(wake) = wake {
            wake();
        }
        fits
    }
}

#[derive(Default)]
pub(crate) struct Workers {
    entries: BTreeMap<ObjectId, Entry>,
    inbox: Option<Arc<Mutex<Inbox>>>,
    sequence: u64,
}
impl Workers {
    pub(crate) fn truncate(&mut self, first: usize) {
        self.entries.retain(|id, _| id.index < first);
    }

    pub(crate) fn register(&mut self, id: ObjectId, origin: &str) {
        self.inbox
            .get_or_insert_with(Default::default)
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .replace(id, Some(0));
        self.entries.insert(
            id,
            Entry {
                generation: 0,
                src: String::new(),
                origin: origin.to_owned(),
                endpoint: None,
            },
        );
    }
    pub(crate) fn contains(&self, id: ObjectId) -> bool {
        self.entries.contains_key(&id)
    }
}
pub(crate) enum Action {
    Load(ObjectId, String),
    Send(ObjectId, WorkerValue),
}
pub(crate) fn register(types: &mut TypeRegistry) -> Result<(), Error> {
    let ty = TypeDefinition::default()
        .inherits("QtQml", "QtObject")
        .property("source", "url", Value::String(String::new()))
        .readonly_property("ready", "bool", Value::Bool(false))
        .signal(
            "message",
            vec![Parameter {
                name: "messageObject".into(),
                ty: Some("var".into()),
            }],
        );
    types.register("QtQml.WorkerScript", "WorkerScript", ty)?;
    types.set_module_version("QtQml.WorkerScript", 6, 12);
    types.set_module_version("QtQml.WorkerScript", 2, 15);
    types.reexport("QtQml", "QtQml.WorkerScript")
}
impl Instance {
    pub fn has_worker_scripts(&self) -> bool {
        !self.workers.entries.is_empty()
    }
    /// The callback can run on a worker thread and must post to the owner.
    pub fn set_worker_waker(&mut self, waker: Option<Arc<dyn Fn() + Send + Sync>>) {
        let inbox = self.workers.inbox.get_or_insert_with(Default::default);
        let mut q = inbox.lock().unwrap_or_else(|e| e.into_inner());
        q.wake = waker.clone();
        let pending = q.pending();
        drop(q);
        if pending {
            if let Some(wake) = waker {
                wake();
            }
        }
    }
    pub fn poll_worker_scripts(&mut self, limit: usize) -> Result<usize, Error> {
        let Some(inbox) = self.workers.inbox.clone() else {
            return Ok(0);
        };
        let mut count = 0;
        let result = (|| {
            while count < limit {
                let Some(Event {
                    id,
                    generation,
                    event,
                    ..
                }) = inbox.lock().unwrap_or_else(|e| e.into_inner()).pop()
                else {
                    break;
                };
                count += 1;
                let Some(entry) = self.workers.entries.get_mut(&id) else {
                    continue;
                };
                if entry.generation != generation {
                    continue;
                }
                match event {
                    WorkerEvent::Ready => self.transaction(|tx| tx.worker_ready(id, true))?,
                    WorkerEvent::Message(v) => self.transaction(|tx| {
                        let input = tx.import_worker(v, 0)?;
                        tx.signal(id, "message", vec![input])
                    })?,
                    WorkerEvent::Stopped(reason) => {
                        entry.endpoint = None;
                        self.transaction(|tx| tx.worker_ready(id, false))?;
                        return Err(Error::new(format!("WorkerScript: {reason}")));
                    }
                    WorkerEvent::Error(reason) => {
                        return Err(Error::new(format!("WorkerScript: {reason}")))
                    }
                }
            }
            Ok(count)
        })();
        let q = inbox.lock().unwrap_or_else(|e| e.into_inner());
        let wake = if q.pending() { q.wake.clone() } else { None };
        drop(q);
        if let Some(wake) = wake {
            wake();
        }
        result
    }
    pub(crate) fn commit_workers(&mut self, actions: Vec<Action>) {
        for action in actions {
            let id = match &action {
                Action::Load(id, _) | Action::Send(id, _) => *id,
            };
            let Some(entry) = self.workers.entries.get_mut(&id) else {
                continue;
            };
            let inbox = self.workers.inbox.as_ref().expect("registered worker");
            if let Action::Load(_, source) = &action {
                if entry.src == *source {
                    continue;
                }
                entry.endpoint = None;
                entry.src = source.clone();
                self.workers.sequence += 1;
                entry.generation = self.workers.sequence;
                inbox
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .replace(id, Some(entry.generation));
            }
            let events = WorkerDelivery {
                inbox: Arc::downgrade(inbox),
                id,
                generation: entry.generation,
            };
            let result = match action {
                Action::Load(_, src) if !src.is_empty() => self
                    .javascript
                    .engine
                    .as_ref()
                    .ok_or_else(|| Error::new("WorkerScript needs a script engine"))
                    .and_then(|e| {
                        e.start_worker(
                            self.javascript.resources.clone(),
                            &entry.origin,
                            &src,
                            events.clone(),
                        )
                    })
                    .map(|actor| entry.endpoint = Some(actor)),
                Action::Load(..) => Ok(()),
                Action::Send(_, v) => entry
                    .endpoint
                    .as_ref()
                    .ok_or_else(|| Error::new("WorkerScript has no source"))
                    .and_then(|actor| actor.send(v)),
            };
            if let Err(e) = result {
                events.publish(WorkerEvent::Error(e.message));
            }
        }
    }
}
impl Transaction<'_> {
    fn worker_ready(&mut self, id: ObjectId, ready: bool) -> Result<(), Error> {
        let key = self.model.key(id, "ready")?;
        self.set_value(key, Value::Bool(ready));
        Ok(())
    }
    pub(crate) fn initialize_workers(&mut self, first: usize) {
        self.worker_sources
            .extend(self.model.workers.entries.keys().copied().filter(|id| id.index >= first));
    }
    pub(crate) fn stage_workers(&mut self) -> Result<(), Error> {
        while let Some(id) = self.worker_sources.pop_first() {
            let Value::String(source) = self.model.get(id, "source")? else {
                return Err(Error::new("invalid WorkerScript.source"));
            };
            let previous = self
                .worker_actions
                .iter()
                .rev()
                .find_map(|a| match a {
                    Action::Load(w, s) if *w == id => Some(s),
                    _ => None,
                })
                .unwrap_or(&self.model.workers.entries[&id].src);
            if source != *previous {
                self.worker_ready(id, false)?;
                self.worker_actions.push(Action::Load(id, source));
            }
        }
        Ok(())
    }
    pub(crate) fn send_worker(&mut self, id: ObjectId, args: &[Value]) -> Result<Value, Error> {
        if self.in_binding() {
            return Err(Error::new("a binding cannot send worker messages"));
        }
        let [input] = args else {
            return Err(Error::new("WorkerScript.sendMessage expects one message"));
        };
        let value = self.export_worker(input, 0)?;
        self.worker_actions.push(Action::Send(id, value));
        Ok(Value::Undefined)
    }
    fn export_worker(&mut self, input: &Value, depth: usize) -> Result<WorkerValue, Error> {
        self.budget()?;
        if depth >= 64 {
            return Err(Error::new("cyclic or deeply nested worker message"));
        }
        Ok(match input {
            Value::Undefined => WorkerValue::Undefined,
            Value::Null => WorkerValue::Null,
            Value::Bool(b) => WorkerValue::Bool(*b),
            Value::Number(n) => WorkerValue::Number(if *n == 0. && n.is_sign_negative() {
                "-0".into()
            } else {
                input.js_string()
            }),
            Value::String(s) => WorkerValue::String(s.clone()),
            Value::List(values) => WorkerValue::Array(
                values
                    .iter()
                    .map(|v| self.export_worker(v, depth + 1))
                    .collect::<Result<_, _>>()?,
            ),
            Value::Record(r) => {
                let fields = self.model.record_entries(r)?.clone();
                if r.is_array() {
                    WorkerValue::Array(
                        (0..self.model.array_length(r)?)
                            .map(|i| {
                                self.export_worker(
                                    fields.get(&i.to_string()).unwrap_or(&Value::Undefined),
                                    depth + 1,
                                )
                            })
                            .collect::<Result<_, _>>()?,
                    )
                } else {
                    WorkerValue::Object(
                        fields
                            .iter()
                            .map(|(k, v)| Ok((k.clone(), self.export_worker(v, depth + 1)?)))
                            .collect::<Result<_, Error>>()?,
                    )
                }
            }
            _ => {
                return Err(Error::new(
                    "WorkerScript messages require plain objects, arrays or primitives",
                ))
            }
        })
    }
    fn import_worker(&mut self, input: WorkerValue, depth: usize) -> Result<Value, Error> {
        self.budget()?;
        if depth >= 64 {
            return Err(Error::new("deeply nested worker message"));
        }
        Ok(match input {
            WorkerValue::Undefined => Value::Undefined,
            WorkerValue::Null => Value::Null,
            WorkerValue::Bool(b) => Value::Bool(b),
            WorkerValue::String(s) => Value::String(s),
            WorkerValue::Number(n) => {
                Value::Number(n.parse().map_err(|_| Error::new("invalid worker number"))?)
            }
            WorkerValue::Array(values) => {
                let items = values
                    .into_iter()
                    .map(|v| self.import_worker(v, depth + 1))
                    .collect::<Result<_, _>>()?;
                self.new_array(items)
            }
            WorkerValue::Object(values) => {
                let fields = values
                    .into_iter()
                    .map(|(k, v)| Ok((k, self.import_worker(v, depth + 1)?)))
                    .collect::<Result<_, Error>>()?;
                self.new_record(fields)
            }
        })
    }
}
