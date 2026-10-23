//! The production scope policy and transport lifecycle with observable native
//! calls. Device keyboard behavior remains a separate Windows validation gate.

use super::super::tests::{Call, Client, FakeSystem};
use super::*;
use ferroui_base::input::text_input::ITextInputMethodImpl;
use ferroui_base::input::{Key, PhysicalKey};

const FIRST: isize = 0x40;
const SECOND: isize = 0x41;
const JAPANESE: isize = 0x411;

struct Window(isize);

impl Imm32Parent for Window {
    fn hwnd(&self) -> isize {
        self.0
    }
    fn desktop_scaling(&self) -> f64 {
        1.0
    }
    fn set_ignore_wm_char(&self, _value: bool) {}
    fn raw_text_input(&self, _timestamp: u64, _text: &str) -> bool {
        true
    }
    fn raw_key_press(&self, _key: Key, _physical_key: PhysicalKey) {}
}

fn parent(hwnd: isize) -> Rc<dyn Imm32Parent> {
    Rc::new(Window(hwnd))
}

fn options(names: &[InputScopeNameValue]) -> TextInputOptions {
    TextInputOptions {
        input_scope_names: Some(Rc::from(names)),
        ..Default::default()
    }
}

fn active(system: &Rc<FakeSystem>, parent: &Rc<dyn Imm32Parent>) -> Rc<Imm32InputMethod> {
    let input = Imm32InputMethod::new(system.clone());
    input.set_language_and_window(Rc::downgrade(parent), parent.hwnd(), JAPANESE);
    input.set_client(Some(Client::new()));
    system.run_posted();
    system.take_calls();
    input
}

fn scope_calls(system: &FakeSystem) -> Vec<Call> {
    system
        .take_calls()
        .into_iter()
        .filter(|call| matches!(call, Call::SetInputScopes(..)))
        .collect()
}

#[test]
fn imm32_input_scope_preserves_all_43_native_values_and_secret_hints() {
    let expected = [
        0, 1, 5, 7, 20, 21, 23, 24, 25, 28, 29, 31, 32, 33, 34, 35, 37, 38, 39, 40, 41, 44, 45, 46,
        47, 48, 49, 50, 51, 52, 53, 54, 55, 57, 58, 59, 60, 62, 63, 64, 65, 67, 68,
    ];
    for (scope, raw) in InputScopeNameValue::ALL.into_iter().zip(expected) {
        let result = native_scopes(&options(&[scope]));
        let expected = if [31, 63, 64, 65].contains(&raw) {
            vec![raw, 61]
        } else {
            vec![raw]
        };
        assert_eq!(&*result, expected.as_slice(), "{scope:?}");
    }
}

#[test]
fn imm32_input_scope_preserves_order_duplicates_and_the_immutable_snapshot() {
    use InputScopeNameValue as Scope;
    let snapshot: Rc<[Scope]> = Rc::from([
        Scope::EmailSmtpAddress,
        Scope::Digits,
        Scope::EmailSmtpAddress,
    ]);
    let options = TextInputOptions {
        input_scope_names: Some(snapshot.clone()),
        ..Default::default()
    };
    assert_eq!(&*native_scopes(&options), &[5, 28, 5]);
    assert!(Rc::ptr_eq(
        options.input_scope_names.as_ref().unwrap(),
        &snapshot
    ));
    assert_eq!(
        &*snapshot,
        &[
            Scope::EmailSmtpAddress,
            Scope::Digits,
            Scope::EmailSmtpAddress
        ]
    );
}

