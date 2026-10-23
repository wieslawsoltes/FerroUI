use crate::platform::{
    FeedbackAction, FeedbackType, IPlatformFeedback, IPopupImpl, ITopLevelImpl, PlatformFeedback,
};
use crate::primitives::popup_positioning::{
    IPopupPositioner, PopupGravity, PopupPositionerParameters,
};
use crate::primitives::FlyoutBaseImplExt;
use crate::testing::{
    MockWindowingPlatform, TestServices, UnitTestApplication, UnitTestApplicationScope,
};
use crate::{
    Border, ContextMenu, ContextMenuEventArgs, Control, Flyout, FlyoutShowMode, MenuBaseImplExt,
    PlacementMode, RichEditBox, StackPanel, Window,
};
use ferroui_base::animation::TimeSpan;
use ferroui_base::input::platform::PlatformHotkeyConfiguration;
use ferroui_base::input::text_input::{
    ITextInputMethodImpl, TextInputMethodClient, TextInputMethodConnection, TextInputOptions,
};
use ferroui_base::input::{
    ContextRequestedEventArgs, IPointer, InputElement, Key, KeyEventArgs, KeyGesture, KeyModifiers,
    MouseButton, Pointer, PointerPointProperties, PointerPressedEventArgs,
    PointerReleasedEventArgs, PointerType, PointerUpdateKind, RawInputModifiers,
};
use ferroui_base::interactivity::{IRoutedEventArgs, RoutingStrategies};
use ferroui_base::media::text_formatting::testing::{
    TextTestScope, DESIGN_EM_HEIGHT, GLYPH_ADVANCE,
};
use ferroui_base::media::{FlowDirection, TextWrapping};
use ferroui_base::threading::Dispatcher;
use ferroui_base::{FerroLocator, Point, Rect, Ref, Thickness};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[path = "rich_edit_input_focus_tests.rs"]
mod input_focus_tests;

#[derive(Default)]
struct RecordingInputMethod {
    client: RefCell<Option<Rc<dyn TextInputMethodClient>>>,
    connection: RefCell<Option<Rc<TextInputMethodConnection>>>,
    resets: Cell<usize>,
}

impl ITextInputMethodImpl for RecordingInputMethod {
    fn set_client(&self, client: Option<Rc<dyn TextInputMethodClient>>) {
        *self.connection.borrow_mut() = client
            .as_ref()
            .and_then(|client| client.input_method_connection());
        *self.client.borrow_mut() = client;
    }
    fn set_cursor_rect(&self, _: Rect) {}
    fn set_options(&self, _: &TextInputOptions) {}
    fn reset(&self) {
        self.resets.set(self.resets.get() + 1);
        let client = self.client.borrow().clone();
        if let Some(client) = client {
            client.set_preedit_text(None);
        }
    }
}

#[derive(Default)]
struct RecordingFeedback(RefCell<Vec<(FeedbackAction, FeedbackType)>>);

impl IPlatformFeedback for RecordingFeedback {
    fn perform(&self, action: FeedbackAction, kind: FeedbackType) -> bool {
        self.0.borrow_mut().push((action, kind));
        true
    }
}

struct RecordingPositioner {
    inner: Rc<dyn IPopupPositioner>,
    requests: Rc<RefCell<Vec<PopupPositionerParameters>>>,
}

impl IPopupPositioner for RecordingPositioner {
    fn update(&self, parameters: PopupPositionerParameters) {
        self.requests.borrow_mut().push(parameters);
        self.inner.update(parameters);
    }
}

struct Fixture {
    window: Ref<Window>,
    panel: Ref<StackPanel>,
    owner: Ref<RichEditBox>,
    native: Rc<RecordingInputMethod>,
    feedback: Rc<RecordingFeedback>,
    positions: Rc<RefCell<Vec<PopupPositionerParameters>>>,
    _text: TextTestScope,
    _app: UnitTestApplicationScope,
}

