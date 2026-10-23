//! HTML input-method hints for the persistent hidden text-input element.
//!
//! HTML inputmode/enterkeyhint are hints, not validation or input types:
//! https://html.spec.whatwg.org/multipage/interaction.html#input-modalities:-the-inputmode-attribute
//! One persistent textarea preserves UTF-16 selection and native composition.
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
    pub(crate) fn from_options(options: &TextInputOptions) -> Self {
        use InputScopeNameValue as Scope;
        let scope = options.effective_input_scope();
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

    fn scope(value: InputScopeNameValue) -> TextInputOptions {
        TextInputOptions {
            input_scope_names: Some(Rc::from([value])),
            ..Default::default()
        }
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
            assert_eq!(
                BrowserTextInputOptions::from_options(&scope(name)).input_mode,
                mode,
                "{name:?}"
            );
        }
    }

    #[test]
    fn browser_scope_order_and_empty_fallback_preserve_legacy_content_hints() {
        let mut options = TextInputOptions {
            content_type: TextInputContentType::Digits,
            ..Default::default()
        };
        assert_eq!(
            BrowserTextInputOptions::from_options(&options).input_mode,
            "tel"
        );
        options.input_scope_names = Some(Rc::from([]));
        assert_eq!(
            BrowserTextInputOptions::from_options(&options).input_mode,
            "tel"
        );
        options.input_scope_names = Some(Rc::from([
            InputScopeNameValue::EmailSmtpAddress,
            InputScopeNameValue::Digits,
        ]));
        assert_eq!(
            BrowserTextInputOptions::from_options(&options).input_mode,
            "email"
        );
        options.input_scope_names = Some(Rc::from([InputScopeNameValue::Default]));
        assert_eq!(
            BrowserTextInputOptions::from_options(&options).input_mode,
            "text"
        );
    }

    #[test]
    fn browser_sensitive_input_suppresses_correction_and_capitalization_without_changing_keyboard()
    {
        let mut options = scope(InputScopeNameValue::NumericPassword);
        options.show_suggestions = Some(true);
        options.auto_capitalization = true;
        options.uppercase = true;
        let mapped = BrowserTextInputOptions::from_options(&options);
        assert_eq!(mapped.input_mode, "numeric");
        assert!(mapped.sensitive);
        assert!(!mapped.suggestions);
        assert_eq!(mapped.capitalization, "none");
        options.input_scope_names = Some(Rc::from([InputScopeNameValue::EmailSmtpAddress]));
        options.is_sensitive = true;
        assert_eq!(
            BrowserTextInputOptions::from_options(&options).input_mode,
            "email"
        );
        assert!(!BrowserTextInputOptions::from_options(&options).suggestions);
    }

    #[test]
    fn browser_return_key_and_language_hints_are_explicit_and_owned() {
        let mut options = scope(InputScopeNameValue::Search);
        assert_eq!(
            BrowserTextInputOptions::from_options(&options).enter_key_hint,
            "search"
        );
        options.multiline = true;
        assert_eq!(
            BrowserTextInputOptions::from_options(&options).enter_key_hint,
            "enter"
        );
        options.return_key_type = TextInputReturnKeyType::Next;
        options.locale_hints = Some(Rc::from([String::new(), "pl-PL".into(), "en-US".into()]));
        let mapped = BrowserTextInputOptions::from_options(&options);
        assert_eq!(mapped.enter_key_hint, "next");
        options.locale_hints = None;
        assert_eq!(mapped.language, "pl-PL");
    }

    #[test]
    fn browser_capitalization_and_suggestions_use_framework_policy() {
        let mut options = TextInputOptions::default();
        assert!(!BrowserTextInputOptions::from_options(&options).suggestions);
        options.show_suggestions = Some(true);
        options.auto_capitalization = true;
        assert!(BrowserTextInputOptions::from_options(&options).suggestions);
        assert_eq!(
            BrowserTextInputOptions::from_options(&options).capitalization,
            "sentences"
        );
        options.uppercase = true;
        assert_eq!(
            BrowserTextInputOptions::from_options(&options).capitalization,
            "characters"
        );
        options.lowercase = true;
        assert_eq!(
            BrowserTextInputOptions::from_options(&options).capitalization,
            "none"
        );
    }
}
