//! Positions are computed from the metrics of the test fonts (see
//! `ferroui_base::media::text_formatting::testing`): every glyph advances
//! half an em and a line is 1.1 em high, so at the default size of 12 a
//! character is 6 wide and a line 13.2 high.

use crate::documents::{InlineCollection, InlineUIContainer, Run};
use crate::test_support::{test_scope, TestRoot};
use crate::testing::{
    MockWindowImpl, MockWindowingPlatform, TestClipboardFailure, TestClipboardImpl, TestServices,
    UnitTestApplication, UnitTestApplicationScope,
};
use crate::text_box_tests::{panic_message, record_log_messages, EXPECTED_CLIPBOARD_EXCEPTIONS};
use crate::utils::ClipboardHelper;
use crate::{Border, Control, SelectableTextBlock, Window};
use ferroui_base::input::platform::{
    ClipboardError, ClipboardErrorKind, ClipboardExtensions, ClipboardType, IClipboard, IPlatformClipboardManagerImpl,
    PlatformClipboardManager,
};
use ferroui_base::media::text_formatting::testing::TextTestScope;
use ferroui_base::platform::IPlatformRenderInterface;
use ferroui_base::threading::Dispatcher;
use ferroui_base::{FerroLocator, LocatorExtensions};
use ferroui_base::input::{
    FocusChangedEventArgs, IPointer, InputElement, Key, KeyEventArgs, KeyModifiers, MouseButton, Pointer,
    PointerEventArgs, PointerPointProperties, PointerPressedEventArgs, PointerReleasedEventArgs, PointerType,
    PointerUpdateKind, RawInputModifiers,
};
use ferroui_base::interactivity::Interactive;
use ferroui_base::media::text_formatting::ShapedTextRun;
use ferroui_base::media::{Brushes, FontStyle, FontWeight, IBrush, TextAlignment};
use ferroui_base::{Point, Rect, Ref, Size, Visual};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

fn infinity() -> Size {
    Size::new(f64::INFINITY, f64::INFINITY)
}

fn text_block(text: &str) -> Ref<SelectableTextBlock> {
    let target = SelectableTextBlock::new();
    target.set_text(Some(text));
    target
}

fn utf16_length(text: &str) -> i32 {
    text.encode_utf16().count() as i32
}

/// Raises pointer events on an element the way a mouse would.
struct Mouse {
    pointer: Rc<Pointer>,
    next_stamp: Cell<u64>,
    pressed_buttons: Cell<RawInputModifiers>,
    pressed_button: Cell<MouseButton>,
}

impl Mouse {
    fn new() -> Self {
        Self {
            pointer: Pointer::new(Pointer::get_next_free_id(), PointerType::Mouse, true),
            next_stamp: Cell::new(1),
            pressed_buttons: Cell::new(RawInputModifiers::NONE),
            pressed_button: Cell::new(MouseButton::None),
        }
    }

    fn timestamp(&self) -> u64 {
        let stamp = self.next_stamp.get();
        self.next_stamp.set(stamp + 1);
        stamp
    }

    fn pointer(&self) -> Rc<dyn IPointer> {
        self.pointer.clone()
    }

    fn convert(mouse_button: MouseButton) -> RawInputModifiers {
        match mouse_button {
            MouseButton::Left => RawInputModifiers::LEFT_MOUSE_BUTTON,
            MouseButton::Right => RawInputModifiers::RIGHT_MOUSE_BUTTON,
            MouseButton::Middle => RawInputModifiers::MIDDLE_MOUSE_BUTTON,
            _ => RawInputModifiers::NONE,
        }
    }

    fn root_of(target: &Interactive) -> Ref<Visual> {
        target.visual_root().unwrap_or_else(|| target.to_ref().upcast())
    }

    /// Presses a button at a position in the coordinates of the root.
    fn down(&self, target: &Interactive, mouse_button: MouseButton, position: Point) {
        self.down_with(target, mouse_button, position, KeyModifiers::NONE, 1);
    }

