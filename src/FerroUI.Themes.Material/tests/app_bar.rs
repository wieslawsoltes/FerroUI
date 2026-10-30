//! Source-state, arranged layout, real scroll routing and lifecycle conformance.
use crate::app_bar_state::{cubic_bezier, interpolate_container};
use crate::{
    MaterialAppBar, MaterialAppBarFling, MaterialAppBarScrollBehavior as Behavior,
    MaterialAppBarSize as Variant, MaterialAppBarState, MaterialMotionRole, MaterialMotionScheme,
    MaterialTheme, register_types,
};
use ferroui_base::animation::{IClock, IGlobalClock, PlayState, TimeSpan};
use ferroui_base::input::{
    IPointer, InputElement, KeyModifiers, KeyboardDevice, MouseButton, NavigationMethod, Pointer,
    PointerEventArgs, PointerPointProperties, PointerPressedEventArgs, PointerReleasedEventArgs,
    PointerType,
};
use ferroui_base::media::{Color, FlowDirection};
use ferroui_base::reactive::{IDisposable, IObservable, IObserver, LightweightSubject};
use ferroui_base::{ObjectType, Point, Ref, Thickness, Vector, Visual};
use ferroui_controls::automation::peers::{ButtonAutomationPeer, ControlAutomationPeer};
use ferroui_controls::automation::{AccessibilityView, AutomationProperties};
use ferroui_controls::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};
use ferroui_controls::{
    Border, Button, Control, Grid, RowDefinitions, ScrollViewer, StackPanel, TextBlock, Window,
};
use std::cell::Cell;
use std::rc::Rc;

struct Clock {
    subject: LightweightSubject<TimeSpan>,
    state: Cell<PlayState>,
}
impl IObservable<TimeSpan> for Clock {
    fn subscribe(&self, observer: Rc<dyn IObserver<TimeSpan>>) -> Rc<dyn IDisposable> {
        self.subject.subscribe(observer)
    }
}
impl IClock for Clock {
    fn play_state(&self) -> PlayState {
        self.state.get()
    }
    fn set_play_state(&self, value: PlayState) {
        self.state.set(value);
    }
}
impl IGlobalClock for Clock {}
fn application() -> (UnitTestApplicationScope, Rc<Clock>) {
    register_types();
    let clock = Rc::new(Clock {
        subject: LightweightSubject::new(),
        state: Cell::new(PlayState::Run),
    });
    let app = UnitTestApplication::start(
        TestServices::styled_window()
            .with_keyboard_device(|| Some(KeyboardDevice::new()))
            .with_global_clock(clock.clone())
            .with_platform_settings(crate::motion::MotionTestSettings::new())
            .with_theme(|| MaterialTheme::new().as_style()),
    );
    (app, clock)
}
fn named<T: ObjectType>(visual: &Visual, name: &str) -> Ref<T> {
    visual
        .get_visual_descendants()
        .filter(|item| item.name().as_deref() == Some(name))
        .find_map(|item| item.cast::<T>())
        .unwrap_or_else(|| panic!("missing app bar part {name}"))
}
fn app_bar(variant: Variant, reduced: bool) -> Ref<MaterialAppBar> {
    let bar = MaterialAppBar::new();
    bar.set_size_variant(variant);
    bar.set_title(Some("Library".into()));
    bar.set_reduced_motion(reduced);
    bar
}
fn window(control: &Control) -> Ref<Window> {
    let window = Window::new();
    window.set_width(480.0);
    window.set_height(640.0);
    window.set_content(Some(Control::boxed(control.to_ref())));
    window.show();
    window.update_layout();
    window
}
fn scroll_host(behavior: Behavior) -> (Ref<Window>, Ref<MaterialAppBar>, Ref<ScrollViewer>) {
    let bar = app_bar(Variant::LargeFlexible, true);
    bar.set_subtitle(Some("Everything you saved".into()));
    bar.set_scroll_behavior(behavior);
    let scroll = ScrollViewer::new();
    let body = Border::new();
    body.set_height(1600.0);
    scroll.set_content(Some(Control::boxed(&body)));
    let grid = Grid::new();
    grid.set_row_definitions(RowDefinitions::parse("Auto,*").unwrap());
    Grid::set_row(&scroll, 1);
    grid.children().add(bar.clone().upcast::<Control>());
    grid.children().add(scroll.clone().upcast::<Control>());
    bar.set_scroll_source(Some(scroll.clone()));
    (window(&grid), bar, scroll)
}

