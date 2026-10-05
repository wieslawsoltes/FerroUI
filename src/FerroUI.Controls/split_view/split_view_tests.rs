//! The reference tests that show a window run in the test application with
//! the styled window services and a mock global clock; here the theme is the
//! test theme, whose split view theme mirrors the reference simple theme.

use crate::test_support::{test_scope, TestScope};
use crate::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};
use crate::{
    Border, Button, Control, GridLength, Panel, SplitView, SplitViewDisplayMode, SplitViewPanePlacement, TopLevel,
    Window,
};
use ferroui_base::animation::{IClock, IGlobalClock, PlayState, TimeSpan};
use ferroui_base::input::{
    InputElement, IPointer, Key, KeyEventArgs, KeyModifiers, MouseButton, Pointer, PointerPointProperties,
    PointerReleasedEventArgs, PointerType,
};
use ferroui_base::interactivity::{Interactive, RoutedEventArgs};
use ferroui_base::layout::{HorizontalAlignment, ILayoutManager, VerticalAlignment};
use ferroui_base::media::text_formatting::testing::TextTestScope;
use ferroui_base::platform::IPlatformRenderInterface;
use ferroui_base::reactive::{IDisposable, IObservable, IObserver, LightweightSubject};
use ferroui_base::{FerroLocator, LocatorExtensions, Point, Rect, Ref, Size, Visual};
use std::cell::Cell;
use std::rc::Rc;

/// The global clock of a test: never pulses.
struct MockGlobalClock {
    subject: LightweightSubject<TimeSpan>,
    play_state: Cell<PlayState>,
}

impl IObservable<TimeSpan> for MockGlobalClock {
    fn subscribe(&self, observer: Rc<dyn IObserver<TimeSpan>>) -> Rc<dyn IDisposable> {
        self.subject.subscribe(observer)
    }
}

impl IClock for MockGlobalClock {
    fn play_state(&self) -> PlayState {
        self.play_state.get()
    }

    fn set_play_state(&self, value: PlayState) {
        self.play_state.set(value)
    }
}

impl IGlobalClock for MockGlobalClock {}

/// The test application and the text services it runs with.
struct TestApplication {
    _app: UnitTestApplicationScope,
    _text: TextTestScope,
}

/// Starts the test application with the styled window services and a mock
/// global clock.
fn start() -> TestApplication {
    let text = TextTestScope::new();
    let render_interface = FerroLocator::current()
        .get_service::<dyn IPlatformRenderInterface>()
        .expect("the text services have a render interface");
    let clock: Rc<dyn IGlobalClock> =
        Rc::new(MockGlobalClock { subject: LightweightSubject::new(), play_state: Cell::new(PlayState::Run) });
    let app = UnitTestApplication::start(
        TestServices::styled_window().with_global_clock(clock).with_render_interface(render_interface),
    );
    TestApplication { _app: app, _text: text }
}

fn scoped() -> TestScope {
    test_scope()
}

fn window() -> Ref<Window> {
    let window = Window::new();
    window.set_width(1280.0);
    window.set_height(720.0);
    window
}

fn show(window: &Ref<Window>, split_view: &Ref<SplitView>) {
    window.set_content(Some(Control::boxed(split_view.clone())));
    window.show();
}

/// Raises a pointer released event on `target` whose source is `source`.
fn raise_pointer_released(target: &Interactive, source: &Ref<SplitView>, window: &Ref<Window>, position: Point) {
    let pointer: Rc<dyn IPointer> = Pointer::new(Pointer::get_next_free_id(), PointerType::Mouse, true);
    let root: Ref<Visual> = window.clone().upcast();
    target.raise_event(&PointerReleasedEventArgs::new(
        source.clone().upcast::<Interactive>(),
        pointer,
        &root,
        position,
        0,
        PointerPointProperties::default(),
        KeyModifiers::NONE,
        MouseButton::Left,
    ));
}

fn raise_escape_key_down(target: &Interactive) {
    let mut e = KeyEventArgs::new();
    e.key = Key::Escape;
    e.set_routed_event(Some(InputElement::key_down_event()));
    target.raise_event(&e);
}

