//! Tests of WinUI's `SemanticZoom` (#1271; docs/porting/winui-controls.md, section 10.20), after
//! `SemanticZoomIntegrationTests.cpp` of microsoft-ui-xaml where it does not need
//! DirectManipulation: the views and their owner, the change that waits for the template, the
//! jump list of a grouped grid and the list of its groups in both directions, the group headers,
//! the keyboard, the zoom-out button, the automation peer and the zoom on the compositor.

use super::semantic_zoom_transition::{ZOOM_DURATION, ZOOM_MIN};
use super::*;
use crate::automation::peers::{AutomationControlType, ControlAutomationPeer, SemanticZoomAutomationPeer};
use crate::automation::provider::{IToggleProvider, ToggleState};
use crate::compositor_hit_testing_tests::CompositorTestServices;
use crate::list_view::list_view_tests::list_view_template;
use crate::presenters::ContentPresenter;
use crate::templates::{FuncControlTemplate, FuncTemplateNameScopeExtensions, IControlTemplate};
use crate::test_support::{boxed_str, string_of, test_scope, TestRoot};
use crate::{
    Button, CollectionViewGroup, ContentControl, Control, GridView, GroupedItemsView, ItemsSource, ListView,
    ListViewBaseHeaderItem, Panel,
};
use ferroui_base::data::TemplateBinding;
use ferroui_base::input::{InputElement, Key, KeyEventArgs, KeyModifiers};
use ferroui_base::rendering::composition::server::ServerCompositionVisual;
use ferroui_base::rendering::composition::{CompositionVisual, ElementComposition, ICompositionObjectAnimations};
use ferroui_base::{BoxedValue, Ref, Size};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};

/// WinUI's template without the ScrollViewer of DirectManipulation: the two presenters and the
/// zoom-out button.
fn semantic_zoom_template() -> Option<Rc<dyn IControlTemplate>> {
    Some(FuncControlTemplate::new(|_, scope| {
        let root = Panel::new();
        for (name, property) in [
            ("ZoomedInPresenter", SemanticZoom::zoomed_in_view_property()),
            ("ZoomedOutPresenter", SemanticZoom::zoomed_out_view_property()),
        ] {
            let presenter = ContentPresenter::new();
            presenter.set_name(Some(name.to_owned()));
            presenter.bind_binding(
                ContentPresenter::content_property().as_property(),
                &TemplateBinding::new(property.as_property()),
            );
            root.children().add(presenter.register_in_name_scope(&**scope));
        }
        let button = Button::new();
        button.set_name(Some("ZoomOutButton".to_owned()));
        root.children().add(button.register_in_name_scope(&**scope));
        root.upcast()
    }))
}

/// Four groups of `count` items each: `G<g>-<i>`, under the headers `G<g>`.
fn groups(count: usize) -> Rc<GroupedItemsView> {
    GroupedItemsView::new(
        (0..4)
            .map(|group| {
                let items = (0..count).map(|index| boxed_str(&format!("G{group}-{index}")));
                CollectionViewGroup::new(boxed_str(&format!("G{group}")), ItemsSource::from_items(items))
            })
            .collect(),
    )
}

struct Fixture {
    zoom: Ref<SemanticZoom>,
    grid: Ref<GridView>,
    list: Ref<ListView>,
    view: Rc<GroupedItemsView>,
    _root: Ref<TestRoot>,
}