#[test]
fn state_clamps_offsets_and_tracks_overlap_for_pre_scrolled_content() {
    let mut state = MaterialAppBarState::new(-88.0);
    state.set_height_offset(-44.0);
    assert_eq!(0.5, state.collapsed_fraction());
    state.set_content_offset(-22.0);
    assert_eq!(0.25, state.overlapped_fraction());
    state.set_height_offset(-999.0);
    assert_eq!(-88.0, state.height_offset());
    state.set_height_offset(f64::NAN);
    assert_eq!(-88.0, state.height_offset());
    state.set_height_offset_limit(-20.0);
    assert_eq!(-20.0, state.height_offset());
    state.set_content_offset(0.0);
    state.set_content_at_start(false);
    assert_eq!(1.0, state.overlapped_fraction());
    state.set_content_at_start(true);
    assert_eq!(0.0, state.overlapped_fraction());
}

#[test]
fn pre_scroll_contract_distinguishes_pinned_enter_always_and_exit_until_collapsed() {
    for behavior in [
        Behavior::Pinned,
        Behavior::EnterAlways,
        Behavior::ExitUntilCollapsed,
    ] {
        let mut state = MaterialAppBarState::new(-88.0);
        let pinned = behavior == Behavior::Pinned;
        assert_eq!(
            if pinned { 0.0 } else { -32.0 },
            state.pre_scroll(behavior, -32.0)
        );
        assert_eq!(if pinned { 0.0 } else { -32.0 }, state.height_offset());
        let reveal = behavior == Behavior::EnterAlways;
        assert_eq!(
            if reveal { 16.0 } else { 0.0 },
            state.pre_scroll(behavior, 16.0)
        );
        if !pinned {
            assert_eq!(-200.0, state.pre_scroll(behavior, -200.0));
            assert_eq!(-88.0, state.height_offset());
        }
    }
}

#[test]
fn exit_until_collapsed_expands_only_after_the_child_runs_out_of_downward_scroll() {
    let mut state = MaterialAppBarState::new(-88.0);
    state.set_height_offset(-88.0);
    assert_eq!(
        0.0,
        state.post_scroll(Behavior::ExitUntilCollapsed, 40.0, 0.0)
    );
    assert_eq!(-88.0, state.height_offset());
    assert_eq!(
        24.0,
        state.post_scroll(Behavior::ExitUntilCollapsed, 0.0, 24.0)
    );
    assert_eq!(-64.0, state.height_offset());
    assert_eq!(40.0, state.content_offset());
    state.reset_content_after_fling(10.0);
    assert_eq!(0.0, state.content_offset());
}

#[test]
fn native_snap_has_one_percent_tolerance_and_halfway_collapses() {
    let mut state = MaterialAppBarState::new(-100.0);
    for (offset, target) in [
        (-0.9, None),
        (-1.0, Some(0.0)),
        (-49.99, Some(0.0)),
        (-50.0, Some(-100.0)),
        (-100.0, None),
    ] {
        state.set_height_offset(offset);
        assert_eq!(target, state.snap_target());
    }
}

#[test]
fn fling_uses_android_density_normalized_physics_and_native_spline_endpoints() {
    let fling = MaterialAppBarFling::new(1000.0);
    assert_eq!(0.555, fling.duration_seconds());
    assert!((194.31362 - fling.target_distance()).abs() < 0.001);
    assert!(fling.sample(0.0).0.abs() < 0.01);
    let halfway = fling.sample(0.25);
    assert!(halfway.0 > 150.0 && halfway.0 < 170.0);
    assert!(halfway.1 > 250.0 && halfway.1 < 450.0);
    assert_eq!((fling.target_distance(), 0.0), fling.sample(2.0));
    assert_eq!(
        -fling.target_distance(),
        MaterialAppBarFling::new(-1000.0).target_distance()
    );
    assert_eq!(MaterialAppBarFling::new(0.0).sample(1.0), (0.0, 0.0));
}

