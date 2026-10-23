//! The rest of the API (phase 3): styles, the resources and the theme
//! variant, transitions and animations, popups and context menus.

use crate::generated::builders::*;
use crate::reactive::Owner;
use crate::{
    animation, ms, rerender, style, transition, transitions, use_resource, use_theme_variant, IntoElement, StyleBuilder,
};
use ferroui_base::layout::ILayoutManager;
use ferroui_base::media::text_formatting::testing::TextTestScope;
use ferroui_base::platform::IPlatformRenderInterface;
use ferroui_base::styling::ThemeVariant;
use ferroui_base::{BoxedValue, FerroLocator, LocatorExtensions, Ref, Visual};
use ferroui_controls::primitives::Popup;
use ferroui_controls::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};
use ferroui_controls::{Button, Control, TextBlock, Window};
use std::rc::Rc;

struct App {
    _app: UnitTestApplicationScope,
    _text: TextTestScope,
}

fn app() -> App {
    let text = TextTestScope::new();
    let render_interface =
        FerroLocator::current().get_service::<dyn IPlatformRenderInterface>().expect("the text services have a render interface");
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

fn text_of(control: &Ref<Control>) -> Option<String> {
    control.get_visual_descendants().find_map(|visual| visual.cast::<TextBlock>()).and_then(|text| text.text())
}

#[test]
fn styles_of_an_element_apply_to_the_controls_below_it() {
    let _app = app();
    let accent: StyleBuilder = style("TextBlock.accent")
        .set(TextBlock::font_size_property(), 30.0)
        .nested(style("^.strong").set(TextBlock::font_size_property(), 40.0))
        .nested(style("^:not(.strong).dim").set(TextBlock::font_size_property(), 10.0));
    let owner = Owner::new_root();
    let panel = owner.with(|| {
        stack_panel()
            .styles([accent])
            .child(text_block().name("a").class("accent").text("accent"))
            .child(text_block().name("b").class("accent strong").text("strong"))
            .child(text_block().name("c").text("plain"))
            .into_element()
            .mount()
    });
    let window = window_with(&panel);
    let find = |name: &str| {
        panel.get_visual_descendants().filter_map(|visual| visual.cast::<TextBlock>()).find(|text| text.name().as_deref() == Some(name)).unwrap()
    };
    assert_eq!(find("a").font_size(), 30.0);
    assert_eq!(find("b").font_size(), 40.0, "a nested style applies with its parent's selector");
    assert_ne!(find("c").font_size(), 30.0);
    window.close();
    owner.dispose();
}

#[test]
#[should_panic(expected = "does not parse")]
fn a_selector_that_does_not_parse_says_so() {
    let _ = style("Button..");
}

#[test]
fn a_component_follows_the_theme_variant_and_a_resource() {
    let _app = app();
    let owner = Owner::new_root();
    let panel = owner.with(|| {
        stack_panel()
            .child(rerender(|| {
                let variant = match use_theme_variant() {
                    Some(variant) if variant == ThemeVariant::dark() => "dark",
                    Some(_) => "other",
                    None => "none",
                };
                let greeting = use_resource("Greeting")
                    .and_then(|value| value.downcast_ref::<String>().cloned())
                    .unwrap_or_else(|| String::from("no greeting"));
                text_block().text(format!("{variant}: {greeting}"))
            }))
            .into_element()
            .mount()
    });
    let window = window_with(&panel);
    window.resources().set("Greeting", Some(Rc::new(String::from("hello")) as BoxedValue));
    window.set_value(ThemeVariant::requested_theme_variant_property(), Some(ThemeVariant::dark()));
    assert_eq!(text_of(&panel).as_deref(), Some("dark: hello"));
    window.resources().set("Greeting", Some(Rc::new(String::from("hi")) as BoxedValue));
    assert_eq!(text_of(&panel).as_deref(), Some("dark: hi"), "the component follows a changed resource");
    window.close();
    owner.dispose();
}

#[test]
fn transitions_and_animations_are_built_for_the_animation_system() {
    let owner = Owner::new_root();
    let control = owner.with(|| {
        border()
            .transitions(transitions([
                transition(Visual::opacity_property(), ms(200)),
                transition(ferroui_controls::Border::background_property(), ms(300)),
            ]))
            .into_element()
            .mount()
    });
    let transitions = control.transitions().expect("transitions");
    assert_eq!(transitions.count(), 2);

    let built = animation(ms(600))
        .key_frame(0.0, |frame| frame.set(Visual::opacity_property(), 0.0))
        .key_frame(1.0, |frame| frame.set(Visual::opacity_property(), 1.0))
        .build();
    assert_eq!(built.children().count(), 2);
    assert_eq!(built.duration(), ferroui_base::animation::TimeSpan::from(ms(600)));
    owner.dispose();
}

#[test]
fn a_popup_holds_its_child_element_and_a_context_menu_is_an_element() {
    let owner = Owner::new_root();
    let panel = owner.with(|| {
        stack_panel()
            .child(popup().child(text_block().text("in the popup")))
            .child(button().content("menu").context_menu(context_menu().child(menu_item().header("Copy")).child(menu_item().header("Paste"))))
            .into_element()
            .mount()
    });
    let children: Vec<Ref<Control>> = panel.cast::<ferroui_controls::StackPanel>().unwrap().children().to_vec().into_iter().collect();
    let popup = children[0].cast::<Popup>().expect("a popup");
    let child = popup.child().expect("the child of the popup");
    assert_eq!(child.cast::<TextBlock>().and_then(|text| text.text()).as_deref(), Some("in the popup"));
    let button = children[1].cast::<Button>().expect("a button");
    let menu = button.context_menu().expect("the context menu");
    assert_eq!(menu.items().count(), 2);
    owner.dispose();
}

#[cfg(feature = "treedatagrid")]
#[test]
fn the_builders_of_other_crates_make_their_controls() {
    let owner = Owner::new_root();
    let control = owner.with(|| crate::generated::treedatagrid::tree_data_grid().into_element().mount());
    assert!(control.cast::<ferroui_controls_treedatagrid::TreeDataGrid>().is_some());
    owner.dispose();
}