impl Fixture {
    fn new(overlay: bool) -> Self {
        let native = Rc::new(RecordingInputMethod::default());
        let feedback = Rc::new(RecordingFeedback::default());
        let positions = Rc::new(RefCell::new(Vec::new()));
        let method = native.clone();
        let native_feedback = feedback.clone();
        let native_positions = positions.clone();
        let app =
            UnitTestApplication::start(TestServices::focusable_window().with_windowing_platform(
                MockWindowingPlatform::with_window_impl(move || {
                    let window = MockWindowingPlatform::create_window_mock();
                    window.render_scaling.set(2.0);
                    window.desktop_scaling.set(2.0);
                    window.setup_feature::<dyn ITextInputMethodImpl>(method.clone());
                    window.setup_feature::<dyn IPlatformFeedback>(native_feedback.clone());
                    let weak = Rc::downgrade(&window);
                    let requests = native_positions.clone();
                    window.setup_create_popup(move |_| {
                        if overlay {
                            None
                        } else {
                            let parent: Rc<dyn ITopLevelImpl> = weak.upgrade()?;
                            let popup = MockWindowingPlatform::create_popup_mock(parent);
                            let inner = IPopupImpl::popup_positioner(&*popup).unwrap();
                            popup.setup_popup_positioner(Some(Rc::new(RecordingPositioner {
                                inner,
                                requests: requests.clone(),
                            })));
                            Some(popup as Rc<dyn IPopupImpl>)
                        }
                    });
                    window
                }),
            ));
        // Win32 registers Shift+F10 alongside the generic Apps key gesture.
        // Exercise both through the same configured Control keyboard route.
        let mut hotkeys = PlatformHotkeyConfiguration::default();
        hotkeys
            .open_context_menu
            .push(KeyGesture::new(Key::F10, KeyModifiers::SHIFT));
        FerroLocator::current_mutable()
            .bind::<PlatformHotkeyConfiguration>()
            .to_constant(Rc::new(hotkeys));
        let text = TextTestScope::new();
        let owner = RichEditBox::new();
        owner.set_template(Some(crate::rich_edit_box_tests::rich_edit_box_template()));
        owner.set_font_size(12.0);
        owner.set_caret_blink_interval(TimeSpan::ZERO);
        owner.set_width(240.0);
        owner.set_height(100.0);
        owner.set_margin(Thickness::new(31.0, 19.0, 0.0, 0.0));
        owner.set_padding(Thickness::new(13.0, 7.0, 9.0, 5.0));
        owner.set_text("alpha beta\ngamma delta").unwrap();
        owner.select(2, 17);
        owner.editor().clear_history();
        let panel = StackPanel::new();
        panel.children().add(owner.clone());
        let window = Window::new();
        window.set_width(500.0);
        window.set_height(300.0);
        window.set_content(Some(Control::boxed(panel.clone())));
        window.show();
        owner.focus();
        window.update_layout();
        Dispatcher::ui_thread().run_jobs(None);
        assert!(owner.is_focused());
        Self {
            window,
            panel,
            owner,
            native,
            feedback,
            positions,
            _text: text,
            _app: app,
        }
    }

    fn flyout(&self) -> Ref<Flyout> {
        let content = Border::new();
        content.set_width(50.0);
        content.set_height(24.0);
        let flyout = Flyout::new();
        flyout.set_content(Some(Control::boxed(content)));
        flyout.set_show_mode(FlyoutShowMode::Transient);
        self.owner.set_context_flyout(&flyout);
        flyout
    }

    fn layout(&self) {
        self.window.update_layout();
        Dispatcher::ui_thread().run_jobs(None);
        self.window.update_layout();
    }

    fn root_point(&self, point: Point) -> Point {
        point.transform(self.owner.transform_to_visual(&self.window).unwrap())
    }

