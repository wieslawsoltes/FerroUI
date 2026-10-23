//! The public IBus ContentType property's purpose and hint mapping.
//!
//! Protocol values are pinned to IBus 1.5.32, commit
//! 0fd4e408a52d3d15179c643fd54d310aafd8875f, `src/ibustypes.h`.
//! These are input-method hints; they never validate or mask document text.

use super::ibus_enums::{IBusInputHints as Hints, IBusInputPurpose as Purpose};
use ferroui_base::input::text_input::{TextInputContentType, TextInputOptions};
use ferroui_base::input::InputScopeNameValue as Scope;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct IBusInputOptions {
    purpose: Purpose,
    hints: Hints,
}

impl IBusInputOptions {
    pub(crate) fn from_options(options: &TextInputOptions) -> Self {
        let scope = options.effective_input_scope();
        let purpose = scope.map_or_else(
            || match options.content_type {
                TextInputContentType::Normal
                | TextInputContentType::Social
                | TextInputContentType::Search => Purpose::FreeForm,
                TextInputContentType::Alpha => Purpose::Alpha,
                TextInputContentType::Digits => Purpose::Phone,
                TextInputContentType::Pin => Purpose::Pin,
                TextInputContentType::Number => Purpose::Number,
                TextInputContentType::Email => Purpose::Email,
                TextInputContentType::Url => Purpose::Url,
                TextInputContentType::Name => Purpose::Name,
                TextInputContentType::Password => Purpose::Password,
            },
            purpose_for_scope,
        );
        // A keyboard override must not erase independently configured privacy.
        let private = options.is_sensitive
            || matches!(
                options.content_type,
                TextInputContentType::Password | TextInputContentType::Pin
            )
            || scope.is_some_and(|scope| {
                matches!(
                    scope,
                    Scope::Password
                        | Scope::NumericPassword
                        | Scope::NumericPin
                        | Scope::AlphanumericPin
                )
            });
        let mut hints = Hints::empty();
        if private {
            hints |= Hints::PRIVATE | Hints::NO_SPELLCHECK;
        } else {
            match options.show_suggestions {
                Some(true) => hints |= Hints::SPELLCHECK | Hints::WORD_COMPLETION,
                Some(false) => hints |= Hints::NO_SPELLCHECK,
                None => {}
            }
        }
        if options.lowercase {
            hints |= Hints::LOWERCASE;
        }
        if options.uppercase {
            hints |= Hints::UPPERCASE_CHARS;
        }
        if options.auto_capitalization {
            hints |= Hints::UPPERCASE_SENTENCES;
        }
        match scope {
            Some(Scope::Chat) => hints |= Hints::EMOJI,
            Some(Scope::ChatWithoutEmoji) => hints |= Hints::NO_EMOJI,
            None if options.content_type == TextInputContentType::Social => hints |= Hints::EMOJI,
            _ => {}
        }
        Self { purpose, hints }
    }

    pub(crate) fn wire_value(self) -> (u32, u32) {
        (self.purpose as u32, self.hints.bits())
    }
}

