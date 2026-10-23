//! Actual browser adapter callbacks through manager-owned RichEditBox editing,
//! routed interception, document formatting and transaction history.
use super::*;
use crate::browser_text_input_method::ITextInputPage;
use crate::browser_text_input_options::BrowserTextInputOptions;
use ferroui_base::input::text_input::{ITextInputMethodImpl, TextInputOptions};
use ferroui_base::input::{InputElement, InputScopeNameValue, TextInputEventArgs};
use ferroui_base::interactivity::RoutingStrategies;
use ferroui_base::media::text_formatting::testing::TextTestScope;
use ferroui_base::media::FontWeight;
use ferroui_base::Ref;
use ferroui_controls::documents::{CharacterFormat, Paragraph, Run};
use ferroui_controls::rich_text::TextFormatValue;
use ferroui_controls::templates::{FuncControlTemplate, FuncTemplateNameScopeExtensions};
use ferroui_controls::testing::{
    MockWindowingPlatform, TestServices, UnitTestApplication, UnitTestApplicationScope,
};
use ferroui_controls::{Border, Control, RichEditBox, StackPanel, Window};
use std::cell::RefCell;

#[derive(Default)]
struct PageState {
    text: RefCell<String>,
    selection: Cell<TextSelection>,
    revision: Cell<u32>,
    writes: Cell<usize>,
    options: RefCell<Vec<BrowserTextInputOptions>>,
    before_text: RefCell<Option<Box<dyn FnOnce()>>>,
}

struct RecordingPage(Rc<PageState>);

impl ITextInputPage for RecordingPage {
    fn hide_input_element(&self) {}
    fn show_input_element(&self) {}
    fn focus_input_element(&self) {}
    fn focus_container_element(&self) {}
    fn clear_input_element(&self, revision: u32) {
        self.set_surrounding_text("", 0, 0, revision);
    }
    fn set_surrounding_text(&self, text: &str, anchor: i32, active: i32, revision: u32) {
        let callback = self.0.before_text.borrow_mut().take();
        if let Some(callback) = callback {
            callback();
        }
        // Match the real textarea API so a raw CR source cannot accidentally
        // pass adapter tests by remaining unnormalized in this recorder.
        self.0
            .text
            .replace(text.replace("\r\n", "\n").replace('\r', "\n"));
        self.0.selection.set(TextSelection::new(anchor, active));
        self.0.revision.set(revision);
        self.0.writes.set(self.0.writes.get() + 1);
    }
    fn set_bounds(&self, _x: i32, _y: i32, _width: i32, _height: i32, _caret: i32) {}
    fn set_input_options(&self, options: &BrowserTextInputOptions) {
        self.0.options.borrow_mut().push(options.clone());
    }
    fn selection_start(&self) -> i32 {
        let selection = self.0.selection.get();
        selection.start.min(selection.end)
    }
    fn selection_end(&self) -> i32 {
        let selection = self.0.selection.get();
        selection.start.max(selection.end)
    }
}

struct Fixture {
    window: Ref<Window>,
    owner: Ref<RichEditBox>,
    other: Ref<RichEditBox>,
    method: Rc<BrowserTextInputMethod>,
    page: Rc<PageState>,
    _text: TextTestScope,
    _app: UnitTestApplicationScope,
}

impl Fixture {
    fn new() -> Self {
        let page = Rc::new(PageState::default());
        let method =
            BrowserTextInputMethod::with_page(Weak::new(), Box::new(RecordingPage(page.clone())));
        let platform_method = method.clone();
        let app =
            UnitTestApplication::start(TestServices::focusable_window().with_windowing_platform(
                MockWindowingPlatform::with_window_impl(move || {
                    let window = MockWindowingPlatform::create_window_mock();
                    window.setup_feature::<dyn ITextInputMethodImpl>(platform_method.clone());
                    window
                }),
            ));
        let text = TextTestScope::new();
        let owner = Self::editor();
        owner.select(1, 3);
        owner
            .apply_format(&CharacterFormat {
                font_weight: Some(FontWeight::Bold),
                ..Default::default()
            })
            .unwrap();
        owner.editor().clear_history();
        let other = Self::editor();
        other.select(1, 3);
        let panel = StackPanel::new();
        panel.children().add(owner.clone());
        panel.children().add(other.clone());
        let window = Window::new();
        window.set_width(260.0);
        window.set_height(180.0);
        window.set_content(Some(Control::boxed(panel)));
        window.show();
        owner.focus();
        assert!(owner.is_focused());
        let fixture = Self {
            window,
            owner,
            other,
            method,
            page,
            _text: text,
            _app: app,
        };
        assert!(fixture.connection().is_current());
        fixture.assert_mirror();
        fixture
    }