    fn down_with(
        &self,
        target: &Interactive,
        mouse_button: MouseButton,
        position: Point,
        modifiers: KeyModifiers,
        click_count: i32,
    ) {
        self.pressed_buttons.set(self.pressed_buttons.get() | Self::convert(mouse_button));
        self.pressed_button.set(mouse_button);

        let properties = PointerPointProperties::new(
            self.pressed_buttons.get(),
            match mouse_button {
                MouseButton::Left => PointerUpdateKind::LeftButtonPressed,
                MouseButton::Middle => PointerUpdateKind::MiddleButtonPressed,
                MouseButton::Right => PointerUpdateKind::RightButtonPressed,
                _ => PointerUpdateKind::Other,
            },
        );

        let element: Option<Ref<InputElement>> = target.to_ref().cast::<InputElement>();
        self.pointer.capture(element.as_ref());

        let root = Self::root_of(target);
        target.raise_event(&PointerPressedEventArgs::new(
            target.to_ref(),
            self.pointer(),
            &root,
            position,
            self.timestamp(),
            properties,
            modifiers,
            click_count,
        ));
    }

    /// Moves the pointer to a position in the coordinates of the root.
    fn move_(&self, target: &Interactive, position: Point) {
        let root = Self::root_of(target);
        target.raise_event(&PointerEventArgs::new(
            Some(InputElement::pointer_moved_event()),
            target.to_ref(),
            self.pointer(),
            Some(&*root),
            position,
            self.timestamp(),
            PointerPointProperties::new(self.pressed_buttons.get(), PointerUpdateKind::Other),
            KeyModifiers::NONE,
        ));
    }

    /// Releases a button at a position in the coordinates of the root.
    fn up(&self, target: &Interactive, mouse_button: MouseButton, position: Point) {
        let button = Self::convert(mouse_button);
        self.pressed_buttons.set((self.pressed_buttons.get() | button) ^ button);

        let properties = PointerPointProperties::new(
            self.pressed_buttons.get(),
            match mouse_button {
                MouseButton::Left => PointerUpdateKind::LeftButtonReleased,
                MouseButton::Middle => PointerUpdateKind::MiddleButtonReleased,
                MouseButton::Right => PointerUpdateKind::RightButtonReleased,
                _ => PointerUpdateKind::Other,
            },
        );

        let root = Self::root_of(target);
        target.raise_event(&PointerReleasedEventArgs::new(
            target.to_ref(),
            self.pointer(),
            &root,
            position,
            self.timestamp(),
            properties,
            KeyModifiers::NONE,
            self.pressed_button.get(),
        ));

        self.pointer.capture(None);
    }
}

/// The services of the tests that show the text block in a window: the
/// test fonts under the application of the windowing tests.
struct WindowServices {
    _app: UnitTestApplicationScope,
    _text: TextTestScope,
}

fn window_services() -> WindowServices {
    let text = TextTestScope::new();
    let render_interface = FerroLocator::current()
        .get_service::<dyn IPlatformRenderInterface>()
        .expect("the text services have a render interface");
    let app = UnitTestApplication::start(TestServices::styled_window().with_render_interface(render_interface));
    WindowServices { _app: app, _text: text }
}

/// Shows `target` in a window whose platform implementation exposes
/// `clipboard` as its clipboard.
fn window_with_clipboard(target: &Ref<SelectableTextBlock>, clipboard: &Rc<TestClipboardImpl>) -> Ref<Window> {
    let window_impl = MockWindowingPlatform::create_window_mock();
    window_impl.setup_feature::<dyn IClipboard>(clipboard.clipboard());
    show_in_window(target, window_impl)
}

/// Shows `target` in a window whose platform implementation has a
/// clipboard manager with `primary_selection` as the primary selection.
fn window_with_primary_selection(
    target: &Ref<SelectableTextBlock>,
    primary_selection: &Rc<TestClipboardImpl>,
) -> Ref<Window> {
    let window_impl = MockWindowingPlatform::create_window_mock();
    window_impl.setup_feature::<dyn IPlatformClipboardManagerImpl>(Rc::new(PlatformClipboardManager::new(
        None,
        Some(primary_selection.clipboard()),
    )));
    show_in_window(target, window_impl)
}

fn show_in_window(target: &Ref<SelectableTextBlock>, window_impl: Rc<MockWindowImpl>) -> Ref<Window> {
    let window = Window::with_impl(window_impl);
    window.set_content(Some(Control::boxed(target)));
    window.show();
    window
}

