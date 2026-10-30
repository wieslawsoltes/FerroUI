//! Configured read-only context requests. Native source contracts are kept
//! separate from FerroUI's routed-event, ContextMenu and popup lifecycle policy.

use crate::documents::{InlineUIContainer, Paragraph};
use crate::platform::{
    FeedbackAction, FeedbackType, IPlatformFeedback, IPopupImpl, ITopLevelImpl, PlatformFeedback,
};
use crate::primitives::popup_positioning::{IPopupPositioner, PopupPositionerParameters};
use crate::primitives::FlyoutBaseImplExt;
use crate::testing::{
    MockWindowingPlatform, TestServices, UnitTestApplication, UnitTestApplicationScope,
};
use crate::{
    Border, ContextMenu, ContextMenuEventArgs, Control, Flyout, FlyoutShowMode, MenuBaseImplExt,
    PlacementMode, RichTextBlock, RichTextBlockOverflow, StackPanel, Window,
};
use ferroui_base::data::core::ValueType;
use ferroui_base::input::platform::PlatformHotkeyConfiguration;
use ferroui_base::input::{
    ContextRequestedEventArgs, IPointer, InputElement, Key, KeyEventArgs, KeyGesture, KeyModifiers,
    MouseButton, Pointer, PointerPointProperties, PointerPressedEventArgs,
    PointerReleasedEventArgs, PointerType, PointerUpdateKind, RawInputModifiers,
};
use ferroui_base::interactivity::{RoutedEvent, RoutedEventArgs, RoutingStrategies};
use ferroui_base::media::text_formatting::testing::TextTestScope;
use ferroui_base::media::{FlowDirection, ITransform};
use ferroui_base::metadata::from_markup_value;
use ferroui_base::threading::Dispatcher;
use ferroui_base::{FerroLocator, Matrix, Point, Rect, Ref, StaticType, Thickness};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

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

/// Coordinate conversion is an application callback boundary, even when the
/// transform's resulting matrix is the identity. Arm after attachment/layout
/// so each regression exercises the intended request-time read.
#[derive(Default)]
struct ReadCallbackTransform {
    callback: RefCell<Option<Box<dyn FnOnce()>>>,
}

impl ReadCallbackTransform {
    fn arm(&self, callback: impl FnOnce() + 'static) {
        *self.callback.borrow_mut() = Some(Box::new(callback));
    }
}

impl ITransform for ReadCallbackTransform {
    fn value(&self) -> Matrix {
        let callback = self.callback.borrow_mut().take();
        if let Some(callback) = callback {
            callback();
        }
        Matrix::IDENTITY
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
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
    view: Ref<RichTextBlock>,
    feedback: Rc<RecordingFeedback>,
    positions: Rc<RefCell<Vec<PopupPositionerParameters>>>,
    _text: TextTestScope,
    _app: UnitTestApplicationScope,
}

impl Fixture {
    fn new() -> Self {
        let positions = Rc::new(RefCell::new(Vec::new()));
        let native_positions = positions.clone();
        let feedback = Rc::new(RecordingFeedback::default());
        let native_feedback = feedback.clone();
        let app =
            UnitTestApplication::start(TestServices::focusable_window().with_windowing_platform(
                MockWindowingPlatform::with_window_impl(move || {
                    let window = MockWindowingPlatform::create_window_mock();
                    window.render_scaling.set(2.0);
                    window.desktop_scaling.set(2.0);
                    window.setup_feature::<dyn IPlatformFeedback>(native_feedback.clone());
                    let weak = Rc::downgrade(&window);
                    let requests = native_positions.clone();
                    window.setup_create_popup(move |_| {
                        let parent: Rc<dyn ITopLevelImpl> = weak.upgrade()?;
                        let popup = MockWindowingPlatform::create_popup_mock(parent);
                        let inner = IPopupImpl::popup_positioner(&*popup).unwrap();
                        popup.setup_popup_positioner(Some(Rc::new(RecordingPositioner {
                            inner,
                            requests: requests.clone(),
                        })));
                        Some(popup as Rc<dyn IPopupImpl>)
                    });
                    window
                }),
            ));
        let mut hotkeys = PlatformHotkeyConfiguration::default();
        hotkeys
            .open_context_menu
            .push(KeyGesture::new(Key::F10, KeyModifiers::SHIFT));
        FerroLocator::current_mutable()
            .bind::<PlatformHotkeyConfiguration>()
            .to_constant(Rc::new(hotkeys));
        let text = TextTestScope::new();
        let view = RichTextBlock::new();
        view.set_font_size(12.0);
        view.set_width(240.0);
        view.set_height(60.0);
        view.set_margin(Thickness::new(31.0, 19.0, 0.0, 0.0));
        view.set_padding(Thickness::new(13.0, 7.0, 9.0, 5.0));
        view.blocks()
            .add(Paragraph::with_text("alpha beta gamma delta"));
        view.select(2, 17);
        let panel = StackPanel::new();
        panel.children().add(view.clone());
        let window = Window::new();
        window.set_width(500.0);
        window.set_height(400.0);
        window.set_content(Some(Control::boxed(panel.clone())));
        window.show();
        view.focus();
        window.update_layout();
        Dispatcher::ui_thread().run_jobs(None);
        Self {
            window,
            panel,
            view,
            feedback,
            positions,
            _text: text,
            _app: app,
        }
    }

