//! Resource references of the builders, with typed keys and text.

use crate::builder::IntoValue;
use crate::generated::builders::*;
use crate::reactive::{signal, Owner};
use crate::{rerender, rgb, use_theme_resource, IntoElement, ThemeResourceKey};
use ferroui_base::controls::ResourceDictionary;
use ferroui_base::layout::ILayoutManager;
use ferroui_base::media::text_formatting::testing::TextTestScope;
use ferroui_base::media::{Color, Colors, IBrush};
use ferroui_base::platform::IPlatformRenderInterface;
use ferroui_base::styling::ThemeVariant;
use ferroui_base::{BoxedValue, FerroLocator, LocatorExtensions, Ref, Thickness};
use ferroui_controls::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};
use ferroui_controls::{Border, Control, Decorator, TextBlock, Window};
use std::cell::Cell;
use std::rc::Rc;

const BACKGROUND: ThemeResourceKey<Rc<dyn IBrush>> = ThemeResourceKey::new("Background");
const ACCENT: ThemeResourceKey<Color> = ThemeResourceKey::new("Accent");
const PADDING: ThemeResourceKey<Thickness> = ThemeResourceKey::new("Padding");

struct App {
    _app: UnitTestApplicationScope,
    _text: TextTestScope,
}

fn app() -> App {
    let text = TextTestScope::new();
    let render_interface = FerroLocator::current()
        .get_service::<dyn IPlatformRenderInterface>()
        .expect("the text services have a render interface");
    let app = UnitTestApplication::start(TestServices::styled_window().with_render_interface(render_interface));
    App { _app: app, _text: text }
}

fn window_with(content: &Ref<Control>) -> Ref<Window> {
    let window = Window::new();
    window.set_width(300.0);
    window.set_height(200.0);
    window.set_content(Some(Control::boxed(content)));
    window.show();
    window.layout_manager().execute_layout_pass();
    window
}

fn brush(color: Color) -> Option<BoxedValue> {
    let brush: Option<Rc<dyn IBrush>> = color.into_value();
    brush.map(|brush| Rc::new(brush) as BoxedValue)
}

/// A dictionary with a brush and a color for the light and the dark variant.
fn themed() -> Ref<ResourceDictionary> {
    let dictionary = ResourceDictionary::new();
    for (variant, background, accent) in [
        (ThemeVariant::light(), Colors::WHITE, rgb(0x101010)),
        (ThemeVariant::dark(), Colors::BLACK, rgb(0xF0F0F0)),
    ] {
        let theme = ResourceDictionary::new();
        theme.set("Background", brush(background));
        theme.set("Accent", Some(Rc::new(accent) as BoxedValue));
        dictionary.add_theme_dictionary(variant, theme);
    }
    dictionary
}

fn color_of(brush: Option<Rc<dyn IBrush>>) -> Option<Color> {
    brush?.as_solid_color_brush().map(|solid| solid.color())
}

#[test]
fn a_dynamic_resource_follows_the_theme_variant_with_no_run() {
    let _app = app();
    let runs = Rc::new(Cell::new(0));
    let counted = runs.clone();
    let owner = Owner::new_root();
    let root = owner.with(|| {
        rerender(move || {
            counted.set(counted.get() + 1);
            border()
                .dynamic_resource(Border::background_property(), BACKGROUND)
                .dynamic_resource(Border::border_brush_property(), ACCENT)
        })
        .into_element()
        .mount()
    });
    let window = window_with(&root);
    window.set_resources(themed());
    window.set_value(ThemeVariant::requested_theme_variant_property(), Some(ThemeVariant::light()));
    let target = root.cast::<Border>().unwrap();
    assert_eq!(color_of(target.background()), Some(Colors::WHITE));
    assert_eq!(color_of(target.border_brush()), Some(rgb(0x101010)), "a color resource on a brush property");

    let before = runs.get();
    window.set_value(ThemeVariant::requested_theme_variant_property(), Some(ThemeVariant::dark()));
    assert_eq!(color_of(target.background()), Some(Colors::BLACK));
    assert_eq!(color_of(target.border_brush()), Some(rgb(0xF0F0F0)));
    assert_eq!(runs.get(), before, "the property follows the resource and no component runs");
    window.close();
    owner.dispose();
}

