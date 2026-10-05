use crate::browser_input_handler::BrowserInputHandler;
use crate::interop::{input_helper, JsObject};
use ferroui_base::input::text_input::{ITextInputMethodImpl, TextInputMethodClient, TextInputOptions, TextSelection};
use ferroui_base::reactive::IDisposable;
use ferroui_base::Rect;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

/// The calls the text input method makes into the page: the hidden input
/// element of the view and the element of the view itself. Replaced by a
/// recorder in the tests.
pub(crate) trait ITextInputPage {
    fn hide_input_element(&self);
    fn show_input_element(&self);
    fn focus_input_element(&self);
    fn focus_container_element(&self);
    fn clear_input_element(&self);
    fn set_surrounding_text(&self, text: &str, start: i32, end: i32);
    fn set_bounds(&self, x: i32, y: i32, width: i32, height: i32, caret: i32);
    fn selection_start(&self) -> i32;
    fn selection_end(&self) -> i32;
}

struct TextInputPage {
    input_element: JsObject,
    container_element: JsObject,
}

impl ITextInputPage for TextInputPage {
    fn hide_input_element(&self) {
        input_helper::hide_element(&self.input_element);
    }

    fn show_input_element(&self) {
        input_helper::show_element(&self.input_element);
    }

    fn focus_input_element(&self) {
        input_helper::focus_element(&self.input_element);
    }

    fn focus_container_element(&self) {
        input_helper::focus_element(&self.container_element);
    }

    fn clear_input_element(&self) {
        input_helper::clear_input_element(&self.input_element);
    }

    fn set_surrounding_text(&self, text: &str, start: i32, end: i32) {
        input_helper::set_surrounding_text(&self.input_element, text, start, end);
    }

    fn set_bounds(&self, x: i32, y: i32, width: i32, height: i32, caret: i32) {
        input_helper::set_bounds(&self.input_element, x, y, width, height, caret);
    }

    fn selection_start(&self) -> i32 {
        input_helper::get_selection_start(&self.input_element)
    }

    fn selection_end(&self) -> i32 {
        input_helper::get_selection_end(&self.input_element)
    }
}

/// The text input method of a view: the hidden input element of the view
/// stands in for the text the client edits, so that the input method of
/// the browser has an element to compose into.
pub struct BrowserTextInputMethod {
    weak_self: Weak<BrowserTextInputMethod>,
    page: Box<dyn ITextInputPage>,
    input_handler: Weak<BrowserInputHandler>,
    client: RefCell<Option<Rc<dyn TextInputMethodClient>>>,
    /// The subscriptions to the events of the current client.
    subscriptions: RefCell<Vec<Rc<dyn IDisposable>>>,
    is_composing: Cell<bool>,
    /// Set while the page reports that the keyboard focus left the view.
    focus_is_leaving_view: Cell<bool>,
}

impl BrowserTextInputMethod {
    /// Creates the text input method of the view in `container_element`,
    /// whose hidden input element is `input_element`.
    ///
    /// # Panics
    /// Panics when one of the elements is null or undefined.
    pub(crate) fn new(
        input_handler: Weak<BrowserInputHandler>,
        container_element: JsObject,
        input_element: JsObject,
    ) -> Rc<Self> {
        if input_element.is_null() || input_element.is_undefined() {
            panic!("Value cannot be null. (Parameter 'inputElement')");
        }
        if container_element.is_null() || container_element.is_undefined() {
            panic!("Value cannot be null. (Parameter 'containerElement')");
        }

        Self::with_page(input_handler, Box::new(TextInputPage { input_element, container_element }))
    }

    pub(crate) fn with_page(input_handler: Weak<BrowserInputHandler>, page: Box<dyn ITextInputPage>) -> Rc<Self> {
        Rc::new_cyclic(|weak_self| Self {
            weak_self: weak_self.clone(),
            page,
            input_handler,
            client: RefCell::new(None),
            subscriptions: RefCell::new(Vec::new()),
            is_composing: Cell::new(false),
            focus_is_leaving_view: Cell::new(false),
        })
    }

    /// Whether the input method of the browser is composing text.
    pub fn is_composing(&self) -> bool {
        self.is_composing.get()
    }

