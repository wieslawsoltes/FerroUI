//! The node kinds of Osprey Native as FerroUI elements (docs/osprey-native-host.md, section 4):
//! one mapping per kind. A kind this host does not draw yet is drawn as a visible box that names
//! it and is reported; it is never drawn as something else. `DRAWN` and `REFUSED` together are
//! every kind of the schema, each once, which the tests check.

use std::cell::RefCell;
use std::rc::Rc;

use ferroui_base::input::Key as KeyName;
use ferroui_base::layout::{HorizontalAlignment, Orientation, VerticalAlignment};
use ferroui_base::media::{Color, FontFamily, FontStyle, FontWeight, TextWrapping};
use ferroui_base::utilities::UriRef;
use ferroui_base::{CornerRadius, Thickness};
use ferroui_controls::primitives::{SelectingItemsControl, ToggleButton};
use ferroui_controls::{Expander, TextBox};
use ferroui_view::element::with_key;
use ferroui_view::prelude::*;
use serde_json::Value;

use crate::host::Host;
use crate::session::Session;
use crate::theme::{self, Look};

/// The node kinds this host draws, each with its mapping in `node` or `container`.
pub const DRAWN: &[&str] = &[
    "stack",
    "text",
    "button",
    "field",
    "slot",
    "spacer",
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
    "editor",
];

/// The node kinds this host draws as a box that names them, until it maps them.
pub const REFUSED: &[&str] = &["math", "image", "video", "board", "scene"];

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
    /// The shapes the scenes of the region being drawn were streamed, by `streamSlot`.
    streams: Vec<Rc<Vec<crate::boundary::Streamed>>>,
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
        streams: Vec::new(),
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
    let streams = std::mem::replace(&mut cx.streams, region.streams.clone());
    cx.stack.push(key.to_owned());
    let element = self::node(cx, &node);
    cx.stack.pop();
    cx.streams = streams;
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
        "rich-text" => rich_text(cx, node),
        "button" => button_node(cx, node),
        "field" => field(cx, node),
        "slot" => embed(cx, text(node, "key")),
        "spacer" => spacer(),
        "evidence" => pair(cx, node),
        "pair" => pair(cx, node),
        "badge" => badge(cx, node),
        "label" => label_node(cx, node),
        "icon" => icon(cx, node),
        "link" => link(cx, node),
        "divider" => separator()
            .margin(Thickness::new(0.0, 2.0, 0.0, 2.0))
            .into(),
        "segments" => segments(cx, node),
        "empty" => empty(cx, node),
        "progress" => progress(cx, node),
        "meter" => meter(cx, node),
        "swatch" => swatch(cx, node),
        "brand" => brand(cx),
        "toggle" => toggle(cx, node),
        "picker" => picker(cx, node),
        kind => container(cx, kind, node),
    }
}

/// Maps the containers and the collections.
fn container(cx: &mut Cx, kind: &str, node: &Value) -> Element {
    match kind {
        "disclosure" => disclosure(cx, node),
        "alert" => alert(cx, node),
        "section" => section(cx, node),
        "screen" => screen(cx, node),
        "split" => split(cx, node),
        "layer" => layer(cx, node),
        "framed" => framed(cx, node),
        "themed" => themed(cx, node),
        "dialog" => dialog(cx, node),
        "context-menu" => context_menu(cx, node),
        "aside" => aside(cx, node),
        "tabs" => tabs(cx, node),
        "rows" => rows(cx, node),
        "editor" => editor(cx, node),
        kind => refused(
            cx,
            format!("node kind {kind} is not drawn by this host yet"),
        ),
    }
}

/// A column or a row of children, `gap` apart. A spacer among them takes the free space, so the
/// stack is then a grid whose spacers are star-sized.
fn stack(cx: &mut Cx, node: &Value) -> Element {
    stacked(
        cx,
        list(node, "children"),
        text(node, "axis") == "horizontal",
    )
}

