//! Tests of the pivot (#443, PV-1 and PV-2): the template parts, the panel, the selection through
//! the state machine, the visible items, the item events, the keyboard, the lock and the headers.

use super::pivot_common::PIVOT_PANEL_MULTIPLIER;
use super::pivot_state_machine::PivotState;
use super::*;
use crate::automation::peers::{AutomationControlType, ControlAutomationPeer};
use crate::presenters::{ItemsPresenter, ScrollContentPresenter};
use crate::primitives::{ScrollBarVisibility, SnapPointsAlignment, SnapPointsType};
use crate::templates::{FuncControlTemplate, FuncTemplateNameScopeExtensions, IControlTemplate};
use crate::test_support::{boxed_str, string_of, test_scope, TestRoot};
use crate::{ContentControl, Control, Grid, ItemsControl, Panel, ScrollViewer, SelectionMode, StackPanel};
use ferroui_base::data::TemplateBinding;
use ferroui_base::input::{InputElement, Key, KeyEventArgs, KeyModifiers};
use ferroui_base::layout::Orientation;
use ferroui_base::threading::Dispatcher;
use ferroui_base::Ref;
use std::cell::RefCell;
use std::rc::Rc;

const WIDTH: f64 = 400.0;

fn scroll_viewer_template() -> Rc<dyn IControlTemplate> {
    FuncControlTemplate::for_type::<ScrollViewer>(|_, scope| {
        let presenter = ScrollContentPresenter::new();
        presenter.set_name(Some("PART_ContentPresenter".to_string()));
        let panel = Panel::new();
        panel.children().add(presenter.register_in_name_scope(&**scope));
        panel.upcast()
    })
}

/// WinUI's template, reduced to the parts of PV-1.
fn pivot_template() -> Option<Rc<dyn IControlTemplate>> {
    Some(FuncControlTemplate::new(|_, scope| {
        let presenter = ItemsPresenter::new();
        presenter.set_name(Some("PivotItemPresenter".to_string()));
        let property = ItemsControl::items_panel_property().as_property();
        presenter.bind_binding(property, &TemplateBinding::new(property));
        // The headers (PV-2): the static and the dynamic panels in the header clipper.
        let static_header = PivotHeaderPanel::new();
        static_header.set_name(Some("StaticHeader".to_string()));
        let header = PivotHeaderPanel::new();
        header.set_name(Some("Header".to_string()));
        let header_grid = Grid::new();
        header_grid.children().add(static_header.register_in_name_scope(&**scope));
        header_grid.children().add(header.register_in_name_scope(&**scope));
        let header_clipper = ContentControl::new();
        header_clipper.set_name(Some("HeaderClipper".to_string()));
        header_clipper.set_content(Some(Control::boxed(header_grid)));
        let layout_element = StackPanel::new();
        layout_element.set_name(Some("PivotLayoutElement".to_string()));
        layout_element.children().add(header_clipper.register_in_name_scope(&**scope));
        layout_element.children().add(presenter.register_in_name_scope(&**scope));
        let panel = PivotPanel::new();
        panel.set_name(Some("Panel".to_string()));
        panel.children().add(layout_element.register_in_name_scope(&**scope));
        let scroll_viewer = ScrollViewer::new();
        scroll_viewer.set_name(Some("ScrollViewer".to_string()));
        scroll_viewer.set_template(Some(scroll_viewer_template()));
        scroll_viewer.set_horizontal_scroll_bar_visibility(ScrollBarVisibility::Hidden);
        scroll_viewer.set_vertical_scroll_bar_visibility(ScrollBarVisibility::Disabled);
        scroll_viewer.set_horizontal_snap_points_type(SnapPointsType::MandatorySingle);
        scroll_viewer.set_horizontal_snap_points_alignment(SnapPointsAlignment::Center);
        scroll_viewer.set_content(Some(Control::boxed(panel.register_in_name_scope(&**scope))));
        scroll_viewer.register_in_name_scope(&**scope).upcast()
    }))
}