    fn client(&self) -> Option<Rc<dyn TextInputMethodClient>> {
        self.client.borrow().clone()
    }

    fn handler(&self, action: fn(&BrowserTextInputMethod)) -> Rc<dyn Fn()> {
        let this = self.weak_self.clone();
        Rc::new(move || {
            if let Some(this) = this.upgrade() {
                action(&this);
            }
        })
    }

    /// Runs `action` as the reaction to the keyboard focus of the page
    /// having left the view: a client that goes away meanwhile does not
    /// bring the focus back.
    pub(crate) fn while_focus_leaves_view(&self, action: impl FnOnce()) {
        let previous = self.focus_is_leaving_view.replace(true);
        action();
        self.focus_is_leaving_view.set(previous);
    }

    /// Gives the focus of the page to the hidden input element, unless the
    /// focus is just leaving the view.
    // Differs from the original, which focuses unconditionally: a client that updates its
    // cursor while it loses the focus would take the focus back from the element of the page
    // the user went to.
    fn focus_input_element(&self) {
        if !self.focus_is_leaving_view.get() {
            self.page.focus_input_element();
        }
    }

    fn hide_ime(&self) {
        self.page.hide_input_element();
        // Differs from the original, which always gives the focus back to the element of the
        // view. There the framework never learns that the page moved the focus elsewhere; here
        // it does, drops its focused element and with it the client, and focusing the view then
        // would take the focus back from the element of the page the user just went to.
        if !self.focus_is_leaving_view.get() {
            self.page.focus_container_element();
        }
    }

    fn input_pane_activation_requested(&self) {
        if self.client().is_some() {
            self.show_ime();
        }
    }

    fn show_ime(&self) {
        self.page.show_input_element();
        self.focus_input_element();
    }

    fn surrounding_text_changed(&self) {
        if let Some(client) = self.client() {
            let surrounding_text = client.surrounding_text();
            let selection = client.selection();

            self.page.set_surrounding_text(&surrounding_text, selection.start, selection.end);
        }
    }

    /// The hidden input element is about to change: `input_type` is the
    /// kind of the change and `start` and `end` the range it targets.
    pub fn on_before_input(&self, input_type: &str, mut start: i32, mut end: i32) {
        if input_type != "deleteByComposition" {
            if input_type == "deleteContentBackward" {
                start = self.page.selection_start();
                end = self.page.selection_end();
            } else {
                start = -1;
                end = -1;
            }
        }

        if start != -1 && end != -1 {
            if let Some(client) = self.client() {
                client.set_selection(TextSelection::new(start, end));
            }
        }
    }

    /// The input method of the browser started composing.
    pub fn on_composition_start(&self) {
        let Some(client) = self.client() else {
            return;
        };

        client.set_preedit_text(None);
        self.is_composing.set(true);
    }

    /// The text being composed changed.
    pub fn on_composition_update(&self, data: Option<&str>) {
        let Some(client) = self.client() else {
            return;
        };

        client.set_preedit_text(data);
    }

    /// The input method of the browser finished composing; `data` is the
    /// text it committed.
    pub fn on_composition_end(&self, data: Option<&str>) {
        let Some(client) = self.client() else {
            return;
        };

        self.is_composing.set(false);

        client.set_preedit_text(None);

        if let Some(data) = data {
            if let Some(input_handler) = self.input_handler.upgrade() {
                input_handler.raw_text_event(data);
            }
        }
    }
}

impl ITextInputMethodImpl for BrowserTextInputMethod {
    fn set_client(&self, client: Option<Rc<dyn TextInputMethodClient>>) {
        if self.client().is_some() {
            let subscriptions = std::mem::take(&mut *self.subscriptions.borrow_mut());
            for subscription in subscriptions {
                subscription.dispose();
            }
        }

        if let Some(client) = &client {
            let subscriptions = vec![
                client.surrounding_text_changed(self.handler(Self::surrounding_text_changed)),
                client.input_pane_activation_requested(self.handler(Self::input_pane_activation_requested)),
            ];
            *self.subscriptions.borrow_mut() = subscriptions;
        }

        self.page.clear_input_element();

        let old_client = self.client.replace(client);
        drop(old_client);

        if let Some(client) = self.client() {
            self.show_ime();

            let surrounding_text = client.surrounding_text();
            let selection = client.selection();

            self.page.set_surrounding_text(&surrounding_text, selection.start, selection.end);
        } else {
            self.hide_ime();
        }
    }