    fn keyboard(&self, key: Key, modifiers: KeyModifiers) -> KeyEventArgs {
        let mut args = KeyEventArgs::new();
        args.key = key;
        args.key_modifiers = modifiers;
        args.set_routed_event(Some(InputElement::key_up_event()));
        self.owner.raise_event(&args);
        args
    }

    fn right_release(&self, point: Point) -> PointerReleasedEventArgs {
        let source = self.owner.text_presenter();
        let args = PointerReleasedEventArgs::new(
            source.clone(),
            Pointer::new(Pointer::get_next_free_id(), PointerType::Mouse, true),
            &self.window,
            point,
            1,
            PointerPointProperties::new(
                RawInputModifiers::NONE,
                PointerUpdateKind::RightButtonReleased,
            ),
            KeyModifiers::NONE,
            MouseButton::Right,
        );
        source.raise_event(&args);
        args
    }

    fn assert_anchor(&self, expected: Point) {
        self.layout();
        let requests = self.positions.borrow();
        let last = requests
            .last()
            .expect("the open native popup was positioned");
        assert_eq!(
            last.anchor_rectangle,
            Rect::new(expected.x, expected.y, 0.0, 0.0)
        );
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.window.close();
    }
}

#[test]
fn rich_context_args_share_handled_and_request_anchors_have_weak_owners() {
    let args = ContextMenuEventArgs::new(12.25, 43.75);
    let shared = args.share();
    assert_eq!(args.cursor_left(), 12.25);
    assert_eq!(args.cursor_top(), 43.75);
    shared.set_handled(true);
    assert!(args.handled());
    assert_eq!(ContextMenuEventArgs::default().cursor_left(), 0.0);
    assert_eq!(ContextMenuEventArgs::default().cursor_top(), 0.0);

    let owner = Border::new();
    let weak = owner.downgrade();
    let request = ContextRequestedEventArgs::new();
    let copied = ContextRequestedEventArgs::from_context_requested_event_args(&request);
    request.set_placement_rect(&owner, Rect::new(3.25, 7.5, 0.0, 0.0));
    assert_eq!(copied.try_get_position(None), None);
    assert_eq!(
        copied.try_get_placement_rect(&owner),
        Some(Rect::new(3.25, 7.5, 0.0, 0.0))
    );
    copied.clear_placement_rect();
    assert_eq!(request.try_get_placement_rect(&owner), None);
    request.set_placement_rect(&owner, Rect::new(1.0, 2.0, 0.0, 0.0));
    drop(owner);
    assert!(weak.upgrade().is_none());
    assert_eq!(request.try_get_placement_rect(&Border::new()), None);
}

#[test]
fn rich_context_pointer_reports_root_dips_once_and_handled_cancels_both_menu_kinds() {
    let f = Fixture::new(false);
    let flyout = f.flyout();
    let menu = ContextMenu::new();
    f.owner.set_context_menu(&menu);
    let calls = Rc::new(RefCell::new(Vec::new()));
    let observed = calls.clone();
    let owner = f.owner.downgrade();
    f.owner.context_menu_opening(move |sender, args| {
        let owner = owner.upgrade().unwrap();
        assert_eq!(
            sender.to_ref(),
            owner
                .clone()
                .upcast::<ferroui_base::interactivity::Interactive>()
        );
        assert_eq!(args.source(), Some(owner.upcast()));
        observed
            .borrow_mut()
            .push(Point::new(args.cursor_left(), args.cursor_top()));
        args.set_handled(true);
    });
    let point = Point::new(170.25, 57.75);
    assert!(f.right_release(point).handled());
    assert_eq!(&*calls.borrow(), &[point]);
    assert!(!flyout.is_open());
    assert!(!menu.is_open());
    assert!(f.positions.borrow().is_empty());
    assert!(!f.owner.can_undo());
}

