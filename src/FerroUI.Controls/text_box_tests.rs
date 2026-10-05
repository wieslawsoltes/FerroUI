//! The tests of the text box.
//!
//! Every test runs under a unit test application: the key handling of the
//! text box reads the hotkeys of the platform settings of the application.
//! The tests that use a clipboard show the text box in a window whose
//! platform implementation exposes an in-memory clipboard. Pixel positions
//! are computed from the metrics of the test fonts (see `ferroui_base::media::text_formatting::testing`): every
//! glyph advances half an em and a line is 1.1 em high, so at the default
//! size of 12 a character is 6 wide and a line 13.2 high.
//!
//! `create_template` is the template of the reference tests (the caret index
//! of the presenter is bound both ways); the tests the reference runs with a
//! themed text box in a window use `create_themed_template`.

use crate::i_command_source::register_command_source;
use crate::mouse_test_helper::MouseTestHelper;
use crate::presenters::TextPresenter;
use crate::primitives::{ScrollBarVisibility, TemplatedControlImpl};
use crate::templates::{FuncControlTemplate, FuncTemplateNameScopeExtensions, IControlTemplate};
use crate::test_command::TestCommand;
use crate::test_support::TestRoot;
use crate::test_support_buttons::focus_scope;
use crate::testing::{
    MockWindowImpl, MockWindowingPlatform, TestClipboardFailure, TestClipboardImpl, TestLogSink, TestServices,
    UnitTestApplication, UnitTestApplicationScope,
};
use crate::utils::ClipboardHelper;
use crate::{
    Control, ControlImpl, HotKeyManager, PastingFromClipboardEventArgs, ScrollViewer, StackPanel,
    TextBlock, TextBox, TextBoxImpl, Window,
};
use ferroui_base::controls::NameScopeRef;
use ferroui_base::data::core::Maybe;
use ferroui_base::data::model::{Event, INotifyPropertyChanged, Model};
use ferroui_base::data::{
    BindingMode, BindingPriority, ReflectionBinding, RelativeSource, RelativeSourceMode, TemplateBinding,
};
use ferroui_base::input::platform::{
    ClipboardError, ClipboardErrorKind, ClipboardExtensions, ClipboardType, IClipboard, IPlatformClipboardManagerImpl,
    PlatformClipboardManager,
};
use ferroui_base::input::raw::{RawKeyEventArgs, RawKeyEventType};
use ferroui_base::input::text_input::{TextInputMethodClient, TextInputMethodClientRequestedEventArgs, TextSelection};
use ferroui_base::input::{
    ICommand, ICommandSource, IInputDevice, InputElement, InputElementImpl, Key, KeyDeviceType, KeyEventArgs,
    KeyGesture, KeyModifiers, MouseButton, PhysicalKey, RawInputModifiers, TextInputEventArgs,
};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::reactive::IDisposable;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::logical_tree::LogicalTreeAttachmentEventArgs;
use ferroui_base::media::text_formatting::testing::TextTestScope;
use ferroui_base::media::{Brushes, IBrush, TextWrapping};
use ferroui_base::platform::IPlatformRenderInterface;
use ferroui_base::reactive::ObservableExt;
use ferroui_base::threading::Dispatcher;
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_model, ferro_property, instantiate, BoxedValue, FerroObjectExtensions,
    FerroLocator, FerroObjectImpl, FerroObjectImplExt, IntoRef, LocatorExtensions, Point, Ref, Size,
    StyledElementImpl, StyledElementImplExt, StyledProperty, Thickness, VisualImpl,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

// --- helpers ------------------------------------------------------------------

fn infinity() -> Size {
    Size::new(f64::INFINITY, f64::INFINITY)
}

fn utf16_length(text: &str) -> i32 {
    text.encode_utf16().count() as i32
}

/// The line break of the platform.
fn new_line() -> &'static str {
    if cfg!(windows) {
        "\r\n"
    } else {
        "\n"
    }
}

fn template_binding(path: &str) -> Rc<ReflectionBinding> {
    let binding = ReflectionBinding::new(path).with_mode(BindingMode::TwoWay);
    binding.set_priority(BindingPriority::Template);
    binding.set_relative_source(Some(RelativeSource::new(RelativeSourceMode::TemplatedParent)));
    binding
}

fn build_template(_control: &Ref<TextBox>, scope: &NameScopeRef) -> Ref<Control> {
    let presenter = TextPresenter::new();
    presenter.set_name(Some("PART_TextPresenter".to_string()));
    presenter.bind_binding(TextPresenter::text_property().as_property(), &template_binding("Text"));
    presenter.bind_binding(TextPresenter::caret_index_property().as_property(), &template_binding("CaretIndex"));

    let scroll_viewer = ScrollViewer::new();
    scroll_viewer.set_name(Some("PART_ScrollViewer".to_string()));
    scroll_viewer.set_template(Some(FuncControlTemplate::for_type::<ScrollViewer>(
        crate::scroll_viewer_tests::create_template,
    )));
    scroll_viewer.set_content(Some(Control::boxed(presenter.register_in_name_scope(&**scope))));

    scroll_viewer.register_in_name_scope(&**scope).upcast()
}

/// The control template of the text boxes of the tests: a scroll viewer
/// named `PART_ScrollViewer` holding a text presenter named
/// `PART_TextPresenter` whose text and caret index are bound both ways to
/// the text box.
pub(crate) fn create_template() -> Option<Rc<dyn IControlTemplate>> {
    Some(FuncControlTemplate::for_type::<TextBox>(build_template))
}

fn build_themed_template(_control: &Ref<TextBox>, scope: &NameScopeRef) -> Ref<Control> {
    let presenter = TextPresenter::new();
    presenter.set_name(Some("PART_TextPresenter".to_string()));
    presenter.bind_binding(
        TextPresenter::text_property().as_property(),
        &TemplateBinding::new(TextBox::text_property().as_property()).with_mode(BindingMode::TwoWay),
    );

    let one_way = [
        (TextPresenter::caret_index_property().as_property(), TextBox::caret_index_property().as_property()),
        (TextPresenter::selection_start_property().as_property(), TextBox::selection_start_property().as_property()),
        (TextPresenter::selection_end_property().as_property(), TextBox::selection_end_property().as_property()),
        (TextPresenter::password_char_property().as_property(), TextBox::password_char_property().as_property()),
        (TextPresenter::reveal_password_property().as_property(), TextBox::reveal_password_property().as_property()),
    ];

    for (target, source) in one_way {
        presenter.bind_binding(target, &TemplateBinding::new(source));
    }

    let scroll_viewer = ScrollViewer::new();
    scroll_viewer.set_name(Some("PART_ScrollViewer".to_string()));
    scroll_viewer.set_template(Some(FuncControlTemplate::for_type::<ScrollViewer>(
        crate::scroll_viewer_tests::create_template,
    )));
    scroll_viewer.set_content(Some(Control::boxed(presenter.register_in_name_scope(&**scope))));

    scroll_viewer.register_in_name_scope(&**scope).upcast()
}

/// A control template with the bindings of the presenter in the default
/// theme of the reference: the text is bound both ways, the caret index, the
/// selection and the password properties follow the text box. For the tests
/// the reference runs with a themed text box in a window.
pub(crate) fn create_themed_template() -> Option<Rc<dyn IControlTemplate>> {
    Some(FuncControlTemplate::for_type::<TextBox>(build_themed_template))
}

/// A text box with the template of the tests and a text.
pub(crate) fn templated_text_box(text: Option<&str>) -> Ref<TextBox> {
    let target = TextBox::new();
    target.set_template(create_template());
    target.set_text(text);
    target
}

/// Raises a key down event on a text box.
pub(crate) fn raise_key_event(text_box: &TextBox, key: Key, input_modifiers: KeyModifiers) -> KeyEventArgs {
    let mut args = KeyEventArgs::new();
    args.set_routed_event(Some(InputElement::key_down_event()));
    args.key_modifiers = input_modifiers;
    args.key = key;

    text_box.raise_event(&args);

    args
}

/// Raises a text input event on a text box.
pub(crate) fn raise_text_event(text_box: &TextBox, text: &str) {
    let mut args = TextInputEventArgs::new();
    args.set_routed_event(Some(InputElement::text_input_event()));
    args.text = Some(text.to_string());

    text_box.raise_event(&args);
}

fn get_input_method_client(text_box: &TextBox) -> Rc<dyn TextInputMethodClient> {
    let event_args = TextInputMethodClientRequestedEventArgs::new();
    event_args.set_routed_event(Some(InputElement::text_input_method_client_requested_event()));
    text_box.raise_event(&event_args);

    event_args.client().expect("the text box provides an input method client")
}

/// The observable of the text of a text box.
fn text_observable(text_box: &TextBox) -> Rc<dyn ferroui_base::reactive::IObservable<Option<String>>> {
    let object: &ferroui_base::FerroObject = text_box;
    FerroObjectExtensions::get_observable(object, TextBox::text_property())
}

/// The services of a test: the test fonts under the application of the
/// windowing tests.
pub(crate) struct TestScope {
    _app: UnitTestApplicationScope,
    _text: TextTestScope,
}

/// Starts the services of a test for the lifetime of the returned value.
pub(crate) fn test_scope() -> TestScope {
    let text = TextTestScope::new();
    let render_interface = FerroLocator::current()
        .get_service::<dyn IPlatformRenderInterface>()
        .expect("the text services have a render interface");
    let app = UnitTestApplication::start(TestServices::styled_window().with_render_interface(render_interface));
    TestScope { _app: app, _text: text }
}

/// Shows `content` in a window with the given platform implementation.
pub(crate) fn show_in_window(content: impl IntoRef<Control>, window_impl: Rc<MockWindowImpl>) -> Ref<Window> {
    let window = Window::with_impl(window_impl);
    window.set_content(Some(Control::boxed(content)));
    window.show();
    window
}

/// Shows `content` in a window whose platform implementation exposes
/// `clipboard` as its clipboard.
pub(crate) fn window_with_clipboard(content: impl IntoRef<Control>, clipboard: &Rc<TestClipboardImpl>) -> Ref<Window> {
    let window_impl = MockWindowingPlatform::create_window_mock();
    window_impl.setup_feature::<dyn IClipboard>(clipboard.clipboard());
    show_in_window(content, window_impl)
}

/// Shows `content` in a window whose platform implementation has a
/// clipboard manager with a clipboard and `primary_selection` as the
/// primary selection.
fn window_with_primary_selection(
    content: impl IntoRef<Control>,
    primary_selection: &Rc<TestClipboardImpl>,
) -> Ref<Window> {
    let window_impl = MockWindowingPlatform::create_window_mock();
    window_impl.setup_feature::<dyn IPlatformClipboardManagerImpl>(Rc::new(PlatformClipboardManager::new(
        Some(TestClipboardImpl::new().clipboard()),
        Some(primary_selection.clipboard()),
    )));
    show_in_window(content, window_impl)
}

/// The primary selection of a window.
fn primary_selection_of(window: &Window) -> Rc<dyn IClipboard> {
    window.try_get_clipboard(ClipboardType::PrimarySelection).expect("the window has a primary selection")
}

