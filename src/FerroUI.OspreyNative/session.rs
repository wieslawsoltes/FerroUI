//! A running Osprey application: the opaque session, its regions by key, its theme, and the
//! capabilities it asks for (docs/osprey-native-host.md, sections 3 and 6). Nothing here decides
//! what a control means: the host sends what a control delivered under the ordinal the framework
//! issued, and draws the regions the framework returns.

use std::collections::BTreeMap;
use std::rc::Rc;

use serde_json::Value;

use crate::boundary::{self, Boundary, Streamed};
use crate::protocol::{self, events, Envelope, Request, Theme};

/// One region: its node tree and the revision it was drawn at.
#[derive(Debug, Clone)]
pub struct Region {
    pub revision: u64,
    pub node: Rc<Value>,
    /// The shapes each streamed scene of this revision received, in the encoder's walk order.
    pub streams: Vec<Rc<Vec<Streamed>>>,
}

/// What the host is asked to do outside the application, answered by the window when it can.
#[derive(Debug, Clone, PartialEq)]
pub enum Clock {
    /// Deliver one `frame` event per display refresh.
    On,
    /// Stop delivering frames.
    Off,
}

/// The capabilities this host answers; every other kind completes with an error naming it.
pub const ANSWERED: &[&str] = &[
    "animate",
    "store-read",
    "store-write",
    "store-list",
    "store-delete",
    "cancel",
];

/// A running application and what it last returned.
pub struct Session {
    boundary: Box<dyn Boundary>,
    regions: BTreeMap<String, Region>,
    state: String,
    theme: Option<Rc<Theme>>,
    diagnostics: Vec<String>,
    clock: Option<Clock>,
    animating: bool,
    store: crate::store::Store,
    /// How deep the host is in answering requests: 0 for an envelope of a start or an event.
    nesting: usize,
    /// The regions patched and the capabilities asked for by the last envelope at nesting 0.
    last_patched: Vec<String>,
    last_requests: Vec<String>,
}

impl Session {
    /// Starts the application with the launch context and answers what the start envelope asks.
    pub fn start(
        mut boundary: Box<dyn Boundary>,
        context: &str,
        store: crate::store::Store,
    ) -> Result<Self, String> {
        let started = protocol::parse(&boundary.start(context)?)?;
        let mut session = Self {
            boundary,
            regions: BTreeMap::new(),
            state: String::new(),
            theme: None,
            diagnostics: Vec::new(),
            clock: None,
            animating: false,
            store,
            nesting: 0,
            last_patched: Vec::new(),
            last_requests: Vec::new(),
        };
        let requests = session.apply(started)?;
        session.answer(requests);
        Ok(session)
    }

    /// Sends one event and applies the envelope it returns, answering its requests. A refused
    /// envelope leaves every region as it was and is reported as a diagnostic.
    pub fn send(&mut self, event: &str) {
        let next = self
            .boundary
            .dispatch(&self.state, event)
            .and_then(|text| protocol::parse(&text));
        match next.and_then(|envelope| self.apply(envelope)) {
            Ok(requests) => self.answer(requests),
            Err(problem) => self.diagnostics.push(problem),
        }
    }

    /// A control's value under its ordinal, in the revision of the region it was drawn in.
    pub fn control(&mut self, region: &str, revision: u64, ordinal: u64, value: Value) {
        self.send(&events::control(
            events::now(),
            region,
            revision,
            ordinal,
            value,
        ));
    }

    /// Applies a checked envelope: it is used whole, after its streamed shapes were matched.
    fn apply(&mut self, envelope: Envelope) -> Result<Vec<Request>, String> {
        if self.nesting == 0 {
            self.last_patched = envelope
                .patches
                .iter()
                .map(|patch| patch.key.clone())
                .collect();
            self.last_requests = envelope
                .requests
                .iter()
                .map(|request| request.kind.clone())
                .collect();
        }
        let mut stream = boundary::take_stream().into_iter();
        let mut patched = Vec::with_capacity(envelope.patches.len());
        for patch in envelope.patches {
            let mut node = patch.node;
            let mut streams = Vec::new();
            attach(&mut node, &mut stream, &mut streams)
                .map_err(|problem| format!("region {}: {problem}", patch.key))?;
            patched.push((
                patch.key,
                Region {
                    revision: patch.revision,
                    node: Rc::new(node),
                    streams,
                },
            ));
        }
        if stream.next().is_some() {
            return Err("the application streamed more shapes than its scenes counted".to_owned());
        }
        for (key, text) in &envelope.slices {
            boundary::put_slice(key, text)?;
        }
        self.state = envelope.state;
        for key in envelope.removed {
            self.regions.remove(&key);
        }
        self.regions.extend(patched);
        if let Some(theme) = envelope.theme {
            self.theme = Some(Rc::new(theme));
        }
        self.diagnostics.extend(envelope.diagnostics);
        Ok(envelope.requests)
    }