    fn editor() -> Ref<RichEditBox> {
        let owner = RichEditBox::new();
        owner.set_template(Some(FuncControlTemplate::for_type::<RichEditBox>(
            |_, scope| {
                let host = Border::new();
                host.set_name(Some("ContentElement".to_owned()));
                host.register_in_name_scope(&**scope).upcast()
            },
        )));
        owner.set_height(80.0);
        owner.set_accepts_return(true);
        owner.set_text("abcd").unwrap();
        owner.editor().clear_history();
        owner
    }

    fn connection(&self) -> Rc<TextInputMethodConnection> {
        self.method
            .connection
            .borrow()
            .clone()
            .expect("the focused rich editor supplies an exact input connection")
    }

    fn before(&self, input_type: &str) -> (u32, u32) {
        let revision = self.page.revision.get();
        let selection = self.page.selection.get();
        let value = self.page.text.borrow().clone();
        let token = self.method.on_dom_before_input(
            revision,
            input_type,
            &value,
            selection.start,
            selection.end,
        );
        assert_ne!(token, 0, "{input_type} should capture a stable beforeinput");
        (revision, token)
    }

    fn input(
        &self,
        ticket: (u32, u32),
        input_type: &str,
        value: &str,
        anchor: i32,
        active: i32,
    ) -> bool {
        self.method
            .on_dom_input(ticket.0, ticket.1, input_type, value, anchor, active)
    }

    fn compose(&self) -> u32 {
        let value = self.page.text.borrow().clone();
        let selection = self.page.selection.get();
        let token = self.method.on_dom_composition_start(
            self.page.revision.get(),
            &value,
            selection.start,
            selection.end,
        );
        assert_ne!(token, 0);
        assert!(self.method.is_composing());
        token
    }

    fn assert_mirror(&self) {
        let state = self.connection().edit_state().unwrap();
        let projected = DomTextProjection::new(&state).unwrap();
        assert_eq!(*self.page.text.borrow(), projected.state.text);
        assert_eq!(self.page.selection.get(), projected.state.selection);
    }

    fn set_raw_run(&self, text: &str) {
        let paragraph = Paragraph::new();
        paragraph.add_child(Run::with_text(Some(text)));
        self.owner.blocks().clear();
        self.owner.blocks().add(paragraph);
        self.owner.select(0, 0);
        self.owner.editor().clear_history();
        assert_eq!(
            self.owner.text(),
            text,
            "direct Run text preserves exact model line endings"
        );
    }

    fn record_input(&self) -> Rc<Cell<usize>> {
        let count = Rc::new(Cell::new(0));
        let recorded = count.clone();
        self.owner.add_handler_with(
            InputElement::text_input_event(),
            move |_, _: &TextInputEventArgs| recorded.set(recorded.get() + 1),
            RoutingStrategies::TUNNEL,
            false,
        );
        count
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.window.close();
    }
}

#[test]
fn browser_dom_input_correction_uses_real_exact_transaction_and_preserves_formatting() {
    let f = Fixture::new();
    f.owner.set_text("teh cats").unwrap();
    f.owner.select(4, 8);
    f.owner
        .apply_format(&CharacterFormat {
            font_weight: Some(FontWeight::Bold),
            ..Default::default()
        })
        .unwrap();
    f.owner.select(8, 8);
    f.owner.editor().clear_history();
    let before = f.owner.document().snapshot();
    let count = f.record_input();
    let ticket = f.before("insertReplacementText");

    assert!(f.input(ticket, "insertReplacementText", "the cats", 8, 8));

    assert_eq!(f.owner.text(), "the cats");
    assert_eq!(count.get(), 1);
    assert_eq!(
        f.owner
            .document()
            .character_format_at(5)
            .unwrap()
            .font_weight,
        Some(FontWeight::Bold)
    );
    f.assert_mirror();
    assert!(f.owner.undo().unwrap());
    assert_eq!(f.owner.document().snapshot(), before);
    assert!(!f.owner.undo().unwrap());
    assert!(f.owner.redo().unwrap());
    assert_eq!(f.owner.text(), "the cats");
    f.assert_mirror();
}