/// Places a text on a clipboard and waits for the clipboard.
fn set_clipboard_text(clipboard: &Rc<dyn IClipboard>, text: &str) {
    let task = Dispatcher::ui_thread().to_task_scheduler().start_local(clipboard.set_text_async(Some(text)));
    Dispatcher::ui_thread().run_jobs(None);
    task.result().expect("the clipboard has answered").expect("the clipboard has taken the text");
}

/// Reads the text of a clipboard.
fn get_clipboard_text(clipboard: &Rc<dyn IClipboard>) -> Option<String> {
    let task = Dispatcher::ui_thread().to_task_scheduler().start_local(clipboard.try_get_text_async());
    Dispatcher::ui_thread().run_jobs(None);
    task.result().expect("the clipboard has answered").expect("the clipboard has delivered its text")
}

/// Hosts a text box the way the top level of the reference tests does: in a
/// root, with the initial layout pass executed.
fn host_in_root(target: &Ref<TextBox>) -> Ref<TestRoot> {
    let root = TestRoot::with_child(target.clone());
    root.execute_initial_layout_pass();
    root
}

struct Class1 {
    bar: RefCell<Option<String>>,
    property_changed: Event<str>,
}

impl Class1 {
    fn new(bar: Option<&str>) -> Rc<Self> {
        Model::new_model(Self { bar: RefCell::new(bar.map(str::to_owned)), property_changed: Event::new() })
    }

    fn bar(&self) -> Option<String> {
        self.bar.borrow().clone()
    }

    fn set_bar(&self, value: Option<String>) {
        *self.bar.borrow_mut() = value;
        self.property_changed.raise("Bar");
    }
}

impl INotifyPropertyChanged for Class1 {
    fn property_changed(&self) -> &Event<str> {
        &self.property_changed
    }
}

ferro_model!(Class1, |b| b
    .notify_property_changed()
    .property::<Maybe<String>>("Bar", |o| o.bar(), |o, v| o.set_bar(v)));

// --- tests --------------------------------------------------------------------

/// The `TestContextMenu` of the reference: a context menu that says it is
/// open.
fn test_context_menu() -> Ref<crate::ContextMenu> {
    let menu = crate::ContextMenu::new();
    menu.set_is_open(true);
    menu
}

#[test]
fn opening_context_menu_does_not_lose_selection() {
    let _scope = test_scope();
    let _focus = focus_scope();
    let target1 = templated_text_box(Some("1234"));
    target1.set_context_menu(&test_context_menu());

    let target2 = templated_text_box(Some("5678"));

    let sp = StackPanel::new();
    sp.children().add(target1.clone());
    sp.children().add(target2.clone());

    target1.apply_template();
    target2.apply_template();

    let _root = TestRoot::with_child(sp);

    target1.set_selection_start(0);
    target1.set_selection_end(3);

    target1.focus();
    assert!(!target2.is_focused());
    assert!(target1.is_focused());

    target2.focus();

    assert_eq!("123", target1.selected_text());
}

#[test]
fn text_box_should_lose_focus_when_disabled() {
    let _scope = test_scope();
    let _focus = focus_scope();
    let target = templated_text_box(None);

    target.apply_template();

    let _root = TestRoot::with_child(target.clone());

    target.focus();
    assert!(target.is_focused());
    target.set_is_enabled(false);
    assert!(!target.is_focused());
    assert!(!target.is_enabled());
}

// `Opening_Context_Flyout_Does_not_Lose_Selection` of the reference. The
// text box is shown in a window: a flyout needs a top-level to open in.
#[test]
fn opening_context_flyout_does_not_lose_selection() {
    let _scope = test_scope();
    let _focus = focus_scope();

    let flyout = crate::MenuFlyout::new();
    for header in ["Item 1", "Item 2", "Item 3"] {
        let item = crate::MenuItem::new();
        item.set_header(crate::test_support::boxed_str(header));
        flyout.items().add(Some(Control::boxed(item)));
    }

    let target1 = templated_text_box(Some("1234"));
    target1.set_context_flyout(&flyout);

    target1.apply_template();

    let _root = show_in_window(&target1, MockWindowingPlatform::create_window_mock());

    target1.set_selection_start(0);
    target1.set_selection_end(3);

    target1.focus();
    assert!(target1.is_focused());

    target1.context_flyout().expect("the context flyout").show_at(&target1);

    assert!(flyout.is_open());
    assert_eq!("123", target1.selected_text());
}

#[test]
fn default_binding_mode_should_be_two_way() {
    let _scope = test_scope();

    assert_eq!(
        BindingMode::TwoWay,
        TextBox::text_property().get_metadata(TextBox::TYPE).default_binding_mode()
    );
}

#[test]
fn text_box_ignore_word_move_in_password_field() {
    let _scope = test_scope();
    let target = TextBox::new();
    target.set_template(create_template());
    target.set_password_char('*');
    target.set_text(Some("passw0rd"));

    target.apply_template();
    target.measure(infinity());
    target.set_caret_index(8);
    raise_key_event(&target, Key::Left, KeyModifiers::CONTROL);

    assert_eq!(7, target.caret_index());
}

#[test]
fn caret_index_can_moved_to_position_after_the_end_of_text_with_arrow_key() {
    let _scope = test_scope();
    let target = templated_text_box(Some("1234"));

    target.apply_template();

    target.measure(infinity());

    target.set_caret_index(3);
    raise_key_event(&target, Key::Right, KeyModifiers::NONE);

    assert_eq!(4, target.caret_index());
}

#[test]
fn control_backspace_should_set_caret_position_to_the_start_of_the_deletion() {
    let _scope = test_scope();
    let target = templated_text_box(Some("First Second Third"));
    target.set_selection_start(13);
    target.set_selection_end(13);

    target.set_caret_index(10);
    target.apply_template();

    // (First Second |Third)
    raise_key_event(&target, Key::Back, KeyModifiers::CONTROL);
    // (First |Third)

    assert_eq!(6, target.caret_index());
}

#[test]
fn control_backspace_should_remove_the_double_whitespace_if_caret_index_was_at_the_end_of_a_word() {
    let _scope = test_scope();
    let target = templated_text_box(Some("First Second Third"));
    target.set_selection_start(12);
    target.set_selection_end(12);

    target.apply_template();

    // (First Second| Third)
    raise_key_event(&target, Key::Back, KeyModifiers::CONTROL);
    // (First| Third)

    assert_eq!(Some("First Third"), target.text().as_deref());
}

#[test]
fn control_backspace_undo_should_return_caret_position() {
    let _scope = test_scope();
    let target = templated_text_box(Some("First Second Third"));
    target.set_selection_start(9);
    target.set_selection_end(9);

    target.apply_template();

    // (First Second| Third)
    raise_key_event(&target, Key::Back, KeyModifiers::CONTROL);
    // (First| Third)

    target.undo();
    // (First Second| Third)

    assert_eq!(9, target.caret_index());
}

#[test]
fn press_ctrl_a_select_all_text() {
    let _scope = test_scope();
    let target = templated_text_box(Some("1234"));

    target.apply_template();

    raise_key_event(&target, Key::A, KeyModifiers::CONTROL);

    assert_eq!(0, target.selection_start());
    assert_eq!(4, target.selection_end());
}

#[test]
fn press_ctrl_a_select_all_null_text() {
    let _scope = test_scope();
    let target = templated_text_box(None);

    raise_key_event(&target, Key::A, KeyModifiers::CONTROL);

    assert_eq!(0, target.selection_start());
    assert_eq!(0, target.selection_end());
}

#[test]
fn press_ctrl_z_will_not_modify_text() {
    let _scope = test_scope();
    let target = templated_text_box(Some("1234"));

    raise_key_event(&target, Key::Z, KeyModifiers::CONTROL);

    assert_eq!(Some("1234"), target.text().as_deref());
}

#[test]
fn control_backspace_should_remove_the_word_before_the_caret_if_there_is_no_selection() {
    let _scope = test_scope();
    let text_box = templated_text_box(Some("First Second Third Fourth"));
    text_box.set_selection_start(5);
    text_box.set_selection_end(5);

    text_box.apply_template();

    // (First| Second Third Fourth)
    raise_key_event(&text_box, Key::Back, KeyModifiers::CONTROL);
    assert_eq!(Some(" Second Third Fourth"), text_box.text().as_deref());

    // ( Second |Third Fourth)
    text_box.set_caret_index(8);
    raise_key_event(&text_box, Key::Back, KeyModifiers::CONTROL);
    assert_eq!(Some(" Third Fourth"), text_box.text().as_deref());

    // ( Thi|rd Fourth)
    text_box.set_caret_index(4);
    raise_key_event(&text_box, Key::Back, KeyModifiers::CONTROL);
    assert_eq!(Some(" rd Fourth"), text_box.text().as_deref());

    // ( rd F[ou]rth)
    text_box.set_selection_start(5);
    text_box.set_selection_end(7);

    raise_key_event(&text_box, Key::Back, KeyModifiers::CONTROL);
    assert_eq!(Some(" rd Frth"), text_box.text().as_deref());

    // ( |rd Frth)
    text_box.set_caret_index(1);
    raise_key_event(&text_box, Key::Back, KeyModifiers::CONTROL);
    assert_eq!(Some("rd Frth"), text_box.text().as_deref());
}

#[test]
fn control_delete_should_remove_the_word_after_the_caret_if_there_is_no_selection() {
    let _scope = test_scope();
    let text_box = templated_text_box(Some("First Second Third Fourth"));
    text_box.set_caret_index(19);

    text_box.apply_template();

    // (First Second Third |Fourth)
    raise_key_event(&text_box, Key::Delete, KeyModifiers::CONTROL);
    assert_eq!(Some("First Second Third "), text_box.text().as_deref());

    // (First Second |Third )
    text_box.set_caret_index(13);
    raise_key_event(&text_box, Key::Delete, KeyModifiers::CONTROL);
    assert_eq!(Some("First Second "), text_box.text().as_deref());

    // (First Sec|ond )
    text_box.set_caret_index(9);
    raise_key_event(&text_box, Key::Delete, KeyModifiers::CONTROL);
    assert_eq!(Some("First Sec"), text_box.text().as_deref());

    // (Fi[rs]t Sec )
    text_box.set_selection_start(2);
    text_box.set_selection_end(4);

    raise_key_event(&text_box, Key::Delete, KeyModifiers::CONTROL);
    assert_eq!(Some("Fit Sec"), text_box.text().as_deref());

    // (Fit Sec| )
    let text = text_box.text().unwrap_or_default() + " ";
    text_box.set_text(Some(&text));
    text_box.set_caret_index(7);
    raise_key_event(&text_box, Key::Delete, KeyModifiers::CONTROL);
    assert_eq!(Some("Fit Sec"), text_box.text().as_deref());
}

#[test]
fn setting_selection_start_to_selection_end_sets_caret_position_to_selection_start() {
    let _scope = test_scope();
    let text_box = TextBox::new();
    text_box.set_text(Some("0123456789"));

    text_box.set_selection_start(2);
    text_box.set_selection_end(2);

    assert_eq!(2, text_box.caret_index());
}