#[test]
fn imm32_input_scope_empty_names_use_legacy_hints_and_default_is_explicit() {
    let expected = [
        (TextInputContentType::Normal, vec![0]),
        (TextInputContentType::Alpha, vec![40]),
        (TextInputContentType::Digits, vec![28]),
        (TextInputContentType::Pin, vec![64, 61]),
        (TextInputContentType::Number, vec![29]),
        (TextInputContentType::Email, vec![5]),
        (TextInputContentType::Url, vec![1]),
        (TextInputContentType::Name, vec![7]),
        (TextInputContentType::Password, vec![31, 61]),
        (TextInputContentType::Social, vec![58]),
        (TextInputContentType::Search, vec![50]),
    ];
    for (content_type, expected) in expected {
        for input_scope_names in [None, Some(Rc::from([]))] {
            let options = TextInputOptions {
                content_type,
                input_scope_names,
                ..Default::default()
            };
            assert_eq!(&*native_scopes(&options), expected.as_slice());
        }
    }
    let mut explicit = options(&[InputScopeNameValue::Default]);
    explicit.content_type = TextInputContentType::Email;
    assert_eq!(&*native_scopes(&explicit), &[0]);
}

#[test]
fn imm32_input_scope_keyboard_override_preserves_independent_privacy() {
    for content_type in [TextInputContentType::Password, TextInputContentType::Pin] {
        let mut explicit = options(&[InputScopeNameValue::EmailSmtpAddress]);
        explicit.content_type = content_type;
        assert_eq!(&*native_scopes(&explicit), &[5, 61]);
    }
    let mut explicit = options(&[InputScopeNameValue::Digits]);
    explicit.is_sensitive = true;
    assert_eq!(&*native_scopes(&explicit), &[28, 61]);
    explicit.input_scope_names = Some(Rc::from([
        InputScopeNameValue::Text,
        InputScopeNameValue::NumericPin,
    ]));
    assert_eq!(&*native_scopes(&explicit), &[57, 64, 61]);
}

#[test]
fn imm32_input_scope_site_retains_options_sent_before_client_assignment() {
    let system = FakeSystem::new();
    let owner = parent(FIRST);
    let input = Imm32InputMethod::new(system.clone());
    input.set_language_and_window(Rc::downgrade(&owner), FIRST, JAPANESE);
    let site = input.for_window(Rc::downgrade(&owner));
    let initial = options(&[
        InputScopeNameValue::EmailSmtpAddress,
        InputScopeNameValue::Digits,
    ]);
    site.set_options(&initial);
    assert!(scope_calls(&system).is_empty());
    site.set_client(Some(Client::new()));
    assert_eq!(
        scope_calls(&system),
        vec![Call::SetInputScopes(FIRST, vec![5, 28])]
    );
    assert_eq!(
        input.input_options.options.borrow().as_ref(),
        Some(&initial)
    );
    // Scope ownership does not depend on a posted IMM association or a HIMC.
    assert!(input.contexts.borrow().is_empty());
    input.release_window(FIRST, &Rc::downgrade(&owner));
    system.run_posted();
    assert_eq!(
        scope_calls(&system),
        vec![Call::SetInputScopes(FIRST, vec![0])]
    );
    assert!(system.input_scopes.borrow().is_empty());
    assert!(!input.is_active());
}

#[test]
fn imm32_input_scope_identical_native_hints_keep_the_latest_full_options() {
    let system = FakeSystem::new();
    let owner = parent(FIRST);
    let input = active(&system, &owner);
    let mut current = options(&[InputScopeNameValue::EmailSmtpAddress]);
    input.set_options(&current);
    system.take_calls();
    input.set_options(&current);
    current.multiline = true;
    current.lowercase = true;
    current.show_suggestions = Some(false);
    current.locale_hints = Some(Rc::from(["fr-FR".to_owned()]));
    input.set_options(&current);
    assert!(scope_calls(&system).is_empty());
    assert_eq!(
        input.input_options.options.borrow().as_ref(),
        Some(&current)
    );
}

#[test]
fn imm32_input_scope_updates_preserve_an_active_composition() {
    let system = FakeSystem::new();
    let owner = parent(FIRST);
    let input = active(&system, &owner);
    input.handle_composition_start();
    *system.composition_string.borrow_mut() = Some("候補".to_owned());
    input.handle_composition(0, super::super::GCS::GCS_COMPSTR as isize, 1);
    system.take_calls();
    input.set_options(&options(&[InputScopeNameValue::Digits]));
    assert_eq!(
        system.take_calls(),
        vec![Call::SetInputScopes(FIRST, vec![28])]
    );
    assert!(input.is_composing());
    assert_eq!(input.composition().as_deref(), Some("候補"));
}