impl Fixture {
    /// The gallery's page: a grouped grid as the zoomed-in view, the list of its groups
    /// (`View.CollectionGroups`) as the zoomed-out view.
    fn new(templated: bool) -> Self {
        let view = groups(20);
        let grid = GridView::new();
        grid.set_template(list_view_template());
        grid.set_items_source(Some(ItemsSource::new(view.clone())));
        let list = ListView::new();
        list.set_template(list_view_template());
        let collection_groups = (0..view.group_count()).filter_map(|index| view.group_at(index));
        list.set_items_source(Some(ItemsSource::from_items(
            collection_groups.map(|group| Some(Rc::new(group) as BoxedValue)),
        )));
        let zoom = SemanticZoom::new();
        zoom.set_width(400.0);
        zoom.set_height(120.0);
        zoom.set_zoomed_in_view(Some(grid.clone().upcast()));
        zoom.set_zoomed_out_view(Some(list.clone().upcast()));
        if templated {
            zoom.set_template(semantic_zoom_template());
        }
        let root = TestRoot::with_child(zoom.clone());
        root.layout_manager().execute_initial_layout_pass();
        Self { zoom, grid, list, view, _root: root }
    }

    fn presenter(&self, name: &str) -> Ref<Control> {
        self.zoom
            .get_template_descendants()
            .into_iter()
            .find(|element| element.name().as_deref() == Some(name))
            .and_then(|element| element.cast::<Control>())
            .expect("a presenter of the template")
    }

    fn record(&self) -> Rc<RefCell<Vec<(&'static str, SemanticZoomViewChangedEventArgs)>>> {
        let events = Rc::new(RefCell::new(Vec::new()));
        let started = events.clone();
        self.zoom.view_change_started(move |_, args| started.borrow_mut().push(("started", args.clone()))).detach();
        let completed = events.clone();
        self.zoom
            .view_change_completed(move |_, args| completed.borrow_mut().push(("completed", args.clone())))
            .detach();
        events
    }
}

fn group_of(item: &Option<BoxedValue>) -> Option<Rc<CollectionViewGroup>> {
    item.as_ref()?.downcast_ref::<Rc<CollectionViewGroup>>().cloned()
}

fn text(item: &Option<BoxedValue>) -> String {
    item.as_ref().and_then(string_of).unwrap_or_default()
}

#[test]
fn the_defaults_and_the_views_know_their_semantic_zoom() {
    let _scope = test_scope();
    let zoom = SemanticZoom::new();
    assert!(zoom.is_zoomed_in_view_active());
    assert!(zoom.can_change_views());
    assert!(zoom.is_zoom_out_button_enabled());
    assert!(zoom.zoomed_in_view().is_none() && zoom.zoomed_out_view().is_none());

    let fixture = Fixture::new(true);
    let (grid, list) = (&fixture.grid, &fixture.list);
    assert!(grid.semantic_zoom_owner().as_ref() == Some(&fixture.zoom));
    assert!(list.semantic_zoom_owner().as_ref() == Some(&fixture.zoom));
    assert!(grid.is_zoomed_in_view() && grid.is_active_view());
    assert!(!list.is_zoomed_in_view() && !list.is_active_view());
    // The zoomed-out presenter waits transparent, without hits.
    let out = fixture.presenter("ZoomedOutPresenter");
    assert_eq!(out.opacity(), 0.0);
    assert!(!out.is_hit_test_visible());
    assert_eq!(fixture.presenter("ZoomedInPresenter").opacity(), 1.0);

    // A replaced view forgets the semantic zoom (InitializeSemanticZoomInformation).
    let other = ListView::new();
    fixture.zoom.set_zoomed_out_view(Some(other.clone().upcast()));
    assert!(list.semantic_zoom_owner().is_none());
    assert!(list.is_zoomed_in_view() && !list.is_active_view());
    assert!(other.semantic_zoom_owner().is_some() && !other.is_zoomed_in_view());
}

