//! The node kinds of Osprey Native as FerroUI elements (docs/osprey-native-host.md, section 4):
//! one mapping per kind. A kind this host does not draw yet is drawn as a visible box that names
//! it and is reported; it is never drawn as something else. `DRAWN` and `REFUSED` together are
//! every kind of the schema, each once, which the tests check.

use std::cell::RefCell;
use std::rc::Rc;

use ferroui_base::input::Key as KeyName;
use ferroui_base::layout::{HorizontalAlignment, Orientation, VerticalAlignment};
use ferroui_base::media::{Color, FontWeight};
use ferroui_base::Thickness;
use ferroui_controls::TextBox;
use ferroui_view::element::with_key;
use ferroui_view::prelude::*;
use serde_json::Value;

use crate::host::Host;
use crate::session::Session;
use crate::theme::{self, Look};

/// The node kinds this host draws, each with its mapping in `node`.
pub const DRAWN: &[&str] = &["stack", "text", "button", "field", "slot", "spacer"];

/// The node kinds this host draws as a box that names them, until it maps them.
pub const REFUSED: &[&str] = &[
    "rich-text",
    "evidence",
    "pair",
    "badge",
    "label",
    "icon",
    "link",
    "divider",
    "segments",
    "empty",
    "progress",
    "meter",
    "swatch",
    "brand",
    "toggle",
    "picker",
    "disclosure",
    "alert",
    "section",
    "screen",
    "split",
    "layer",
    "framed",
    "themed",
    "dialog",
    "context-menu",
    "aside",
    "tabs",
    "rows",
    "math",
    "image",
    "video",
    "editor",
    "board",
    "scene",
];

/// What a render reads: the host, the session, the region being drawn and the look in force.
pub struct Cx<'s> {
    host: &'s Rc<Host>,
    session: &'s Session,
    /// The region being drawn and its revision: the binding of each control drawn in it.
    key: String,
    revision: u64,
    look: Look,
    /// The regions being embedded, to refuse a slot cycle.
    stack: Vec<String>,
}

/// The element of the root region.
pub fn root(host: &Rc<Host>, session: &Session, look: Look) -> Element {
    let mut cx = Cx {
        host,
        session,
        key: String::new(),
        revision: 0,
        look,
        stack: Vec::new(),
    };
    embed(&mut cx, "root")
}

/// The element of a region, in place of the slot that names it.
fn embed(cx: &mut Cx, key: &str) -> Element {
    if cx.stack.iter().any(|item| item == key) {
        return refused(cx, format!("slot cycle at region {key}"));
    }
    let Some(region) = cx.session.region(key) else {
        return refused(cx, format!("missing region {key}"));
    };
    let node = region.node.clone();
    let revision = std::mem::replace(&mut cx.revision, region.revision);
    let outer = std::mem::replace(&mut cx.key, key.to_owned());
    cx.stack.push(key.to_owned());
    let element = self::node(cx, &node);
    cx.stack.pop();
    cx.key = outer;
    cx.revision = revision;
    // Keyed by region, so a region keeps its controls wherever its slot moves among siblings.
    with_key(element, format!("region:{key}").into())
}

/// Maps one node by its kind.
fn node(cx: &mut Cx, node: &Value) -> Element {
    match text(node, "kind") {
        "stack" => stack(cx, node),
        "text" => text_node(cx, node),
        "button" => button_node(cx, node),
        "field" => field(cx, node),
        "slot" => embed(cx, text(node, "key")),
        "spacer" => spacer(),
        kind => refused(
            cx,
            format!("node kind {kind} is not drawn by this host yet"),
        ),
    }
}

/// A column or a row of children, `gap` apart. A spacer among them takes the free space, so the
/// stack is then a grid whose spacers are star-sized.
fn stack(cx: &mut Cx, node: &Value) -> Element {
    let horizontal = text(node, "axis") == "horizontal";
    let gap = cx.look.gap();
    let items = list(node, "children");
    let children: Vec<Element> = items.iter().map(|child| self::node(cx, child)).collect();
    if !items.iter().any(|child| text(child, "kind") == "spacer") {
        let orientation = if horizontal {
            Orientation::Horizontal
        } else {
            Orientation::Vertical
        };
        return stack_panel()
            .orientation(orientation)
            .spacing(gap)
            .children(children)
            .into();
    }
    let sizes: Vec<&str> = items
        .iter()
        .map(|child| {
            if text(child, "kind") == "spacer" {
                "*"
            } else {
                "Auto"
            }
        })
        .collect();
    let placed: Vec<Element> = children
        .into_iter()
        .enumerate()
        .map(|(index, child)| {
            let cell = border().child(child);
            if horizontal {
                cell.grid_column(index as i32)
            } else {
                cell.grid_row(index as i32)
            }
            .into()
        })
        .collect();
    let grid = grid().children(placed);
    if horizontal {
        grid.column_definitions(&sizes.join(","))
            .column_spacing(gap)
            .into()
    } else {
        grid.row_definitions(&sizes.join(","))
            .row_spacing(gap)
            .into()
    }
}

