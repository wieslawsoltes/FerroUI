//! The windowless check of the real boundary (docs/osprey-native-host.md, section 7): start the
//! application, drive it with events, and assert what its envelopes say.
//!
//! - [`probe`]: Osprey's probe application, tapping its Add button by the ordinal the node
//!   carries and expecting "Count 1";
//! - [`replay`]: an application's recorded `smoke.jsonl`, one step per line: `{"send": .., "expect":
//!   ..}`, where `send` is an event, a control found by region and label (a scene when the label is
//!   left out), or null to observe the start envelope, and `expect` names the regions that exist,
//!   the regions patched, a text a region shows, the capabilities asked for and the count of
//!   diagnostics.

use serde_json::Value;

use crate::boundary::Boundary;
use crate::protocol::events;
use crate::session::Session;
use crate::store::Store;

/// Starts the probe, taps Add, and checks that the root region reads "Count 1".
pub fn probe(boundary: Box<dyn Boundary>, context: &str) -> Result<(), String> {
    let mut session = Session::start(boundary, context, Store::none())?;
    for key in ["root", "note"] {
        if session.region(key).is_none() {
            let keys: Vec<&str> = session.keys().collect();
            return Err(format!(
                "the start envelope has no {key} region; it holds {keys:?}"
            ));
        }
    }
    if session.theme().is_none() {
        return Err("the start envelope carried no theme".to_owned());
    }
    let root = session.region("root").ok_or("no root region")?;
    let revision = root.revision;
    let ordinal = node_event(&root.node, Some("button"), Some("Add"))
        .ok_or("the root region has no button Add")?;
    session.control("root", revision, ordinal, Value::Null);
    let root = session
        .region("root")
        .ok_or("the root region is gone after the tap")?;
    if !has_text(&root.node, "Count 1", false) {
        return Err(format!(
            "the root region does not read \"Count 1\" after the tap: {}",
            root.node
        ));
    }
    if !session.diagnostics().is_empty() {
        return Err(format!(
            "the application reported {}",
            session.diagnostics().join("; ")
        ));
    }
    Ok(())
}

/// Replays a recorded script; the answer is the number of steps.
pub fn replay(boundary: Box<dyn Boundary>, context: &str, script: &str) -> Result<usize, String> {
    let steps = parse(script)?;
    let mut session =
        Session::start(boundary, context, Store::none()).map_err(|problem| at(1, problem))?;
    let mut requests = session.take_requests();
    for (index, (line, step)) in steps.iter().enumerate() {
        let send = step
            .get("send")
            .ok_or_else(|| at(*line, "a step has no send".to_owned()))?;
        let expect = step.get("expect").cloned().unwrap_or(Value::Null);
        if !send.is_null() {
            let event = event(&session, send).map_err(|problem| at(*line, problem))?;
            session.send(&event);
            requests = session.take_requests();
        } else if index != 0 {
            return Err(at(
                *line,
                "only the first step may observe the start envelope".to_owned(),
            ));
        }
        check(&session, &requests, &expect).map_err(|problem| at(*line, problem))?;
    }
    Ok(steps.len())
}

/// Nonblank lines that are not `//` comments, each one JSON step, with its line number.
fn parse(script: &str) -> Result<Vec<(usize, Value)>, String> {
    script
        .lines()
        .enumerate()
        .filter(|(_, line)| !line.trim().is_empty() && !line.trim().starts_with("//"))
        .map(|(index, line)| {
            serde_json::from_str(line.trim())
                .map(|step| (index + 1, step))
                .map_err(|problem| at(index + 1, format!("malformed line: {problem}")))
        })
        .collect()
}

fn at(line: usize, problem: String) -> String {
    format!("line {line}: {problem}")
}