#[test]
fn imm32_input_scope_reentrant_update_applies_the_latest_names_without_borrows() {
    let system = FakeSystem::new();
    let owner = parent(FIRST);
    let input = active(&system, &owner);
    let weak = Rc::downgrade(&input);
    *system.next_hook.borrow_mut() = Some((
        |call| matches!(call, Call::SetInputScopes(..)),
        Box::new(move || {
            let input = weak.upgrade().unwrap();
            input.set_options(&options(&[
                InputScopeNameValue::Digits,
                InputScopeNameValue::Url,
            ]));
        }),
    ));
    input.set_options(&options(&[InputScopeNameValue::EmailSmtpAddress]));
    assert_eq!(
        scope_calls(&system),
        vec![
            Call::SetInputScopes(FIRST, vec![5]),
            Call::SetInputScopes(FIRST, vec![28, 1]),
        ]
    );
    assert_eq!(system.input_scopes.borrow().get(&FIRST), Some(&vec![28, 1]));
}

#[test]
fn imm32_input_scope_failed_native_set_is_uncached_and_retries_on_the_next_update() {
    let system = FakeSystem::new();
    let owner = parent(FIRST);
    let input = active(&system, &owner);
    let desired = options(&[InputScopeNameValue::Digits]);
    system.refuse_input_scopes.set(true);
    input.set_options(&desired);
    assert_eq!(
        scope_calls(&system),
        vec![Call::SetInputScopes(FIRST, vec![28])]
    );
    assert!(
        input.input_options.windows.borrow()[0]
            .applied
            .borrow()
            .is_none()
    );
    assert!(system.input_scopes.borrow().is_empty());
    system.refuse_input_scopes.set(false);
    input.set_options(&desired);
    assert_eq!(
        scope_calls(&system),
        vec![Call::SetInputScopes(FIRST, vec![28])]
    );
    input.set_options(&desired);
    assert!(scope_calls(&system).is_empty());
    assert_eq!(system.input_scopes.borrow().get(&FIRST), Some(&vec![28]));
}

#[test]
fn imm32_input_scope_failure_during_reentry_does_not_discard_the_newer_intent() {
    let system = FakeSystem::new();
    let owner = parent(FIRST);
    let input = active(&system, &owner);
    let weak = Rc::downgrade(&input);
    let callback_system = system.clone();
    system.refuse_input_scopes.set(true);
    *system.next_hook.borrow_mut() = Some((
        |call| matches!(call, Call::SetInputScopes(..)),
        Box::new(move || {
            callback_system.refuse_input_scopes.set(false);
            weak.upgrade()
                .unwrap()
                .set_options(&options(&[InputScopeNameValue::Url]));
        }),
    ));
    input.set_options(&options(&[InputScopeNameValue::Digits]));
    assert_eq!(
        scope_calls(&system),
        vec![
            Call::SetInputScopes(FIRST, vec![28]),
            Call::SetInputScopes(FIRST, vec![1])
        ]
    );
    assert_eq!(system.input_scopes.borrow().get(&FIRST), Some(&vec![1]));
}

#[test]
fn imm32_input_scope_owner_handoff_rejects_stale_options_reset_detach_and_drop() {
    let system = FakeSystem::new();
    let owner = parent(FIRST);
    let input = active(&system, &owner);
    let first = input.for_window(Rc::downgrade(&owner));
    let second = input.for_window(Rc::downgrade(&owner));
    first.set_options(&options(&[InputScopeNameValue::EmailSmtpAddress]));
    first.set_client(Some(Client::new()));
    second.set_options(&options(&[InputScopeNameValue::Digits]));
    // A direct Some-to-Some transition has no detached interval.
    second.set_client(Some(Client::new()));
    system.run_posted();
    system.take_calls();
    first.set_options(&options(&[InputScopeNameValue::Url]));
    first.reset();
    first.set_client(None);
    drop(first);
    system.run_posted();
    assert!(scope_calls(&system).is_empty());
    assert!(input.is_active());
    assert_eq!(system.input_scopes.borrow().get(&FIRST), Some(&vec![28]));
    second.set_client(None);
    assert_eq!(
        scope_calls(&system),
        vec![Call::SetInputScopes(FIRST, vec![0])]
    );
}