/// Free space in a stack (see `stack`); alone it is an empty element.
fn spacer() -> Element {
    border().into()
}

/// Text in its role's size and weight and its tone's colour.
fn text_node(cx: &mut Cx, node: &Value) -> Element {
    let role = text(node, "role");
    let mut element = text_block()
        .text(text(node, "text").to_owned())
        .text_wrapping(ferroui_base::media::TextWrapping::Wrap);
    if let Some(size) = theme::role_size(role) {
        element = element.font_size(size);
    }
    if theme::role_bold(role) {
        element = element.font_weight(FontWeight::SemiBold);
    }
    if role == "mono" {
        element = element.font_family(ferroui_base::media::FontFamily::from(
            "Menlo, Consolas, monospace",
        ));
    }
    if let Some(ink) = tone(cx, text(node, "tone"), theme::role_tone(role)) {
        element = element.foreground(ink);
    }
    element.into()
}

/// The colour of a tone, else of the role's fallback token; `None` leaves FerroUI's colour.
fn tone(cx: &Cx, tone: &str, fallback: &str) -> Option<Color> {
    cx.look.colour(tone).or_else(|| cx.look.colour(fallback))
}

/// The binding of a control: the region, its revision and the ordinal the framework issued.
#[derive(Clone)]
struct Binding {
    region: String,
    revision: u64,
    ordinal: u64,
}

impl Binding {
    fn send(&self, host: &Rc<Host>, value: Value) {
        host.control(&self.region, self.revision, self.ordinal, value);
    }
}

fn binding(cx: &Cx, node: &Value, member: &str) -> Option<Binding> {
    let ordinal = node.get(member).and_then(Value::as_u64)?;
    Some(Binding {
        region: cx.key.clone(),
        revision: cx.revision,
        ordinal,
    })
}

/// A symbol's glyph, or a box naming a symbol the framework does not define.
fn symbol(cx: &mut Cx, name: &str) -> Element {
    match crate::symbols::glyph(name) {
        Some(glyph) => text_block()
            .text(glyph)
            .vertical_alignment(VerticalAlignment::Center)
            .into(),
        None => refused(cx, format!("unknown symbol {name}")),
    }
}

/// A button: its label after its symbol, its role's look, enabled or not; a click sends a tap.
fn button_node(cx: &mut Cx, node: &Value) -> Element {
    let Some(binding) = binding(cx, node, "event") else {
        return refused(cx, "a button has no event ordinal".to_owned());
    };
    let label = text(node, "label").to_owned();
    let name = text(node, "symbol");
    let content: Element = if name.is_empty() {
        Element::Text(Rc::from(label.as_str()))
    } else {
        stack_panel()
            .orientation(Orientation::Horizontal)
            .spacing(6.0)
            .child(symbol(cx, name))
            .child(
                text_block()
                    .text(label)
                    .vertical_alignment(VerticalAlignment::Center),
            )
            .into()
    };
    let host = cx.host.clone();
    let mut element = button()
        .child(content)
        .is_enabled(flag(node, "enabled"))
        .on_click(move |_| {
            binding.send(&host, Value::Null);
        });
    match text(node, "role") {
        "prominent" => {
            element = element.class("accent");
            if let Some(fill) = cx.look.colour("accent") {
                element = element.background(fill);
            }
        }
        "destructive" => {
            if let Some(ink) = cx.look.colour("danger") {
                element = element.foreground(ink);
            }
        }
        _ => {
            // A theme that names the colours of a plain button draws it with them.
            if let (Some(fill), Some(ink)) =
                (cx.look.colour("button"), cx.look.colour("buttonText"))
            {
                element = element.background(fill).foreground(ink);
            }
        }
    }
    element.into()
}

