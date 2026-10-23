//! The input method of a top-level: connects the client of the framework
//! (a text box that has the focus) with the input method of the system.

use super::ferro_input_connection::FerroInputConnection;
use super::text_edit_buffer::{ExtractedText, IAndroidInputMethod, KeyEventToDispatch};
use crate::i_init_editor_info::{EditorInfo, IInitEditorInfo, InitEditorInfo};
use ferroui_base::input::text_input::{
    ITextInputMethodImpl, TextInputContentType, TextInputMethodClient, TextInputOptions, TextInputReturnKeyType,
    TextSelection,
};
use ferroui_base::input::InputScopeNameValue;
use ferroui_base::reactive::IDisposable;
use ferroui_base::Rect;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

/// `android.text.InputType`.
pub(crate) mod input_types {
    pub const MASK_CLASS: i32 = 0x0000_000f;
    pub const MASK_VARIATION: i32 = 0x0000_0ff0;
    pub const CLASS_TEXT: i32 = 0x0000_0001;
    pub const CLASS_NUMBER: i32 = 0x0000_0002;
    pub const CLASS_PHONE: i32 = 0x0000_0003;
    pub const TEXT_FLAG_CAP_SENTENCES: i32 = 0x0000_4000;
    pub const TEXT_FLAG_MULTI_LINE: i32 = 0x0002_0000;
    pub const TEXT_FLAG_NO_SUGGESTIONS: i32 = 0x0008_0000;
    pub const TEXT_VARIATION_NORMAL: i32 = 0x0000_0000;
    pub const TEXT_VARIATION_URI: i32 = 0x0000_0010;
    pub const TEXT_VARIATION_EMAIL_ADDRESS: i32 = 0x0000_0020;
    pub const TEXT_VARIATION_PERSON_NAME: i32 = 0x0000_0060;
    pub const TEXT_VARIATION_PASSWORD: i32 = 0x0000_0080;
    pub const TEXT_VARIATION_VISIBLE_PASSWORD: i32 = 0x0000_0090;
    pub const NUMBER_FLAG_SIGNED: i32 = 0x0000_1000;
    pub const NUMBER_FLAG_DECIMAL: i32 = 0x0000_2000;
    pub const NUMBER_VARIATION_PASSWORD: i32 = 0x0000_0010;
}

/// `EditorInfo.IME_FLAG_*`.
pub(crate) mod ime_flags {
    pub const NO_PERSONALIZED_LEARNING: i32 = 0x0100_0000;
    pub const NO_FULLSCREEN: i32 = 0x0200_0000;
    pub const NO_EXTRACT_UI: i32 = 0x1000_0000;
    pub const NO_ENTER_ACTION: i32 = 0x4000_0000;
}

/// `TextUtils.CAP_MODE_SENTENCES`.
pub(crate) const CAPITALIZATION_MODE_SENTENCES: i32 = 0x0000_4000;

/// The actions of the enter key, as flags of the editor
/// (`EditorInfo.IME_ACTION_*`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(i32)]
pub(crate) enum CustomImeFlags {
    ActionNone = 0x0000_0001,
    ActionGo = 0x0000_0002,
    ActionSearch = 0x0000_0003,
    ActionSend = 0x0000_0004,
    ActionNext = 0x0000_0005,
    ActionDone = 0x0000_0006,
    ActionPrevious = 0x0000_0007,
}

/// The view of the input method (`TView` of the reference: a view that
/// takes the description of its editor) and the input method manager of
/// its context, as far as the input method calls them.
pub(crate) trait IInputMethodHost: IInitEditorInfo {
    /// `Focusable = true` and `FocusableInTouchMode = true`.
    fn set_focusable(&self);
    /// `View.RequestFocus`.
    fn request_focus(&self);
    /// `InputMethodManager.RestartInput(view)`.
    fn restart_input(&self);
    /// `InputMethodManager.ShowSoftInput(view, ShowFlags.Implicit)`.
    fn show_soft_input(&self);
    /// `InputMethodManager.HideSoftInputFromWindow(view.WindowToken, HideSoftInputFlags.ImplicitOnly)`.
    fn hide_soft_input(&self);
    /// `InputMethodManager.UpdateSelection(view, ...)`.
    fn update_selection(&self, sel_start: i32, sel_end: i32, candidates_start: i32, candidates_end: i32);
    /// `InputMethodManager.UpdateExtractedText(view, token, text)`.
    fn update_extracted_text(&self, token: i32, text: &ExtractedText);
    /// `View.DispatchKeyEvent`.
    fn dispatch_key_event(&self, key_event: &KeyEventToDispatch);
}

pub(crate) struct AndroidInputMethod {
    this: Weak<AndroidInputMethod>,
    host: Rc<dyn IInputMethodHost>,
    api_level: i32,
    client: RefCell<Option<Rc<dyn TextInputMethodClient>>>,
    client_generation: Cell<u64>,
    input_connection: RefCell<Option<Rc<FerroInputConnection>>>,
    input_connection_generation: Cell<u64>,
    /// Only an options refresh may carry the current composing region into
    /// the next native connection. A client assignment cancels this handoff.
    options_restart_connection: RefCell<Option<Weak<FerroInputConnection>>>,
    /// The three subscriptions to the events of the client.
    client_events: RefCell<Vec<Rc<dyn IDisposable>>>,
}

impl AndroidInputMethod {
    pub fn new(host: Rc<dyn IInputMethodHost>, api_level: i32) -> Rc<AndroidInputMethod> {
        host.set_focusable();

        Rc::new_cyclic(|this| AndroidInputMethod {
            this: this.clone(),
            host,
            api_level,
            client: RefCell::new(None),
            client_generation: Cell::new(0),
            input_connection: RefCell::new(None),
            input_connection_generation: Cell::new(0),
            options_restart_connection: RefCell::new(None),
            client_events: RefCell::new(Vec::new()),
        })
    }