    fn flyout(&self, owner: &Control) -> Ref<Flyout> {
        let content = Border::new();
        content.set_width(50.0);
        content.set_height(24.0);
        let flyout = Flyout::new();
        flyout.set_content(Some(Control::boxed(content)));
        flyout.set_show_mode(FlyoutShowMode::Transient);
        owner.set_context_flyout(&flyout);
        flyout
    }

    fn overflow(&self, previous: &RichTextBlock) -> Ref<RichTextBlockOverflow> {
        let overflow = RichTextBlockOverflow::new();
        overflow.set_width(240.0);
        overflow.set_height(60.0);
        overflow.set_margin(Thickness::new(70.0, 13.0, 0.0, 0.0));
        previous.set_overflow_content_target(Some(overflow.clone()));
        self.panel.children().add(overflow.clone());
        self.layout();
        overflow
    }

    fn layout(&self) {
        self.window.update_layout();
        Dispatcher::ui_thread().run_jobs(None);
        self.window.update_layout();
    }

    fn root_point(&self, view: &RichTextBlock, point: Point) -> Point {
        point.transform(view.transform_to_visual(&self.window).unwrap())
    }

    fn assert_anchor(&self, expected: Point) {
        self.layout();
        assert_eq!(
            self.positions
                .borrow()
                .last()
                .expect("the popup was positioned")
                .anchor_rectangle,
            Rect::new(expected.x, expected.y, 0.0, 0.0),
        );
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.window.close();
    }
}

fn keyboard(target: &InputElement, key: Key, modifiers: KeyModifiers) -> KeyEventArgs {
    let mut args = KeyEventArgs::new();
    args.key = key;
    args.key_modifiers = modifiers;
    args.set_routed_event(Some(InputElement::key_up_event()));
    target.raise_event(&args);
    args
}

fn right_release(view: &RichTextBlock, window: &Window, point: Point) -> PointerReleasedEventArgs {
    let args = PointerReleasedEventArgs::new(
        view.to_ref(),
        Pointer::new(Pointer::get_next_free_id(), PointerType::Mouse, true),
        window,
        point,
        1,
        PointerPointProperties::new(
            RawInputModifiers::NONE,
            PointerUpdateKind::RightButtonReleased,
        ),
        KeyModifiers::NONE,
        MouseButton::Right,
    );
    view.raise_event(&args);
    args
}

#[test]
fn rich_readonly_context_pointer_reports_root_dips_and_cancels_configured_menus() {
    let f = Fixture::new();
    let flyout = f.flyout(&f.view);
    let menu = ContextMenu::new();
    f.view.set_context_menu(&menu);
    let calls = Rc::new(RefCell::new(Vec::new()));
    let observed = calls.clone();
    let view = f.view.downgrade();
    f.view.context_menu_opening(move |sender, args| {
        let view = view.upgrade().unwrap();
        assert_eq!(
            sender.to_ref(),
            view.clone()
                .upcast::<ferroui_base::interactivity::Interactive>()
        );
        assert_eq!(args.source(), Some(view.upcast()));
        observed
            .borrow_mut()
            .push(Point::new(args.cursor_left(), args.cursor_top()));
        args.set_handled(true);
    });
    let snapshot = f.view.document().snapshot();
    let point = Point::new(170.25, 57.75);
    assert!(right_release(&f.view, &f.window, point).handled());
    assert_eq!(
        &*calls.borrow(),
        &[point],
        "FerroUI root coordinates are already DIPs at scale 2"
    );
    assert!(!flyout.is_open());
    assert!(!menu.is_open());
    assert!(f.positions.borrow().is_empty());
    assert_eq!(f.view.document().snapshot(), snapshot);
    assert_eq!((f.view.selection_start(), f.view.selection_end()), (2, 17));
}

#[test]
fn rich_readonly_context_keyboard_uses_view_origin_for_selection_and_direction() {
    let f = Fixture::new();
    let flyout = f.flyout(&f.view);
    flyout.set_placement(PlacementMode::Top);
    let calls = Rc::new(RefCell::new(Vec::new()));
    let observed = calls.clone();
    f.view.context_menu_opening(move |_, args| {
        observed
            .borrow_mut()
            .push(Point::new(args.cursor_left(), args.cursor_top()));
    });
    for direction in [FlowDirection::LeftToRight, FlowDirection::RightToLeft] {
        f.view.set_flow_direction(direction);
        for (anchor, active) in [(2, 17), (17, 2), (3, 3)] {
            f.view.select(anchor, active);
            f.layout();
            let expected = f.root_point(&f.view, Point::default());
            assert!(keyboard(&f.view, Key::Apps, KeyModifiers::NONE).handled());
            assert_eq!(calls.borrow().last(), Some(&expected));
            assert!(flyout.is_open());
            f.assert_anchor(expected);
            assert_eq!(flyout.placement(), PlacementMode::Top);
            assert_eq!(
                (f.view.selection_start(), f.view.selection_end()),
                (anchor, active)
            );
            flyout.hide();
        }
    }
    // Native explicit custom flyouts are available even when selection is
    // disabled; the default text-control flyout has a separate gate.
    f.view.set_is_text_selection_enabled(false);
    assert!(keyboard(&f.view, Key::F10, KeyModifiers::SHIFT).handled());
    assert!(flyout.is_open());
    f.assert_anchor(f.root_point(&f.view, Point::default()));
}

#[test]
fn rich_readonly_context_programmatic_repeats_raise_opening_before_retained_popup_placement() {
    let f = Fixture::new();
    let flyout = f.flyout(&f.view);
    let calls = Rc::new(Cell::new(0));
    let observed = calls.clone();
    let cancel = Rc::new(Cell::new(false));
    let cancel_handler = cancel.clone();
    f.view.context_menu_opening(move |_, args| {
        observed.set(observed.get() + 1);
        args.set_handled(cancel_handler.get());
    });
    let point = f.root_point(&f.view, Point::new(40.5, 20.25));
    assert!(right_release(&f.view, &f.window, point).handled());
    f.assert_anchor(point);
    let canceled = ContextRequestedEventArgs::new();
    cancel.set(true);
    f.view.raise_event(&canceled);
    assert!(canceled.handled());
    assert_eq!(calls.get(), 2);
    assert!(flyout.is_open());
    f.assert_anchor(point);
    cancel.set(false);
    let repeated = ContextRequestedEventArgs::new();
    f.view.raise_event(&repeated);
    assert!(repeated.handled());
    assert_eq!(calls.get(), 3);
    assert!(
        flyout.is_open(),
        "a programmatic pointerless request is not a key toggle"
    );
    f.assert_anchor(f.root_point(&f.view, Point::default()));
}

#[test]
fn rich_readonly_context_actual_keys_keep_typed_cancellation_before_shared_closing() {
    let f = Fixture::new();
    let flyout = f.flyout(&f.view);
    let typed = Rc::new(Cell::new(0));
    let observed = typed.clone();
    let cancel_typed = Rc::new(Cell::new(false));
    let cancel_handler = cancel_typed.clone();
    f.view.context_menu_opening(move |_, args| {
        observed.set(observed.get() + 1);
        args.set_handled(cancel_handler.get());
    });
    assert!(keyboard(&f.view, Key::Apps, KeyModifiers::NONE).handled());
    assert!(flyout.is_open());
    assert_eq!(typed.get(), 1);
    let closing = Rc::new(Cell::new(0));
    let observed = closing.clone();
    let cancel_closing = Rc::new(Cell::new(true));
    let cancel_handler = cancel_closing.clone();
    let _closing = flyout.closing(move |args| {
        observed.set(observed.get() + 1);
        args.set_cancel(cancel_handler.get());
    });
    cancel_typed.set(true);
    assert!(keyboard(&f.view, Key::Apps, KeyModifiers::NONE).handled());
    assert_eq!(typed.get(), 2);
    assert_eq!(closing.get(), 0);
    assert!(flyout.is_open());
    cancel_typed.set(false);
    assert!(keyboard(&f.view, Key::F10, KeyModifiers::SHIFT).handled());
    assert_eq!(typed.get(), 3);
    assert_eq!(closing.get(), 1);
    assert!(flyout.is_open());
    cancel_closing.set(false);
    assert!(keyboard(&f.view, Key::Apps, KeyModifiers::NONE).handled());
    assert_eq!(typed.get(), 4);
    assert_eq!(closing.get(), 2);
    assert!(!flyout.is_open());
}

#[test]
fn rich_readonly_context_nested_programmatic_request_does_not_inherit_key_toggle() {
    let f = Fixture::new();
    let flyout = f.flyout(&f.view);
    assert!(keyboard(&f.view, Key::Apps, KeyModifiers::NONE).handled());
    let calls = Rc::new(Cell::new(0));
    let observed = calls.clone();
    let view = f.view.downgrade();
    f.view.context_menu_opening(move |_, _| {
        observed.set(observed.get() + 1);
        if observed.get() == 1 {
            let nested = ContextRequestedEventArgs::new();
            view.upgrade().unwrap().raise_event(&nested);
            assert!(nested.handled());
        }
    });
    let closes = Rc::new(Cell::new(0));
    let observed = closes.clone();
    let _closing = flyout.closing(move |_| observed.set(observed.get() + 1));
    assert!(keyboard(&f.view, Key::Apps, KeyModifiers::NONE).handled());
    assert_eq!(calls.get(), 2);
    assert_eq!(closes.get(), 0);
    assert!(flyout.is_open());
    assert!(keyboard(&f.view, Key::F10, KeyModifiers::SHIFT).handled());
    assert_eq!(calls.get(), 3);
    assert_eq!(closes.get(), 1);
    assert!(!flyout.is_open());
}

#[test]
fn rich_readonly_context_nested_keyboard_request_supersedes_outer_key_identity() {
    let f = Fixture::new();
    let flyout = f.flyout(&f.view);
    assert!(keyboard(&f.view, Key::Apps, KeyModifiers::NONE).handled());
    let calls = Rc::new(Cell::new(0));
    let observed = calls.clone();
    let view = f.view.downgrade();
    f.view.context_menu_opening(move |_, _| {
        observed.set(observed.get() + 1);
        if observed.get() == 1 {
            assert!(keyboard(&view.upgrade().unwrap(), Key::F10, KeyModifiers::SHIFT).handled());
        }
    });
    let closes = Rc::new(Cell::new(0));
    let observed = closes.clone();
    let _closing = flyout.closing(move |_| observed.set(observed.get() + 1));
    assert!(keyboard(&f.view, Key::Apps, KeyModifiers::NONE).handled());
    assert_eq!(calls.get(), 2);
    assert_eq!(closes.get(), 1);
    assert!(!flyout.is_open());
    let programmatic = ContextRequestedEventArgs::new();
    f.view.raise_event(&programmatic);
    assert!(programmatic.handled());
    assert!(
        flyout.is_open(),
        "the completed key scope cannot leak into a new request"
    );
}

#[test]
fn rich_readonly_context_tunnel_nested_requests_cannot_revive_an_older_key_scope() {
    for nested_key in [false, true] {
        let f = Fixture::new();
        let flyout = f.flyout(&f.view);
        assert!(keyboard(&f.view, Key::Apps, KeyModifiers::NONE).handled());
        let typed = Rc::new(Cell::new(0));
        let observed = typed.clone();
        f.view
            .context_menu_opening(move |_, _| observed.set(observed.get() + 1));
        let nested = Rc::new(Cell::new(false));
        let nested_handler = nested.clone();
        let view = f.view.downgrade();
        f.view.add_handler_with(
            InputElement::context_requested_event(),
            move |_, _: &ContextRequestedEventArgs| {
                if !nested_handler.replace(true) {
                    let view = view.upgrade().unwrap();
                    if nested_key {
                        assert!(keyboard(&view, Key::F10, KeyModifiers::SHIFT).handled());
                    } else {
                        let request = ContextRequestedEventArgs::new();
                        view.raise_event(&request);
                        assert!(request.handled());
                    }
                }
            },
            RoutingStrategies::TUNNEL,
            false,
        );
        assert!(keyboard(&f.view, Key::Apps, KeyModifiers::NONE).handled());
        assert_eq!(
            typed.get(),
            1,
            "only the newer request reaches the typed opening"
        );
        assert_eq!(flyout.is_open(), !nested_key);
        let later = ContextRequestedEventArgs::new();
        f.view.raise_event(&later);
        assert!(later.handled());
        assert!(flyout.is_open());
        assert_eq!(typed.get(), 2);
    }
}

#[test]
fn rich_readonly_context_key_handler_replacement_or_removal_closes_previous_popup_once() {
    for (remove, cancel_close) in [(false, false), (true, false), (false, true), (true, true)] {
        let f = Fixture::new();
        let previous = f.flyout(&f.view);
        assert!(keyboard(&f.view, Key::Apps, KeyModifiers::NONE).handled());
        let replacement = f.flyout(&f.view);
        f.view.set_context_flyout(&previous);
        let view = f.view.downgrade();
        let accepted = replacement.downgrade();
        f.view.context_menu_opening(move |_, _| {
            let view = view.upgrade().unwrap();
            view.set_context_flyout(if remove {
                None
            } else {
                accepted.upgrade().map(|flyout| flyout.upcast())
            });
        });
        let closes = Rc::new(Cell::new(0));
        let observed = closes.clone();
        let _closing = previous.closing(move |args| {
            observed.set(observed.get() + 1);
            args.set_cancel(cancel_close);
        });
        assert!(keyboard(&f.view, Key::Apps, KeyModifiers::NONE).handled());
        assert_eq!(closes.get(), 1);
        assert_eq!(previous.is_open(), cancel_close);
        assert_eq!(replacement.is_open(), !remove && !cancel_close);
    }
}

#[test]
fn rich_readonly_context_unconfigured_or_prehandled_requests_do_not_project_an_event() {
    let f = Fixture::new();
    let calls = Rc::new(Cell::new(0));
    let observed = calls.clone();
    f.view
        .context_menu_opening(move |_, _| observed.set(observed.get() + 1));
    let request = ContextRequestedEventArgs::new();
    f.view.raise_event(&request);
    assert!(!request.handled());
    assert_eq!(
        calls.get(),
        0,
        "automatic selection Copy menus remain a separate path"
    );
    let flyout = f.flyout(&f.view);
    let prehandled = ContextRequestedEventArgs::new();
    prehandled.set_handled(true);
    f.view.raise_event(&prehandled);
    assert_eq!(calls.get(), 0);
    assert!(!flyout.is_open());
    f.panel
        .add_handler(RichTextBlock::context_menu_opening_event(), |_, args| {
            args.set_handled(true)
        });
    assert!(keyboard(&f.view, Key::Apps, KeyModifiers::NONE).handled());
    assert_eq!(calls.get(), 1);
    assert!(!flyout.is_open());
}

#[test]
fn rich_readonly_context_opening_can_replace_or_remove_the_configured_flyout() {
    for remove in [false, true] {
        let f = Fixture::new();
        let previous = f.flyout(&f.view);
        let replacement = f.flyout(&f.view);
        f.view.set_context_flyout(&previous);
        let view = f.view.downgrade();
        let accepted = replacement.downgrade();
        f.view.context_menu_opening(move |_, _| {
            let view = view.upgrade().unwrap();
            if remove {
                view.set_context_flyout(None);
            } else {
                view.set_context_flyout(accepted.upgrade().map(|flyout| flyout.upcast()));
            }
        });
        assert!(keyboard(&f.view, Key::Apps, KeyModifiers::NONE).handled());
        assert!(!previous.is_open());
        assert_eq!(replacement.is_open(), !remove);
    }
}

#[test]
fn rich_readonly_context_nested_request_owns_the_opening_and_pointer_anchor() {
    let f = Fixture::new();
    let flyout = f.flyout(&f.view);
    let calls = Rc::new(Cell::new(0));
    let observed = calls.clone();
    let view = f.view.downgrade();
    let window = f.window.downgrade();
    // Keep the nested pointer inside the requester (root Y19..79): the
    // shared popup positioner clamps explicit anchors to its target bounds.
    let point = Point::new(201.25, 61.75);
    f.view.context_menu_opening(move |_, _| {
        observed.set(observed.get() + 1);
        if observed.get() == 1 {
            right_release(&view.upgrade().unwrap(), &window.upgrade().unwrap(), point);
        }
    });
    let opened = Rc::new(Cell::new(0));
    let observed = opened.clone();
    let _opened = flyout.opened(move || observed.set(observed.get() + 1));
    assert!(keyboard(&f.view, Key::Apps, KeyModifiers::NONE).handled());
    assert_eq!(calls.get(), 2);
    assert_eq!(opened.get(), 1);
    assert!(flyout.is_open());
    f.assert_anchor(point);
}

#[test]
fn rich_readonly_context_unconfigured_nested_request_retires_outer_intent() {
    let f = Fixture::new();
    let flyout = f.flyout(&f.view);
    let view = f.view.downgrade();
    let replacement = flyout.downgrade();
    let calls = Rc::new(Cell::new(0));
    let observed = calls.clone();
    f.view.context_menu_opening(move |_, _| {
        observed.set(observed.get() + 1);
        let view = view.upgrade().unwrap();
        view.set_context_flyout(None);
        let nested = ContextRequestedEventArgs::new();
        view.raise_event(&nested);
        assert!(!nested.handled());
        view.set_context_flyout(replacement.upgrade().map(|flyout| flyout.upcast()));
    });
    assert!(keyboard(&f.view, Key::Apps, KeyModifiers::NONE).handled());
    assert_eq!(calls.get(), 1);
    assert!(!flyout.is_open());
    assert!(f.positions.borrow().is_empty());
}

#[test]
fn rich_readonly_context_detach_and_reattach_invalidates_the_typed_opening() {
    let f = Fixture::new();
    let flyout = f.flyout(&f.view);
    let view = f.view.downgrade();
    let panel = f.panel.downgrade();
    f.view.context_menu_opening(move |_, _| {
        let view = view.upgrade().unwrap();
        let panel = panel.upgrade().unwrap();
        panel.children().remove(&view);
        panel.children().add(view);
    });
    assert!(keyboard(&f.view, Key::Apps, KeyModifiers::NONE).handled());
    assert!(!flyout.is_open());
    assert!(f.positions.borrow().is_empty());
}

#[test]
fn rich_readonly_context_transform_reentry_retires_outer_before_typed_opening() {
    let f = Fixture::new();
    let flyout = f.flyout(&f.view);
    let transform = Rc::new(ReadCallbackTransform::default());
    f.view.set_render_transform(Some(transform.clone()));
    f.layout();
    let calls = Rc::new(RefCell::new(Vec::new()));
    let observed = calls.clone();
    f.view.context_menu_opening(move |_, args| {
        observed
            .borrow_mut()
            .push(Point::new(args.cursor_left(), args.cursor_top()));
    });
    let opened = Rc::new(Cell::new(0));
    let observed = opened.clone();
    let _opened = flyout.opened(move || observed.set(observed.get() + 1));
    let view = f.view.downgrade();
    let window = f.window.downgrade();
    let point = Point::new(203.25, 63.75);
    transform.arm(move || {
        assert!(
            right_release(&view.upgrade().unwrap(), &window.upgrade().unwrap(), point).handled()
        );
    });

    assert!(keyboard(&f.view, Key::Apps, KeyModifiers::NONE).handled());
    assert_eq!(
        &*calls.borrow(),
        &[point],
        "the outer event was retired before publication"
    );
    assert_eq!(opened.get(), 1);
    assert!(flyout.is_open());
    f.assert_anchor(point);
}

#[test]
fn rich_readonly_context_transform_detach_reattach_suppresses_stale_typed_opening() {
    let f = Fixture::new();
    let flyout = f.flyout(&f.view);
    let transform = Rc::new(ReadCallbackTransform::default());
    f.view.set_render_transform(Some(transform.clone()));
    f.layout();
    let calls = Rc::new(Cell::new(0));
    let observed = calls.clone();
    f.view
        .context_menu_opening(move |_, _| observed.set(observed.get() + 1));
    let view = f.view.downgrade();
    let panel = f.panel.downgrade();
    transform.arm(move || {
        let view = view.upgrade().unwrap();
        let panel = panel.upgrade().unwrap();
        panel.children().remove(&view);
        panel.children().add(view);
    });

    assert!(keyboard(&f.view, Key::Apps, KeyModifiers::NONE).handled());
    assert_eq!(calls.get(), 0);
    assert!(!flyout.is_open());
    assert!(f.positions.borrow().is_empty());
    f.layout();
    assert!(keyboard(&f.view, Key::Apps, KeyModifiers::NONE).handled());
    assert_eq!(
        calls.get(),
        1,
        "the next attached request is still accepted"
    );
    assert!(flyout.is_open());
}

#[test]
fn rich_readonly_context_placement_transform_preserves_nested_flyout_anchor() {
    let f = Fixture::new();
    let flyout = f.flyout(&f.view);
    let transform = Rc::new(ReadCallbackTransform::default());
    f.view.set_render_transform(Some(transform.clone()));
    f.layout();
    let calls = Rc::new(Cell::new(0));
    let observed = calls.clone();
    let view = f.view.downgrade();
    let window = f.window.downgrade();
    let point = Point::new(205.25, 65.75);
    f.view.context_menu_opening(move |_, _| {
        observed.set(observed.get() + 1);
        if observed.get() == 1 {
            let view = view.clone();
            let window = window.clone();
            transform.arm(move || {
                assert!(
                    right_release(&view.upgrade().unwrap(), &window.upgrade().unwrap(), point)
                        .handled()
                );
            });
        }
    });
    let opened = Rc::new(Cell::new(0));
    let observed = opened.clone();
    let _opened = flyout.opened(move || observed.set(observed.get() + 1));

    assert!(right_release(&f.view, &f.window, Point::new(170.5, 55.5)).handled());
    assert_eq!(calls.get(), 2);
    assert_eq!(opened.get(), 1);
    assert!(flyout.is_open());
    f.assert_anchor(point);
}

#[test]
fn rich_readonly_context_placement_transform_unconfigured_reentry_retires_same_flyout() {
    let f = Fixture::new();
    let flyout = f.flyout(&f.view);
    let transform = Rc::new(ReadCallbackTransform::default());
    f.view.set_render_transform(Some(transform.clone()));
    f.layout();
    let calls = Rc::new(Cell::new(0));
    let observed = calls.clone();
    let view = f.view.downgrade();
    let current = flyout.downgrade();
    f.view.context_menu_opening(move |_, _| {
        observed.set(observed.get() + 1);
        let view = view.clone();
        let current = current.clone();
        transform.arm(move || {
            let view = view.upgrade().unwrap();
            view.set_context_flyout(None);
            let nested = ContextRequestedEventArgs::new();
            view.raise_event(&nested);
            assert!(!nested.handled());
            view.set_context_flyout(current.upgrade().map(|flyout| flyout.upcast()));
        });
    });

    assert!(right_release(&f.view, &f.window, Point::new(170.5, 55.5)).handled());
    assert_eq!(calls.get(), 1);
    assert!(
        !flyout.is_open(),
        "restoring the same property does not restore older intent"
    );
    assert!(f.positions.borrow().is_empty());
}

#[test]
fn rich_readonly_context_menu_placement_transform_preserves_newer_menu_choice() {
    let f = Fixture::new();
    let flyout = f.flyout(&f.view);
    let menu = ContextMenu::new();
    f.view.set_context_menu(&menu);
    let transform = Rc::new(ReadCallbackTransform::default());
    f.view.set_render_transform(Some(transform.clone()));
    f.layout();
    let calls = Rc::new(Cell::new(0));
    let observed = calls.clone();
    let view = f.view.downgrade();
    let window = f.window.downgrade();
    let point = Point::new(207.25, 67.75);
    f.view.context_menu_opening(move |_, _| {
        observed.set(observed.get() + 1);
        if observed.get() == 1 {
            let view = view.clone();
            let window = window.clone();
            transform.arm(move || {
                let view = view.upgrade().unwrap();
                view.set_context_menu(None);
                assert!(right_release(&view, &window.upgrade().unwrap(), point).handled());
            });
        }
    });

    assert!(right_release(&f.view, &f.window, Point::new(170.5, 55.5)).handled());
    assert_eq!(calls.get(), 2);
    assert!(!menu.is_open(), "the old snapshotted menu must not reopen");
    assert!(flyout.is_open());
    f.assert_anchor(point);
}

#[test]
fn rich_readonly_context_menu_opening_reentry_preserves_newer_menu_choice() {
    let f = Fixture::new();
    let flyout = f.flyout(&f.view);
    let menu = ContextMenu::new();
    f.view.set_context_menu(&menu);
    let calls = Rc::new(Cell::new(0));
    let observed = calls.clone();
    f.view
        .context_menu_opening(move |_, _| observed.set(observed.get() + 1));
    let view = f.view.downgrade();
    let window = f.window.downgrade();
    let point = Point::new(209.25, 69.75);
    let _opening = menu.opening(move |_| {
        let view = view.upgrade().unwrap();
        view.set_context_menu(None);
        assert!(right_release(&view, &window.upgrade().unwrap(), point).handled());
    });

    assert!(keyboard(&f.view, Key::Apps, KeyModifiers::NONE).handled());
    assert_eq!(calls.get(), 2);
    assert!(!menu.is_open());
    assert!(flyout.is_open());
    f.assert_anchor(point);
}

#[test]
fn rich_readonly_context_menu_precedence_survives_canceled_property_removal() {
    for remove in [false, true] {
        let f = Fixture::new();
        let flyout = f.flyout(&f.view);
        let menu = ContextMenu::new();
        f.view.set_context_menu(&menu);
        let calls = Rc::new(Cell::new(0));
        let observed = calls.clone();
        let view = f.view.downgrade();
        let _opening = menu.opening(move |args| {
            observed.set(observed.get() + 1);
            if remove {
                view.upgrade().unwrap().set_context_menu(None);
            }
            args.set_cancel(true);
        });
        assert!(keyboard(&f.view, Key::Apps, KeyModifiers::NONE).handled());
        assert_eq!(calls.get(), 1);
        assert!(!menu.is_open());
        assert!(!flyout.is_open());
    }
}

#[test]
fn rich_readonly_context_overflow_inherits_root_master_and_keeps_requesting_target() {
    let f = Fixture::new();
    let first = f.overflow(&f.view);
    let second = f.overflow(&first);
    let root_flyout = f.flyout(&f.view);
    let intermediate = f.flyout(&first);
    let calls = Rc::new(Cell::new(0));
    for view in [
        f.view.clone(),
        first.clone().upcast(),
        second.clone().upcast(),
    ] {
        let observed = calls.clone();
        view.context_menu_opening(move |_, _| observed.set(observed.get() + 1));
    }
    let point = f.root_point(&second, Point::new(19.5, 11.25));
    assert!(right_release(&second, &f.window, point).handled());
    assert!(root_flyout.is_open());
    assert!(!intermediate.is_open());
    assert_eq!(root_flyout.target(), Some(second.clone().upcast()));
    assert!(
        second.context_flyout().is_none(),
        "fallback does not write the local property"
    );
    assert_eq!(
        calls.get(),
        0,
        "the native modern flyout helper has no overflow event branch"
    );
    f.assert_anchor(point);
    root_flyout.hide();
    assert!(keyboard(&first, Key::Apps, KeyModifiers::NONE).handled());
    assert!(intermediate.is_open());
    assert_eq!(intermediate.target(), Some(first.clone().upcast()));
    assert!(!root_flyout.is_open());
}

#[test]
fn rich_readonly_context_overflow_local_override_clear_and_orphan_chain() {
    let f = Fixture::new();
    let first = f.overflow(&f.view);
    let second = f.overflow(&first);
    let master = f.flyout(&f.view);
    let intermediate = f.flyout(&first);
    let own = f.flyout(&second);
    assert!(keyboard(&second, Key::Apps, KeyModifiers::NONE).handled());
    assert!(own.is_open());
    assert!(!master.is_open());
    assert!(!intermediate.is_open());
    own.hide();
    second.clear_value(Control::context_flyout_property());
    assert!(keyboard(&second, Key::Apps, KeyModifiers::NONE).handled());
    assert!(master.is_open());
    master.hide();
    f.view.set_overflow_content_target(None);
    let request = ContextRequestedEventArgs::new();
    second.raise_event(&request);
    assert!(!request.handled());
    assert!(!own.is_open());
    assert!(!master.is_open());
    assert!(
        !intermediate.is_open(),
        "orphan predecessor is not a root master"
    );
}

#[test]
fn rich_readonly_context_default_overflow_flyout_uses_master_selection_gate() {
    let f = Fixture::new();
    let overflow = f.overflow(&f.view);
    let inherited = f.flyout(&f.view);
    f.view.set_is_text_selection_enabled(false);
    overflow.set_is_text_selection_enabled(true);
    assert!(!overflow.is_set(Control::context_flyout_property().as_property()));
    let blocked = ContextRequestedEventArgs::new();
    overflow.raise_event(&blocked);
    assert!(!blocked.handled());
    assert!(!inherited.is_open());
    assert!(!right_release(
        &overflow,
        &f.window,
        f.root_point(&overflow, Point::new(5.0, 8.0))
    )
    .handled());
    assert!(!inherited.is_open());

    // CDependencyObject::IsPropertyDefault tracks assignment provenance. A
    // local null still selects the master's flyout, but bypasses the default
    // text-flyout gate; clearing that value restores the default gate.
    overflow.set_context_flyout(None);
    assert!(overflow.is_set(Control::context_flyout_property().as_property()));
    let explicit_null = ContextRequestedEventArgs::new();
    overflow.raise_event(&explicit_null);
    assert!(explicit_null.handled());
    assert!(inherited.is_open());
    inherited.hide();
    overflow.clear_value(Control::context_flyout_property());
    let default_again = ContextRequestedEventArgs::new();
    overflow.raise_event(&default_again);
    assert!(!default_again.handled());
    assert!(!inherited.is_open());

    let own = f.flyout(&overflow);
    let configured = ContextRequestedEventArgs::new();
    overflow.raise_event(&configured);
    assert!(configured.handled());
    assert!(
        own.is_open(),
        "local custom flyouts are independent of text selection"
    );
    own.hide();
    overflow.clear_value(Control::context_flyout_property());
    f.view.set_is_text_selection_enabled(true);
    overflow.set_is_text_selection_enabled(false);
    let enabled_master = ContextRequestedEventArgs::new();
    overflow.raise_event(&enabled_master);
    assert!(enabled_master.handled());
    assert!(
        inherited.is_open(),
        "the native overflow gate reads its root master's policy"
    );
}

#[test]
fn rich_readonly_context_local_menu_is_independent_of_default_flyout_selection_gate() {
    let f = Fixture::new();
    let overflow = f.overflow(&f.view);
    let inherited = f.flyout(&f.view);
    f.view.set_is_text_selection_enabled(false);
    let menu = ContextMenu::new();
    overflow.set_context_menu(&menu);
    let request = ContextRequestedEventArgs::new();
    overflow.raise_event(&request);
    assert!(request.handled());
    assert!(menu.is_open());
    assert!(!inherited.is_open());
    menu.close();
}

#[test]
fn rich_readonly_context_overflow_keyboard_anchor_dismissal_and_target_cancellation() {
    let f = Fixture::new();
    let first = f.overflow(&f.view);
    let second = f.overflow(&first);
    let flyout = f.flyout(&f.view);
    assert!(keyboard(&second, Key::Apps, KeyModifiers::NONE).handled());
    assert!(flyout.is_open());
    f.assert_anchor(f.root_point(&second, Point::default()));
    // A sibling continuation must not cancel a flyout placed on another view.
    first.raise_event(&RoutedEventArgs::with_event(
        InputElement::context_canceled_event(),
    ));
    assert!(flyout.is_open());
    let close_calls = Rc::new(Cell::new(0));
    let observed = close_calls.clone();
    let cancel = Rc::new(Cell::new(true));
    let cancel_handler = cancel.clone();
    let _closing = flyout.closing(move |args| {
        observed.set(observed.get() + 1);
        args.set_cancel(cancel_handler.get());
    });
    second.raise_event(&RoutedEventArgs::with_event(
        InputElement::context_canceled_event(),
    ));
    assert_eq!(close_calls.get(), 1);
    assert!(flyout.is_open());
    cancel.set(false);
    assert!(keyboard(&second, Key::Apps, KeyModifiers::NONE).handled());
    assert_eq!(
        close_calls.get(),
        2,
        "open flyout hotkey recognizes the inherited owner"
    );
    assert!(!flyout.is_open());
    assert!(keyboard(&second, Key::F10, KeyModifiers::SHIFT).handled());
    assert!(flyout.is_open());
    f.panel.children().remove(&second);
    assert!(
        !flyout.is_open(),
        "existing placement-target detachment closes the popup"
    );
}

#[test]
fn rich_readonly_context_overflow_local_menu_precedes_inherited_flyout() {
    let f = Fixture::new();
    let overflow = f.overflow(&f.view);
    let flyout = f.flyout(&f.view);
    let menu = ContextMenu::new();
    menu.set_width(80.0);
    menu.set_height(30.0);
    overflow.set_context_menu(&menu);
    assert!(keyboard(&overflow, Key::Apps, KeyModifiers::NONE).handled());
    assert!(menu.is_open());
    assert!(!flyout.is_open());
    f.assert_anchor(f.root_point(&overflow, Point::default()));
    menu.close();
}

#[test]
fn rich_readonly_context_inherited_flyout_opening_cancellation_is_not_retried() {
    let f = Fixture::new();
    let overflow = f.overflow(&f.view);
    let flyout = f.flyout(&f.view);
    let calls = Rc::new(Cell::new(0));
    let observed = calls.clone();
    let _opening = flyout.opening(move |args| {
        observed.set(observed.get() + 1);
        args.set_cancel(true);
    });
    assert!(keyboard(&overflow, Key::Apps, KeyModifiers::NONE).handled());
    assert_eq!(calls.get(), 1);
    assert!(!flyout.is_open());
}

#[test]
fn rich_readonly_context_embedded_reader_has_one_public_opening_owner() {
    let f = Fixture::new();
    let child = RichTextBlock::new();
    child.set_width(60.0);
    child.set_height(24.0);
    child.blocks().add(Paragraph::with_text("nested"));
    f.view.blocks().add(Paragraph::new());
    let paragraph = f.view.blocks().get(1).cast::<Paragraph>().unwrap();
    paragraph.add_child(InlineUIContainer::with_child(child.clone()));
    let child_flyout = f.flyout(&child);
    let outer_flyout = f.flyout(&f.view);
    let sources = Rc::new(RefCell::new(Vec::new()));
    let observed = sources.clone();
    f.view.context_menu_opening(move |_, args| {
        observed.borrow_mut().push(args.source());
    });
    f.layout();
    let request = ContextRequestedEventArgs::new();
    child.raise_event(&request);
    assert!(request.handled());
    assert!(child_flyout.is_open());
    assert!(!outer_flyout.is_open());
    assert_eq!(&*sources.borrow(), &[Some(child.upcast())]);
}

#[test]
fn rich_readonly_context_holding_keeps_overflow_feedback_and_selection_policy() {
    let f = Fixture::new();
    let overflow = f.overflow(&f.view);
    let flyout = f.flyout(&f.view);
    overflow.set_is_text_selection_enabled(false);
    InputElement::set_is_hold_with_mouse_enabled(&overflow, true);
    PlatformFeedback::set_feedback_type(&overflow, FeedbackType::Haptic);
    let pointer = Pointer::new(Pointer::get_next_free_id(), PointerType::Mouse, true);
    let point = f.root_point(&overflow, Point::new(20.0, 20.0));
    pointer.capture(Some(&overflow.clone().upcast()));
    let before = Dispatcher::timers_for_unit_tests();
    overflow.raise_event(&PointerPressedEventArgs::new(
        overflow.clone(),
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
    assert_eq!(timers.len(), 1);
    Dispatcher::force_fire_timer_for_unit_tests(&timers[0]);
    assert!(flyout.is_open());
    assert_eq!(flyout.target(), Some(overflow.clone().upcast()));
    assert_eq!(
        &*f.feedback.0.borrow(),
        &[(FeedbackAction::hold(), FeedbackType::Haptic)]
    );
    f.assert_anchor(point);
    pointer.capture(None);
}