#[test]
fn setting_text_updates_caret_position() {
    let _scope = test_scope();
    let target = TextBox::new();
    target.set_text(Some("Initial Text"));
    target.set_caret_index(11);

    let invoked = Rc::new(Cell::new(false));
    let skipped = Rc::new(Cell::new(false));

    let (weak, invoked_in, skipped_in) = (target.downgrade(), invoked.clone(), skipped.clone());
    let _subscription = text_observable(&target).subscribe_fn(move |_| {
        // The first value is the current one (`Skip(1)` in the reference).
        if !skipped_in.replace(true) {
            return;
        }

        // The caret index should be set before the text changed notification, as we don't want
        // to notify with an invalid caret index.
        assert_eq!(7, weak.upgrade().unwrap().caret_index());
        invoked_in.set(true);
    });

    target.set_text(Some("Changed"));

    assert!(invoked.get());
}

#[test]
fn press_enter_does_not_accept_return() {
    let _scope = test_scope();
    let target = TextBox::new();
    target.set_template(create_template());
    target.set_accepts_return(false);
    target.set_text(Some("1234"));

    target.apply_template();

    raise_key_event(&target, Key::Enter, KeyModifiers::NONE);

    assert_eq!(Some("1234"), target.text().as_deref());
}

#[test]
fn press_enter_add_default_newline() {
    let _scope = test_scope();
    let target = templated_text_box(None);
    target.set_accepts_return(true);

    target.apply_template();

    raise_key_event(&target, Key::Enter, KeyModifiers::NONE);

    assert_eq!(Some(new_line()), target.text().as_deref());
}

#[test]
fn press_enter_add_custom_newline() {
    let _scope = test_scope();
    let target = templated_text_box(None);
    target.set_accepts_return(true);
    target.set_new_line("Test");

    target.apply_template();

    raise_key_event(&target, Key::Enter, KeyModifiers::NONE);

    assert_eq!(Some("Test"), target.text().as_deref());
}

#[test]
fn has_correct_horizontal_scroll_bar_visibility() {
    let cases = [
        (false, TextWrapping::NoWrap, ScrollBarVisibility::Hidden),
        (false, TextWrapping::Wrap, ScrollBarVisibility::Disabled),
        (true, TextWrapping::NoWrap, ScrollBarVisibility::Auto),
        (true, TextWrapping::Wrap, ScrollBarVisibility::Disabled),
    ];

    for (accepts_return, wrapping, expected) in cases {
        let _scope = test_scope();
        let target = TextBox::new();
        target.set_accepts_return(accepts_return);
        target.set_text_wrapping(wrapping);

        assert_eq!(
            expected,
            ScrollViewer::get_horizontal_scroll_bar_visibility(&target),
            "{accepts_return} {wrapping:?}"
        );
    }
}

#[test]
fn selection_end_doesnt_cause_exception() {
    let _scope = test_scope();
    let target = templated_text_box(Some("0123456789"));

    target.apply_template();

    target.set_selection_start(0);
    target.set_selection_end(9);

    target.set_text(Some("123"));

    raise_text_event(&target, "456");
}

#[test]
fn selection_start_doesnt_cause_exception() {
    let _scope = test_scope();
    let target = templated_text_box(Some("0123456789"));

    target.apply_template();

    target.set_selection_start(8);
    target.set_selection_end(9);

    target.set_text(Some("123"));

    raise_text_event(&target, "456");
}

#[test]
fn selection_start_end_are_valid_ater_text_change() {
    let _scope = test_scope();
    let target = templated_text_box(Some("0123456789"));

    target.set_selection_start(8);
    target.set_selection_end(9);

    target.set_text(Some("123"));

    assert!(target.selection_start() <= 3);
    assert!(target.selection_end() <= 3);
}

#[test]
fn selected_text_changes_on_selection_change() {
    let _scope = test_scope();
    let target = templated_text_box(Some("0123456789"));

    target.apply_template();

    assert_eq!("", target.selected_text());

    target.set_selection_start(2);
    target.set_selection_end(4);

    assert_eq!("23", target.selected_text());
}

#[test]
fn selected_text_edits_text() {
    let _scope = test_scope();
    let target = templated_text_box(Some("0123"));

    target.apply_template();

    target.set_selected_text(Some("AA"));
    assert_eq!(Some("AA0123"), target.text().as_deref());

    target.set_selection_start(1);
    target.set_selection_end(3);
    target.set_selected_text(Some("BB"));

    assert_eq!(Some("ABB123"), target.text().as_deref());
}

#[test]
fn selected_text_can_clear_text() {
    let _scope = test_scope();
    let target = templated_text_box(Some("0123"));
    target.set_selection_start(1);
    target.set_selection_end(3);
    target.set_selected_text(Some(""));

    assert_eq!(Some("03"), target.text().as_deref());
}

#[test]
fn selected_text_null_clears_text() {
    let _scope = test_scope();
    let target = templated_text_box(Some("0123"));
    target.set_selection_start(1);
    target.set_selection_end(3);
    target.set_selected_text(None);

    assert_eq!(Some("03"), target.text().as_deref());
}

#[test]
fn coerce_caret_index_doesnt_cause_exception_with_malformed_line_ending() {
    let _scope = test_scope();
    let target = templated_text_box(Some("0123456789\r"));
    target.set_caret_index(11);
}

#[test]
fn textbox_doesnt_crash_when_receives_input_and_template_not_applied() {
    for key in [Key::Up, Key::Down, Key::Home, Key::End] {
        let _scope = test_scope();
        let _focus = focus_scope();
        let target1 = templated_text_box(Some("1234"));

        let _root = TestRoot::with_child(target1.clone());

        target1.focus();
        assert!(target1.is_focused());

        raise_key_event(&target1, key, KeyModifiers::NONE);
    }
}

/// Two templated text boxes in a stack panel in a root.
fn two_text_boxes(text1: Option<&str>, text2: Option<&str>) -> (Ref<TextBox>, Ref<TextBox>, Ref<TestRoot>) {
    let target1 = templated_text_box(text1);
    let target2 = templated_text_box(text2);
    let sp = StackPanel::new();
    sp.children().add(target1.clone());
    sp.children().add(target2.clone());

    target1.apply_template();
    target2.apply_template();

    let root = TestRoot::with_child(sp);

    (target1, target2, root)
}

#[test]
fn text_box_got_focus_and_lost_focus_work_properly() {
    let _scope = test_scope();
    let _focus = focus_scope();
    let (target1, target2, _root) = two_text_boxes(Some("1234"), Some("5678"));

    let gfcount = Rc::new(Cell::new(0));
    let lfcount = Rc::new(Cell::new(0));

    let count = gfcount.clone();
    target1.got_focus(move |_, _| count.set(count.get() + 1));
    let count = lfcount.clone();
    target2.lost_focus(move |_, _| count.set(count.get() + 1));

    target2.focus();
    assert!(!target1.is_focused());
    assert!(target2.is_focused());

    target1.focus();
    assert!(!target2.is_focused());
    assert!(target1.is_focused());

    assert_eq!(1, gfcount.get());
    assert_eq!(1, lfcount.get());
}

#[test]
fn text_box_caret_index_persists_when_focus_lost() {
    let _scope = test_scope();
    let _focus = focus_scope();
    let (target1, target2, _root) = two_text_boxes(Some("1234"), Some("5678"));

    target2.focus();
    target2.set_caret_index(2);
    assert!(!target1.is_focused());
    assert!(target2.is_focused());

    target1.focus();

    assert_eq!(2, target2.caret_index());
}

#[test]
fn text_box_reveal_password_reset_when_lost_focus() {
    let _scope = test_scope();
    let _focus = focus_scope();
    let (target1, target2, _root) = two_text_boxes(Some("1234"), Some("5678"));
    target1.set_password_char('*');

    target1.focus();
    target1.set_reveal_password(true);

    target2.focus();

    assert!(!target1.reveal_password());
}

#[test]
fn setting_bound_text_to_null_works() {
    let _scope = test_scope();
    let source = Class1::new(Some("bar"));
    let target = templated_text_box(None);
    target.set_data_context(Some(source.clone()));

    target.apply_template();

    target.bind_binding(TextBox::text_property().as_property(), &ReflectionBinding::new("Bar"));

    assert_eq!(Some("bar"), target.text().as_deref());
    source.set_bar(None);
    assert_eq!(None, target.text());
}

#[test]
fn max_length_works_properly() {
    let cases = [
        ("abc", "d", 3, 0, 0, false, "abc"),
        ("abc", "dd", 4, 3, 3, false, "abcd"),
        ("abc", "ddd", 3, 0, 2, true, "ddc"),
        ("abc", "dddd", 4, 1, 3, true, "addd"),
        ("abc", "ddddd", 5, 3, 3, true, "abcdd"),
    ];

    for (inital_text, text_input, max_length, selection_start, selection_end, from_clipboard, expected) in cases {
        let _scope = test_scope();
        let clipboard = TestClipboardImpl::new();
        let target = templated_text_box(Some(inital_text));
        target.set_max_length(max_length);
        target.set_selection_start(selection_start);
        target.set_selection_end(selection_end);

        let _top_level = window_with_clipboard(&target, &clipboard);

        target.measure(infinity());

        if from_clipboard {
            clipboard.set_text(Some(text_input));

            raise_key_event(&target, Key::V, KeyModifiers::CONTROL);
            clipboard.set_text(None);
        } else {
            raise_text_event(&target, text_input);
        }

        assert_eq!(Some(expected), target.text().as_deref(), "{text_input}");
    }
}

#[test]
fn keys_allow_undo() {
    let cases = [
        (Key::X, KeyModifiers::CONTROL),
        (Key::Back, KeyModifiers::NONE),
        (Key::Delete, KeyModifiers::NONE),
        (Key::Tab, KeyModifiers::NONE),
        (Key::Enter, KeyModifiers::NONE),
    ];

    for (key, modifiers) in cases {
        let _scope = test_scope();
        let clipboard = TestClipboardImpl::new();
        let target = templated_text_box(Some("0123"));
        target.set_accepts_return(true);
        target.set_accepts_tab(true);

        let _top_level = window_with_clipboard(&target, &clipboard);

        target.apply_template();
        target.set_selection_start(1);
        target.set_selection_end(3);

        raise_key_event(&target, key, modifiers);
        assert_ne!(Some("0123"), target.text().as_deref(), "{key:?}");
        raise_key_event(&target, Key::Z, KeyModifiers::CONTROL); // undo
        assert_eq!(Some("0123"), target.text().as_deref(), "{key:?}");
    }
}

/// The values of the text of a text box, from the current one on.
fn record_text(target: &TextBox) -> Rc<RefCell<Vec<Option<String>>>> {
    let values = Rc::new(RefCell::new(Vec::new()));
    let recorded = values.clone();
    text_observable(target).subscribe_fn(move |x| recorded.borrow_mut().push(x));
    values
}

#[test]
fn setting_selected_text_should_fire_single_text_changed_notification() {
    let _scope = test_scope();
    let target = templated_text_box(Some("0123"));
    target.set_accepts_return(true);
    target.set_accepts_tab(true);
    target.set_selection_start(1);
    target.set_selection_end(3);

    let values = record_text(&target);

    target.set_selected_text(Some("A"));

    assert_eq!(vec![Some("0123".to_string()), Some("0A3".to_string())], *values.borrow());
}

