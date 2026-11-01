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
    Model {
        id: u64,
        revision: u64,
        rows: Vec<BTreeMap<String, Self>>,
    },
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
            Self::Model { rows, .. } => rows.iter().fold(0usize, |n, row| {
                row.iter().fold(n, |total, (k, v)| {
                    total
                        .saturating_add(k.len() + 32)
                        .saturating_add(v.estimated_bytes())
                })
            }),
            _ => 0,
        };
        body.saturating_add(std::mem::size_of::<Self>())
    }
}
#[derive(Debug, Serialize, Deserialize)]
pub struct WorkerEdit {
    pub method: String,
    pub args: Vec<WorkerValue>,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct WorkerModelEdits {
    pub id: u64,
    pub revision: u64,
    pub edits: Vec<WorkerEdit>,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "t", content = "v")]
pub enum WorkerEvent {
    Ready,
    Message(WorkerValue),
    Sync {
        id: u64,
        revision: u64,
        edits: Vec<WorkerEdit>,
    },
    SyncTree(Vec<WorkerModelEdits>),
    Error(String),
    Stopped(String),
}
pub trait WorkerEndpoint {
    fn send(&self, value: WorkerValue) -> Result<(), Error>;
}

fn edit_bytes(edits: &[WorkerEdit]) -> usize {
    edits.iter().fold(0usize, |n, e| {
        e.args
            .iter()
            .fold(n.saturating_add(e.method.len() + 32), |n, v| {
                n.saturating_add(v.estimated_bytes())
            })
    })
}
struct Entry {
    generation: u64,
    src: String,
    origin: String,
    endpoint: Option<Box<dyn WorkerEndpoint>>,
    models: BTreeMap<u64, ObjectId>,
}
struct ModelAck {
    complete: Box<dyn FnOnce(Result<Vec<u64>, Error>) + Send>,
    pending: Arc<std::sync::atomic::AtomicBool>,
}

/// Dropping the waiter revokes a sync request that has not yet started on the
/// owner thread. A timeout cannot silently mutate the model on a later poll.
pub struct WorkerSync {
    done: std::sync::mpsc::Receiver<Result<Vec<u64>, Error>>,
    _request: WorkerSyncRequest,
}
/// Keep a request alive until the QML owner commits its model batch. Dropping
/// it before delivery revokes the batch, including after a browser deadline.
pub struct WorkerSyncRequest {
    pending: Arc<std::sync::atomic::AtomicBool>,
}
impl WorkerSync {
    pub fn recv_timeout(
        &self,
        timeout: std::time::Duration,
    ) -> Result<Result<u64, Error>, std::sync::mpsc::RecvTimeoutError> {
        self.recv_revisions(timeout).map(|result| {
            result.and_then(|all| {
                all.first()
                    .copied()
                    .ok_or_else(|| Error::new("empty synchronization receipt"))
            })
        })
    }