/// A pivot of `count` items, laid out in a root.
fn pivot(count: usize) -> (Ref<Pivot>, Ref<TestRoot>, Vec<Ref<PivotItem>>) {
    let headers: Vec<String> = (0..count).map(|index| format!("Item {index}")).collect();
    pivot_with_headers(&headers)
}

/// A pivot with an item per header, laid out in a root.
fn pivot_with_headers(headers: &[String]) -> (Ref<Pivot>, Ref<TestRoot>, Vec<Ref<PivotItem>>) {
    let count = headers.len();
    let pivot = Pivot::new();
    pivot.set_template(pivot_template());
    pivot.set_width(WIDTH);
    pivot.set_height(300.0);
    let items: Vec<Ref<PivotItem>> = (0..count)
        .map(|index| {
            PivotItem::with_header(boxed_str(&headers[index]).unwrap(), boxed_str(&format!("Content {index}")))
        })
        .collect();
    for item in &items {
        pivot.items().add(Some(Control::boxed(item.clone())));
    }
    let root = TestRoot::with_child(pivot.clone());
    root.layout_manager().execute_initial_layout_pass();
    (pivot, root, items)
}

fn layout(root: &TestRoot) {
    root.layout_manager().execute_layout_pass();
}

fn settle(root: &TestRoot) {
    for _ in 0..4 {
        Dispatcher::ui_thread().run_jobs(None);
        layout(root);
    }
}

#[test]
fn the_defaults_are_winui_s() {
    let _scope = test_scope();
    let pivot = Pivot::new();
    assert!(pivot.is_header_items_carousel_enabled());
    assert!(!pivot.is_locked());
    assert_eq!(pivot.header_focus_visual_placement(), PivotHeaderFocusVisualPlacement::ItemHeaders);
    assert_eq!(pivot.selection_mode(), SelectionMode::ALWAYS_SELECTED);
    assert!(pivot.title().is_none() && pivot.left_header().is_none() && pivot.right_header().is_none());
    let item = PivotItem::new();
    assert_eq!(Pivot::get_slide_in_animation_group(&item), PivotSlideInAnimationGroup::Default);
}

#[test]
fn the_panel_is_a_thousand_sections_wide_with_a_snap_point_per_section() {
    let _scope = test_scope();
    let (pivot, _root, _items) = pivot(3);
    assert_eq!(pivot.machine_state(), PivotState::Idle);
    let panel = pivot.panel_part().unwrap();
    assert_eq!(panel.section_width(), WIDTH);
    assert_eq!(panel.desired_size().width, WIDTH * PIVOT_PANEL_MULTIPLIER as f64);
    assert_eq!(
        panel.get_regular_snap_points(Orientation::Horizontal, SnapPointsAlignment::Center),
        (WIDTH, WIDTH / 2.0)
    );
    // The layout element is arranged at the middle of the panel.
    assert_eq!(panel.section_offset(), WIDTH * PIVOT_PANEL_MULTIPLIER as f64 / 2.0);
}

#[test]
fn only_the_selected_item_is_shown() {
    let _scope = test_scope();
    let (pivot, _root, items) = pivot(3);
    assert_eq!(pivot.selected_index(), 0);
    assert_eq!(items[0].opacity(), 1.0);
    assert!(items[0].is_hit_test_visible());
    for item in &items[1..] {
        assert_eq!(item.opacity(), 0.0);
        assert!(!item.is_hit_test_visible());
    }
}