#[test]
fn split_view_pane_opening_should_fire_before_pane_opened() {
    let _scope = scoped();
    let split_view = SplitView::new();

    let handled_opening = Rc::new(Cell::new(false));
    split_view.pane_opening({
        let handled_opening = handled_opening.clone();
        move |_, _| handled_opening.set(true)
    });

    split_view.pane_opened({
        let handled_opening = handled_opening.clone();
        move |_, _| assert!(handled_opening.get())
    });

    split_view.set_is_pane_open(true);
}

#[test]
fn split_view_pane_closing_should_fire_before_pane_closed() {
    let _scope = scoped();
    let split_view = SplitView::new();
    split_view.set_is_pane_open(true);

    let handled_closing = Rc::new(Cell::new(false));
    split_view.pane_closing({
        let handled_closing = handled_closing.clone();
        move |_, _| handled_closing.set(true)
    });

    split_view.pane_closed({
        let handled_closing = handled_closing.clone();
        move |_, _| assert!(handled_closing.get())
    });

    split_view.set_is_pane_open(false);
}

#[test]
fn split_view_cancel_close_should_prevent_pane_from_closing() {
    let _scope = scoped();
    let split_view = SplitView::new();
    split_view.set_is_pane_open(true);

    split_view.pane_closing(|_, e| e.set_cancel(true));

    split_view.set_is_pane_open(false);

    assert!(split_view.is_pane_open());
}

#[test]
fn split_view_template_settings_are_correct_for_display_modes() {
    let _app = start();
    let wnd = window();
    let split_view = SplitView::new();
    show(&wnd, &split_view);

    let zero_grid_length = GridLength::from_pixels(0.0);
    let compact_length = split_view.compact_pane_length();
    let compact_grid_length = GridLength::from_pixels(compact_length);

    // Overlay is the default display mode.
    assert_eq!(0.0, split_view.template_settings().closed_pane_width());
    assert_eq!(zero_grid_length, split_view.template_settings().pane_column_grid_length());

    split_view.set_display_mode(SplitViewDisplayMode::CompactOverlay);
    assert_eq!(compact_length, split_view.template_settings().closed_pane_width());
    assert_eq!(compact_grid_length, split_view.template_settings().pane_column_grid_length());

    split_view.set_display_mode(SplitViewDisplayMode::Inline);
    assert_eq!(0.0, split_view.template_settings().closed_pane_width());
    assert_eq!(GridLength::AUTO, split_view.template_settings().pane_column_grid_length());

    split_view.set_display_mode(SplitViewDisplayMode::CompactInline);
    assert_eq!(compact_length, split_view.template_settings().closed_pane_width());
    assert_eq!(GridLength::AUTO, split_view.template_settings().pane_column_grid_length());
}

#[test]
fn split_view_template_settings_update_with_compact_pane_length() {
    let _scope = scoped();
    let split_view = SplitView::new();

    // CompactInline:
    //    - ClosedPaneWidth = CompactPaneLength
    //    - PaneColumnGridLength = Auto
    split_view.set_display_mode(SplitViewDisplayMode::CompactInline);

    let compact_length = split_view.compact_pane_length();

    assert_eq!(GridLength::AUTO, split_view.template_settings().pane_column_grid_length());
    assert_eq!(compact_length, split_view.template_settings().closed_pane_width());

    split_view.set_compact_pane_length(100.0);

    assert_eq!(GridLength::AUTO, split_view.template_settings().pane_column_grid_length());
    assert_eq!(100.0, split_view.template_settings().closed_pane_width());

    // CompactOverlay:
    //    - ClosedPaneWidth = CompactPaneLength
    //    - PaneColumnGridLength = GridLength { CompactPaneLength, Pixel }
    split_view.set_display_mode(SplitViewDisplayMode::CompactOverlay);
    split_view.set_compact_pane_length(50.0);

    assert_eq!(GridLength::from_pixels(50.0), split_view.template_settings().pane_column_grid_length());
    assert_eq!(50.0, split_view.template_settings().closed_pane_width());

    // Value shouldn't change for these - changing the display mode will
    // update the template settings with the right value.
    split_view.set_display_mode(SplitViewDisplayMode::Inline);
    split_view.set_compact_pane_length(1.0);

    assert_eq!(GridLength::AUTO, split_view.template_settings().pane_column_grid_length());
    assert_eq!(0.0, split_view.template_settings().closed_pane_width());

    split_view.set_display_mode(SplitViewDisplayMode::Overlay);
    split_view.set_compact_pane_length(2.0);

    assert_eq!(GridLength::from_pixels(0.0), split_view.template_settings().pane_column_grid_length());
    assert_eq!(0.0, split_view.template_settings().closed_pane_width());
}