#[test]
fn a_change_before_the_template_waits_for_it() {
    let _scope = test_scope();
    let fixture = Fixture::new(false);
    let events = fixture.record();
    fixture.zoom.set_is_zoomed_in_view_active(false);
    fixture.zoom.set_is_zoomed_in_view_active(true);
    fixture.zoom.set_is_zoomed_in_view_active(false);
    assert!(events.borrow().is_empty(), "no change before the template");
    fixture.zoom.set_template(semantic_zoom_template());
    fixture.zoom.apply_template();
    let events = events.borrow();
    let names: Vec<_> = events.iter().map(|(name, _)| *name).collect();
    assert_eq!(names, ["started", "completed"]);
    assert!(events[0].1 == events[1].1, "both events share the arguments");
    assert!(events[0].1.is_source_zoomed_in_view());
    assert!(fixture.list.is_active_view() && !fixture.grid.is_active_view());
    assert_eq!(fixture.presenter("ZoomedOutPresenter").opacity(), 1.0);
    assert_eq!(fixture.presenter("ZoomedInPresenter").opacity(), 0.0);
    assert!(fixture.zoom.classes().contains(":zoomed-out") || fixture.zoom.pseudo_classes().contains(":zoomed-out"));
}

/// Zooming out of the grouped grid: the source is the first item of the first group (the jump
/// list), the destination its group in the list of groups; zooming in from a group of the list
/// the destination is the object of the group's header, and the grid scrolls to that group.
#[test]
fn the_jump_list_goes_between_the_items_and_their_groups() {
    let _scope = test_scope();
    let fixture = Fixture::new(true);
    let events = fixture.record();
    fixture.zoom.toggle_active_view();
    assert!(!fixture.zoom.is_zoomed_in_view_active());
    {
        let events = events.borrow();
        assert_eq!(events.len(), 2);
        let args = &events[0].1;
        assert!(args.is_source_zoomed_in_view());
        assert_eq!(text(&args.source_item().item()), "G0-0");
        let group = group_of(&args.destination_item().item()).expect("the group of the item");
        assert!(Rc::ptr_eq(&group, &fixture.view.group_at(0).unwrap()));
    }
    assert!(fixture.list.is_active_view());
    assert!(fixture.presenter("ZoomedOutPresenter").is_hit_test_visible());
    assert!(!fixture.presenter("ZoomedInPresenter").is_hit_test_visible());

    // An item of the zoomed-out list is invoked: the grid goes to the group G2.
    events.borrow_mut().clear();
    let third = fixture.list.container_from_index(2).expect("the container of the third group");
    assert!(fixture.list.zoomed_out_item_invoked(&third, false));
    assert!(fixture.zoom.is_zoomed_in_view_active());
    let events = events.borrow();
    let args = &events[0].1;
    assert!(!args.is_source_zoomed_in_view());
    let source = group_of(&args.source_item().item()).expect("the invoked group");
    assert!(Rc::ptr_eq(&source, &fixture.view.group_at(2).unwrap()));
    assert_eq!(text(&args.destination_item().item()), "G2");
    let remainder = args.destination_item().remainder();
    assert!(remainder.x.is_finite() && remainder.y.is_finite());
    // MakeVisible: the header of G2 at the start of the viewport.
    let viewer = fixture.grid.scroll_viewer().expect("the scroll viewer of the grid");
    assert!(viewer.extent().height > viewer.viewport().height, "{:?} {:?}", viewer.extent(), viewer.viewport());
    assert!(viewer.offset().y > 0.0, "the grid scrolled to the group: {:?}", viewer.offset());
}

/// A tap on a group header of the zoomed-in view shows the zoomed-out view at that group, at
/// the bottom of the list (WinUI's `m_tpSZRequestingItem` and `SetIsBottomAlignment`).
#[test]
fn a_tap_on_a_group_header_zooms_out_to_its_group() {
    let _scope = test_scope();
    let fixture = Fixture::new(true);
    let events = fixture.record();
    fixture.zoom.update_layout();
    let header = fixture
        .grid
        .get_visual_descendants()
        .into_iter()
        .filter_map(|element| element.cast::<ListViewBaseHeaderItem>())
        .find(|header| text(&header.content()) == "G1")
        .expect("the header of G1");
    // ListViewBaseHeaderItem::on_tapped hands the tap to the list.
    assert!(fixture.grid.group_header_interaction(&header));
    assert!(!fixture.zoom.is_zoomed_in_view_active());
    let events = events.borrow();
    let args = &events[0].1;
    assert_eq!(text(&args.source_item().item()), "G1-0");
    assert!(args.destination_item().is_bottom_alignment());
    let group = group_of(&args.destination_item().item()).expect("the group of the header");
    assert!(Rc::ptr_eq(&group, &fixture.view.group_at(1).unwrap()));
}