    pub fn recv_revisions(
        &self,
        timeout: std::time::Duration,
    ) -> Result<Result<Vec<u64>, Error>, std::sync::mpsc::RecvTimeoutError> {
        self.done.recv_timeout(timeout)
    }
}
impl Drop for WorkerSyncRequest {
    fn drop(&mut self) {
        self.pending
            .store(false, std::sync::atomic::Ordering::Release);
    }
}
struct Event {
    id: ObjectId,
    generation: u64,
    event: WorkerEvent,
    bytes: usize,
    ack: Option<ModelAck>,
}
#[derive(Default)]
struct Inbox {
    queue: VecDeque<Event>,
    // An overflowing generation gets one terminal marker at its exact output
    // boundary. Markers are separately bounded by the number of live workers.
    data_count: usize,
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
                    self.data_count -= usize::from(event.bytes != 0);
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
        !self.queue.is_empty()
    }
    fn pop(&mut self) -> Option<Event> {
        let event = self.queue.pop_front()?;
        self.bytes -= event.bytes;
        self.data_count -= usize::from(event.bytes != 0);
        Some(event)
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
        self.enqueue(event, None)
    }
    pub fn synchronize(
        &self,
        id: u64,
        revision: u64,
        edits: Vec<WorkerEdit>,
    ) -> Result<WorkerSync, Error> {
        self.synchronize_tree(vec![WorkerModelEdits {
            id,
            revision,
            edits,
        }])
    }
    pub fn synchronize_tree(&self, models: Vec<WorkerModelEdits>) -> Result<WorkerSync, Error> {
        let (send, done) = std::sync::mpsc::channel();
        let request = self.synchronize_tree_with(models, move |result| {
            let _ = send.send(result);
        })?;
        Ok(WorkerSync {
            done,
            _request: request,
        })
    }

    /// Deliver the revision receipt asynchronously on the owner thread. A
    /// retired worker drops its completion callback and invalidates the request.
    pub fn synchronize_tree_with(
        &self,
        models: Vec<WorkerModelEdits>,
        complete: impl FnOnce(Result<Vec<u64>, Error>) + Send + 'static,
    ) -> Result<WorkerSyncRequest, Error> {
        let pending = Arc::new(std::sync::atomic::AtomicBool::new(true));
        let ack = ModelAck {
            complete: Box::new(complete),
            pending: pending.clone(),
        };
        self.enqueue(WorkerEvent::SyncTree(models), Some(ack))
            .then_some(WorkerSyncRequest { pending })
            .ok_or_else(|| Error::new("WorkerScript receiver unavailable"))
    }
    fn enqueue(&self, event: WorkerEvent, ack: Option<ModelAck>) -> bool {
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
            WorkerEvent::Sync { edits, .. } => edit_bytes(edits),
            WorkerEvent::SyncTree(models) => models.iter().fold(0usize, |n, m| {
                n.saturating_add(32).saturating_add(edit_bytes(&m.edits))
            }),
            WorkerEvent::Ready => 0,
        });
        let fits = q.data_count < EVENT_LIMIT && q.bytes.saturating_add(bytes) <= BYTE_LIMIT;
        if fits {
            q.data_count += 1;
            q.bytes += bytes;
            q.queue.push_back(Event {
                id: self.id,
                generation: self.generation,
                event,
                ack,
                bytes,
            });
        } else {
            q.stopped.insert((self.id, self.generation));
            q.queue.push_back(Event {
                id: self.id,
                generation: self.generation,
                bytes: 0,
                ack: None,
                event: WorkerEvent::Stopped("worker output limit exceeded".into()),
            });
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
    pub(crate) fn retire_model_grants(&mut self, dead: &BTreeSet<ObjectId>) {
        for entry in self.entries.values_mut() {
            entry.models.retain(|_, id| !dead.contains(id));
        }
    }
    pub(crate) fn unregister(&mut self, id: ObjectId) {
        if self.entries.remove(&id).is_some() {
            if let Some(inbox) = &self.inbox {
                inbox
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .replace(id, None);
            }
        }
    }
    pub(crate) fn truncate(&mut self, first: usize) {
        let retired: Vec<_> = self
            .entries
            .keys()
            .filter(|id| id.index >= first)
            .copied()
            .collect();
        for id in retired {
            self.unregister(id);
        }
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
                models: Default::default(),
            },
        );
    }
    pub(crate) fn contains(&self, id: ObjectId) -> bool {
        self.entries.contains_key(&id)
    }
}
pub(crate) enum Action {
    Load(ObjectId, String),
    Send(ObjectId, WorkerValue, BTreeMap<u64, ObjectId>),
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
                    ack,
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
                        let input = tx.import_worker(id, v, 0)?;
                        tx.signal(id, "message", vec![input])
                    })?,
                    WorkerEvent::Sync {
                        id: model,
                        revision,
                        edits,
                    } => {
                        self.apply_worker_batch(
                            id,
                            vec![WorkerModelEdits {
                                id: model,
                                revision,
                                edits,
                            }],
                            ack,
                        )?;
                    }
                    WorkerEvent::SyncTree(models) => self.apply_worker_batch(id, models, ack)?,
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
    fn apply_worker_batch(
        &mut self,
        worker: ObjectId,
        models: Vec<WorkerModelEdits>,
        ack: Option<ModelAck>,
    ) -> Result<(), Error> {
        if ack
            .as_ref()
            .is_some_and(|a| !a.pending.swap(false, std::sync::atomic::Ordering::AcqRel))
        {
            return Ok(());
        }
        let result = self.apply_worker_edits(worker, models);
        if let Some(ack) = ack {
            (ack.complete)(result);
            Ok(())
        } else {
            result.map(drop)
        }
    }

    fn apply_worker_edits(
        &mut self,
        worker: ObjectId,
        models: Vec<WorkerModelEdits>,
    ) -> Result<Vec<u64>, Error> {
        if models.is_empty() {
            return Err(Error::new("empty worker synchronization"));
        }
        let grants = &self.workers.entries[&worker].models;
        let mut unique = BTreeSet::new();
        let mut batch = Vec::with_capacity(models.len());
        for model in models {
            let id = *grants
                .get(&model.id)
                .ok_or_else(|| Error::new("unknown worker ListModel"))?;
            if !unique.insert(id) || self.worker_model_revision(id)? != model.revision {
                return Err(Error::new("ListModel changed before WorkerScript.sync"));
            }
            batch.push((id, model.edits));
        }
        self.transaction(|tx| {
            let mut participants = Vec::with_capacity(batch.len());
            for (id, edits) in batch {
                participants.push(id);
                for edit in edits {
                    if tx.model.node(id).is_none() {
                        return Err(Error::new("worker edit targets a removed model"));
                    }
                    if !matches!(
                        edit.method.as_str(),
                        "append" | "insert" | "clear" | "move" | "remove" | "set" | "setProperty"
                    ) {
                        return Err(Error::new("invalid worker ListModel edit"));
                    }
                    let args = edit
                        .args
                        .into_iter()
                        .map(|v| tx.import_worker(worker, v, 0))
                        .collect::<Result<Vec<_>, _>>()?;
                    tx.model_call(id, &edit.method, &args, &mut BTreeSet::new())?;
                }
            }
            tx.settle()?;
            participants
                .into_iter()
                .map(|id| tx.model.worker_model_revision(id))
                .collect()
        })
    }
    pub(crate) fn commit_workers(&mut self, actions: Vec<Action>) {
        for action in actions {
            let id = match &action {
                Action::Load(id, _) | Action::Send(id, ..) => *id,
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
                entry.models.clear();
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
                Action::Send(_, v, grants) => entry
                    .endpoint
                    .as_ref()
                    .ok_or_else(|| Error::new("WorkerScript has no source"))
                    .and_then(|actor| actor.send(v))
                    .map(|()| entry.models.extend(grants)),
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
        self.worker_sources.extend(
            self.model
                .workers
                .entries
                .keys()
                .copied()
                .filter(|id| id.index >= first),
        );
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
        // An imperative method can select a replacement source and send to it
        // in one transaction. Preserve that order in the post-commit actions.
        if self.worker_sources.contains(&id) {
            self.stage_workers()?;
        }
        let entry = &self.model.workers.entries[&id];
        let restarting = self
            .worker_actions
            .iter()
            .any(|a| matches!(a, Action::Load(owner, _) if *owner == id));
        if !restarting
            && self
                .model
                .workers
                .inbox
                .as_ref()
                .expect("registered worker")
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .stopped
                .contains(&(id, entry.generation))
        {
            return Err(Error::new(
                "WorkerScript stopped after reaching its output limit",
            ));
        }
        let mut grants = BTreeMap::new();
        let value = self.export_worker(input, 0, &mut grants)?;
        self.worker_actions.push(Action::Send(id, value, grants));
        Ok(Value::Undefined)
    }
    fn export_worker(
        &mut self,
        input: &Value,
        depth: usize,
        grants: &mut BTreeMap<u64, ObjectId>,
    ) -> Result<WorkerValue, Error> {
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
                    .map(|v| self.export_worker(v, depth + 1, grants))
                    .collect::<Result<_, _>>()?,
            ),
            Value::Object(id) if self.model.models.contains_key(id) => {
                let (token, revision, rows) = self.model.worker_model_snapshot(*id)?;
                grants.insert(token, *id);
                let rows = rows
                    .into_iter()
                    .map(|row| {
                        row.into_iter()
                            .map(|(k, v)| Ok((k, self.export_worker(&v, depth + 1, grants)?)))
                            .collect::<Result<_, Error>>()
                    })
                    .collect::<Result<_, _>>()?;
                WorkerValue::Model {
                    id: token,
                    revision,
                    rows,
                }
            }
            Value::Record(r) => {
                let fields = self.model.record_entries(r)?.clone();
                if r.is_array() {
                    WorkerValue::Array(
                        (0..self.model.array_length(r)?)
                            .map(|i| {
                                self.export_worker(
                                    fields.get(&i.to_string()).unwrap_or(&Value::Undefined),
                                    depth + 1,
                                    grants,
                                )
                            })
                            .collect::<Result<_, _>>()?,
                    )
                } else {
                    WorkerValue::Object(
                        fields
                            .iter()
                            .map(|(k, v)| {
                                Ok((k.clone(), self.export_worker(v, depth + 1, grants)?))
                            })
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
    fn import_worker(
        &mut self,
        worker: ObjectId,
        input: WorkerValue,
        depth: usize,
    ) -> Result<Value, Error> {
        self.budget()?;
        if depth >= 64 {
            return Err(Error::new("deeply nested worker message"));
        }
        Ok(match input {
            WorkerValue::Model { id, .. } => {
                let object = *self.model.workers.entries[&worker]
                    .models
                    .get(&id)
                    .ok_or_else(|| Error::new("unknown worker ListModel"))?;
                if !self.model.models.contains_key(&object) {
                    return Err(Error::new("retired worker ListModel"));
                }
                Value::Object(object)
            }
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
                    .map(|v| self.import_worker(worker, v, depth + 1))
                    .collect::<Result<_, _>>()?;
                self.new_array(items)
            }
            WorkerValue::Object(values) => {
                let fields = values
                    .into_iter()
                    .map(|(k, v)| Ok((k, self.import_worker(worker, v, depth + 1)?)))
                    .collect::<Result<_, Error>>()?;
                self.new_record(fields)
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overflow_follows_accepted_deliveries_without_waiting_for_other_workers_to_go_idle() {
        let doc = ir::parse(
            r#"
            import QtQml
            import QtQml.WorkerScript
            QtObject {
                property WorkerScript first: WorkerScript { id: firstActor }
                property WorkerScript next: WorkerScript { id: secondActor }
            }
        "#,
        )
        .unwrap();
        let mut app = Instance::new(&doc, &TypeRegistry::new(), Default::default()).unwrap();
        let a = app.id("firstActor").unwrap();
        let b = app.id("secondActor").unwrap();
        let delivery = |id| WorkerDelivery {
            inbox: Arc::downgrade(app.workers.inbox.as_ref().unwrap()),
            id,
            generation: app.workers.entries[&id].generation,
        };
        let first = delivery(a);
        let second = delivery(b);
        assert!(first.publish(WorkerEvent::Ready));
        assert!(second.publish(WorkerEvent::Ready));
        app.poll_worker_scripts(2).unwrap();
        let message = || WorkerEvent::Message(WorkerValue::Undefined);
        for _ in 0..EVENT_LIMIT {
            assert!(first.publish(message()));
        }
        assert!(!first.publish(message()));
        for _ in 0..EVENT_LIMIT {
            assert_eq!(app.poll_worker_scripts(1).unwrap(), 1);
            // This later delivery keeps the inbox nonempty across every poll.
            assert!(second.publish(message()));
        }
        assert_eq!(app.get(a, "ready").unwrap(), Value::Bool(true));
        assert!(app
            .poll_worker_scripts(1)
            .unwrap_err()
            .message
            .contains("output limit"));
        assert_eq!(app.get(a, "ready").unwrap(), Value::Bool(false));
        assert_eq!(app.get(b, "ready").unwrap(), Value::Bool(true));
        assert!(!first.publish(message()));
        assert!(app.call(a, "sendMessage", vec![Value::Number(1.)]).is_err());
        assert_eq!(app.poll_worker_scripts(EVENT_LIMIT).unwrap(), EVENT_LIMIT);
    }

    #[test]
    fn dropping_a_sync_waiter_revokes_unstarted_edits() {
        let doc = ir::parse(
            r#"
            import QtQml
            import QtQml.WorkerScript
            QtObject {
                property WorkerScript task: WorkerScript { id: actor }
                property ListModel rows: ListModel { id: numbers }
            }
        "#,
        )
        .unwrap();
        let mut app = Instance::new(&doc, &TypeRegistry::new(), Default::default()).unwrap();
        let worker = app.id("actor").unwrap();
        let model = app.id("numbers").unwrap();
        let (token, revision, _) = app.worker_model_snapshot(model).unwrap();
        let entry = app.workers.entries.get_mut(&worker).unwrap();
        entry.models.insert(token, model);
        let delivery = WorkerDelivery {
            inbox: Arc::downgrade(app.workers.inbox.as_ref().unwrap()),
            id: worker,
            generation: entry.generation,
        };
        let edits = || {
            vec![WorkerEdit {
                method: "append".into(),
                args: vec![WorkerValue::Object(BTreeMap::from([(
                    "number".into(),
                    WorkerValue::Number("7".into()),
                )]))],
            }]
        };
        let stale = delivery.synchronize(token, revision, edits()).unwrap();
        drop(stale);
        app.poll_worker_scripts(10).unwrap();
        assert_eq!(app.get(model, "count").unwrap(), Value::Number(0.));
        let live = delivery.synchronize(token, revision, edits()).unwrap();
        app.poll_worker_scripts(10).unwrap();
        assert!(
            live.recv_timeout(std::time::Duration::from_secs(1))
                .unwrap()
                .unwrap()
                > revision
        );
        assert_eq!(app.get(model, "count").unwrap(), Value::Number(1.));
    }
}
