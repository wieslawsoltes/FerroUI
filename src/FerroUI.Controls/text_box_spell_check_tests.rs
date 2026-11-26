//! Spell checking of a text box (#1261): the checks through the registered spell checker, the
//! underlines of the presenter, the edits that keep them, and the proofing items of the context
//! flyout.

use crate::presenters::text_presenter::spelling_error_geometry;
use crate::presenters::TextPresenter;
use crate::testing::MockWindowingPlatform;
use crate::text_box::text_box_spell_check::adjust_spelling_errors;
use crate::text_box_tests::{show_in_window, templated_text_box, test_scope};
use crate::{Control, MenuFlyout, MenuItem, TextBox};
use ferroui_base::input::ContextRequestedEventArgs;
use ferroui_base::interactivity::{IRoutedEventArgs, RoutedEventArgs};
use ferroui_base::platform::{
    register_spell_checker, ISpellChecker, SpellCheckCompletion, SpellingError,
};
use ferroui_base::threading::Dispatcher;
use ferroui_base::{AnyValue, BoxedValue, Point, Rect, Ref};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// A spell checker that knows the misspellings `helo` and `wrold`.
#[derive(Default)]
struct TestSpellChecker {
    checks: Cell<usize>,
    languages: RefCell<Vec<Option<String>>>,
    ignored: RefCell<Vec<String>>,
    added: RefCell<Vec<String>>,
}

impl ISpellChecker for TestSpellChecker {
    fn check_spelling(
        &self,
        text: Rc<[u16]>,
        language: Option<&str>,
        completion: SpellCheckCompletion,
    ) {
        self.checks.set(self.checks.get() + 1);
        self.languages
            .borrow_mut()
            .push(language.map(str::to_owned));
        let text = String::from_utf16_lossy(&text);
        let mut errors = Vec::new();
        let mut start = 0;
        for word in text.split(|c: char| !c.is_alphanumeric()) {
            let known = self
                .ignored
                .borrow()
                .iter()
                .chain(self.added.borrow().iter())
                .any(|w| w == word);
            if (word == "helo" || word == "wrold") && !known {
                errors.push(SpellingError::new(start, word.encode_utf16().count()));
            }
            start += word.encode_utf16().count() + 1;
        }
        completion(errors);
    }

    fn suggestions(&self, word: &str, _language: Option<&str>) -> Vec<String> {
        match word {
            "helo" => ["hello", "help", "hero", "halo"]
                .map(str::to_owned)
                .to_vec(),
            "wrold" => vec!["world".to_owned()],
            _ => Vec::new(),
        }
    }

    fn ignore_word(&self, word: &str) {
        self.ignored.borrow_mut().push(word.to_owned());
    }

    fn add_to_dictionary(&self, word: &str) {
        self.added.borrow_mut().push(word.to_owned());
    }
}

fn checker() -> Rc<TestSpellChecker> {
    let checker = Rc::new(TestSpellChecker::default());
    register_spell_checker(checker.clone());
    checker
}

fn presenter_of(text_box: &TextBox) -> Ref<TextPresenter> {
    text_box.presenter().expect("the template has a presenter")
}

fn errors_of(text_box: &TextBox) -> Vec<SpellingError> {
    presenter_of(text_box)
        .spelling_errors()
        .map(|errors| errors.to_vec())
        .unwrap_or_default()
}

fn run_jobs() {
    Dispatcher::ui_thread().run_jobs(None);
}

fn header_of(item: &Option<BoxedValue>) -> Option<String> {
    let item = item.as_ref()?;
    let value: &dyn AnyValue = &**item;
    let control = value.downcast_ref::<Ref<Control>>()?;
    let header = control.cast::<MenuItem>()?.header()?;
    let header: &dyn AnyValue = &*header;
    header.downcast_ref::<String>().cloned()
}

fn headers_of(flyout: &MenuFlyout) -> Vec<Option<String>> {
    (0..flyout.items().view().count())
        .map(|index| header_of(&flyout.items().view().get_at(index)))
        .collect()
}

fn shown_text_box(text: &str) -> (Ref<TextBox>, Ref<crate::Window>) {
    let text_box = templated_text_box(Some(text));
    text_box.apply_template();
    let window = show_in_window(&text_box, MockWindowingPlatform::create_window_mock());
    (text_box, window)
}

#[test]
fn is_spell_check_enabled_is_true_by_default() {
    let _scope = test_scope();
    assert!(TextBox::new().is_spell_check_enabled());
}

