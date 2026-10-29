use super::support::start_themed_application;
use crate::{MaterialBottomSheetHost, MaterialSheetState};
use ferroui_base::input::{
    IPointer, InputElement, KeyModifiers, MouseButton, Pointer, PointerEventArgs,
    PointerPointProperties, PointerPressedEventArgs, PointerReleasedEventArgs, PointerType,
    PointerUpdateKind, RawInputModifiers,
};
use ferroui_base::{Point, Ref};
use ferroui_controls::{Border, Button, Control, Grid, ScrollViewer, Window};
use std::cell::Cell;
use std::rc::Rc;

fn host(
    sheet_height: f64,
) -> (
    Ref<Window>,
    Ref<MaterialBottomSheetHost>,
    Ref<Border>,
    Ref<Button>,
) {
    let window = Window::new();
    window.set_width(480.0);
    window.set_height(800.0);
    let grid = Grid::new();
    let trigger = Button::new();
    trigger.set_content(Some(Rc::new("Open".to_string())));
    grid.children().add(trigger.clone().upcast::<Control>());
    let sheet = MaterialBottomSheetHost::new();
    sheet.set_reduced_motion(true);
    let body = Border::new();
    body.set_height(sheet_height - 48.0);
    sheet.set_content(Some(Control::boxed(&body)));
    grid.children().add(sheet.clone().upcast::<Control>());
    window.set_content(Some(Control::boxed(&grid)));
    window.show();
    window.update_layout();
    (window, sheet, body, trigger)
}

#[test]
fn sheet_measures_real_content_and_back_visits_partial_before_dismiss() {
    let _app = start_themed_application();
    let (window, sheet, _, _) = host(600.0);
    assert!(!sheet.is_visible());
    assert!(sheet.show());
    window.update_layout();
    assert_eq!(MaterialSheetState::PartiallyExpanded, sheet.current_state());
    assert_eq!(400.0, sheet.anchors().partially_expanded);
    assert_eq!(200.0, sheet.anchors().expanded);
    assert_eq!(400.0, sheet.offset());
    assert!(sheet.expand());
    assert_eq!(200.0, sheet.offset());
    assert!(sheet.handle_back());
    assert_eq!(MaterialSheetState::PartiallyExpanded, sheet.current_state());
    assert!(sheet.handle_back());
    assert_eq!(MaterialSheetState::Hidden, sheet.current_state());
    assert!(!sheet.is_visible());
}

#[test]
fn short_sheet_remains_partial_then_expands_when_content_grows() {
    let _app = start_themed_application();
    let (window, sheet, body, _) = host(300.0);
    sheet.show();
    window.update_layout();
    assert_eq!(MaterialSheetState::PartiallyExpanded, sheet.state());
    assert_eq!(500.0, sheet.offset());
    body.set_height(552.0);
    window.update_layout();
    assert_eq!(MaterialSheetState::Expanded, sheet.state());
    assert_eq!(200.0, sheet.offset());
}

#[test]
fn dismissal_veto_and_completion_are_identity_safe() {
    let _app = start_themed_application();
    let (window, sheet, _, _) = host(600.0);
    let dismissed = Rc::new(Cell::new(0));
    let count = dismissed.clone();
    sheet.dismissed(move |_, _| count.set(count.get() + 1));
    sheet.show();
    window.update_layout();
    sheet.set_confirm_state_change(Some(Rc::new(|state| state != MaterialSheetState::Hidden)));
    assert!(!sheet.hide());
    assert_eq!(MaterialSheetState::PartiallyExpanded, sheet.state());
    assert_eq!(0, dismissed.get());
    sheet.set_confirm_state_change(None);
    assert!(sheet.hide());
    assert_eq!(1, dismissed.get());
    assert!(sheet.hide());
    assert_eq!(1, dismissed.get());
}

#[test]
fn sheet_restores_previous_focus_when_dismissed() {
    let _app = start_themed_application();
    let (window, sheet, _, trigger) = host(300.0);
    assert!(trigger.focus());
    sheet.show();
    window.update_layout();
    assert!(sheet.is_keyboard_focus_within());
    sheet.hide();
    assert!(trigger.is_focused());
}