/// A labelled text field. `edit` sends every change; `submit` sends the text on Return.
fn field(cx: &mut Cx, node: &Value) -> Element {
    let Some(binding) = binding(cx, node, "event") else {
        return refused(cx, "a field has no event ordinal".to_owned());
    };
    let value = text(node, "value").to_owned();
    let host = cx.host.clone();
    let mut input = text_box()
        .text(value.clone())
        .placeholder_text(text(node, "prompt").to_owned())
        .accepts_return(flag(node, "multiline"))
        .horizontal_alignment(HorizontalAlignment::Stretch);
    if flag(node, "secure") {
        input = input.password_char('\u{2022}');
    }
    if flag(node, "multiline") {
        input = input
            .text_wrapping(ferroui_base::media::TextWrapping::Wrap)
            .min_height(64.0);
    }
    let typed = Rc::new(RefCell::new(value.clone()));
    let seen = typed.clone();
    let edit = text(node, "commit") == "edit";
    let on_edit = binding.clone();
    let edit_host = host.clone();
    input = input.on_changed(TextBox::text_property(), move |text: &Option<String>| {
        let text = text.clone().unwrap_or_default();
        *seen.borrow_mut() = text.clone();
        if edit && !edit_host.is_patching() && text != value {
            on_edit.send(&edit_host, Value::String(text));
        }
    });
    if !edit {
        input = input.on_key_down(move |event| {
            if event.key == KeyName::Return && !host.is_patching() {
                binding.send(&host, Value::String(typed.borrow().clone()));
            }
        });
    }
    let label = text(node, "label");
    let mut column = stack_panel().spacing(4.0);
    if !label.is_empty() {
        column = column.child(text_block().text(label.to_owned()));
    }
    column.child(input).into()
}

/// A part this host cannot draw: a box that names it, and a diagnostic.
fn refused(cx: &mut Cx, message: String) -> Element {
    cx.host.refuse(message.clone());
    let ink = Color::from_argb(0xFF, 0xC4, 0x2B, 0x1C);
    border()
        .border_brush(ink)
        .border_thickness(1.0)
        .padding(Thickness::uniform(6.0))
        .child(text_block().text(message).foreground(ink).font_size(12.0))
        .into()
}

/// A string member of a checked node.
pub(crate) fn text<'a>(node: &'a Value, name: &str) -> &'a str {
    node.get(name).and_then(Value::as_str).unwrap_or("")
}

/// A boolean member of a checked node.
pub(crate) fn flag(node: &Value, name: &str) -> bool {
    node.get(name).and_then(Value::as_bool).unwrap_or(false)
}

/// An array member of a checked node.
pub(crate) fn list<'a>(node: &'a Value, name: &str) -> &'a [Value] {
    node.get(name)
        .and_then(Value::as_array)
        .map_or(&[], Vec::as_slice)
}

#[cfg(test)]
mod tests {
    use super::{DRAWN, REFUSED};
    use crate::schema::KINDS;

    const SOURCE: &str = include_str!("view.rs");

    #[test]
    fn every_node_kind_is_drawn_or_refused_exactly_once() {
        let mut schema: Vec<&str> = KINDS.iter().map(|kind| kind.name).collect();
        schema.sort_unstable();
        let mut declared: Vec<&str> = DRAWN.iter().chain(REFUSED).copied().collect();
        declared.sort_unstable();
        assert_eq!(
            declared, schema,
            "DRAWN and REFUSED must name every kind of the schema once"
        );
    }

    #[test]
    fn every_drawn_kind_has_its_mapping_and_no_refused_kind_has_one() {
        for kind in DRAWN {
            assert!(
                SOURCE.contains(&format!("\"{kind}\" =>")),
                "view.rs maps no {kind}"
            );
        }
        for kind in REFUSED {
            assert!(
                !SOURCE.contains(&format!("\"{kind}\" =>")),
                "view.rs maps {kind}: move it to DRAWN"
            );
        }
    }

    /// With `OSPREY_NATIVE_SOURCE` naming the `src` folder of the framework's `@osprey/native`
    /// package, the schema is checked against the framework's own `Node` type: a kind added there
    /// fails this test until the schema, and so `DRAWN` or `REFUSED`, has it.
    #[test]
    fn the_schema_has_every_kind_of_the_framework() {
        let Some(folder) = std::env::var_os("OSPREY_NATIVE_SOURCE") else {
            return;
        };
        let source = std::fs::read_to_string(std::path::Path::new(&folder).join("node.ospml"))
            .expect("node.ospml");
        let mut framework: Vec<String> = source
            .lines()
            .skip_while(|line| !line.starts_with("type Node A ="))
            .skip(1)
            .take_while(|line| line.is_empty() || line.starts_with(char::is_whitespace))
            .filter_map(|line| line.strip_prefix("    "))
            .filter(|name| {
                name.starts_with(char::is_uppercase) && name.chars().all(char::is_alphanumeric)
            })
            .map(wire)
            .collect();
        framework.sort();
        let mut schema: Vec<String> = KINDS.iter().map(|kind| kind.name.to_owned()).collect();
        schema.sort();
        assert_eq!(schema, framework);
    }

    /// The encoder's wire name of a constructor: `ContextMenu` is `context-menu`, `Typeset` is `math`.
    fn wire(name: &str) -> String {
        if name == "Typeset" {
            return "math".to_owned();
        }
        let mut wire = String::new();
        for (index, letter) in name.chars().enumerate() {
            if letter.is_uppercase() && index > 0 {
                wire.push('-');
            }
            wire.push(letter.to_ascii_lowercase());
        }
        wire
    }
}