#[test]
fn misspelled_words_are_underlined_after_the_check() {
    let _scope = test_scope();
    let checker = checker();
    let (text_box, _window) = shown_text_box("helo big wrold");

    run_jobs();

    assert_eq!(
        vec![SpellingError::new(0, 4), SpellingError::new(9, 5)],
        errors_of(&text_box)
    );
    // TextElement.Language (en-US outside an application) is the language of the check.
    assert_eq!(
        Some("en-US"),
        checker
            .languages
            .borrow()
            .last()
            .cloned()
            .flatten()
            .as_deref()
    );
}

#[test]
fn nothing_is_checked_without_a_spell_checker() {
    let _scope = test_scope();
    let (text_box, _window) = shown_text_box("helo");

    run_jobs();

    assert!(errors_of(&text_box).is_empty());
}

#[test]
fn disabling_spell_check_removes_the_underlines() {
    let _scope = test_scope();
    let checker = checker();
    let (text_box, _window) = shown_text_box("helo");
    run_jobs();
    assert_eq!(1, errors_of(&text_box).len());

    text_box.set_is_spell_check_enabled(false);
    text_box.set_text(Some("helo wrold"));
    run_jobs();

    assert!(errors_of(&text_box).is_empty());
    assert_eq!(1, checker.checks.get());
}

#[test]
fn password_and_read_only_boxes_are_not_checked() {
    let _scope = test_scope();
    let _checker = checker();
    let (text_box, _window) = shown_text_box("helo");
    run_jobs();
    assert_eq!(1, errors_of(&text_box).len());

    text_box.set_password_char('*');
    assert!(errors_of(&text_box).is_empty());
    text_box.set_password_char('\0');
    run_jobs();
    assert_eq!(1, errors_of(&text_box).len());

    text_box.set_is_read_only(true);
    assert!(errors_of(&text_box).is_empty());
}

#[test]
fn checks_of_several_changes_are_coalesced() {
    let _scope = test_scope();
    let checker = checker();
    let (text_box, _window) = shown_text_box("a");
    run_jobs();
    let checks = checker.checks.get();

    text_box.set_text(Some("helo"));
    text_box.set_text(Some("helo wrold"));
    text_box.set_text(Some("helo wrold!"));
    run_jobs();

    assert_eq!(checks + 1, checker.checks.get());
    assert_eq!(
        vec![SpellingError::new(0, 4), SpellingError::new(5, 5)],
        errors_of(&text_box)
    );
}

#[test]
fn the_word_being_typed_is_not_underlined() {
    let _scope = test_scope();
    let _checker = checker();
    let _focus = crate::test_support_buttons::focus_scope();
    let (text_box, _window) = shown_text_box("");
    text_box.focus();

    crate::text_box_tests::raise_text_event(&text_box, "helo");
    run_jobs();
    assert!(errors_of(&text_box).is_empty());

    crate::text_box_tests::raise_text_event(&text_box, " ");
    run_jobs();
    assert_eq!(vec![SpellingError::new(0, 4)], errors_of(&text_box));
}

#[test]
fn an_edit_keeps_the_underlines_it_does_not_touch() {
    let a = SpellingError::new(0, 4);
    let b = SpellingError::new(5, 5);
    let units = |text: &str| text.encode_utf16().collect::<Vec<_>>();
    // An insertion before the second word moves it.
    assert_eq!(
        vec![a, SpellingError::new(9, 5)],
        adjust_spelling_errors(&[a, b], &units("helo wrold"), &units("helo big wrold"))
    );
    // A letter removed from a word drops its underline until the next check, and moves the rest.
    assert_eq!(
        vec![SpellingError::new(4, 5)],
        adjust_spelling_errors(&[a, b], &units("helo wrold"), &units("hel wrold"))
    );
    assert_eq!(
        vec![a],
        adjust_spelling_errors(&[a, b], &units("helo wrold"), &units("helo wroldx"))
    );
    // A separator after a word keeps it; one removed between two words drops both.
    assert_eq!(
        vec![a, b],
        adjust_spelling_errors(&[a, b], &units("helo wrold"), &units("helo wrold."))
    );
    assert!(adjust_spelling_errors(&[a, b], &units("helo wrold"), &units("helowrold")).is_empty());
    // An unchanged text keeps every underline.
    assert_eq!(
        vec![a, b],
        adjust_spelling_errors(&[a, b], &units("helo wrold"), &units("helo wrold"))
    );
}