fn raise_key_down(target: &SemanticZoom, key: Key, modifiers: KeyModifiers) -> bool {
    let mut e = KeyEventArgs::new();
    e.key = key;
    e.key_modifiers = modifiers;
    e.set_routed_event(Some(InputElement::key_down_event()));
    target.raise_event(&e);
    e.handled()
}

#[test]
fn ctrl_minus_zooms_out_and_ctrl_plus_zooms_in() {
    let _scope = test_scope();
    let fixture = Fixture::new(true);
    let zoom = &fixture.zoom;
    assert!(!raise_key_down(zoom, Key::OemMinus, KeyModifiers::NONE));
    assert!(zoom.is_zoomed_in_view_active());
    assert!(!raise_key_down(zoom, Key::OemPlus, KeyModifiers::CONTROL), "already zoomed in");
    assert!(raise_key_down(zoom, Key::Subtract, KeyModifiers::CONTROL | KeyModifiers::SHIFT));
    assert!(!zoom.is_zoomed_in_view_active());
    assert!(raise_key_down(zoom, Key::Add, KeyModifiers::CONTROL));
    assert!(zoom.is_zoomed_in_view_active());
    zoom.set_can_change_views(false);
    assert!(!raise_key_down(zoom, Key::OemMinus, KeyModifiers::CONTROL));
    zoom.toggle_active_view();
    assert!(zoom.is_zoomed_in_view_active(), "no change while CanChangeViews is false");
}

#[test]
fn the_zoom_out_button_toggles_and_hides_with_a_change() {
    let _scope = test_scope();
    let fixture = Fixture::new(true);
    let button = fixture
        .zoom
        .get_template_descendants()
        .into_iter()
        .find_map(|element| element.cast::<Button>())
        .expect("the zoom-out button");
    assert!(!fixture.zoom.is_zoom_out_button_visible());
    button.raise_event(&ferroui_base::interactivity::RoutedEventArgs::with_event(Button::click_event()));
    assert!(!fixture.zoom.is_zoomed_in_view_active());
    assert!(!fixture.zoom.is_zoom_out_button_visible());
}

#[test]
fn the_automation_peer_toggles_the_views_and_shows_the_active_view() {
    let _scope = test_scope();
    let fixture = Fixture::new(true);
    let peer = ControlAutomationPeer::create_peer_for_element(&fixture.zoom);
    assert_eq!(peer.get_automation_control_type(), AutomationControlType::Group);
    assert_eq!(peer.get_class_name(), "SemanticZoom");
    let toggle = peer.get_provider::<dyn IToggleProvider>().expect("the toggle pattern");
    assert_eq!(toggle.toggle_state(), ToggleState::On);
    let children = peer.get_children();
    assert_eq!(children.len(), 1);
    assert!(peer.cast::<SemanticZoomAutomationPeer>().is_some());
    toggle.toggle().unwrap();
    assert!(!fixture.zoom.is_zoomed_in_view_active());
    assert_eq!(toggle.toggle_state(), ToggleState::Off);
}

fn server(visual: &Rc<CompositionVisual>) -> Rc<ServerCompositionVisual> {
    visual.compositor().server().get::<ServerCompositionVisual>(visual.server()).unwrap()
}

fn seek(visual: &Rc<CompositionVisual>, progress: f32) {
    for property in ["Scale.X", "Scale.Y", "Opacity"] {
        let controller = visual.try_get_animation_controller(property).expect("a running animation");
        controller.pause();
        controller.set_progress(progress);
    }
}