#[test]
fn a_dynamic_resource_follows_a_change_and_a_removal() {
    let _app = app();
    let owner = Owner::new_root();
    let root = owner.with(|| border().dynamic_resource(Decorator::padding_property(), PADDING).into_element().mount());
    let window = window_with(&root);
    let target = root.cast::<Border>().unwrap();
    assert_eq!(target.padding(), Thickness::default(), "no resource yet");
    window.resources().set("Padding", Some(Rc::new(Thickness::uniform(4.0)) as BoxedValue));
    assert_eq!(target.padding(), Thickness::uniform(4.0));
    window.resources().set("Padding", Some(Rc::new(Thickness::uniform(8.0)) as BoxedValue));
    assert_eq!(target.padding(), Thickness::uniform(8.0));
    window.resources().remove(&"Padding".into());
    assert_eq!(target.padding(), Thickness::default(), "a resource that is gone clears the value");
    window.close();
    owner.dispose();
}

#[test]
fn a_static_resource_takes_the_first_value_only() {
    let _app = app();
    let owner = Owner::new_root();
    let root = owner.with(|| border().static_resource(Decorator::padding_property(), PADDING).into_element().mount());
    let window = window_with(&root);
    let target = root.cast::<Border>().unwrap();
    window.resources().set("Padding", Some(Rc::new(Thickness::uniform(4.0)) as BoxedValue));
    assert_eq!(target.padding(), Thickness::uniform(4.0));
    window.resources().set("Padding", Some(Rc::new(Thickness::uniform(8.0)) as BoxedValue));
    assert_eq!(target.padding(), Thickness::uniform(4.0), "a static resource does not follow");
    window.close();
    owner.dispose();
}

#[test]
fn a_text_key_works_like_a_typed_one() {
    let _app = app();
    let owner = Owner::new_root();
    let root = owner.with(|| {
        stack_panel()
            .child(border().dynamic_resource(Border::background_property(), "Background"))
            .child(border().dynamic_resource(Border::border_brush_property(), "Accent"))
            .child(text_block().dynamic_resource(TextBlock::text_property(), "Title"))
            .into_element()
            .mount()
    });
    let window = window_with(&root);
    window.set_resources(themed());
    window.resources().set("Title", Some(Rc::new(String::from("hello")) as BoxedValue));
    window.set_value(ThemeVariant::requested_theme_variant_property(), Some(ThemeVariant::dark()));
    let mut borders = root.get_visual_descendants().filter_map(|visual| visual.cast::<Border>());
    let (first, second) = (borders.next().unwrap(), borders.next().unwrap());
    assert_eq!(color_of(first.background()), Some(Colors::BLACK));
    assert_eq!(color_of(second.border_brush()), Some(rgb(0xF0F0F0)));
    let text = root.get_visual_descendants().find_map(|visual| visual.cast::<TextBlock>()).unwrap();
    assert_eq!(text.text().as_deref(), Some("hello"));
    window.close();
    owner.dispose();
}

#[test]
fn a_component_that_runs_again_keeps_its_reference() {
    let _app = app();
    let owner = Owner::new_root();
    let (tick, root) = owner.with(|| {
        let tick = signal(0);
        let root = rerender(move || {
            border().width(f64::from(tick.get()) + 1.0).dynamic_resource(Decorator::padding_property(), PADDING)
        })
        .into_element()
        .mount();
        (tick, root)
    });
    let window = window_with(&root);
    let target = root.cast::<Border>().unwrap();
    window.resources().set("Padding", Some(Rc::new(Thickness::uniform(4.0)) as BoxedValue));
    tick.set(1);
    assert_eq!(target.width(), 2.0);
    window.resources().set("Padding", Some(Rc::new(Thickness::uniform(6.0)) as BoxedValue));
    assert_eq!(target.padding(), Thickness::uniform(6.0), "the reference still follows after a run");
    window.close();
    owner.dispose();
}

#[test]
fn a_hook_reads_a_typed_key() {
    let _app = app();
    let owner = Owner::new_root();
    let root = owner.with(|| {
        stack_panel()
            .child(rerender(|| {
                let padding = use_theme_resource(PADDING).unwrap_or_default();
                text_block().text(format!("{}", padding.left))
            }))
            .into_element()
            .mount()
    });
    let window = window_with(&root);
    window.resources().set("Padding", Some(Rc::new(Thickness::uniform(3.0)) as BoxedValue));
    let text = root.get_visual_descendants().find_map(|visual| visual.cast::<TextBlock>()).unwrap();
    assert_eq!(text.text().as_deref(), Some("3"));
    window.close();
    owner.dispose();
}
