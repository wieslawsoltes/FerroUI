//! Input scope names from WinUI's pinned Microsoft.UI.Xaml.Input model.
//! Copyright (c) Microsoft Corporation. See NOTICE.md for source attribution.

use super::text_input::TextInputContentType;
use crate::utilities::FormatError;

/// A virtual keyboard and input-method hint. Values match the WinUI public ABI;
/// they do not restrict, validate, or obscure the editor's document contents.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum InputScopeNameValue {
    #[default]
    Default = 0,
    Url = 1,
    EmailSmtpAddress = 5,
    PersonalFullName = 7,
    CurrencyAmountAndSymbol = 20,
    CurrencyAmount = 21,
    DateMonthNumber = 23,
    DateDayNumber = 24,
    DateYear = 25,
    Digits = 28,
    Number = 29,
    Password = 31,
    TelephoneNumber = 32,
    TelephoneCountryCode = 33,
    TelephoneAreaCode = 34,
    TelephoneLocalNumber = 35,
    TimeHour = 37,
    TimeMinutesOrSeconds = 38,
    NumberFullWidth = 39,
    AlphanumericHalfWidth = 40,
    AlphanumericFullWidth = 41,
    Hiragana = 44,
    KatakanaHalfWidth = 45,
    KatakanaFullWidth = 46,
    Hanja = 47,
    HangulHalfWidth = 48,
    HangulFullWidth = 49,
    Search = 50,
    Formula = 51,
    SearchIncremental = 52,
    ChineseHalfWidth = 53,
    ChineseFullWidth = 54,
    NativeScript = 55,
    Text = 57,
    Chat = 58,
    NameOrPhoneNumber = 59,
    EmailNameOrAddress = 60,
    Maps = 62,
    NumericPassword = 63,
    NumericPin = 64,
    AlphanumericPin = 65,
    FormulaNumber = 67,
    ChatWithoutEmoji = 68,
}

impl InputScopeNameValue {
    /// Every public value in the pinned WinUI InputScopeNameValue enumeration.
    pub const ALL: [Self; 43] = [
        Self::Default,
        Self::Url,
        Self::EmailSmtpAddress,
        Self::PersonalFullName,
        Self::CurrencyAmountAndSymbol,
        Self::CurrencyAmount,
        Self::DateMonthNumber,
        Self::DateDayNumber,
        Self::DateYear,
        Self::Digits,
        Self::Number,
        Self::Password,
        Self::TelephoneNumber,
        Self::TelephoneCountryCode,
        Self::TelephoneAreaCode,
        Self::TelephoneLocalNumber,
        Self::TimeHour,
        Self::TimeMinutesOrSeconds,
        Self::NumberFullWidth,
        Self::AlphanumericHalfWidth,
        Self::AlphanumericFullWidth,
        Self::Hiragana,
        Self::KatakanaHalfWidth,
        Self::KatakanaFullWidth,
        Self::Hanja,
        Self::HangulHalfWidth,
        Self::HangulFullWidth,
        Self::Search,
        Self::Formula,
        Self::SearchIncremental,
        Self::ChineseHalfWidth,
        Self::ChineseFullWidth,
        Self::NativeScript,
        Self::Text,
        Self::Chat,
        Self::NameOrPhoneNumber,
        Self::EmailNameOrAddress,
        Self::Maps,
        Self::NumericPassword,
        Self::NumericPin,
        Self::AlphanumericPin,
        Self::FormulaNumber,
        Self::ChatWithoutEmoji,
    ];