fn close(actual: f64, expected: f64) {
    assert!((actual - expected).abs() < 0.05, "{actual} != {expected}");
}

/// The zoom of a change runs on the compositor: both presenters shrink when the semantic zoom
/// zooms out (the zoomed-in view from 1 to 0.5, the zoomed-out view from 2 to 1) and cross-fade;
/// `ViewChangeCompleted` waits for the end of the animations.
#[test]
fn the_views_zoom_and_fade_on_the_compositor() {
    assert_eq!(ZOOM_DURATION, Duration::from_millis(300));
    assert_eq!(ZOOM_MIN, 0.5);
    let services = CompositorTestServices::new(Size::new(400.0, 300.0));
    let zoom = SemanticZoom::new();
    zoom.set_template(semantic_zoom_template());
    let (zoomed_in, zoomed_out) = (ContentControl::new(), ContentControl::new());
    zoom.set_zoomed_in_view(Some(zoomed_in.upcast()));
    zoom.set_zoomed_out_view(Some(zoomed_out.upcast()));
    services.set_content(&zoom);
    services.run_jobs();
    let part = |name: &str| {
        zoom.get_template_descendants()
            .into_iter()
            .find(|element| element.name().as_deref() == Some(name))
            .and_then(|element| element.cast::<Control>())
            .unwrap()
    };
    let (in_part, out_part) = (part("ZoomedInPresenter"), part("ZoomedOutPresenter"));
    let in_visual = ElementComposition::get_element_visual(&in_part).expect("a composited presenter");
    let out_visual = ElementComposition::get_element_visual(&out_part).expect("a composited presenter");
    let completed = Rc::new(RefCell::new(0));
    let counter = completed.clone();
    zoom.view_change_completed(move |_, _| *counter.borrow_mut() += 1).detach();

    zoom.set_is_zoomed_in_view_active(false);
    assert!(zoom.is_view_change_animating());
    assert_eq!(*completed.borrow(), 0, "completed waits for the zoom");
    // Halfway: the opacity is linear, the scale follows the spline (most of the way already).
    seek(&in_visual, 0.5);
    seek(&out_visual, 0.5);
    services.run_jobs();
    services.run_jobs();
    let (in_scale, out_scale) = (server(&in_visual).scale().x as f64, server(&out_visual).scale().y as f64);
    assert!(in_scale > 0.5 && in_scale < 0.75, "{in_scale}");
    assert!(out_scale > 1.0 && out_scale < 1.5, "{out_scale}");
    close(server(&out_visual).opacity() as f64, 0.5);
    close(server(&in_visual).opacity() as f64, 0.5);
    seek(&in_visual, 1.0);
    seek(&out_visual, 1.0);
    services.run_jobs();
    services.run_jobs();
    close(server(&in_visual).scale().x as f64, 0.5);
    close(server(&out_visual).scale().y as f64, 1.0);
    close(server(&out_visual).opacity() as f64, 1.0);
    close(server(&in_visual).opacity() as f64, 0.0);
    for visual in [&in_visual, &out_visual] {
        for property in ["Scale.X", "Scale.Y", "Opacity"] {
            if let Some(controller) = visual.try_get_animation_controller(property) {
                controller.resume();
            }
        }
    }
    let started = Instant::now();
    while zoom.is_view_change_animating() && started.elapsed() < Duration::from_secs(2) {
        services.run_jobs();
    }
    services.run_jobs();
    assert!(!zoom.is_view_change_animating());
    assert_eq!(*completed.borrow(), 1);
    assert_eq!(out_part.opacity(), 1.0);
    assert_eq!(in_part.opacity(), 0.0);
    close(server(&in_visual).scale().x as f64, 1.0);

    // A change during a zoom completes the running one first.
    zoom.set_is_zoomed_in_view_active(true);
    assert!(zoom.is_view_change_animating());
    zoom.set_is_zoomed_in_view_active(false);
    assert_eq!(*completed.borrow(), 2);
}
