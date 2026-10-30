//! Maps the [`TextInputOptions`] of the framework onto the
//! `zwp_text_input_v3` `(content_hint, content_purpose)` pair (the port of
//! `TextInputOptionsConverter.cs`).
//!
//! The reference returns the enumerations of its generated bindings. The
//! bindings of the port exist on Linux only, so the two types are declared
//! here with the values of the protocol (`text-input-unstable-v3.xml`) and are
//! turned into the types of the bindings where a request is sent
//! (`server/transient/wayland_text_input_v3.rs`); the conversion is the same
//! on every system, which is where its tests run.

use ferroui_base::input::text_input::{TextInputContentType, TextInputOptions};
use ferroui_base::input::InputScopeNameValue;

bitflags::bitflags! {
    /// `zwp_text_input_v3.content_hint`: a bit field.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct ContentHint: u32 {
        const NONE = 0x0;
        const COMPLETION = 0x1;
        const SPELLCHECK = 0x2;
        const AUTO_CAPITALIZATION = 0x4;
        const LOWERCASE = 0x8;
        const UPPERCASE = 0x10;
        const TITLECASE = 0x20;
        const HIDDEN_TEXT = 0x40;
        const SENSITIVE_DATA = 0x80;
        const LATIN = 0x100;
        const MULTILINE = 0x200;
    }
}

/// `zwp_text_input_v3.content_purpose`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum ContentPurpose {
    Normal = 0,
    Alpha = 1,
    Digits = 2,
    Number = 3,
    Phone = 4,
    Url = 5,
    Email = 6,
    Name = 7,
    Password = 8,
    Pin = 9,
    Date = 10,
    Time = 11,
    Datetime = 12,
    Terminal = 13,
}

pub fn convert(options: &TextInputOptions) -> (ContentHint, ContentPurpose) {
    convert_with_input_scope(options, None)
}

/// [`convert`] with the first input scope of the owner of a client of the exact-edit connection,
/// which wins over the content type (not in the reference; decisions D5 and D8 of
/// `docs/porting/winui-rich-text.md`). Without a scope the result is that of [`convert`].
pub fn convert_with_input_scope(options: &TextInputOptions, scope: Option<InputScopeNameValue>) -> (ContentHint, ContentPurpose) {
    let mut hint = ContentHint::NONE;
    let mut purpose = ContentPurpose::Normal;

    if let Some(scope) = scope {
        (hint, purpose) = convert_scope(scope);
    } else {
        match options.content_type {
            TextInputContentType::Normal => {}
            TextInputContentType::Alpha => hint |= ContentHint::LATIN,
            TextInputContentType::Digits => purpose = ContentPurpose::Digits,
            TextInputContentType::Pin => purpose = ContentPurpose::Pin,
            TextInputContentType::Number => purpose = ContentPurpose::Number,
            TextInputContentType::Email => purpose = ContentPurpose::Email,
            TextInputContentType::Url => purpose = ContentPurpose::Url,
            TextInputContentType::Name => purpose = ContentPurpose::Name,
            TextInputContentType::Password => {
                purpose = ContentPurpose::Password;
                hint |= ContentHint::HIDDEN_TEXT | ContentHint::SENSITIVE_DATA;
            }
            TextInputContentType::Social | TextInputContentType::Search => {}
        }
    }

    if options.multiline {
        hint |= ContentHint::MULTILINE;
    }
    if options.is_sensitive {
        hint |= ContentHint::SENSITIVE_DATA;
    }
    if options.lowercase {
        hint |= ContentHint::LOWERCASE;
    }
    if options.uppercase {
        hint |= ContentHint::UPPERCASE;
    }
    if options.auto_capitalization {
        hint |= ContentHint::AUTO_CAPITALIZATION;
    }

    if options.show_suggestions == Some(true) {
        hint |= ContentHint::COMPLETION | ContentHint::SPELLCHECK;
    }

    // Password always overrides: even with ShowSuggestions=true, never reveal text. A secret
    // input scope does the same, and an input scope never takes away the privacy a Password
    // content type asks for (as for IBus); these hints protect the candidates and the learning of
    // the input method, they do not mask or validate the text of the control.
    let secret_scope = matches!(
        scope,
        Some(
            InputScopeNameValue::Password
                | InputScopeNameValue::NumericPassword
                | InputScopeNameValue::NumericPin
                | InputScopeNameValue::AlphanumericPin
        )
    );
    if secret_scope || options.content_type == TextInputContentType::Password {
        hint &= !(ContentHint::COMPLETION | ContentHint::SPELLCHECK);
        hint |= ContentHint::HIDDEN_TEXT | ContentHint::SENSITIVE_DATA;
    }

    (hint, purpose)
}