#[test]
fn browser_dom_input_repeated_character_keeps_the_native_insertion_in_its_formatted_run() {
    let f = Fixture::new();
    f.owner.set_text("aaaa").unwrap();
    f.owner.select(2, 4);
    f.owner
        .apply_format(&CharacterFormat {
            font_weight: Some(FontWeight::Bold),
            ..Default::default()
        })
        .unwrap();
    f.owner.select(2, 2);
    f.owner.editor().clear_history();
    let before = f.owner.document().snapshot();
    let ticket = f.before("insertText");

    assert!(f.input(ticket, "insertText", "aaaaa", 3, 3));

    assert_ne!(
        f.owner
            .document()
            .character_format_at(3)
            .unwrap()
            .font_weight,
        Some(FontWeight::Bold)
    );
    assert_eq!(
        f.owner
            .document()
            .character_format_at(4)
            .unwrap()
            .font_weight,
        Some(FontWeight::Bold)
    );
    assert!(f.owner.undo().unwrap());
    assert_eq!(f.owner.document().snapshot(), before);
}

#[test]
fn browser_dom_input_mobile_without_beforeinput_preserves_lf_and_directed_utf16_selection() {
    let f = Fixture::new();
    f.owner.set_text("a\r\n😀b").unwrap();
    f.owner.select(4, 4);
    f.owner.editor().clear_history();
    assert_eq!(*f.page.text.borrow(), "a\n😀b");
    let revision = f.page.revision.get();

    assert!(f.input((revision, 0), "insertText", "a\n😀\nZb", 6, 4));

    assert_eq!(f.owner.text(), "a\n😀\nZb");
    assert_eq!((f.owner.selection_start(), f.owner.selection_end()), (6, 4));
    f.assert_mirror();
    assert!(f.owner.undo().unwrap());
    assert_eq!(f.owner.text(), "a\n😀b");
    assert!(!f.owner.undo().unwrap());
}

#[test]
fn browser_dom_input_raw_run_newlines_are_mirrored_without_becoming_a_noop_edit() {
    let f = Fixture::new();
    f.set_raw_run("a\r\n😀\rb");
    f.owner.select(7, 3);
    let before = f.owner.document().snapshot();
    let count = f.record_input();
    assert_eq!(*f.page.text.borrow(), "a\n😀\nb");
    assert_eq!(f.page.selection.get(), TextSelection::new(6, 2));

    assert!(f.input((f.page.revision.get(), 0), "insertText", "a\n😀\nb", 6, 2));

    assert_eq!(f.owner.document().snapshot(), before);
    assert_eq!((f.owner.selection_start(), f.owner.selection_end()), (7, 3));
    assert_eq!(count.get(), 0);
    assert!(!f.owner.can_undo());
    f.assert_mirror();

    // An actual mobile insertion uses the normalized caret to locate the edit
    // after the lone CR, retaining both original line-ending spellings.
    assert!(f.input((f.page.revision.get(), 0), "insertText", "a\n😀\nZb", 6, 5));
    assert_eq!(f.owner.text(), "a\r\n😀\rZb");
    assert_eq!((f.owner.selection_start(), f.owner.selection_end()), (7, 6));
    assert_eq!(count.get(), 1);
    f.assert_mirror();
    assert!(f.owner.undo().unwrap());
    assert_eq!(f.owner.document().snapshot(), before);
    assert!(!f.owner.undo().unwrap());
}

#[test]
fn browser_dom_input_raw_run_replacement_maps_both_selections_and_preserves_tail_formatting() {
    let f = Fixture::new();
    f.set_raw_run("head\r\n😀\rtail");
    f.owner.select(9, 13);
    f.owner
        .apply_format(&CharacterFormat {
            font_weight: Some(FontWeight::Bold),
            ..Default::default()
        })
        .unwrap();
    f.owner.select(8, 6);
    f.owner.editor().clear_history();
    let before = f.owner.document().snapshot();
    assert_eq!(f.page.selection.get(), TextSelection::new(7, 5));
    let ticket = f.before("insertReplacementText");

    assert!(f.input(ticket, "insertReplacementText", "head\nX\ntail", 6, 5));

    assert_eq!(f.owner.text(), "head\r\nX\rtail");
    assert_eq!((f.owner.selection_start(), f.owner.selection_end()), (7, 6));
    // Offset 8 is the insertion boundary before the tail. Query its complete
    // nonempty range so caret affinity cannot select the preceding CR's format.
    let TextFormatValue::Uniform(tail) = f
        .owner
        .text_document()
        .get_range(8, 12)
        .unwrap()
        .character_format()
        .unwrap()
    else {
        panic!("the whole retained tail must keep a uniform format");
    };
    assert_eq!(tail.font_weight, Some(FontWeight::Bold));
    f.assert_mirror();
    assert!(f.owner.undo().unwrap());
    assert_eq!(f.owner.document().snapshot(), before);
    assert!(!f.owner.undo().unwrap());
    assert!(f.owner.redo().unwrap());
    assert_eq!(f.owner.text(), "head\r\nX\rtail");
}

