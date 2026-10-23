//! Semantic input hints supported by AppKit's NSTextContent protocol.
//!
//! AppKit exposes content meaning, not a WinUI keyboard type for every scope.
//! https://developer.apple.com/documentation/appkit/nstextcontent/contenttype
//! Correction/prediction still require a separate exact replacement bridge and
//! NSTextCheckingClient. This policy does not enable either service.

use ferroui_base::input::text_input::{TextInputContentType, TextInputOptions};
use ferroui_base::input::InputScopeNameValue;
use std::cell::Cell;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum NativeTextContentType {
    #[default]
    None,
    Url,
    EmailAddress,
    Name,
    TelephoneNumber,
}

impl NativeTextContentType {
    fn from_options(options: &TextInputOptions) -> Self {
        use InputScopeNameValue as Scope;
        let scope = options.effective_input_scope();
        let sensitive = options.is_sensitive
            || matches!(
                options.content_type,
                TextInputContentType::Password | TextInputContentType::Pin
            )
            || matches!(
                scope,
                Some(
                    Scope::Password
                        | Scope::NumericPassword
                        | Scope::NumericPin
                        | Scope::AlphanumericPin
                )
            );
        // Do not advertise semantic completion/AutoFill hints for sensitive
        // entry or when the owner explicitly disables suggestions. This does
        // not mask the document or change the user's keyboard input source.
        if sensitive || options.show_suggestions == Some(false) {
            return Self::None;
        }
        match scope {
            Some(scope) => match scope {
                Scope::Url => Self::Url,
                Scope::EmailSmtpAddress | Scope::EmailNameOrAddress => Self::EmailAddress,
                Scope::PersonalFullName => Self::Name,
                Scope::TelephoneNumber => Self::TelephoneNumber,
                // A component is not a whole telephone number or birthdate;
                // a PIN is not a one-time code. The remaining keyboard/script
                // scopes have no corresponding NSTextContent semantic type.
                Scope::Default
                | Scope::CurrencyAmountAndSymbol
                | Scope::CurrencyAmount
                | Scope::DateMonthNumber
                | Scope::DateDayNumber
                | Scope::DateYear
                | Scope::Digits
                | Scope::Number
                | Scope::Password
                | Scope::TelephoneCountryCode
                | Scope::TelephoneAreaCode
                | Scope::TelephoneLocalNumber
                | Scope::TimeHour
                | Scope::TimeMinutesOrSeconds
                | Scope::NumberFullWidth
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
                | Scope::NameOrPhoneNumber
                | Scope::Maps
                | Scope::NumericPassword
                | Scope::NumericPin
                | Scope::AlphanumericPin
                | Scope::FormulaNumber
                | Scope::ChatWithoutEmoji => Self::None,
            },
            None => match options.content_type {
                TextInputContentType::Name => Self::Name,
                TextInputContentType::Email => Self::EmailAddress,
                TextInputContentType::Url => Self::Url,
                // Legacy Digits does not assert telephone-number semantics.
                _ => Self::None,
            },
        }
    }
}

/// Requested policy and its last native delivery are separate: options may
/// arrive before the client, and the last client must clear the view's hint.
/// The guarded loop also makes a synchronous callback's newer policy win.
#[derive(Default)]
pub(crate) struct NativeTextInputOptions {
    requested: Cell<NativeTextContentType>,
    active: Cell<bool>,
    applied: Cell<Option<NativeTextContentType>>,
    applying: Cell<bool>,
}

impl NativeTextInputOptions {
    pub(crate) fn set_options(&self, options: &TextInputOptions) {
        self.requested
            .set(NativeTextContentType::from_options(options));
    }

    pub(crate) fn set_active(&self, active: bool) {
        self.active.set(active);
    }

