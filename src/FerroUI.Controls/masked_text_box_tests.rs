//! The tests of the masked text box.
//!
//! Every test runs under a unit test application, and the tests that use a
//! clipboard show the text box in a window whose platform implementation
//! exposes an in-memory clipboard (see the tests of the text box). The
//! reference tests run with the `en-US` culture, whose
//! mask separators (`/`, `:`, `.`, `,`) are those of the default culture of
//! the masked text provider.

use crate::primitives::ScrollBarVisibility;
use crate::test_support::TestRoot;
use crate::test_support_buttons::focus_scope;
use crate::testing::{MockWindowingPlatform, TestClipboardImpl};
use crate::text_box_tests::{
    create_template, panic_message, raise_key_event, raise_text_event, record_log_messages,
    run_and_capture_unhandled_panic, show_in_window, test_scope, window_with_clipboard,
};
use crate::utils::{culture_with_separators, ClipboardHelper, MaskedTextProvider};
use ferroui_base::utilities::CultureInfo;
use crate::{Control, MaskedTextBox, ScrollViewer, StackPanel, TextBox};
use ferroui_base::data::core::Maybe;
use ferroui_base::data::model::{Event, INotifyPropertyChanged, Model};
use ferroui_base::data::{BindingMode, ReflectionBinding};
use ferroui_base::input::platform::{ClipboardError, ClipboardErrorKind};
use ferroui_base::input::{Key, KeyModifiers};
use ferroui_base::media::TextWrapping;
use ferroui_base::reactive::ObservableExt;
use ferroui_base::{ferro_model, FerroObject, FerroObjectExtensions, Ref, Size};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

// --- helpers ------------------------------------------------------------------

fn infinity() -> Size {
    Size::new(f64::INFINITY, f64::INFINITY)
}

/// The line break of the platform.
fn new_line() -> &'static str {
    if cfg!(windows) {
        "\r\n"
    } else {
        "\n"
    }
}

/// A masked text box with the template of the tests.
fn templated() -> Ref<MaskedTextBox> {
    let target = MaskedTextBox::new();
    target.set_template(create_template());
    target
}

/// A masked text box with the template of the tests and a text.
fn templated_with_text(text: &str) -> Ref<MaskedTextBox> {
    let target = templated();
    target.set_text(Some(text));
    target
}

/// A masked text box with the template of the tests, a mask and a text.
fn masked(mask: &str, text: Option<&str>) -> Ref<MaskedTextBox> {
    let target = templated();
    target.set_mask(Some(mask));
    if text.is_some() {
        target.set_text(text);
    }
    target
}

/// Two templated masked text boxes in a stack panel in a root.
fn two_text_boxes(text1: &str, text2: &str) -> (Ref<MaskedTextBox>, Ref<MaskedTextBox>, Ref<TestRoot>) {
    let target1 = templated_with_text(text1);
    let target2 = templated_with_text(text2);
    let sp = StackPanel::new();
    sp.children().add(target1.clone());
    sp.children().add(target2.clone());

    target1.apply_template();
    target2.apply_template();

    let root = TestRoot::with_child(sp);

    (target1, target2, root)
}

/// Hosts a text box the way the top level of the reference tests does: in a
/// root, with the initial layout pass executed.
fn host_in_root(target: &Ref<MaskedTextBox>) -> Ref<TestRoot> {
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
    let target1 = templated_with_text("1234");
    target1.set_context_menu(&test_context_menu());

    let target2 = templated_with_text("5678");

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

    let target1 = templated_with_text("1234");
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
        TextBox::text_property().get_metadata(MaskedTextBox::TYPE).default_binding_mode()
    );
}

#[test]
fn caret_index_can_moved_to_position_after_the_end_of_text_with_arrow_key() {
    let _scope = test_scope();
    let target = templated_with_text("1234");

    target.apply_template();
    target.set_caret_index(3);
    target.measure(infinity());

    raise_key_event(&target, Key::Right, KeyModifiers::NONE);

    assert_eq!(4, target.caret_index());
}

#[test]
fn press_ctrl_a_select_all_text() {
    let _scope = test_scope();
    let target = templated_with_text("1234");

    target.apply_template();

    raise_key_event(&target, Key::A, KeyModifiers::CONTROL);

    assert_eq!(0, target.selection_start());
    assert_eq!(4, target.selection_end());
}

