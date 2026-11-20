//! The envelope Osprey Native encodes (protocol version 1) and the events a host sends back
//! (docs/osprey-native-host.md, section 3). An envelope is checked whole before anything of it is
//! used: its members, every node of every patch against the schema of its kind, and every scene
//! primitive. One malformed part refuses the whole envelope, with a diagnostic that names it; the
//! host never draws a part of an envelope or a fallback.

use serde_json::{Map, Value};

use crate::schema;

/// The only protocol version this host speaks.
pub const VERSION: u64 = 1;

/// One decoded envelope.
#[derive(Debug, Clone)]
pub struct Envelope {
    /// The opaque session, sent back unchanged with the next event.
    pub state: String,
    /// Texts the host holds for the application under their keys.
    pub slices: Vec<(String, String)>,
    /// Regions whose node trees changed.
    pub patches: Vec<Patch>,
    /// Regions that are gone.
    pub removed: Vec<String>,
    /// Capabilities the application asks the host to answer.
    pub requests: Vec<Request>,
    /// What the framework reports.
    pub diagnostics: Vec<String>,
    /// The application's theme (the start envelope).
    pub theme: Option<Theme>,
}

/// A region's new node tree at its revision.
#[derive(Debug, Clone)]
pub struct Patch {
    pub key: String,
    pub revision: u64,
    pub node: Value,
}

/// A capability request: its kind, the ticket its completion answers, and its own arguments.
#[derive(Debug, Clone)]
pub struct Request {
    pub ticket: String,
    pub kind: String,
    pub fields: Map<String, Value>,
}

impl Request {
    /// A string argument of the request.
    pub fn text(&self, name: &str) -> Result<&str, String> {
        self.fields
            .get(name)
            .and_then(Value::as_str)
            .ok_or_else(|| format!("{} has no valid {name}", self.kind))
    }
}

/// One colour role in both appearances, `#RRGGBB`.
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub name: String,
    pub light: String,
    pub dark: String,
}

/// The application's theme: colour roles, a spacing scale in points, motion durations in ms.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Theme {
    pub tokens: Vec<Token>,
    pub spacing: Vec<i64>,
    pub feedback: i64,
    pub settle: i64,
}

/// Decodes and checks one envelope; any malformed part refuses all of it.
pub fn parse(text: &str) -> Result<Envelope, String> {
    let value: Value = serde_json::from_str(text)
        .map_err(|problem| format!("the envelope is not JSON: {problem}"))?;
    let members = value.as_object().ok_or("the envelope is not an object")?;
    let version = members
        .get("version")
        .and_then(Value::as_u64)
        .ok_or("the envelope has no version")?;
    if version != VERSION {
        return Err(format!(
            "protocol version {version} is not the version {VERSION} this host speaks"
        ));
    }
    let state = members
        .get("state")
        .and_then(Value::as_str)
        .ok_or("the envelope has no state")?
        .to_owned();
    let patches = array(members, "patches", true)?
        .iter()
        .map(patch)
        .collect::<Result<Vec<_>, _>>()?;
    let requests = array(members, "requests", true)?
        .iter()
        .map(request)
        .collect::<Result<Vec<_>, _>>()?;
    let slices = array(members, "slices", false)?
        .iter()
        .map(slice)
        .collect::<Result<Vec<_>, _>>()?;
    let removed = strings(array(members, "removed", false)?, "removed")?;
    let diagnostics = strings(array(members, "diagnostics", false)?, "diagnostics")?;
    let theme = match members.get("theme") {
        None | Some(Value::Null) => None,
        Some(found) => {
            Some(theme(found).map_err(|problem| format!("the envelope's theme: {problem}"))?)
        }
    };
    Ok(Envelope {
        state,
        slices,
        patches,
        removed,
        requests,
        diagnostics,
        theme,
    })
}

/// A member array, empty when absent and not required.
fn array<'a>(
    members: &'a Map<String, Value>,
    name: &str,
    required: bool,
) -> Result<&'a [Value], String> {
    match members.get(name) {
        Some(Value::Array(items)) => Ok(items),
        None if !required => Ok(&[]),
        None => Err(format!("the envelope has no {name}")),
        Some(_) => Err(format!("the envelope's {name} is not an array")),
    }
}

fn strings(items: &[Value], name: &str) -> Result<Vec<String>, String> {
    items
        .iter()
        .map(|item| {
            item.as_str()
                .map(str::to_owned)
                .ok_or_else(|| format!("an entry of {name} is not text"))
        })
        .collect()
}

