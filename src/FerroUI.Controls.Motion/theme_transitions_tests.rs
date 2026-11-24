//! Tests of the theme transitions, in a window rendered by the test compositor. The frame after a
//! start shows the first key frame of an animation; the later ones follow the clock.

use crate::{
    AddDeleteThemeTransition, ContentThemeTransition, EntranceThemeTransition, PopupThemeTransition,
    RepositionThemeTransition, ThemeTransition, ThemeTransitions, TransitionCollection,
};
use ferroui_base::media::Brushes;
use ferroui_base::rendering::composition::server::ServerCompositionVisual;
use ferroui_base::rendering::composition::{CompositionVisual, ElementComposition};
use ferroui_base::{BoxedValue, Ref, ScalarTransition, Visual};
use ferroui_controls::presenters::ItemsPresenter;
use ferroui_controls::templates::{FuncControlTemplate, FuncTemplateNameScopeExtensions};
use ferroui_controls::testing::{CompositorTestServices, MockWindowImpl, TestServices, UnitTestApplication};
use ferroui_controls::{Border, Canvas, Control, ItemsControl, StackPanel, Window};
use std::rc::Rc;
use std::time::{Duration, Instant};

/// A window with `content`, shown but not rendered yet.
fn open(services: &CompositorTestServices, content: Ref<Control>) -> Ref<Window> {
    let platform = MockWindowImpl::window(400.0, 300.0);
    services.setup(&platform);
    let window = Window::with_impl(platform);
    window.set_content(Some(Control::boxed(&content)));
    window.show();
    window
}

fn show(services: &CompositorTestServices, content: Ref<Control>) -> Ref<Window> {
    let window = open(services, content);
    services.run_jobs();
    services.run_jobs();
    window
}

fn block() -> Ref<Border> {
    let border = Border::new();
    border.set_width(50.0);
    border.set_height(40.0);
    border.set_background(Some(Brushes::blue()));
    border
}

fn collection(item: Ref<ThemeTransition>) -> Option<TransitionCollection> {
    Some(TransitionCollection::from_items([item]))
}

fn element_visual(visual: &Visual) -> Rc<CompositionVisual> {
    ElementComposition::get_element_visual(visual).expect("the element has a composition visual")
}

fn server(visual: &Rc<CompositionVisual>) -> Rc<ServerCompositionVisual> {
    visual.compositor().server().get::<ServerCompositionVisual>(visual.server()).expect("the server visual")
}

/// The opacity and the vertical translation on the render thread after one more frame.
fn first_frame(services: &CompositorTestServices, visual: &Visual) -> (f32, f64) {
    services.run_jobs();
    let server = server(&element_visual(visual));
    (server.opacity(), server.translation().y)
}

#[test]
fn the_defaults_are_those_of_winui() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let entrance = EntranceThemeTransition::new();
    assert_eq!((entrance.from_horizontal_offset(), entrance.from_vertical_offset()), (0.0, 28.0));
    assert!(!entrance.is_staggering_enabled());
    assert!(RepositionThemeTransition::new().is_staggering_enabled());
    let content = ContentThemeTransition::new();
    assert_eq!((content.horizontal_offset(), content.vertical_offset()), (0.0, 28.0));
    let popup = PopupThemeTransition::new();
    assert_eq!((popup.from_horizontal_offset(), popup.from_vertical_offset()), (0.0, 40.0));
}

#[test]
fn the_children_of_a_panel_enter_from_the_offset_when_it_loads() {
    let services = CompositorTestServices::start(TestServices::styled_window());
    let panel = StackPanel::new();
    let child = block();
    panel.children().add(child.clone());
    ThemeTransitions::set_children_transitions(&panel, collection(EntranceThemeTransition::new().upcast()));
    let _window = open(&services, panel.upcast());
    assert_eq!(first_frame(&services, &child), (0.0, 28.0));
}

#[test]
fn a_child_added_later_enters_and_the_others_stay() {
    let services = CompositorTestServices::start(TestServices::styled_window());
    let panel = StackPanel::new();
    let first = block();
    panel.children().add(first.clone());
    let _window = show(&services, panel.clone().upcast());
    // Its composition visual exists on the render thread before the collection is set.
    element_visual(&first);
    services.run_jobs();
    ThemeTransitions::set_children_transitions(&panel, collection(EntranceThemeTransition::new().upcast()));
    services.run_jobs();

    let added = block();
    panel.children().add(added.clone());
    assert_eq!(first_frame(&services, &added), (0.0, 28.0));
    assert_eq!(first_frame(&services, &first), (1.0, 0.0), "setting the collection plays nothing");
}