/// Runs the clipboard operations the text block posted to the dispatcher.
fn run_clipboard_operations() {
    Dispatcher::ui_thread().run_jobs(None);
}

fn root_with(target: &Ref<SelectableTextBlock>, client_size: Size) -> Ref<TestRoot> {
    let root = TestRoot::with_child(target.clone());
    root.set_client_size(client_size);

    root.measure(client_size);
    root.arrange(Rect::from_size(client_size));
    root.execute_initial_layout_pass();

    root
}

fn raise_key_down(target: &SelectableTextBlock, key: Key, modifiers: KeyModifiers) -> KeyEventArgs {
    let mut args = KeyEventArgs::new();
    args.set_routed_event(Some(InputElement::key_down_event()));
    args.key = key;
    args.key_modifiers = modifiers;
    target.raise_event(&args);
    args
}

// Content: Run("foo") + InlineUIContainer + Run("bar")
// The text of the inlines is "foo\u{FFFC}bar" (indices 0-6):
//   0='f', 1='o', 2='o', 3='\u{FFFC}' (embedded control), 4='b', 5='a', 6='r'
#[test]
fn selection_with_inline_ui_container_returns_correct_text() {
    let cases: [(i32, i32, &str); 7] = [
        // Entirely before the container.
        (0, 3, "foo"),
        // Exactly the character of the container.
        (3, 4, "\u{FFFC}"),
        // Up to and including the container (fencepost: last char before "bar").
        (0, 4, "foo\u{FFFC}"),
        // Starting exactly after the container (fencepost: first char of "bar").
        (4, 7, "bar"),
        // The container through the end.
        (3, 7, "\u{FFFC}bar"),
        // Spanning the container (one char either side).
        (2, 5, "o\u{FFFC}b"),
        // The entire content.
        (0, 7, "foo\u{FFFC}bar"),
    ];

    for (start, end, expected) in cases {
        let _scope = test_scope();
        let target = SelectableTextBlock::new();

        let inlines = target.inlines().unwrap();
        inlines.add(Run::with_text(Some("foo")));
        inlines.add(InlineUIContainer::with_child(Border::new()));
        inlines.add(Run::with_text(Some("bar")));

        target.measure(infinity());

        // The selection indices correspond to character positions of the
        // text layout. The embedded control run occupies 1 position and the
        // text of the inlines has a matching U+FFFC placeholder, so they
        // stay in sync.
        target.set_selection_start(start);
        target.set_selection_end(end);

        assert_eq!(target.selected_text(), expected, "selection {start}..{end}");
    }
}

#[test]
fn dragging_selection_should_reach_end_of_text_when_text_is_aligned() {
    for text_alignment in [TextAlignment::Center, TextAlignment::Right] {
        let _scope = test_scope();
        let target = SelectableTextBlock::new();
        target.set_width(200.0);
        target.set_text(Some("Aligned text"));
        target.set_text_alignment(text_alignment);

        let root = root_with(&target, Size::new(300.0, 100.0));

        let text_length = utf16_length(&target.text().unwrap());

        let first_character_bounds = target.text_layout().hit_test_text_position(0);
        let last_character_bounds = target.text_layout().hit_test_text_position(text_length - 1);
        let mouse = Mouse::new();
        let start_point = Point::new(
            first_character_bounds.x + first_character_bounds.width / 2.0,
            first_character_bounds.y + first_character_bounds.height / 2.0,
        );
        let end_point = Point::new(
            (target.bounds().width - 1.0).min(last_character_bounds.right() + 10.0),
            last_character_bounds.y + last_character_bounds.height / 2.0,
        );

        mouse.down(&target, MouseButton::Left, target.translate_point(start_point, &root).unwrap_or_default());
        mouse.move_(&target, target.translate_point(end_point, &root).unwrap_or_default());

        assert_eq!(text_length, target.selection_start().max(target.selection_end()), "{text_alignment:?}");
    }
}