fn purpose_for_scope(scope: Scope) -> Purpose {
    match scope {
        Scope::Url => Purpose::Url,
        Scope::EmailSmtpAddress | Scope::EmailNameOrAddress => Purpose::Email,
        Scope::PersonalFullName | Scope::NameOrPhoneNumber => Purpose::Name,
        Scope::CurrencyAmountAndSymbol
        | Scope::CurrencyAmount
        | Scope::Number
        | Scope::NumberFullWidth
        | Scope::FormulaNumber => Purpose::Number,
        Scope::DateMonthNumber
        | Scope::DateDayNumber
        | Scope::DateYear
        | Scope::Digits
        | Scope::TimeHour
        | Scope::TimeMinutesOrSeconds => Purpose::Digits,
        Scope::TelephoneNumber
        | Scope::TelephoneCountryCode
        | Scope::TelephoneAreaCode
        | Scope::TelephoneLocalNumber => Purpose::Phone,
        Scope::Password | Scope::AlphanumericPin => Purpose::Password,
        Scope::NumericPassword | Scope::NumericPin => Purpose::Pin,
        // IBus has no script or width selector. Alpha excludes digits and is
        // therefore not a substitute for an alphanumeric keyboard hint.
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
        | Scope::ChatWithoutEmoji => Purpose::FreeForm,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::rc::Rc;

    fn scoped(scope: Scope) -> TextInputOptions {
        TextInputOptions {
            input_scope_names: Some(Rc::from([scope])),
            ..Default::default()
        }
    }

    #[test]
    fn ibus_input_scope_protocol_values_match_pinned_purpose_and_hint_constants() {
        assert_eq!(
            [
                Purpose::FreeForm,
                Purpose::Alpha,
                Purpose::Digits,
                Purpose::Number,
                Purpose::Phone,
                Purpose::Url,
                Purpose::Email,
                Purpose::Name,
                Purpose::Password,
                Purpose::Pin,
                Purpose::Terminal
            ]
            .map(|value| value as u32),
            [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
        );
        assert_eq!(
            [
                Hints::SPELLCHECK,
                Hints::NO_SPELLCHECK,
                Hints::WORD_COMPLETION,
                Hints::LOWERCASE,
                Hints::UPPERCASE_CHARS,
                Hints::UPPERCASE_WORDS,
                Hints::UPPERCASE_SENTENCES,
                Hints::INHIBIT_OSK,
                Hints::VERTICAL_WRITING,
                Hints::EMOJI,
                Hints::NO_EMOJI,
                Hints::PRIVATE
            ]
            .map(|value| value.bits()),
            [1, 2, 4, 8, 16, 32, 64, 128, 256, 512, 1024, 2048],
        );
    }

    #[test]
    fn ibus_input_scope_maps_all_43_names_without_confusing_digits_phones_or_alphanumeric_text() {
        let purposes = [
            0, 5, 6, 7, 3, 3, 2, 2, 2, 2, 3, 8, 4, 4, 4, 4, 2, 2, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 7, 6, 0, 9, 9, 8, 3, 0,
        ];
        for (scope, expected) in Scope::ALL.into_iter().zip(purposes) {
            assert_eq!(
                IBusInputOptions::from_options(&scoped(scope))
                    .wire_value()
                    .0,
                expected,
                "{scope:?}"
            );
        }
    }

    #[test]
    fn ibus_input_scope_first_name_and_empty_names_preserve_explicit_and_legacy_purposes() {
        let mut options = TextInputOptions {
            content_type: TextInputContentType::Email,
            input_scope_names: Some(Rc::from([Scope::Digits, Scope::TelephoneNumber])),
            ..Default::default()
        };
        assert_eq!(
            IBusInputOptions::from_options(&options).wire_value(),
            (2, 0)
        );
        assert_eq!(
            options.input_scope_names.as_deref().unwrap(),
            &[Scope::Digits, Scope::TelephoneNumber]
        );
        options.input_scope_names = Some(Rc::from([Scope::Default]));
        assert_eq!(
            IBusInputOptions::from_options(&options).wire_value(),
            (0, 0)
        );
        options.input_scope_names = Some(Rc::from([]));
        assert_eq!(
            IBusInputOptions::from_options(&options).wire_value(),
            (6, 0)
        );
        options.input_scope_names = None;
        assert_eq!(
            IBusInputOptions::from_options(&options).wire_value(),
            (6, 0)
        );
        options.content_type = TextInputContentType::Digits;
        assert_eq!(
            IBusInputOptions::from_options(&options).wire_value(),
            (4, 0)
        );
        options.content_type = TextInputContentType::Alpha;
        assert_eq!(
            IBusInputOptions::from_options(&options).wire_value(),
            (1, 0)
        );
    }

    #[test]
    fn ibus_input_scope_privacy_survives_keyboard_overrides_and_suppresses_learning_hints() {
        for content_type in [TextInputContentType::Password, TextInputContentType::Pin] {
            let options = TextInputOptions {
                content_type,
                show_suggestions: Some(true),
                ..scoped(Scope::Text)
            };
            assert_eq!(
                IBusInputOptions::from_options(&options).wire_value(),
                (0, (Hints::PRIVATE | Hints::NO_SPELLCHECK).bits())
            );
        }
        for scope in [
            Scope::Password,
            Scope::AlphanumericPin,
            Scope::NumericPin,
            Scope::NumericPassword,
        ] {
            let options = TextInputOptions {
                show_suggestions: Some(true),
                ..scoped(scope)
            };
            let mapped = IBusInputOptions::from_options(&options);
            assert!(mapped.hints.contains(Hints::PRIVATE | Hints::NO_SPELLCHECK));
            assert!(!mapped
                .hints
                .intersects(Hints::SPELLCHECK | Hints::WORD_COMPLETION));
        }
        let options = TextInputOptions {
            is_sensitive: true,
            ..scoped(Scope::EmailSmtpAddress)
        };
        assert_eq!(
            IBusInputOptions::from_options(&options).wire_value(),
            (6, (Hints::PRIVATE | Hints::NO_SPELLCHECK).bits())
        );
    }

    #[test]
    fn ibus_input_scope_suggestion_case_and_emoji_hints_remain_independent() {
        let mut options = TextInputOptions {
            lowercase: true,
            uppercase: true,
            auto_capitalization: true,
            show_suggestions: Some(true),
            ..scoped(Scope::Chat)
        };
        assert_eq!(
            IBusInputOptions::from_options(&options).hints,
            Hints::SPELLCHECK
                | Hints::WORD_COMPLETION
                | Hints::LOWERCASE
                | Hints::UPPERCASE_CHARS
                | Hints::UPPERCASE_SENTENCES
                | Hints::EMOJI
        );
        options.input_scope_names = Some(Rc::from([Scope::ChatWithoutEmoji]));
        options.show_suggestions = Some(false);
        let hints = IBusInputOptions::from_options(&options).hints;
        assert!(hints.contains(Hints::NO_EMOJI | Hints::NO_SPELLCHECK));
        assert!(!hints.intersects(Hints::EMOJI | Hints::SPELLCHECK | Hints::WORD_COMPLETION));
    }
}
