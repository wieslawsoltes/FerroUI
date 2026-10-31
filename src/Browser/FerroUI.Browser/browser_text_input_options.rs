//! HTML input-method hints for the persistent hidden text-input element.
//!
//! HTML inputmode/enterkeyhint are hints, not validation or input types:
//! https://html.spec.whatwg.org/multipage/interaction.html#input-modalities:-the-inputmode-attribute
//! One persistent textarea preserves UTF-16 selection and native composition.
//!
//! Only clients of the exact-edit connection get these hints (decision D5 of
//! `docs/porting/winui-rich-text.md`); their input scope comes from the
//! connection, not from `TextInputOptions` (decision D8).
use ferroui_base::input::text_input::{
    TextInputContentType, TextInputOptions, TextInputReturnKeyType,
};
use ferroui_base::input::InputScopeNameValue;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct BrowserTextInputOptions {
    pub input_mode: &'static str,
    pub enter_key_hint: &'static str,
    pub capitalization: &'static str,
    pub suggestions: bool,
    pub sensitive: bool,
    pub language: String,
}

impl BrowserTextInputOptions {
    /// The hints of `options`, with `scope` the first input scope of the
    /// owner (`TextInputMethodConnection::effective_input_scope`).
    pub(crate) fn from_options(options: &TextInputOptions, scope: Option<InputScopeNameValue>) -> Self {
        use InputScopeNameValue as Scope;
        let input_mode = match scope {
            Some(
                Scope::TelephoneNumber
                | Scope::TelephoneCountryCode
                | Scope::TelephoneAreaCode
                | Scope::TelephoneLocalNumber,
            ) => "tel",
            Some(
                Scope::Digits
                | Scope::DateMonthNumber
                | Scope::DateDayNumber
                | Scope::DateYear
                | Scope::TimeHour
                | Scope::TimeMinutesOrSeconds
                | Scope::NumericPassword
                | Scope::NumericPin,
            ) => "numeric",
            Some(
                Scope::Number
                | Scope::NumberFullWidth
                | Scope::CurrencyAmount
                | Scope::CurrencyAmountAndSymbol
                | Scope::FormulaNumber,
            ) => "decimal",
            Some(Scope::EmailSmtpAddress | Scope::EmailNameOrAddress) => "email",
            Some(Scope::Url) => "url",
            Some(Scope::Search | Scope::SearchIncremental | Scope::Maps) => "search",
            Some(_) => "text",
            None => match options.content_type {
                TextInputContentType::Digits => "tel",
                TextInputContentType::Pin => "numeric",
                TextInputContentType::Number => "decimal",
                TextInputContentType::Email => "email",
                TextInputContentType::Url => "url",
                TextInputContentType::Search => "search",
                _ => "text",
            },
        };
        let sensitive = options.is_sensitive
            || matches!(
                scope,
                Some(
                    Scope::Password
                        | Scope::NumericPassword
                        | Scope::NumericPin
                        | Scope::AlphanumericPin
                )
            )
            || matches!(
                options.content_type,
                TextInputContentType::Password | TextInputContentType::Pin
            );
        let enter_key_hint = match options.return_key_type {
            TextInputReturnKeyType::Return => "enter",
            TextInputReturnKeyType::Done => "done",
            TextInputReturnKeyType::Go => "go",
            TextInputReturnKeyType::Next => "next",
            TextInputReturnKeyType::Previous => "previous",
            TextInputReturnKeyType::Search => "search",
            TextInputReturnKeyType::Send => "send",
            TextInputReturnKeyType::Default if options.multiline => "enter",
            TextInputReturnKeyType::Default if input_mode == "search" => "search",
            TextInputReturnKeyType::Default => "done",
        };
        Self {
            input_mode,
            enter_key_hint,
            capitalization: if sensitive || options.lowercase {
                "none"
            } else if options.uppercase {
                "characters"
            } else if options.auto_capitalization {
                "sentences"
            } else {
                "none"
            },
            // The adapter further gates this request on the current client's
            // transactional exact-edit capability before enabling DOM correction.
            suggestions: options.show_suggestions == Some(true) && !sensitive,
            sensitive,
            language: options
                .locale_hints
                .as_deref()
                .and_then(|locales| locales.iter().find(|locale| !locale.is_empty()))
                .cloned()
                .unwrap_or_default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::rc::Rc;

    fn map(options: &TextInputOptions, scope: Option<InputScopeNameValue>) -> BrowserTextInputOptions {
        BrowserTextInputOptions::from_options(options, scope)
    }

    #[test]
    fn browser_scope_distinguishes_numeric_telephone_decimal_and_text_keyboards() {
        use InputScopeNameValue as S;
        for (name, mode) in [
            (S::Digits, "numeric"),
            (S::TelephoneNumber, "tel"),
            (S::TelephoneCountryCode, "tel"),
            (S::NumericPin, "numeric"),
            (S::AlphanumericPin, "text"),
            (S::CurrencyAmount, "decimal"),
            (S::FormulaNumber, "decimal"),
            (S::EmailNameOrAddress, "email"),
            (S::Url, "url"),
            (S::SearchIncremental, "search"),
            (S::Hiragana, "text"),
            (S::ChatWithoutEmoji, "text"),
        ] {
            assert_eq!(map(&TextInputOptions::default(), Some(name)).input_mode, mode, "{name:?}");
        }
    }

    #[test]
    fn browser_scope_order_and_empty_fallback_preserve_legacy_content_hints() {
        let options = TextInputOptions { content_type: TextInputContentType::Digits, ..Default::default() };
        // No scope (also an empty list of scopes): the content type decides.
        assert_eq!(map(&options, None).input_mode, "tel");
        // The first scope wins over the content type.
        assert_eq!(map(&options, Some(InputScopeNameValue::EmailSmtpAddress)).input_mode, "email");
        assert_eq!(map(&options, Some(InputScopeNameValue::Default)).input_mode, "text");
    }

    #[test]
    fn browser_sensitive_input_suppresses_correction_and_capitalization_without_changing_keyboard() {
        let mut options = TextInputOptions {
            show_suggestions: Some(true),
            auto_capitalization: true,
            uppercase: true,
            ..Default::default()
        };
        let mapped = map(&options, Some(InputScopeNameValue::NumericPassword));
        assert_eq!(mapped.input_mode, "numeric");
        assert!(mapped.sensitive);
        assert!(!mapped.suggestions);
        assert_eq!(mapped.capitalization, "none");
        options.is_sensitive = true;
        assert_eq!(map(&options, Some(InputScopeNameValue::EmailSmtpAddress)).input_mode, "email");
        assert!(!map(&options, Some(InputScopeNameValue::EmailSmtpAddress)).suggestions);
    }

    #[test]
    fn browser_return_key_and_language_hints_are_explicit_and_owned() {
        let search = Some(InputScopeNameValue::Search);
        let mut options = TextInputOptions::default();
        assert_eq!(map(&options, search).enter_key_hint, "search");
        options.multiline = true;
        assert_eq!(map(&options, search).enter_key_hint, "enter");
        options.return_key_type = TextInputReturnKeyType::Next;
        options.locale_hints = Some(Rc::from([String::new(), "pl-PL".into(), "en-US".into()]));
        let mapped = map(&options, search);
        assert_eq!(mapped.enter_key_hint, "next");
        options.locale_hints = None;
        assert_eq!(mapped.language, "pl-PL");
    }

    #[test]
    fn browser_capitalization_and_suggestions_use_framework_policy() {
        let mut options = TextInputOptions::default();
        assert!(!map(&options, None).suggestions);
        options.show_suggestions = Some(true);
        options.auto_capitalization = true;
        assert!(map(&options, None).suggestions);
        assert_eq!(map(&options, None).capitalization, "sentences");
        options.uppercase = true;
        assert_eq!(map(&options, None).capitalization, "characters");
        options.lowercase = true;
        assert_eq!(map(&options, None).capitalization, "none");
    }
}