#[test]
fn disabling_partial_state_retargets_expanded() {
    let _app = start_themed_application();
    let (window, sheet, _, _) = host(600.0);
    sheet.show();
    window.update_layout();
    sheet.set_is_partial_enabled(false);
    assert_eq!(MaterialSheetState::Expanded, sheet.state());
    assert_eq!(200.0, sheet.offset());
}

#[test]
fn host_is_not_retained_after_detachment_and_pending_layout() {
    let _app = start_themed_application();
    let (window, sheet, _, _) = host(600.0);
    sheet.show();
    window.update_layout();
    let weak = sheet.downgrade();
    window.set_content(None);
    window.update_layout();
    drop(sheet);
    window.close();
    ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);
    assert!(weak.upgrade().is_none());
}

fn touch_press(source: &Control, window: &Window, point: Point) -> Rc<Pointer> {
    let pointer = Pointer::new(Pointer::get_next_free_id(), PointerType::Touch, true);
    source.raise_event(&PointerPressedEventArgs::new(
        source.to_ref(),
        pointer.clone(),
        window,
        point,
        100,
        PointerPointProperties::new(
            RawInputModifiers::LEFT_MOUSE_BUTTON,
            PointerUpdateKind::LeftButtonPressed,
        ),
        KeyModifiers::NONE,
        1,
    ));
    pointer
}

fn touch_move(
    source: &Control,
    window: &Window,
    pointer: &Rc<Pointer>,
    point: Point,
    timestamp: u64,
) -> bool {
    let event = PointerEventArgs::new(
        Some(InputElement::pointer_moved_event()),
        source.to_ref(),
        pointer.clone(),
        Some(window),
        point,
        timestamp,
        PointerPointProperties::new(
            RawInputModifiers::LEFT_MOUSE_BUTTON,
            PointerUpdateKind::Other,
        ),
        KeyModifiers::NONE,
    );
    source.raise_event(&event);
    event.handled()
}

fn touch_release(source: &Control, window: &Window, pointer: &Rc<Pointer>, point: Point) {
    touch_release_at(source, window, pointer, point, 500);
}

fn touch_release_at(
    source: &Control,
    window: &Window,
    pointer: &Rc<Pointer>,
    point: Point,
    timestamp: u64,
) {
    source.raise_event(&PointerReleasedEventArgs::new(
        source.to_ref(),
        pointer.clone(),
        window,
        point,
        timestamp,
        PointerPointProperties::new(
            RawInputModifiers::NONE,
            PointerUpdateKind::LeftButtonReleased,
        ),
        KeyModifiers::NONE,
        MouseButton::Left,
    ));
}

#[test]
fn touch_scroll_expands_before_child_and_collapses_only_after_child_reaches_start() {
    let _app = start_themed_application();
    let (window, sheet, body, _) = host(1048.0);
    sheet.show();
    window.update_layout();
    let scroll = sheet
        .get_visual_descendants()
        .find_map(|visual| {
            (visual.name().as_deref() == Some("PART_ContentScroll"))
                .then(|| visual.cast::<ScrollViewer>())
                .flatten()
        })
        .unwrap();
    assert_eq!(400.0, sheet.offset());
    let pointer = touch_press(&body, &window, Point::new(100.0, 600.0));
    assert!(!touch_move(
        &body,
        &window,
        &pointer,
        Point::new(100.0, 598.0),
        110
    ));
    assert_eq!(400.0, sheet.offset());
    assert!(touch_move(
        &body,
        &window,
        &pointer,
        Point::new(100.0, 100.0),
        130
    ));
    assert_eq!(sheet.anchors().expanded, sheet.offset());
    assert!(
        scroll.offset().y > 50.0,
        "unconsumed upward motion must reach the child"
    );
    let child_offset = scroll.offset().y;
    assert!(touch_move(
        &sheet,
        &window,
        &pointer,
        Point::new(100.0, 100.0 + child_offset / 2.0),
        160
    ));
    assert_eq!(
        0.0,
        sheet.offset(),
        "the sheet must stay expanded while the child can scroll back"
    );
    assert!((scroll.offset().y - child_offset / 2.0).abs() < 0.001);
    assert!(touch_move(
        &sheet,
        &window,
        &pointer,
        Point::new(100.0, 300.0),
        190
    ));
    assert_eq!(0.0, scroll.offset().y);
    assert!(
        sheet.offset() > 80.0,
        "remaining downward motion must collapse the sheet"
    );
    touch_release(&sheet, &window, &pointer, Point::new(100.0, 300.0));
    assert!(pointer.captured().is_none());
    assert!(!sheet.classes().contains(":dragging"));
}

