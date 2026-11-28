//! A gallery of phase 3 of the declarative API (`docs/declarative-ui.md`, section 10): a
//! component that runs again, under a theme variant it switches, with
//!
//! - styles of an element (`styles: [style(of::<Button>().class("accent"))...]`) with a nested style;
//! - the theme variant and typed resources of the theme ([`use_theme_variant`], `keys`,
//!   `dynamic_resource`, [`use_theme_resource`]);
//! - a transition of a width ([`transition`]);
//! - a context menu given as an element;
//! - a templated control whose whole theme is written in Rust (`chip_theme`);
//! - a view model bound with typed property paths (`#[derive(ViewModel)]`);
//! - a colour view of `ferroui-controls-color-picker` and a TreeDataGrid of
//!   `ferroui-controls-treedatagrid`, written with their generated builders.
//!
//! `cargo run -p view-gallery`.

use ferroui_base::data::model::Event;
use ferroui_base::layout::{Layoutable, Orientation, VerticalAlignment};
use ferroui_base::styling::{ControlTheme, ThemeVariant};
use ferroui_base::{BoxedValue, Ref, Visual};
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::primitives::{TemplatedControl, ToggleButton};
use ferroui_controls::{ContentControl, GridLength};
use ferroui_controls_treedatagrid::models::TextColumn;
use ferroui_controls_treedatagrid::TreeDataGridSourceRef;
use ferroui_treedatagrid_core::models::ColumnRef;
use ferroui_treedatagrid_core::{Accessor, FlatTreeDataGridSource, Models};
use ferroui_themes_fluent::keys;
use ferroui_view::prelude::*;
use std::cell::RefCell;
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

/// A view model: state that lives outside of the component and notifies its changes. One
/// declaration gives the accessors, the names of the changes and the typed properties.
#[derive(ViewModel)]
#[computed(text: String, depends_on(name))]
pub struct Greeting {
    #[property]
    name: RefCell<String>,
    property_changed: Event<str>,
}

impl Greeting {
    fn new() -> Rc<Self> {
        Rc::new(Self { name: RefCell::new(String::from("FerroUI")), property_changed: Event::default() })
    }

    fn text(&self) -> String {
        format!("Hello, {}", self.name.borrow())
    }
}

/// A toggle button drawn as a chip: its whole control theme is Rust, with its template, its
/// named parts and the styles of its states, and it follows the resources of the theme variant.
fn chip_theme() -> Ref<ControlTheme> {
    control_theme::<ToggleButton>()
        .set(TemplatedControl::padding_property(), (12.0, 4.0))
        .set(TemplatedControl::corner_radius_property(), 12.0)
        .set(TemplatedControl::border_thickness_property(), 1.0)
        .dynamic_resource(TemplatedControl::background_property(), keys::SystemControlBackgroundAltHighBrush)
        .dynamic_resource(TemplatedControl::border_brush_property(), keys::SystemControlHighlightAccentBrush)
        .template(|| {
            border()
                .name("PART_Border")
                .template_bind(ferroui_controls::Border::background_property(), TemplatedControl::background_property())
                .template_bind(ferroui_controls::Border::border_brush_property(), TemplatedControl::border_brush_property())
                .template_bind(
                    ferroui_controls::Border::border_thickness_property(),
                    TemplatedControl::border_thickness_property(),
                )
                .template_bind(ferroui_controls::Border::corner_radius_property(), TemplatedControl::corner_radius_property())
                .child(
                    content_presenter()
                        .name("PART_ContentPresenter")
                        .template_bind(ContentPresenter::content_property(), ContentControl::content_property())
                        .template_bind(ContentPresenter::padding_property(), TemplatedControl::padding_property()),
                )
                .into_element()
        })
        .nested(
            style(nesting().pointer_over().template().of::<ferroui_controls::Border>())
                .set(Visual::opacity_property(), 0.85),
        )
        .nested(
            style(nesting().checked().template().of::<ferroui_controls::Border>())
                .dynamic_resource(ferroui_controls::Border::background_property(), keys::SystemControlHighlightAccentBrush),
        )
        .build()
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
    let accent = use_theme_resource(keys::SystemControlHighlightAccentBrush);
    let accent_name = accent
        .as_ref()
        .and_then(|brush| brush.as_solid_color_brush().map(|solid| solid.color().to_string()))
        .unwrap_or_else(|| String::from("none"));
    let wide = use_signal(|| false);
    let bar_transitions = use_hook(|| transitions([transition(Layoutable::width_property(), ms(400))]));
    let source = use_hook(languages);
    let greeting = use_hook(Greeting::new);
    let chip = use_hook(chip_theme);
    let is_dark = variant.as_ref() == Some(&ThemeVariant::dark());

    rsx! {
        Border { dynamic_resource(ferroui_controls::Border::background_property(), keys::SystemControlBackgroundAltHighBrush),
            padding: 16.0,
            StackPanel { spacing: 12.0,
                styles: [
                    style(of::<Button>().class("accent"))
                        .set(TemplatedControl::background_property(), rgb(0x0063B1))
                        .set(TemplatedControl::foreground_property(), "White")
                        .nested(style(nesting().pointer_over()).set(Visual::opacity_property(), 0.85)),
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
                StackPanel { orientation: Orientation::Horizontal, spacing: 8.0,
                    ToggleButton { name: "ChipRust", theme: chip.clone(), "A theme in Rust" }
                    ToggleButton { name: "ChipOther", theme: chip.clone(), is_checked: true, "Checked" }
                }
                StackPanel { orientation: Orientation::Horizontal, spacing: 8.0,
                    data_context: greeting.clone() as BoxedValue,
                    TextBox { name: "GreetingName", width: 180.0, text: Greeting::NAME.two_way() }
                    TextBlock { vertical_alignment: VerticalAlignment::Center, text: Greeting::TEXT.one_way() }
                }
                Border { name: "Bar", height: 12.0, corner_radius: 6.0, background: rgb(0x0063B1),
                    horizontal_alignment: ferroui_base::layout::HorizontalAlignment::Left,
                    width: if wide.get() { 480.0 } else { 120.0 }, transitions: bar_transitions }
                Grid { column_definitions: [auto(), star(1.0)], height: 330.0,
                    ColorView { width: 300.0, vertical_alignment: VerticalAlignment::Top }
                    TreeDataGrid { Grid.column: 1, margin: (16.0, 0.0, 0.0, 0.0), source: source }
                }
            }
        }
    }
}