    fn input_connection(&self) -> Option<Rc<FerroInputConnection>> {
        self.input_connection.borrow().clone()
    }

    fn client_input_pane_activation_requested(&self) {
        if self.is_active() {
            self.host.show_soft_input();
        }
    }

    fn client_selection_changed(&self) {
        match self.input_connection() {
            Some(input_connection) if !input_connection.is_in_batch_edit() && !input_connection.is_in_update() => {}
            _ => return,
        }
        self.on_selection_changed();
    }

    fn on_selection_changed(&self) {
        let (Some(client), Some(input_connection)) = (self.client(), self.input_connection()) else {
            return;
        };
        if input_connection.is_in_update() {
            return;
        }

        self.on_surrounding_text_changed();

        input_connection.set_is_in_update(true);

        let selection = client.selection();

        let composition = match input_connection.edit_buffer().composition() {
            Some(composition) if input_connection.edit_buffer().has_composition() => composition,
            _ => TextSelection::new(-1, -1),
        };

        self.host.update_selection(selection.start, selection.end, composition.start, composition.end);

        input_connection.set_is_in_update(false);
    }

    fn client_surrounding_text_changed(&self) {
        match self.input_connection() {
            Some(input_connection) if !input_connection.is_in_batch_edit() && !input_connection.is_in_update() => {}
            _ => return,
        }
        self.on_surrounding_text_changed();
    }

    fn on_surrounding_text_changed(&self) {
        if let Some(input_connection) = self.input_connection() {
            input_connection.update_state();
        }
    }

    /// Fills the description of the editor from the options of the text
    /// input, as the function the reference gives its view does.
    pub(crate) fn fill_editor_info(options: &TextInputOptions, out_attrs: &mut EditorInfo, api_level: i32) {
        use input_types::*;

        let scope = options.effective_input_scope();
        out_attrs.input_type = if let Some(scope) = scope {
            input_type_for_scope(scope)
        } else {
            match options.content_type {
                TextInputContentType::Email => CLASS_TEXT | TEXT_VARIATION_EMAIL_ADDRESS,
                TextInputContentType::Number => CLASS_NUMBER | NUMBER_FLAG_DECIMAL | NUMBER_FLAG_SIGNED,
                TextInputContentType::Password => CLASS_TEXT | TEXT_VARIATION_PASSWORD,
                TextInputContentType::Pin => CLASS_NUMBER | NUMBER_VARIATION_PASSWORD,
                TextInputContentType::Digits => CLASS_PHONE,
                TextInputContentType::Url => CLASS_TEXT | TEXT_VARIATION_URI,
                TextInputContentType::Name => CLASS_TEXT | TEXT_VARIATION_PERSON_NAME,
                // Alpha/Normal/Social/Search have no dedicated Android keyboard
                _ => CLASS_TEXT,
            }
        };

        // InputType flags are specific to their class. Preserve the existing
        // legacy conversion, but do not add text flags to scoped numeric pads.
        let accepts_text_flags = scope.is_none() || (out_attrs.input_type & MASK_CLASS) == CLASS_TEXT;
        if options.auto_capitalization && accepts_text_flags {
            out_attrs.initial_caps_mode = CAPITALIZATION_MODE_SENTENCES;
            out_attrs.input_type |= TEXT_FLAG_CAP_SENTENCES;
        }

        if options.multiline && accepts_text_flags {
            out_attrs.input_type |= TEXT_FLAG_MULTI_LINE;
        }

        out_attrs.ime_options = match options.return_key_type {
            TextInputReturnKeyType::Return => ime_flags::NO_ENTER_ACTION,
            TextInputReturnKeyType::Go => CustomImeFlags::ActionGo as i32,
            TextInputReturnKeyType::Send => CustomImeFlags::ActionSend as i32,
            TextInputReturnKeyType::Search => CustomImeFlags::ActionSearch as i32,
            TextInputReturnKeyType::Next => CustomImeFlags::ActionNext as i32,
            TextInputReturnKeyType::Previous => CustomImeFlags::ActionPrevious as i32,
            TextInputReturnKeyType::Done => CustomImeFlags::ActionDone as i32,
            TextInputReturnKeyType::Default => {
                if options.multiline {
                    ime_flags::NO_ENTER_ACTION
                } else {
                    CustomImeFlags::ActionDone as i32
                }
            }
        };

        out_attrs.ime_options |= ime_flags::NO_FULLSCREEN | ime_flags::NO_EXTRACT_UI;

        if options.show_suggestions == Some(false) && (out_attrs.input_type & MASK_CLASS) == CLASS_TEXT {
            out_attrs.input_type |= TEXT_FLAG_NO_SUGGESTIONS;

            if (out_attrs.input_type & MASK_VARIATION) == TEXT_VARIATION_NORMAL {
                out_attrs.input_type |= TEXT_VARIATION_VISIBLE_PASSWORD;
            }
        }

        let scope_is_secret = matches!(
            scope,
            Some(
                InputScopeNameValue::Password
                    | InputScopeNameValue::NumericPassword
                    | InputScopeNameValue::NumericPin
                    | InputScopeNameValue::AlphanumericPin
            )
        );
        let legacy_is_secret =
            matches!(options.content_type, TextInputContentType::Password | TextInputContentType::Pin);
        if options.is_sensitive || scope_is_secret || legacy_is_secret {
            if (scope.is_none() && options.is_sensitive) || (out_attrs.input_type & MASK_CLASS) == CLASS_TEXT {
                out_attrs.input_type |= TEXT_FLAG_NO_SUGGESTIONS;
            }

            if api_level >= 26 {
                out_attrs.ime_options |= ime_flags::NO_PERSONALIZED_LEARNING;
            }
        }

        if let Some(locale_hints) = options.locale_hints.as_ref().filter(|hints| !hints.is_empty()) {
            out_attrs.hint_locales = Some(locale_hints.to_vec());
        }
    }
}