#[test]
fn selecting_an_item_flies_to_it_and_raises_the_item_events() {
    let _scope = test_scope();
    let (pivot, root, items) = pivot(3);
    let log = Rc::new(RefCell::new(Vec::new()));
    let loaded_log = log.clone();
    let _loaded =
        pivot.pivot_item_loaded(move |_, e| loaded_log.borrow_mut().push(format!("loaded {:?}", e.item().header())));
    let unloaded_log = log.clone();
    let _unloaded = pivot
        .pivot_item_unloaded(move |_, e| unloaded_log.borrow_mut().push(format!("unloaded {:?}", e.item().header())));

    pivot.set_selected_index(2);
    // The selection changes at once; the machine flies the content out, then in.
    assert_eq!(pivot.selected_index(), 2);
    assert_eq!(pivot.machine_state(), PivotState::InFlyOutProgrammaticInertial);
    settle(&root);
    assert_eq!(pivot.machine_state(), PivotState::Idle);
    assert_eq!(items[2].opacity(), 1.0);
    assert_eq!(items[0].opacity(), 0.0);
    assert_eq!(log.borrow().len(), 2);
    assert!(log.borrow()[0].starts_with("unloaded"));
    assert!(log.borrow()[1].starts_with("loaded"));
}

#[test]
fn an_index_out_of_the_items_is_rejected() {
    let _scope = test_scope();
    let (pivot, _root, _items) = pivot(3);
    pivot.set_selected_index(5);
    assert_eq!(pivot.selected_index(), 0);
}

#[test]
fn control_page_down_and_control_tab_move_through_the_items() {
    let _scope = test_scope();
    let (pivot, root, _items) = pivot(3);
    let press = |key: Key, modifiers: KeyModifiers| {
        let mut e = KeyEventArgs::new();
        e.set_routed_event(Some(InputElement::key_down_event()));
        e.key = key;
        e.key_modifiers = modifiers;
        pivot.raise_event(&e);
        e.handled()
    };
    assert!(press(Key::PageDown, KeyModifiers::CONTROL));
    settle(&root);
    assert_eq!(pivot.selected_index(), 1);
    assert!(press(Key::Tab, KeyModifiers::CONTROL | KeyModifiers::SHIFT));
    settle(&root);
    assert_eq!(pivot.selected_index(), 0);
    // The headers fit, so they are static and do not wrap (WinUI's `ShouldWrap`).
    assert!(pivot.uses_static_headers());
    assert!(press(Key::PageUp, KeyModifiers::CONTROL));
    settle(&root);
    assert_eq!(pivot.selected_index(), 0);
    assert!(!press(Key::PageDown, KeyModifiers::NONE));
}

#[test]
fn a_locked_pivot_keeps_its_item() {
    let _scope = test_scope();
    let (pivot, _root, _items) = pivot(3);
    pivot.set_is_locked(true);
    assert_eq!(pivot.machine_state(), PivotState::Locked);
    assert!(!pivot.move_to_next_item(true));
    assert_eq!(pivot.selected_index(), 0);
    pivot.set_is_locked(false);
    assert_eq!(pivot.machine_state(), PivotState::Idle);
}

#[test]
fn removing_the_selected_item_shows_the_next() {
    let _scope = test_scope();
    let (pivot, root, items) = pivot(3);
    pivot.items().remove_at(0);
    settle(&root);
    assert_eq!(pivot.selected_index(), 0);
    assert_eq!(items[1].opacity(), 1.0);
}

#[test]
fn the_peers_are_a_tab_and_tab_items() {
    let _scope = test_scope();
    let (pivot, _root, items) = pivot(2);
    let peer = ControlAutomationPeer::create_peer_for_element(&pivot.clone().upcast());
    assert_eq!(peer.get_automation_control_type(), AutomationControlType::Tab);
    let item_peer = ControlAutomationPeer::create_peer_for_element(&items[1].clone().upcast());
    assert_eq!(item_peer.get_automation_control_type(), AutomationControlType::TabItem);
    assert_eq!(item_peer.get_name(), "Item 1");
}

fn headers(pivot: &Pivot) -> Vec<Ref<PivotHeaderItem>> {
    pivot.header_items()
}