#[test]
fn imm32_input_scope_window_migration_clears_the_old_window_and_ignores_its_release() {
    let system = FakeSystem::new();
    let first = parent(FIRST);
    let second = parent(SECOND);
    let input = active(&system, &first);
    input.set_options(&options(&[InputScopeNameValue::EmailSmtpAddress]));
    system.take_calls();
    input.set_language_and_window(Rc::downgrade(&second), SECOND, JAPANESE);
    assert_eq!(
        scope_calls(&system),
        vec![
            Call::SetInputScopes(FIRST, vec![0]),
            Call::SetInputScopes(SECOND, vec![5])
        ]
    );
    input.release_window(FIRST, &Rc::downgrade(&first));
    assert!(scope_calls(&system).is_empty());
    assert_eq!(system.input_scopes.borrow().get(&SECOND), Some(&vec![5]));
    input.release_window(SECOND, &Rc::downgrade(&second));
    assert_eq!(
        scope_calls(&system),
        vec![Call::SetInputScopes(SECOND, vec![0])]
    );
}

#[test]
fn imm32_input_scope_reused_handle_is_cleared_only_by_its_current_parent_identity() {
    let system = FakeSystem::new();
    let first = parent(FIRST);
    let replacement = parent(FIRST);
    let input = active(&system, &first);
    input.set_options(&options(&[InputScopeNameValue::EmailSmtpAddress]));
    input.release_window(FIRST, &Rc::downgrade(&first));
    input.set_language_and_window(Rc::downgrade(&replacement), FIRST, JAPANESE);
    let site = input.for_window(Rc::downgrade(&replacement));
    site.set_options(&options(&[InputScopeNameValue::Digits]));
    site.set_client(Some(Client::new()));
    system.take_calls();
    input.release_window(FIRST, &Rc::downgrade(&first));
    assert!(scope_calls(&system).is_empty());
    assert_eq!(system.input_scopes.borrow().get(&FIRST), Some(&vec![28]));
    input.release_window(FIRST, &Rc::downgrade(&replacement));
    assert_eq!(
        scope_calls(&system),
        vec![Call::SetInputScopes(FIRST, vec![0])]
    );
}

#[test]
fn imm32_input_scope_reentrant_window_release_cannot_restore_retired_hints() {
    for delayed in [false, true] {
        let system = FakeSystem::new();
        system.complete_input_scopes_after_callback.set(delayed);
        let owner = parent(FIRST);
        let input = active(&system, &owner);
        let weak = Rc::downgrade(&input);
        let parent = Rc::downgrade(&owner);
        *system.next_hook.borrow_mut() = Some((
            |call| matches!(call, Call::SetInputScopes(..)),
            Box::new(move || {
                weak.upgrade().unwrap().release_window(FIRST, &parent);
            }),
        ));
        input.set_options(&options(&[InputScopeNameValue::EmailSmtpAddress]));
        system.run_posted();
        assert_eq!(
            scope_calls(&system),
            vec![
                Call::SetInputScopes(FIRST, vec![5]),
                Call::SetInputScopes(FIRST, vec![0]),
                Call::SetInputScopes(FIRST, vec![0])
            ],
            "native effect after callback: {delayed}",
        );
        assert!(!input.is_active());
        assert_eq!(input.hwnd(), 0);
        assert!(system.input_scopes.borrow().is_empty());
        input.set_options(&options(&[InputScopeNameValue::Digits]));
        assert!(scope_calls(&system).is_empty());
    }
}