#[test]
fn entering_text_with_selected_text_should_fire_single_text_changed_notification() {
    let _scope = test_scope();
    let target = templated_text_box(Some("0123"));
    target.set_accepts_return(true);
    target.set_accepts_tab(true);
    target.set_selection_start(1);
    target.set_selection_end(3);

    let values = record_text(&target);

    raise_text_event(&target, "A");

    assert_eq!(vec![Some("0123".to_string()), Some("0A3".to_string())], *values.borrow());
}

#[test]
fn insert_multiline_text_should_accept_extra_lines_when_accepts_return_is_true() {
    let _scope = test_scope();
    let target = TextBox::new();
    target.set_accepts_return(true);

    let text = format!("123 {}456", new_line());

    raise_text_event(&target, &text);

    assert_eq!(Some(text), target.text());
}

#[test]
fn insert_multiline_text_should_discard_extra_lines_when_accepts_return_is_false() {
    let _scope = test_scope();
    let target = TextBox::new();
    target.set_accepts_return(false);

    raise_text_event(&target, "123 \r456");

    assert_eq!(Some("123 "), target.text().as_deref());

    target.set_text(Some(""));

    raise_text_event(&target, "123 \r\n456");

    assert_eq!(Some("123 "), target.text().as_deref());
}

#[test]
fn should_fullfill_max_lines_contraint() {
    let _scope = test_scope();
    let clipboard = TestClipboardImpl::new();
    let target = templated_text_box(Some("ABC"));
    target.set_max_lines(1);
    target.set_accepts_return(true);

    let _top_level = window_with_clipboard(&target, &clipboard);

    target.apply_template();
    target.measure(infinity());

    let initial_height = target.desired_size().height;

    clipboard.set_text(Some(new_line()));

    raise_key_event(&target, Key::V, KeyModifiers::CONTROL);
    clipboard.set_text(None);

    raise_text_event(&target, new_line());

    assert_eq!(Some(3), target.text().map(|text| text.lines().count()));

    target.invalidate_measure();
    target.measure(infinity());

    assert_eq!(initial_height, target.desired_size().height);
}

/// The presenter and the scroll viewer of a text box whose template is
/// applied.
fn template_parts(target: &TextBox) -> (Ref<TextPresenter>, Ref<ScrollViewer>) {
    let text_presenter = target.find_descendant_of_type::<TextPresenter>(false).expect("the presenter");
    assert_eq!(Some("PART_TextPresenter"), text_presenter.name().as_deref());

    let scroll_viewer = target.find_descendant_of_type::<ScrollViewer>(false).expect("the scroll viewer");
    assert_eq!(Some("PART_ScrollViewer"), scroll_viewer.name().as_deref());

    (text_presenter, scroll_viewer)
}

#[test]
fn max_lines_sets_scroll_viewer_max_height() {
    for max_lines in [1, 2, 3] {
        let _scope = test_scope();
        let target = templated_text_box(None);
        target.set_max_lines(max_lines);

        // Define an explicit whole number line height for predictable calculations.
        target.set_line_height(20.0);

        let _root = host_in_root(&target);

        let (text_presenter, scroll_viewer) = template_parts(&target);
        assert_eq!(Thickness::uniform(0.0), text_presenter.margin()); // The test assumes no margin on the presenter.

        assert_eq!(f64::from(max_lines) * target.line_height(), scroll_viewer.max_height());
    }
}

#[test]
fn max_lines_sets_scroll_viewer_max_height_with_text_presenter_margin() {
    for max_lines in [1, 2, 3] {
        let _scope = test_scope();
        let target = templated_text_box(None);
        target.set_max_lines(max_lines);

        // Define an explicit whole number line height for predictable calculations.
        target.set_line_height(20.0);

        let _root = host_in_root(&target);

        let (text_presenter, scroll_viewer) = template_parts(&target);
        let text_presenter_margin = Thickness::symmetric(0.0, 3.0);
        text_presenter.set_margin(text_presenter_margin);

        target.invalidate_measure();
        target.measure(infinity());

        assert_eq!(
            (f64::from(max_lines) * target.line_height()) + text_presenter_margin.top + text_presenter_margin.bottom,
            scroll_viewer.max_height()
        );
    }
}

#[test]
fn should_fullfill_min_lines_contraint() {
    let _scope = test_scope();
    let target = templated_text_box(Some("ABC \n DEF \n GHI"));
    target.set_min_lines(3);
    target.set_accepts_return(true);

    let _root = host_in_root(&target);

    target.apply_template();
    target.measure(infinity());

    let initial_height = target.desired_size().height;

    // Three lines of 13.2 (the metrics of the test fonts), rounded up.
    assert_eq!(40.0, initial_height);

    target.set_text(Some(""));

    target.invalidate_measure();
    target.measure(infinity());

    assert_eq!(initial_height, target.desired_size().height);
}

#[test]
fn min_lines_sets_scroll_viewer_min_height() {
    for min_lines in [1, 2, 3] {
        let _scope = test_scope();
        let target = templated_text_box(None);
        target.set_min_lines(min_lines);

        // Define an explicit whole number line height for predictable calculations.
        target.set_line_height(20.0);

        let _root = host_in_root(&target);

        let (text_presenter, scroll_viewer) = template_parts(&target);
        assert_eq!(Thickness::uniform(0.0), text_presenter.margin()); // The test assumes no margin on the presenter.

        assert_eq!(f64::from(min_lines) * target.line_height(), scroll_viewer.min_height());
    }
}

#[test]
fn min_lines_sets_scroll_viewer_min_height_with_text_presenter_margin() {
    for min_lines in [1, 2, 3] {
        let _scope = test_scope();
        let target = templated_text_box(None);
        target.set_min_lines(min_lines);

        // Define an explicit whole number line height for predictable calculations.
        target.set_line_height(20.0);

        let _root = host_in_root(&target);

        let (text_presenter, scroll_viewer) = template_parts(&target);
        let text_presenter_margin = Thickness::symmetric(0.0, 3.0);
        text_presenter.set_margin(text_presenter_margin);

        target.invalidate_measure();
        target.measure(infinity());

        assert_eq!(
            (f64::from(min_lines) * target.line_height()) + text_presenter_margin.top + text_presenter_margin.bottom,
            scroll_viewer.min_height()
        );
    }
}

#[test]
fn line_count_is_correct() {
    let cases = [(None, 1), (Some(""), 1), (Some("Hello"), 1), (Some("Hello\r\nWorld"), 2)];

    for (text, line_count) in cases {
        let _scope = test_scope();
        let target = templated_text_box(text);
        target.set_accepts_return(true);

        let _root = host_in_root(&target);

        target.apply_template();
        target.measure(infinity());

        assert_eq!(line_count, target.get_line_count(), "{text:?}");
    }
}

#[test]
fn unmeasured_text_box_has_negative_line_count() {
    let _scope = test_scope();
    let b = TextBox::new();
    assert_eq!(-1, b.get_line_count());
}

#[test]
fn line_count_is_correct_after_text_change() {
    let _scope = test_scope();
    let target = templated_text_box(Some("Hello"));
    target.set_accepts_return(true);

    let _root = host_in_root(&target);

    target.apply_template();
    target.measure(infinity());

    assert_eq!(1, target.get_line_count());

    target.set_text(Some("Hello\r\nWorld"));

    assert_eq!(2, target.get_line_count());
}

#[test]
fn visible_line_count_does_not_affect_line_count() {
    let _scope = test_scope();
    let target = templated_text_box(Some("Hello\r\nWorld\r\nHello\r\nFerro"));
    target.set_accepts_return(true);
    target.set_max_lines(2);

    let _root = host_in_root(&target);

    target.apply_template();
    target.measure(infinity());

    assert_eq!(4, target.get_line_count());
}

#[test]
fn can_undo_can_redo_is_false_when_initialized() {
    let _scope = test_scope();
    let tb = templated_text_box(Some("New Text"));

    tb.measure(infinity());

    assert!(!tb.can_undo());
    assert!(!tb.can_redo());
}

/// Types "ABC", "DEF" and "123" with a space key between them.
///
/// Undo/redo snapshots are taken on every space, but only when the space is
/// handled as a key down (spaces of text input events do not count), and
/// every 7 characters in a long word. The spaces do not add spaces: they
/// are sent only as key events and not as text events.
fn type_three_words(tb: &TextBox) {
    raise_text_event(tb, "ABC");
    raise_key_event(tb, Key::Space, KeyModifiers::NONE);
    raise_text_event(tb, "DEF");
    raise_key_event(tb, Key::Space, KeyModifiers::NONE);
    raise_text_event(tb, "123");
}

#[test]
fn can_undo_can_redo_and_programmatic_undo_redo_works() {
    let _scope = test_scope();
    let tb = templated_text_box(None);

    tb.measure(infinity());

    type_three_words(&tb);

    assert_eq!(Some("ABCDEF123"), tb.text().as_deref());

    assert!(tb.can_undo());

    tb.undo();

    // Undo will take us back one step.
    assert_eq!(Some("ABCDEF"), tb.text().as_deref());

    assert!(tb.can_redo());

    tb.redo();

    // Redo should restore us.
    assert_eq!(Some("ABCDEF123"), tb.text().as_deref());
}

/// A text box bound both ways to `Bar` of a source, after " edit", a space
/// key and " more" were typed at the end of the text "initial".
fn edited_bound_text_box() -> (Rc<Class1>, Ref<TextBox>) {
    let source = Class1::new(Some("initial"));
    let text_box = templated_text_box(None);
    text_box.set_data_context(Some(source.clone()));

    text_box.bind_binding(
        TextBox::text_property().as_property(),
        &ReflectionBinding::new("Bar").with_mode(BindingMode::TwoWay),
    );
    text_box.measure(infinity());
    text_box.set_caret_index(utf16_length(&text_box.text().unwrap()));

    raise_text_event(&text_box, " edit");
    raise_key_event(&text_box, Key::Space, KeyModifiers::NONE);
    raise_text_event(&text_box, " more");

    assert_eq!(Some("initial edit more"), source.bar().as_deref());
    assert!(text_box.can_undo());

    (source, text_box)
}

#[test]
fn binding_source_change_clears_undo_history() {
    let _scope = test_scope();
    let (source, text_box) = edited_bound_text_box();

    source.set_bar(Some("replacement".to_string()));

    assert_eq!(Some("replacement"), text_box.text().as_deref());
    assert!(!text_box.can_undo());
    assert!(!text_box.can_redo());
}

#[test]
fn two_way_binding_source_echo_does_not_clear_undo_history() {
    let _scope = test_scope();
    let (source, text_box) = edited_bound_text_box();

    text_box.undo();

    assert_eq!(Some("initial edit"), text_box.text().as_deref());
    assert_eq!(Some("initial edit"), source.bar().as_deref());
}