#[test]
fn split_view_pointer_closes_pane_in_overlay_mode() {
    let _app = start();
    let wnd = window();
    let split_view = SplitView::new();
    show(&wnd, &split_view);

    split_view.set_is_pane_open(true);

    raise_pointer_released(&split_view, &split_view, &wnd, Point::new(1270.0, 30.0));

    assert!(!split_view.is_pane_open());

    // Inline shouldn't close the pane.
    split_view.set_display_mode(SplitViewDisplayMode::Inline);
    split_view.set_is_pane_open(true);

    raise_pointer_released(&split_view, &split_view, &wnd, Point::new(1270.0, 30.0));

    assert!(split_view.is_pane_open());
}

#[test]
fn split_view_pointer_should_not_close_pane_if_over_pane() {
    let _app = start();
    let wnd = window();
    let click_border = Border::new();
    click_border.set_width(100.0);
    click_border.set_height(100.0);
    click_border.set_horizontal_alignment(HorizontalAlignment::Left);
    click_border.set_vertical_alignment(VerticalAlignment::Top);
    let split_view = SplitView::new();
    split_view.set_pane(Some(Control::boxed(click_border.clone())));
    show(&wnd, &split_view);

    split_view.set_is_pane_open(true);

    raise_pointer_released(&click_border, &split_view, &wnd, Point::new(5.0, 5.0));

    assert!(split_view.is_pane_open());
}

#[test]
fn split_view_escape_key_closes_light_dismissable_pane() {
    let _app = start();
    let wnd = window();
    let button = Button::new();
    let split_view = SplitView::new();
    split_view.set_pane(Some(Control::boxed(button.clone())));
    show(&wnd, &split_view);

    split_view.set_is_pane_open(true);

    raise_escape_key_down(&button);

    assert!(!split_view.is_pane_open());

    split_view.set_display_mode(SplitViewDisplayMode::Inline);

    split_view.set_is_pane_open(true);

    raise_escape_key_down(&button);

    assert!(split_view.is_pane_open());
}

#[test]
fn top_level_back_requested_closes_light_dismissable_pane() {
    let _app = start();
    let wnd = window();
    let split_view = SplitView::new();
    show(&wnd, &split_view);

    split_view.set_is_pane_open(true);

    wnd.raise_event(&RoutedEventArgs::with_event(TopLevel::back_requested_event()));

    assert!(!split_view.is_pane_open());

    split_view.set_display_mode(SplitViewDisplayMode::Inline);
    split_view.set_is_pane_open(true);

    wnd.raise_event(&RoutedEventArgs::with_event(TopLevel::back_requested_event()));

    assert!(split_view.is_pane_open());
}

#[test]
fn top_level_back_requested_should_not_be_handled_when_pane_is_closed() {
    for display_mode in [SplitViewDisplayMode::Overlay, SplitViewDisplayMode::CompactOverlay] {
        let _app = start();
        let wnd = window();
        let split_view = SplitView::new();
        split_view.set_display_mode(display_mode);
        show(&wnd, &split_view);

        // Pane is closed: the split view must ignore the event so back
        // navigation can proceed.
        assert!(!split_view.is_pane_open());

        let closed_args = RoutedEventArgs::with_event(TopLevel::back_requested_event());
        wnd.raise_event(&closed_args);

        assert!(!closed_args.handled());

        // Pane is open: the split view should close it and handle the event.
        split_view.set_is_pane_open(true);

        let open_args = RoutedEventArgs::with_event(TopLevel::back_requested_event());
        wnd.raise_event(&open_args);

        assert!(open_args.handled());
        assert!(!split_view.is_pane_open());
    }
}