#[test]
fn browser_dom_input_deletes_the_whole_raw_crlf_and_preserves_other_line_endings() {
    let f = Fixture::new();
    f.set_raw_run("a\r\nb\rc");
    f.owner.select(3, 3);
    let before = f.owner.document().snapshot();
    assert_eq!(f.page.selection.get(), TextSelection::new(2, 2));
    let ticket = f.before("deleteContentBackward");

    assert!(f.input(ticket, "deleteContentBackward", "ab\nc", 1, 1));

    assert_eq!(f.owner.text(), "ab\rc");
    assert_eq!((f.owner.selection_start(), f.owner.selection_end()), (1, 1));
    f.assert_mirror();
    assert!(f.owner.undo().unwrap());
    assert_eq!(f.owner.document().snapshot(), before);
    assert!(!f.owner.undo().unwrap());
}

#[test]
fn browser_dom_input_rejects_unrepresentable_crlf_carets_and_projected_surrogate_interiors() {
    let f = Fixture::new();
    f.set_raw_run("a\r\n😀\rb");
    f.owner.select(3, 3);
    let before = f.owner.document().snapshot();
    let ticket = f.before("insertText");
    assert!(!f.input(ticket, "insertText", "a\n😀\nb", 3, 3));
    assert_eq!(f.owner.document().snapshot(), before);
    assert!(!f.owner.can_undo());

    f.owner.select(2, 2);
    assert_eq!((f.owner.selection_start(), f.owner.selection_end()), (2, 2));
    assert_eq!(f.page.selection.get(), TextSelection::new(1, 1));
    let revision = f.page.revision.get();
    assert_eq!(
        f.method
            .on_dom_before_input(revision, "insertText", "a\n😀\nb", 1, 1),
        0
    );
    assert!(!f.input((f.page.revision.get(), 0), "insertText", "ax\n😀\nb", 2, 2));
    assert_eq!(
        f.method
            .on_dom_composition_start(f.page.revision.get(), "a\n😀\nb", 1, 1),
        0
    );
    assert_eq!(f.owner.document().snapshot(), before);
    assert_eq!((f.owner.selection_start(), f.owner.selection_end()), (2, 2));
    assert!(!f.owner.can_undo());
    assert!(!f.owner.editor().is_composing());
    f.assert_mirror();
}

#[test]
fn browser_dom_input_rejects_new_crlf_pairs_without_rewriting_adjacent_model_content() {
    let f = Fixture::new();
    for (source, start, end, input_type, value, caret) in [
        ("a\rb", 2, 2, "insertLineBreak", "a\n\nb", 3),
        ("a\rx\nb", 2, 3, "deleteContentBackward", "a\n\nb", 2),
    ] {
        f.set_raw_run(source);
        f.owner.select(start, end);
        let before = f.owner.document().snapshot();
        let ticket = f.before(input_type);
        assert!(!f.input(ticket, input_type, value, caret, caret));
        assert_eq!(f.owner.document().snapshot(), before);
        assert_eq!(
            (f.owner.selection_start(), f.owner.selection_end()),
            (start, end)
        );
        assert!(!f.owner.can_undo());
        f.assert_mirror();
    }
}

#[test]
fn browser_dom_composition_raw_run_newlines_survive_cancel_commit_and_owner_change() {
    let f = Fixture::new();
    f.set_raw_run("a\r\nbc\rd");
    f.owner.select(3, 5);
    let before = f.owner.document().snapshot();
    let token = f.compose();
    f.method.on_dom_composition_update(token, "候補");
    assert_eq!(f.owner.text(), "a\r\n候補\rd");
    f.method.on_dom_composition_end(token, "", "a\nbc\nd", 2, 4);
    assert_eq!(f.owner.document().snapshot(), before);
    assert!(!f.owner.can_undo());
    f.assert_mirror();

    let count = f.record_input();
    let token = f.compose();
    f.method.on_dom_composition_update(token, "候補");
    f.method
        .on_dom_composition_end(token, "日本", "a\n日本\nd", 4, 2);
    assert_eq!(f.owner.text(), "a\r\n日本\rd");
    assert_eq!((f.owner.selection_start(), f.owner.selection_end()), (5, 3));
    assert_eq!(count.get(), 1);
    f.assert_mirror();
    assert!(f.owner.undo().unwrap());
    assert_eq!(f.owner.document().snapshot(), before);
    assert!(!f.owner.undo().unwrap());

    f.owner.select(3, 5);
    let token = f.compose();
    f.method.on_dom_composition_update(token, "preview");
    f.other.focus();
    assert_eq!(f.owner.document().snapshot(), before);
    f.method
        .on_dom_composition_end(token, "late", "a\nlate\nd", 6, 6);
    assert_eq!(f.owner.document().snapshot(), before);
    assert_eq!(f.other.text(), "abcd");
    assert_eq!(count.get(), 1);
    assert!(!f.owner.can_undo());
    assert!(!f.other.can_undo());
    f.assert_mirror();
}