/// The native view owns rendering and validation; these EditorInfo values only
/// advise the IME. Android has no script/width-specific input type. Non-numeric
/// script/width scopes use normal text; full-width numbers use the numeric pad.
fn input_type_for_scope(scope: InputScopeNameValue) -> i32 {
    use input_types::*;
    use InputScopeNameValue as Scope;
    match scope {
        Scope::Url => CLASS_TEXT | TEXT_VARIATION_URI,
        Scope::EmailSmtpAddress | Scope::EmailNameOrAddress => CLASS_TEXT | TEXT_VARIATION_EMAIL_ADDRESS,
        Scope::PersonalFullName | Scope::NameOrPhoneNumber => CLASS_TEXT | TEXT_VARIATION_PERSON_NAME,
        Scope::CurrencyAmountAndSymbol
        | Scope::CurrencyAmount
        | Scope::Number
        | Scope::NumberFullWidth
        | Scope::FormulaNumber => CLASS_NUMBER | NUMBER_FLAG_DECIMAL | NUMBER_FLAG_SIGNED,
        Scope::DateMonthNumber
        | Scope::DateDayNumber
        | Scope::DateYear
        | Scope::Digits
        | Scope::TimeHour
        | Scope::TimeMinutesOrSeconds => CLASS_NUMBER,
        Scope::TelephoneNumber
        | Scope::TelephoneCountryCode
        | Scope::TelephoneAreaCode
        | Scope::TelephoneLocalNumber => CLASS_PHONE,
        Scope::Password | Scope::AlphanumericPin => CLASS_TEXT | TEXT_VARIATION_PASSWORD,
        Scope::NumericPassword | Scope::NumericPin => CLASS_NUMBER | NUMBER_VARIATION_PASSWORD,
        Scope::Default
        | Scope::AlphanumericHalfWidth
        | Scope::AlphanumericFullWidth
        | Scope::Hiragana
        | Scope::KatakanaHalfWidth
        | Scope::KatakanaFullWidth
        | Scope::Hanja
        | Scope::HangulHalfWidth
        | Scope::HangulFullWidth
        | Scope::Search
        | Scope::Formula
        | Scope::SearchIncremental
        | Scope::ChineseHalfWidth
        | Scope::ChineseFullWidth
        | Scope::NativeScript
        | Scope::Text
        | Scope::Chat
        | Scope::Maps
        | Scope::ChatWithoutEmoji => CLASS_TEXT,
    }
}

impl IAndroidInputMethod for AndroidInputMethod {
    fn client(&self) -> Option<Rc<dyn TextInputMethodClient>> {
        self.client.borrow().clone()
    }

    fn on_batch_edit_ended(&self) {
        match self.input_connection() {
            Some(input_connection) if !input_connection.is_in_batch_edit() => {}
            _ => return,
        }
        self.on_selection_changed();
    }

    fn dispatch_key_event(&self, key_event: &KeyEventToDispatch) {
        self.host.dispatch_key_event(key_event);
    }

    fn hide_soft_input(&self) {
        self.host.hide_soft_input();
    }

    fn update_selection(&self, sel_start: i32, sel_end: i32, candidates_start: i32, candidates_end: i32) {
        self.host.update_selection(sel_start, sel_end, candidates_start, candidates_end);
    }

    fn update_extracted_text(&self, token: i32, text: &ExtractedText) {
        self.host.update_extracted_text(token, text);
    }
}

impl ITextInputMethodImpl for AndroidInputMethod {
    fn reset(&self) {}

    fn set_client(&self, client: Option<Rc<dyn TextInputMethodClient>>) {
        self.client_generation.set(self.client_generation.get().wrapping_add(1));
        self.options_restart_connection.borrow_mut().take();
        let subscriptions = std::mem::take(&mut *self.client_events.borrow_mut());
        for subscription in subscriptions {
            subscription.dispose();
        }

        *self.client.borrow_mut() = client;

        let client = self.client();
        if let Some(client) = client {
            self.host.request_focus();

            self.host.restart_input();

            self.host.show_soft_input();

            if let Some(input_connection) = self.input_connection() {
                input_connection.update_state();
            }

            let handler = |call: fn(&AndroidInputMethod)| -> Rc<dyn Fn()> {
                let this = self.this.clone();
                Rc::new(move || {
                    if let Some(this) = this.upgrade() {
                        call(&this);
                    }
                })
            };
            *self.client_events.borrow_mut() = vec![
                client.surrounding_text_changed(handler(Self::client_surrounding_text_changed)),
                client.selection_changed(handler(Self::client_selection_changed)),
                client.input_pane_activation_requested(handler(Self::client_input_pane_activation_requested)),
            ];
        } else {
            self.host.restart_input();
            *self.input_connection.borrow_mut() = None;
            self.host.hide_soft_input();
        }
    }

    fn set_cursor_rect(&self, _rect: Rect) {}