#[test]
fn all_six_app_bar_variants_have_native_heights_and_flexible_typography() {
    let (_app, _) = application();
    for (variant, subtitle, height, font) in [
        (Variant::Small, false, 64.0, 22.0),
        (Variant::CenterAligned, false, 64.0, 22.0),
        (Variant::Medium, false, 112.0, 24.0),
        (Variant::Large, false, 152.0, 28.0),
        (Variant::MediumFlexible, false, 112.0, 28.0),
        (Variant::MediumFlexible, true, 136.0, 28.0),
        (Variant::LargeFlexible, false, 120.0, 36.0),
        (Variant::LargeFlexible, true, 152.0, 36.0),
    ] {
        let bar = app_bar(variant, true);
        if subtitle {
            bar.set_subtitle(Some("All files".into()));
        }
        let window = window(&bar);
        assert_eq!(height, bar.bounds().height, "{variant:?}");
        let text: Ref<TextBlock> = named(
            &bar,
            if variant.has_two_rows() {
                "PART_ExpandedTitleText"
            } else {
                "PART_CompactTitleText"
            },
        );
        assert_eq!(font, text.font_size());
        bar.set_scroll_behavior(Behavior::ExitUntilCollapsed);
        bar.collapse();
        window.update_layout();
        assert_eq!(
            if variant.has_two_rows() { 64.0 } else { 0.0 },
            bar.bounds().height
        );
        window.close();
    }
}

#[test]
fn centered_title_is_centered_in_the_whole_bar_and_avoids_unequal_action_slots() {
    let (_app, _) = application();
    let bar = app_bar(Variant::CenterAligned, true);
    let nav = Border::new();
    nav.set_width(48.0);
    nav.set_height(48.0);
    bar.set_navigation(Some(Control::boxed(&nav)));
    let actions = Border::new();
    actions.set_width(96.0);
    actions.set_height(48.0);
    bar.set_actions(Some(Control::boxed(&actions)));
    let window = window(&bar);
    let title: Ref<StackPanel> = named(&bar, "PART_CompactTitle");
    let center = title
        .translate_point(Point::new(title.bounds().width / 2.0, 0.0), &bar)
        .unwrap();
    assert!((center.x - 240.0).abs() < 1.0);
    assert_eq!(4.0, nav.translate_point(Point::default(), &bar).unwrap().x);
    assert_eq!(
        380.0,
        actions.translate_point(Point::default(), &bar).unwrap().x
    );
    bar.set_flow_direction(FlowDirection::RightToLeft);
    window.update_layout();
    assert!(
        nav.translate_point(Point::default(), &window).unwrap().x
            > actions
                .translate_point(Point::default(), &window)
                .unwrap()
                .x
    );
}

#[test]
fn expanded_title_uses_native_last_baseline_padding_and_clips_below_compact_row() {
    let (_app, _) = application();
    let bar = app_bar(Variant::Large, true);
    let window = window(&bar);
    let title: Ref<TextBlock> = named(&bar, "PART_ExpandedTitleText");
    let position = title.translate_point(Point::default(), &bar).unwrap();
    assert_eq!(16.0, position.x);
    assert!((152.0 - 28.0 - position.y - title.text_layout().baseline()).abs() < 1.0);
    bar.set_height_offset(-80.0);
    window.update_layout();
    let clip: Ref<Border> = named(&bar, "PART_ExpandedClip");
    assert_eq!(64.0, clip.bounds().y);
    assert_eq!(8.0, clip.bounds().height);
    assert!(clip.clip_to_bounds());
}

#[test]
fn title_crossfade_and_accessibility_switch_follow_native_fraction_not_visible_alpha() {
    let (_app, _) = application();
    let bar = app_bar(Variant::MediumFlexible, true);
    let _window = window(&bar);
    bar.set_height_offset(-24.0);
    let compact: Ref<StackPanel> = named(&bar, "PART_CompactTitle");
    let expanded: Ref<StackPanel> = named(&bar, "PART_ExpandedTitle");
    assert!((compact.opacity() - cubic_bezier(0.5, 0.8, 0.0, 0.8, 0.15)).abs() < 1e-10);
    assert_eq!(0.5, expanded.opacity());
    assert!(compact.opacity() < 0.5);
    let compact_text: Ref<TextBlock> = named(&bar, "PART_CompactTitleText");
    let expanded_text: Ref<TextBlock> = named(&bar, "PART_ExpandedTitleText");
    assert_eq!(
        AccessibilityView::Default,
        AutomationProperties::get_accessibility_view(&compact_text)
    );
    assert_eq!(
        AccessibilityView::Raw,
        AutomationProperties::get_accessibility_view(&expanded_text)
    );
    assert_eq!(1, AutomationProperties::get_heading_level(&compact_text));
    bar.set_height_offset(-23.0);
    assert_eq!(
        AccessibilityView::Raw,
        AutomationProperties::get_accessibility_view(&compact_text)
    );
    assert_eq!(
        AccessibilityView::Default,
        AutomationProperties::get_accessibility_view(&expanded_text)
    );
}

