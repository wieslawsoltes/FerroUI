//! The input scope of the CoreText edit context of a client of the exact-edit
//! connection (`docs/porting/winui-rich-text.md`, decisions D5 and D8).
//!
//! Options can arrive before the lazy CoreText context exists. Keep their
//! latest value independently of that context and release all borrows before
//! invoking native setters, which can synchronously replace the input client.
//!
//! A client without the connection keeps the mapping of the reference
//! ([`legacy_scope`]), applied by the adapter as the reference applies it.
use crate::interop::CoreTextInputScope;
use ferroui_base::input::text_input::{TextInputContentType, TextInputOptions};
use ferroui_base::input::InputScopeNameValue;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// The options of the owner and its input scopes, which a connection client's
/// owner gives through the connection (`TextInputOptions.InputScopeNames`,
/// decision D8: not a field of `TextInputOptions` within 0.4).
#[derive(Clone, Default, PartialEq)]
struct PendingOptions {
    options: TextInputOptions,
    input_scope_names: Option<Rc<[InputScopeNameValue]>>,
}

#[derive(Default)]
pub(crate) struct CoreTextInputOptions {
    options: RefCell<PendingOptions>,
    revision: Cell<u64>,
    applying: Cell<bool>,
}

struct ApplyingGuard<'a>(&'a Cell<bool>);

impl Drop for ApplyingGuard<'_> {
    fn drop(&mut self) {
        self.0.set(false);
    }
}

impl CoreTextInputOptions {
    pub(crate) fn update(
        &self,
        options: &TextInputOptions,
        input_scope_names: Option<Rc<[InputScopeNameValue]>>,
    ) {
        let pending = PendingOptions {
            options: options.clone(),
            input_scope_names,
        };
        if *self.options.borrow() != pending {
            self.options.replace(pending);
            self.revision.set(self.revision.get().wrapping_add(1));
        }
    }

    /// The input scopes of a newly installed connection client, with the
    /// options the manager published for its owner before installing it.
    pub(crate) fn update_input_scope_names(
        &self,
        input_scope_names: Option<Rc<[InputScopeNameValue]>>,
    ) {
        let options = self.options.borrow().options.clone();
        self.update(&options, input_scope_names);
    }

    pub(crate) fn apply<E>(
        &self,
        mut set_scope: impl FnMut(CoreTextInputScope) -> Result<(), E>,
    ) -> Result<(), E> {
        if self.applying.replace(true) {
            return Ok(());
        }
        let _guard = ApplyingGuard(&self.applying);
        loop {
            let revision = self.revision.get();
            let scope = {
                let pending = self.options.borrow();
                native_scope(&pending.options, pending.input_scope_names.as_deref())
            };
            set_scope(scope)?;
            if revision == self.revision.get() {
                return Ok(());
            }
            // A native callback changed the options while the previous setter
            // was in flight. Finish that call before applying the latest scope.
        }
    }
}

/// The scope of the reference for a client without the connection: the
/// content types it names, and `Text` for every other one (`Pin` and `Name`
/// included).
pub(crate) fn legacy_scope(content_type: TextInputContentType) -> CoreTextInputScope {
    match content_type {
        TextInputContentType::Email => CoreTextInputScope::EmailAddress,
        TextInputContentType::Number => CoreTextInputScope::Number,
        TextInputContentType::Password => CoreTextInputScope::Password,
        TextInputContentType::Digits => CoreTextInputScope::Digits,
        TextInputContentType::Url => CoreTextInputScope::Url,
        TextInputContentType::Search => CoreTextInputScope::Search,
        _ => CoreTextInputScope::Text,
    }
}

