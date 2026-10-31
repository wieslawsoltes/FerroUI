//! The editor info of a client of the exact-edit connection (R12 of
//! `docs/porting/winui-rich-text.md`, decisions D5 and D8): the first input scope of
//! its owner chooses the keyboard, a password also turns off suggestions and
//! learning, and a live change of the options restarts the input only when the
//! editor info changes. A legacy client keeps upstream's editor info and never
//! restarts the input for its options.
use super::super::text_edit_buffer::test_support::{TestClient, TestInputMethod};
use super::tests::{info, input_method, HostCall};
use super::*;
use ferroui_base::data::TemplateBinding;
use ferroui_base::input::text_input::ITextInputMethodImpl;
use ferroui_base::Ref;
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::templates::{FuncControlTemplate, FuncTemplateNameScopeExtensions};
use ferroui_controls::testing::{MockWindowingPlatform, TestServices, UnitTestApplication, UnitTestApplicationScope};
use ferroui_controls::{Border, ContentControl, Control, Window};
use InputScopeNameValue as Scope;

/// The input method of the window of the unit-test application: the one under test
/// is driven by hand, with the client the manager connected.
struct NoInputMethod;

impl ITextInputMethodImpl for NoInputMethod {
    fn set_client(&self, _client: Option<Rc<dyn TextInputMethodClient>>) {}
    fn set_cursor_rect(&self, _rect: Rect) {}
    fn set_options(&self, _options: &TextInputOptions) {}
    fn reset(&self) {}
}

/// A focused element of a window that offers a client: the input method manager of
/// the window gives a client of the exact-edit connection its connection.
struct Owner {
    element: Ref<Border>,
    window: Ref<Window>,
    _app: UnitTestApplicationScope,
}

impl Owner {
    fn new(client: &Rc<TestClient>) -> Self {
        let app = UnitTestApplication::start(TestServices::focusable_window().with_windowing_platform(
            MockWindowingPlatform::with_window_impl(|| {
                let window = MockWindowingPlatform::create_window_mock();
                window.setup_feature::<dyn ITextInputMethodImpl>(Rc::new(NoInputMethod));
                window
            }),
        ));
        let element = Border::new();
        element.set_focusable(true);
        element.set_height(20.0);
        let offered = client.clone();
        element.text_input_method_client_requested(move |_, e| e.set_client(Some(offered.clone())));
        let window = Window::new();
        window.set_template(Some(FuncControlTemplate::for_type::<Window>(|_, scope| {
            let presenter = ContentPresenter::new();
            presenter.set_name(Some("PART_ContentPresenter".to_owned()));
            let content = ContentControl::content_property().as_property();
            presenter.bind_binding(content, &TemplateBinding::new(content));
            presenter.register_in_name_scope(&**scope).upcast()
        })));
        window.set_content(Some(Control::boxed(element.clone())));
        window.show();
        assert!(element.focus());
        Self { element, window, _app: app }
    }

    fn set_scopes(&self, names: &[InputScopeNameValue]) {
        TextInputOptions::set_input_scope_names(&self.element, Some(Rc::from(names)));
    }
}

impl Drop for Owner {
    fn drop(&mut self) {
        self.window.close();
    }
}

fn connection_info(options: &TextInputOptions, scope: Option<InputScopeNameValue>) -> EditorInfo {
    let mut out_attrs = EditorInfo::default();
    AndroidInputMethod::fill_connection_editor_info(options, scope, &mut out_attrs, 36);
    out_attrs
}

#[test]
fn a_legacy_client_keeps_upstreams_editor_info_and_never_restarts_for_its_options() {
    use input_types::*;
    let (host, input_method) = input_method();
    let client = TestClient::new("abc", 1, 1);
    input_method.set_options(&TextInputOptions::default());
    input_method.set_client(Some(client.clone()));
    assert!(client.input_method_connection().is_none());
    host.create_input_connection().unwrap();
    host.take_calls();

    // Upstream installs the new function and nothing else.
    for content_type in [TextInputContentType::Email, TextInputContentType::Password, TextInputContentType::Pin] {
        let options = TextInputOptions { content_type, ..Default::default() };
        input_method.set_options(&options);
        assert_eq!(host.take_calls(), []);
        let (_, editor) = host.create_input_connection().unwrap();
        assert_eq!(editor, info(&options));
    }
    // Upstream's password: no suggestion or learning flags unless it is sensitive.
    let (_, editor) = host.create_input_connection().unwrap();
    assert_eq!(editor.input_type, CLASS_NUMBER | NUMBER_VARIATION_PASSWORD);
    input_method.set_options(&TextInputOptions { content_type: TextInputContentType::Password, ..Default::default() });
    let (_, editor) = host.create_input_connection().unwrap();
    assert_eq!(editor.input_type, 0x81);
    assert_eq!(editor.ime_options & ime_flags::NO_PERSONALIZED_LEARNING, 0);
}