#[test]
fn horizontal_touch_is_left_to_the_child_and_detach_releases_a_sheet_capture() {
    let _app = start_themed_application();
    let (window, sheet, body, _) = host(600.0);
    sheet.show();
    window.update_layout();
    let pointer = touch_press(&body, &window, Point::new(100.0, 600.0));
    assert!(!touch_move(
        &body,
        &window,
        &pointer,
        Point::new(180.0, 602.0),
        130
    ));
    assert_eq!(400.0, sheet.offset());
    touch_release(&body, &window, &pointer, Point::new(180.0, 602.0));
    let pointer = touch_press(&body, &window, Point::new(100.0, 600.0));
    assert!(touch_move(
        &body,
        &window,
        &pointer,
        Point::new(100.0, 500.0),
        130
    ));
    assert!(pointer.captured().is_some());
    window.set_content(None);
    assert!(pointer.captured().is_none());
}

#[test]
fn persistent_expanded_sheet_cannot_drag_into_a_disabled_partial_anchor() {
    let _app = start_themed_application();
    let (window, sheet, body, _) = host(600.0);
    sheet.set_is_partial_enabled(false);
    sheet.set_is_dismiss_enabled(false);
    sheet.show();
    window.update_layout();
    let expanded = sheet.anchors().expanded;
    let pointer = touch_press(&body, &window, Point::new(100.0, 400.0));
    assert!(touch_move(
        &body,
        &window,
        &pointer,
        Point::new(100.0, 600.0),
        130
    ));
    assert_eq!(expanded, sheet.offset());
    touch_release(&sheet, &window, &pointer, Point::new(100.0, 600.0));
    assert_eq!(MaterialSheetState::Expanded, sheet.current_state());
}

#[test]
fn automation_exposes_semantic_sheet_states_and_honors_dismissal_policy() {
    use ferroui_controls::automation::{
        ExpandCollapseState,
        peers::{AutomationControlType, ControlAutomationPeer},
        provider::IExpandCollapseProvider,
    };
    let _app = start_themed_application();
    let (window, sheet, _, _) = host(600.0);
    let peer = ControlAutomationPeer::create_peer_for_element(&sheet);
    assert_eq!(
        AutomationControlType::Pane,
        peer.get_automation_control_type()
    );
    let provider = peer
        .get_provider::<dyn IExpandCollapseProvider>()
        .expect("sheet expansion actions");
    assert_eq!(
        ExpandCollapseState::Collapsed,
        provider.expand_collapse_state()
    );
    sheet.show();
    window.update_layout();
    assert_eq!(
        ExpandCollapseState::PartiallyExpanded,
        provider.expand_collapse_state()
    );
    provider.expand().unwrap();
    assert_eq!(
        ExpandCollapseState::Expanded,
        provider.expand_collapse_state()
    );
    sheet.set_is_dismiss_enabled(false);
    assert!(provider.collapse().is_err());
    assert_eq!(MaterialSheetState::Expanded, sheet.state());
    sheet.set_is_dismiss_enabled(true);
    sheet.set_confirm_state_change(Some(Rc::new(|state| state != MaterialSheetState::Hidden)));
    assert!(provider.collapse().is_err());
    assert_eq!(
        ExpandCollapseState::Expanded,
        provider.expand_collapse_state()
    );
    sheet.set_confirm_state_change(None);
    provider.collapse().unwrap();
    assert_eq!(
        ExpandCollapseState::Collapsed,
        provider.expand_collapse_state()
    );
    sheet.set_is_enabled(false);
    assert!(provider.expand().is_err());
    window.close();
}