    fn set_options(&self, options: &TextInputOptions) {
        *self.options_restart_connection.borrow_mut() =
            if self.input_connection_generation.get() == self.client_generation.get() {
                self.input_connection().as_ref().map(Rc::downgrade)
            } else {
                None
            };
        let this = self.this.clone();
        let options = options.clone();
        let init: InitEditorInfo = Rc::new(move |top_level, out_attrs: &mut EditorInfo| {
            let this = this.upgrade()?;
            let _client = this.client()?;
            let generation = this.client_generation.get();
            let previous = this.options_restart_connection.borrow_mut().take().and_then(|weak| weak.upgrade());
            let composition = previous.and_then(|previous| {
                this.input_connection()
                    .filter(|current| Rc::ptr_eq(current, &previous))
                    .and_then(|current| current.edit_buffer().composition())
            });

            let input_method: Weak<dyn IAndroidInputMethod> = this.this.clone();
            let input_connection = FerroInputConnection::new(Rc::downgrade(top_level), input_method);
            if let Some(composition) = composition {
                input_connection.edit_buffer().set_composition(Some(composition));
                // Clamping the region reads surrounding text from the client,
                // which can synchronously replace focus in application code.
                if this.client_generation.get() != generation {
                    return None;
                }
            }
            *this.input_connection.borrow_mut() = Some(input_connection.clone());
            this.input_connection_generation.set(generation);

            AndroidInputMethod::fill_editor_info(&options, out_attrs, this.api_level);

            Some(input_connection)
        });
        self.host.init_editor_info(init);
        // A live Names/NameValue edit must replace EditorInfo on the current
        // connection; installing the callback alone only affected future focus.
        if self.is_active() {
            self.host.restart_input();
        }
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests of the input method.
    use super::super::text_edit_buffer::test_support::{TestClient, TestInputMethod};
    use super::super::text_edit_buffer::IInputConnectionTopLevel;
    use super::*;

    #[derive(Clone, Debug, PartialEq, Eq)]
    enum HostCall {
        SetFocusable,
        RequestFocus,
        RestartInput,
        ShowSoftInput,
        HideSoftInput,
        UpdateSelection(i32, i32, i32, i32),
        UpdateExtractedText(i32),
        DispatchKeyEvent,
    }

    #[derive(Default)]
    struct Host {
        calls: RefCell<Vec<HostCall>>,
        init: RefCell<Option<InitEditorInfo>>,
    }

    impl Host {
        fn take_calls(&self) -> Vec<HostCall> {
            std::mem::take(&mut *self.calls.borrow_mut())
        }

        /// What the view does when the system asks for an input connection.
        fn create_input_connection(&self) -> Option<(Rc<FerroInputConnection>, EditorInfo)> {
            // The top-level of the tests of the buffer: it only records.
            let top_level: Rc<dyn IInputConnectionTopLevel> = TestInputMethod::new(None);
            TOP_LEVELS.with(|top_levels| top_levels.borrow_mut().push(top_level.clone()));
            let init = self.init.borrow().clone()?;
            let mut out_attrs = EditorInfo::default();
            init(&top_level, &mut out_attrs).map(|connection| (connection, out_attrs))
        }
    }

    ferroui_base::ferro_thread_local! {
        // The connection holds its top-level weakly: the tests keep them.
        static TOP_LEVELS: RefCell<Vec<Rc<dyn IInputConnectionTopLevel>>> = const { RefCell::new(Vec::new()) };
    }

    impl IInitEditorInfo for Host {
        fn init_editor_info(&self, init: InitEditorInfo) {
            *self.init.borrow_mut() = Some(init);
        }
    }

    impl IInputMethodHost for Host {
        fn set_focusable(&self) {
            self.calls.borrow_mut().push(HostCall::SetFocusable);
        }

        fn request_focus(&self) {
            self.calls.borrow_mut().push(HostCall::RequestFocus);
        }

        fn restart_input(&self) {
            self.calls.borrow_mut().push(HostCall::RestartInput);
        }

        fn show_soft_input(&self) {
            self.calls.borrow_mut().push(HostCall::ShowSoftInput);
        }

        fn hide_soft_input(&self) {
            self.calls.borrow_mut().push(HostCall::HideSoftInput);
        }

        fn update_selection(&self, sel_start: i32, sel_end: i32, candidates_start: i32, candidates_end: i32) {
            self.calls.borrow_mut().push(HostCall::UpdateSelection(
                sel_start,
                sel_end,
                candidates_start,
                candidates_end,
            ));
        }

        fn update_extracted_text(&self, token: i32, _text: &ExtractedText) {
            self.calls.borrow_mut().push(HostCall::UpdateExtractedText(token));
        }

        fn dispatch_key_event(&self, _key_event: &KeyEventToDispatch) {
            self.calls.borrow_mut().push(HostCall::DispatchKeyEvent);
        }
    }

    fn input_method() -> (Rc<Host>, Rc<AndroidInputMethod>) {
        let host = Rc::new(Host::default());
        let input_method = AndroidInputMethod::new(host.clone(), 36);
        assert_eq!(host.take_calls(), vec![HostCall::SetFocusable]);
        (host, input_method)
    }

    fn info(options: &TextInputOptions) -> EditorInfo {
        let mut out_attrs = EditorInfo::default();
        AndroidInputMethod::fill_editor_info(options, &mut out_attrs, 36);
        out_attrs
    }

    #[test]
    fn a_client_takes_the_focus_restarts_the_input_and_shows_the_keyboard() {
        let (host, input_method) = input_method();
        let client = TestClient::new("abc", 1, 1);
        input_method.set_client(Some(client.clone()));
        assert!(input_method.is_active());
        assert_eq!(host.take_calls(), vec![HostCall::RequestFocus, HostCall::RestartInput, HostCall::ShowSoftInput]);

        // The client asks for the input pane again (a tap into a text box that has the focus).
        client.raise_input_pane_activation_requested();
        assert_eq!(host.take_calls(), vec![HostCall::ShowSoftInput]);

        input_method.set_client(None);
        assert!(!input_method.is_active());
        assert_eq!(host.take_calls(), vec![HostCall::RestartInput, HostCall::HideSoftInput]);

        // The events of a client that was let go are not followed.
        client.raise_input_pane_activation_requested();
        client.raise_selection_changed();
        assert!(host.take_calls().is_empty());
    }

    #[test]
    fn the_view_gets_no_connection_without_a_client_and_one_per_request_with_it() {
        let (host, input_method) = input_method();
        input_method.set_options(&TextInputOptions::default());
        assert!(host.create_input_connection().is_none());

        input_method.set_client(Some(TestClient::new("abc", 1, 1)));
        let (first, out_attrs) = host.create_input_connection().expect("a connection");
        assert_eq!(out_attrs, info(&TextInputOptions::default()));
        let (second, _) = host.create_input_connection().expect("a connection");
        assert!(!Rc::ptr_eq(&first, &second));
    }

    #[test]
    fn a_change_of_the_client_is_reported_through_the_connection() {
        let (host, input_method) = input_method();
        let client = TestClient::new("abc", 1, 1);
        input_method.set_options(&TextInputOptions::default());
        input_method.set_client(Some(client.clone()));
        let (connection, _) = host.create_input_connection().expect("a connection");
        host.take_calls();

        client.selection.set(TextSelection::new(2, 3));
        client.raise_selection_changed();
        // The state of the connection, then the selection of the client as it is.
        assert_eq!(
            host.take_calls(),
            vec![HostCall::UpdateSelection(2, 3, -1, -1), HostCall::UpdateSelection(2, 3, -1, -1)]
        );

        client.raise_surrounding_text_changed();
        assert_eq!(host.take_calls(), vec![HostCall::UpdateSelection(2, 3, -1, -1)]);

        // Not while the connection is in a batch or applies its commands.
        connection.begin_batch_edit();
        client.raise_selection_changed();
        client.raise_surrounding_text_changed();
        assert!(host.take_calls().is_empty());
        connection.end_batch_edit();
        host.take_calls();
        connection.set_is_in_update(true);
        client.raise_selection_changed();
        assert!(host.take_calls().is_empty());
        connection.set_is_in_update(false);

        input_method.on_batch_edit_ended();
        assert_eq!(host.take_calls().len(), 2);

        // A new client reports the state of the connection the view still has.
        input_method.set_client(Some(TestClient::new("xyz", 0, 0)));
        assert_eq!(
            host.take_calls(),
            vec![
                HostCall::RequestFocus,
                HostCall::RestartInput,
                HostCall::ShowSoftInput,
                HostCall::UpdateSelection(0, 0, -1, -1)
            ]
        );
    }

    #[test]
    fn the_content_type_is_the_input_type() {
        use input_types::*;
        let of = |content_type| info(&TextInputOptions { content_type, ..Default::default() }).input_type;
        assert_eq!(of(TextInputContentType::Normal), CLASS_TEXT);
        assert_eq!(of(TextInputContentType::Alpha), CLASS_TEXT);
        assert_eq!(of(TextInputContentType::Social), CLASS_TEXT);
        assert_eq!(of(TextInputContentType::Search), CLASS_TEXT);
        assert_eq!(of(TextInputContentType::Email), 0x21);
        assert_eq!(of(TextInputContentType::Number), 0x3002);
        // Password retains its privacy hints as well as its keyboard purpose,
        // including when a later scope changes that purpose independently.
        assert_eq!(
            of(TextInputContentType::Password),
            CLASS_TEXT | TEXT_VARIATION_PASSWORD | TEXT_FLAG_NO_SUGGESTIONS
        );
        assert_eq!(of(TextInputContentType::Pin), 0x12);
        assert_eq!(of(TextInputContentType::Digits), 3);
        assert_eq!(of(TextInputContentType::Url), 0x11);
        assert_eq!(of(TextInputContentType::Name), 0x61);
    }

    #[test]
    fn input_scope_distinguishes_numeric_components_phones_and_alphanumeric_pins() {
        use InputScopeNameValue as Scope;
        let cases = [
            (Scope::Digits, 0x2),
            (Scope::DateMonthNumber, 0x2),
            (Scope::DateDayNumber, 0x2),
            (Scope::DateYear, 0x2),
            (Scope::TimeHour, 0x2),
            (Scope::TimeMinutesOrSeconds, 0x2),
            (Scope::TelephoneNumber, 0x3),
            (Scope::TelephoneCountryCode, 0x3),
            (Scope::TelephoneAreaCode, 0x3),
            (Scope::TelephoneLocalNumber, 0x3),
            (Scope::Number, 0x3002),
            (Scope::CurrencyAmount, 0x3002),
            (Scope::FormulaNumber, 0x3002),
            (Scope::EmailSmtpAddress, 0x21),
            (Scope::EmailNameOrAddress, 0x21),
            (Scope::PersonalFullName, 0x61),
            (Scope::NameOrPhoneNumber, 0x61),
            (Scope::NumericPassword, 0x12),
            (Scope::NumericPin, 0x12),
            (Scope::Password, 0x80081),
            (Scope::AlphanumericPin, 0x80081),
        ];
        for (scope, expected) in cases {
            let options = TextInputOptions {
                content_type: TextInputContentType::Url,
                input_scope_names: Some(Rc::from([scope])),
                ..Default::default()
            };
            assert_eq!(info(&options).input_type, expected, "{scope:?}");
        }
    }

    #[test]
    fn input_scope_first_name_and_clear_update_the_android_editor_callback() {
        use InputScopeNameValue as Scope;
        let (host, input_method) = input_method();
        input_method.set_client(Some(TestClient::new("", 0, 0)));
        host.take_calls();
        let mut options = TextInputOptions {
            content_type: TextInputContentType::Digits,
            input_scope_names: Some(Rc::from([Scope::Digits, Scope::TelephoneNumber])),
            ..Default::default()
        };
        input_method.set_options(&options);
        assert_eq!(host.take_calls(), [HostCall::RestartInput]);
        let (_, editor) = host.create_input_connection().unwrap();
        assert_eq!(editor.input_type, 2);
        options.input_scope_names = Some(Rc::from([Scope::Default, Scope::Digits]));
        input_method.set_options(&options);
        assert_eq!(host.take_calls(), [HostCall::RestartInput]);
        let (_, editor) = host.create_input_connection().unwrap();
        assert_eq!(editor.input_type, 1);
        options.input_scope_names = Some(Rc::from([]));
        input_method.set_options(&options);
        assert_eq!(host.take_calls(), [HostCall::RestartInput]);
        let (_, editor) = host.create_input_connection().unwrap();
        assert_eq!(editor.input_type, 3);
    }

    #[test]
    fn input_scope_restart_preserves_the_same_clients_composing_region() {
        let (host, input_method) = input_method();
        let client = TestClient::new("abcd", 1, 3);
        let top_level: Rc<dyn IInputConnectionTopLevel> = TestInputMethod::new(Some(client.clone()));
        input_method.set_options(&TextInputOptions::default());
        input_method.set_client(Some(client.clone()));
        let init = host.init.borrow().clone().unwrap();
        let first = init(&top_level, &mut EditorInfo::default()).unwrap();
        assert!(first.set_composing_text(Some("x"), 1));
        assert_eq!(client.text(), "axd");
        assert_eq!(first.edit_buffer().composition(), Some(TextSelection::new(1, 2)));
        host.take_calls();

        input_method.set_options(&TextInputOptions {
            input_scope_names: Some(Rc::from([InputScopeNameValue::Hiragana])),
            ..Default::default()
        });
        assert_eq!(host.take_calls(), [HostCall::RestartInput]);
        first.close_connection();
        let init = host.init.borrow().clone().unwrap();
        let next = init(&top_level, &mut EditorInfo::default()).unwrap();
        assert!(!Rc::ptr_eq(&first, &next));
        assert_eq!(next.edit_buffer().composition(), Some(TextSelection::new(1, 2)));
        // The next IME update must replace its old marked text instead of
        // inserting beside it after the option-only connection restart.
        assert!(next.set_composing_text(Some("yz"), 1));
        assert_eq!(client.text(), "ayzd");
        assert!(next.commit_text(Some("accepted"), 1));
        assert_eq!(client.text(), "aacceptedd");
        assert_eq!(next.edit_buffer().composition(), None);
    }

    #[test]
    fn input_scope_restart_does_not_carry_composition_across_a_client_assignment() {
        for same_client in [false, true] {
            let (host, input_method) = input_method();
            let client = TestClient::new("abcd", 1, 3);
            input_method.set_options(&TextInputOptions::default());
            input_method.set_client(Some(client.clone()));
            let (first, _) = host.create_input_connection().unwrap();
            assert!(first.set_composing_region(1, 3));
            input_method.set_options(&TextInputOptions {
                input_scope_names: Some(Rc::from([InputScopeNameValue::EmailSmtpAddress])),
                ..Default::default()
            });
            // A focus switch or explicit client reinstall before Android asks
            // for its connection retires the pending options-only handoff.
            input_method.set_client(Some(if same_client { client } else { TestClient::new("else", 0, 0) }));
            // Further option changes before the new client gets a connection
            // must not reuse the old client's still-retained native connection.
            input_method.set_options(&TextInputOptions {
                input_scope_names: Some(Rc::from([InputScopeNameValue::Search])),
                ..Default::default()
            });
            let (next, _) = host.create_input_connection().unwrap();
            assert_eq!(next.edit_buffer().composition(), None);
        }
    }

    #[test]
    fn input_scope_numeric_pads_do_not_get_text_flags_and_secrets_disable_learning() {
        use InputScopeNameValue as Scope;
        for scope in [Scope::Digits, Scope::NumericPin] {
            let mut options = TextInputOptions {
                input_scope_names: Some(Rc::from([scope])),
                auto_capitalization: true,
                multiline: true,
                show_suggestions: Some(true),
                ..Default::default()
            };
            let editor = info(&options);
            assert_eq!(editor.input_type, if scope == Scope::Digits { 2 } else { 0x12 });
            assert_eq!(editor.initial_caps_mode, 0);
            assert_eq!(editor.ime_options & ime_flags::NO_PERSONALIZED_LEARNING != 0, scope == Scope::NumericPin);
            options.input_scope_names = Some(Rc::from([Scope::AlphanumericPin]));
            let mut old = EditorInfo::default();
            AndroidInputMethod::fill_editor_info(&options, &mut old, 25);
            assert_eq!(old.ime_options & ime_flags::NO_PERSONALIZED_LEARNING, 0);
            assert_ne!(old.input_type & input_types::TEXT_FLAG_NO_SUGGESTIONS, 0);
        }
    }

    #[test]
    fn input_scope_override_preserves_independently_configured_privacy() {
        use input_types::*;
        use InputScopeNameValue as Scope;

        for (content_type, is_sensitive) in [
            (TextInputContentType::Password, false),
            (TextInputContentType::Pin, false),
            (TextInputContentType::Normal, true),
        ] {
            for (scope, input_type) in [
                (Scope::Default, CLASS_TEXT | TEXT_FLAG_NO_SUGGESTIONS),
                (Scope::EmailSmtpAddress, CLASS_TEXT | TEXT_VARIATION_EMAIL_ADDRESS | TEXT_FLAG_NO_SUGGESTIONS),
                (Scope::Digits, CLASS_NUMBER),
                (Scope::TelephoneNumber, CLASS_PHONE),
            ] {
                let options = TextInputOptions {
                    content_type,
                    is_sensitive,
                    input_scope_names: Some(Rc::from([scope])),
                    show_suggestions: Some(true),
                    ..Default::default()
                };
                let editor = info(&options);
                assert_eq!(editor.input_type, input_type, "{content_type:?}/{is_sensitive}/{scope:?}");
                assert_ne!(editor.ime_options & ime_flags::NO_PERSONALIZED_LEARNING, 0);
                let mut old = EditorInfo::default();
                AndroidInputMethod::fill_editor_info(&options, &mut old, 25);
                assert_eq!(old.input_type, input_type);
                assert_eq!(old.ime_options & ime_flags::NO_PERSONALIZED_LEARNING, 0);
            }
        }
    }

    #[test]
    fn the_return_key_is_the_action_of_the_editor() {
        let of = |return_key_type, multiline| {
            info(&TextInputOptions { return_key_type, multiline, ..Default::default() }).ime_options
        };
        let always = ime_flags::NO_FULLSCREEN | ime_flags::NO_EXTRACT_UI;
        assert_eq!(of(TextInputReturnKeyType::Default, false), 6 | always);
        assert_eq!(of(TextInputReturnKeyType::Default, true), ime_flags::NO_ENTER_ACTION | always);
        assert_eq!(of(TextInputReturnKeyType::Return, false), ime_flags::NO_ENTER_ACTION | always);
        assert_eq!(of(TextInputReturnKeyType::Go, false), 2 | always);
        assert_eq!(of(TextInputReturnKeyType::Search, false), 3 | always);
        assert_eq!(of(TextInputReturnKeyType::Send, false), 4 | always);
        assert_eq!(of(TextInputReturnKeyType::Next, false), 5 | always);
        assert_eq!(of(TextInputReturnKeyType::Done, true), 6 | always);
        assert_eq!(of(TextInputReturnKeyType::Previous, false), 7 | always);
    }

    #[test]
    fn the_other_options_are_flags() {
        use input_types::*;
        let caps = info(&TextInputOptions { auto_capitalization: true, multiline: true, ..Default::default() });
        assert_eq!(caps.input_type, CLASS_TEXT | TEXT_FLAG_CAP_SENTENCES | TEXT_FLAG_MULTI_LINE);
        assert_eq!(caps.initial_caps_mode, CAPITALIZATION_MODE_SENTENCES);

        // No suggestions: a normal text also becomes a visible password, which is how a keyboard is told.
        let plain = info(&TextInputOptions { show_suggestions: Some(false), ..Default::default() });
        assert_eq!(plain.input_type, CLASS_TEXT | TEXT_FLAG_NO_SUGGESTIONS | TEXT_VARIATION_VISIBLE_PASSWORD);
        let email = info(&TextInputOptions {
            show_suggestions: Some(false),
            content_type: TextInputContentType::Email,
            ..Default::default()
        });
        assert_eq!(email.input_type, CLASS_TEXT | TEXT_VARIATION_EMAIL_ADDRESS | TEXT_FLAG_NO_SUGGESTIONS);
        let number = info(&TextInputOptions {
            show_suggestions: Some(false),
            content_type: TextInputContentType::Digits,
            ..Default::default()
        });
        assert_eq!(number.input_type, CLASS_PHONE);
        assert_eq!(info(&TextInputOptions { show_suggestions: Some(true), ..Default::default() }).input_type, 1);

        let sensitive = info(&TextInputOptions { is_sensitive: true, ..Default::default() });
        assert_eq!(sensitive.input_type, CLASS_TEXT | TEXT_FLAG_NO_SUGGESTIONS);
        assert_ne!(sensitive.ime_options & ime_flags::NO_PERSONALIZED_LEARNING, 0);
        let mut old = EditorInfo::default();
        AndroidInputMethod::fill_editor_info(
            &TextInputOptions { is_sensitive: true, ..Default::default() },
            &mut old,
            25,
        );
        assert_eq!(old.ime_options & ime_flags::NO_PERSONALIZED_LEARNING, 0);

        assert_eq!(info(&TextInputOptions::default()).hint_locales, None);
        let hints: Rc<[String]> = Rc::from(vec!["pl-PL".to_string(), "en".to_string()]);
        let hinted = info(&TextInputOptions { locale_hints: Some(hints), ..Default::default() });
        assert_eq!(hinted.hint_locales, Some(vec!["pl-PL".to_string(), "en".to_string()]));
        let empty: Rc<[String]> = Rc::from(Vec::new());
        assert_eq!(info(&TextInputOptions { locale_hints: Some(empty), ..Default::default() }).hint_locales, None);
    }
}

#[cfg(target_os = "android")]
pub(crate) use imp::{extracted_text_to_java, ViewInputMethodHost};

#[cfg(target_os = "android")]
mod imp {
    use super::{ExtractedText, IInitEditorInfo, IInputMethodHost, InitEditorInfo, KeyEventToDispatch};
    use crate::interop::java::{
        call_boolean, call_object, call_static_object, call_void, new_object, JavaClass, JavaLocal, JavaObject,
        JavaRef, JavaValue,
    };
    use crate::interop::natives::PLATFORM_HELPER;
    use std::cell::RefCell;