/// Gets the undo and redo stacks of a text box in a state where both were
/// active.
fn exercise_undo_redo(tb: &TextBox) {
    type_three_words(tb);

    assert_eq!(Some("ABCDEF123"), tb.text().as_deref());
    assert!(tb.can_undo());
    tb.undo();
    // Undo will take us back one step.
    assert_eq!(Some("ABCDEF"), tb.text().as_deref());
    assert!(tb.can_redo());
    tb.redo();
    // Redo should restore us.
    assert_eq!(Some("ABCDEF123"), tb.text().as_deref());
}

#[test]
fn setting_undo_limit_clears_undo_redo() {
    let _scope = test_scope();
    let tb = templated_text_box(None);

    tb.measure(infinity());

    exercise_undo_redo(&tb);

    // Change the undo limit, this should clear both stacks setting CanUndo and CanRedo to false.
    tb.set_undo_limit(1);

    assert!(!tb.can_undo());
    assert!(!tb.can_redo());
}

#[test]
fn setting_is_undo_enabled_to_false_clears_undo_redo() {
    let _scope = test_scope();
    let tb = templated_text_box(None);

    tb.measure(infinity());

    exercise_undo_redo(&tb);

    // Disable undo/redo, this should clear both stacks setting CanUndo and CanRedo to false.
    tb.set_is_undo_enabled(false);

    assert!(!tb.can_undo());
    assert!(!tb.can_redo());
}

#[test]
fn empty_text_box_initializes_clipboard_command_states() {
    let _scope = test_scope();
    let tb = TextBox::new();

    assert!(!tb.can_copy());
    assert!(!tb.can_cut());
    assert!(tb.can_paste());
}

/// A text box with the text "abcd" and "bc" selected, in a top-level whose
/// platform implementation exposes `clipboard`.
fn create_text_box_in_top_level(clipboard: &Rc<TestClipboardImpl>) -> (Ref<TextBox>, Ref<Window>) {
    let text_box = templated_text_box(Some("abcd"));
    text_box.set_selection_start(1);
    text_box.set_selection_end(3);

    let top_level = window_with_clipboard(&text_box, clipboard);

    text_box.measure(infinity());

    (text_box, top_level)
}

/// Runs `action` and the dispatcher jobs it leaves behind, and returns the
/// panic that came out of the dispatcher, if any.
pub(crate) fn run_and_capture_unhandled_panic(action: impl FnOnce()) -> Option<Box<dyn std::any::Any + Send>> {
    action();

    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| Dispatcher::ui_thread().run_jobs(None))).err()
}

/// The message of a panic.
pub(crate) fn panic_message(payload: &(dyn std::any::Any + Send)) -> Option<&str> {
    payload.downcast_ref::<&'static str>().copied().or_else(|| payload.downcast_ref::<String>().map(String::as_str))
}

/// The failures the reference expects from a platform clipboard
/// (`ExpectedClipboardExceptions`): a timeout, a canceled operation, a
/// denied access and a failed call of the platform clipboard interface.
pub(crate) const EXPECTED_CLIPBOARD_EXCEPTIONS: [ClipboardErrorKind; 4] = [
    ClipboardErrorKind::Timeout,
    ClipboardErrorKind::Canceled,
    ClipboardErrorKind::AccessDenied,
    ClipboardErrorKind::Platform,
];

/// Records the message templates of the events logged on this thread until
/// the returned sink is disposed.
pub(crate) fn record_log_messages() -> (Rc<RefCell<Vec<String>>>, Rc<dyn IDisposable>) {
    let messages: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));
    let recorded = messages.clone();
    let sink = TestLogSink::start(move |_, _, _, message_template, _| {
        recorded.borrow_mut().push(message_template.to_owned());
    });
    (messages, sink)
}

#[test]
fn cut_does_not_delete_selection_when_clipboard_fails() {
    for exception_type in EXPECTED_CLIPBOARD_EXCEPTIONS {
        let _scope = test_scope();

        let clipboard_impl = TestClipboardImpl::throwing(exception_type);
        let (target, _top_level) = create_text_box_in_top_level(&clipboard_impl);
        let (messages, sink) = record_log_messages();

        let unhandled = run_and_capture_unhandled_panic(|| target.cut());
        sink.dispose();

        assert!(unhandled.is_none(), "{exception_type:?}");
        assert_eq!(1, clipboard_impl.set_data_count());
        assert_eq!(Some("abcd"), target.text().as_deref());
        assert_eq!(1, target.selection_start());
        assert_eq!(3, target.selection_end());
        assert_eq!(vec!["Failed to write text to clipboard: {Error}"], *messages.borrow(), "{exception_type:?}");
    }
}

#[test]
fn cut_deletes_selection_when_clipboard_succeeds() {
    let _scope = test_scope();
    let clipboard = TestClipboardImpl::new();

    let (target, _top_level) = create_text_box_in_top_level(&clipboard);

    let unhandled = run_and_capture_unhandled_panic(|| target.cut());

    assert!(unhandled.is_none());
    assert_eq!(Some("ad"), target.text().as_deref());
    assert_eq!(Some("bc".to_owned()), clipboard.text());
}

#[test]
fn copy_does_not_throw_when_clipboard_fails() {
    for exception_type in EXPECTED_CLIPBOARD_EXCEPTIONS {
        let _scope = test_scope();

        let clipboard_impl = TestClipboardImpl::throwing(exception_type);
        let (target, _top_level) = create_text_box_in_top_level(&clipboard_impl);
        let (messages, sink) = record_log_messages();

        let unhandled = run_and_capture_unhandled_panic(|| target.copy());
        sink.dispose();

        assert!(unhandled.is_none(), "{exception_type:?}");
        assert_eq!(1, clipboard_impl.set_data_count());
        assert_eq!(Some("abcd"), target.text().as_deref());
        assert_eq!(vec!["Failed to write text to clipboard: {Error}"], *messages.borrow(), "{exception_type:?}");
    }
}

#[test]
fn paste_does_not_change_text_when_clipboard_fails() {
    for exception_type in EXPECTED_CLIPBOARD_EXCEPTIONS {
        let _scope = test_scope();

        let clipboard_impl = TestClipboardImpl::throwing(exception_type);
        let (target, _top_level) = create_text_box_in_top_level(&clipboard_impl);
        let (messages, sink) = record_log_messages();

        let unhandled = run_and_capture_unhandled_panic(|| target.paste());
        sink.dispose();

        assert!(unhandled.is_none(), "{exception_type:?}");
        assert_eq!(1, clipboard_impl.try_get_data_count());
        assert_eq!(Some("abcd"), target.text().as_deref());
        assert!(!target.can_undo());
        assert_eq!(vec!["Failed to read text from clipboard: {Error}"], *messages.borrow(), "{exception_type:?}");
    }
}

// The reference fails the clipboard with an `InvalidOperationException` and
// expects it out of the dispatcher. The failure that is not one a platform
// clipboard is known for is the error of the kind `Other`: it leaves the
// operation and is raised from a dispatcher job.
#[test]
fn clipboard_operations_do_not_swallow_unexpected_exceptions() {
    let _scope = test_scope();

    let clipboard_impl = TestClipboardImpl::throwing(ClipboardErrorKind::Other);
    let (target, _top_level) = create_text_box_in_top_level(&clipboard_impl);
    let (messages, sink) = record_log_messages();
    let expected = ClipboardHelper::unhandled_message(&ClipboardError::from_kind(ClipboardErrorKind::Other));

    for operation in [TextBox::cut, TextBox::copy, TextBox::paste] {
        // The failure does not come out of the operation itself.
        operation(&target);

        let unhandled = run_and_capture_unhandled_panic(|| ()).expect("the failure of the clipboard");

        assert_eq!(Some(expected.as_str()), panic_message(&*unhandled));
    }
    sink.dispose();

    assert_eq!(2, clipboard_impl.set_data_count());
    assert_eq!(1, clipboard_impl.try_get_data_count());
    assert_eq!(Some("abcd"), target.text().as_deref());
    assert_eq!(1, target.selection_start());
    assert_eq!(3, target.selection_end());
    assert!(messages.borrow().is_empty());
}

// Not in the reference: a clipboard that panics (one that was disposed, for
// instance) is not swallowed either.
#[test]
fn clipboard_operations_do_not_swallow_unexpected_panics() {
    let _scope = test_scope();

    let clipboard = TestClipboardImpl::failing(TestClipboardFailure::Panics);
    let (target, _top_level) = create_text_box_in_top_level(&clipboard);

    for operation in [TextBox::cut, TextBox::copy, TextBox::paste] {
        let unhandled = run_and_capture_unhandled_panic(|| operation(&target)).expect("the panic of the clipboard");

        assert_eq!(Some("The clipboard failed."), panic_message(&*unhandled));
    }

    assert_eq!(2, clipboard.set_data_count());
    assert_eq!(1, clipboard.try_get_data_count());
    assert_eq!(Some("abcd"), target.text().as_deref());
}

// Not in the reference: a clipboard whose operations never complete leaves
// everything as it is and logs nothing.
#[test]
fn clipboard_operations_wait_for_a_clipboard_that_never_completes() {
    let _scope = test_scope();

    let clipboard = TestClipboardImpl::failing(TestClipboardFailure::NeverCompletes);
    let (target, _top_level) = create_text_box_in_top_level(&clipboard);
    let (messages, sink) = record_log_messages();

    for operation in [TextBox::cut, TextBox::copy, TextBox::paste] {
        assert!(run_and_capture_unhandled_panic(|| operation(&target)).is_none());
    }
    sink.dispose();

    assert_eq!(2, clipboard.set_data_count());
    assert_eq!(1, clipboard.try_get_data_count());
    assert_eq!(Some("abcd"), target.text().as_deref());
    assert_eq!(1, target.selection_start());
    assert_eq!(3, target.selection_end());
    assert!(messages.borrow().is_empty());
}

#[test]
fn command_states_update_when_read_only_and_password_char_change() {
    let _scope = test_scope();
    let tb = templated_text_box(Some("1234"));
    tb.set_selection_start(1);
    tb.set_selection_end(3);

    tb.measure(infinity());

    assert!(tb.can_copy());
    assert!(tb.can_cut());
    assert!(tb.can_paste());

    tb.set_is_read_only(true);

    assert!(tb.can_copy());
    assert!(!tb.can_cut());
    assert!(!tb.can_paste());

    tb.set_password_char('*');

    assert!(!tb.can_copy());
    assert!(!tb.can_cut());
    assert!(!tb.can_paste());

    tb.set_is_read_only(false);

    assert!(!tb.can_copy());
    assert!(!tb.can_cut());
    assert!(tb.can_paste());

    tb.set_password_char('\0');

    assert!(tb.can_copy());
    assert!(tb.can_cut());
    assert!(tb.can_paste());
}

#[test]
fn command_states_update_when_reveal_password_changes() {
    let _scope = test_scope();
    let tb = templated_text_box(Some("1234"));
    tb.set_password_char('*');
    tb.set_selection_start(1);
    tb.set_selection_end(3);

    tb.measure(infinity());

    assert!(!tb.can_copy());
    assert!(!tb.can_cut());
    assert!(tb.can_paste());

    tb.set_reveal_password(true);

    assert!(tb.can_copy());
    assert!(tb.can_cut());
    assert!(tb.can_paste());

    tb.set_reveal_password(false);

    assert!(!tb.can_copy());
    assert!(!tb.can_cut());
    assert!(tb.can_paste());
}

