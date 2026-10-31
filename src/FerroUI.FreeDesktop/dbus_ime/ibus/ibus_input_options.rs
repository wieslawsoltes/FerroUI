//! The purpose and hints of the IBus `ContentType` property (not in the reference, whose
//! `SetOptions` is a no-op for IBus). They are set only for a client of the exact-edit
//! connection (`docs/porting/winui-rich-text.md`, decision D5); its input scope comes through the
//! connection (decision D8).
//!
//! The values are those of IBus 1.5.32, commit 0fd4e408a52d3d15179c643fd54d310aafd8875f,
//! `src/ibustypes.h`. They are hints to the input method; they never validate or mask the text.

use super::ibus_enums::{IBusInputHints as Hints, IBusInputPurpose as Purpose};
use ferroui_base::input::text_input::{TextInputContentType, TextInputOptions};
use ferroui_base::input::InputScopeNameValue as Scope;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct IBusInputOptions {
    purpose: Purpose,
    hints: Hints,
}

impl IBusInputOptions {
    /// The content type of `options` and the first input scope of the owner (`scope`), which
    /// wins over the content type.
    pub(crate) fn from_options(options: &TextInputOptions, scope: Option<Scope>) -> Self {
        let purpose = scope.map_or_else(
            || match options.content_type {
                TextInputContentType::Normal | TextInputContentType::Social | TextInputContentType::Search => {
                    Purpose::FreeForm
                }
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
        // An input scope for the keyboard does not take away privacy asked for otherwise.
        let private = options.is_sensitive
            || matches!(options.content_type, TextInputContentType::Password | TextInputContentType::Pin)
            || matches!(scope, Some(Scope::Password | Scope::NumericPassword | Scope::NumericPin | Scope::AlphanumericPin));
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

    /// The value of the property: `(purpose, hints)`.
    pub(crate) fn wire_value(self) -> (u32, u32) {
        (self.purpose as u32, self.hints.bits())
    }
}

fn purpose_for_scope(scope: Scope) -> Purpose {
    match scope {
        Scope::Url => Purpose::Url,
        Scope::EmailSmtpAddress | Scope::EmailNameOrAddress => Purpose::Email,
        Scope::PersonalFullName | Scope::NameOrPhoneNumber => Purpose::Name,
        Scope::CurrencyAmountAndSymbol | Scope::CurrencyAmount | Scope::Number | Scope::NumberFullWidth | Scope::FormulaNumber => {
            Purpose::Number
        }
        Scope::DateMonthNumber
        | Scope::DateDayNumber
        | Scope::DateYear
        | Scope::Digits
        | Scope::TimeHour
        | Scope::TimeMinutesOrSeconds => Purpose::Digits,
        Scope::TelephoneNumber | Scope::TelephoneCountryCode | Scope::TelephoneAreaCode | Scope::TelephoneLocalNumber => {
            Purpose::Phone
        }
        Scope::Password | Scope::AlphanumericPin => Purpose::Password,
        Scope::NumericPassword | Scope::NumericPin => Purpose::Pin,
        // IBus has no script or width selector. Alpha excludes digits, so it does not stand
        // for an alphanumeric scope.
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

    fn wire(options: &TextInputOptions, scope: Option<Scope>) -> (u32, u32) {
        IBusInputOptions::from_options(options, scope).wire_value()
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
            0, 5, 6, 7, 3, 3, 2, 2, 2, 2, 3, 8, 4, 4, 4, 4, 2, 2, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 7, 6, 0,
            9, 9, 8, 3, 0,
        ];
        assert_eq!(Scope::ALL.len(), purposes.len());
        for (scope, expected) in Scope::ALL.into_iter().zip(purposes) {
            assert_eq!(wire(&TextInputOptions::default(), Some(scope)).0, expected, "{scope:?}");
        }
    }

    #[test]
    fn ibus_input_scope_wins_over_the_content_type_and_without_one_the_content_type_is_mapped() {
        let mut options = TextInputOptions { content_type: TextInputContentType::Email, ..Default::default() };
        assert_eq!(wire(&options, Some(Scope::Digits)), (2, 0));
        assert_eq!(wire(&options, Some(Scope::Default)), (0, 0));
        assert_eq!(wire(&options, None), (6, 0));
        options.content_type = TextInputContentType::Digits;
        assert_eq!(wire(&options, None), (4, 0));
        options.content_type = TextInputContentType::Alpha;
        assert_eq!(wire(&options, None), (1, 0));
    }

    #[test]
    fn ibus_input_scope_privacy_survives_keyboard_overrides_and_suppresses_learning_hints() {
        for content_type in [TextInputContentType::Password, TextInputContentType::Pin] {
            let options = TextInputOptions { content_type, show_suggestions: Some(true), ..Default::default() };
            assert_eq!(wire(&options, Some(Scope::Text)), (0, (Hints::PRIVATE | Hints::NO_SPELLCHECK).bits()));
        }
        for scope in [Scope::Password, Scope::AlphanumericPin, Scope::NumericPin, Scope::NumericPassword] {
            let options = TextInputOptions { show_suggestions: Some(true), ..Default::default() };
            let mapped = IBusInputOptions::from_options(&options, Some(scope));
            assert!(mapped.hints.contains(Hints::PRIVATE | Hints::NO_SPELLCHECK));
            assert!(!mapped.hints.intersects(Hints::SPELLCHECK | Hints::WORD_COMPLETION));
        }
        let options = TextInputOptions { is_sensitive: true, ..Default::default() };
        assert_eq!(wire(&options, Some(Scope::EmailSmtpAddress)), (6, (Hints::PRIVATE | Hints::NO_SPELLCHECK).bits()));
    }

    #[test]
    fn ibus_input_scope_suggestion_case_and_emoji_hints_remain_independent() {
        let mut options = TextInputOptions {
            lowercase: true,
            uppercase: true,
            auto_capitalization: true,
            show_suggestions: Some(true),
            ..Default::default()
        };
        assert_eq!(
            IBusInputOptions::from_options(&options, Some(Scope::Chat)).hints,
            Hints::SPELLCHECK
                | Hints::WORD_COMPLETION
                | Hints::LOWERCASE
                | Hints::UPPERCASE_CHARS
                | Hints::UPPERCASE_SENTENCES
                | Hints::EMOJI
        );
        options.show_suggestions = Some(false);
        let hints = IBusInputOptions::from_options(&options, Some(Scope::ChatWithoutEmoji)).hints;
        assert!(hints.contains(Hints::NO_EMOJI | Hints::NO_SPELLCHECK));
        assert!(!hints.intersects(Hints::EMOJI | Hints::SPELLCHECK | Hints::WORD_COMPLETION));
    }
}