#[test]
fn every_item_has_a_header_bound_to_its_header() {
    let _scope = test_scope();
    let (pivot, _root, items) = pivot(3);
    let headers = headers(&pivot);
    assert_eq!(headers.len(), 3);
    assert_eq!(headers[1].content().as_ref().and_then(string_of).as_deref(), Some("Item 1"));
    items[1].set_header(boxed_str("Renamed"));
    assert_eq!(headers[1].content().as_ref().and_then(string_of).as_deref(), Some("Renamed"));
    assert_eq!(headers[0].visual_state_name(), "Selected");
    assert_eq!(headers[1].visual_state_name(), "Unselected");
    assert!(headers[0].classes().contains(":selected"));
}

#[test]
fn tapping_a_header_selects_its_item() {
    let _scope = test_scope();
    let (pivot, root, _items) = pivot(3);
    let headers = headers(&pivot);
    pivot.on_header_item_tapped(&headers[2]);
    settle(&root);
    assert_eq!(pivot.selected_index(), 2);
    assert_eq!(headers[2].visual_state_name(), "Selected");
    assert_eq!(headers[0].visual_state_name(), "Unselected");
    // Locked, the taps are ignored and the other headers are hidden.
    pivot.set_is_locked(true);
    pivot.on_header_item_tapped(&headers[0]);
    settle(&root);
    assert_eq!(pivot.selected_index(), 2);
    assert_eq!(headers[0].visual_state_name(), "UnselectedLocked");
}

#[test]
fn the_dynamic_panel_starts_with_the_selected_header() {
    let _scope = test_scope();
    let long: Vec<String> =
        (0..3).map(|index| format!("A header far too long to fit beside the others, number {index}")).collect();
    let (pivot, root, _items) = pivot_with_headers(&long);
    // The headers do not fit: they wrap (the dynamic panel).
    assert!(!pivot.uses_static_headers());
    assert!(pivot.should_wrap_for_tests());
    pivot.set_selected_index(1);
    settle(&root);
    let headers = headers(&pivot);
    // The selected header is arranged first, the one before it last.
    assert!(headers[1].bounds().x < headers[2].bounds().x);
    assert!(headers[2].bounds().x < headers[0].bounds().x);
}

#[test]
fn without_the_carousel_the_headers_are_static_and_do_not_wrap() {
    let _scope = test_scope();
    let (pivot, root, _items) = pivot(3);
    pivot.set_is_header_items_carousel_enabled(false);
    settle(&root);
    assert!(pivot.uses_static_headers());
    assert!(pivot.classes().contains(":header-static"));
    // Static headers do not wrap: from the first item there is no previous one.
    assert!(!pivot.move_to_previous_item(pivot.should_wrap_for_tests()));
    assert!(pivot.move_to_last_item());
    settle(&root);
    assert_eq!(pivot.selected_index(), 2);
}

#[test]
fn the_header_template_reaches_the_headers() {
    let _scope = test_scope();
    let (pivot, _root, _items) = pivot(2);
    let template: Rc<dyn crate::templates::IDataTemplate> = crate::templates::FuncDataTemplate::new(
        |_: Option<&ferroui_base::BoxedValue>| true,
        |_, _| Some(crate::TextBlock::new().upcast::<Control>()),
        false,
    );
    pivot.set_header_template(Some(template));
    assert!(headers(&pivot).iter().all(|header| header.content_template().is_some()));
}

#[test]
fn the_header_keys_move_through_the_items() {
    let _scope = test_scope();
    let (pivot, root, _items) = pivot(3);
    let press = |key: Key| {
        let mut e = KeyEventArgs::new();
        e.set_routed_event(Some(InputElement::key_down_event()));
        e.key = key;
        pivot.on_header_key_down_for_tests(&e);
        e.handled()
    };
    assert!(press(Key::Right));
    settle(&root);
    assert_eq!(pivot.selected_index(), 1);
    assert!(press(Key::End));
    settle(&root);
    assert_eq!(pivot.selected_index(), 2);
    assert!(press(Key::Home));
    settle(&root);
    assert_eq!(pivot.selected_index(), 0);
}