#[test]
fn rich_context_pointer_opens_at_request_point_without_changing_flyout_placement() {
    let f = Fixture::new(false);
    let flyout = f.flyout();
    flyout.set_placement(PlacementMode::Top);
    let weak = flyout.downgrade();
    f.owner
        .context_menu_opening(move |_, _| assert!(!weak.upgrade().unwrap().is_open()));
    let point = f.root_point(Point::new(35.25, 38.75));
    assert!(f.right_release(point).handled());
    assert!(flyout.is_open());
    assert_eq!(flyout.placement(), PlacementMode::Top);
    assert_eq!(
        flyout.popup().placement(),
        PlacementMode::BottomEdgeAlignedLeft
    );
    f.assert_anchor(point);
    flyout.hide();
    flyout.show_at(&f.owner);
    assert_eq!(flyout.popup().placement(), PlacementMode::Top);
    assert_eq!(flyout.popup().placement_rect(), None);
}

#[test]
fn rich_context_repeated_request_repositions_the_existing_owned_popup() {
    let f = Fixture::new(false);
    let flyout = f.flyout();
    let first = f.root_point(Point::new(30.5, 25.5));
    let second = f.root_point(Point::new(80.5, 35.5));
    assert!(f.right_release(first).handled());
    f.assert_anchor(first);
    assert!(f.right_release(second).handled());
    assert!(flyout.is_open());
    f.assert_anchor(second);
}

#[test]
fn rich_context_ordinary_control_requests_keep_pointer_and_keyboard_defaults() {
    let f = Fixture::new(false);
    let flyout = f.flyout();
    assert!(f.keyboard(Key::Apps, KeyModifiers::NONE).handled());
    flyout.hide();
    let ordinary = Border::new();
    ordinary.set_height(40.0);
    ordinary.set_context_flyout(&flyout);
    f.panel.children().add(ordinary.clone());
    f.layout();
    flyout.set_placement(PlacementMode::Top);
    ordinary.raise_event(&ContextRequestedEventArgs::new());
    assert!(flyout.is_open());
    assert_eq!(flyout.popup().placement(), PlacementMode::Top);
    assert_eq!(flyout.popup().placement_rect(), None);
    flyout.hide();
    ordinary.raise_event(&PointerReleasedEventArgs::new(
        ordinary.clone(),
        Pointer::new(Pointer::get_next_free_id(), PointerType::Mouse, true),
        &f.window,
        Point::new(20.0, 150.0),
        1,
        PointerPointProperties::new(
            RawInputModifiers::NONE,
            PointerUpdateKind::RightButtonReleased,
        ),
        KeyModifiers::NONE,
        MouseButton::Right,
    ));
    assert!(flyout.is_open());
    assert_eq!(flyout.popup().placement(), PlacementMode::Pointer);
    assert_eq!(flyout.popup().placement_rect(), None);
}

