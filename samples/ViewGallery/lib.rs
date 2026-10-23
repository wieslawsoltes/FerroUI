//! A gallery of phase 3 of the declarative API (`docs/declarative-ui.md`, section 10): a
//! component that runs again, under a theme variant it switches, with
//!
//! - styles of an element (`styles: [style("Button.accent")...]`) with a nested style;
//! - the theme variant and resources of the theme ([`use_theme_variant`], [`use_resource_as`]);
//! - a transition of a width ([`transition`]);
//! - a context menu given as an element;
//! - a colour view of `ferroui-controls-color-picker` and a TreeDataGrid of
//!   `ferroui-controls-treedatagrid`, written with their generated builders.
//!
//! `cargo run -p view-gallery`.

use ferroui_base::layout::{Layoutable, Orientation, VerticalAlignment};
use ferroui_base::media::IBrush;
use ferroui_base::styling::ThemeVariant;
use ferroui_base::{BoxedValue, Visual};
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::GridLength;
use ferroui_controls_treedatagrid::models::TextColumn;
use ferroui_controls_treedatagrid::TreeDataGridSourceRef;
use ferroui_treedatagrid_core::models::ColumnRef;
use ferroui_treedatagrid_core::{Accessor, FlatTreeDataGridSource, Models};
use ferroui_view::prelude::*;
use std::rc::Rc;

/// A row of the TreeDataGrid.
#[derive(Clone, PartialEq, Debug)]
pub struct Language {
    pub name: String,
    pub year: i32,
    pub paradigm: String,
}

fn languages() -> TreeDataGridSourceRef {
    let rows = [
        ("Rust", 2015, "multi-paradigm, systems"),
        ("C#", 2000, "object-oriented"),
        ("F#", 2005, "functional-first"),
        ("TypeScript", 2012, "typed JavaScript"),
        ("Haskell", 1990, "purely functional"),
        ("Swift", 2014, "multi-paradigm"),
        ("Kotlin", 2016, "multi-paradigm"),
        ("Go", 2012, "concurrent, imperative"),
    ];
    let items: Rc<Vec<Rc<Language>>> = Rc::new(
        rows.iter()
            .map(|(name, year, paradigm)| Rc::new(Language { name: name.to_string(), year: *year, paradigm: paradigm.to_string() }))
            .collect(),
    );
    let source = FlatTreeDataGridSource::new(items as Models<Language>);
    let header = |text: &str| Some(Rc::new(text.to_string()) as BoxedValue);
    source.columns().add(TextColumn::new(
        header("Language"),
        Accessor::member("Name", |x: &Rc<Language>| x.name.clone()),
        Some(GridLength::STAR),
        None,
    ) as ColumnRef<Language>);
    source.columns().add(
        TextColumn::new(header("Since"), Accessor::member("Year", |x: &Rc<Language>| x.year), None, None) as ColumnRef<Language>,
    );
    source.columns().add(TextColumn::new(
        header("Paradigm"),
        Accessor::member("Paradigm", |x: &Rc<Language>| x.paradigm.clone()),
        Some(GridLength::STAR),
        None,
    ) as ColumnRef<Language>);
    TreeDataGridSourceRef::of(source)
}

/// The gallery: the theme variant it shows is a hook of its own.
#[component(rerender)]
pub fn Gallery(#[prop(default)] start_dark: bool) -> Element {
    let dark = use_signal(move || start_dark);
    let variant = if dark.get() { ThemeVariant::dark() } else { ThemeVariant::light() };
    rsx! {
        ThemeVariantScope { requested_theme_variant: variant,
            Page { dark }
        }
    }
}

/// The page inside the theme variant scope: it reads the variant and the resources of the
/// scope, so it runs again when the variant changes.
#[component(rerender)]
fn Page(dark: Signal<bool>) -> Element {
    let variant = use_theme_variant();
    let background = use_resource_as::<Rc<dyn IBrush>>("SystemControlBackgroundAltHighBrush");
    let accent = use_resource_as::<Rc<dyn IBrush>>("SystemControlHighlightAccentBrush");
    let accent_name = accent
        .as_ref()
        .and_then(|brush| brush.as_solid_color_brush().map(|solid| solid.color().to_string()))
        .unwrap_or_else(|| String::from("none"));
    let wide = use_signal(|| false);
    let bar_transitions = use_hook(|| transitions([transition(Layoutable::width_property(), ms(400))]));
    let source = use_hook(languages);
    let is_dark = variant.as_ref() == Some(&ThemeVariant::dark());

    rsx! {
        Border { background: background, padding: 16.0,
            StackPanel { spacing: 12.0,
                styles: [
                    style("Button.accent")
                        .set(TemplatedControl::background_property(), "#0063B1")
                        .set(TemplatedControl::foreground_property(), "White")
                        .nested(style("^:pointerover").set(Visual::opacity_property(), 0.85)),
                ],
                TextBlock { font_size: 20.0,
                    text: format!("Theme variant: {} (use_theme_variant), accent: {accent_name} (use_resource_as)",
                        if is_dark { "Dark" } else { "Light" }) }
                StackPanel { orientation: Orientation::Horizontal, spacing: 8.0,
                    Button { name: "Theme", class: "accent", on_click: move |_| dark.update(|dark| *dark = !*dark),
                        {if is_dark { "Switch to light" } else { "Switch to dark" }} }
                    Button { name: "Grow", on_click: move |_| wide.update(|wide| *wide = !*wide),
                        context_menu: rsx! { ContextMenu { MenuItem { header: "Copy" } MenuItem { header: "Paste" } } },
                        "Animate the bar (right-click: a context menu)" }
                }
                Border { name: "Bar", height: 12.0, corner_radius: 6.0, background: "#0063B1",
                    horizontal_alignment: ferroui_base::layout::HorizontalAlignment::Left,
                    width: if wide.get() { 480.0 } else { 120.0 }, transitions: bar_transitions }
                Grid { column_definitions: "Auto,*", height: 330.0,
                    ColorView { width: 300.0, vertical_alignment: VerticalAlignment::Top }
                    TreeDataGrid { Grid.column: 1, margin: (16.0, 0.0, 0.0, 0.0), source: source }
                }
            }
        }
    }
}