#[test]
fn browser_dom_composition_rejects_a_new_crlf_pair_and_restores_its_exact_preview_source() {
    let f = Fixture::new();
    f.set_raw_run("a\rb");
    f.owner.select(2, 2);
    let before = f.owner.document().snapshot();
    let count = f.record_input();
    let token = f.compose();
    f.method.on_dom_composition_update(token, "\n");

    f.method.on_dom_composition_end(token, "\n", "a\n\nb", 3, 3);

    assert_eq!(f.owner.document().snapshot(), before);
    assert_eq!(count.get(), 0);
    assert!(!f.owner.can_undo());
    assert!(!f.method.is_composing());
    assert!(!f.owner.editor().is_composing());
    f.assert_mirror();
}

#[test]
fn browser_dom_input_empty_replacement_deletes_the_selection_and_undoes_once() {
    let f = Fixture::new();
    let before = f.owner.document().snapshot();
    let ticket = f.before("deleteContentBackward");

    assert!(f.input(ticket, "deleteContentBackward", "ad", 1, 1));

    assert_eq!(f.owner.text(), "ad");
    f.assert_mirror();
    assert!(f.owner.undo().unwrap());
    assert_eq!(f.owner.document().snapshot(), before);
    assert!(!f.owner.undo().unwrap());
}

#[test]
fn browser_dom_input_after_handled_keyboard_commit_does_not_dispatch_or_record_it_twice() {
    let f = Fixture::new();
    let count = f.record_input();
    assert!(f.connection().commit_text("X", 1).unwrap());
    let revision = f.page.revision.get();
    assert_eq!(f.owner.text(), "aXd");
    assert_eq!(count.get(), 1);

    assert!(f.input((revision, 0), "insertText", "aXd", 2, 2));

    assert_eq!(count.get(), 1);
    assert!(f.owner.undo().unwrap());
    assert_eq!(f.owner.text(), "abcd");
    assert!(!f.owner.undo().unwrap());
}

#[test]
fn browser_dom_input_captured_before_focus_change_cannot_edit_an_identical_new_owner() {
    let f = Fixture::new();
    let before = f.owner.document().snapshot();
    let other = f.other.document().snapshot();
    let ticket = f.before("insertText");
    f.other.focus();
    assert!(f.other.is_focused());

    assert!(!f.input(ticket, "insertText", "aXd", 2, 2));

    assert_eq!(f.owner.document().snapshot(), before);
    assert_eq!(f.other.document().snapshot(), other);
    assert!(!f.owner.can_undo());
    assert!(!f.other.can_undo());
    f.assert_mirror();
}

#[test]
fn browser_dom_input_becoming_read_only_invalidates_pending_edit_without_changing_text_or_history()
{
    let f = Fixture::new();
    let before = f.owner.document().snapshot();
    let ticket = f.before("insertText");
    f.owner.set_is_read_only(true);

    assert!(!f.input(ticket, "insertText", "aXd", 2, 2));

    assert_eq!(f.owner.document().snapshot(), before);
    assert!(!f.owner.can_undo());
    assert!(f.method.client().is_none());
    assert_eq!(*f.page.text.borrow(), "");
}

#[test]
fn browser_dom_input_document_or_selection_round_trip_rejects_a_previously_captured_edit() {
    let f = Fixture::new();
    let ticket = f.before("insertText");
    let original = f.owner.document().snapshot();
    f.owner.select(0, 0);
    f.owner.select(1, 3);
    assert_eq!(f.owner.document().snapshot(), original);
    assert!(!f.input(ticket, "insertText", "aXd", 2, 2));
    assert_eq!(f.owner.document().snapshot(), original);
    assert!(!f.owner.can_undo());

    let ticket = f.before("insertText");
    f.owner
        .apply_format(&CharacterFormat {
            font_weight: Some(FontWeight::Normal),
            ..Default::default()
        })
        .unwrap();
    let changed = f.owner.document().snapshot();
    assert!(!f.input(ticket, "insertText", "aXd", 2, 2));
    assert_eq!(f.owner.document().snapshot(), changed);
    assert!(f.owner.undo().unwrap());
    assert_eq!(f.owner.document().snapshot(), original);
    assert!(!f.owner.undo().unwrap());
    f.assert_mirror();
}

