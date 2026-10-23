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
    let mut hint = ContentHint::NONE;
    let mut purpose = ContentPurpose::Normal;

    let scope = options.effective_input_scope();
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

    // These hints ask the input method to protect its candidates and learning;
    // they do not mask or validate the control's document.
    let password_scope = matches!(
        scope,
        Some(
            InputScopeNameValue::Password
                | InputScopeNameValue::NumericPassword
                | InputScopeNameValue::NumericPin
                | InputScopeNameValue::AlphanumericPin
        )
    );
    if password_scope || (scope.is_none() && options.content_type == TextInputContentType::Password) {
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
        Scope::CurrencyAmountAndSymbol
        | Scope::CurrencyAmount
        | Scope::Number
        | Scope::NumberFullWidth
        | Scope::FormulaNumber => ContentPurpose::Number,
        Scope::DateMonthNumber | Scope::DateDayNumber | Scope::DateYear => ContentPurpose::Date,
        Scope::TimeHour | Scope::TimeMinutesOrSeconds => ContentPurpose::Time,
        Scope::Digits => ContentPurpose::Digits,
        Scope::TelephoneNumber
        | Scope::TelephoneCountryCode
        | Scope::TelephoneAreaCode
        | Scope::TelephoneLocalNumber => ContentPurpose::Phone,
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
    // The protocol has no script/width selector. Half-width alphanumeric input
    // can request Latin; a full-width or non-Latin scope must not force Latin.
    let hint = if scope == Scope::AlphanumericHalfWidth { ContentHint::LATIN } else { ContentHint::NONE };
    (hint, purpose)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::rc::Rc;

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
                input_scope_names: Some(Rc::from([scope])),
                show_suggestions: Some(true),
                ..Default::default()
            };
            let (hint, purpose) = convert(&options);
            assert_eq!(purpose, expected, "{scope:?}");
            let secret = matches!(expected, ContentPurpose::Password | ContentPurpose::Pin);
            assert_eq!(hint.contains(ContentHint::HIDDEN_TEXT | ContentHint::SENSITIVE_DATA), secret);
            assert_eq!(hint.contains(ContentHint::COMPLETION | ContentHint::SPELLCHECK), !secret);
        }
    }

    #[test]
    fn input_scope_default_and_empty_names_have_distinct_fallback_and_privacy() {
        let mut options = TextInputOptions {
            content_type: TextInputContentType::Password,
            input_scope_names: Some(Rc::from([InputScopeNameValue::Default, InputScopeNameValue::NumericPin])),
            ..Default::default()
        };
        assert_eq!(convert(&options), (ContentHint::NONE, ContentPurpose::Normal));
        options.input_scope_names = Some(Rc::from([]));
        assert_eq!(
            convert(&options),
            (ContentHint::HIDDEN_TEXT | ContentHint::SENSITIVE_DATA, ContentPurpose::Password)
        );
        options.input_scope_names = Some(Rc::from([InputScopeNameValue::AlphanumericHalfWidth]));
        assert_eq!(convert(&options), (ContentHint::LATIN, ContentPurpose::Normal));
        for scope in [
            InputScopeNameValue::AlphanumericFullWidth,
            InputScopeNameValue::Hiragana,
            InputScopeNameValue::HangulFullWidth,
        ] {
            options.input_scope_names = Some(Rc::from([scope]));
            assert_eq!(convert(&options), (ContentHint::NONE, ContentPurpose::Normal));
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