fn patch(value: &Value) -> Result<Patch, String> {
    let key = value
        .get("key")
        .and_then(Value::as_str)
        .ok_or("a patch has no key")?
        .to_owned();
    let revision = value
        .get("revision")
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("patch {key} has no revision"))?;
    let node = value
        .get("node")
        .cloned()
        .ok_or_else(|| format!("patch {key} has no node"))?;
    schema::check(&node).map_err(|problem| format!("region {key}: {problem}"))?;
    Ok(Patch {
        key,
        revision,
        node,
    })
}

fn request(value: &Value) -> Result<Request, String> {
    let mut fields = value
        .as_object()
        .cloned()
        .ok_or("a request is not an object")?;
    let ticket = match fields.remove("ticket") {
        Some(Value::String(ticket)) => ticket,
        _ => return Err("a request has no ticket".to_owned()),
    };
    let kind = match fields.remove("kind") {
        Some(Value::String(kind)) => kind,
        _ => return Err(format!("request {ticket} has no kind")),
    };
    Ok(Request {
        ticket,
        kind,
        fields,
    })
}

fn slice(value: &Value) -> Result<(String, String), String> {
    let key = value
        .get("key")
        .and_then(Value::as_str)
        .ok_or("a slice has no key")?;
    let text = value
        .get("text")
        .and_then(Value::as_str)
        .ok_or_else(|| format!("slice {key} has no text"))?;
    Ok((key.to_owned(), text.to_owned()))
}

/// Decodes a theme: tokens of `#RRGGBB` colours, a spacing scale, two durations.
pub fn theme(value: &Value) -> Result<Theme, String> {
    let tokens = value
        .get("tokens")
        .and_then(Value::as_array)
        .ok_or("no tokens")?;
    let tokens = tokens
        .iter()
        .map(|token| {
            let text = |name: &str| {
                token
                    .get(name)
                    .and_then(Value::as_str)
                    .map(str::to_owned)
                    .ok_or_else(|| format!("a token has no {name}"))
            };
            let token = Token {
                name: text("name")?,
                light: text("light")?,
                dark: text("dark")?,
            };
            for colour in [&token.light, &token.dark] {
                crate::theme::hex(colour)
                    .ok_or_else(|| format!("token {} has the colour {colour}", token.name))?;
            }
            Ok(token)
        })
        .collect::<Result<Vec<_>, String>>()?;
    let spacing = value
        .get("spacing")
        .and_then(Value::as_array)
        .ok_or("no spacing")?
        .iter()
        .map(|step| {
            step.as_i64()
                .ok_or_else(|| "a spacing step is not an integer".to_owned())
        })
        .collect::<Result<Vec<_>, _>>()?;
    let number = |name: &str| {
        value
            .get(name)
            .and_then(Value::as_i64)
            .ok_or_else(|| format!("no {name}"))
    };
    Ok(Theme {
        tokens,
        spacing,
        feedback: number("feedback")?,
        settle: number("settle")?,
    })
}

/// The events a host sends: the JSON `ui::Events` decodes.
pub mod events {
    use serde_json::{json, Value};

    /// A control's value under the ordinal the framework issued, in its region's revision.
    pub fn control(now: i64, region: &str, revision: u64, ordinal: u64, value: Value) -> String {
        json!({"kind": "ui", "now": now, "region": region, "revision": revision, "ordinal": ordinal, "value": value})
            .to_string()
    }

    /// A capability that succeeded.
    pub fn completed_ok(now: i64, ticket: &str, result: Value) -> String {
        json!({"kind": "completed", "now": now, "ticket": ticket, "ok": true, "result": result})
            .to_string()
    }

    /// A capability that failed.
    pub fn completed_err(now: i64, ticket: &str, error: &str) -> String {
        json!({"kind": "completed", "now": now, "ticket": ticket, "ok": false, "error": error})
            .to_string()
    }

    /// One display frame and the seconds since the one before, always written as a float.
    pub fn frame(now: i64, elapsed: f64) -> String {
        format!("{{\"kind\":\"frame\",\"now\":{now},\"elapsed\":{elapsed:?}}}")
    }

    /// A lifecycle phase (`active`, `background`, `open:<path>`).
    pub fn lifecycle(now: i64, phase: &str) -> String {
        json!({"kind": "lifecycle", "now": now, "phase": phase}).to_string()
    }

    /// The launch context: Unix seconds, the local UTC offset, the platform, the home folder and
    /// the arguments the application was opened with.
    pub fn start_context(
        now: i64,
        offset: i64,
        platform: &str,
        home: &str,
        arguments: &[String],
    ) -> String {
        json!({"now": now, "offset": offset, "platform": platform, "home": home, "arguments": arguments}).to_string()
    }

    /// Unix seconds now.
    pub fn now() -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |since| {
                i64::try_from(since.as_secs()).unwrap_or(i64::MAX)
            })
    }

    /// The value a tap sends.
    pub const TAP: Value = Value::Null;
}
