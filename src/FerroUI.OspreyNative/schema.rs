//! The node schema of Osprey Native 0.29 (`ui::Encode::walk`): every node kind the framework
//! encodes and the members each carries. A patch is checked against it before the host uses it,
//! node by node, so a kind the host does not know, a missing member or a member of the wrong type
//! refuses the envelope instead of drawing a guess.

use serde_json::Value;

/// The type of one member of a node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Member {
    /// A string.
    Text,
    /// A boolean.
    Flag,
    /// An integer.
    Whole,
    /// Any number.
    Real,
    /// One node.
    Node,
    /// An array of nodes.
    Nodes,
    /// An array of strings.
    Texts,
    /// An array of `{key, label}` choices.
    Choices,
    /// An array of `{text, ink, italic, emphasis}` runs.
    Runs,
    /// An array of tab entries: `{key, label, symbol, child}`.
    Tabs,
    /// An array of row entries: `{key, symbol, group, child}`.
    Rows,
    /// A theme object.
    Theme,
    /// A frame style object.
    Style,
    /// An array of scene primitives (checked by `scene::decode`).
    Shapes,
    /// An array of objects, read as they are by the mapping of the kind.
    Objects,
    /// Any JSON value, read as it is by the mapping of the kind.
    Any,
}

use Member::*;

/// One node kind: its wire name and its members, each required unless marked optional.
#[derive(Debug)]
pub struct Kind {
    pub name: &'static str,
    pub members: &'static [(&'static str, Member)],
    pub optional: &'static [(&'static str, Member)],
}

