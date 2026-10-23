//! The inspector of the declarative API: the mounted trees of the thread as
//! instances ([`mounted_trees`]) and a view of them ([`inspector`]): the
//! components with their keys, runs and hooks, the controls with their style
//! classes and property values and where each value comes from, the text,
//! the lists that follow state with their items.
//!
//! With the feature `desktop`, `launch` opens it in a window on Shift+F12 in
//! debug builds ([`open_inspector`]); F12 is left to the DevTools.

use crate::element::{Element, Key};
use crate::hooks::{use_effect_with, use_hook, use_signal};
use crate::reactive::on_cleanup;
use crate::template::template_of;
use ferroui_base::layout::{Orientation, VerticalAlignment};
use ferroui_base::BoxedValue;
use ferroui_controls::ItemsSource;
use std::rc::Rc;

/// An instance of a mounted tree, described.
#[derive(Clone, Debug, PartialEq)]
pub struct InstanceNode {
    /// What it is.
    pub kind: InstanceKind,
    /// Its key among its siblings.
    pub key: Option<Key>,
    /// The instances below it.
    pub children: Vec<InstanceNode>,
}

/// What an instance is.
#[derive(Clone, Debug, PartialEq)]
pub enum InstanceKind {
    /// A component that runs again.
    Component {
        /// Its name.
        name: String,
        /// How often it ran.
        runs: u64,
        /// The number of its hooks.
        hooks: usize,
    },
    /// A control made from an element.
    Control {
        /// Its class.
        class: String,
        /// Its `Name`.
        name: Option<String>,
        /// Its style classes.
        classes: Vec<String>,
        /// The properties the element gives it.
        properties: Vec<PropertyInfo>,
    },
    /// Text.
    Text(String),
    /// A control made elsewhere.
    Existing {
        /// Its class.
        class: String,
    },
    /// Children that follow state.
    Dynamic {
        /// The number of items.
        items: usize,
    },
}

/// A property an element gives its control.
#[derive(Clone, Debug, PartialEq)]
pub struct PropertyInfo {
    /// The name of the property.
    pub name: String,
    /// Its value on the control now.
    pub value: String,
    /// Where the element's value comes from: `value`, `follows state`,
    /// `two-way`, `element`, `binding`.
    pub source: &'static str,
}

/// A mounted tree: what mounted it (`tree`, `ComponentHost`) and its root
/// instances.
#[derive(Clone, Debug, PartialEq)]
pub struct MountedTree {
    /// What mounted it.
    pub origin: &'static str,
    /// Its root instances.
    pub roots: Vec<InstanceNode>,
}

/// The live mounted trees of the thread, described now.
pub fn mounted_trees() -> Vec<MountedTree> {
    crate::mount::snapshot().into_iter().map(|(origin, roots)| MountedTree { origin, roots }).collect()
}

impl InstanceNode {
    /// A line that names the instance.
    pub fn label(&self) -> String {
        let key = self.key.as_ref().map(|key| match key {
            Key::Int(value) => format!(" key={value}"),
            Key::Str(value) => format!(" key=\"{value}\""),
        });
        let key = key.unwrap_or_default();
        match &self.kind {
            InstanceKind::Component { name, runs, hooks } => format!("<{name}>{key}  runs {runs}, hooks {hooks}"),
            InstanceKind::Control { class, name, classes, .. } => {
                let name = name.as_ref().map(|name| format!(" #{name}")).unwrap_or_default();
                let classes: String = classes.iter().map(|class| format!(".{class}")).collect();
                format!("{class}{name}{classes}{key}")
            }
            InstanceKind::Text(text) => format!("\"{text}\""),
            InstanceKind::Existing { class } => format!("{class} (existing){key}"),
            InstanceKind::Dynamic { items } => format!("[list that follows state: {items} items]"),
        }
    }

    /// Whether the node or a node below it is the inspector's own view.
    fn is_inspector(&self) -> bool {
        matches!(&self.kind, InstanceKind::Component { name, .. } if name == INSPECTOR_NAME)
            || self.children.iter().any(InstanceNode::is_inspector)
    }
}

const INSPECTOR_NAME: &str = "InspectorView";
/// What the hosts of the inspector's own rows are called.
const INSPECTOR_ORIGIN: &str = "inspector";

/// A row of the inspector: an instance at its depth, with what the details
/// pane shows.
#[derive(Clone, Debug, PartialEq)]
struct Row {
    depth: usize,
    label: String,
    details: Vec<String>,
}