#[test]
fn a_connection_client_gets_the_keyboard_of_the_first_scope_and_restarts_only_when_it_changes() {
    let (host, input_method) = input_method();
    let client = TestClient::editing("", 0, 0);
    let owner = Owner::new(&client);
    assert!(client.input_method_connection().is_some_and(|connection| connection.is_current()));
    let options = TextInputOptions { content_type: TextInputContentType::Digits, ..Default::default() };
    input_method.set_options(&options);
    input_method.set_client(Some(client.clone()));
    assert_eq!(host.take_calls(), [HostCall::RequestFocus, HostCall::RestartInput, HostCall::ShowSoftInput]);
    let (_, editor) = host.create_input_connection().unwrap();
    assert_eq!(editor.input_type, 3);

    owner.set_scopes(&[Scope::Digits, Scope::TelephoneNumber]);
    input_method.set_options(&options);
    assert_eq!(host.take_calls(), [HostCall::RestartInput]);
    let (_, editor) = host.create_input_connection().unwrap();
    assert_eq!(editor.input_type, 2);

    // The same editor info: no restart.
    input_method.set_options(&options);
    owner.set_scopes(&[Scope::Digits]);
    input_method.set_options(&options);
    assert_eq!(host.take_calls(), []);

    owner.set_scopes(&[Scope::Default, Scope::Digits]);
    input_method.set_options(&options);
    assert_eq!(host.take_calls(), [HostCall::RestartInput]);
    let (_, editor) = host.create_input_connection().unwrap();
    assert_eq!(editor.input_type, 1);

    // An empty list leaves the keyboard to the content type.
    owner.set_scopes(&[]);
    input_method.set_options(&options);
    assert_eq!(host.take_calls(), [HostCall::RestartInput]);
    let (_, editor) = host.create_input_connection().unwrap();
    assert_eq!(editor.input_type, 3);

    // Before the system asked for a connection of a new binding, nothing is restarted.
    input_method.set_client(Some(client.clone()));
    host.take_calls();
    owner.set_scopes(&[Scope::Url]);
    input_method.set_options(&options);
    assert_eq!(host.take_calls(), []);
    let (_, editor) = host.create_input_connection().unwrap();
    assert_eq!(editor.input_type, 0x11);
}

#[test]
fn a_legacy_client_after_a_connection_client_does_not_restart_for_its_options() {
    let (host, input_method) = input_method();
    let editing = TestClient::editing("", 0, 0);
    let owner = Owner::new(&editing);
    input_method.set_options(&TextInputOptions::default());
    input_method.set_client(Some(editing.clone()));
    host.create_input_connection().unwrap();
    owner.set_scopes(&[Scope::EmailSmtpAddress]);

    let legacy = TestClient::new("abc", 0, 0);
    input_method.set_client(Some(legacy));
    host.take_calls();
    input_method.set_options(&TextInputOptions { content_type: TextInputContentType::Url, ..Default::default() });
    assert_eq!(host.take_calls(), []);
    let (_, editor) = host.create_input_connection().unwrap();
    assert_eq!(editor.input_type, 0x11);
}

#[test]
fn a_restart_for_changed_options_keeps_the_composing_region_of_the_same_client() {
    let (host, input_method) = input_method();
    let client = TestClient::editing("abcd", 1, 3);
    let owner = Owner::new(&client);
    let top_level: Rc<dyn IInputConnectionTopLevel> = TestInputMethod::new(Some(client.clone()));
    input_method.set_options(&TextInputOptions::default());
    input_method.set_client(Some(client.clone()));
    let init = host.init.borrow().clone().unwrap();
    let first = init(&top_level, &mut EditorInfo::default()).unwrap();
    assert!(first.set_composing_text(Some("x"), 1));
    assert_eq!(client.text(), "axd");
    assert_eq!(first.edit_buffer().composition(), Some(TextSelection::new(1, 2)));
    host.take_calls();

    owner.set_scopes(&[Scope::EmailSmtpAddress]);
    input_method.set_options(&TextInputOptions::default());
    assert_eq!(host.take_calls(), [HostCall::RestartInput]);
    first.close_connection();
    let init = host.init.borrow().clone().unwrap();
    let next = init(&top_level, &mut EditorInfo::default()).unwrap();
    assert!(!Rc::ptr_eq(&first, &next));
    assert_eq!(next.edit_buffer().composition(), Some(TextSelection::new(1, 2)));
    // The next update replaces the old marked text instead of inserting beside it.
    assert!(next.set_composing_text(Some("yz"), 1));
    assert_eq!(client.text(), "ayzd");
    assert!(next.commit_text(Some("accepted"), 1));
    assert_eq!(client.text(), "aacceptedd");
    assert_eq!(next.edit_buffer().composition(), None);
}