/// WinUI's XAML scope names and CoreText's scope names have several aliases.
/// Keep the conversion explicit: the public enums do not have identical names
/// or sets of variants even though their shared native values match.
///
/// The first input scope of the owner wins (as `TextInputMethodConnection::effective_input_scope`);
/// without one, the content type maps as [`legacy_scope`] does, with `Pin` and
/// `Name` given their own native scopes (an addition for connection clients).
fn native_scope(
    options: &TextInputOptions,
    input_scope_names: Option<&[InputScopeNameValue]>,
) -> CoreTextInputScope {
    use CoreTextInputScope as Native;
    use InputScopeNameValue as Scope;
    if let Some(scope) = input_scope_names.and_then(|names| names.first().copied()) {
        return match scope {
            Scope::Default => Native::Default,
            Scope::Url => Native::Url,
            Scope::EmailSmtpAddress => Native::EmailAddress,
            Scope::PersonalFullName => Native::PersonalFullName,
            Scope::CurrencyAmountAndSymbol => Native::CurrencyAmountAndSymbol,
            Scope::CurrencyAmount => Native::CurrencyAmount,
            Scope::DateMonthNumber => Native::DateMonth,
            Scope::DateDayNumber => Native::DateDay,
            Scope::DateYear => Native::DateYear,
            Scope::Digits => Native::Digits,
            Scope::Number => Native::Number,
            Scope::Password => Native::Password,
            Scope::TelephoneNumber => Native::TelephoneNumber,
            Scope::TelephoneCountryCode => Native::TelephoneCountryCode,
            Scope::TelephoneAreaCode => Native::TelephoneAreaCode,
            Scope::TelephoneLocalNumber => Native::TelephoneLocalNumber,
            Scope::TimeHour => Native::TimeHour,
            Scope::TimeMinutesOrSeconds => Native::TimeMinuteOrSecond,
            Scope::NumberFullWidth => Native::NumberFullWidth,
            Scope::AlphanumericHalfWidth => Native::AlphanumericHalfWidth,
            Scope::AlphanumericFullWidth => Native::AlphanumericFullWidth,
            Scope::Hiragana => Native::Hiragana,
            Scope::KatakanaHalfWidth => Native::KatakanaHalfWidth,
            Scope::KatakanaFullWidth => Native::KatakanaFullWidth,
            Scope::Hanja => Native::Hanja,
            Scope::HangulHalfWidth => Native::HangulHalfWidth,
            Scope::HangulFullWidth => Native::HangulFullWidth,
            Scope::Search => Native::Search,
            Scope::Formula => Native::Formula,
            Scope::SearchIncremental => Native::SearchIncremental,
            Scope::ChineseHalfWidth => Native::ChineseHalfWidth,
            Scope::ChineseFullWidth => Native::ChineseFullWidth,
            Scope::NativeScript => Native::NativeScript,
            Scope::Text => Native::Text,
            Scope::Chat => Native::Chat,
            Scope::NameOrPhoneNumber => Native::NameOrPhoneNumber,
            Scope::EmailNameOrAddress => Native::EmailUserNameOrAddress,
            Scope::Maps => Native::Maps,
            Scope::NumericPassword => Native::PasswordNumeric,
            Scope::NumericPin => Native::PinNumeric,
            Scope::AlphanumericPin => Native::PinAlphanumeric,
            Scope::FormulaNumber => Native::FormulaNumber,
            Scope::ChatWithoutEmoji => Native::ChatWithoutEmoji,
        };
    }

    // Empty or absent Names preserve the established FerroUI content hints.
    match options.content_type {
        TextInputContentType::Pin => Native::PinNumeric,
        TextInputContentType::Name => Native::PersonalFullName,
        content_type => legacy_scope(content_type),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn options(content_type: TextInputContentType) -> TextInputOptions {
        TextInputOptions {
            content_type,
            ..TextInputOptions::default()
        }
    }

    #[test]
    fn core_text_input_scope_preserves_all_43_native_values() {
        for scope in InputScopeNameValue::ALL {
            let pending = CoreTextInputOptions::default();
            pending.update(&options(TextInputContentType::Search), Some(Rc::from([scope])));
            pending
                .apply(|native| {
                    assert_eq!(native.0, scope as i32, "{scope:?}");
                    Ok::<_, ()>(())
                })
                .unwrap();
        }
    }

    #[test]
    fn core_text_input_scope_distinguishes_empty_names_from_explicit_default() {
        let options = options(TextInputContentType::Email);
        assert_eq!(native_scope(&options, Some(&[])), CoreTextInputScope::EmailAddress);
        assert_eq!(
            native_scope(
                &options,
                Some(&[InputScopeNameValue::Default, InputScopeNameValue::Hiragana])
            ),
            CoreTextInputScope::Default
        );
        assert_eq!(
            native_scope(
                &options,
                Some(&[InputScopeNameValue::Hiragana, InputScopeNameValue::Default])
            ),
            CoreTextInputScope::Hiragana
        );
    }

    #[test]
    fn core_text_input_scope_reentry_applies_updated_names_with_the_same_content_type() {
        let pending = CoreTextInputOptions::default();
        let options = options(TextInputContentType::Normal);
        pending.update(&options, Some(Rc::from([InputScopeNameValue::Hiragana])));
        let calls = RefCell::new(Vec::new());
        pending
            .apply(|scope| {
                calls.borrow_mut().push(scope);
                if scope == CoreTextInputScope::Hiragana {
                    pending.update(
                        &options,
                        Some(Rc::from([InputScopeNameValue::KatakanaFullWidth])),
                    );
                }
                Ok::<_, ()>(())
            })
            .unwrap();
        assert_eq!(
            *calls.borrow(),
            [
                CoreTextInputScope::Hiragana,
                CoreTextInputScope::KatakanaFullWidth
            ]
        );
    }

    #[test]
    fn core_text_first_context_uses_the_last_options_received_before_creation() {
        let pending = CoreTextInputOptions::default();
        pending.update(&options(TextInputContentType::Url), None);
        pending.update(&options(TextInputContentType::Email), None);
        let native_calls = RefCell::new(Vec::new());
        // There is no native context when the manager first calls set_options.
        // Its first client later creates one and applies the retained options.
        pending
            .apply(|scope| {
                native_calls.borrow_mut().push(scope);
                Ok::<_, ()>(())
            })
            .unwrap();
        assert_eq!(*native_calls.borrow(), [CoreTextInputScope::EmailAddress]);
    }

    #[test]
    fn core_text_options_reentrant_replacement_wins_after_the_old_native_setter() {
        let pending = CoreTextInputOptions::default();
        pending.update(&options(TextInputContentType::Email), None);
        let calls = RefCell::new(Vec::new());
        let native_scope = Cell::new(CoreTextInputScope::Text);
        pending
            .apply(|scope| {
                calls.borrow_mut().push(scope);
                if scope == CoreTextInputScope::EmailAddress {
                    pending.update(&options(TextInputContentType::Search), None);
                    pending
                        .apply(|_| -> Result<(), ()> {
                            panic!("a nested setter must wait for the old native call")
                        })
                        .unwrap();
                }
                // The old native setter completes after its reentrant callback.
                native_scope.set(scope);
                Ok::<_, ()>(())
            })
            .unwrap();
        assert_eq!(
            *calls.borrow(),
            [CoreTextInputScope::EmailAddress, CoreTextInputScope::Search]
        );
        assert_eq!(native_scope.get(), CoreTextInputScope::Search);
    }

    #[test]
    fn core_text_failed_option_application_preserves_the_latest_value_for_retry() {
        let pending = CoreTextInputOptions::default();
        pending.update(&options(TextInputContentType::Digits), None);
        assert_eq!(
            pending.apply(|scope| {
                assert_eq!(scope, CoreTextInputScope::Digits);
                pending.update(&options(TextInputContentType::Number), None);
                Err::<(), _>("native failure")
            }),
            Err("native failure")
        );
        let native_scope = Cell::new(CoreTextInputScope::Text);
        pending
            .apply(|scope| {
                native_scope.set(scope);
                Ok::<_, ()>(())
            })
            .unwrap();
        assert_eq!(native_scope.get(), CoreTextInputScope::Number);
    }

    /// A client without the connection keeps the mapping of the reference
    /// (`WinUITextInputMethod.SetOptions`): `Pin`, `Name` and the content types it
    /// does not name are `Text` (decision D5).
    #[test]
    fn legacy_content_types_map_as_the_reference() {
        let cases = [
            (TextInputContentType::Normal, CoreTextInputScope::Text),
            (TextInputContentType::Alpha, CoreTextInputScope::Text),
            (TextInputContentType::Digits, CoreTextInputScope::Digits),
            (TextInputContentType::Pin, CoreTextInputScope::Text),
            (TextInputContentType::Number, CoreTextInputScope::Number),
            (TextInputContentType::Email, CoreTextInputScope::EmailAddress),
            (TextInputContentType::Url, CoreTextInputScope::Url),
            (TextInputContentType::Name, CoreTextInputScope::Text),
            (TextInputContentType::Password, CoreTextInputScope::Password),
            (TextInputContentType::Social, CoreTextInputScope::Text),
            (TextInputContentType::Search, CoreTextInputScope::Search),
        ];
        for (content_type, expected) in cases {
            assert_eq!(legacy_scope(content_type), expected, "{content_type:?}");
        }
    }

    /// The scopes of a connection client installed after the manager published
    /// its options apply with those options.
    #[test]
    fn input_scope_names_of_a_new_connection_client_keep_the_published_options() {
        let pending = CoreTextInputOptions::default();
        pending.update(&options(TextInputContentType::Pin), None);
        let received = Cell::new(None);
        let apply = || {
            pending
                .apply(|scope| {
                    received.set(Some(scope));
                    Ok::<_, ()>(())
                })
                .unwrap()
        };
        apply();
        assert_eq!(received.get(), Some(CoreTextInputScope::PinNumeric));
        pending.update_input_scope_names(Some(Rc::from([InputScopeNameValue::Hiragana])));
        apply();
        assert_eq!(received.get(), Some(CoreTextInputScope::Hiragana));
        pending.update_input_scope_names(None);
        apply();
        assert_eq!(received.get(), Some(CoreTextInputScope::PinNumeric));
    }

    /// For a connection client, `Pin` and `Name` have native scopes of their own.
    #[test]
    fn core_text_content_types_preserve_native_name_and_numeric_pin_semantics() {
        // These are keyboard/input-processor hints; the adapter does not add
        // text validation or impose the native scope's typical PIN length.
        let cases = [
            (TextInputContentType::Normal, CoreTextInputScope::Text),
            (TextInputContentType::Alpha, CoreTextInputScope::Text),
            (TextInputContentType::Digits, CoreTextInputScope::Digits),
            (TextInputContentType::Pin, CoreTextInputScope::PinNumeric),
            (TextInputContentType::Number, CoreTextInputScope::Number),
            (
                TextInputContentType::Email,
                CoreTextInputScope::EmailAddress,
            ),
            (TextInputContentType::Url, CoreTextInputScope::Url),
            (
                TextInputContentType::Name,
                CoreTextInputScope::PersonalFullName,
            ),
            (TextInputContentType::Password, CoreTextInputScope::Password),
            (TextInputContentType::Social, CoreTextInputScope::Text),
            (TextInputContentType::Search, CoreTextInputScope::Search),
        ];
        for (content_type, expected) in cases {
            let pending = CoreTextInputOptions::default();
            pending.update(&options(content_type), None);
            let received = Cell::new(None);
            pending
                .apply(|scope| {
                    received.set(Some(scope));
                    Ok::<_, ()>(())
                })
                .unwrap();
            assert_eq!(received.get(), Some(expected), "{content_type:?}");
        }
    }
}