#[test]
fn selection_foreground_should_not_reset_run_typeface_and_style() {
    let _scope = test_scope();
    let target = SelectableTextBlock::new();
    let selection_foreground_brush: Rc<dyn IBrush> = Brushes::red();
    target.set_selection_foreground_brush(Some(selection_foreground_brush.clone()));

    let run = Run::with_text(Some("Hello"));
    run.set_font_weight(FontWeight::Bold);
    run.set_font_style(FontStyle::Italic);
    run.set_font_size(20.0);

    target.inlines().unwrap().add(run.clone());

    target.measure(infinity());

    target.set_selection_start(0);
    target.set_selection_end(utf16_length(&run.text().unwrap()));

    target.measure(infinity());

    let text_layout = target.text_layout();

    let mut shaped_runs = 0;
    let mut checked = false;

    for text_line in text_layout.text_lines() {
        for text_run in text_line.text_runs().iter() {
            let Some(shaped_run) = text_run.downcast_ref::<ShapedTextRun>() else {
                continue;
            };

            shaped_runs += 1;

            if checked {
                continue;
            }
            checked = true;

            // The first shaped run is the selected one.
            let properties = shaped_run.run_properties();

            assert_eq!(FontWeight::Bold, properties.typeface().weight());
            assert_eq!(FontStyle::Italic, properties.typeface().style());

            let foreground_brush = properties.foreground_brush().expect("the selection foreground");
            assert!(std::ptr::addr_eq(Rc::as_ptr(&selection_foreground_brush), Rc::as_ptr(foreground_brush)));
        }
    }

    assert!(shaped_runs > 0);
}

#[test]
fn pointer_selection_is_published_to_primary_selection() {
    let _services = window_services();
    let primary_selection = TestClipboardImpl::new();

    let target = text_block("0123");
    let window = window_with_primary_selection(&target, &primary_selection);

    let mouse = Mouse::new();
    mouse.down(&target, MouseButton::Left, Point::new(1.0, 300.0));
    mouse.move_(&target, Point::new(700.0, 300.0));
    mouse.up(&target, MouseButton::Left, Point::new(700.0, 300.0));

    assert_eq!("0123", target.selected_text());

    run_clipboard_operations();
    let text = Rc::new(RefCell::new(None));
    let clipboard = window.try_get_clipboard(ClipboardType::PrimarySelection).expect("the window has a primary selection");
    let _ = Dispatcher::ui_thread().invoke_async_task_local({
        let text = text.clone();
        move || async move {
            *text.borrow_mut() = clipboard.try_get_text_async().await.expect("the text of the primary selection");
        }
    });
    run_clipboard_operations();
    assert_eq!(Some("0123".to_owned()), *text.borrow());
    assert_eq!(Some("0123".to_owned()), primary_selection.text());
    // The window has no default clipboard: only the primary selection.
    assert!(window.clipboard().is_none());
}

#[test]
fn inlines_changes_should_update_selection() {
    let _scope = test_scope();
    let target = SelectableTextBlock::new();
    let inlines = target.inlines().unwrap();
    inlines.add(Run::with_text(Some("foo")));
    target.set_selection_end(3);

    let selected_text_changed = Rc::new(Cell::new(false));
    let changed = selected_text_changed.clone();
    let _subscription = target.property_changed(move |e| {
        if e.property() == SelectableTextBlock::selected_text_property().as_property() {
            changed.set(true);
        }
    });

    inlines.add(Run::with_text(Some("bar")));

    assert!(selected_text_changed.get());

    target.set_selection_start(6);
    target.set_selection_end(0);
    inlines.remove_at(1);

    assert_eq!(3, target.selection_start());
    assert_eq!(0, target.selection_end());

    target.set_selection_start(0);
    target.set_selection_end(3);

    inlines.set(0, Run::with_text(Some("a")).upcast());

    assert_eq!(0, target.selection_start());
    assert_eq!(1, target.selection_end());
    assert_eq!("a", target.selected_text());
}

#[test]
fn text_changes_should_update_selection() {
    let _scope = test_scope();
    let target = text_block("foo");
    target.set_selection_end(3);

    let selected_text_changed = Rc::new(Cell::new(false));
    let changed = selected_text_changed.clone();
    let _subscription = target.property_changed(move |e| {
        if e.property() == SelectableTextBlock::selected_text_property().as_property() {
            changed.set(true);
        }
    });

    target.set_text(Some("a"));

    assert_eq!(0, target.selection_start());
    assert_eq!(1, target.selection_end());
    assert!(selected_text_changed.get());
}