#[test]
fn rich_context_keyboard_uses_normalized_selection_edges_and_rtl_popup_gravity() {
    let f = Fixture::new(false);
    let flyout = f.flyout();
    let calls = Rc::new(RefCell::new(Vec::new()));
    let observed = calls.clone();
    f.owner.context_menu_opening(move |_, args| {
        observed
            .borrow_mut()
            .push(Point::new(args.cursor_left(), args.cursor_top()));
    });
    for direction in [FlowDirection::LeftToRight, FlowDirection::RightToLeft] {
        f.owner.set_flow_direction(direction);
        for (anchor, active, key, modifiers) in [
            (2, 17, Key::Apps, KeyModifiers::NONE),
            (17, 2, Key::F10, KeyModifiers::SHIFT),
        ] {
            f.owner.select(anchor, active);
            f.layout();
            let presenter = f.owner.text_presenter();
            let lines = presenter.line_metrics();
            assert_eq!(lines.len(), 2, "the fixture crosses a paragraph boundary");
            assert_eq!((lines[0].start, lines[1].start), (0, 11));
            let advance =
                f.owner.font_size() * f64::from(GLYPH_ADVANCE) / f64::from(DESIGN_EM_HEIGHT);
            assert_eq!(lines[0].bounds.width, 10.0 * advance);
            // The selected Latin source starts at column 2 of the first line
            // and ends at column 6 of the second. Pick that logical edge in
            // the view, then transform the point; reflection changes which
            // corner of an owner-space bounding box contains the same edge.
            let local = if direction == FlowDirection::RightToLeft {
                Point::new(lines[0].bounds.x + 2.0 * advance, lines[0].bounds.bottom())
            } else {
                Point::new(lines[1].bounds.x + 6.0 * advance, lines[1].bounds.bottom())
            };
            let expected = local.transform(presenter.transform_to_visual(&f.window).unwrap());
            assert!(f.keyboard(key, modifiers).handled());
            assert!(flyout.is_open());
            assert_eq!(calls.borrow().last(), Some(&expected), "{direction:?}");
            f.assert_anchor(expected);
            let gravity = f.positions.borrow().last().unwrap().gravity();
            assert_eq!(
                gravity,
                if direction == FlowDirection::RightToLeft {
                    PopupGravity::BOTTOM_LEFT
                } else {
                    PopupGravity::BOTTOM_RIGHT
                }
            );
            assert_eq!(
                (f.owner.selection_start(), f.owner.selection_end()),
                (anchor, active)
            );
            flyout.hide();
        }
    }
    assert_eq!(calls.borrow().len(), 4);
    assert!(!f.owner.can_undo());
}

#[test]
fn rich_context_collapsed_keyboard_request_tracks_the_shaped_caret() {
    let f = Fixture::new(false);
    let flyout = f.flyout();
    for direction in [FlowDirection::LeftToRight, FlowDirection::RightToLeft] {
        f.owner.set_flow_direction(direction);
        f.owner.select(14, 14);
        f.layout();
        let presenter = f.owner.text_presenter();
        let caret = presenter.cursor_rectangle();
        let point = if direction == FlowDirection::RightToLeft {
            caret.bottom_left()
        } else {
            caret.bottom_right()
        };
        let expected = point.transform(presenter.transform_to_visual(&f.window).unwrap());
        assert!(f.keyboard(Key::Apps, KeyModifiers::NONE).handled());
        assert!(flyout.is_open());
        f.assert_anchor(expected);
        flyout.hide();
    }
}

#[test]
fn rich_context_keyboard_selection_end_at_soft_wrap_stays_on_selected_line() {
    let f = Fixture::new(false);
    let flyout = f.flyout();
    f.owner
        .set_text("one two three four five six seven eight")
        .unwrap();
    f.owner.set_width(90.0);
    f.owner.set_text_wrapping(TextWrapping::Wrap);
    f.layout();
    let lines = f.owner.line_metrics();
    assert!(lines.len() > 1);
    let first = &lines[0];
    f.owner.select(0, first.start + first.length);
    f.layout();
    let expected = f.root_point(f.owner.selection_rects().last().unwrap().bottom_right());
    assert!(f.keyboard(Key::Apps, KeyModifiers::NONE).handled());
    assert!(flyout.is_open());
    f.assert_anchor(expected);
    assert!(expected.y <= f.root_point(lines[1].bounds.top_left()).y);
}

#[test]
fn rich_context_parent_handled_and_prehandled_context_requests_do_not_open() {
    let f = Fixture::new(false);
    let flyout = f.flyout();
    let calls = Rc::new(Cell::new(0));
    let observed = calls.clone();
    f.panel
        .add_handler(RichEditBox::context_menu_opening_event(), move |_, args| {
            observed.set(observed.get() + 1);
            args.set_handled(true);
        });
    assert!(f.keyboard(Key::Apps, KeyModifiers::NONE).handled());
    assert_eq!(calls.get(), 1);
    assert!(!flyout.is_open());
    let request = ContextRequestedEventArgs::new();
    request.set_handled(true);
    f.owner.raise_event(&request);
    assert_eq!(calls.get(), 1);
    assert!(!flyout.is_open());
}