fn convert_scope(scope: InputScopeNameValue) -> (ContentHint, ContentPurpose) {
    use InputScopeNameValue as Scope;
    let purpose = match scope {
        Scope::Url => ContentPurpose::Url,
        Scope::EmailSmtpAddress | Scope::EmailNameOrAddress => ContentPurpose::Email,
        Scope::PersonalFullName | Scope::NameOrPhoneNumber => ContentPurpose::Name,
        Scope::CurrencyAmountAndSymbol | Scope::CurrencyAmount | Scope::Number | Scope::NumberFullWidth | Scope::FormulaNumber => {
            ContentPurpose::Number
        }
        Scope::DateMonthNumber | Scope::DateDayNumber | Scope::DateYear => ContentPurpose::Date,
        Scope::TimeHour | Scope::TimeMinutesOrSeconds => ContentPurpose::Time,
        Scope::Digits => ContentPurpose::Digits,
        Scope::TelephoneNumber | Scope::TelephoneCountryCode | Scope::TelephoneAreaCode | Scope::TelephoneLocalNumber => {
            ContentPurpose::Phone
        }
        Scope::Password | Scope::AlphanumericPin => ContentPurpose::Password,
        Scope::NumericPassword | Scope::NumericPin => ContentPurpose::Pin,
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
        | Scope::ChatWithoutEmoji => ContentPurpose::Normal,
    };
    // The protocol has no script or width selector. Half-width alphanumeric input can ask for
    // Latin; a full-width or non-Latin scope must not force it.
    let hint = if scope == Scope::AlphanumericHalfWidth { ContentHint::LATIN } else { ContentHint::NONE };
    (hint, purpose)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn input_scope_maps_components_phones_names_and_secrets_to_distinct_purposes() {
        use InputScopeNameValue as Scope;
        let cases = [
            (Scope::Digits, ContentPurpose::Digits),
            (Scope::DateMonthNumber, ContentPurpose::Date),
            (Scope::DateDayNumber, ContentPurpose::Date),
            (Scope::DateYear, ContentPurpose::Date),
            (Scope::TimeHour, ContentPurpose::Time),
            (Scope::TimeMinutesOrSeconds, ContentPurpose::Time),
            (Scope::TelephoneNumber, ContentPurpose::Phone),
            (Scope::TelephoneCountryCode, ContentPurpose::Phone),
            (Scope::TelephoneAreaCode, ContentPurpose::Phone),
            (Scope::TelephoneLocalNumber, ContentPurpose::Phone),
            (Scope::EmailNameOrAddress, ContentPurpose::Email),
            (Scope::EmailSmtpAddress, ContentPurpose::Email),
            (Scope::PersonalFullName, ContentPurpose::Name),
            (Scope::NameOrPhoneNumber, ContentPurpose::Name),
            (Scope::CurrencyAmountAndSymbol, ContentPurpose::Number),
            (Scope::FormulaNumber, ContentPurpose::Number),
            (Scope::NumericPassword, ContentPurpose::Pin),
            (Scope::NumericPin, ContentPurpose::Pin),
            (Scope::AlphanumericPin, ContentPurpose::Password),
            (Scope::Password, ContentPurpose::Password),
        ];
        for (scope, expected) in cases {
            let options = TextInputOptions {
                content_type: TextInputContentType::Url,
                show_suggestions: Some(true),
                ..Default::default()
            };
            let (hint, purpose) = convert_with_input_scope(&options, Some(scope));
            assert_eq!(purpose, expected, "{scope:?}");
            let secret = matches!(expected, ContentPurpose::Password | ContentPurpose::Pin);
            assert_eq!(hint.contains(ContentHint::HIDDEN_TEXT | ContentHint::SENSITIVE_DATA), secret);
            assert_eq!(hint.contains(ContentHint::COMPLETION | ContentHint::SPELLCHECK), !secret);
        }
    }

    #[test]
    fn an_input_scope_keeps_the_privacy_hints_of_a_password_content_type() {
        let options = TextInputOptions {
            content_type: TextInputContentType::Password,
            show_suggestions: Some(true),
            ..Default::default()
        };
        assert_eq!(
            convert_with_input_scope(&options, Some(InputScopeNameValue::Default)),
            (ContentHint::HIDDEN_TEXT | ContentHint::SENSITIVE_DATA, ContentPurpose::Normal)
        );
        let (hint, purpose) = convert_with_input_scope(&options, Some(InputScopeNameValue::Number));
        assert_eq!(purpose, ContentPurpose::Number);
        assert!(hint.contains(ContentHint::HIDDEN_TEXT | ContentHint::SENSITIVE_DATA));
        assert!(!hint.intersects(ContentHint::COMPLETION | ContentHint::SPELLCHECK));
    }

    #[test]
    fn without_an_input_scope_the_content_type_is_converted_as_upstream() {
        let mut options = TextInputOptions { content_type: TextInputContentType::Password, ..Default::default() };
        assert_eq!(
            convert_with_input_scope(&options, None),
            (ContentHint::HIDDEN_TEXT | ContentHint::SENSITIVE_DATA, ContentPurpose::Password)
        );
        assert_eq!(convert_with_input_scope(&options, None), convert(&options));
        options.content_type = TextInputContentType::Alpha;
        assert_eq!(
            convert_with_input_scope(&options, Some(InputScopeNameValue::AlphanumericHalfWidth)),
            (ContentHint::LATIN, ContentPurpose::Normal)
        );
        for scope in [InputScopeNameValue::AlphanumericFullWidth, InputScopeNameValue::Hiragana, InputScopeNameValue::HangulFullWidth] {
            assert_eq!(convert_with_input_scope(&options, Some(scope)), (ContentHint::NONE, ContentPurpose::Normal));
        }
    }

    #[test]
    fn default_is_none_normal() {
        let (h, p) = convert(&TextInputOptions::default());
        assert_eq!(h, ContentHint::NONE);
        assert_eq!(p, ContentPurpose::Normal);
    }

    #[test]
    fn content_type_maps_purpose() {
        let cases = [
            (TextInputContentType::Digits, ContentPurpose::Digits),
            (TextInputContentType::Pin, ContentPurpose::Pin),
            (TextInputContentType::Number, ContentPurpose::Number),
            (TextInputContentType::Email, ContentPurpose::Email),
            (TextInputContentType::Url, ContentPurpose::Url),
            (TextInputContentType::Name, ContentPurpose::Name),
        ];
        for (content_type, expected) in cases {
            let (_, p) = convert(&TextInputOptions { content_type, ..Default::default() });
            assert_eq!(p, expected);
        }
    }

    #[test]
    fn alpha_adds_latin_hint() {
        let (h, _) = convert(&TextInputOptions { content_type: TextInputContentType::Alpha, ..Default::default() });
        assert!(h.contains(ContentHint::LATIN));
    }

    #[test]
    fn password_forces_hidden_and_sensitive_even_with_show_suggestions() {
        let (h, p) = convert(&TextInputOptions {
            content_type: TextInputContentType::Password,
            show_suggestions: Some(true),
            ..Default::default()
        });
        assert_eq!(p, ContentPurpose::Password);
        assert!(h.contains(ContentHint::HIDDEN_TEXT));
        assert!(h.contains(ContentHint::SENSITIVE_DATA));
        assert!(!h.contains(ContentHint::COMPLETION));
        assert!(!h.contains(ContentHint::SPELLCHECK));
    }

    #[test]
    fn flags_compose_into_hint() {
        let (h, _) = convert(&TextInputOptions {
            multiline: true,
            is_sensitive: true,
            lowercase: true,
            uppercase: true,
            auto_capitalization: true,
            show_suggestions: Some(true),
            ..Default::default()
        });
        assert!(h.contains(ContentHint::MULTILINE));
        assert!(h.contains(ContentHint::SENSITIVE_DATA));
        assert!(h.contains(ContentHint::LOWERCASE));
        assert!(h.contains(ContentHint::UPPERCASE));
        assert!(h.contains(ContentHint::AUTO_CAPITALIZATION));
        assert!(h.contains(ContentHint::COMPLETION));
        assert!(h.contains(ContentHint::SPELLCHECK));
    }

    #[test]
    fn show_suggestions_false_omits_completion() {
        let (h, _) = convert(&TextInputOptions { show_suggestions: Some(false), ..Default::default() });
        assert!(!h.contains(ContentHint::COMPLETION));
        assert!(!h.contains(ContentHint::SPELLCHECK));
    }
}