#[test]
fn a_client_assignment_cancels_the_composing_region_of_a_restart() {
    for same_client in [false, true] {
        let (host, input_method) = input_method();
        let client = TestClient::editing("abcd", 1, 3);
        let owner = Owner::new(&client);
        input_method.set_options(&TextInputOptions::default());
        input_method.set_client(Some(client.clone()));
        let (first, _) = host.create_input_connection().unwrap();
        assert!(first.set_composing_region(1, 3));
        owner.set_scopes(&[Scope::EmailSmtpAddress]);
        input_method.set_options(&TextInputOptions::default());
        // A focus change or a new binding before the system asks for its connection
        // retires the handoff.
        input_method.set_client(Some(if same_client { client.clone() } else { TestClient::editing("else", 0, 0) }));
        owner.set_scopes(&[Scope::Search]);
        input_method.set_options(&TextInputOptions::default());
        let (next, _) = host.create_input_connection().unwrap();
        assert_eq!(next.edit_buffer().composition(), None);
    }
}

#[test]
fn input_scope_distinguishes_numeric_components_phones_and_alphanumeric_pins() {
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
        (Scope::Hiragana, 0x1),
    ];
    let options = TextInputOptions { content_type: TextInputContentType::Url, ..Default::default() };
    for (scope, expected) in cases {
        assert_eq!(connection_info(&options, Some(scope)).input_type, expected, "{scope:?}");
    }
}

#[test]
fn without_a_scope_a_connection_client_differs_from_upstream_only_for_passwords_and_pins() {
    use input_types::*;
    for content_type in [
        TextInputContentType::Normal,
        TextInputContentType::Alpha,
        TextInputContentType::Digits,
        TextInputContentType::Number,
        TextInputContentType::Email,
        TextInputContentType::Url,
        TextInputContentType::Name,
        TextInputContentType::Social,
        TextInputContentType::Search,
    ] {
        for (auto_capitalization, multiline, show_suggestions, is_sensitive) in
            [(false, false, None, false), (true, true, Some(false), false), (false, true, Some(true), true)]
        {
            let options = TextInputOptions {
                content_type,
                auto_capitalization,
                multiline,
                show_suggestions,
                is_sensitive,
                ..Default::default()
            };
            assert_eq!(connection_info(&options, None), info(&options), "{options:?}");
        }
    }
    // P33: a password or PIN of a connection client also turns off suggestions (text
    // class) and personalized learning; upstream's values stay for legacy clients.
    let password = TextInputOptions { content_type: TextInputContentType::Password, ..Default::default() };
    assert_eq!(info(&password).input_type, 0x81);
    let editor = connection_info(&password, None);
    assert_eq!(editor.input_type, CLASS_TEXT | TEXT_VARIATION_PASSWORD | TEXT_FLAG_NO_SUGGESTIONS);
    assert_ne!(editor.ime_options & ime_flags::NO_PERSONALIZED_LEARNING, 0);
    let pin = TextInputOptions { content_type: TextInputContentType::Pin, ..Default::default() };
    let editor = connection_info(&pin, None);
    assert_eq!(editor.input_type, 0x12);
    assert_ne!(editor.ime_options & ime_flags::NO_PERSONALIZED_LEARNING, 0);
}

#[test]
fn input_scope_numeric_pads_do_not_get_text_flags_and_secrets_disable_learning() {
    for scope in [Scope::Digits, Scope::NumericPin] {
        let options = TextInputOptions {
            auto_capitalization: true,
            multiline: true,
            show_suggestions: Some(true),
            ..Default::default()
        };
        let editor = connection_info(&options, Some(scope));
        assert_eq!(editor.input_type, if scope == Scope::Digits { 2 } else { 0x12 });
        assert_eq!(editor.initial_caps_mode, 0);
        assert_eq!(editor.ime_options & ime_flags::NO_PERSONALIZED_LEARNING != 0, scope == Scope::NumericPin);
        let mut old = EditorInfo::default();
        AndroidInputMethod::fill_connection_editor_info(&options, Some(Scope::AlphanumericPin), &mut old, 25);
        assert_eq!(old.ime_options & ime_flags::NO_PERSONALIZED_LEARNING, 0);
        assert_ne!(old.input_type & input_types::TEXT_FLAG_NO_SUGGESTIONS, 0);
    }
}

#[test]
fn input_scope_override_preserves_independently_configured_privacy() {
    use input_types::*;
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
            let options = TextInputOptions { content_type, is_sensitive, show_suggestions: Some(true), ..Default::default() };
            let editor = connection_info(&options, Some(scope));
            assert_eq!(editor.input_type, input_type, "{content_type:?}/{is_sensitive}/{scope:?}");
            assert_ne!(editor.ime_options & ime_flags::NO_PERSONALIZED_LEARNING, 0);
            let mut old = EditorInfo::default();
            AndroidInputMethod::fill_connection_editor_info(&options, Some(scope), &mut old, 25);
            assert_eq!(old.input_type, input_type);
            assert_eq!(old.ime_options & ime_flags::NO_PERSONALIZED_LEARNING, 0);
        }
    }
}