#[test]
fn with_default_is_pane_open_value_should_have_closed_pseudo_class_set() {
    // Testing the pseudoclasses of this control requires placing the split
    // view on a window prior to asserting them, because some of the
    // pseudoclasses are set either when the template is applied or the
    // control is attached to the visual tree.
    let _app = start();
    let wnd = window();
    let split_view = SplitView::new();
    show(&wnd, &split_view);

    assert!(split_view.classes().contains(":closed"));
}

#[test]
fn split_view_shouldnt_close_panel_when_is_pane_open_true_then_display_mode_changed() {
    let _app = start();
    let wnd = window();
    let split_view = SplitView::new();
    split_view.set_display_mode(SplitViewDisplayMode::CompactOverlay);
    show(&wnd, &split_view);

    split_view.set_is_pane_open(true);

    raise_pointer_released(&split_view, &split_view, &wnd, Point::new(1270.0, 30.0));

    assert!(!split_view.is_pane_open());

    // Inline shouldn't close the pane.
    split_view.set_is_pane_open(true);

    // Change the display mode once the pane is already open.
    split_view.set_display_mode(SplitViewDisplayMode::Inline);

    raise_pointer_released(&split_view, &split_view, &wnd, Point::new(1270.0, 30.0));

    assert!(split_view.is_pane_open());
}

// ---------------------------------------------------------------------------
// Additional tests of this port (not present in the reference test file).
// ---------------------------------------------------------------------------

#[test]
fn types_are_registered_under_the_reference_namespaces() {
    crate::register_types();

    assert_eq!("FerroUI.Controls.SplitView", SplitView::TYPE.full_name());
    assert_eq!(
        "FerroUI.Controls.Primitives.SplitViewTemplateSettings",
        crate::primitives::SplitViewTemplateSettings::TYPE.full_name()
    );
}

/// The bounds of the pane root and the content root of the template, in the
/// coordinates of the split view (the grid of the template fills it).
fn part_bounds(split_view: &SplitView) -> (Rect, Rect) {
    let part = |name: &str| {
        split_view
            .get_visual_descendants()
            .filter_map(|x| x.cast::<Panel>())
            .find(|x| x.name().as_deref() == Some(name))
            .unwrap_or_else(|| panic!("no {name}"))
    };
    (part("PART_PaneRoot").bounds(), part("ContentRoot").bounds())
}

/// Shows a split view of 1280 x 720 with the placement, without the pane
/// animation of the theme, and returns the bounds of the pane root and the
/// content root for the display mode and the pane state.
fn arranged(
    placement: SplitViewPanePlacement,
    display_mode: SplitViewDisplayMode,
    is_pane_open: bool,
) -> (Rect, Rect) {
    let _app = start();
    let wnd = window();
    let split_view = SplitView::new();
    split_view.set_pane_placement(placement);
    split_view.set_display_mode(display_mode);
    show(&wnd, &split_view);

    let pane_root = split_view
        .get_visual_descendants()
        .filter_map(|x| x.cast::<Panel>())
        .find(|x| x.name().as_deref() == Some("PART_PaneRoot"))
        .expect("no pane root");
    pane_root.set_transitions(None);

    split_view.set_is_pane_open(is_pane_open);
    wnd.layout_manager().execute_layout_pass();

    assert_eq!(Size::new(1280.0, 720.0), split_view.bounds().size());
    part_bounds(&split_view)
}

const PLACEMENTS: [SplitViewPanePlacement; 4] = [
    SplitViewPanePlacement::Left,
    SplitViewPanePlacement::Right,
    SplitViewPanePlacement::Top,
    SplitViewPanePlacement::Bottom,
];