#[test]
fn physical_scroll_source_consumes_pre_child_post_in_native_order() {
    let (_app, _) = application();
    let (window, bar, scroll) = scroll_host(Behavior::ExitUntilCollapsed);
    assert_eq!(-40.0, bar.scroll_content_by(-40.0));
    window.update_layout();
    assert_eq!(-40.0, bar.height_offset());
    assert_eq!(0.0, scroll.offset().y);
    bar.scroll_content_by(-100.0);
    window.update_layout();
    assert_eq!(-88.0, bar.height_offset());
    assert_eq!(0.0, scroll.offset().y);
    bar.scroll_content_by(-120.0);
    window.update_layout();
    assert_eq!(120.0, scroll.offset().y);
    bar.scroll_content_by(60.0);
    window.update_layout();
    assert_eq!(60.0, scroll.offset().y);
    assert_eq!(-88.0, bar.height_offset());
    bar.scroll_content_by(100.0);
    window.update_layout();
    assert_eq!(0.0, scroll.offset().y);
    assert_eq!(-48.0, bar.height_offset());
    bar.set_scroll_behavior(Behavior::EnterAlways);
    scroll.set_offset(Vector::new(0.0, 300.0));
    window.update_layout();
    bar.scroll_content_by(20.0);
    window.update_layout();
    assert_eq!(-68.0, bar.height_offset());
    assert_eq!(300.0, scroll.offset().y);
}

#[test]
fn real_touch_routing_preserves_taps_and_horizontal_motion_then_captures_vertical_drag() {
    let (_app, _) = application();
    let (window, bar, scroll) = scroll_host(Behavior::ExitUntilCollapsed);
    let pointer = Pointer::new(892, PointerType::Touch, true);
    let press = |point| {
        PointerPressedEventArgs::new(
            scroll.to_ref(),
            pointer.clone(),
            &window,
            point,
            100,
            PointerPointProperties::default(),
            KeyModifiers::NONE,
            1,
        )
    };
    scroll.raise_event(&press(Point::new(100.0, 250.0)));
    let horizontal = PointerEventArgs::new(
        Some(InputElement::pointer_moved_event()),
        scroll.to_ref(),
        pointer.clone(),
        Some(&window),
        Point::new(130.0, 252.0),
        110,
        PointerPointProperties::default(),
        KeyModifiers::NONE,
    );
    scroll.raise_event(&horizontal);
    assert!(!horizontal.handled());
    assert_eq!(0.0, bar.height_offset());
    scroll.raise_event(&press(Point::new(100.0, 250.0)));
    let vertical = PointerEventArgs::new(
        Some(InputElement::pointer_moved_event()),
        scroll.to_ref(),
        pointer.clone(),
        Some(&window),
        Point::new(100.0, 210.0),
        120,
        PointerPointProperties::default(),
        KeyModifiers::NONE,
    );
    scroll.raise_event(&vertical);
    assert!(vertical.handled());
    assert_eq!(
        -32.0,
        bar.height_offset(),
        "the first drag delta excludes 8dp touch slop"
    );
    assert!(pointer.captured().is_some());
    let release = PointerReleasedEventArgs::new(
        scroll.to_ref(),
        pointer.clone(),
        &window,
        Point::new(100.0, 210.0),
        200,
        PointerPointProperties::default(),
        KeyModifiers::NONE,
        MouseButton::None,
    );
    scroll.raise_event(&release);
    assert!(pointer.captured().is_none());
    assert_eq!(0.0, bar.height_offset(), "a below-halfway drag snaps open");
}

