//! Semantic input hints supported by AppKit's NSTextContent protocol, for a
//! client of the exact-edit connection only (`docs/porting/winui-rich-text.md`,
//! decisions D5 and D8). Upstream's `SetOptions` is a no-op, which every other
//! client keeps: no hint is ever sent for it.
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
    /// The hint of the options and the first input scope of the owner of a
    /// connection client (`TextInputMethodConnection::effective_input_scope`).
    pub(crate) fn from_options(options: &TextInputOptions, scope: Option<InputScopeNameValue>) -> Self {
        use InputScopeNameValue as Scope;
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
///
/// The view starts without a hint, so nothing is delivered until a connection
/// client asks for one: a legacy client never reaches the native interface.
#[derive(Default)]
pub(crate) struct NativeTextInputOptions {
    requested: Cell<NativeTextContentType>,
    active: Cell<bool>,
    applied: Cell<NativeTextContentType>,
    applying: Cell<bool>,
}

impl NativeTextInputOptions {
    pub(crate) fn set_requested(&self, requested: NativeTextContentType) {
        self.requested.set(requested);
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
            if self.applied.get() == requested {
                return;
            }
            self.applied.set(requested);
            send(requested);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use InputScopeNameValue as Scope;

    fn of(options: &TextInputOptions, scope: Option<InputScopeNameValue>) -> NativeTextContentType {
        NativeTextContentType::from_options(options, scope)
    }

    fn scoped(scope: InputScopeNameValue) -> NativeTextContentType {
        of(&TextInputOptions::default(), Some(scope))
    }

    #[test]
    fn native_input_scope_all_names_have_a_semantic_mapping_or_explicit_fallback() {
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
            assert_eq!(scoped(name), expected, "{name:?}");
        }
    }

    #[test]
    fn native_input_scope_absent_falls_back_and_an_explicit_default_clears_the_content_type_hint() {
        let options = TextInputOptions {
            content_type: TextInputContentType::Email,
            ..Default::default()
        };
        // Without a scope (none set, or an empty list) the content type decides.
        assert_eq!(of(&options, None), NativeTextContentType::EmailAddress);
        assert_eq!(of(&options, Some(Scope::Default)), NativeTextContentType::None);
        assert_eq!(of(&options, Some(Scope::TelephoneNumber)), NativeTextContentType::TelephoneNumber);
        for (content_type, expected) in [
            (TextInputContentType::Name, NativeTextContentType::Name),
            (TextInputContentType::Url, NativeTextContentType::Url),
            (TextInputContentType::Digits, NativeTextContentType::None),
            (TextInputContentType::Normal, NativeTextContentType::None),
        ] {
            let options = TextInputOptions { content_type, ..Default::default() };
            assert_eq!(of(&options, None), expected, "{content_type:?}");
        }
    }

    #[test]
    fn native_input_scope_sensitive_and_suggestion_policy_override_semantic_hints() {
        let mut options = TextInputOptions {
            show_suggestions: Some(true),
            is_sensitive: true,
            ..Default::default()
        };
        assert_eq!(of(&options, Some(Scope::EmailSmtpAddress)), NativeTextContentType::None);
        options.is_sensitive = false;
        for legacy in [TextInputContentType::Password, TextInputContentType::Pin] {
            options.content_type = legacy;
            assert_eq!(of(&options, Some(Scope::EmailSmtpAddress)), NativeTextContentType::None);
        }
        options.content_type = TextInputContentType::Normal;
        options.show_suggestions = Some(false);
        assert_eq!(of(&options, Some(Scope::EmailSmtpAddress)), NativeTextContentType::None);
        for secret in [Scope::Password, Scope::NumericPassword, Scope::NumericPin, Scope::AlphanumericPin] {
            let options = TextInputOptions {
                content_type: TextInputContentType::Email,
                show_suggestions: Some(true),
                ..Default::default()
            };
            assert_eq!(of(&options, Some(secret)), NativeTextContentType::None);
        }
    }

    #[test]
    fn native_input_options_send_nothing_until_a_hint_is_requested() {
        // The legacy path (decision D5): inactive, or active without a hint, never reaches
        // the native interface, whose view starts without a content type.
        let state = NativeTextInputOptions::default();
        let mut sent = Vec::new();
        state.apply(|value| sent.push(value));
        state.set_active(true);
        state.apply(|value| sent.push(value));
        state.set_active(false);
        state.set_requested(NativeTextContentType::Url);
        state.apply(|value| sent.push(value));
        assert!(sent.is_empty());
    }

    #[test]
    fn native_input_options_follow_active_client_and_do_not_repeat_unchanged_delivery() {
        let state = NativeTextInputOptions::default();
        let mut sent = Vec::new();
        state.set_requested(NativeTextContentType::Url);
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
        state.set_requested(NativeTextContentType::Url);
        state.apply(|value| {
            sent.push(value);
            if value == NativeTextContentType::Url {
                state.set_requested(NativeTextContentType::Name);
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