#[test]
fn coerce_caret_index_on_text_changed() {
    let _scope = test_scope();
    let target = text_block("foo");
    target.set_selection_start(3);
    target.set_selection_end(3);

    target.set_text(Some("a"));

    assert_eq!(1, target.selection_start());
    assert_eq!(1, target.selection_end());
}

#[test]
fn can_copy_tracks_whether_selection_covers_any_character() {
    for (start, end, expected) in [(2, 2, false), (1, 3, true), (3, 1, true), (0, 4, true)] {
        let _scope = test_scope();
        let target = text_block("abcd");

        target.measure(infinity());

        target.set_selection_start(start);
        target.set_selection_end(end);

        assert_eq!(expected, target.can_copy(), "selection {start}..{end}");
    }
}

#[test]
fn can_copy_tracks_selection_over_inlines() {
    let _scope = test_scope();
    let target = SelectableTextBlock::new();

    let inlines = target.inlines().unwrap();
    inlines.add(Run::with_text(Some("foo")));
    inlines.add(Run::with_text(Some("bar")));

    target.measure(infinity());

    assert!(!target.can_copy());

    target.set_selection_start(2);
    target.set_selection_end(5);

    assert!(target.can_copy());

    target.clear_selection();

    assert!(!target.can_copy());
}

#[test]
fn right_click_below_text_should_keep_selection() {
    let _scope = test_scope();
    let target = text_block("first line\nsecond line\n");
    target.set_width(200.0);

    let root = root_with(&target, Size::new(300.0, 200.0));

    let mouse = Mouse::new();
    let first_character_bounds = target.text_layout().hit_test_text_position(0);
    let start = target
        .translate_point(
            Point::new(
                first_character_bounds.x + 1.0,
                first_character_bounds.y + first_character_bounds.height / 2.0,
            ),
            &root,
        )
        .unwrap_or_default();
    let below_text = target
        .translate_point(
            Point::new(target.text_layout().width() / 2.0, target.text_layout().height() + 10.0),
            &root,
        )
        .unwrap_or_default();

    mouse.down(&target, MouseButton::Left, start);
    mouse.move_(&target, below_text);
    mouse.up(&target, MouseButton::Left, below_text);

    mouse.down(&target, MouseButton::Right, below_text);
    mouse.up(&target, MouseButton::Right, below_text);

    assert_eq!(0, target.selection_start());
    assert_eq!(utf16_length(&target.text().unwrap()), target.selection_end());
    assert!(target.can_copy());
}

#[test]
fn right_click_on_unselected_text_should_move_selection() {
    let _scope = test_scope();
    let target = text_block("first line\nsecond line");
    target.set_width(200.0);

    let root = root_with(&target, Size::new(300.0, 200.0));

    target.set_selection_start(0);
    target.set_selection_end(5);

    let character_bounds = target.text_layout().hit_test_text_position(14);
    let on_unselected_text = target.translate_point(character_bounds.center(), &root).unwrap_or_default();
    let mouse = Mouse::new();

    mouse.down(&target, MouseButton::Right, on_unselected_text);
    mouse.up(&target, MouseButton::Right, on_unselected_text);

    assert_eq!(target.selection_start(), target.selection_end());
    assert!(!target.can_copy());
}

#[test]
fn should_shape_inlines_when_text_layout_is_created_between_content_change_and_measure() {
    let _scope = test_scope();

    let target = SelectableTextBlock::new();
    target.set_inlines(Some(InlineCollection::new()));

    target.measure(Size::new(1000.0, 1000.0));
    target.arrange(Rect::new(0.0, 0.0, 1000.0, 1000.0));

    let inlines = InlineCollection::new();
    inlines.add(Run::with_text(Some("Hello World")));
    target.set_inlines(Some(inlines));

    let _ = target.text_layout();

    target.measure(Size::new(1000.0, 1000.0));

    assert!(target.desired_size().width > 0.0, "DesiredSize was {:?}", target.desired_size());
}

// ---------------------------------------------------------------------------
// Tests that are not in the reference test class: they cover the members it
// leaves untested (keys, clipboard, click counts, focus).
// ---------------------------------------------------------------------------

#[test]
fn is_focusable_by_default() {
    let _scope = test_scope();
    let target = SelectableTextBlock::new();

    assert!(target.focusable());
}