#[test]
fn rich_context_opening_can_install_replace_or_remove_the_current_flyout() {
    for action in [0, 1, 2] {
        let f = Fixture::new(false);
        let replacement = f.flyout();
        f.owner.set_context_flyout(None);
        let previous = if action == 0 { None } else { Some(f.flyout()) };
        let owner = f.owner.downgrade();
        let accepted = replacement.downgrade();
        f.owner.context_menu_opening(move |_, _| {
            let owner = owner.upgrade().unwrap();
            if action == 2 {
                owner.set_context_flyout(None);
            } else {
                owner.set_context_flyout(
                    accepted
                        .upgrade()
                        .map(|flyout| flyout.upcast::<crate::primitives::FlyoutBase>()),
                );
            }
        });
        f.keyboard(Key::Apps, KeyModifiers::NONE);
        assert_eq!(replacement.is_open(), action != 2);
        assert!(previous.is_none_or(|previous| !previous.is_open()));
    }
}

#[test]
fn rich_context_nested_request_consumes_outer_opening_and_uses_new_selection() {
    let f = Fixture::new(false);
    let flyout = f.flyout();
    let calls = Rc::new(Cell::new(0));
    let observed = calls.clone();
    let owner = f.owner.downgrade();
    f.owner.context_menu_opening(move |_, _| {
        observed.set(observed.get() + 1);
        if observed.get() == 1 {
            let owner = owner.upgrade().unwrap();
            owner.select(3, 3);
            owner.raise_event(&ContextRequestedEventArgs::new());
        }
    });
    let opened = Rc::new(Cell::new(0));
    let observed = opened.clone();
    let _opened = flyout.opened(move || observed.set(observed.get() + 1));
    assert!(f.keyboard(Key::Apps, KeyModifiers::NONE).handled());
    assert_eq!(calls.get(), 2);
    assert_eq!(opened.get(), 1);
    let expected = f.root_point(f.owner.cursor_rectangle().bottom_right());
    f.assert_anchor(expected);
}

#[test]
fn rich_context_detach_and_reattach_during_opening_invalidates_the_request() {
    let f = Fixture::new(false);
    let flyout = f.flyout();
    let panel = f.panel.downgrade();
    let owner = f.owner.downgrade();
    f.owner.context_menu_opening(move |_, _| {
        let panel = panel.upgrade().unwrap();
        let owner = owner.upgrade().unwrap();
        panel.children().remove(&owner);
        panel.children().add(owner);
    });
    assert!(f.keyboard(Key::Apps, KeyModifiers::NONE).handled());
    assert!(!flyout.is_open());
    assert!(f.positions.borrow().is_empty());
}

#[test]
fn rich_context_menu_precedence_and_opening_cancellation_are_not_retried() {
    let f = Fixture::new(false);
    let flyout = f.flyout();
    let menu = ContextMenu::new();
    f.owner.set_context_menu(&menu);
    let calls = Rc::new(Cell::new(0));
    let observed = calls.clone();
    let _opening = menu.opening(move |args| {
        observed.set(observed.get() + 1);
        args.set_cancel(true);
    });
    assert!(f.keyboard(Key::Apps, KeyModifiers::NONE).handled());
    assert_eq!(calls.get(), 1);
    assert!(!menu.is_open());
    assert!(!flyout.is_open());
}

#[test]
fn rich_context_menu_uses_the_same_anchor_and_takes_precedence_over_the_flyout() {
    let f = Fixture::new(false);
    let flyout = f.flyout();
    let menu = ContextMenu::new();
    menu.set_width(80.0);
    menu.set_height(30.0);
    f.owner.set_context_menu(&menu);
    f.owner.select(3, 3);
    f.layout();
    let expected = f.root_point(f.owner.cursor_rectangle().bottom_right());
    assert!(f.keyboard(Key::Apps, KeyModifiers::NONE).handled());
    assert!(menu.is_open());
    assert!(!flyout.is_open());
    f.assert_anchor(expected);
    menu.close();
}