    /// `InputMethodManager.SHOW_IMPLICIT` and `InputMethodManager.HIDE_IMPLICIT_ONLY`.
    const SHOW_IMPLICIT: i32 = 1;
    const HIDE_IMPLICIT_ONLY: i32 = 1;

    /// The extracted text of the system for the values of the edit buffer.
    pub(crate) fn extracted_text_to_java(text: &ExtractedText) -> Option<JavaLocal> {
        call_static_object(
            &JavaClass::find(PLATFORM_HELPER),
            "newExtractedText",
            "(Ljava/lang/String;IIIIII)Landroid/view/inputmethod/ExtractedText;",
            &[
                JavaValue::String(&text.text),
                JavaValue::Int(text.flags),
                JavaValue::Int(text.partial_start_offset),
                JavaValue::Int(text.partial_end_offset),
                JavaValue::Int(text.selection_start),
                JavaValue::Int(text.selection_end),
                JavaValue::Int(text.start_offset),
            ],
        )
    }

    /// The Java view of the framework and the input method manager of its
    /// context.
    pub(crate) struct ViewInputMethodHost {
        view: JavaObject,
        imm: JavaObject,
        init_editor_info: RefCell<Option<InitEditorInfo>>,
    }

    impl ViewInputMethodHost {
        /// # Panics
        /// Panics when the context has no input method manager.
        pub fn new(view: JavaObject, context: &JavaObject) -> Self {
            let imm = call_object(
                context,
                "getSystemService",
                "(Ljava/lang/String;)Ljava/lang/Object;",
                &[JavaValue::String("input_method")],
            );
            let Some(imm) = imm else {
                panic!("Context.InputMethodService is expected to be not null.");
            };
            Self { view, imm: imm.to_global(), init_editor_info: RefCell::new(None) }
        }