    /// Answers each request in order; a completion's own requests are answered after it.
    fn answer(&mut self, requests: Vec<Request>) {
        self.nesting += 1;
        for request in requests {
            let completion = match request.kind.as_str() {
                "animate" => {
                    let on = request
                        .fields
                        .get("on")
                        .and_then(Value::as_bool)
                        .unwrap_or(false);
                    self.animating = on;
                    self.clock = Some(if on { Clock::On } else { Clock::Off });
                    events::completed_ok(events::now(), &request.ticket, Value::Null)
                }
                "store-read" | "store-write" | "store-list" | "store-delete" => {
                    match self.store.answer(&request) {
                        Ok(result) => events::completed_ok(events::now(), &request.ticket, result),
                        Err(problem) => {
                            events::completed_err(events::now(), &request.ticket, &problem)
                        }
                    }
                }
                // Every capability this host answers completes synchronously, so nothing is
                // left in flight to cancel.
                "cancel" => events::completed_ok(events::now(), &request.ticket, Value::Null),
                kind => {
                    let problem = format!("unsupported capability {kind}");
                    self.diagnostics
                        .push(format!("{problem} ({})", request.ticket));
                    events::completed_err(events::now(), &request.ticket, &problem)
                }
            };
            self.send(&completion);
        }
        self.nesting -= 1;
    }

    /// The regions the last envelope of a start or an event patched (not of its completions).
    pub fn last_patched(&self) -> &[String] {
        &self.last_patched
    }

    /// The capability kinds the last envelope of a start or an event asked for.
    pub fn take_requests(&mut self) -> Vec<String> {
        std::mem::take(&mut self.last_requests)
    }

    /// The region drawn under `key`.
    pub fn region(&self, key: &str) -> Option<&Region> {
        self.regions.get(key)
    }

    /// Every region key, in order.
    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.regions.keys().map(String::as_str)
    }

    /// The application's theme.
    pub fn theme(&self) -> Option<&Rc<Theme>> {
        self.theme.as_ref()
    }

    /// Everything reported since the session started: by the framework and by this host.
    pub fn diagnostics(&self) -> &[String] {
        &self.diagnostics
    }

    /// Adds a diagnostic of the host (a mapping that is refused, a malformed value).
    pub fn diagnose(&mut self, message: String) {
        if !self.diagnostics.contains(&message) {
            self.diagnostics.push(message);
        }
    }

    /// Whether the application has the frame clock on.
    pub fn animating(&self) -> bool {
        self.animating
    }

    /// The change of the frame clock the last answers asked for, once.
    pub fn take_clock(&mut self) -> Option<Clock> {
        self.clock.take()
    }
}

/// Gives each streamed scene in `node` its run of the call's stream, walking the members in the
/// order the framework's encoder walks them, which is the order it streamed them in.
fn attach(
    node: &mut Value,
    stream: &mut impl Iterator<Item = Streamed>,
    streams: &mut Vec<Rc<Vec<Streamed>>>,
) -> Result<(), String> {
    let kind = node
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned();
    if kind == "scene" {
        let count = node.get("streamed").and_then(Value::as_u64).unwrap_or(0);
        if count == 0 {
            return Ok(());
        }
        let run: Vec<Streamed> = stream.take(count as usize).collect();
        if run.len() as u64 != count {
            return Err(format!(
                "a scene counted {count} streamed shapes but the application sent {}",
                run.len()
            ));
        }
        if let Some(fields) = node.as_object_mut() {
            fields.insert("streamSlot".to_owned(), Value::from(streams.len()));
        }
        streams.push(Rc::new(run));
        return Ok(());
    }
    let order: &[&str] = match kind.as_str() {
        "screen" => &["toolbar", "children"],
        "aside" => &["pane", "aside"],
        "context-menu" => &["body", "items"],
        "framed" | "themed" | "dialog" => &["body"],
        "tabs" => &["tabs"],
        "rows" => &["rows"],
        _ => &["children"],
    };
    for member in order {
        match node.get_mut(*member) {
            Some(Value::Array(items)) => {
                for item in items {
                    let inner = if item.get("kind").is_some() {
                        item
                    } else if let Some(child) = item.get_mut("child") {
                        child
                    } else {
                        continue;
                    };
                    attach(inner, stream, streams)?;
                }
            }
            Some(single @ Value::Object(_)) => attach(single, stream, streams)?,
            _ => {}
        }
    }
    Ok(())
}