#[test]
fn press_ctrl_a_select_all_null_text() {
    let _scope = test_scope();
    let target = templated();

    raise_key_event(&target, Key::A, KeyModifiers::CONTROL);

    assert_eq!(0, target.selection_start());
    assert_eq!(0, target.selection_end());
}

#[test]
fn press_ctrl_z_will_not_modify_text() {
    let _scope = test_scope();
    let target = templated_with_text("1234");

    raise_key_event(&target, Key::Z, KeyModifiers::CONTROL);

    assert_eq!(Some("1234"), target.text().as_deref());
}

#[test]
fn control_backspace_should_remove_the_word_before_the_caret_if_there_is_no_selection() {
    let _scope = test_scope();
    let text_box = templated_with_text("First Second Third Fourth");
    text_box.set_caret_index(5);

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
    let text_box = templated_with_text("First Second Third Fourth");
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
    let text = text_box.text().unwrap() + " ";
    text_box.set_text(Some(&text));
    text_box.set_caret_index(7);
    raise_key_event(&text_box, Key::Delete, KeyModifiers::CONTROL);
    assert_eq!(Some("Fit Sec"), text_box.text().as_deref());
}

#[test]
fn setting_selection_start_to_selection_end_sets_caret_position_to_selection_start() {
    let _scope = test_scope();
    let text_box = MaskedTextBox::new();
    text_box.set_text(Some("0123456789"));

    text_box.set_selection_start(2);
    text_box.set_selection_end(2);
    assert_eq!(2, text_box.caret_index());
}