#[test]
fn select_all_and_clear_selection() {
    let _scope = test_scope();
    let target = text_block("ab\u{1F600}");

    target.select_all();

    // The indices are UTF-16 code units: the emoji is a surrogate pair.
    assert_eq!(0, target.selection_start());
    assert_eq!(4, target.selection_end());
    assert_eq!("ab\u{1F600}", target.selected_text());

    target.set_selection_start(2);
    target.clear_selection();

    assert_eq!(2, target.selection_end());
    assert_eq!("", target.selected_text());
}

#[test]
fn selection_index_between_carriage_return_and_line_feed_moves_after_the_pair() {
    let _scope = test_scope();
    let target = text_block("a\r\nb");

    target.set_selection_end(2);
    assert_eq!(3, target.selection_end());

    target.set_selection_start(-1);
    assert_eq!(0, target.selection_start());

    target.set_selection_start(10);
    assert_eq!(4, target.selection_start());
}

#[test]
fn copy_hotkey_copies_the_selection_to_the_clipboard() {
    let _services = window_services();
    let clipboard = TestClipboardImpl::new();

    let target = text_block("abcd");
    let _window = window_with_clipboard(&target, &clipboard);

    target.set_selection_start(1);
    target.set_selection_end(3);

    let args = raise_key_down(&target, Key::C, KeyModifiers::CONTROL);

    assert!(args.handled());
    run_clipboard_operations();
    assert_eq!(1, clipboard.set_data_count());
    assert_eq!(Some("bc".to_owned()), clipboard.text());

    // A key that is no hotkey is left unhandled.
    let args = raise_key_down(&target, Key::C, KeyModifiers::NONE);

    assert!(!args.handled());
    run_clipboard_operations();
    assert_eq!(1, clipboard.set_data_count());
}

#[test]
fn copy_does_nothing_without_selection() {
    let _services = window_services();
    let clipboard = TestClipboardImpl::new();

    let target = text_block("abcd");
    let _window = window_with_clipboard(&target, &clipboard);

    target.copy();

    run_clipboard_operations();
    assert_eq!(0, clipboard.set_data_count());
}

#[test]
fn copy_does_nothing_without_a_top_level_or_a_clipboard() {
    // Not in a top-level.
    {
        let _scope = test_scope();
        let target = text_block("abcd");
        let _root = root_with(&target, Size::new(100.0, 100.0));
        target.set_selection_end(4);

        target.copy();
        Dispatcher::ui_thread().run_jobs(None);
    }

    // In a top-level whose platform has no clipboard.
    let _services = window_services();
    let target = text_block("abcd");
    let window = show_in_window(&target, MockWindowingPlatform::create_window_mock());
    target.set_selection_end(4);
    assert!(window.clipboard().is_none());

    target.copy();
    run_clipboard_operations();
}

#[test]
fn handling_copying_to_clipboard_cancels_the_copy() {
    let _services = window_services();
    let clipboard = TestClipboardImpl::new();

    let target = text_block("abcd");
    let _window = window_with_clipboard(&target, &clipboard);
    target.set_selection_end(4);

    let raised = Rc::new(Cell::new(0));
    let counter = raised.clone();
    let token = target.copying_to_clipboard(move |_, e| {
        counter.set(counter.get() + 1);
        e.set_handled(true);
    });

    target.copy();

    assert_eq!(1, raised.get());
    run_clipboard_operations();
    assert_eq!(0, clipboard.set_data_count());

    // Without the handler the selection is copied.
    target.remove_handler(SelectableTextBlock::copying_to_clipboard_event(), token);
    target.copy();
    run_clipboard_operations();
    assert_eq!(1, clipboard.set_data_count());
    assert_eq!(Some("abcd".to_owned()), clipboard.text());
}

/// A text block with the text "abcd" and "bc" selected, in a window whose
/// platform implementation exposes `clipboard`.
fn create_selectable_text_block_in_top_level(clipboard: &Rc<TestClipboardImpl>) -> (Ref<SelectableTextBlock>, Ref<Window>) {
    let target = text_block("abcd");
    target.set_selection_start(1);
    target.set_selection_end(3);

    let window = window_with_clipboard(&target, clipboard);

    assert!(target.can_copy());

    (target, window)
}