#[test]
fn browser_dom_input_rejected_policy_or_surrogate_range_resynchronizes_without_history() {
    let f = Fixture::new();
    f.owner.set_text("a😀b").unwrap();
    f.owner.select(1, 3);
    f.owner.editor().clear_history();
    let before = f.owner.document().snapshot();
    for (value, anchor, active) in [("a😀b", 2, 2), ("aXb", -1, 2), ("aXb", 2, 10)] {
        let ticket = f.before("insertReplacementText");
        assert!(!f.input(ticket, "insertReplacementText", value, anchor, active));
        assert_eq!(f.owner.document().snapshot(), before);
        f.assert_mirror();
    }
    f.owner.set_accepts_return(false);
    let ticket = f.before("insertLineBreak");
    assert!(!f.input(ticket, "insertLineBreak", "a\nb", 2, 2));
    f.owner.set_max_length(4);
    let ticket = f.before("insertText");
    assert!(!f.input(ticket, "insertText", "alongb", 5, 5));
    assert_eq!(f.owner.document().snapshot(), before);
    assert!(!f.owner.can_undo());
    f.assert_mirror();
}

#[test]
fn browser_dom_input_intercepted_edit_and_reentrant_focus_leave_canonical_owners_intact() {
    let f = Fixture::new();
    let before = f.owner.document().snapshot();
    let handler = f.owner.add_handler_with(
        InputElement::text_input_event(),
        |_, event: &TextInputEventArgs| event.set_handled(true),
        RoutingStrategies::TUNNEL,
        false,
    );
    let ticket = f.before("insertReplacementText");
    assert!(!f.input(ticket, "insertReplacementText", "aXd", 2, 2));
    assert_eq!(f.owner.document().snapshot(), before);
    assert!(!f.owner.can_undo());
    f.assert_mirror();
    f.owner
        .remove_handler(InputElement::text_input_event(), handler);

    let other = f.other.clone();
    f.owner.add_handler_with(
        InputElement::text_input_event(),
        move |_, _: &TextInputEventArgs| {
            other.focus();
        },
        RoutingStrategies::TUNNEL,
        false,
    );
    let ticket = f.before("insertReplacementText");
    assert!(!f.input(ticket, "insertReplacementText", "aXd", 2, 2));
    assert_eq!(f.owner.document().snapshot(), before);
    assert_eq!(f.other.text(), "abcd");
    assert!(!f.owner.can_undo());
    assert!(!f.other.can_undo());
    f.assert_mirror();
}

#[test]
fn browser_dom_input_mirror_reentry_keeps_latest_owner_after_late_old_page_write() {
    let f = Fixture::new();
    f.other.set_text("second😀").unwrap();
    f.other.select(8, 6);
    let other = f.other.clone();
    f.page.before_text.replace(Some(Box::new(move || {
        other.focus();
    })));

    f.owner.select(0, 0);

    assert!(f.other.is_focused());
    assert_eq!(*f.page.text.borrow(), "second😀");
    assert_eq!(f.page.selection.get(), TextSelection::new(8, 6));
    f.assert_mirror();
}

#[test]
fn browser_dom_composition_preserves_native_preview_then_commits_one_rich_undo_unit() {
    let f = Fixture::new();
    let before = f.owner.document().snapshot();
    let token = f.compose();
    let writes = f.page.writes.get();
    f.method.on_dom_composition_update(token, "候補");
    assert_eq!(f.owner.text(), "a候補d");
    assert_eq!(f.page.writes.get(), writes);
    assert!(!f.owner.can_undo());
    let count = f.record_input();

    f.method
        .on_dom_composition_end(token, "確定", "a確定d", 3, 3);

    assert_eq!(f.owner.text(), "a確定d");
    assert!(!f.method.is_composing());
    assert!(!f.owner.editor().is_composing());
    assert_eq!(count.get(), 1);
    f.assert_mirror();
    let revision = f.page.revision.get();
    assert!(f.input((revision, 0), "insertText", "a確定d", 3, 3));
    assert_eq!(count.get(), 1);
    assert!(f.owner.undo().unwrap());
    assert_eq!(f.owner.document().snapshot(), before);
    assert!(!f.owner.undo().unwrap());
}