#[test]
fn snap_uses_default_effects_spring_and_reduced_motion_removes_live_clock_work() {
    let (_app, clock) = application();
    let bar = app_bar(Variant::Large, false);
    bar.set_scroll_behavior(Behavior::ExitUntilCollapsed);
    let window = window(&bar);
    bar.set_height_offset(-60.0);
    bar.settle(0.0);
    assert!(clock.subject.has_observers());
    clock.subject.on_next(TimeSpan::from_seconds(0.0));
    clock.subject.on_next(TimeSpan::from_seconds(0.05));
    let expected = MaterialMotionScheme::Expressive
        .spring(MaterialMotionRole::DefaultEffects)
        .sample(0.05, -60.0, -88.0, 0.0)
        .value;
    assert!((bar.height_offset() - expected).abs() < 1e-8);
    bar.set_reduced_motion(true);
    window.update_layout();
    assert_eq!(-88.0, bar.height_offset());
    assert!(!clock.subject.has_observers());
    bar.set_reduced_motion(false);
    assert!(!clock.subject.has_observers());
}

#[test]
fn single_row_overlap_color_uses_default_effects_and_two_row_color_uses_oklab() {
    let (_app, clock) = application();
    let bar = app_bar(Variant::Small, false);
    let _window = window(&bar);
    bar.post_scroll(-20.0, 0.0);
    clock.subject.on_next(TimeSpan::from_seconds(0.0));
    clock.subject.on_next(TimeSpan::from_seconds(0.05));
    let a = bar
        .background()
        .unwrap()
        .as_solid_color_brush()
        .unwrap()
        .color();
    let b = bar
        .scrolled_background()
        .unwrap()
        .as_solid_color_brush()
        .unwrap()
        .color();
    let amount = MaterialMotionScheme::Expressive
        .spring(MaterialMotionRole::DefaultEffects)
        .sample(0.05, 0.0, 1.0, 0.0)
        .value;
    let surface: Ref<Border> = named(&bar, "PART_Container");
    assert_eq!(
        interpolate_container(a, b, amount),
        surface
            .background()
            .unwrap()
            .as_solid_color_brush()
            .unwrap()
            .color()
    );
    assert_ne!(
        Color::from_rgb(128, 128, 128),
        interpolate_container(
            Color::from_rgb(0, 0, 0),
            Color::from_rgb(255, 255, 255),
            0.5
        )
    );
}

#[test]
fn dynamic_subtitle_safe_insets_and_height_changes_keep_scroll_state_bounded() {
    let (_app, _) = application();
    let bar = app_bar(Variant::LargeFlexible, true);
    bar.set_safe_insets(Thickness::new(8.0, 24.0, 12.0, 0.0));
    let window = window(&bar);
    assert_eq!(144.0, bar.bounds().height);
    bar.set_subtitle(Some("Recent items".into()));
    window.update_layout();
    assert_eq!(176.0, bar.bounds().height);
    bar.set_height_offset(-80.0);
    bar.set_subtitle(None);
    window.update_layout();
    assert_eq!(-56.0, bar.height_offset());
    assert_eq!(88.0, bar.bounds().height);
}

#[test]
fn navigation_actions_keep_keyboard_and_invoke_semantics_while_titles_collapse() {
    let (_app, _) = application();
    let bar = app_bar(Variant::Large, true);
    let action = Button::new();
    action.set_content(Some(Rc::new("Search".to_owned())));
    bar.set_actions(Some(Control::boxed(&action)));
    let calls = Rc::new(Cell::new(0));
    let count = calls.clone();
    action.click(move |_, _| count.set(count.get() + 1));
    let window = window(&bar);
    bar.collapse();
    window.update_layout();
    assert!(action.focus_with(NavigationMethod::Tab, KeyModifiers::NONE));
    let peer = ControlAutomationPeer::create_peer_for_element(&action)
        .cast::<ButtonAutomationPeer>()
        .unwrap();
    peer.invoke().unwrap();
    assert_eq!(1, calls.get());
    assert_eq!("Search", peer.get_name());
}

#[test]
fn source_handlers_timers_and_clock_never_retain_a_detached_bar() {
    let (_app, clock) = application();
    let (window, bar, scroll) = scroll_host(Behavior::EnterAlways);
    bar.set_reduced_motion(false);
    bar.set_height_offset(-60.0);
    bar.settle(0.0);
    assert!(clock.subject.has_observers());
    let weak = bar.downgrade();
    window.set_content(None);
    window.update_layout();
    assert!(
        !bar.has_clock_subscription(),
        "the bar must dispose its own observer even while ScrollViewer transitions are alive"
    );
    drop(bar);
    window.close();
    ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);
    assert!(weak.upgrade().is_none());
    scroll.set_offset(Vector::new(0.0, 20.0));
}