#[test]
fn rich_context_flyout_opening_cancellation_is_not_retried() {
    let f = Fixture::new(false);
    let flyout = f.flyout();
    let calls = Rc::new(Cell::new(0));
    let observed = calls.clone();
    let _opening = flyout.opening(move |args| {
        observed.set(observed.get() + 1);
        args.set_cancel(true);
    });
    assert!(f.keyboard(Key::Apps, KeyModifiers::NONE).handled());
    assert_eq!(calls.get(), 1);
    assert!(!flyout.is_open());
}

#[test]
fn rich_context_opening_preserves_live_composition_and_owned_popup_focus() {
    for overlay in [false, true] {
        let f = Fixture::new(overlay);
        let connection = f.native.connection.borrow().clone().unwrap();
        let session = connection.begin_composition().unwrap();
        session.update("candidate", Some(4)).unwrap();
        let client = f.owner.text_input_method_client();
        let revision = client.input_method_state_revision();
        let events = Rc::new(RefCell::new(Vec::new()));
        let observed = events.clone();
        let _selection = client.selection_changed(Rc::new(move || {
            observed.borrow_mut().push("selection");
        }));
        let observed = events.clone();
        let _text = client.surrounding_text_changed(Rc::new(move || {
            observed.borrow_mut().push("text");
        }));
        let snapshot = f.owner.document().snapshot();
        let selection = (f.owner.selection_start(), f.owner.selection_end());
        let resets = f.native.resets.get();
        let flyout = f.flyout();
        let focus = Border::new();
        focus.set_focusable(true);
        focus.set_width(50.0);
        focus.set_height(24.0);
        flyout.set_content(Some(Control::boxed(focus.clone())));
        flyout.set_show_mode(FlyoutShowMode::Standard);
        let canceled = f
            .owner
            .context_menu_opening(|_, args| args.set_handled(true));
        assert!(f.keyboard(Key::Apps, KeyModifiers::NONE).handled());
        assert!(!flyout.is_open());
        assert!(session.is_current());
        assert!(f.owner.is_focused());
        f.owner
            .remove_handler(RichEditBox::context_menu_opening_event(), canceled);
        assert!(f.keyboard(Key::Apps, KeyModifiers::NONE).handled());
        assert!(session.is_current(), "before layout; overlay={overlay}");
        f.layout();
        assert!(flyout.is_open());
        assert!(focus.is_focused());
        assert!(session.is_current(), "after layout; overlay={overlay}");
        assert!(connection.is_current());
        assert_eq!(f.native.resets.get(), resets);
        assert_eq!(f.owner.document().snapshot(), snapshot);
        assert_eq!(
            (f.owner.selection_start(), f.owner.selection_end()),
            selection
        );
        assert!(!f.owner.can_undo());
        flyout.hide();
        assert!(f.owner.is_focused());
        assert!(!focus.is_focused());
        assert!(session.is_current(), "after hide; overlay={overlay}");
        assert!(Rc::ptr_eq(
            &connection,
            f.native.connection.borrow().as_ref().unwrap()
        ));
        assert_eq!(client.input_method_state_revision(), revision);
        assert!(events.borrow().is_empty());
        assert_eq!(f.owner.document().snapshot(), snapshot);
        assert_eq!(f.native.resets.get(), resets);
        assert!(!f.owner.can_undo());
        assert!(session.cancel().unwrap());
    }
}