fn assert_read_only_hotkey_leaves_state_untouched(
    text_box: &TextBox,
    key: Key,
    input_modifiers: KeyModifiers,
    handled: bool,
) {
    let original_text = text_box.text();
    let original_caret_index = text_box.caret_index();
    let original_selection_start = text_box.selection_start();
    let original_selection_end = text_box.selection_end();
    let original_can_undo = text_box.can_undo();
    let original_can_redo = text_box.can_redo();

    let args = raise_key_event(text_box, key, input_modifiers);

    assert_eq!(handled, args.handled(), "{key:?} {input_modifiers:?}");
    assert_eq!(original_text, text_box.text(), "{key:?} {input_modifiers:?}");
    assert_eq!(original_caret_index, text_box.caret_index(), "{key:?} {input_modifiers:?}");
    assert_eq!(original_selection_start, text_box.selection_start(), "{key:?} {input_modifiers:?}");
    assert_eq!(original_selection_end, text_box.selection_end(), "{key:?} {input_modifiers:?}");
    assert_eq!(original_can_undo, text_box.can_undo(), "{key:?} {input_modifiers:?}");
    assert_eq!(original_can_redo, text_box.can_redo(), "{key:?} {input_modifiers:?}");
}

fn assert_read_only_gesture_leaves_state_untouched(text_box: &TextBox, gestures: &[KeyGesture], handled: bool) {
    assert!(!gestures.is_empty());
    let gesture = &gestures[0];
    assert_read_only_hotkey_leaves_state_untouched(text_box, gesture.key(), gesture.key_modifiers(), handled);
}

#[test]
fn read_only_editing_hotkeys_do_not_modify_text_or_undo_state() {
    let _scope = test_scope();
    let tb = templated_text_box(None);
    tb.set_accepts_return(true);
    tb.set_accepts_tab(true);

    tb.measure(infinity());

    raise_text_event(&tb, "ABC");
    raise_key_event(&tb, Key::Space, KeyModifiers::NONE);
    raise_text_event(&tb, "DEF");

    assert_eq!(Some("ABCDEF"), tb.text().as_deref());

    tb.undo();

    assert_eq!(Some("ABC"), tb.text().as_deref());
    assert!(tb.can_undo());
    assert!(tb.can_redo());

    tb.set_is_read_only(true);
    let text_length = utf16_length(&tb.text().unwrap());
    tb.set_caret_index(text_length);
    tb.set_selection_start(0);
    tb.set_selection_end(text_length);

    let original_text = tb.text();
    let original_caret_index = tb.caret_index();
    let original_selection_start = tb.selection_start();
    let original_selection_end = tb.selection_end();
    let original_can_undo = tb.can_undo();
    let original_can_redo = tb.can_redo();

    let cut_raised = Rc::new(Cell::new(0));
    let paste_raised = Rc::new(Cell::new(0));
    let count = cut_raised.clone();
    tb.cutting_to_clipboard(move |_, _| count.set(count.get() + 1));
    let count = paste_raised.clone();
    tb.pasting_from_clipboard(move |_, _| count.set(count.get() + 1));

    let hotkeys = crate::Application::current()
        .and_then(|application| application.platform_settings())
        .expect("the platform settings of the application")
        .hotkey_configuration();

    assert_read_only_gesture_leaves_state_untouched(&tb, &hotkeys.cut, true);
    assert_read_only_gesture_leaves_state_untouched(&tb, &hotkeys.paste, true);
    assert_read_only_gesture_leaves_state_untouched(&tb, &hotkeys.undo, true);
    assert_read_only_gesture_leaves_state_untouched(&tb, &hotkeys.redo, true);
    assert_read_only_hotkey_leaves_state_untouched(&tb, Key::Back, KeyModifiers::NONE, true);
    assert_read_only_hotkey_leaves_state_untouched(&tb, Key::Back, KeyModifiers::CONTROL, true);
    assert_read_only_hotkey_leaves_state_untouched(&tb, Key::Delete, KeyModifiers::NONE, true);
    assert_read_only_hotkey_leaves_state_untouched(&tb, Key::Delete, KeyModifiers::CONTROL, true);
    assert_read_only_hotkey_leaves_state_untouched(&tb, Key::Enter, KeyModifiers::NONE, true);
    assert_read_only_hotkey_leaves_state_untouched(&tb, Key::Tab, KeyModifiers::NONE, true);
    assert_read_only_hotkey_leaves_state_untouched(&tb, Key::Space, KeyModifiers::NONE, false);

    assert_eq!(original_text, tb.text());
    assert_eq!(original_caret_index, tb.caret_index());
    assert_eq!(original_selection_start, tb.selection_start());
    assert_eq!(original_selection_end, tb.selection_end());
    assert_eq!(original_can_undo, tb.can_undo());
    assert_eq!(original_can_redo, tb.can_redo());
    assert_eq!(0, cut_raised.get());
    assert_eq!(0, paste_raised.get());
}

#[test]
fn undo_limit_count_is_respected() {
    let _scope = test_scope();
    let tb = TextBox::new();
    tb.set_template(create_template());
    tb.set_undo_limit(3); // Something small for this test.

    tb.measure(infinity());

    // Push 3 undoable actions, we should only be able to recover 2.
    type_three_words(&tb);

    assert_eq!(Some("ABCDEF123"), tb.text().as_deref());

    // Undo will take us back one step.
    tb.undo();
    assert_eq!(Some("ABCDEF"), tb.text().as_deref());

    // Undo again.
    tb.undo();
    assert_eq!(Some("ABC"), tb.text().as_deref());

    // We now should not be able to undo again.
    assert!(!tb.can_undo());
}

#[test]
fn should_move_caret_to_end_of_line() {
    let _scope = test_scope();
    let tb = templated_text_box(Some("AB\nAB"));

    tb.measure(infinity());

    raise_key_event(&tb, Key::End, KeyModifiers::SHIFT);

    assert_eq!(2, tb.caret_index());
}

const LEFT_TO_RIGHT_SELECTIONS: [(i32, i32); 5] = [(2, 4), (0, 4), (2, 6), (0, 6), (3, 4)];
const RIGHT_TO_LEFT_SELECTIONS: [(i32, i32); 5] = [(4, 2), (4, 0), (6, 2), (6, 0), (4, 3)];

/// Selects a range of "ABCDEF", presses a key and checks that the selection
/// collapsed to the expected position.
fn pressing_key_removes_selection(selections: [(i32, i32); 5], key: Key, expected: impl Fn(i32, i32) -> i32) {
    for (selection_start, selection_end) in selections {
        let _scope = test_scope();
        let tb = templated_text_box(Some("ABCDEF"));

        tb.measure(infinity());
        tb.set_caret_index(selection_start);
        tb.set_selection_start(selection_start);
        tb.set_selection_end(selection_end);

        raise_key_event(&tb, key, KeyModifiers::NONE);

        let expected = expected(selection_start, selection_end);

        assert_eq!(expected, tb.selection_start(), "{selection_start} {selection_end}");
        assert_eq!(expected, tb.selection_end(), "{selection_start} {selection_end}");
        assert_eq!(expected, tb.caret_index(), "{selection_start} {selection_end}");
    }
}

#[test]
fn when_selection_from_left_to_right_pressing_right_should_remove_selection_moving_caret_to_end_of_previous_selection()
{
    pressing_key_removes_selection(LEFT_TO_RIGHT_SELECTIONS, Key::Right, |_, selection_end| selection_end);
}

#[test]
fn when_selection_from_left_to_right_pressing_left_should_remove_selection_moving_caret_to_start_of_previous_selection()
{
    pressing_key_removes_selection(LEFT_TO_RIGHT_SELECTIONS, Key::Left, |selection_start, _| selection_start);
}

#[test]
fn when_selection_from_right_to_left_pressing_right_should_remove_selection_moving_caret_to_start_of_previous_selection()
{
    pressing_key_removes_selection(RIGHT_TO_LEFT_SELECTIONS, Key::Right, |selection_start, _| selection_start);
}

#[test]
fn when_selection_from_right_to_left_pressing_left_should_remove_selection_moving_caret_to_end_of_previous_selection()
{
    pressing_key_removes_selection(RIGHT_TO_LEFT_SELECTIONS, Key::Left, |_, selection_end| selection_end);
}

/// Selects all of "ABCDEF" from a caret position, presses a key and checks
/// that the selection collapsed to the expected position.
fn select_all_then_key_removes_selection(key: Key, expected: i32) {
    for caret_index in [0, 2, 4, 6] {
        let _scope = test_scope();
        let tb = templated_text_box(Some("ABCDEF"));

        tb.measure(infinity());
        tb.set_caret_index(caret_index);

        raise_key_event(&tb, Key::A, KeyModifiers::CONTROL);
        raise_key_event(&tb, key, KeyModifiers::NONE);

        assert_eq!(expected, tb.selection_start(), "{caret_index}");
        assert_eq!(expected, tb.selection_end(), "{caret_index}");
        assert_eq!(expected, tb.caret_index(), "{caret_index}");
    }
}

#[test]
fn when_select_all_from_position_left_should_remove_selection_moving_caret_to_start() {
    select_all_then_key_removes_selection(Key::Left, 0);
}

#[test]
fn when_select_all_from_position_right_should_remove_selection_moving_caret_to_end() {
    select_all_then_key_removes_selection(Key::Right, 6);
}

#[test]
fn when_selection_from_left_to_right_pressing_up_should_remove_selection_moving_caret_to_start_of_previous_selection()
{
    pressing_key_removes_selection(LEFT_TO_RIGHT_SELECTIONS, Key::Up, |selection_start, _| selection_start);
}

#[test]
fn when_selection_from_right_to_left_pressing_up_should_remove_selection_moving_caret_to_end_of_previous_selection() {
    pressing_key_removes_selection(RIGHT_TO_LEFT_SELECTIONS, Key::Up, |_, selection_end| selection_end);
}

#[test]
fn when_select_all_from_position_up_should_remove_selection_moving_caret_to_start() {
    select_all_then_key_removes_selection(Key::Up, 0);
}

#[test]
fn when_selection_from_left_to_right_pressing_down_should_remove_selection_moving_caret_to_end_of_previous_selection()
{
    pressing_key_removes_selection(LEFT_TO_RIGHT_SELECTIONS, Key::Down, |_, selection_end| selection_end);
}

#[test]
fn when_selection_from_right_to_left_pressing_down_should_remove_selection_moving_caret_to_start_of_previous_selection()
{
    pressing_key_removes_selection(RIGHT_TO_LEFT_SELECTIONS, Key::Down, |selection_start, _| selection_start);
}

#[test]
fn when_select_all_from_position_down_should_remove_selection_moving_caret_to_end() {
    select_all_then_key_removes_selection(Key::Down, 6);
}

/// The three lines of the multiline selection tests, with both line breaks
/// a checkout of the reference source can give its raw string literal.
fn multiline_texts() -> [String; 2] {
    ["\n", "\r\n"].map(|line_break| ["AAAAAA", "BBBB", "CCCCCCCC"].join(line_break))
}