fn rows(trees: &[MountedTree]) -> Vec<Row> {
    fn walk(node: &InstanceNode, depth: usize, out: &mut Vec<Row>) {
        let key = node.key.as_ref().map(|key| format!("key: {key:?}"));
        let mut details: Vec<String> = match &node.kind {
            InstanceKind::Control { class, name, classes, properties } => {
                let mut lines = vec![format!("control: {class}")];
                lines.extend(name.as_ref().map(|name| format!("name: {name}")));
                if !classes.is_empty() {
                    lines.push(format!("classes: {}", classes.join(" ")));
                }
                if properties.is_empty() {
                    lines.push(String::from("(no properties set by the element)"));
                }
                lines.extend(properties.iter().map(|p| format!("{} = {}   ({})", p.name, p.value, p.source)));
                lines
            }
            InstanceKind::Component { name, runs, hooks } => {
                vec![format!("component: {name}"), format!("runs: {runs}"), format!("hooks: {hooks}")]
            }
            InstanceKind::Text(text) => vec![format!("text: {text:?}")],
            InstanceKind::Existing { class } => vec![format!("an existing control: {class}")],
            InstanceKind::Dynamic { items } => vec![format!("a list that follows state, {items} items")],
        };
        details.extend(key);
        out.push(Row { depth, label: node.label(), details });
        for child in &node.children {
            walk(child, depth + 1, out);
        }
    }
    let mut out = Vec::new();
    for tree in trees.iter().filter(|tree| tree.origin != INSPECTOR_ORIGIN && !tree.roots.iter().any(InstanceNode::is_inspector)) {
        out.push(Row { depth: 0, label: format!("mounted: {}", tree.origin), details: Vec::new() });
        for root in &tree.roots {
            walk(root, 1, &mut out);
        }
    }
    out
}

/// The inspector: the instances of the mounted trees of the thread (but its
/// own), refreshed on a click or live (every second), with the details of the
/// selected one.
pub fn inspector() -> Element {
    crate::element::component_element::<InspectorMarker>(INSPECTOR_NAME, None, inspector_view, None)
}

struct InspectorMarker;

fn inspector_view() -> Element {
    let version = use_signal(|| 0_u64);
    let live = use_signal(|| Some(false));
    let selected = use_signal(|| -1_i32);
    use_effect_with(live.get() == Some(true), move |live| {
        if *live {
            let timer = ferroui_base::threading::DispatcherTimer::run(
                move || {
                    if version.is_alive() {
                        version.update(|v| *v += 1);
                    }
                    version.is_alive()
                },
                std::time::Duration::from_secs(1),
                ferroui_base::threading::DispatcherPriority::BACKGROUND,
            );
            on_cleanup(move || timer.dispose());
        }
    });
    let _ = version.get();
    let rows = rows(&mounted_trees());
    let details = usize::try_from(selected.get()).ok().and_then(|index| rows.get(index)).map(|row| row.details.clone()).unwrap_or_default();
    let count = rows.len();
    let source = ItemsSource::from_items(rows.into_iter().map(|row| Some(Rc::new(row) as BoxedValue)));
    let item_template = use_hook(|| {
        template_of(INSPECTOR_ORIGIN, |row: Row| {
            crate::generated::builders::text_block()
                .text(row.label)
                .margin((row.depth as f64 * 14.0, 0.0, 0.0, 0.0))
        })
    });
    use crate::prelude::*;
    grid()
        .row_definitions("Auto,*")
        .column_definitions("*,360")
        .margin(8.0)
        .child(
            stack_panel()
                .orientation(Orientation::Horizontal)
                .spacing(8.0)
                .margin((0.0, 0.0, 0.0, 8.0))
                .grid_column_span(2)
                .child(button().name("Refresh").content("Refresh").on_click(move |_| version.update(|v| *v += 1)))
                .child(check_box().name("Live").content("Live").is_checked(live.two_way()))
                .child(text_block().vertical_alignment(VerticalAlignment::Center).text(format!("{count} rows"))),
        )
        .child(list_box().name("Instances").grid_row(1).items_source(source).item_template(item_template).selected_index(selected.two_way()))
        .child(
            scroll_viewer().grid_row(1).grid_column(1).margin((8.0, 0.0, 0.0, 0.0)).child(
                stack_panel()
                    .spacing(2.0)
                    .children(details.into_iter().map(|line| text_block().text(line)).collect::<Vec<_>>()),
            ),
        )
        .into()
}

/// Opens the inspector in a window of its own.
#[cfg(feature = "desktop")]
pub fn open_inspector() -> ferroui_base::Ref<ferroui_controls::Window> {
    use crate::prelude::*;
    open_window(|| window().title("Components").width(960.0).height(640.0).child(inspector()).into_element())
}

/// Opens the inspector on Shift+F12 in `window` (`launch` does it in debug
/// builds).
#[cfg(feature = "desktop")]
pub fn attach_inspector(window: &ferroui_base::Ref<ferroui_controls::Window>) {
    use ferroui_base::input::{InputElement, Key as KeyCode, KeyModifiers};
    window.add_handler(InputElement::key_down_event(), |_, e| {
        if e.key == KeyCode::F12 && e.key_modifiers.contains(KeyModifiers::SHIFT) {
            let _ = open_inspector();
        }
    });
}