#[test]
fn rich_context_dismissal_focus_redirection_retires_the_original_composition() {
    for overlay in [false, true] {
        let f = Fixture::new(overlay);
        let outside = Border::new();
        outside.set_focusable(true);
        outside.set_height(30.0);
        f.panel.children().add(outside.clone());
        f.layout();
        let before = f.owner.document().snapshot();
        let connection = f.native.connection.borrow().clone().unwrap();
        let session = connection.begin_composition().unwrap();
        session.update("candidate", Some(4)).unwrap();
        let flyout = f.flyout();
        let focus = Border::new();
        focus.set_focusable(true);
        focus.set_width(50.0);
        focus.set_height(24.0);
        flyout.set_content(Some(Control::boxed(focus.clone())));
        flyout.set_show_mode(FlyoutShowMode::Standard);
        assert!(f.keyboard(Key::Apps, KeyModifiers::NONE).handled());
        assert!(focus.is_focused());
        assert!(session.is_current());
        let redirected = Rc::new(Cell::new(0));
        let observed = redirected.clone();
        let destination = outside.clone().upcast::<InputElement>();
        f.owner
            .add_handler(InputElement::getting_focus_event(), move |_, args| {
                assert!(args.try_set_new_focused_element(Some(&destination)));
                observed.set(observed.get() + 1);
            });
        flyout.hide();
        assert_eq!(redirected.get(), 1);
        assert!(outside.is_focused());
        assert!(!connection.is_current());
        assert!(!session.is_current());
        assert!(!f.owner.editor().is_composing());
        assert_eq!(f.owner.document().snapshot(), before);
        assert!(!f.owner.can_undo());
    }
}

#[test]
fn rich_context_holding_routes_opening_and_feedback_only_for_an_opened_menu() {
    for cancel in [false, true] {
        let f = Fixture::new(false);
        let flyout = f.flyout();
        InputElement::set_is_hold_with_mouse_enabled(&f.owner, true);
        PlatformFeedback::set_feedback_type(&f.owner, FeedbackType::Haptic);
        let calls = Rc::new(RefCell::new(Vec::new()));
        let observed = calls.clone();
        f.owner.context_menu_opening(move |_, args| {
            observed
                .borrow_mut()
                .push(Point::new(args.cursor_left(), args.cursor_top()));
            args.set_handled(cancel);
        });
        let pointer = Pointer::new(Pointer::get_next_free_id(), PointerType::Mouse, true);
        let point = f.root_point(Point::new(30.0, 30.0));
        pointer.capture(Some(&f.owner.clone().upcast()));
        let before = Dispatcher::timers_for_unit_tests();
        f.owner.raise_event(&PointerPressedEventArgs::new(
            f.owner.clone(),
            pointer.clone(),
            &f.window,
            point,
            1,
            PointerPointProperties::new(
                RawInputModifiers::LEFT_MOUSE_BUTTON,
                PointerUpdateKind::LeftButtonPressed,
            ),
            KeyModifiers::NONE,
            1,
        ));
        let timers: Vec<_> = Dispatcher::timers_for_unit_tests()
            .into_iter()
            .filter(|timer| !before.iter().any(|old| Rc::ptr_eq(old, timer)))
            .collect();
        assert_eq!(timers.len(), 1, "the hold gesture scheduled its timer");
        Dispatcher::force_fire_timer_for_unit_tests(&timers[0]);
        assert_eq!(&*calls.borrow(), &[point]);
        assert_eq!(flyout.is_open(), !cancel);
        assert_eq!(f.feedback.0.borrow().len(), usize::from(!cancel));
        if !cancel {
            assert_eq!(
                f.feedback.0.borrow()[0],
                (FeedbackAction::hold(), FeedbackType::Haptic)
            );
            f.assert_anchor(point);
        }
        f.owner.raise_event(&PointerReleasedEventArgs::new(
            f.owner.clone(),
            pointer.clone(),
            &f.window,
            point,
            2,
            PointerPointProperties::new(
                RawInputModifiers::NONE,
                PointerUpdateKind::LeftButtonReleased,
            ),
            KeyModifiers::NONE,
            MouseButton::Left,
        ));
        pointer.capture(None);
    }
}