fn animated_sheet_application() -> ferroui_controls::testing::UnitTestApplicationScope {
    use ferroui_controls::testing::{TestServices, UnitTestApplication};
    crate::register_types();
    UnitTestApplication::start(
        TestServices::styled_window()
            .with_keyboard_device(|| Some(ferroui_base::input::KeyboardDevice::new()))
            .with_platform_settings(crate::motion::MotionTestSettings::new())
            .with_global_clock(Rc::new(super::support::MockGlobalClock::new()))
            .with_theme(|| crate::MaterialTheme::new().as_style()),
    )
}

fn content_scroll(sheet: &MaterialBottomSheetHost) -> Ref<ScrollViewer> {
    sheet
        .get_visual_descendants()
        .find_map(|visual| {
            (visual.name().as_deref() == Some("PART_ContentScroll"))
                .then(|| visual.cast::<ScrollViewer>())
                .flatten()
        })
        .unwrap()
}

fn fling_up(window: &Window, sheet: &MaterialBottomSheetHost, body: &Control) {
    let pointer = touch_press(body, window, Point::new(100.0, 600.0));
    assert!(touch_move(
        body,
        window,
        &pointer,
        Point::new(100.0, 500.0),
        130
    ));
    touch_release_at(sheet, window, &pointer, Point::new(100.0, 500.0), 140);
    assert!(pointer.captured().is_none());
}

#[test]
fn upward_pre_fling_settles_the_sheet_and_consumes_velocity_before_the_child() {
    let _app = animated_sheet_application();
    let (window, sheet, body, _) = host(5048.0);
    sheet.show();
    window.update_layout();
    sheet.set_reduced_motion(false);
    let scroll = content_scroll(&sheet);
    fling_up(&window, &sheet, &body);
    assert!(sheet.offset() > sheet.anchors().expanded);
    assert_eq!(MaterialSheetState::Expanded, sheet.state());
    assert!(!sheet.has_content_fling());
    assert!(sheet.is_animating());
    sheet.advance_for_test(1.0);
    assert_eq!(
        0.0,
        scroll.offset().y,
        "pre-fling must consume all upward velocity into the sheet"
    );
    assert!((sheet.offset() - sheet.anchors().expanded).abs() < 0.1);
    window.close();
}

#[test]
fn expanded_sheet_child_keeps_native_spline_inertia_without_inertial_sheet_scroll() {
    let _app = animated_sheet_application();
    let (window, sheet, body, _) = host(5048.0);
    sheet.expand();
    window.update_layout();
    sheet.set_reduced_motion(false);
    let scroll = content_scroll(&sheet);
    fling_up(&window, &sheet, &body);
    let release_offset = scroll.offset().y;
    let curve = crate::MaterialAppBarFling::new(-release_offset / 0.03);
    assert!(sheet.has_content_fling());
    assert!(release_offset > 0.0);
    sheet.advance_for_test(0.05);
    assert!((scroll.offset().y - (release_offset - curve.sample(0.05).0)).abs() < 1e-6);
    assert_eq!(sheet.anchors().expanded, sheet.offset());
    assert_eq!(MaterialSheetState::Expanded, sheet.state());
    sheet.advance_for_test(10.0);
    assert!(!sheet.has_content_fling());
    assert!((scroll.offset().y - (release_offset - curve.target_distance())).abs() < 1e-6);
    assert_eq!(sheet.anchors().expanded, sheet.offset());
    window.close();
}