#[test]
fn imm32_input_scope_reentrant_clear_preserves_a_replacement_window_owner() {
    let system = FakeSystem::new();
    let first = parent(FIRST);
    let second = parent(SECOND);
    let input = active(&system, &first);
    input.set_options(&options(&[InputScopeNameValue::EmailSmtpAddress]));
    system.take_calls();
    let site = input.for_window(Rc::downgrade(&second));
    site.set_options(&options(&[InputScopeNameValue::Digits]));
    let weak = Rc::downgrade(&input);
    let callback_site = site.clone();
    let callback_parent = Rc::downgrade(&second);
    *system.next_hook.borrow_mut() = Some((
        |call| matches!(call, Call::SetInputScopes(_, scopes) if scopes == &[0]),
        Box::new(move || {
            let input = weak.upgrade().unwrap();
            input.set_language_and_window(callback_parent, SECOND, JAPANESE);
            callback_site.set_client(Some(Client::new()));
        }),
    ));
    input.release_window(FIRST, &Rc::downgrade(&first));
    system.run_posted();
    assert_eq!(
        scope_calls(&system),
        vec![
            Call::SetInputScopes(FIRST, vec![0]),
            Call::SetInputScopes(SECOND, vec![28])
        ]
    );
    assert_eq!(input.hwnd(), SECOND);
    assert!(input.is_active());
    assert_eq!(system.input_scopes.borrow().get(&SECOND), Some(&vec![28]));
}

#[test]
fn imm32_input_scope_reentrant_handoff_during_set_keeps_new_options_on_the_recipient() {
    let system = FakeSystem::new();
    let first = parent(FIRST);
    let second = parent(SECOND);
    let input = active(&system, &first);
    let site = input.for_window(Rc::downgrade(&second));
    site.set_options(&options(&[InputScopeNameValue::Digits]));
    let callback_site = site.clone();
    let weak = Rc::downgrade(&input);
    let callback_parent = Rc::downgrade(&second);
    *system.next_hook.borrow_mut() = Some((
        |call| matches!(call, Call::SetInputScopes(..)),
        Box::new(move || {
            let input = weak.upgrade().unwrap();
            input.set_language_and_window(callback_parent, SECOND, JAPANESE);
            callback_site.set_client(Some(Client::new()));
        }),
    ));
    input.set_options(&options(&[InputScopeNameValue::EmailSmtpAddress]));
    system.run_posted();
    assert_eq!(
        scope_calls(&system),
        vec![
            Call::SetInputScopes(FIRST, vec![5]),
            Call::SetInputScopes(FIRST, vec![0]),
            Call::SetInputScopes(SECOND, vec![28])
        ]
    );
    assert_eq!(system.input_scopes.borrow().get(&SECOND), Some(&vec![28]));
    assert!(!system.input_scopes.borrow().contains_key(&FIRST));
}

#[test]
fn imm32_input_scope_failed_clear_remains_pending_until_a_later_retry() {
    let system = FakeSystem::new();
    let first = parent(FIRST);
    let second = parent(SECOND);
    let input = active(&system, &first);
    let desired = options(&[InputScopeNameValue::EmailSmtpAddress]);
    input.set_options(&desired);
    system.take_calls();
    system.refuse_input_scopes.set(true);
    input.set_language_and_window(Rc::downgrade(&second), SECOND, JAPANESE);
    assert_eq!(
        scope_calls(&system),
        vec![Call::SetInputScopes(FIRST, vec![0])]
    );
    assert!(!system.input_scopes.borrow().contains_key(&SECOND));
    system.refuse_input_scopes.set(false);
    input.set_options(&desired);
    assert_eq!(
        scope_calls(&system),
        vec![
            Call::SetInputScopes(FIRST, vec![0]),
            Call::SetInputScopes(SECOND, vec![5])
        ]
    );
    assert!(!system.input_scopes.borrow().contains_key(&FIRST));
}

#[test]
fn imm32_input_scope_transport_drop_clears_a_live_window_association() {
    let system = FakeSystem::new();
    let owner = parent(FIRST);
    let input = active(&system, &owner);
    input.set_options(&options(&[InputScopeNameValue::Digits]));
    system.take_calls();
    drop(input);
    assert_eq!(
        scope_calls(&system),
        vec![Call::SetInputScopes(FIRST, vec![0])]
    );
    assert!(system.input_scopes.borrow().is_empty());
}