#[test]
fn when_selecting_multiline_selection_should_be_extended_with_up_arrow_key_till_start_of_text() {
    for text in multiline_texts() {
        for caret_offset_from_end in [0, 4, 8] {
            let _scope = test_scope();
            let tb = templated_text_box(Some(&text));
            tb.set_accepts_return(true);
            tb.apply_template();
            tb.measure(infinity());
            tb.set_caret_index(utf16_length(&text) - caret_offset_from_end);

            raise_key_event(&tb, Key::Up, KeyModifiers::SHIFT);
            raise_key_event(&tb, Key::Up, KeyModifiers::SHIFT);
            raise_key_event(&tb, Key::Up, KeyModifiers::SHIFT);
            raise_key_event(&tb, Key::Up, KeyModifiers::SHIFT);

            assert_eq!(0, tb.selection_end(), "{text:?} {caret_offset_from_end}");
        }
    }
}

#[test]
fn when_selecting_multiline_selection_should_be_extended_with_down_arrow_key_till_end_of_text() {
    for text in multiline_texts() {
        for caret_offset_from_start in [0, 3, 6] {
            let _scope = test_scope();
            let tb = templated_text_box(Some(&text));
            tb.set_accepts_return(true);
            tb.apply_template();
            tb.measure(infinity());
            tb.set_caret_index(caret_offset_from_start);

            raise_key_event(&tb, Key::Down, KeyModifiers::SHIFT);
            raise_key_event(&tb, Key::Down, KeyModifiers::SHIFT);
            raise_key_event(&tb, Key::Down, KeyModifiers::SHIFT);
            raise_key_event(&tb, Key::Down, KeyModifiers::SHIFT);

            assert_eq!(utf16_length(&text), tb.selection_end(), "{text:?} {caret_offset_from_start}");
        }
    }
}

// Deferred: `TextBox_In_AdornerLayer_Will_Not_Cause_Collection_Modified_In_VisualLayerManager_Measure` and
// `..._Arrange` (wait for `VisualLayerManager` and `AdornerLayer`).

#[test]
fn should_scroll_caret_to_line() {
    let latin = "A\nBB\nCCC\nDDDD";
    let arabic = "واحد\nاثنين\nثلاثة\nأربعة";
    let cases = [
        (latin, 0, 0),
        (latin, 1, 2),
        (latin, 2, 5),
        (latin, 3, 9),
        (arabic, 0, 0),
        (arabic, 1, 5),
        (arabic, 2, 11),
        (arabic, 3, 17),
    ];

    for (text, target_line_index, expected_caret_index) in cases {
        let _scope = test_scope();
        let tb = templated_text_box(Some(text));
        tb.apply_template();
        tb.scroll_to_line(target_line_index);
        assert_eq!(expected_caret_index, tb.caret_index(), "{text:?} {target_line_index}");
    }
}

#[test]
fn should_throw_argument_out_of_range() {
    let _scope = test_scope();
    let tb = templated_text_box(Some(""));
    tb.apply_template();

    for line_index in [-1, 1] {
        let tb = tb.clone();
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || tb.scroll_to_line(line_index)));
        assert!(result.is_err(), "{line_index}");
    }
}

#[test]
fn input_method_client_surrounding_text_returns_empty_for_empty_line() {
    let _scope = test_scope();

    let text_box = templated_text_box(Some(""));
    text_box.set_caret_index(0);
    text_box.apply_template();

    let client = get_input_method_client(&text_box);

    assert_eq!("", client.surrounding_text());
}

#[test]
fn input_method_client_surrounding_text_uses_full_document_for_multiline_text() {
    let _scope = test_scope();

    let text_box = templated_text_box(Some("one\ntwo"));
    text_box.set_caret_index(5);
    text_box.apply_template();

    let client = get_input_method_client(&text_box);

    assert_eq!("one\ntwo", client.surrounding_text());
    assert_eq!(TextSelection::new(5, 5), client.selection());
}

#[test]
fn preedit_survives_reentrant_layout_invalidation_from_caret_bounds_changed() {
    let _scope = test_scope();

    let text_box = templated_text_box(Some("hello"));
    text_box.set_caret_index(5);

    let _root = host_in_root(&text_box);

    let client = get_input_method_client(&text_box);
    let presenter = text_box.find_descendant_of_type::<TextPresenter>(false).expect("the presenter");

    // A subscriber of the caret bounds may synchronously invalidate the text layout
    // (the candidate window of an input method or a caret-following overlay forcing a layout pass
    // does exactly that). The preedit update must still complete against the layout
    // it computed with instead of dereferencing the reentrantly cleared field.
    let weak = presenter.downgrade();
    let _subscription = presenter.caret_bounds_changed(move || {
        if let Some(presenter) = weak.upgrade() {
            presenter.hide_caret();
        }
    });

    client.set_preedit_text_with_cursor(Some("かん"), Some(2));

    assert_eq!(Some("かん"), presenter.preedit_text().as_deref());
}

#[test]
fn input_method_client_selection_setter_uses_document_offsets_for_multiline_text() {
    let _scope = test_scope();

    let text_box = templated_text_box(Some("one\ntwo"));
    text_box.set_caret_index(5);
    text_box.apply_template();

    let client = get_input_method_client(&text_box);
    client.set_selection(TextSelection::new(0, 3));

    assert_eq!(0, text_box.selection_start());
    assert_eq!(3, text_box.selection_end());
    assert_eq!("one", text_box.selected_text());
}

#[test]
fn backspace_should_delete_last_character_in_line_and_keep_caret_on_same_line() {
    let _scope = test_scope();

    let text_box = templated_text_box(Some("a\nb"));
    text_box.set_caret_index(3);
    text_box.apply_template();

    let _root = host_in_root(&text_box);

    let text_presenter = text_box.find_descendant_of_type::<TextPresenter>(false).expect("the presenter");

    let old_caret_y = text_presenter.get_cursor_rectangle().y;
    assert_ne!(0.0, old_caret_y);

    raise_key_event(&text_box, Key::Back, KeyModifiers::NONE);

    assert_eq!(Some("a\n"), text_box.text().as_deref());
    assert_eq!(2, text_box.caret_index());
    assert_eq!(2, text_presenter.caret_index());

    let caret_y = text_presenter.get_cursor_rectangle().y;
    assert_eq!(old_caret_y, caret_y);
}

#[test]
fn losing_focus_should_not_reset_selection() {
    let _scope = test_scope();
    let _focus = focus_scope();
    let target1 = templated_text_box(Some("1234"));
    target1.set_clear_selection_on_lost_focus(false);

    target1.apply_template();

    let target2 = templated_text_box(None);

    target2.apply_template();

    let sp = StackPanel::new();
    sp.children().add(target1.clone());
    sp.children().add(target2.clone());

    let _root = TestRoot::with_child(sp);

    target1.set_selection_start(0);
    target1.set_selection_end(4);

    target1.focus();

    assert!(target1.is_focused());

    assert_eq!("1234", target1.selected_text());

    target2.focus();

    assert_eq!("1234", target1.selected_text());
}

#[test]
fn backspace_should_delete_crlf_newline_character_at_once() {
    let _scope = test_scope();
    let target = templated_text_box(Some("First\r\nSecond"));
    target.set_caret_index(7);
    target.apply_template();

    // (First\r\nSecond)
    raise_key_event(&target, Key::Back, KeyModifiers::NONE);
    // (FirstSecond)

    assert_eq!(Some("FirstSecond"), target.text().as_deref());
}

#[test]
fn placeholder_foreground_can_be_set() {
    let _scope = test_scope();
    let red: Rc<dyn IBrush> = Brushes::red();
    let target = templated_text_box(None);
    target.set_placeholder_text(Some("Enter text"));
    target.set_placeholder_foreground(Some(red.clone()));

    target.apply_template();

    assert!(target.placeholder_foreground() == Some(red));
}

#[test]
fn placeholder_foreground_defaults_to_null() {
    let _scope = test_scope();
    let target = templated_text_box(None);
    target.set_placeholder_text(Some("Enter text"));

    target.apply_template();

    assert!(target.placeholder_foreground().is_none());
}

#[test]
fn placeholder_foreground_can_be_set_to_null() {
    let _scope = test_scope();
    let blue: Rc<dyn IBrush> = Brushes::blue();
    let target = templated_text_box(None);
    target.set_placeholder_text(Some("Enter text"));
    target.set_placeholder_foreground(Some(blue));

    target.apply_template();

    target.set_placeholder_foreground(None);

    assert!(target.placeholder_foreground().is_none());
}

// The pointer tests of the reference show a themed text box in a window and
// address it in window coordinates. Here the text box has the template of
// the tests and fills the window; with the metrics of the test fonts "0123"
// is 24 wide, so the press at x = 1 hits the leading edge of the first
// character (the reference presses at x = 5, inside the padding of the
// theme) and x = 700 is past the end of the text.

/// A text box with the text "0123".
fn text_box_to_show() -> Ref<TextBox> {
    let target = TextBox::new();
    target.set_template(create_themed_template());
    target.set_text(Some("0123"));
    target
}

#[test]
fn pointer_selection_is_published_to_primary_selection() {
    let _scope = test_scope();
    let primary_selection = TestClipboardImpl::new();
    let target = text_box_to_show();
    let window = window_with_primary_selection(&target, &primary_selection);

    let mouse = MouseTestHelper::new();
    mouse.down_at(&target, MouseButton::Left, Point::new(1.0, 300.0), 1);
    mouse.move_(&target, Point::new(700.0, 300.0));
    mouse.up_at(&target, MouseButton::Left, Point::new(700.0, 300.0));

    assert_eq!("0123", target.selected_text());
    assert_eq!(Some("0123".to_owned()), get_clipboard_text(&primary_selection_of(&window)));
}

#[test]
fn pointer_selection_is_not_published_to_primary_selection_for_password_box() {
    let _scope = test_scope();
    let primary_selection = TestClipboardImpl::new();
    let target = text_box_to_show();
    target.set_password_char('*');
    let window = window_with_primary_selection(&target, &primary_selection);

    let mouse = MouseTestHelper::new();
    mouse.down_at(&target, MouseButton::Left, Point::new(1.0, 300.0), 1);
    mouse.move_(&target, Point::new(700.0, 300.0));
    mouse.up_at(&target, MouseButton::Left, Point::new(700.0, 300.0));

    assert_eq!(0, target.selection_start().min(target.selection_end()));
    assert_eq!(4, target.selection_start().max(target.selection_end()));
    assert_eq!(None, get_clipboard_text(&primary_selection_of(&window)));
    assert_eq!(0, primary_selection.set_data_count());
}