#[test]
fn browser_dom_composition_cancel_empty_deletion_and_interception_restore_the_right_snapshot() {
    let f = Fixture::new();
    let before = f.owner.document().snapshot();
    let token = f.compose();
    f.method.on_dom_composition_update(token, "preview");
    f.method.on_dom_composition_end(token, "", "abcd", 1, 3);
    assert_eq!(f.owner.document().snapshot(), before);
    assert!(!f.owner.can_undo());
    f.assert_mirror();

    let token = f.compose();
    f.method.on_dom_composition_update(token, "");
    f.method.on_dom_composition_end(token, "", "ad", 1, 1);
    assert_eq!(f.owner.text(), "ad");
    assert!(f.owner.undo().unwrap());
    assert_eq!(f.owner.document().snapshot(), before);
    assert!(!f.owner.undo().unwrap());
    f.owner.editor().clear_history();

    let token = f.compose();
    f.method.on_dom_composition_update(token, "preview");
    f.owner.add_handler_with(
        InputElement::text_input_event(),
        |_, event: &TextInputEventArgs| event.set_handled(true),
        RoutingStrategies::TUNNEL,
        false,
    );
    f.method
        .on_dom_composition_end(token, "commit", "acommitd", 7, 7);
    assert_eq!(f.owner.document().snapshot(), before);
    assert!(!f.owner.can_undo());
    assert!(!f.method.is_composing());
    assert!(!f.owner.editor().is_composing());
    f.assert_mirror();
}

#[test]
fn browser_dom_composition_empty_final_commits_directed_selection_and_one_undo_unit() {
    for previews in [
        vec![],
        vec!["preview"],
        vec!["候補", "😀", ""],
        vec!["😀", "next"],
    ] {
        let f = Fixture::new();
        let before = f.owner.document().snapshot();
        let count = f.record_input();
        let token = f.compose();
        for preview in previews {
            f.method.on_dom_composition_update(token, preview);
            assert!(!f.owner.can_undo());
        }

        f.method.on_dom_composition_end(token, "", "ad", 2, 0);

        assert_eq!(f.owner.text(), "ad");
        assert_eq!((f.owner.selection_start(), f.owner.selection_end()), (2, 0));
        assert!(!f.method.is_composing());
        assert!(!f.owner.editor().is_composing());
        assert_eq!(count.get(), 1);
        f.assert_mirror();
        let after = f.owner.document().snapshot();
        assert!(f.owner.undo().unwrap());
        assert_eq!(f.owner.document().snapshot(), before);
        assert_eq!((f.owner.selection_start(), f.owner.selection_end()), (1, 3));
        assert!(!f.owner.undo().unwrap());
        assert!(f.owner.redo().unwrap());
        assert_eq!(f.owner.document().snapshot(), after);
        assert_eq!((f.owner.selection_start(), f.owner.selection_end()), (2, 0));
        assert!(!f.owner.redo().unwrap());
    }
}

#[test]
fn browser_dom_composition_empty_final_preserves_raw_line_endings_and_maps_selection() {
    let f = Fixture::new();
    f.set_raw_run("a\r\nbc\rd");
    f.owner.select(3, 5);
    let before = f.owner.document().snapshot();
    let token = f.compose();
    f.method.on_dom_composition_update(token, "😀\r\n候補");
    assert_eq!(f.owner.text(), "a\r\n😀\n候補\rd");

    f.method.on_dom_composition_end(token, "", "a\n\nd", 4, 2);

    assert_eq!(f.owner.text(), "a\r\n\rd");
    assert_eq!((f.owner.selection_start(), f.owner.selection_end()), (5, 3));
    f.assert_mirror();
    assert!(f.owner.undo().unwrap());
    assert_eq!(f.owner.document().snapshot(), before);
    assert!(!f.owner.undo().unwrap());
    assert!(f.owner.redo().unwrap());
    assert_eq!((f.owner.selection_start(), f.owner.selection_end()), (5, 3));
}

#[test]
fn browser_dom_composition_invalid_final_value_or_selection_cancels_without_history() {
    for (data, value, anchor, active) in [
        ("", "aXd", 1, 1),
        ("", "ad", -1, 1),
        ("", "ad", 1, 3),
        ("X", "aYd", 2, 2),
        ("😀", "a😀d", 2, 2),
        ("X", "aXd", 2, 10),
    ] {
        let f = Fixture::new();
        let before = f.owner.document().snapshot();
        let count = f.record_input();
        let token = f.compose();
        f.method.on_dom_composition_update(token, "preview");

        f.method
            .on_dom_composition_end(token, data, value, anchor, active);

        assert_eq!(f.owner.document().snapshot(), before);
        assert_eq!((f.owner.selection_start(), f.owner.selection_end()), (1, 3));
        assert_eq!(count.get(), 0);
        assert!(!f.owner.can_undo());
        assert!(!f.method.is_composing());
        assert!(!f.owner.editor().is_composing());
        f.assert_mirror();
    }
}