        /// What the input method said the next input connection is made
        /// with.
        pub fn editor_info_init(&self) -> Option<InitEditorInfo> {
            self.init_editor_info.borrow().clone()
        }
    }

    impl IInitEditorInfo for ViewInputMethodHost {
        fn init_editor_info(&self, init: InitEditorInfo) {
            *self.init_editor_info.borrow_mut() = Some(init);
        }
    }

    impl IInputMethodHost for ViewInputMethodHost {
        fn set_focusable(&self) {
            call_void(&self.view, "setFocusable", "(Z)V", &[JavaValue::Boolean(true)]);
            call_void(&self.view, "setFocusableInTouchMode", "(Z)V", &[JavaValue::Boolean(true)]);
        }

        fn request_focus(&self) {
            call_boolean(&self.view, "requestFocus", "()Z", &[]);
        }

        fn restart_input(&self) {
            call_void(&self.imm, "restartInput", "(Landroid/view/View;)V", &[JavaValue::Object(Some(&self.view))]);
        }

        fn show_soft_input(&self) {
            call_boolean(
                &self.imm,
                "showSoftInput",
                "(Landroid/view/View;I)Z",
                &[JavaValue::Object(Some(&self.view)), JavaValue::Int(SHOW_IMPLICIT)],
            );
        }