// Not in the reference, which logs whatever the primary selection throws
// when a selection is published: the failure stays with the publication,
// whatever its kind, instead of reaching the dispatcher.
#[test]
fn failing_primary_selection_does_not_break_pointer_selection() {
    let failures = EXPECTED_CLIPBOARD_EXCEPTIONS
        .into_iter()
        .chain([ClipboardErrorKind::Other])
        .map(TestClipboardFailure::Fails)
        .chain([TestClipboardFailure::Panics]);

    for failure in failures {
        let _scope = test_scope();
        let primary_selection = TestClipboardImpl::failing(failure);
        let target = text_box_to_show();
        let _window = window_with_primary_selection(&target, &primary_selection);
        let (messages, sink) = record_log_messages();

        let mouse = MouseTestHelper::new();
        let unhandled = run_and_capture_unhandled_panic(|| {
            mouse.down_at(&target, MouseButton::Left, Point::new(1.0, 300.0), 1);
            mouse.move_(&target, Point::new(700.0, 300.0));
            mouse.up_at(&target, MouseButton::Left, Point::new(700.0, 300.0));
        });
        sink.dispose();

        assert!(unhandled.is_none(), "{failure:?}");
        assert_eq!("0123", target.selected_text());
        assert_eq!(1, primary_selection.set_data_count());
        assert_eq!(vec!["Failed to write text to primary selection: {Error}"], *messages.borrow(), "{failure:?}");
    }
}

// Not in the reference: nobody observes the paste of a middle click, so a
// failure of the primary selection never reaches the dispatcher. The ones a
// platform clipboard is known for are logged.
#[test]
fn middle_click_does_not_paste_when_primary_selection_fails() {
    for kind in EXPECTED_CLIPBOARD_EXCEPTIONS.into_iter().chain([ClipboardErrorKind::Other]) {
        let _scope = test_scope();
        let primary_selection = TestClipboardImpl::throwing(kind);
        let target = text_box_to_show();
        let _window = window_with_primary_selection(&target, &primary_selection);
        let (messages, sink) = record_log_messages();

        let mouse = MouseTestHelper::new();
        let unhandled = run_and_capture_unhandled_panic(|| {
            mouse.down_at(&target, MouseButton::Middle, Point::new(700.0, 300.0), 1);
            mouse.up_at(&target, MouseButton::Middle, Point::new(700.0, 300.0));
        });
        sink.dispose();

        assert!(unhandled.is_none(), "{kind:?}");
        assert_eq!(Some("0123"), target.text().as_deref());
        assert_eq!(1, primary_selection.try_get_data_count());
        let expected: Vec<&str> = if kind == ClipboardErrorKind::Other {
            Vec::new()
        } else {
            vec!["Failed to read text from clipboard: {Error}"]
        };
        assert_eq!(expected, *messages.borrow(), "{kind:?}");
    }
}

#[test]
fn middle_click_pastes_primary_selection_at_click_position() {
    let _scope = test_scope();
    let primary_selection = TestClipboardImpl::new();
    let target = text_box_to_show();
    let window = window_with_primary_selection(&target, &primary_selection);

    set_clipboard_text(&primary_selection_of(&window), "abc");

    let pasting_clipboard: Rc<RefCell<Option<Option<Rc<dyn IClipboard>>>>> = Rc::new(RefCell::new(None));
    let pasting = pasting_clipboard.clone();
    target.pasting_from_clipboard(move |_, e| {
        let e = e.downcast_ref::<PastingFromClipboardEventArgs>().expect("the args of a paste");
        *pasting.borrow_mut() = Some(e.clipboard());
    });

    let mouse = MouseTestHelper::new();
    mouse.down_at(&target, MouseButton::Middle, Point::new(700.0, 300.0), 1);
    mouse.up_at(&target, MouseButton::Middle, Point::new(700.0, 300.0));

    assert_eq!(Some("0123abc"), target.text().as_deref());
    let pasting_clipboard = pasting_clipboard.borrow().clone().expect("the paste event was raised");
    let pasting_clipboard = pasting_clipboard.expect("the clipboard of the paste");
    assert!(std::ptr::addr_eq(Rc::as_ptr(&primary_selection_of(&window)), Rc::as_ptr(&pasting_clipboard)));

    // The pasted-over selection was not changed by the gesture, so it must not be published.
    assert_eq!(Some("abc".to_owned()), get_clipboard_text(&primary_selection_of(&window)));
    assert_eq!(1, primary_selection.set_data_count());
}

#[test]
fn middle_click_does_not_paste_when_read_only() {
    let _scope = test_scope();
    let primary_selection = TestClipboardImpl::new();
    let target = text_box_to_show();
    target.set_is_read_only(true);
    let window = window_with_primary_selection(&target, &primary_selection);

    set_clipboard_text(&primary_selection_of(&window), "abc");

    let mouse = MouseTestHelper::new();
    mouse.down_at(&target, MouseButton::Middle, Point::new(700.0, 300.0), 1);
    mouse.up_at(&target, MouseButton::Middle, Point::new(700.0, 300.0));

    assert_eq!(Some("0123"), target.text().as_deref());
}

#[test]
fn middle_click_does_nothing_without_primary_selection() {
    let _scope = test_scope();
    let target = text_box_to_show();
    let window = show_in_window(&target, MockWindowingPlatform::create_window_mock());

    assert!(window.try_get_clipboard(ClipboardType::PrimarySelection).is_none());

    let mouse = MouseTestHelper::new();
    mouse.down_at(&target, MouseButton::Middle, Point::new(700.0, 300.0), 1);
    mouse.up_at(&target, MouseButton::Middle, Point::new(700.0, 300.0));

    assert_eq!(Some("0123"), target.text().as_deref());
}

#[test]
fn paste_raises_event_when_no_clipboard_is_available() {
    let _scope = test_scope();
    let target = text_box_to_show();
    let window = show_in_window(&target, MockWindowingPlatform::create_window_mock());

    assert!(window.clipboard().is_none());

    let pasting_clipboard: Rc<RefCell<Option<Option<Rc<dyn IClipboard>>>>> = Rc::new(RefCell::new(None));
    let pasting = pasting_clipboard.clone();
    target.pasting_from_clipboard(move |_, e| {
        let args = e.downcast_ref::<PastingFromClipboardEventArgs>().expect("the args of a paste");
        *pasting.borrow_mut() = Some(args.clipboard());
        e.set_handled(true);
    });

    target.paste();

    let pasting_clipboard = pasting_clipboard.borrow().clone().expect("the paste event was raised");
    assert!(pasting_clipboard.is_none());
}

// --- tests of other classes that need a text box ------------------------------

// `TextBlockTests.Can_Call_Measure_Without_InvalidateTextLayout` of the reference.
#[test]
fn text_block_can_call_measure_without_invalidate_text_layout() {
    let _scope = test_scope();
    let target = TextBlock::new();

    let text_box = TextBox::new();
    text_box.set_text(Some("Hello"));
    target.inlines().unwrap().add_control(text_box);

    target.measure(infinity());

    target.invalidate_measure();

    target.measure(infinity());
}

// `TextBlockTests.Embedded_Control_Should_Keep_Focus` of the reference.
#[test]
fn text_block_embedded_control_should_keep_focus() {
    let _scope = test_scope();
    let _focus = focus_scope();
    let target = TextBlock::new();

    let root = TestRoot::with_child(target.clone());

    let text_box = templated_text_box(Some("Hello"));

    target.inlines().unwrap().add_control(text_box.clone());

    target.measure(infinity());

    text_box.focus();

    let focus_manager = root.presentation_source().unwrap().input_root().focus_manager().unwrap();
    let text_box_element: Ref<InputElement> = text_box.clone().upcast();

    assert_eq!(Some(text_box_element.clone()), focus_manager.get_focused_element());

    target.invalidate_measure();

    assert_eq!(Some(text_box_element.clone()), focus_manager.get_focused_element());

    target.measure(infinity());

    assert_eq!(Some(text_box_element), focus_manager.get_focused_element());
}

/// A text box that is a command source: its command focuses it
/// (`HotKeyedTextBox` of the hot keyed controls tests of the reference).
#[repr(C)]
struct HotKeyedTextBox {
    base: TextBox,
    hotkey: Cell<Option<KeyGesture>>,
}

ferro_class!(HotKeyedTextBox: TextBox);
ferro_impl_classes!(
    HotKeyedTextBox: VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    TextBoxImpl
);

impl FerroObjectImpl for HotKeyedTextBox {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        register_command_source::<HotKeyedTextBox>(|control| Rc::new(HotKeyedTextBoxHandle(control)));
    }
}

impl StyledElementImpl for HotKeyedTextBox {
    fn on_attached_to_logical_tree(this: &Self, e: &LogicalTreeAttachmentEventArgs) {
        if let Some(hotkey) = this.hotkey.get() {
            this.set_value(Self::hot_key_property(), Some(hotkey));
        }

        Self::parent_on_attached_to_logical_tree(this, e);
    }

    fn on_detached_from_logical_tree(this: &Self, e: &LogicalTreeAttachmentEventArgs) {
        if let Some(hotkey) = this.hot_key() {
            this.hotkey.set(Some(hotkey));
            this.set_value(Self::hot_key_property(), None);
        }

        Self::parent_on_detached_from_logical_tree(this, e);
    }

    fn style_key_override(_this: &Self) -> &'static ferroui_base::TypeInfo {
        TextBox::TYPE
    }
}

struct HotKeyedTextBoxHandle(Ref<HotKeyedTextBox>);

impl ICommandSource for HotKeyedTextBoxHandle {
    fn command(&self) -> Option<Rc<dyn ICommand>> {
        let weak = self.0.downgrade();
        TestCommand::with_can_execute_and_execute(
            |_| true,
            move |_: Option<&BoxedValue>| {
                if let Some(control) = weak.upgrade() {
                    control.focus();
                }
            },
        )
        .as_command()
    }

    fn command_parameter(&self) -> Option<BoxedValue> {
        None
    }

    fn can_execute_changed(&self) {}

    fn is_effectively_enabled(&self) -> bool {
        self.0.is_effectively_enabled()
    }
}

impl HotKeyedTextBox {
    ferro_property!(
        fn hot_key_property() -> StyledProperty<Option<KeyGesture>> {
            HotKeyManager::hot_key_property().add_owner::<HotKeyedTextBox>()
        }
    );

    fn new() -> Ref<Self> {
        instantiate(Self { base: TextBox::construct(), hotkey: Cell::new(None) })
    }

    fn hot_key(&self) -> Option<KeyGesture> {
        self.get_value(Self::hot_key_property())
    }

    fn set_hot_key(&self, value: Option<KeyGesture>) {
        self.set_value(Self::hot_key_property(), value)
    }
}

// `HotKeyedControlsTests.HotKeyedTextBox_Focus_Performed_On_Hotkey` of the reference.
#[test]
fn hot_keyed_text_box_focus_performed_on_hotkey() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let focus = focus_scope();

    let keyboard_device = focus.keyboard.clone();
    let hot_keyed_text_box = HotKeyedTextBox::new();
    hot_keyed_text_box.set_hot_key(Some(KeyGesture::new(Key::F, KeyModifiers::CONTROL)));
    let root = crate::Window::new();
    root.set_content(Some(crate::Control::boxed(hot_keyed_text_box.clone())));
    root.show();

    assert!(!hot_keyed_text_box.is_focused());

    keyboard_device.process_raw_event(&RawKeyEventArgs::new(
        keyboard_device.clone(),
        0,
        root.input_root(),
        RawKeyEventType::KeyDown,
        Key::F,
        RawInputModifiers::CONTROL,
        PhysicalKey::F,
        Some("f".to_string()),
        KeyDeviceType::Keyboard,
    ));

    assert!(hot_keyed_text_box.is_focused());
}