const fn kind(name: &'static str, members: &'static [(&'static str, Member)]) -> Kind {
    Kind {
        name,
        members,
        optional: &[],
    }
}

/// Every node kind Osprey Native 0.29 encodes, in the order of `ui::Encode::walk`.
pub const KINDS: &[Kind] = &[
    kind("rich-text", &[("runs", Runs), ("role", Text)]),
    kind("text", &[("text", Text), ("role", Text), ("tone", Text)]),
    kind("evidence", &[("label", Text), ("value", Text)]),
    kind("pair", &[("label", Text), ("value", Text)]),
    kind(
        "badge",
        &[("label", Text), ("symbol", Text), ("tone", Text)],
    ),
    kind("label", &[("label", Text), ("symbol", Text)]),
    kind("icon", &[("symbol", Text)]),
    kind("link", &[("label", Text), ("url", Text)]),
    kind("spacer", &[]),
    kind("divider", &[]),
    kind("segments", &[("tones", Texts), ("label", Text)]),
    kind(
        "empty",
        &[("symbol", Text), ("title", Text), ("detail", Text)],
    ),
    kind("progress", &[("label", Text), ("detail", Text)]),
    kind(
        "meter",
        &[
            ("label", Text),
            ("value", Whole),
            ("total", Whole),
            ("tone", Text),
        ],
    ),
    kind("swatch", &[("name", Text), ("light", Text), ("dark", Text)]),
    kind("slot", &[("key", Text)]),
    kind("brand", &[]),
    kind(
        "button",
        &[
            ("label", Text),
            ("symbol", Text),
            ("role", Text),
            ("enabled", Flag),
            ("event", Whole),
        ],
    ),
    kind(
        "field",
        &[
            ("label", Text),
            ("value", Text),
            ("prompt", Text),
            ("secure", Flag),
            ("multiline", Flag),
            ("commit", Text),
            ("event", Whole),
        ],
    ),
    kind("toggle", &[("label", Text), ("on", Flag), ("event", Whole)]),
    kind(
        "picker",
        &[
            ("label", Text),
            ("choices", Choices),
            ("selected", Text),
            ("event", Whole),
        ],
    ),
    kind(
        "disclosure",
        &[
            ("title", Text),
            ("expanded", Flag),
            ("event", Whole),
            ("children", Nodes),
        ],
    ),
    kind(
        "alert",
        &[
            ("title", Text),
            ("message", Text),
            ("visible", Flag),
            ("event", Whole),
        ],
    ),
    kind("stack", &[("axis", Text), ("children", Nodes)]),
    kind(
        "section",
        &[("title", Text), ("footer", Text), ("children", Nodes)],
    ),
    kind(
        "screen",
        &[
            ("title", Text),
            ("subtitle", Text),
            ("toolbar", Nodes),
            ("children", Nodes),
        ],
    ),
    kind("split", &[("children", Nodes)]),
    kind("layer", &[("children", Nodes)]),
    kind("framed", &[("style", Style), ("body", Node)]),
    kind("themed", &[("theme", Theme), ("body", Node)]),
    kind(
        "dialog",
        &[("title", Text), ("event", Whole), ("body", Node)],
    ),
    kind("context-menu", &[("body", Node), ("items", Nodes)]),
    kind(
        "aside",
        &[
            ("side", Text),
            ("pane", Node),
            ("aside", Node),
            ("width", Whole),
            ("least", Whole),
            ("most", Whole),
            ("keep", Whole),
            ("event", Whole),
        ],
    ),
    kind(
        "tabs",
        &[("selected", Text), ("tabs", Tabs), ("event", Whole)],
    ),
    kind(
        "rows",
        &[("selected", Text), ("rows", Rows), ("event", Whole)],
    ),
    kind("math", &[("math", Any), ("spoken", Text)]),
    kind(
        "image",
        &[
            ("source", Text),
            ("alt", Text),
            ("width", Whole),
            ("height", Whole),
        ],
    ),
    kind(
        "video",
        &[
            ("source", Text),
            ("poster", Text),
            ("captions", Text),
            ("alt", Text),
            ("seconds", Whole),
        ],
    ),
    kind(
        "editor",
        &[
            ("label", Text),
            ("value", Text),
            ("inline", Flag),
            ("cursor", Whole),
            ("focusEpoch", Whole),
            ("marks", Objects),
            ("event", Whole),
            ("placeEvent", Whole),
        ],
    ),
    Kind {
        name: "board",
        members: &[
            ("slice", Text),
            ("count", Whole),
            ("lasso", Flag),
            ("backdrop", Text),
            ("panning", Flag),
            ("event", Whole),
            ("lassoEvent", Whole),
        ],
        optional: &[("panEvent", Whole)],
    },
    Kind {
        name: "scene",
        members: &[
            ("width", Real),
            ("height", Real),
            ("zoom", Real),
            ("atlas", Texts),
            ("shapes", Shapes),
            ("streamed", Whole),
            ("event", Whole),
        ],
        optional: &[("access", Objects)],
    },
];

/// The schema of `name`, if the framework encodes such a kind.
pub fn find(name: &str) -> Option<&'static Kind> {
    KINDS.iter().find(|kind| kind.name == name)
}

/// Checks a node and everything below it; the error names the first part that is malformed.
pub fn check(node: &Value) -> Result<(), String> {
    check_at(node, 0)
}

/// How deep nodes may nest before a tree is refused rather than walked.
const DEEPEST: usize = 256;

fn check_at(node: &Value, depth: usize) -> Result<(), String> {
    if depth > DEEPEST {
        return Err(format!("nodes nest deeper than {DEEPEST}"));
    }
    let members = node.as_object().ok_or("a node is not an object")?;
    let name = members
        .get("kind")
        .and_then(Value::as_str)
        .ok_or("a node has no kind")?;
    let kind = find(name).ok_or_else(|| format!("unknown node kind {name}"))?;
    for (member, of) in kind.members {
        let value = members
            .get(*member)
            .ok_or_else(|| format!("a {name} node has no {member}"))?;
        check_member(name, member, *of, value, depth)?;
    }
    for (member, of) in kind.optional {
        if let Some(value) = members.get(*member) {
            check_member(name, member, *of, value, depth)?;
        }
    }
    Ok(())
}

fn check_member(
    kind: &str,
    member: &str,
    of: Member,
    value: &Value,
    depth: usize,
) -> Result<(), String> {
    let wrong = || format!("the {member} of a {kind} node is not {}", describe(of));
    let entries = |fields: &[&str], child: bool| -> Result<(), String> {
        for entry in value.as_array().ok_or_else(wrong)? {
            for field in fields {
                entry
                    .get(*field)
                    .and_then(Value::as_str)
                    .ok_or_else(wrong)?;
            }
            if child {
                check_at(entry.get("child").ok_or_else(wrong)?, depth + 1)?;
            }
        }
        Ok(())
    };
    match of {
        Text => value.as_str().map(drop).ok_or_else(wrong),
        Flag => value.as_bool().map(drop).ok_or_else(wrong),
        Whole => value.as_i64().map(drop).ok_or_else(wrong),
        Real => value
            .as_f64()
            .filter(|real| real.is_finite())
            .map(drop)
            .ok_or_else(wrong),
        Node => check_at(value, depth + 1),
        Nodes => value
            .as_array()
            .ok_or_else(wrong)?
            .iter()
            .try_for_each(|child| check_at(child, depth + 1)),
        Texts => entries(&[], false).and_then(|()| {
            value
                .as_array()
                .into_iter()
                .flatten()
                .all(Value::is_string)
                .then_some(())
                .ok_or_else(wrong)
        }),
        Choices => entries(&["key", "label"], false),
        Runs => {
            entries(&["text", "ink", "emphasis"], false)?;
            value
                .as_array()
                .into_iter()
                .flatten()
                .all(|run| run.get("italic").is_some_and(Value::is_boolean))
                .then_some(())
                .ok_or_else(wrong)
        }
        Tabs => entries(&["key", "label", "symbol"], true),
        Rows => entries(&["key", "symbol", "group"], true),
        Theme => crate::protocol::theme(value)
            .map(drop)
            .map_err(|problem| format!("{}: {problem}", wrong())),
        Style => value.as_object().map(drop).ok_or_else(wrong),
        Shapes => {
            crate::scene::check_shapes(value).map_err(|problem| format!("a scene: {problem}"))
        }
        Objects => value
            .as_array()
            .filter(|items| items.iter().all(Value::is_object))
            .map(drop)
            .ok_or_else(wrong),
        Any => Ok(()),
    }
}

fn describe(of: Member) -> &'static str {
    match of {
        Text => "text",
        Flag => "a boolean",
        Whole => "an integer",
        Real => "a finite number",
        Node => "a node",
        Nodes => "an array of nodes",
        Texts => "an array of text",
        Choices => "an array of choices",
        Runs => "an array of text runs",
        Tabs => "an array of tabs",
        Rows => "an array of rows",
        Theme => "a theme",
        Style => "a frame style",
        Shapes => "an array of primitives",
        Objects => "an array of objects",
        Any => "a value",
    }
}