    /// The public WinUI symbolic name.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Default => "Default",
            Self::Url => "Url",
            Self::EmailSmtpAddress => "EmailSmtpAddress",
            Self::PersonalFullName => "PersonalFullName",
            Self::CurrencyAmountAndSymbol => "CurrencyAmountAndSymbol",
            Self::CurrencyAmount => "CurrencyAmount",
            Self::DateMonthNumber => "DateMonthNumber",
            Self::DateDayNumber => "DateDayNumber",
            Self::DateYear => "DateYear",
            Self::Digits => "Digits",
            Self::Number => "Number",
            Self::Password => "Password",
            Self::TelephoneNumber => "TelephoneNumber",
            Self::TelephoneCountryCode => "TelephoneCountryCode",
            Self::TelephoneAreaCode => "TelephoneAreaCode",
            Self::TelephoneLocalNumber => "TelephoneLocalNumber",
            Self::TimeHour => "TimeHour",
            Self::TimeMinutesOrSeconds => "TimeMinutesOrSeconds",
            Self::NumberFullWidth => "NumberFullWidth",
            Self::AlphanumericHalfWidth => "AlphanumericHalfWidth",
            Self::AlphanumericFullWidth => "AlphanumericFullWidth",
            Self::Hiragana => "Hiragana",
            Self::KatakanaHalfWidth => "KatakanaHalfWidth",
            Self::KatakanaFullWidth => "KatakanaFullWidth",
            Self::Hanja => "Hanja",
            Self::HangulHalfWidth => "HangulHalfWidth",
            Self::HangulFullWidth => "HangulFullWidth",
            Self::Search => "Search",
            Self::Formula => "Formula",
            Self::SearchIncremental => "SearchIncremental",
            Self::ChineseHalfWidth => "ChineseHalfWidth",
            Self::ChineseFullWidth => "ChineseFullWidth",
            Self::NativeScript => "NativeScript",
            Self::Text => "Text",
            Self::Chat => "Chat",
            Self::NameOrPhoneNumber => "NameOrPhoneNumber",
            Self::EmailNameOrAddress => "EmailNameOrAddress",
            Self::Maps => "Maps",
            Self::NumericPassword => "NumericPassword",
            Self::NumericPin => "NumericPin",
            Self::AlphanumericPin => "AlphanumericPin",
            Self::FormulaNumber => "FormulaNumber",
            Self::ChatWithoutEmoji => "ChatWithoutEmoji",
        }
    }

    /// Converts only defined WinUI values, never manufacturing an invalid enum.
    pub fn from_raw(value: i32) -> Option<Self> {
        Self::ALL.into_iter().find(|item| *item as i32 == value)
    }

    /// Parses a public symbolic name or a defined numeric value.
    pub fn parse(value: &str) -> Result<Self, FormatError> {
        let value = value.trim();
        Self::ALL
            .into_iter()
            .find(|item| item.name().eq_ignore_ascii_case(value))
            .or_else(|| value.parse::<i32>().ok().and_then(Self::from_raw))
            .ok_or_else(|| FormatError::new("The input scope name is not a defined WinUI value."))
    }

    /// Best matching existing FerroUI content type for hosts without this exact
    /// scope. The original scope remains in TextInputOptions for richer hosts.
    pub fn content_type(self) -> TextInputContentType {
        use TextInputContentType as Content;
        match self {
            Self::Url => Content::Url,
            Self::EmailSmtpAddress | Self::EmailNameOrAddress => Content::Email,
            Self::PersonalFullName | Self::NameOrPhoneNumber => Content::Name,
            Self::CurrencyAmountAndSymbol
            | Self::CurrencyAmount
            | Self::Number
            | Self::NumberFullWidth
            | Self::FormulaNumber => Content::Number,
            Self::DateMonthNumber
            | Self::DateDayNumber
            | Self::DateYear
            | Self::Digits
            | Self::TelephoneNumber
            | Self::TelephoneCountryCode
            | Self::TelephoneAreaCode
            | Self::TelephoneLocalNumber
            | Self::TimeHour
            | Self::TimeMinutesOrSeconds => Content::Digits,
            Self::NumericPassword | Self::NumericPin => Content::Pin,
            Self::Password | Self::AlphanumericPin => Content::Password,
            Self::Search | Self::SearchIncremental | Self::Maps => Content::Search,
            Self::Chat => Content::Social,
            Self::AlphanumericHalfWidth => Content::Alpha,
            _ => Content::Normal,
        }
    }
}

impl From<TextInputContentType> for InputScopeNameValue {
    fn from(value: TextInputContentType) -> Self {
        match value {
            TextInputContentType::Normal => Self::Default,
            TextInputContentType::Alpha => Self::AlphanumericHalfWidth,
            TextInputContentType::Digits => Self::Digits,
            TextInputContentType::Pin => Self::NumericPin,
            TextInputContentType::Number => Self::Number,
            TextInputContentType::Email => Self::EmailSmtpAddress,
            TextInputContentType::Url => Self::Url,
            TextInputContentType::Name => Self::PersonalFullName,
            TextInputContentType::Password => Self::Password,
            TextInputContentType::Social => Self::Chat,
            TextInputContentType::Search => Self::Search,
        }
    }
}

crate::ferro_markup_enum!(InputScopeNameValue {
    Default, Url, EmailSmtpAddress, PersonalFullName, CurrencyAmountAndSymbol,
    CurrencyAmount, DateMonthNumber, DateDayNumber, DateYear, Digits, Number,
    Password, TelephoneNumber, TelephoneCountryCode, TelephoneAreaCode,
    TelephoneLocalNumber, TimeHour, TimeMinutesOrSeconds, NumberFullWidth,
    AlphanumericHalfWidth, AlphanumericFullWidth, Hiragana, KatakanaHalfWidth,
    KatakanaFullWidth, Hanja, HangulHalfWidth, HangulFullWidth, Search, Formula,
    SearchIncremental, ChineseHalfWidth, ChineseFullWidth, NativeScript, Text,
    Chat, NameOrPhoneNumber, EmailNameOrAddress, Maps, NumericPassword, NumericPin,
    AlphanumericPin, FormulaNumber, ChatWithoutEmoji,
}, { namespace: "FerroUI.Input" });