#[test]
fn copy_does_not_throw_when_clipboard_fails() {
    for exception_type in EXPECTED_CLIPBOARD_EXCEPTIONS {
        let _services = window_services();

        let clipboard_impl = TestClipboardImpl::throwing(exception_type);
        let (target, _window) = create_selectable_text_block_in_top_level(&clipboard_impl);
        let (messages, sink) = record_log_messages();

        target.copy();

        let unhandled = std::panic::catch_unwind(std::panic::AssertUnwindSafe(run_clipboard_operations));
        sink.dispose();

        assert!(unhandled.is_ok(), "{exception_type:?}");
        assert_eq!(1, clipboard_impl.set_data_count());
        assert_eq!(vec!["Failed to write text to clipboard: {Error}"], *messages.borrow(), "{exception_type:?}");
    }
}

// The reference fails the clipboard with an `InvalidOperationException` and
// expects it out of the dispatcher: the failure that is not one a platform
// clipboard is known for is the error of the kind `Other`.
#[test]
fn copy_does_not_swallow_unexpected_exceptions() {
    let _services = window_services();

    let clipboard_impl = TestClipboardImpl::throwing(ClipboardErrorKind::Other);
    let (target, _window) = create_selectable_text_block_in_top_level(&clipboard_impl);
    let (messages, sink) = record_log_messages();

    target.copy();

    let unhandled = std::panic::catch_unwind(std::panic::AssertUnwindSafe(run_clipboard_operations))
        .expect_err("the failure of the clipboard");
    sink.dispose();

    let expected = ClipboardHelper::unhandled_message(&ClipboardError::from_kind(ClipboardErrorKind::Other));
    assert_eq!(Some(expected.as_str()), panic_message(&*unhandled));
    assert_eq!(1, clipboard_impl.set_data_count());
    assert!(messages.borrow().is_empty());
}

// Not in the reference: a clipboard that panics is not swallowed either.
#[test]
fn copy_does_not_swallow_unexpected_panics() {
    let _services = window_services();

    let clipboard = TestClipboardImpl::failing(TestClipboardFailure::Panics);
    let (target, _window) = create_selectable_text_block_in_top_level(&clipboard);

    target.copy();

    let unhandled = std::panic::catch_unwind(std::panic::AssertUnwindSafe(run_clipboard_operations))
        .expect_err("the panic of the clipboard");

    assert_eq!(Some(&"The clipboard failed."), unhandled.downcast_ref::<&'static str>());
    assert_eq!(1, clipboard.set_data_count());
}

// Not in the reference: a clipboard whose operation never completes logs
// nothing and raises nothing.
#[test]
fn copy_waits_for_a_clipboard_that_never_completes() {
    let _services = window_services();

    let clipboard = TestClipboardImpl::failing(TestClipboardFailure::NeverCompletes);
    let (target, _window) = create_selectable_text_block_in_top_level(&clipboard);
    let (messages, sink) = record_log_messages();

    target.copy();

    let unhandled = std::panic::catch_unwind(std::panic::AssertUnwindSafe(run_clipboard_operations));
    sink.dispose();

    assert!(unhandled.is_ok());
    assert_eq!(1, clipboard.set_data_count());
    assert!(messages.borrow().is_empty());
}

// Not in the reference, which logs whatever the primary selection throws
// when a selection is published.
#[test]
fn failing_primary_selection_is_logged_when_a_selection_is_published() {
    let failures = EXPECTED_CLIPBOARD_EXCEPTIONS
        .into_iter()
        .chain([ClipboardErrorKind::Other])
        .map(TestClipboardFailure::Fails)
        .chain([TestClipboardFailure::Panics]);

    for failure in failures {
        let _services = window_services();
        let primary_selection = TestClipboardImpl::failing(failure);
        let target = text_block("0123");
        let _window = window_with_primary_selection(&target, &primary_selection);
        let (messages, sink) = record_log_messages();

        let mouse = crate::mouse_test_helper::MouseTestHelper::new();
        let unhandled = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            mouse.down_at(&target, MouseButton::Left, Point::new(1.0, 5.0), 1);
            mouse.move_(&target, Point::new(700.0, 5.0));
            mouse.up_at(&target, MouseButton::Left, Point::new(700.0, 5.0));
            run_clipboard_operations();
        }));
        sink.dispose();

        assert!(unhandled.is_ok(), "{failure:?}");
        assert_eq!(1, primary_selection.set_data_count(), "{failure:?}");
        assert_eq!(vec!["Failed to write text to primary selection: {Error}"], *messages.borrow(), "{failure:?}");
    }
}