#[cfg(windows)]
#[test]
fn imm32_input_scope_native_sdk_values_match_every_public_name() {
    use windows_sys::Win32::UI::TextServices as sdk;
    let expected = [
        sdk::IS_DEFAULT,
        sdk::IS_URL,
        sdk::IS_EMAIL_SMTPEMAILADDRESS,
        sdk::IS_PERSONALNAME_FULLNAME,
        sdk::IS_CURRENCY_AMOUNTANDSYMBOL,
        sdk::IS_CURRENCY_AMOUNT,
        sdk::IS_DATE_MONTH,
        sdk::IS_DATE_DAY,
        sdk::IS_DATE_YEAR,
        sdk::IS_DIGITS,
        sdk::IS_NUMBER,
        sdk::IS_PASSWORD,
        sdk::IS_TELEPHONE_FULLTELEPHONENUMBER,
        sdk::IS_TELEPHONE_COUNTRYCODE,
        sdk::IS_TELEPHONE_AREACODE,
        sdk::IS_TELEPHONE_LOCALNUMBER,
        sdk::IS_TIME_HOUR,
        sdk::IS_TIME_MINORSEC,
        sdk::IS_NUMBER_FULLWIDTH,
        sdk::IS_ALPHANUMERIC_HALFWIDTH,
        sdk::IS_ALPHANUMERIC_FULLWIDTH,
        sdk::IS_HIRAGANA,
        sdk::IS_KATAKANA_HALFWIDTH,
        sdk::IS_KATAKANA_FULLWIDTH,
        sdk::IS_HANJA,
        sdk::IS_HANGUL_HALFWIDTH,
        sdk::IS_HANGUL_FULLWIDTH,
        sdk::IS_SEARCH,
        sdk::IS_FORMULA,
        sdk::IS_SEARCH_INCREMENTAL,
        sdk::IS_CHINESE_HALFWIDTH,
        sdk::IS_CHINESE_FULLWIDTH,
        sdk::IS_NATIVE_SCRIPT,
        sdk::IS_TEXT,
        sdk::IS_CHAT,
        sdk::IS_NAME_OR_PHONENUMBER,
        sdk::IS_EMAILNAME_OR_ADDRESS,
        sdk::IS_MAPS,
        sdk::IS_NUMERIC_PASSWORD,
        sdk::IS_NUMERIC_PIN,
        sdk::IS_ALPHANUMERIC_PIN,
        sdk::IS_FORMULA_NUMBER,
        sdk::IS_CHAT_WITHOUT_EMOJI,
    ];
    assert_eq!(InputScopeNameValue::ALL.map(|value| value as i32), expected);
    assert_eq!(IS_PRIVATE, sdk::IS_PRIVATE);
}

#[cfg(windows)]
#[test]
fn imm32_input_scope_native_api_accepts_multiple_scopes_and_clears_before_destroy() {
    use crate::interop::unmanaged_methods as native;
    struct OwnedWindow {
        hwnd: isize,
        class_name: String,
    }
    impl Drop for OwnedWindow {
        fn drop(&mut self) {
            if self.hwnd != 0 {
                native::set_input_scopes(self.hwnd, &[0]);
                native::destroy_window(self.hwnd);
            }
            native::unregister_class(&self.class_name);
        }
    }
    let class_name = format!(
        "FerroUI.InputScope.Test.{}",
        native::get_current_thread_id()
    );
    let atom = native::register_class_ex(&class_name, 0, native::default_wnd_proc, 0);
    assert_ne!(
        atom,
        0,
        "RegisterClassEx failed: {}",
        native::get_last_error()
    );
    let window = OwnedWindow {
        hwnd: native::create_window_ex(0, atom, 0, 0, 0, 1, 1, 0),
        class_name,
    };
    assert_ne!(
        window.hwnd,
        0,
        "CreateWindowEx failed: {}",
        native::get_last_error()
    );
    assert!(!native::set_input_scopes(0, &[5]));
    assert!(!native::set_input_scopes(window.hwnd, &[]));
    assert!(native::set_input_scopes(window.hwnd, &[5, 28]));
    assert!(native::set_input_scopes(window.hwnd, &[28, 61]));
    assert!(native::set_input_scopes(window.hwnd, &[0]));
}