/// A pane of the given length at the edge of the placement.
fn pane_rect(placement: SplitViewPanePlacement, length: f64) -> Rect {
    match placement {
        SplitViewPanePlacement::Left => Rect::new(0.0, 0.0, length, 720.0),
        SplitViewPanePlacement::Right => Rect::new(1280.0 - length, 0.0, length, 720.0),
        SplitViewPanePlacement::Top => Rect::new(0.0, 0.0, 1280.0, length),
        SplitViewPanePlacement::Bottom => Rect::new(0.0, 720.0 - length, 1280.0, length),
    }
}

/// The rest of the split view next to a pane of the given length.
fn content_rect(placement: SplitViewPanePlacement, pane_length: f64) -> Rect {
    match placement {
        SplitViewPanePlacement::Left => Rect::new(pane_length, 0.0, 1280.0 - pane_length, 720.0),
        SplitViewPanePlacement::Right => Rect::new(0.0, 0.0, 1280.0 - pane_length, 720.0),
        SplitViewPanePlacement::Top => Rect::new(0.0, pane_length, 1280.0, 720.0 - pane_length),
        SplitViewPanePlacement::Bottom => Rect::new(0.0, 0.0, 1280.0, 720.0 - pane_length),
    }
}

#[test]
fn inline_pane_is_arranged_next_to_the_content_for_every_placement() {
    for placement in PLACEMENTS {
        let (pane, content) = arranged(placement, SplitViewDisplayMode::Inline, true);
        assert_eq!(pane_rect(placement, 320.0), pane, "{placement:?}");
        assert_eq!(content_rect(placement, 320.0), content, "{placement:?}");

        let (pane, content) = arranged(placement, SplitViewDisplayMode::Inline, false);
        assert_eq!(pane_rect(placement, 0.0).size(), pane.size(), "{placement:?}");
        assert_eq!(content_rect(placement, 0.0), content, "{placement:?}");
    }
}

#[test]
fn compact_inline_pane_is_arranged_next_to_the_content_for_every_placement() {
    for placement in PLACEMENTS {
        let (pane, content) = arranged(placement, SplitViewDisplayMode::CompactInline, true);
        assert_eq!(pane_rect(placement, 320.0), pane, "{placement:?}");
        assert_eq!(content_rect(placement, 320.0), content, "{placement:?}");

        let (pane, content) = arranged(placement, SplitViewDisplayMode::CompactInline, false);
        assert_eq!(pane_rect(placement, 48.0), pane, "{placement:?}");
        assert_eq!(content_rect(placement, 48.0), content, "{placement:?}");
    }
}

#[test]
fn compact_overlay_pane_keeps_the_content_next_to_the_compact_pane_for_every_placement() {
    for placement in PLACEMENTS {
        let (pane, content) = arranged(placement, SplitViewDisplayMode::CompactOverlay, false);
        assert_eq!(pane_rect(placement, 48.0), pane, "{placement:?}");
        assert_eq!(content_rect(placement, 48.0), content, "{placement:?}");

        // The open pane lies over the content, which stays where it is.
        let (pane, content) = arranged(placement, SplitViewDisplayMode::CompactOverlay, true);
        assert_eq!(pane_rect(placement, 320.0), pane, "{placement:?}");
        assert_eq!(content_rect(placement, 48.0), content, "{placement:?}");
    }
}

#[test]
fn overlay_pane_leaves_the_whole_split_view_to_the_content_for_every_placement() {
    for placement in PLACEMENTS {
        let (pane, content) = arranged(placement, SplitViewDisplayMode::Overlay, false);
        assert_eq!(pane_rect(placement, 0.0).size(), pane.size(), "{placement:?}");
        assert_eq!(content_rect(placement, 0.0), content, "{placement:?}");

        let (pane, content) = arranged(placement, SplitViewDisplayMode::Overlay, true);
        assert_eq!(pane_rect(placement, 320.0), pane, "{placement:?}");
        assert_eq!(content_rect(placement, 0.0), content, "{placement:?}");
    }
}