    pub(crate) fn apply(&self, mut send: impl FnMut(NativeTextContentType)) {
        if self.applying.replace(true) {
            return;
        }
        struct Guard<'a>(&'a Cell<bool>);
        impl Drop for Guard<'_> {
            fn drop(&mut self) {
                self.0.set(false);
            }
        }
        let _guard = Guard(&self.applying);
        loop {
            let requested = if self.active.get() {
                self.requested.get()
            } else {
                NativeTextContentType::None
            };
            if self.applied.get() == Some(requested) {
                return;
            }
            self.applied.set(Some(requested));
            send(requested);
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
    fn native_input_scope_all_names_have_a_semantic_mapping_or_explicit_fallback() {
        use InputScopeNameValue as Scope;
        use NativeTextContentType as Content;
        let supported = [
            (Scope::Url, Content::Url),
            (Scope::EmailSmtpAddress, Content::EmailAddress),
            (Scope::EmailNameOrAddress, Content::EmailAddress),
            (Scope::PersonalFullName, Content::Name),
            (Scope::TelephoneNumber, Content::TelephoneNumber),
        ];
        assert_eq!(InputScopeNameValue::ALL.len(), 43);
        for name in InputScopeNameValue::ALL {
            let expected = supported
                .iter()
                .find(|(key, _)| *key == name)
                .map_or(Content::None, |(_, value)| *value);
            assert_eq!(Content::from_options(&scope(name)), expected, "{name:?}");
        }
    }

    #[test]
    fn native_input_scope_empty_names_fall_back_and_explicit_default_clears_legacy_hint() {
        let mut options = TextInputOptions {
            content_type: TextInputContentType::Email,
            ..Default::default()
        };
        assert_eq!(
            NativeTextContentType::from_options(&options),
            NativeTextContentType::EmailAddress
        );
        options.input_scope_names = Some(Rc::from([]));
        assert_eq!(
            NativeTextContentType::from_options(&options),
            NativeTextContentType::EmailAddress
        );
        options.input_scope_names = Some(Rc::from([
            InputScopeNameValue::Default,
            InputScopeNameValue::TelephoneNumber,
        ]));
        assert_eq!(
            NativeTextContentType::from_options(&options),
            NativeTextContentType::None
        );
        options.input_scope_names = Some(Rc::from([
            InputScopeNameValue::TelephoneNumber,
            InputScopeNameValue::EmailSmtpAddress,
        ]));
        assert_eq!(
            NativeTextContentType::from_options(&options),
            NativeTextContentType::TelephoneNumber
        );
    }

    #[test]
    fn native_input_scope_sensitive_and_suggestion_policy_override_semantic_hints() {
        let mut options = scope(InputScopeNameValue::EmailSmtpAddress);
        options.show_suggestions = Some(true);
        options.is_sensitive = true;
        assert_eq!(
            NativeTextContentType::from_options(&options),
            NativeTextContentType::None
        );
        options.is_sensitive = false;
        for legacy in [TextInputContentType::Password, TextInputContentType::Pin] {
            options.content_type = legacy;
            assert_eq!(
                NativeTextContentType::from_options(&options),
                NativeTextContentType::None
            );
        }
        options.content_type = TextInputContentType::Normal;
        options.show_suggestions = Some(false);
        assert_eq!(
            NativeTextContentType::from_options(&options),
            NativeTextContentType::None
        );
        for secret in [
            InputScopeNameValue::Password,
            InputScopeNameValue::NumericPassword,
            InputScopeNameValue::NumericPin,
            InputScopeNameValue::AlphanumericPin,
        ] {
            let mut options = scope(secret);
            options.content_type = TextInputContentType::Email;
            options.show_suggestions = Some(true);
            assert_eq!(
                NativeTextContentType::from_options(&options),
                NativeTextContentType::None
            );
        }
    }

    #[test]
    fn native_input_options_follow_active_client_and_do_not_repeat_unchanged_delivery() {
        let state = NativeTextInputOptions::default();
        let mut sent = Vec::new();
        state.set_options(&scope(InputScopeNameValue::Url));
        state.apply(|value| sent.push(value));
        state.set_active(true);
        state.apply(|value| sent.push(value));
        state.apply(|value| sent.push(value));
        state.set_active(false);
        state.apply(|value| sent.push(value));
        state.set_active(true);
        state.apply(|value| sent.push(value));
        assert_eq!(
            sent,
            [
                NativeTextContentType::None,
                NativeTextContentType::Url,
                NativeTextContentType::None,
                NativeTextContentType::Url,
            ]
        );
    }

    #[test]
    fn native_input_options_reentrant_policy_and_client_changes_keep_latest_owner() {
        let state = NativeTextInputOptions::default();
        let mut sent = Vec::new();
        state.set_active(true);
        state.set_options(&scope(InputScopeNameValue::Url));
        state.apply(|value| {
            sent.push(value);
            if value == NativeTextContentType::Url {
                state.set_options(&scope(InputScopeNameValue::PersonalFullName));
                state.apply(|_| panic!("nested delivery must be deferred"));
            } else if value == NativeTextContentType::Name {
                state.set_active(false);
                state.apply(|_| panic!("nested detach must be deferred"));
            }
        });
        assert_eq!(
            sent,
            [
                NativeTextContentType::Url,
                NativeTextContentType::Name,
                NativeTextContentType::None
            ]
        );
    }
}