#[test]
fn setting_text_updates_caret_position() {
    let _scope = test_scope();
    let target = MaskedTextBox::new();
    target.set_text(Some("Initial Text"));
    target.set_caret_index(11);

    let invoked = Rc::new(Cell::new(false));
    let skipped = Rc::new(Cell::new(false));

    let (weak, invoked_in, skipped_in) = (target.downgrade(), invoked.clone(), skipped.clone());
    let object: &FerroObject = &target;
    let _subscription = FerroObjectExtensions::get_observable(object, TextBox::text_property()).subscribe_fn(move |_| {
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
    let target = templated();
    target.set_accepts_return(false);
    target.set_text(Some("1234"));

    raise_key_event(&target, Key::Enter, KeyModifiers::NONE);

    assert_eq!(Some("1234"), target.text().as_deref());
}

#[test]
fn press_enter_add_default_newline() {
    let _scope = test_scope();
    let target = templated();
    target.set_accepts_return(true);

    target.apply_template();

    raise_key_event(&target, Key::Enter, KeyModifiers::NONE);

    assert_eq!(Some(new_line()), target.text().as_deref());
}

#[test]
fn ascii_only_should_not_accept_non_ascii() {
    let cases = [("00/00/0000", "12102000", "12/10/2000"), ("LLLL", "дбs", "____"), ("AA", "Ü1", "__")];

    for (mask, text_event_arg, expected) in cases {
        let _scope = test_scope();
        let target = templated();
        target.set_mask(Some(mask));
        target.set_ascii_only(true);

        raise_text_event(&target, text_event_arg);

        assert_eq!(Some(expected), target.text().as_deref(), "{mask}");
    }
}

#[test]
fn programmatically_set_text_should_not_be_removed_on_key_press() {
    let _scope = test_scope();
    let target = masked("00:00:00.000", Some("12:34:56.000"));

    target.set_caret_index(target.text().unwrap().encode_utf16().count() as i32);
    raise_key_event(&target, Key::Back, KeyModifiers::NONE);

    assert_eq!(Some("12:34:56.00_"), target.text().as_deref());
}

#[test]
fn invalid_programmatically_set_text_should_be_rejected() {
    let _scope = test_scope();
    let target = masked("00:00:00.000", Some("12:34:560000"));

    assert_eq!(Some("__:__:__.___"), target.text().as_deref());
}

#[test]
fn password_char_should_hide_user_input() {
    let cases = [("00/00/0000", "12102000", "**/**/****"), ("LLLL", "дбs", "***_"), ("AA#00", "S2 33", "**_**")];

    for (mask, text_event_arg, expected) in cases {
        let _scope = test_scope();
        let target = templated();
        target.set_mask(Some(mask));
        target.set_password_char('*');

        raise_text_event(&target, text_event_arg);

        assert_eq!(Some(expected), target.text().as_deref(), "{mask}");
    }
}

#[test]
fn mask_should_work_correctly() {
    let cases = [("00/00/0000", "12102000", "12/10/2000"), ("LLLL", "дбs", "дбs_"), ("AA#00", "S2 33", "S2_33")];

    for (mask, text_event_arg, expected) in cases {
        let _scope = test_scope();
        let target = templated();
        target.set_mask(Some(mask));

        raise_text_event(&target, text_event_arg);

        assert_eq!(Some(expected), target.text().as_deref(), "{mask}");
    }
}

#[test]
fn press_enter_add_custom_newline() {
    let _scope = test_scope();
    let target = templated();
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
        let target = MaskedTextBox::new();
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
    let target = templated_with_text("0123456789");

    target.set_selection_start(0);
    target.set_selection_end(9);

    target.set_text(Some("123"));

    raise_text_event(&target, "456");
}

#[test]
fn selection_start_doesnt_cause_exception() {
    let _scope = test_scope();
    let target = templated_with_text("0123456789");

    target.set_selection_start(8);
    target.set_selection_end(9);

    target.set_text(Some("123"));

    raise_text_event(&target, "456");
}

#[test]
fn selection_start_end_are_valid_ater_text_change() {
    let _scope = test_scope();
    let target = templated_with_text("0123456789");

    target.set_selection_start(8);
    target.set_selection_end(9);

    target.set_text(Some("123"));

    assert!(target.selection_start() <= 3);
    assert!(target.selection_end() <= 3);
}

#[test]
fn selected_text_changes_on_selection_change() {
    let _scope = test_scope();
    let target = templated_with_text("0123456789");

    assert_eq!("", target.selected_text());

    target.set_selection_start(2);
    target.set_selection_end(4);

    assert_eq!("23", target.selected_text());
}

#[test]
fn selected_text_edits_text() {
    let _scope = test_scope();
    let target = templated_with_text("0123");

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
    let target = templated_with_text("0123");
    target.set_selection_start(1);
    target.set_selection_end(3);
    target.set_selected_text(Some(""));

    assert_eq!(Some("03"), target.text().as_deref());
}

#[test]
fn selected_text_null_clears_text() {
    let _scope = test_scope();
    let target = templated_with_text("0123");
    target.set_selection_start(1);
    target.set_selection_end(3);
    target.set_selected_text(None);

    assert_eq!(Some("03"), target.text().as_deref());
}

#[test]
fn coerce_caret_index_doesnt_cause_exception_with_malformed_line_ending() {
    let _scope = test_scope();
    let target = templated_with_text("0123456789\r");
    target.set_caret_index(11);
}

#[test]
fn textbox_doesnt_crash_when_receives_input_and_template_not_applied() {
    for key in [Key::Up, Key::Down, Key::Home, Key::End] {
        let _scope = test_scope();
        let _focus = focus_scope();
        let target1 = templated_with_text("1234");

        let _root = TestRoot::with_child(target1.clone());

        target1.focus();
        assert!(target1.is_focused());

        raise_key_event(&target1, key, KeyModifiers::NONE);
    }
}

#[test]
fn text_box_got_focus_and_lost_focus_work_properly() {
    let _scope = test_scope();
    let _focus = focus_scope();
    let (target1, target2, _root) = two_text_boxes("1234", "5678");

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
    let (target1, target2, _root) = two_text_boxes("1234", "5678");

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
    let (target1, target2, _root) = two_text_boxes("1234", "5678");
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
    let target = MaskedTextBox::new();
    target.set_data_context(Some(source.clone()));

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

    for (initial_text, text_input, max_length, selection_start, selection_end, from_clipboard, expected) in cases {
        let _scope = test_scope();
        let clipboard = TestClipboardImpl::new();
        let target = templated_with_text(initial_text);
        target.set_max_length(max_length);
        target.set_selection_start(selection_start);
        target.set_selection_end(selection_end);

        let _top_level = window_with_clipboard(&target, &clipboard);

        target.apply_template();

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
        let target = templated_with_text("0123");
        target.set_accepts_return(true);
        target.set_accepts_tab(true);

        let _top_level = window_with_clipboard(&target, &clipboard);

        target.set_selection_start(1);
        target.set_selection_end(3);

        raise_key_event(&target, key, modifiers);
        raise_key_event(&target, Key::Z, KeyModifiers::CONTROL); // undo
        assert_eq!(Some("0123"), target.text().as_deref(), "{key:?}");
    }
}

#[test]
fn invalid_text_is_coerced_without_raising_intermediate_change() {
    let _scope = test_scope();
    let target = templated();

    let _root = host_in_root(&target);

    let texts: Rc<RefCell<Vec<Option<String>>>> = Rc::new(RefCell::new(Vec::new()));

    let texts_in = texts.clone();
    let _subscription = target.property_changed(move |e| {
        if e.property() == TextBox::text_property().as_property() {
            texts_in.borrow_mut().push(e.get_new_value::<Option<String>>());
        }
    });

    target.set_mask(Some("000"));

    target.set_text(Some("123"));
    target.set_text(Some("abc"));

    assert_eq!(vec![Some("___".to_owned()), Some("123".to_owned())], *texts.borrow());
}

#[test]
fn focusing_and_unfocusing_does_not_create_undo_operation() {
    let _scope = test_scope();
    let _focus = focus_scope();
    let target = templated();
    target.set_mask(Some("000"));
    target.set_hide_prompt_on_leave(true);
    target.set_text(Some("123"));

    let other = templated();

    let sp = StackPanel::new();
    sp.children().add(target.clone());
    sp.children().add(other.clone());

    target.apply_template();
    other.apply_template();

    let _root = TestRoot::with_child(sp);

    target.focus();
    other.focus();

    assert!(!target.can_undo());
}

#[test]
fn masked_edit_remains_undoable() {
    let _scope = test_scope();
    let _focus = focus_scope();
    let target = masked("000", Some("123"));

    target.apply_template();

    let _root = TestRoot::with_child(target.clone());

    target.focus();
    target.set_caret_index(3);

    raise_key_event(&target, Key::Back, KeyModifiers::NONE);

    assert_eq!(Some("12_"), target.text().as_deref());
    assert!(target.can_undo());

    target.undo();

    assert_eq!(Some("123"), target.text().as_deref());
}

#[test]
fn external_replacement_after_masked_edit_clears_undo_history() {
    let _scope = test_scope();
    let _focus = focus_scope();
    let target = masked("000", Some("123"));

    target.apply_template();

    let _root = TestRoot::with_child(target.clone());

    target.focus();
    target.set_caret_index(3);

    raise_key_event(&target, Key::Back, KeyModifiers::NONE);

    assert!(target.can_undo());

    target.set_text(Some("456"));

    assert!(!target.can_undo());
    assert!(!target.can_redo());
}

#[test]
fn clear_and_selected_text_replacement_remain_undoable() {
    let _scope = test_scope();
    let _focus = focus_scope();
    let target = masked("000", Some("123"));

    target.apply_template();

    let _root = TestRoot::with_child(target.clone());

    target.focus();

    target.clear();
    assert!(target.can_undo());

    target.undo();
    target.set_selection_start(0);
    target.set_selection_end(1);
    target.set_selected_text(Some("9"));

    assert!(target.can_undo());
}

// --- additional tests ---------------------------------------------------------
//
// Members of the masked text box that the reference tests do not reach.

#[test]
fn style_key_is_the_text_box() {
    let _scope = test_scope();
    let target = MaskedTextBox::new();

    assert!(std::ptr::eq(TextBox::TYPE, target.style_key()));
}

#[test]
fn defaults_and_mask_state() {
    let _scope = test_scope();
    let target = MaskedTextBox::new();

    assert!(!target.ascii_only());
    assert_eq!(Some(CultureInfo::current_culture()), target.culture());
    assert!(!target.hide_prompt_on_leave());
    assert_eq!(Some(""), target.mask().as_deref());
    assert_eq!('_', target.prompt_char());
    assert_eq!('\0', target.password_char());
    assert!(target.reset_on_prompt());
    assert!(target.reset_on_space());
    assert!(target.mask_provider().is_none());
    assert_eq!(None, target.mask_completed());
    assert_eq!(None, target.mask_full());

    target.set_mask(Some("009"));
    assert_eq!(Some("___"), target.text().as_deref());
    assert_eq!(Some(false), target.mask_completed());
    assert_eq!(Some(false), target.get_direct_value(MaskedTextBox::mask_full_property()));

    target.set_text(Some("12"));
    assert_eq!(Some("12_"), target.text().as_deref());
    assert_eq!(Some(true), target.get_direct_value(MaskedTextBox::mask_completed_property()));
    assert_eq!(Some(false), target.mask_full());

    target.set_text(Some("123"));
    assert_eq!(Some(true), target.mask_full());

    target.set_text(None);
    assert_eq!(Some("___"), target.text().as_deref());
}

#[test]
fn constructed_from_a_provider() {
    let _scope = test_scope();
    let culture = culture_with_separators("x-test", ",", ".", "-", ".", "E");
    let provider = MaskedTextProvider::new_full("00/00", Some(culture.clone()), true, '#', '*', true).unwrap();

    let target = MaskedTextBox::with_provider(&provider);

    assert!(target.ascii_only());
    assert_eq!(Some(culture), target.culture());
    assert_eq!(Some("00/00"), target.mask().as_deref());
    assert_eq!('*', target.password_char());
    assert_eq!('#', target.prompt_char());
    assert_eq!(Some("##-##"), target.text().as_deref());

    let own = target.mask_provider().unwrap();
    assert_eq!('#', own.borrow().prompt_char());
    assert_eq!('*', own.borrow().password_char());
    assert!(own.borrow().ascii_only());
}

#[test]
fn the_provider_follows_the_properties() {
    let _scope = test_scope();
    let target = masked("00/00", Some("1234"));

    assert_eq!(Some("12/34"), target.text().as_deref());

    target.set_prompt_char('#');
    target.set_text(Some("1"));
    assert_eq!(Some("1#/##"), target.text().as_deref());

    target.set_culture(Some(culture_with_separators("x-test", ",", ".", "-", ".", "E")));
    // The new provider is given the displayed text, whose date separator is
    // not the one of the new culture: the text is rejected, as in the reference.
    assert_eq!(Some("##-##"), target.text().as_deref());

    target.set_reset_on_prompt(false);
    target.set_reset_on_space(false);
    let provider = target.mask_provider().unwrap();
    assert!(!provider.borrow().reset_on_prompt());
    assert!(!provider.borrow().reset_on_space());

    target.set_ascii_only(true);
    let provider = target.mask_provider().unwrap();
    assert!(provider.borrow().ascii_only());
    // A new provider takes the current values of the reset properties.
    assert!(!provider.borrow().reset_on_prompt());
    assert!(!provider.borrow().reset_on_space());
}

#[test]
#[should_panic(expected = "PasswordChar and PromptChar values cannot be the same.")]
fn prompt_char_cannot_be_the_password_char() {
    let _scope = test_scope();
    let target = MaskedTextBox::new();
    target.set_password_char('*');
    target.set_prompt_char('*');
}

#[test]
#[should_panic(expected = "PasswordChar and PromptChar values cannot be the same.")]
fn password_char_cannot_be_the_prompt_char_of_the_provider() {
    let _scope = test_scope();
    let target = MaskedTextBox::new();
    target.set_mask(Some("000"));
    target.set_password_char('_');
}

#[test]
#[should_panic(expected = "is not a valid value for PromptChar.")]
fn prompt_char_must_be_an_input_char() {
    let _scope = test_scope();
    let target = MaskedTextBox::new();
    target.set_prompt_char('\u{1}');
}

#[test]
#[should_panic(expected = "is not a valid value for PasswordChar.")]
fn password_char_must_be_valid() {
    let _scope = test_scope();
    let target = MaskedTextBox::new();
    target.set_password_char('\u{1}');
}

#[test]
fn hide_prompt_on_leave_hides_and_restores_the_prompt() {
    let _scope = test_scope();
    let _focus = focus_scope();
    let target = templated();
    target.set_mask(Some("000"));
    target.set_hide_prompt_on_leave(true);
    target.set_text(Some("1"));

    let other = templated();

    let sp = StackPanel::new();
    sp.children().add(target.clone());
    sp.children().add(other.clone());

    target.apply_template();
    other.apply_template();

    let _root = TestRoot::with_child(sp);

    target.focus();
    assert_eq!(Some("1__"), target.text().as_deref());

    other.focus();
    // The lost focus text goes through the coercion of the text, which
    // formats it with the prompt again, as in the reference.
    assert_eq!(Some("1__"), target.text().as_deref());

    target.focus();
    assert_eq!(Some("1__"), target.text().as_deref());
}

#[test]
fn paste_inserts_the_clipboard_text_into_the_mask() {
    let _scope = test_scope();
    let clipboard = TestClipboardImpl::new();
    let target = masked("00/00", None);

    let _top_level = window_with_clipboard(&target, &clipboard);

    clipboard.set_text(Some("1a23"));

    let args = raise_key_event(&target, Key::V, KeyModifiers::CONTROL);

    assert!(args.handled());
    assert_eq!(Some("12/3_"), target.text().as_deref());
    assert_eq!(4, target.caret_index());
    assert!(target.can_undo());
}

#[test]
fn paste_without_clipboard_text_does_nothing() {
    let _scope = test_scope();
    let clipboard = TestClipboardImpl::new();
    let target = masked("00/00", None);

    let _top_level = window_with_clipboard(&target, &clipboard);

    let args = raise_key_event(&target, Key::V, KeyModifiers::CONTROL);

    assert!(!args.handled());
    assert_eq!(Some("__/__"), target.text().as_deref());
}

// Not in the reference, which has no test for a failing paste: a clipboard
// that timed out is silently ignored, one that denied access is logged, and
// every other failure leaves the key handler and is raised on the
// dispatcher.
#[test]
fn paste_does_not_change_text_when_clipboard_fails() {
    let cases: [(ClipboardErrorKind, &[&str], bool); 5] = [
        (ClipboardErrorKind::Timeout, &[], false),
        (ClipboardErrorKind::AccessDenied, &["Failed to read text from clipboard: {Error}"], false),
        (ClipboardErrorKind::Canceled, &[], true),
        (ClipboardErrorKind::Platform, &[], true),
        (ClipboardErrorKind::Other, &[], true),
    ];

    for (kind, expected_messages, raised) in cases {
        let _scope = test_scope();
        let clipboard = TestClipboardImpl::throwing(kind);
        let target = masked("00/00", None);

        let _top_level = window_with_clipboard(&target, &clipboard);
        let (messages, sink) = record_log_messages();

        let mut args = None;
        let unhandled = run_and_capture_unhandled_panic(|| {
            args = Some(raise_key_event(&target, Key::V, KeyModifiers::CONTROL));
        });
        sink.dispose();

        assert!(!args.expect("the key event was raised").handled(), "{kind:?}");
        assert_eq!(1, clipboard.try_get_data_count(), "{kind:?}");
        assert_eq!(Some("__/__"), target.text().as_deref(), "{kind:?}");
        assert_eq!(expected_messages, messages.borrow().as_slice(), "{kind:?}");
        match unhandled {
            Some(payload) => {
                assert!(raised, "{kind:?}");
                let expected = ClipboardHelper::unhandled_message(&ClipboardError::from_kind(kind));
                assert_eq!(Some(expected.as_str()), panic_message(&*payload));
            }
            None => assert!(!raised, "{kind:?}"),
        }
    }
}

#[test]
fn delete_and_space_edit_the_mask() {
    let _scope = test_scope();
    // No template is applied: the text box itself does not edit the text.
    let target = masked("AAA", Some("abc"));

    target.set_caret_index(1);
    let args = raise_key_event(&target, Key::Delete, KeyModifiers::NONE);
    assert!(args.handled());
    assert_eq!(Some("ac_"), target.text().as_deref());
    assert_eq!(1, target.caret_index());

    let args = raise_key_event(&target, Key::Space, KeyModifiers::NONE);
    assert!(args.handled());
    assert_eq!(Some("a_c"), target.text().as_deref());
    assert_eq!(1, target.caret_index());
}

#[test]
fn space_resets_the_selection() {
    let _scope = test_scope();
    let target = masked("000", Some("123"));

    let _root = host_in_root(&target);

    target.set_selection_start(0);
    target.set_selection_end(2);

    raise_text_event(&target, " ");

    // The selection is removed from the provider ("3__"), then the space is
    // inserted at the caret as a reset of that position, which moves the
    // remaining digit, as in the reference.
    assert_eq!(Some("_3_"), target.text().as_deref());
}

#[test]
fn read_only_ignores_text_input() {
    let _scope = test_scope();
    let target = masked("000", Some("1"));
    target.set_is_read_only(true);

    raise_text_event(&target, "2");

    assert_eq!(Some("1__"), target.text().as_deref());
}