#[test]
fn a_changed_offset_reaches_the_next_entrance() {
    let services = CompositorTestServices::start(TestServices::styled_window());
    let panel = StackPanel::new();
    let entrance = EntranceThemeTransition::new();
    ThemeTransitions::set_children_transitions(&panel, collection(entrance.clone().upcast()));
    let _window = show(&services, panel.clone().upcast());
    entrance.set_from_vertical_offset(60.0);
    let added = block();
    panel.children().add(added.clone());
    assert_eq!(first_frame(&services, &added), (0.0, 60.0));
}

#[test]
fn a_moved_element_slides_and_its_first_placement_does_not() {
    let services = CompositorTestServices::start(TestServices::styled_window());
    let canvas = Canvas::new();
    let element = block();
    Canvas::set_left(&element, 10.0);
    canvas.children().add(element.clone());
    ThemeTransitions::set_transitions(&element, collection(RepositionThemeTransition::new().upcast()));
    let _window = show(&services, canvas.upcast());
    services.run_jobs();
    let visual = element_visual(&element);
    assert_eq!(server(&visual).offset().x, 10.0, "the first placement is not animated");

    Canvas::set_left(&element, 110.0);
    services.run_jobs();
    assert_eq!(visual.offset().x, 110.0, "the value is the new place");
    assert_eq!(server(&visual).offset().x, 10.0, "the first frame is the old place");
    let started = Instant::now();
    while server(&visual).offset().x != 110.0 && started.elapsed() < Duration::from_secs(3) {
        services.run_jobs();
    }
    assert_eq!(server(&visual).offset().x, 110.0, "it arrives");
}

#[test]
fn reposition_keeps_the_implicit_transitions_of_the_element() {
    let services = CompositorTestServices::start(TestServices::styled_window());
    let canvas = Canvas::new();
    let element = block();
    element.set_opacity_transition(Some(ScalarTransition::with_duration(Duration::from_millis(100))));
    canvas.children().add(element.clone());
    ThemeTransitions::set_transitions(&element, collection(RepositionThemeTransition::new().upcast()));
    let _window = show(&services, canvas.upcast());
    Canvas::set_left(&element, 60.0);
    services.run_jobs();
    services.run_jobs();
    let animations = element_visual(&element).implicit_animations().expect("the implicit animations");
    assert!(animations.contains_key("Opacity") && animations.contains_key("Offset"));
}

#[test]
fn clearing_the_collection_ends_the_motion() {
    let services = CompositorTestServices::start(TestServices::styled_window());
    let canvas = Canvas::new();
    let element = block();
    canvas.children().add(element.clone());
    ThemeTransitions::set_transitions(&element, collection(RepositionThemeTransition::new().upcast()));
    let _window = show(&services, canvas.upcast());
    Canvas::set_left(&element, 60.0);
    services.run_jobs();
    ThemeTransitions::set_transitions(&element, None);
    Canvas::set_left(&element, 90.0);
    services.run_jobs();
    services.run_jobs();
    assert_eq!(server(&element_visual(&element)).offset().x, 90.0);
}

#[test]
fn add_delete_gives_a_removed_child_its_hide_animation() {
    let services = CompositorTestServices::start(TestServices::styled_window());
    let panel = StackPanel::new();
    let child = block();
    panel.children().add(child.clone());
    ThemeTransitions::set_children_transitions(&panel, collection(AddDeleteThemeTransition::new().upcast()));
    let _window = show(&services, panel.clone().upcast());
    assert!(ElementComposition::get_implicit_hide_animation(&child).is_some());
    let panel_visual = element_visual(&panel);
    panel.children().remove(&child);
    services.run_jobs();
    assert_eq!(panel_visual.children().count(), 1, "the image of the child stays while it fades");
}

#[test]
fn an_item_added_to_an_items_control_enters() {
    let services = CompositorTestServices::start(TestServices::styled_window());
    let items = ItemsControl::new();
    items.set_template(Some(FuncControlTemplate::for_type::<ItemsControl>(|_, scope| {
        let presenter = ItemsPresenter::new();
        presenter.set_name(Some("PART_ItemsPresenter".to_string()));
        presenter.register_in_name_scope(&**scope).upcast()
    })));
    items.items().add(Some(Control::boxed(&block().upcast::<Control>()) as BoxedValue));
    ThemeTransitions::set_item_container_transitions(&items, collection(ContentThemeTransition::new().upcast()));
    let _window = show(&services, items.clone().upcast());
    services.run_jobs();
    services.run_jobs();

    let added = block();
    items.items().add(Some(Control::boxed(&added.clone().upcast::<Control>()) as BoxedValue));
    assert_eq!(first_frame(&services, &added), (0.0, 28.0));
}