        fn hide_soft_input(&self) {
            let token = call_object(&self.view, "getWindowToken", "()Landroid/os/IBinder;", &[]);
            call_boolean(
                &self.imm,
                "hideSoftInputFromWindow",
                "(Landroid/os/IBinder;I)Z",
                &[
                    JavaValue::Object(token.as_ref().map(|token| token as &dyn JavaRef)),
                    JavaValue::Int(HIDE_IMPLICIT_ONLY),
                ],
            );
        }

        fn update_selection(&self, sel_start: i32, sel_end: i32, candidates_start: i32, candidates_end: i32) {
            call_void(
                &self.imm,
                "updateSelection",
                "(Landroid/view/View;IIII)V",
                &[
                    JavaValue::Object(Some(&self.view)),
                    JavaValue::Int(sel_start),
                    JavaValue::Int(sel_end),
                    JavaValue::Int(candidates_start),
                    JavaValue::Int(candidates_end),
                ],
            );
        }

        fn update_extracted_text(&self, token: i32, text: &ExtractedText) {
            let extracted = extracted_text_to_java(text);
            call_void(
                &self.imm,
                "updateExtractedText",
                "(Landroid/view/View;ILandroid/view/inputmethod/ExtractedText;)V",
                &[
                    JavaValue::Object(Some(&self.view)),
                    JavaValue::Int(token),
                    JavaValue::Object(extracted.as_ref().map(|extracted| extracted as &dyn JavaRef)),
                ],
            );
        }

