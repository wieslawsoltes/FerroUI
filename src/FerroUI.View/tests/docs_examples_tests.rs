//! The Rust examples of the guides (`docs/declarative-ui.md` and the `next` pages of the site),
//! compiled here so that they follow the API. A `keys::` constant of a theme is a local constant
//! of the same type: this crate does not depend on a theme, and `samples/ViewGallery` compiles the
//! same forms with the keys of the Fluent theme.

use crate::prelude::*;
use ferroui_base::data::model::Event;
use ferroui_base::input::InputElement;
use ferroui_base::media::{Colors, IBrush};
use ferroui_base::styling::{ControlTheme, ThemeVariant};
use ferroui_base::{BoxedValue, CornerRadius, Ref, Thickness, Visual};
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::primitives::{TemplatedControl, ToggleButton};
use ferroui_controls::{Border, Button, ContentControl, StackPanel, TextBlock};
use std::cell::RefCell;
use std::rc::Rc;

/// Stand-ins for the constants of `ferroui_themes_fluent::keys`.
#[allow(non_upper_case_globals)]
mod keys {
    use super::*;
    pub const SystemControlBackgroundAltHighBrush: ThemeResourceKey<Rc<dyn IBrush>> =
        ThemeResourceKey::new("SystemControlBackgroundAltHighBrush");
    pub const SystemControlHighlightAccentBrush: ThemeResourceKey<Rc<dyn IBrush>> =
        ThemeResourceKey::new("SystemControlHighlightAccentBrush");
    pub const ControlCornerRadius: ThemeResourceKey<CornerRadius> = ThemeResourceKey::new("ControlCornerRadius");
}

#[test]
fn the_selector_table() {
    let _ = [
        of::<Button>(),
        is_type::<ferroui_controls::Control>(),
        any().class("accent"),
        any().name("title"),
        any().pressed(),
        of::<Button>().class("accent").focus(),
        of::<StackPanel>().child().of::<TextBlock>(),
        of::<StackPanel>().descendant().of::<TextBlock>(),
        of::<Button>().template().of::<ContentPresenter>(),
        of::<Button>().not(any().class("accent")),
        any().nth_child(2, 1),
        of::<Button>().property_equals(InputElement::is_enabled_property(), false),
        any_of([of::<Button>(), of::<TextBlock>()]),
        nesting().pointer_over(),
    ];
}

#[test]
fn the_styling_example() {
    let _: Element = rsx! {
        StackPanel { spacing: 12.0,
            styles: [
                style(of::<Button>().class("accent"))
                    .set(TemplatedControl::background_property(), rgb(0x0063B1))
                    .set(TemplatedControl::foreground_property(), "White")
                    .nested(style(nesting().pointer_over()).set(Visual::opacity_property(), 0.85)),
            ],
            Button { class: "accent", "Save" }
            Button { "Cancel" }
        }
    };
}

#[test]
fn the_resource_examples() {
    let _card = border()
        .dynamic_resource(Border::background_property(), keys::SystemControlBackgroundAltHighBrush)
        .static_resource(Border::corner_radius_property(), keys::ControlCornerRadius);
    let _resources = resource_dictionary()
        .value("CardPadding", Thickness::uniform(12.0))
        .theme_variant(ThemeVariant::light(), resource_dictionary().value("Accent", rgb(0x0063B1)))
        .theme_variant(ThemeVariant::dark(), resource_dictionary().value("Accent", rgb(0x60CDFF)))
        .build();
}

#[test]
fn the_typed_values_example() {
    let _card = border()
        .background(linear_gradient(&[(0.0, rgb(0x0063B1)), (1.0, rgb(0x60CDFF))]).horizontal())
        .box_shadow(shadow(0.0, 2.0, 4.0, argb(0x40000000)))
        .render_transform(translate(10.0, 5.0).rotate(45.0))
        .effect(blur(2.0));
    let _save = button().hot_key(ctrl(KeyCode::S)).content("Save");
    let _ = path_data().move_to(0.0, 0.0).line_to(10.0, 0.0).close();
}

/// Compiled and not run: a geometry needs the render interface of an application.
#[allow(dead_code)]
fn the_icon_of_the_typed_values_example() -> Element {
    path().data(path!("M8 3v5H3v2h5v5h2V10h5V8h-5V3z")).fill(Colors::BLACK).into()
}

#[test]
fn the_rsx_example() {
    let _: Element = rsx! {
        Grid { row_definitions: [auto(), star(1.0)],
            styles: [style(of::<Button>().class("accent")).set(Visual::opacity_property(), 0.9)],
            Border { dynamic_resource(Border::background_property(), keys::SystemControlBackgroundAltHighBrush),
                box_shadow: shadow(0.0, 2.0, 4.0, argb(0x40000000)),
                Button { class: "accent", hot_key: ctrl(KeyCode::S), "Save" }
            }
        }
    };
}

#[allow(dead_code)]
fn chip_theme() -> Ref<ControlTheme> {
    control_theme::<ToggleButton>()
        .set(TemplatedControl::padding_property(), (12.0, 4.0))
        .dynamic_resource(TemplatedControl::background_property(), keys::SystemControlBackgroundAltHighBrush)
        .template(|| {
            border()
                .name("PART_Border")
                .template_bind(Border::background_property(), TemplatedControl::background_property())
                .child(
                    content_presenter()
                        .name("PART_ContentPresenter")
                        .template_bind(ContentPresenter::content_property(), ContentControl::content_property())
                        .template_bind(ContentPresenter::padding_property(), TemplatedControl::padding_property()),
                )
                .into_element()
        })
        .nested(
            style(nesting().checked().template().of::<Border>())
                .dynamic_resource(Border::background_property(), keys::SystemControlHighlightAccentBrush),
        )
        .build()
}

#[derive(ViewModel)]
#[computed(greeting: String, depends_on(name))]
struct Person {
    #[property]
    name: RefCell<String>,
    property_changed: Event<str>,
}

impl Person {
    fn greeting(&self) -> String {
        format!("Hello, {}", self.name())
    }
}

#[allow(dead_code)]
fn form(person: Rc<Person>) -> Element {
    stack_panel()
        .data_context(person as BoxedValue)
        .child(text_box().text(Person::NAME.two_way()))
        .child(text_block().text(Person::GREETING.one_way()))
        .into()
}

#[derive(Route)]
#[route("/people/:id")]
struct PersonRoute {
    id: u32,
}

#[allow(dead_code)]
fn routes() -> Vec<RouteDef> {
    vec![
        route("/", |_| text_block().text("Home").into()),
        typed_route::<PersonRoute>(|person| text_block().text(format!("Person {}", person.id)).into()),
        route("*", |m| text_block().text(format!("Nothing at {}", m.path())).into()),
    ]
}

#[component(rerender)]
#[allow(dead_code)]
fn Open() -> Element {
    let router = use_router();
    rsx! { Button { on_click: move |_| router.navigate_to(&PersonRoute { id: 42 }), "Open person 42" } }
}