#[test]
fn child_boundary_passes_remaining_fling_velocity_to_a_separate_sheet_settle() {
    let _app = animated_sheet_application();
    let (window, sheet, body, _) = host(5048.0);
    sheet.expand();
    window.update_layout();
    sheet.set_reduced_motion(false);
    let scroll = content_scroll(&sheet);
    scroll.set_offset(ferroui_base::Vector::new(0.0, 150.0));
    let pointer = touch_press(&body, &window, Point::new(100.0, 500.0));
    assert!(touch_move(
        &body,
        &window,
        &pointer,
        Point::new(100.0, 600.0),
        130
    ));
    assert!(scroll.offset().y > 0.0);
    touch_release_at(&sheet, &window, &pointer, Point::new(100.0, 600.0), 140);
    assert!(sheet.has_content_fling());
    sheet.advance_for_test(0.05);
    assert_eq!(0.0, scroll.offset().y);
    assert!(!sheet.has_content_fling());
    assert_eq!(
        0.0,
        sheet.offset(),
        "inertial remainder must not be dispatched as a user drag delta"
    );
    assert_eq!(
        MaterialSheetState::PartiallyExpanded,
        sheet.state(),
        "nonzero remaining velocity chooses the adjacent anchor"
    );
    assert!(sheet.is_animating());
    sheet.advance_for_test(0.03);
    assert!(sheet.offset() > 0.0);
    window.close();
}

#[test]
fn child_inertia_stops_on_new_contact_hidden_disabled_or_reduced_motion() {
    let _app = animated_sheet_application();
    for cancel in ["contact", "hidden", "disabled", "reduced"] {
        let (window, sheet, body, _) = host(5048.0);
        sheet.expand();
        window.update_layout();
        sheet.set_reduced_motion(false);
        let scroll = content_scroll(&sheet);
        fling_up(&window, &sheet, &body);
        sheet.advance_for_test(0.02);
        assert!(sheet.has_content_fling());
        let offset = scroll.offset().y;
        match cancel {
            "contact" => {
                let pointer = touch_press(&body, &window, Point::new(100.0, 600.0));
                assert!(!sheet.has_content_fling());
                touch_release(&body, &window, &pointer, Point::new(100.0, 600.0));
            }
            "hidden" => sheet.set_is_visible(false),
            "disabled" => sheet.set_is_enabled(false),
            _ => sheet.set_reduced_motion(true),
        }
        assert!(!sheet.has_content_fling(), "{cancel}");
        sheet.advance_for_test(1.0);
        assert_eq!(offset, scroll.offset().y, "{cancel}");
        window.close();
    }
}

#[test]
fn detached_child_inertia_releases_pending_work_and_does_not_retain_the_sheet() {
    let _app = animated_sheet_application();
    let (window, sheet, body, _) = host(5048.0);
    sheet.expand();
    window.update_layout();
    sheet.set_reduced_motion(false);
    let scroll = content_scroll(&sheet);
    fling_up(&window, &sheet, &body);
    assert!(sheet.has_content_fling());
    let offset = scroll.offset().y;
    let weak = sheet.downgrade();
    window.set_content(None);
    window.update_layout();
    assert!(!sheet.has_content_fling());
    assert!(!sheet.is_animating());
    sheet.advance_for_test(0.5);
    assert_eq!(offset, scroll.offset().y);
    drop(sheet);
    drop(scroll);
    drop(body);
    window.close();
    ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);
    assert!(weak.upgrade().is_none());
}

#[test]
fn child_offset_callback_can_dismiss_without_restoring_an_interrupted_fling() {
    let _app = animated_sheet_application();
    let (window, sheet, body, _) = host(5048.0);
    sheet.expand();
    window.update_layout();
    sheet.set_reduced_motion(false);
    let scroll = content_scroll(&sheet);
    fling_up(&window, &sheet, &body);
    let weak = sheet.downgrade();
    scroll.property_changed(move |event| {
        if event.property() == ScrollViewer::offset_property().as_property() {
            if let Some(sheet) = weak.upgrade() {
                sheet.hide();
            }
        }
    });
    sheet.advance_for_test(0.02);
    assert!(!sheet.has_content_fling());
    assert_eq!(MaterialSheetState::Hidden, sheet.state());
    sheet.advance_for_test(6.0);
    assert_eq!(MaterialSheetState::Hidden, sheet.current_state());
    assert!(!sheet.is_visible());
    window.close();
}