#[test]
fn select_all_hotkey_selects_all_text() {
    let _services = window_services();
    let target = text_block("abcd");

    let args = raise_key_down(&target, Key::A, KeyModifiers::CONTROL);

    assert!(args.handled());
    assert_eq!(0, target.selection_start());
    assert_eq!(4, target.selection_end());
    assert!(target.can_copy());
}

#[test]
fn losing_focus_clears_the_selection() {
    let _scope = test_scope();
    let target = text_block("abcd");
    target.set_selection_start(1);
    target.set_selection_end(3);

    target.raise_event(&FocusChangedEventArgs::new(InputElement::lost_focus_event()));

    assert_eq!(1, target.selection_start());
    assert_eq!(1, target.selection_end());
    assert!(!target.can_copy());
}

// "hello world" at size 12: a character is 6 wide, so x = 8 is in 'e'
// (index 1), x = 44 in 'o' of "world" (index 7).
#[test]
fn click_counts_select_caret_word_and_all() {
    let _scope = test_scope();
    let target = text_block("hello world");
    let _root = root_with(&target, Size::new(300.0, 100.0));

    let mouse = Mouse::new();
    let position = Point::new(8.0, 5.0);

    mouse.down_with(&target, MouseButton::Left, position, KeyModifiers::NONE, 1);
    mouse.up(&target, MouseButton::Left, position);

    assert_eq!((1, 1), (target.selection_start(), target.selection_end()));

    mouse.down_with(&target, MouseButton::Left, position, KeyModifiers::NONE, 2);

    assert_eq!((0, 5), (target.selection_start(), target.selection_end()));
    assert_eq!("hello", target.selected_text());

    // Dragging after a double click extends the selection by words.
    mouse.move_(&target, Point::new(44.0, 5.0));

    assert_eq!((0, 11), (target.selection_start(), target.selection_end()));

    mouse.up(&target, MouseButton::Left, Point::new(44.0, 5.0));

    mouse.down_with(&target, MouseButton::Left, position, KeyModifiers::NONE, 3);
    mouse.up(&target, MouseButton::Left, position);

    assert_eq!((0, 11), (target.selection_start(), target.selection_end()));
}

#[test]
fn shift_click_extends_the_selection() {
    let _scope = test_scope();
    let target = text_block("hello world");
    let _root = root_with(&target, Size::new(300.0, 100.0));

    let mouse = Mouse::new();

    mouse.down(&target, MouseButton::Left, Point::new(13.0, 5.0));
    mouse.up(&target, MouseButton::Left, Point::new(13.0, 5.0));

    assert_eq!((2, 2), (target.selection_start(), target.selection_end()));

    mouse.down_with(&target, MouseButton::Left, Point::new(49.0, 5.0), KeyModifiers::SHIFT, 1);
    mouse.up(&target, MouseButton::Left, Point::new(49.0, 5.0));

    assert_eq!((2, 8), (target.selection_start(), target.selection_end()));
    assert_eq!("llo wo", target.selected_text());
}

#[test]
fn pointer_press_captures_the_pointer_and_release_frees_it() {
    let _scope = test_scope();
    let target = text_block("hello world");
    let _root = root_with(&target, Size::new(300.0, 100.0));

    let mouse = Mouse::new();

    mouse.down(&target, MouseButton::Left, Point::new(13.0, 5.0));
    assert!(mouse.pointer.captured().is_some_and(|captured| captured == target));

    // The mouse releases the capture itself after the event; the control
    // has released it by then.
    let released = Rc::new(Cell::new(false));
    let flag = released.clone();
    let pointer = mouse.pointer.clone();
    target.add_handler_with(
        InputElement::pointer_released_event(),
        move |_, _: &PointerReleasedEventArgs| flag.set(pointer.captured().is_none()),
        ferroui_base::interactivity::RoutingStrategies::BUBBLE,
        true,
    );

    mouse.up(&target, MouseButton::Left, Point::new(13.0, 5.0));

    assert!(released.get());
}