#[test]
fn the_context_flyout_has_the_proofing_items_of_the_word_at_the_caret() {
    let _scope = test_scope();
    let _checker = checker();
    let flyout = MenuFlyout::new();
    let cut = MenuItem::new();
    cut.set_header(Some(Rc::new("Cut".to_owned())));
    flyout.items().add(Some(Control::boxed(cut)));
    let (text_box, _window) = shown_text_box("helo wrold");
    text_box.set_context_flyout(&flyout);
    run_jobs();

    text_box.set_caret_index(2);
    text_box.raise_event(&ContextRequestedEventArgs::new());

    assert!(flyout.is_open());
    let headers = headers_of(&flyout);
    assert_eq!(
        vec![
            Some("hello".to_owned()),
            Some("help".to_owned()),
            Some("hero".to_owned()),
            None,
            Some("Add to dictionary".to_owned()),
            Some("Ignore".to_owned()),
            None,
            Some("Cut".to_owned()),
        ],
        headers
    );

    // A suggestion replaces the word.
    let item = flyout.items().view().get_at(0);
    let value: &dyn AnyValue = &**item.as_ref().unwrap();
    let item = value
        .downcast_ref::<Ref<Control>>()
        .unwrap()
        .cast::<MenuItem>()
        .unwrap();
    item.raise_event(&RoutedEventArgs::with_event(MenuItem::click_event()));
    assert_eq!(Some("hello wrold".to_owned()), text_box.text());

    flyout.hide();
    run_jobs();
    assert_eq!(vec![Some("Cut".to_owned())], headers_of(&flyout));
    assert_eq!(vec![SpellingError::new(6, 5)], errors_of(&text_box));
}

#[test]
fn ignore_and_add_to_dictionary_forget_the_word() {
    let _scope = test_scope();
    let checker = checker();
    let flyout = MenuFlyout::new();
    let (text_box, _window) = shown_text_box("helo wrold helo");
    text_box.set_context_flyout(&flyout);
    run_jobs();
    assert_eq!(3, errors_of(&text_box).len());

    let click = |header: &str| {
        let index = headers_of(&flyout)
            .iter()
            .position(|h| h.as_deref() == Some(header))
            .unwrap();
        let item = flyout.items().view().get_at(index);
        let value: &dyn AnyValue = &**item.as_ref().unwrap();
        let item = value
            .downcast_ref::<Ref<Control>>()
            .unwrap()
            .cast::<MenuItem>()
            .unwrap();
        item.raise_event(&RoutedEventArgs::with_event(MenuItem::click_event()));
    };

    text_box.set_caret_index(0);
    text_box.raise_event(&ContextRequestedEventArgs::new());
    click("Ignore");
    assert_eq!(vec!["helo".to_owned()], *checker.ignored.borrow());
    assert_eq!(vec![SpellingError::new(5, 5)], errors_of(&text_box));
    flyout.hide();
    run_jobs();

    text_box.set_caret_index(7);
    text_box.raise_event(&ContextRequestedEventArgs::new());
    click("Add to dictionary");
    flyout.hide();
    run_jobs();
    assert_eq!(vec!["wrold".to_owned()], *checker.added.borrow());
    assert!(errors_of(&text_box).is_empty());
}

#[test]
fn a_context_request_away_from_a_misspelled_word_adds_nothing() {
    let _scope = test_scope();
    let _checker = checker();
    let flyout = MenuFlyout::new();
    let (text_box, _window) = shown_text_box("helo is");
    text_box.set_context_flyout(&flyout);
    run_jobs();

    text_box.set_caret_index(6);
    text_box.raise_event(&ContextRequestedEventArgs::new());

    assert!(headers_of(&flyout).is_empty());
}

#[test]
fn the_underline_is_a_wavy_line_under_each_rectangle() {
    let _scope = test_scope();
    let rects = [
        Rect::new(10.0, 0.0, 20.0, 16.0),
        Rect::new(0.0, 16.0, 8.0, 16.0),
        Rect::new(4.0, 0.0, 0.0, 16.0),
    ];

    let geometry = spelling_error_geometry(&rects, Point::new(2.0, 3.0)).expect("a geometry");
    let bounds = geometry.bounds();

    // From the left of the second rectangle to the right of the first, in the last pixels of
    // each line.
    assert_eq!(2.0, bounds.x);
    assert_eq!(32.0, bounds.right());
    assert!(bounds.y >= 3.0 + 16.0 - 3.0 && bounds.bottom() <= 3.0 + 32.0);
    assert!(spelling_error_geometry(&rects[2..], Point::default()).is_none());
}