        fn dispatch_key_event(&self, key_event: &KeyEventToDispatch) {
            let class = JavaClass::find("android/view/KeyEvent");
            let dispatch = |event: &dyn JavaRef| {
                call_boolean(
                    &self.view,
                    "dispatchKeyEvent",
                    "(Landroid/view/KeyEvent;)Z",
                    &[JavaValue::Object(Some(event))],
                );
            };
            match key_event {
                KeyEventToDispatch::New { action, code } => {
                    dispatch(&new_object(&class, "(II)V", &[JavaValue::Int(*action), JavaValue::Int(*code)]));
                }
                KeyEventToDispatch::Full {
                    down_time,
                    event_time,
                    action,
                    code,
                    repeat,
                    meta_state,
                    device_id,
                    scancode,
                    flags,
                } => dispatch(&new_object(
                    &class,
                    "(JJIIIIIII)V",
                    &[
                        JavaValue::Long(*down_time),
                        JavaValue::Long(*event_time),
                        JavaValue::Int(*action),
                        JavaValue::Int(*code),
                        JavaValue::Int(*repeat),
                        JavaValue::Int(*meta_state),
                        JavaValue::Int(*device_id),
                        JavaValue::Int(*scancode),
                        JavaValue::Int(*flags),
                    ],
                )),
                KeyEventToDispatch::System(event) => {
                    if let Some(event) = event.downcast_ref::<JavaObject>() {
                        dispatch(event);
                    }
                }
            }
        }
    }
}
