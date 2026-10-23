//! Options can arrive before the lazy CoreText context exists. Keep their
//! latest value independently of that context and release all borrows before
//! invoking native setters, which can synchronously replace the input client.
use crate::interop::CoreTextInputScope;
use ferroui_base::input::text_input::{TextInputContentType, TextInputOptions};
use ferroui_base::input::InputScopeNameValue;
use std::cell::{Cell, RefCell};

#[derive(Default)]
pub(crate) struct CoreTextInputOptions {
    options: RefCell<TextInputOptions>,
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
    pub(crate) fn update(&self, options: &TextInputOptions) {
        if *self.options.borrow() != *options {
            self.options.replace(options.clone());
            self.revision.set(self.revision.get().wrapping_add(1));
        }
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
            let scope = native_scope(&self.options.borrow());
            set_scope(scope)?;
            if revision == self.revision.get() {
                return Ok(());
            }
            // A native callback changed the options while the previous setter
            // was in flight. Finish that call before applying the latest scope.
        }
    }
}

/// WinUI's XAML scope names and CoreText's scope names have several aliases.
/// Keep the conversion explicit: the public enums do not have identical names
/// or sets of variants even though their shared native values match.
fn native_scope(options: &TextInputOptions) -> CoreTextInputScope {
    use CoreTextInputScope as Native;
    use InputScopeNameValue as Scope;
    if let Some(scope) = options.effective_input_scope() {
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
        TextInputContentType::Email => Native::EmailAddress,
        TextInputContentType::Number => Native::Number,
        TextInputContentType::Password => Native::Password,
        TextInputContentType::Digits => Native::Digits,
        TextInputContentType::Pin => Native::PinNumeric,
        TextInputContentType::Name => Native::PersonalFullName,
        TextInputContentType::Url => Native::Url,
        TextInputContentType::Search => Native::Search,
        _ => Native::Text,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::rc::Rc;

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
            pending.update(&TextInputOptions {
                content_type: TextInputContentType::Search,
                input_scope_names: Some(Rc::from([scope])),
                ..Default::default()
            });
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
        let mut options = options(TextInputContentType::Email);
        options.input_scope_names = Some(Rc::from([]));
        assert_eq!(native_scope(&options), CoreTextInputScope::EmailAddress);
        options.input_scope_names = Some(Rc::from([
            InputScopeNameValue::Default,
            InputScopeNameValue::Hiragana,
        ]));
        assert_eq!(native_scope(&options), CoreTextInputScope::Default);
        options.input_scope_names = Some(Rc::from([
            InputScopeNameValue::Hiragana,
            InputScopeNameValue::Default,
        ]));
        assert_eq!(native_scope(&options), CoreTextInputScope::Hiragana);
    }

    #[test]
    fn core_text_input_scope_reentry_applies_updated_names_with_the_same_content_type() {
        let pending = CoreTextInputOptions::default();
        let mut options = options(TextInputContentType::Normal);
        options.input_scope_names = Some(Rc::from([InputScopeNameValue::Hiragana]));
        pending.update(&options);
        let calls = RefCell::new(Vec::new());
        pending
            .apply(|scope| {
                calls.borrow_mut().push(scope);
                if scope == CoreTextInputScope::Hiragana {
                    options.input_scope_names =
                        Some(Rc::from([InputScopeNameValue::KatakanaFullWidth]));
                    pending.update(&options);
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
        pending.update(&options(TextInputContentType::Url));
        pending.update(&options(TextInputContentType::Email));
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
        pending.update(&options(TextInputContentType::Email));
        let calls = RefCell::new(Vec::new());
        let native_scope = Cell::new(CoreTextInputScope::Text);
        pending
            .apply(|scope| {
                calls.borrow_mut().push(scope);
                if scope == CoreTextInputScope::EmailAddress {
                    pending.update(&options(TextInputContentType::Search));
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
        pending.update(&options(TextInputContentType::Digits));
        assert_eq!(
            pending.apply(|scope| {
                assert_eq!(scope, CoreTextInputScope::Digits);
                pending.update(&options(TextInputContentType::Number));
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
            pending.update(&options(content_type));
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
