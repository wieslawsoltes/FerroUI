//! A compiled application's dedicated worker owns this instance and all host
//! references. Only copied values and explicitly watched properties leave it.
use super::*;
use ferroui_qml::workers::{CopyDependencies, WorkerValue};
use serde::Serialize;
use serde_json::value::{to_raw_value, RawValue};
use std::borrow::Cow;
use std::{collections::BTreeSet, sync::Arc};
use wasm_bindgen::prelude::*;

#[wasm_bindgen(module = "/browser_application.js")]
extern "C" {
    #[wasm_bindgen(js_name = applicationModule)]
    fn module_url() -> String;
    #[wasm_bindgen(js_name = wakeApplication)]
    fn wake();
}

/// URL of the packaged ES module containing openQmlApplication and serveQmlApplication.
#[wasm_bindgen(js_name = qmlApplicationModule)]
pub fn browser_application_module() -> String {
    module_url()
}

#[wasm_bindgen]
pub struct BrowserApplication {
    model: ferroui_qml::Instance,
    watches: BTreeMap<(ObjectId, String), Watch>,
    last: Instant,
    timeout: Duration,
}

struct Watch {
    tokens: BTreeSet<u32>,
    snapshot: Option<(Box<RawValue>, CopyDependencies)>,
}

#[derive(Serialize)]
struct Packet<'a> {
    tokens: &'a BTreeSet<u32>,
    update: Update<'a>,
}

#[derive(Serialize)]
#[serde(rename_all = "lowercase")]
enum Update<'a> {
    Value(Cow<'a, RawValue>),
    Error(String),
}

impl BrowserApplication {
    /// Wrap an initialized instance inside its dedicated application worker.
    pub fn new(mut model: ferroui_qml::Instance, timeout: Duration) -> Result<Self, Error> {
        process::available()?;
        let wake: Arc<dyn Fn() + Send + Sync> = Arc::new(wake);
        model.set_worker_waker(Some(wake.clone()));
        model.set_component_waker(Some(wake));
        Ok(Self {
            model,
            watches: Default::default(),
            last: Instant::now(),
            timeout,
        })
    }
    fn object(&self, id: &str) -> Result<ObjectId, Error> {
        if id.is_empty() {
            return Ok(self.model.root());
        }
        self.model
            .id(id)
            .ok_or_else(|| Error::new(format!("unknown QML id '{id}'")))
    }
    fn request(
        &mut self,
        op: &str,
        id: &str,
        member: &str,
        wire: &str,
    ) -> Result<WorkerValue, Error> {
        let owner = self.object(id)?;
        let value = match op {
            "get" => self.model.get(owner, member)?,
            "set" => {
                let data = serde_json::from_str(wire).map_err(|e| Error::new(e.to_string()))?;
                self.model.set_data(owner, member, data)?;
                return Ok(WorkerValue::Undefined);
            }
            "call" => {
                let items: Vec<WorkerValue> =
                    serde_json::from_str(wire).map_err(|e| Error::new(e.to_string()))?;
                return self.model.call_data(owner, member, items, self.timeout);
            }
            _ => return Err(Error::new("unknown QML request")),
        };
        self.model.copy_data(owner, value)
    }
}

fn watched_data(
    model: &mut ferroui_qml::Instance,
    owner: ObjectId,
    member: &str,
) -> Result<(WorkerValue, CopyDependencies), Error> {
    let value = model.get(owner, member)?;
    model.copy_observed_data(owner, value)
}

fn error(e: Error) -> JsValue {
    JsValue::from_str(&e.to_string())
}

#[wasm_bindgen]
impl BrowserApplication {
    pub fn dispatch(
        &mut self,
        op: &str,
        id: &str,
        member: &str,
        data: &str,
    ) -> Result<String, JsValue> {
        let value = self.request(op, id, member, data).map_err(error)?;
        serde_json::to_string(&value).map_err(|e| JsValue::from_str(&e.to_string()))
    }
    pub fn watch(&mut self, token: u32, id: &str, member: &str) -> Result<(), JsValue> {
        let owner = self.object(id).map_err(error)?;
        let key = (owner, member.to_owned());
        if let Some(watch) = self.watches.get_mut(&key) {
            watch.tokens.insert(token);
            return Ok(());
        }
        let (data, deps) = watched_data(&mut self.model, owner, member).map_err(error)?;
        let snapshot =
            (!deps.is_empty()).then(|| (to_raw_value(&data).expect("data packet"), deps));
        self.watches.insert(
            key,
            Watch {
                tokens: [token].into(),
                snapshot,
            },
        );
        Ok(())
    }
    pub fn unwatch(&mut self, token: u32) {
        self.watches.retain(|_, watch| {
            watch.tokens.remove(&token);
            !watch.tokens.is_empty()
        });
    }
    pub fn changes(&mut self) -> String {
        let changes = self
            .model
            .drain_changes()
            .into_iter()
            .map(|c| (c.object, c.property))
            .collect::<BTreeSet<_>>();

        let mut output = Vec::new();
        for (key, watch) in &mut self.watches {
            let (owner, member) = key;
            let changed = changes.contains(key);
            if !changed
                && watch
                    .snapshot
                    .as_mut()
                    .is_none_or(|(_, deps)| deps.is_current(&self.model))
            {
                continue;
            }
            let update = match watched_data(&mut self.model, *owner, member) {
                Ok((data, deps)) => {
                    // Keep one compact representation for equality and the wire
                    // packet. RawValue writes these exact bytes into the envelope.
                    let data = to_raw_value(&data).expect("data packet");
                    if !changed
                        && watch
                            .snapshot
                            .as_ref()
                            .is_some_and(|(old, _)| old.get() == data.get())
                    {
                        // Equal rows can now refer to a different child model.
                        // Retain the newly traversed dependency set in that case.
                        watch.snapshot = Some((data, deps));
                        continue;
                    }
                    if deps.is_empty() {
                        watch.snapshot = None;
                        Update::Value(Cow::Owned(data))
                    } else {
                        let (value, _) = watch.snapshot.insert((data, deps));
                        Update::Value(Cow::Borrowed(&**value))
                    }
                }
                Err(e) => {
                    watch.snapshot = None;
                    Update::Error(e.message)
                }
            };
            output.push(Packet {
                tokens: &watch.tokens,
                update,
            });
        }
        serde_json::to_string(&output).expect("data packet")
    }
    pub fn pump(&mut self) -> Result<(), JsValue> {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last);
        self.last = now;
        self.model.advance_time(elapsed).map_err(error)?;
        self.model.poll_worker_scripts(16).map_err(error)?;
        self.model.poll_component_loads(8).map_err(error)?;
        self.model.process_deferred(16).map_err(error)?;
        Ok(())
    }
    #[wasm_bindgen(js_name = nextWake)]
    pub fn next_wake(&self) -> f64 {
        if self.model.has_deferred_work() {
            0.
        } else {
            self.model
                .next_timer_deadline()
                .map_or(-1., |d| d.as_secs_f64() * 1000.)
        }
    }
}