/// The children `items` as a stack (see `stack`).
fn stacked(cx: &mut Cx, items: &[Value], horizontal: bool) -> Element {
    let gap = cx.look.gap();
    let children: Vec<Element> = items.iter().map(|child| self::node(cx, child)).collect();
    // A spacer takes the free space; so does a child that holds a scene, as the GPUI host passes
    // free space down to scenes.
    let grows: Vec<bool> = items
        .iter()
        .map(|child| text(child, "kind") == "spacer" || holds_scene(cx, child, 0))
        .collect();
    if !grows.contains(&true) {
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
    let sizes: Vec<&str> = grows
        .iter()
        .map(|grows| if *grows { "*" } else { "Auto" })
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

/// Whether a scene is below this node (through slots too), so its stacks give it the free space.
fn holds_scene(cx: &Cx, node: &Value, depth: usize) -> bool {
    if depth > 32 {
        return false;
    }
    match text(node, "kind") {
        "scene" => true,
        "slot" => cx
            .session
            .region(text(node, "key"))
            .is_some_and(|region| holds_scene(cx, &region.node, depth + 1)),
        _ => ["children", "body", "pane"]
            .iter()
            .flat_map(|member| match node.get(*member) {
                Some(Value::Array(items)) => items.iter().collect::<Vec<_>>(),
                Some(single @ Value::Object(_)) => vec![single],
                _ => Vec::new(),
            })
            .any(|child| holds_scene(cx, child, depth + 1)),
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

/// Muted text: the theme's `secondary` colour, else FerroUI's text at lower opacity.
fn muted(cx: &Cx, text: &str) -> Element {
    let element = text_block()
        .text(text.to_owned())
        .font_size(12.0)
        .text_wrapping(TextWrapping::Wrap);
    match cx.look.colour("secondary") {
        Some(ink) => element.foreground(ink).into(),
        None => element.opacity(0.72).into(),
    }
}

/// The children of `member`, mapped in order.
fn children(cx: &mut Cx, node: &Value, member: &str) -> Vec<Element> {
    list(node, member)
        .iter()
        .map(|child| self::node(cx, child))
        .collect()
}

/// A column of mapped children `gap` apart.
fn column(cx: &mut Cx, node: &Value, member: &str) -> Element {
    let gap = cx.look.gap();
    stack_panel()
        .spacing(gap)
        .children(children(cx, node, member))
        .into()
}

/// The theme colour of a semantic tone, else of the accent.
fn tone_or_accent(cx: &Cx, tone: &str) -> Option<Color> {
    cx.look.colour(tone).or_else(|| cx.look.colour("accent"))
}

/// A muted label and its value on one line (`pair`, `evidence`).
fn pair(cx: &mut Cx, node: &Value) -> Element {
    stack_panel()
        .orientation(Orientation::Horizontal)
        .spacing(cx.look.gap())
        .child(muted(cx, text(node, "label")))
        .child(text_block().text(text(node, "value").to_owned()))
        .into()
}

/// A symbol and a label.
fn label_node(cx: &mut Cx, node: &Value) -> Element {
    let mut row = stack_panel()
        .orientation(Orientation::Horizontal)
        .spacing(cx.look.gap());
    if !text(node, "symbol").is_empty() {
        row = row.child(symbol(cx, text(node, "symbol")));
    }
    row.child(
        text_block()
            .text(text(node, "label").to_owned())
            .vertical_alignment(VerticalAlignment::Center),
    )
    .into()
}

/// A large decorative symbol.
fn icon(cx: &mut Cx, node: &Value) -> Element {
    match crate::symbols::glyph(text(node, "symbol")) {
        Some(glyph) => text_block().text(glyph).font_size(28.0).into(),
        None => refused(cx, format!("unknown symbol {}", text(node, "symbol"))),
    }
}

/// A small rounded label in its tone's colour, its symbol before it.
fn badge(cx: &mut Cx, node: &Value) -> Element {
    let mut content = stack_panel()
        .orientation(Orientation::Horizontal)
        .spacing(4.0);
    if !text(node, "symbol").is_empty() {
        content = content.child(symbol(cx, text(node, "symbol")));
    }
    let mut label = text_block()
        .text(text(node, "label").to_owned())
        .font_size(12.0);
    let mut chip = border()
        .corner_radius(CornerRadius::uniform(9.0))
        .border_thickness(1.0)
        .padding(Thickness::new(8.0, 2.0, 8.0, 2.0))
        .horizontal_alignment(HorizontalAlignment::Left);
    match tone_or_accent(cx, text(node, "tone")) {
        Some(tone) => {
            label = label.foreground(tone);
            chip = chip.border_brush(tone);
        }
        None => chip = chip.border_brush(Color::from_argb(0x80, 0x80, 0x80, 0x80)),
    }
    chip.child(content.child(label)).into()
}

/// A labelled `https` page the system opens; any other address is refused.
fn link(cx: &mut Cx, node: &Value) -> Element {
    let url = text(node, "url");
    let parsed = url
        .starts_with("https://")
        .then(|| UriRef::parse(url).ok())
        .flatten();
    match parsed {
        Some(uri) => hyperlink_button()
            .navigate_uri(Some(uri))
            .child(text(node, "label").to_owned())
            .into(),
        None => refused(
            cx,
            format!("a link to {url}, which is not an https address"),
        ),
    }
}

/// A strip of small blocks in their tones' colours, and its label.
fn segments(cx: &mut Cx, node: &Value) -> Element {
    let blocks: Vec<Element> = list(node, "tones")
        .iter()
        .map(|tone| {
            let fill = tone_or_accent(cx, tone.as_str().unwrap_or(""))
                .unwrap_or(Color::from_argb(0x80, 0x80, 0x80, 0x80));
            border()
                .width(16.0)
                .height(8.0)
                .corner_radius(CornerRadius::uniform(2.0))
                .background(fill)
                .into()
        })
        .collect();
    stack_panel()
        .orientation(Orientation::Horizontal)
        .spacing(cx.look.gap())
        .child(
            stack_panel()
                .orientation(Orientation::Horizontal)
                .spacing(2.0)
                .children(blocks),
        )
        .child(muted(cx, text(node, "label")))
        .into()
}

/// A centred empty state: its symbol, title and detail.
fn empty(cx: &mut Cx, node: &Value) -> Element {
    let mut panel = stack_panel()
        .spacing(cx.look.gap())
        .horizontal_alignment(HorizontalAlignment::Center)
        .vertical_alignment(VerticalAlignment::Center);
    if !text(node, "symbol").is_empty() {
        panel = panel.child(icon(cx, node));
    }
    panel
        .child(
            text_block()
                .text(text(node, "title").to_owned())
                .font_weight(FontWeight::SemiBold),
        )
        .child(muted(cx, text(node, "detail")))
        .into()
}

/// An indeterminate progress bar under its label and detail.
fn progress(cx: &mut Cx, node: &Value) -> Element {
    stack_panel()
        .spacing(4.0)
        .child(text_block().text(text(node, "label").to_owned()))
        .child(muted(cx, text(node, "detail")))
        .child(progress_bar().is_indeterminate(true))
        .into()
}

/// A bar filled to `value` of `total` in its tone; the framework clamped both.
fn meter(cx: &mut Cx, node: &Value) -> Element {
    let total = node
        .get("total")
        .and_then(Value::as_i64)
        .unwrap_or(1)
        .max(1) as f64;
    let value = node
        .get("value")
        .and_then(Value::as_i64)
        .unwrap_or(0)
        .max(0) as f64;
    let mut bar = progress_bar()
        .minimum(0.0)
        .maximum(total)
        .value(value.min(total))
        .min_width(120.0);
    if let Some(tone) = tone_or_accent(cx, text(node, "tone")) {
        bar = bar.foreground(tone);
    }
    bar.into()
}

/// A colour in the current appearance beside its name.
fn swatch(cx: &mut Cx, node: &Value) -> Element {
    let shown = if cx.look.dark {
        text(node, "dark")
    } else {
        text(node, "light")
    };
    let Some(fill) = theme::hex(shown) else {
        return refused(cx, format!("a swatch has the colour {shown}"));
    };
    stack_panel()
        .orientation(Orientation::Horizontal)
        .spacing(cx.look.gap())
        .child(
            border()
                .width(16.0)
                .height(16.0)
                .corner_radius(CornerRadius::uniform(3.0))
                .background(fill)
                .border_thickness(1.0)
                .border_brush(Color::from_argb(0x40, 0x80, 0x80, 0x80)),
        )
        .child(
            text_block()
                .text(text(node, "name").to_owned())
                .vertical_alignment(VerticalAlignment::Center),
        )
        .into()
}

/// The framework's wordmark.
fn brand(cx: &mut Cx) -> Element {
    let mut mark = text_block()
        .text("Osprey")
        .font_size(20.0)
        .font_weight(FontWeight::SemiBold);
    if let Some(accent) = cx.look.colour("accent") {
        mark = mark.foreground(accent);
    }
    mark.into()
}

/// A switch with its label; a change sends the new state.
fn toggle(cx: &mut Cx, node: &Value) -> Element {
    let Some(binding) = binding(cx, node, "event") else {
        return refused(cx, "a toggle has no event ordinal".to_owned());
    };
    let on = flag(node, "on");
    let host = cx.host.clone();
    toggle_switch()
        .child(text(node, "label").to_owned())
        .is_checked(Some(on))
        .on_changed(
            ToggleButton::is_checked_property(),
            move |checked: &Option<bool>| {
                let checked = checked.unwrap_or(false);
                if checked != on && !host.is_patching() {
                    binding.send(&host, Value::Bool(checked));
                }
            },
        )
        .into()
}

/// The index of the key `selected` among `keys`, or -1.
fn index_of(keys: &[Option<String>], selected: &str) -> i32 {
    keys.iter()
        .position(|key| key.as_deref() == Some(selected))
        .map_or(-1, |index| index as i32)
}

/// Sends the key at a newly selected index of a choice control, when it is not the selected one.
fn on_choice(
    host: Rc<Host>,
    binding: Binding,
    keys: Vec<Option<String>>,
    selected: String,
) -> impl Fn(&i32) + 'static {
    move |index: &i32| {
        let Some(Some(key)) = usize::try_from(*index)
            .ok()
            .and_then(|index| keys.get(index))
        else {
            return;
        };
        if *key != selected && !host.is_patching() {
            binding.send(&host, Value::String(key.clone()));
        }
    }
}

/// A labelled drop-down of choices; choosing one sends its key.
fn picker(cx: &mut Cx, node: &Value) -> Element {
    let Some(binding) = binding(cx, node, "event") else {
        return refused(cx, "a picker has no event ordinal".to_owned());
    };
    let choices = list(node, "choices");
    let keys: Vec<Option<String>> = choices
        .iter()
        .map(|choice| Some(text(choice, "key").to_owned()))
        .collect();
    let selected = text(node, "selected").to_owned();
    let labels: Vec<Element> = choices
        .iter()
        .map(|choice| Element::Text(Rc::from(text(choice, "label"))))
        .collect();
    let index = index_of(&keys, &selected);
    let changed = on_choice(cx.host.clone(), binding, keys, selected);
    let mut row = stack_panel()
        .orientation(Orientation::Horizontal)
        .spacing(cx.look.gap());
    if !text(node, "label").is_empty() {
        row = row.child(
            text_block()
                .text(text(node, "label").to_owned())
                .vertical_alignment(VerticalAlignment::Center),
        );
    }
    row.child(
        combo_box()
            .children(labels)
            .selected_index(index)
            .min_width(120.0)
            .on_changed(SelectingItemsControl::selected_index_property(), changed),
    )
    .into()
}

/// A titled section that expands to its children; expanding or collapsing sends the new state.
fn disclosure(cx: &mut Cx, node: &Value) -> Element {
    let Some(binding) = binding(cx, node, "event") else {
        return refused(cx, "a disclosure has no event ordinal".to_owned());
    };
    let expanded = flag(node, "expanded");
    let host = cx.host.clone();
    let body = column(cx, node, "children");
    expander()
        .header(text_block().text(text(node, "title").to_owned()))
        .is_expanded(expanded)
        .horizontal_alignment(HorizontalAlignment::Stretch)
        .child(body)
        .on_changed(Expander::is_expanded_property(), move |now: &bool| {
            if *now != expanded && !host.is_patching() {
                binding.send(&host, Value::Bool(*now));
            }
        })
        .into()
}

/// A card with an outline, padding and rounded corners: sections, alerts and dialogs.
fn card(cx: &Cx, line: &str) -> El<ferroui_controls::Border> {
    let edge = cx
        .look
        .colour(line)
        .unwrap_or(Color::from_argb(0x40, 0x80, 0x80, 0x80));
    border()
        .border_thickness(1.0)
        .border_brush(edge)
        .corner_radius(CornerRadius::uniform(6.0))
        .padding(Thickness::uniform(12.0))
}

/// While visible, the alert's title and message and a button that dismisses it (one tap).
fn alert(cx: &mut Cx, node: &Value) -> Element {
    let Some(binding) = binding(cx, node, "event") else {
        return refused(cx, "an alert has no event ordinal".to_owned());
    };
    if !flag(node, "visible") {
        return Element::Empty;
    }
    let host = cx.host.clone();
    card(cx, "caution")
        .child(
            stack_panel()
                .spacing(cx.look.gap())
                .child(
                    text_block()
                        .text(text(node, "title").to_owned())
                        .font_weight(FontWeight::SemiBold),
                )
                .child(
                    text_block()
                        .text(text(node, "message").to_owned())
                        .text_wrapping(TextWrapping::Wrap),
                )
                .child(
                    button()
                        .child("OK")
                        .horizontal_alignment(HorizontalAlignment::Right)
                        .on_click(move |_| {
                            binding.send(&host, Value::Null);
                        }),
                ),
        )
        .into()
}

/// A titled group: its title, children and footer caption.
fn section(cx: &mut Cx, node: &Value) -> Element {
    let mut panel = stack_panel()
        .spacing(cx.look.gap())
        .child(
            text_block()
                .text(text(node, "title").to_owned())
                .font_weight(FontWeight::SemiBold),
        )
        .children(children(cx, node, "children"));
    if !text(node, "footer").is_empty() {
        panel = panel.child(muted(cx, text(node, "footer")));
    }
    card(cx, "separator").child(panel).into()
}

/// A screen: its title and subtitle with the toolbar at the trailing edge, above its body.
fn screen(cx: &mut Cx, node: &Value) -> Element {
    let gap = cx.look.gap();
    let mut heading = stack_panel().grid_column(0).child(
        text_block()
            .text(text(node, "title").to_owned())
            .font_size(20.0)
            .font_weight(FontWeight::SemiBold),
    );
    if !text(node, "subtitle").is_empty() {
        heading = heading.child(muted(cx, text(node, "subtitle")));
    }
    let toolbar = stack_panel()
        .grid_column(1)
        .orientation(Orientation::Horizontal)
        .spacing(gap)
        .vertical_alignment(VerticalAlignment::Center)
        .children(children(cx, node, "toolbar"));
    let body = stacked(cx, list(node, "children"), false);
    grid()
        .row_definitions("Auto,*")
        .row_spacing(gap)
        .child(
            grid()
                .grid_row(0)
                .column_definitions("*,Auto")
                .child(heading)
                .child(toolbar),
        )
        .child(border().grid_row(1).child(body))
        .into()
}

/// Panes side by side, equal at first, with splitters between them.
fn split(cx: &mut Cx, node: &Value) -> Element {
    let panes = children(cx, node, "children");
    let count = panes.len();
    let mut columns = Vec::new();
    let mut cells: Vec<Element> = Vec::new();
    for (index, pane) in panes.into_iter().enumerate() {
        if index > 0 {
            columns.push("Auto");
            cells.push(
                grid_splitter()
                    .grid_column((columns.len() - 1) as i32)
                    .width(4.0)
                    .into(),
            );
        }
        columns.push("*");
        cells.push(
            border()
                .grid_column((columns.len() - 1) as i32)
                .child(pane)
                .into(),
        );
    }
    if count == 0 {
        return Element::Empty;
    }
    grid()
        .column_definitions(&columns.join(","))
        .children(cells)
        .into()
}

/// Children drawn over each other, the first at the back.
fn layer(cx: &mut Cx, node: &Value) -> Element {
    grid().children(children(cx, node, "children")).into()
}

/// The body in the frame's geometry, surface tokens and type. The control metrics of a frame
/// (control height, padding, symbol size and the quiet, icon-only and flat options) are not
/// applied: FerroUI's controls keep their theme's metrics.
fn framed(cx: &mut Cx, node: &Value) -> Element {
    let style = node.get("style").cloned().unwrap_or(Value::Null);
    let whole = |name: &str| style.get(name).and_then(Value::as_i64).unwrap_or(0);
    let named = |name: &str| {
        style
            .get(name)
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_owned()
    };
    let inset = whole("viewportHeightInset");
    if style.get("viewportHeightInset").is_some() && inset != -1 {
        return refused(
            cx,
            format!(
                "a frame sized to the window less {inset} points, which this host does not draw"
            ),
        );
    }
    let body = match node.get("body") {
        Some(body) => self::node(cx, body),
        None => Element::Empty,
    };
    let edge = |value: i64| value.max(0) as f64;
    let mut frame = border()
        .padding(Thickness::new(
            edge(whole("left")),
            edge(whole("top")),
            edge(whole("right")),
            edge(whole("bottom")),
        ))
        .corner_radius(CornerRadius::uniform(edge(whole("radius"))));
    if whole("width") > 0 {
        frame = frame.width(whole("width") as f64);
    }
    if whole("height") > 0 {
        frame = frame.height(whole("height") as f64);
    }
    if let Some(fill) = cx.look.colour(&named("fill")) {
        frame = frame.background(fill);
    }
    if let Some(line) = cx.look.colour(&named("line")) {
        let thickness = match named("lineEdge").as_str() {
            "top" => Thickness::new(0.0, 1.0, 0.0, 0.0),
            "bottom" => Thickness::new(0.0, 0.0, 0.0, 1.0),
            "leading" => Thickness::new(1.0, 0.0, 0.0, 0.0),
            "trailing" => Thickness::new(0.0, 0.0, 1.0, 0.0),
            "none" => Thickness::uniform(0.0),
            _ => Thickness::uniform(1.0),
        };
        frame = frame.border_brush(line).border_thickness(thickness);
    }
    frame = match named("align").as_str() {
        "center" => frame.horizontal_alignment(HorizontalAlignment::Center),
        "end" => frame.horizontal_alignment(HorizontalAlignment::Right),
        _ if style.get("grow").and_then(Value::as_bool).unwrap_or(false) => {
            frame.horizontal_alignment(HorizontalAlignment::Stretch)
        }
        _ => frame,
    };
    let content: Element = if style
        .get("scrollY")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        scroll_viewer().child(body).into()
    } else {
        body
    };
    // The frame's type reaches the text inside it through a content control, whose font and
    // foreground its descendants inherit.
    let mut typed = content_control().child(content);
    if whole("textSize") > 0 {
        typed = typed.font_size(whole("textSize") as f64);
    }
    if !named("font").is_empty() {
        typed = typed.font_family(FontFamily::from(named("font").as_str()));
    }
    if style
        .get("italic")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        typed = typed.font_style(FontStyle::Italic);
    }
    if let Some(ink) = cx.look.colour(&named("ink")) {
        typed = typed.foreground(ink);
    }
    frame.child(typed).into()
}

/// The body with its own theme: its colours and spacing apply below it, on its background.
fn themed(cx: &mut Cx, node: &Value) -> Element {
    let theme = match node.get("theme").map(crate::protocol::theme) {
        Some(Ok(theme)) => Rc::new(theme),
        _ => return refused(cx, "a themed node without a valid theme".to_owned()),
    };
    let dark = cx.look.dark;
    let outer = std::mem::replace(
        &mut cx.look,
        Look {
            theme: Some(theme),
            dark,
        },
    );
    let gap = cx.look.gap();
    let fill = cx.look.colour("background");
    let body = match node.get("body") {
        Some(body) => self::node(cx, body),
        None => Element::Empty,
    };
    let mut surface = border().padding(Thickness::uniform(gap * 3.0)).child(body);
    if let Some(fill) = fill {
        surface = surface.background(fill);
    }
    cx.look = outer;
    surface.into()
}

/// A modal surface drawn in place: its title, its body and a button that dismisses it (a tap).
fn dialog(cx: &mut Cx, node: &Value) -> Element {
    let Some(binding) = binding(cx, node, "event") else {
        return refused(cx, "a dialog has no event ordinal".to_owned());
    };
    let host = cx.host.clone();
    let body = match node.get("body") {
        Some(body) => self::node(cx, body),
        None => Element::Empty,
    };
    let mut surface = card(cx, "separator");
    if let Some(fill) = cx.look.colour("background") {
        surface = surface.background(fill);
    }
    surface
        .child(
            stack_panel()
                .spacing(cx.look.gap())
                .child(
                    text_block()
                        .text(text(node, "title").to_owned())
                        .font_size(17.0)
                        .font_weight(FontWeight::SemiBold),
                )
                .child(body)
                .child(
                    button()
                        .child("Close")
                        .horizontal_alignment(HorizontalAlignment::Right)
                        .on_click(move |_| {
                            binding.send(&host, Value::Null);
                        }),
                ),
        )
        .into()
}

/// The body with a context menu of its actions: each button among them is a menu item that
/// sends its tap; an action of another kind is refused.
fn context_menu(cx: &mut Cx, node: &Value) -> Element {
    let body = match node.get("body") {
        Some(body) => self::node(cx, body),
        None => Element::Empty,
    };
    let mut items: Vec<Element> = Vec::new();
    for item in list(node, "items") {
        if text(item, "kind") != "button" {
            refused(
                cx,
                format!("a context menu action of kind {}", text(item, "kind")),
            );
            continue;
        }
        let Some(binding) = binding(cx, item, "event") else {
            continue;
        };
        let host = cx.host.clone();
        items.push(
            menu_item()
                .header(text_block().text(text(item, "label").to_owned()))
                .is_enabled(flag(item, "enabled"))
                .on_click(move |_| binding.send(&host, Value::Null))
                .into(),
        );
    }
    border()
        .context_menu(ferroui_view::prelude::context_menu().children(items))
        .child(body)
        .into()
}

/// Two columns with a splitter: the pane, and the aside at its leading or trailing edge, its
/// width within `least` and `most`; dragging the splitter sends the aside's new width.
fn aside(cx: &mut Cx, node: &Value) -> Element {
    let Some(binding) = binding(cx, node, "event") else {
        return refused(cx, "an aside has no event ordinal".to_owned());
    };
    let whole = |name: &str| node.get(name).and_then(Value::as_i64).unwrap_or(0);
    let width = whole("width").max(0) as f64;
    let pane = match node.get("pane") {
        Some(pane) => self::node(cx, pane),
        None => Element::Empty,
    };
    let beside = match node.get("aside") {
        Some(beside) => self::node(cx, beside),
        None => Element::Empty,
    };
    let leading = text(node, "side") == "leading";
    let (aside_column, pane_column) = if leading { (0, 2) } else { (2, 0) };
    let host = cx.host.clone();
    let sized = move |e: &ferroui_controls::SizeChangedEventArgs| {
        let now = e.new_size().width.round();
        if (now - width).abs() >= 1.0 && !host.is_patching() {
            binding.send(&host, Value::from(now as i64));
        }
    };
    let least = whole("least").max(0) as f64;
    let most = if whole("most") > 0 {
        whole("most") as f64
    } else {
        f64::INFINITY
    };
    let columns = if leading {
        format!("{width},Auto,*")
    } else {
        format!("*,Auto,{width}")
    };
    grid()
        .column_definitions(&columns)
        .child(border().grid_column(pane_column).child(pane))
        .child(grid_splitter().grid_column(1).width(4.0))
        .child(
            border()
                .grid_column(aside_column)
                .min_width(least)
                .max_width(most)
                .on_size_changed(sized)
                .child(beside),
        )
        .into()
}

/// Tabs: one tab item per entry, its label as the header and its content below; choosing a tab
/// sends its key.
fn tabs(cx: &mut Cx, node: &Value) -> Element {
    let Some(binding) = binding(cx, node, "event") else {
        return refused(cx, "tabs have no event ordinal".to_owned());
    };
    let entries = list(node, "tabs");
    let keys: Vec<Option<String>> = entries
        .iter()
        .map(|entry| Some(text(entry, "key").to_owned()))
        .collect();
    let selected = text(node, "selected").to_owned();
    let items: Vec<Element> = entries
        .iter()
        .map(|entry| {
            let content = match entry.get("child") {
                Some(child) => self::node(cx, child),
                None => Element::Empty,
            };
            tab_item()
                .key(format!("tab:{}", text(entry, "key")))
                .header(text_block().text(text(entry, "label").to_owned()))
                .child(content)
                .into()
        })
        .collect();
    let index = index_of(&keys, &selected);
    let changed = on_choice(cx.host.clone(), binding, keys, selected);
    tab_control()
        .children(items)
        .selected_index(index)
        .on_changed(SelectingItemsControl::selected_index_property(), changed)
        .into()
}

/// Selectable rows: one list item per row (its symbol, then its content), with a header where
/// the group changes; choosing a row sends its key.
fn rows(cx: &mut Cx, node: &Value) -> Element {
    let Some(binding) = binding(cx, node, "event") else {
        return refused(cx, "rows have no event ordinal".to_owned());
    };
    let selected = text(node, "selected").to_owned();
    let mut keys: Vec<Option<String>> = Vec::new();
    let mut items: Vec<Element> = Vec::new();
    let mut group = String::new();
    for row in list(node, "rows") {
        let this_group = text(row, "group");
        if !this_group.is_empty() && this_group != group {
            group = this_group.to_owned();
            keys.push(None);
            items.push(
                list_box_item()
                    .key(format!("group:{group}"))
                    .is_enabled(false)
                    .child(muted(cx, &group))
                    .into(),
            );
        }
        let content = match row.get("child") {
            Some(child) => self::node(cx, child),
            None => Element::Empty,
        };
        let mut line = stack_panel()
            .orientation(Orientation::Horizontal)
            .spacing(cx.look.gap());
        if !text(row, "symbol").is_empty() {
            line = line.child(symbol(cx, text(row, "symbol")));
        }
        keys.push(Some(text(row, "key").to_owned()));
        items.push(
            list_box_item()
                .key(format!("row:{}", text(row, "key")))
                .child(line.child(content))
                .into(),
        );
    }
    let index = index_of(&keys, &selected);
    let changed = on_choice(cx.host.clone(), binding, keys, selected);
    list_box()
        .children(items)
        .selected_index(index)
        .on_changed(SelectingItemsControl::selected_index_property(), changed)
        .into()
}

/// A multi-line editor: its text goes to the `event` binding, its caret to `placeEvent`. The
/// marks of an editor (underlines, badges, notes by line) are not drawn yet.
fn editor(cx: &mut Cx, node: &Value) -> Element {
    let (Some(edit), Some(place)) = (binding(cx, node, "event"), binding(cx, node, "placeEvent"))
    else {
        return refused(cx, "an editor has no event ordinals".to_owned());
    };
    let value = text(node, "value").to_owned();
    let cursor = node.get("cursor").and_then(Value::as_i64).unwrap_or(0);
    let (edit_host, place_host) = (cx.host.clone(), cx.host.clone());
    let inline = flag(node, "inline");
    let mut input = text_box()
        .text(value.clone())
        .caret_index(cursor.clamp(0, i32::MAX as i64) as i32)
        .accepts_return(!inline)
        .accepts_tab(!inline)
        .font_family(FontFamily::from("Menlo, Consolas, monospace"))
        .on_changed(TextBox::text_property(), move |text: &Option<String>| {
            let text = text.clone().unwrap_or_default();
            if text != value && !edit_host.is_patching() {
                edit.send(&edit_host, Value::String(text));
            }
        })
        .on_changed(TextBox::caret_index_property(), move |at: &i32| {
            if i64::from(*at) != cursor && !place_host.is_patching() {
                place.send(&place_host, Value::from(*at));
            }
        });
    if !inline {
        input = input.min_height(120.0);
    }
    let mut panel = stack_panel().spacing(4.0);
    if !text(node, "label").is_empty() {
        panel = panel.child(text_block().text(text(node, "label").to_owned()));
    }
    panel.child(input).into()
}

/// A paragraph of runs: each run's ink, italics and emphasis (`strong`, `code`).
fn rich_text(cx: &mut Cx, node: &Value) -> Element {
    let role = text(node, "role");
    let size = theme::role_size(role);
    let runs: Vec<Element> = list(node, "runs")
        .iter()
        .map(|run| {
            let mut piece = text_block().text(text(run, "text").to_owned());
            if let Some(size) = size {
                piece = piece.font_size(size);
            }
            if let Some(ink) = cx
                .look
                .colour(text(run, "ink"))
                .or_else(|| cx.look.colour(theme::role_tone(role)))
            {
                piece = piece.foreground(ink);
            }
            if flag(run, "italic") {
                piece = piece.font_style(FontStyle::Italic);
            }
            match text(run, "emphasis") {
                "strong" => piece = piece.font_weight(FontWeight::SemiBold),
                "code" => piece = piece.font_family(FontFamily::from("Menlo, Consolas, monospace")),
                _ => {}
            }
            piece.into()
        })
        .collect();
    wrap_panel().children(runs).into()
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