/// The event of a step: a control by region and label, or an event as recorded.
fn event(session: &Session, send: &Value) -> Result<String, String> {
    let Some(control) = send.get("control") else {
        let mut event = send
            .as_object()
            .cloned()
            .ok_or("send is neither an event nor a control")?;
        event
            .entry("now")
            .or_insert_with(|| Value::from(events::now()));
        return Ok(Value::Object(event).to_string());
    };
    let key = control
        .get("region")
        .and_then(Value::as_str)
        .ok_or("control.region is not text")?;
    let label = match control.get("label") {
        None => None,
        Some(label) => Some(label.as_str().ok_or("control.label is not text")?),
    };
    let value = send
        .get("value")
        .cloned()
        .ok_or("a control step has no value")?;
    let region = session
        .region(key)
        .ok_or_else(|| format!("region {key} is absent"))?;
    let kind = if label.is_none() { Some("scene") } else { None };
    let ordinal = node_event(&region.node, kind, label)
        .ok_or_else(|| format!("region {key} has no such control"))?;
    Ok(events::control(
        events::now(),
        key,
        region.revision,
        ordinal,
        value,
    ))
}

/// Checks one step's expectations against the session and the requests of its envelope.
fn check(session: &Session, requests: &[String], expect: &Value) -> Result<(), String> {
    let names = |name: &str| -> Vec<String> {
        expect
            .get(name)
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::to_owned)
                    .collect()
            })
            .unwrap_or_default()
    };
    for key in names("regions") {
        if session.region(&key).is_none() {
            return Err(format!(
                "region {key} is absent; regions are {:?}",
                session.keys().collect::<Vec<_>>()
            ));
        }
    }
    for key in names("patched") {
        if !session.last_patched().contains(&key) {
            return Err(format!(
                "region {key} was not patched; patches were {:?}",
                session.last_patched()
            ));
        }
    }
    if let Some(expected) = expect.get("requests") {
        let wanted = names("requests");
        if expected.is_array() && wanted != requests {
            return Err(format!("requests were {requests:?}, expected {wanted:?}"));
        }
    }
    if let Some(text) = expect.get("text") {
        let key = text
            .get("region")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let contains = text
            .get("contains")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let region = session
            .region(key)
            .ok_or_else(|| format!("region {key} is absent"))?;
        if !has_text(&region.node, contains, true) {
            return Err(format!("region {key} has no text containing {contains:?}"));
        }
    }
    if let Some(count) = expect.get("diagnostics").and_then(Value::as_u64) {
        if session.diagnostics().len() as u64 != count {
            return Err(format!(
                "diagnostics were {:?}, expected {count}",
                session.diagnostics()
            ));
        }
    }
    Ok(())
}

/// The ordinal of the first node of `kind` (any kind when `None`) labelled `label` (any label).
pub fn node_event(node: &Value, kind: Option<&str>, label: Option<&str>) -> Option<u64> {
    let kind_matches =
        kind.is_none_or(|kind| node.get("kind").and_then(Value::as_str) == Some(kind));
    let label_matches =
        label.is_none_or(|label| node.get("label").and_then(Value::as_str) == Some(label));
    match kind_matches && label_matches && node.get("event").is_some() {
        true => node.get("event").and_then(Value::as_u64),
        false => inside(node).find_map(|child| node_event(child, kind, label)),
    }
}

/// Whether a tree shows `text`: a text node's text exactly, or any string containing it.
pub fn has_text(node: &Value, text: &str, substring: bool) -> bool {
    let shown = match substring {
        true => node.as_str(),
        false if node.get("kind").and_then(Value::as_str) == Some("text") => {
            node.get("text").and_then(Value::as_str)
        }
        false => None,
    };
    shown.is_some_and(|shown| shown == text || (substring && shown.contains(text)))
        || inside(node).any(|child| has_text(child, text, substring))
}

/// Everything a node holds, wherever its kind keeps it.
fn inside(node: &Value) -> impl Iterator<Item = &Value> {
    let items: Vec<&Value> = match node {
        Value::Object(members) => members.values().collect(),
        Value::Array(items) => items.iter().collect(),
        _ => Vec::new(),
    };
    items.into_iter()
}