    fn set_cursor_rect(&self, rect: Rect) {
        self.focus_input_element();
        let caret = self.client().map_or(0, |client| client.selection().end);
        self.page.set_bounds(rect.x as i32, rect.y as i32, rect.width as i32, rect.height as i32, caret);
        self.focus_input_element();
    }

    fn set_options(&self, _options: &TextInputOptions) {}

    fn reset(&self) {
        self.page.clear_input_element();
        self.page.set_surrounding_text("", 0, 0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferroui_base::input::text_input::TextInputMethodClientEvents;
    use ferroui_base::{Ref, Visual};

    type Calls = Rc<RefCell<Vec<String>>>;

    struct RecordingPage {
        calls: Calls,
        selection: Rc<Cell<(i32, i32)>>,
    }

    impl RecordingPage {
        fn log(&self, call: impl Into<String>) {
            self.calls.borrow_mut().push(call.into());
        }
    }

    impl ITextInputPage for RecordingPage {
        fn hide_input_element(&self) {
            self.log("hide");
        }

        fn show_input_element(&self) {
            self.log("show");
        }

        fn focus_input_element(&self) {
            self.log("focus input");
        }

        fn focus_container_element(&self) {
            self.log("focus container");
        }

        fn clear_input_element(&self) {
            self.log("clear");
        }

        fn set_surrounding_text(&self, text: &str, start: i32, end: i32) {
            self.log(format!("text '{text}' {start} {end}"));
        }

        fn set_bounds(&self, x: i32, y: i32, width: i32, height: i32, caret: i32) {
            self.log(format!("bounds {x} {y} {width} {height} caret {caret}"));
        }

        fn selection_start(&self) -> i32 {
            self.selection.get().0
        }

        fn selection_end(&self) -> i32 {
            self.selection.get().1
        }
    }

    struct TestClient {
        events: TextInputMethodClientEvents,
        text: RefCell<String>,
        selection: Cell<TextSelection>,
        preedit: RefCell<Vec<Option<String>>>,
    }

    impl TestClient {
        fn new(text: &str, start: i32, end: i32) -> Rc<Self> {
            Rc::new(Self {
                events: TextInputMethodClientEvents::new(),
                text: RefCell::new(text.to_string()),
                selection: Cell::new(TextSelection::new(start, end)),
                preedit: RefCell::new(Vec::new()),
            })
        }
    }

    impl TextInputMethodClient for TestClient {
        fn events(&self) -> &TextInputMethodClientEvents {
            &self.events
        }

        fn text_view_visual(&self) -> Ref<Visual> {
            unreachable!("the text input method of the browser does not ask for the visual")
        }

        fn supports_preedit(&self) -> bool {
            true
        }

        fn supports_surrounding_text(&self) -> bool {
            true
        }

        fn surrounding_text(&self) -> String {
            self.text.borrow().clone()
        }

        fn cursor_rectangle(&self) -> Rect {
            Rect::default()
        }

        fn selection(&self) -> TextSelection {
            self.selection.get()
        }

        fn set_selection(&self, value: TextSelection) {
            self.selection.set(value);
        }

        fn set_preedit_text(&self, preedit_text: Option<&str>) {
            self.preedit.borrow_mut().push(preedit_text.map(str::to_string));
        }
    }

    struct Fixture {
        method: Rc<BrowserTextInputMethod>,
        calls: Calls,
        selection: Rc<Cell<(i32, i32)>>,
    }

    impl Fixture {
        fn new() -> Self {
            let calls: Calls = Rc::new(RefCell::new(Vec::new()));
            let selection = Rc::new(Cell::new((0, 0)));
            let page = RecordingPage { calls: calls.clone(), selection: selection.clone() };
            let method = BrowserTextInputMethod::with_page(Weak::new(), Box::new(page));
            Self { method, calls, selection }
        }

        fn take_calls(&self) -> Vec<String> {
            std::mem::take(&mut *self.calls.borrow_mut())
        }

        fn set_client(&self, client: &Rc<TestClient>) {
            let client: Rc<dyn TextInputMethodClient> = client.clone();
            self.method.set_client(Some(client));
        }
    }

    #[test]
    fn a_client_shows_the_input_element_and_mirrors_its_text() {
        let fixture = Fixture::new();
        let client = TestClient::new("hello", 1, 3);

        fixture.set_client(&client);

        assert_eq!(vec!["clear", "show", "focus input", "text 'hello' 1 3"], fixture.take_calls());
    }

    #[test]
    fn without_a_client_the_input_element_is_hidden_and_the_view_focused() {
        let fixture = Fixture::new();
        fixture.set_client(&TestClient::new("hello", 1, 3));
        fixture.take_calls();

        fixture.method.set_client(None);

        assert_eq!(vec!["clear", "hide", "focus container"], fixture.take_calls());
    }

    #[test]
    fn a_client_that_goes_away_because_the_focus_left_the_view_does_not_take_the_focus_back() {
        let fixture = Fixture::new();
        fixture.set_client(&TestClient::new("hello", 1, 3));
        fixture.take_calls();

        fixture.method.while_focus_leaves_view(|| {
            // The client moves its cursor as it loses the focus, and then goes away.
            fixture.method.set_cursor_rect(Rect::new(1.0, 2.0, 3.0, 4.0));
            fixture.method.set_client(None);
        });
        let calls = fixture.take_calls();
        assert!(!calls.iter().any(|call| call.starts_with("focus")), "{calls:?}");
        assert_eq!(Some(&"hide".to_string()), calls.last());

        // Afterwards the usual behaviour is back.
        fixture.set_client(&TestClient::new("hello", 1, 3));
        fixture.take_calls();
        fixture.method.set_client(None);
        assert_eq!(vec!["clear", "hide", "focus container"], fixture.take_calls());
    }

    #[test]
    fn a_change_of_the_surrounding_text_of_the_client_is_mirrored() {
        let fixture = Fixture::new();
        let client = TestClient::new("hello", 1, 3);
        fixture.set_client(&client);
        fixture.take_calls();

        *client.text.borrow_mut() = "hello world".to_string();
        client.selection.set(TextSelection::new(11, 11));
        client.raise_surrounding_text_changed();

        assert_eq!(vec!["text 'hello world' 11 11"], fixture.take_calls());
    }

    #[test]
    fn a_request_for_the_input_pane_shows_the_input_element_again() {
        let fixture = Fixture::new();
        let client = TestClient::new("", 0, 0);
        fixture.set_client(&client);
        fixture.take_calls();

        client.raise_input_pane_activation_requested();

        assert_eq!(vec!["show", "focus input"], fixture.take_calls());
    }

    #[test]
    fn the_events_of_a_replaced_client_are_no_longer_followed() {
        let fixture = Fixture::new();
        let first = TestClient::new("first", 0, 0);
        let second = TestClient::new("second", 2, 2);
        fixture.set_client(&first);
        fixture.set_client(&second);
        fixture.take_calls();

        first.raise_surrounding_text_changed();
        first.raise_input_pane_activation_requested();
        assert!(fixture.take_calls().is_empty());

        second.raise_surrounding_text_changed();
        assert_eq!(vec!["text 'second' 2 2"], fixture.take_calls());

        fixture.method.set_client(None);
        fixture.take_calls();
        second.raise_surrounding_text_changed();
        second.raise_input_pane_activation_requested();
        assert!(fixture.take_calls().is_empty());
    }

    #[test]
    fn the_cursor_rectangle_moves_the_input_element_to_the_caret_of_the_client() {
        let fixture = Fixture::new();

        fixture.method.set_cursor_rect(Rect::new(10.9, 20.2, 1.0, 16.7));
        assert_eq!(vec!["focus input", "bounds 10 20 1 16 caret 0", "focus input"], fixture.take_calls());

        fixture.set_client(&TestClient::new("hello", 1, 3));
        fixture.take_calls();
        fixture.method.set_cursor_rect(Rect::new(10.9, 20.2, 1.0, 16.7));
        assert_eq!(vec!["focus input", "bounds 10 20 1 16 caret 3", "focus input"], fixture.take_calls());
    }

    #[test]
    fn reset_empties_the_input_element() {
        let fixture = Fixture::new();

        fixture.method.reset();

        assert_eq!(vec!["clear", "text '' 0 0"], fixture.take_calls());
    }

    #[test]
    fn a_deletion_by_composition_selects_the_range_it_targets() {
        let fixture = Fixture::new();
        let client = TestClient::new("hello", 5, 5);
        fixture.set_client(&client);

        fixture.method.on_before_input("deleteByComposition", 2, 4);

        assert_eq!(TextSelection::new(2, 4), client.selection());
    }

    #[test]
    fn a_deletion_by_composition_without_a_range_leaves_the_selection() {
        let fixture = Fixture::new();
        let client = TestClient::new("hello", 5, 5);
        fixture.set_client(&client);

        fixture.method.on_before_input("deleteByComposition", -1, -1);
        fixture.method.on_before_input("deleteByComposition", 2, -1);

        assert_eq!(TextSelection::new(5, 5), client.selection());
    }

    #[test]
    fn a_backward_deletion_selects_the_selection_of_the_input_element() {
        let fixture = Fixture::new();
        let client = TestClient::new("hello", 5, 5);
        fixture.set_client(&client);
        fixture.selection.set((3, 4));

        fixture.method.on_before_input("deleteContentBackward", 0, 1);

        assert_eq!(TextSelection::new(3, 4), client.selection());
    }

    #[test]
    fn other_changes_of_the_input_element_leave_the_selection() {
        let fixture = Fixture::new();
        let client = TestClient::new("hello", 5, 5);
        fixture.set_client(&client);
        fixture.selection.set((3, 4));

        fixture.method.on_before_input("insertText", 0, 1);
        fixture.method.on_before_input("insertCompositionText", 2, 4);

        assert_eq!(TextSelection::new(5, 5), client.selection());
    }

    #[test]
    fn a_change_of_the_input_element_without_a_client_is_ignored() {
        let fixture = Fixture::new();

        fixture.method.on_before_input("deleteByComposition", 2, 4);
        fixture.method.on_before_input("deleteContentBackward", 0, 1);

        assert!(fixture.take_calls().is_empty());
    }

    #[test]
    fn a_composition_sets_the_preedit_text_and_clears_it_at_its_end() {
        let fixture = Fixture::new();
        let client = TestClient::new("", 0, 0);
        fixture.set_client(&client);
        assert!(!fixture.method.is_composing());

        fixture.method.on_composition_start();
        assert!(fixture.method.is_composing());

        fixture.method.on_composition_update(Some("k"));
        fixture.method.on_composition_update(Some("ka"));
        fixture.method.on_composition_update(None);
        assert!(fixture.method.is_composing());

        fixture.method.on_composition_end(Some("\u{304b}"));
        assert!(!fixture.method.is_composing());

        assert_eq!(
            vec![None, Some("k".to_string()), Some("ka".to_string()), None, None],
            *client.preedit.borrow()
        );
    }

    #[test]
    fn a_composition_without_a_client_changes_nothing() {
        let fixture = Fixture::new();

        fixture.method.on_composition_start();
        assert!(!fixture.method.is_composing());

        fixture.method.on_composition_update(Some("k"));
        fixture.method.on_composition_end(Some("k"));

        assert!(!fixture.method.is_composing());
        assert!(fixture.take_calls().is_empty());
    }

    #[test]
    fn a_composition_that_loses_its_client_stays_composing_until_a_client_ends_one() {
        // As in the original: the end of a composition is only seen with a client.
        let fixture = Fixture::new();
        let client = TestClient::new("", 0, 0);
        fixture.set_client(&client);
        fixture.method.on_composition_start();

        fixture.method.set_client(None);
        fixture.method.on_composition_end(None);
        assert!(fixture.method.is_composing());

        fixture.set_client(&client);
        fixture.method.on_composition_end(None);
        assert!(!fixture.method.is_composing());
    }
}