#[test]
fn browser_dom_composition_intercepted_empty_final_or_focus_handoff_restores_owner() {
    for handoff in [false, true] {
        let f = Fixture::new();
        let before = f.owner.document().snapshot();
        let other_before = f.other.document().snapshot();
        let token = f.compose();
        f.method.on_dom_composition_update(token, "preview");
        let other = f.other.clone();
        f.owner.add_handler_with(
            InputElement::text_input_event(),
            move |_, event: &TextInputEventArgs| {
                assert_eq!(event.text.as_deref(), Some(""));
                if handoff {
                    other.focus();
                } else {
                    event.set_handled(true);
                }
            },
            RoutingStrategies::TUNNEL,
            false,
        );

        f.method.on_dom_composition_end(token, "", "ad", 1, 1);

        assert_eq!(f.owner.document().snapshot(), before);
        assert_eq!(f.other.document().snapshot(), other_before);
        assert!(!f.owner.can_undo());
        assert!(!f.other.can_undo());
        assert!(!f.owner.editor().is_composing());
        assert!(!f.method.is_composing());
        assert_eq!(f.other.is_focused(), handoff);
        f.assert_mirror();
    }
}

#[test]
fn browser_dom_composition_empty_final_after_read_only_cannot_modify_owner() {
    let f = Fixture::new();
    let before = f.owner.document().snapshot();
    let token = f.compose();
    f.method.on_dom_composition_update(token, "preview");
    f.owner.set_is_read_only(true);

    f.method.on_dom_composition_end(token, "", "ad", 1, 1);

    assert_eq!(f.owner.document().snapshot(), before);
    assert!(!f.owner.can_undo());
    assert!(!f.owner.editor().is_composing());
    assert!(!f.method.is_composing());
    assert!(f.method.client().is_none());
    assert_eq!(*f.page.text.borrow(), "");
}

#[test]
fn browser_dom_composition_late_end_after_focus_change_does_not_borrow_the_next_owner() {
    let f = Fixture::new();
    let before = f.owner.document().snapshot();
    let token = f.compose();
    f.method.on_dom_composition_update(token, "preview");
    f.other.focus();
    assert_eq!(f.owner.document().snapshot(), before);

    f.method.on_dom_composition_update(token, "late");
    f.method
        .on_dom_composition_end(token, "late", "alated", 5, 5);

    assert_eq!(f.owner.document().snapshot(), before);
    assert_eq!(f.other.text(), "abcd");
    assert!(!f.owner.can_undo());
    assert!(!f.other.can_undo());
    assert!(!f.method.is_composing());
    f.assert_mirror();
}

#[test]
fn browser_dom_input_prediction_requires_exact_connection_and_preserves_composition_on_hint_change()
{
    let f = Fixture::new();
    f.owner.set_is_text_prediction_enabled(true);
    assert!(f.page.options.borrow().last().unwrap().suggestions);
    let token = f.compose();
    f.method.on_dom_composition_update(token, "候補");
    let document = f.owner.document().snapshot();
    let writes = f.page.writes.get();
    let generation = f.method.generation.get();
    f.owner
        .set_input_scope(InputScopeNameValue::EmailSmtpAddress);
    assert_eq!(f.page.options.borrow().last().unwrap().input_mode, "email");
    assert!(f.page.options.borrow().last().unwrap().suggestions);
    assert_eq!(f.page.writes.get(), writes);
    assert_eq!(f.method.generation.get(), generation);
    assert_eq!(f.owner.document().snapshot(), document);
    assert!(f.method.is_composing());
    TextInputOptions::set_is_sensitive(&f.owner, true);
    assert!(!f.page.options.borrow().last().unwrap().suggestions);
    assert!(f.page.options.borrow().last().unwrap().sensitive);
    assert!(f.method.is_composing());
    f.method
        .on_dom_composition_end(token, "候補", "a候補d", 3, 3);
    f.method.set_client(None);
    f.method.set_options(&TextInputOptions {
        show_suggestions: Some(true),
        ..Default::default()
    });
    assert!(!f.page.options.borrow().last().unwrap().suggestions);
}
