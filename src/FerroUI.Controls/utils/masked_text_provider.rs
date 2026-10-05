// Derived from the .NET runtime's `System.ComponentModel.MaskedTextProvider`
// and `System.ComponentModel.MaskedTextResultHint`.
// Copyright (c) .NET Foundation and Contributors. Licensed under the MIT license.

//! A mask-parsing service: the engine behind a masked text box.
//!
//! The mask language:
//!
//! | Mask char | Meaning |
//! |-----------|---------|
//! | `0` | digit, required |
//! | `9` | digit or space, optional |
//! | `#` | digit, space, `+` or `-`, optional |
//! | `L` | letter, required |
//! | `?` | letter, optional |
//! | `&` | any character, required |
//! | `C` | any character, optional |
//! | `A` | alphanumeric, required |
//! | `a` | alphanumeric, optional |
//! | `.` | decimal separator of the culture |
//! | `,` | thousands separator of the culture |
//! | `:` | time separator of the culture |
//! | `/` | date separator of the culture |
//! | `$` | currency symbol of the culture |
//! | `<` | converts the characters that follow to lower case |
//! | `>` | converts the characters that follow to upper case |
//! | `\|` | stops the case conversion |
//! | `\` | escapes the next mask character, making it a literal |
//! | anything else | literal |
//!
//! Positions are indices into the formatted ("test") string, a `Vec<char>`
//! here. The original works on UTF-16 code units, where every character
//! outside the basic multilingual plane is a surrogate pair and a surrogate is
//! never a valid mask or input character. The same holds here: a character
//! outside the BMP is rejected everywhere (mask, prompt, password, input), so
//! a position is always exactly one UTF-16 code unit and positions can be used
//! as UTF-16 text indices unchanged.

use ferroui_base::media::text_formatting::unicode::{Codepoint, GeneralCategory};
use ferroui_base::utilities::CultureInfo;
use std::fmt;

const SPACE_CHAR: char = ' ';
const DEFAULT_PROMPT_CHAR: char = '_';
const NULL_PASSWORD_CHAR: char = '\0';
const DEFAULT_ALLOW_PROMPT: bool = true;
const INVALID_INDEX: i32 = -1;
const EDIT_ANY: u8 = 0;
const EDIT_UNASSIGNED: u8 = 1;
const EDIT_ASSIGNED: u8 = 2;
const FORWARD: bool = true;
const BACKWARD: bool = false;

/// Specifies the result of a [`MaskedTextProvider`] operation. Positive
/// values are successes, negative values failures (see
/// [`MaskedTextProvider::get_operation_result_from_hint`]).
#[repr(i32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MaskedTextResultHint {
    /// The input position is out of the allowed range. Failure.
    PositionOutOfRange = -55,
    /// The position is not editable and the input does not match the literal. Failure.
    NonEditPosition = -54,
    /// There are no more edit positions available. Failure.
    UnavailableEditPosition = -53,
    /// The prompt character is not valid input. Failure.
    PromptCharNotAllowed = -52,
    /// The input character is not valid for the mask language at all. Failure.
    InvalidInput = -51,
    /// A digit or sign was expected. Failure.
    SignedDigitExpected = -5,
    /// A letter was expected. Failure.
    LetterExpected = -4,
    /// A digit was expected. Failure.
    DigitExpected = -3,
    /// An alphanumeric character was expected. Failure.
    AlphanumericCharacterExpected = -2,
    /// An ASCII character was expected. Failure.
    AsciiCharacterExpected = -1,
    /// Unknown; the initial value of a hint.
    Unknown = 0,
    /// The character was escaped (a literal that was skipped, or a prompt or
    /// space over an unassigned position). Success.
    CharacterEscaped = 1,
    /// The operation changed nothing. Success.
    NoEffect = 2,
    /// The operation had a side effect (characters were shifted, or a
    /// position was reset). Success.
    SideEffect = 3,
    /// The primary operation succeeded. Success.
    Success = 4,
}

/// The culture-dependent strings the mask language inserts for its separator
/// characters, read from the culture when the provider is created.
#[derive(Clone, Debug)]
struct MaskSeparators {
    /// Replaces `.` in the mask.
    decimal_separator: String,
    /// Replaces `,` in the mask.
    group_separator: String,
    /// Replaces `/` in the mask.
    date_separator: String,
    /// Replaces `:` in the mask.
    time_separator: String,
    /// Replaces `$` in the mask.
    currency_symbol: String,
}

impl MaskSeparators {
    fn of(culture: &CultureInfo) -> Self {
        let number_format = culture.number_format();
        let date_time_format = culture.date_time_format();
        Self {
            decimal_separator: number_format.number_decimal_separator().to_owned(),
            group_separator: number_format.number_group_separator().to_owned(),
            date_separator: date_time_format.date_separator().to_owned(),
            time_separator: date_time_format.time_separator().to_owned(),
            currency_symbol: number_format.currency_symbol().to_owned(),
        }
    }
}

/// A culture named `name` whose separators are the given ones (for tests).
#[cfg(test)]
pub(crate) fn culture_with_separators(
    name: &str,
    decimal_separator: &str,
    group_separator: &str,
    date_separator: &str,
    time_separator: &str,
    currency_symbol: &str,
) -> CultureInfo {
    use ferroui_base::utilities::{DateTimeFormatInfo, NumberFormatInfo};

    let mut number_format = NumberFormatInfo::new();
    number_format.set_number_decimal_separator(decimal_separator);
    number_format.set_number_group_separator(group_separator);
    number_format.set_currency_symbol(currency_symbol);
    let mut date_time_format = DateTimeFormatInfo::new();
    date_time_format.set_date_separator(date_separator);
    date_time_format.set_time_separator(time_separator);
    CultureInfo::get_culture_info(name).with_number_format(number_format).with_date_time_format(date_time_format)
}

/// The argument errors of [`MaskedTextProvider`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MaskedTextProviderError {
    /// The mask is empty.
    MaskNullOrEmpty,
    /// The mask contains a character that is not valid in a mask.
    MaskInvalidChar,
    /// The prompt or password character is not valid.
    InvalidChar,
    /// The prompt and the password character are the same.
    PasswordAndPromptCharEqual,
}

impl fmt::Display for MaskedTextProviderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::MaskNullOrEmpty => "The Mask value cannot be null or empty.",
            Self::MaskInvalidChar => "The specified mask contains invalid characters.",
            Self::InvalidChar => "The specified character value is not allowed for this property.",
            Self::PasswordAndPromptCharEqual => "The PasswordChar and PromptChar values cannot be the same.",
        })
    }
}

impl std::error::Error for MaskedTextProviderError {}

/// The case conversion a mask modifier (`<`, `>`, `|`) asks for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CaseConversion {
    None,
    ToLower,
    ToUpper,
}

/// The kind of a position of the test string.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CharType {
    EditOptional,
    EditRequired,
    Separator,
    Literal,
}

/// Describes one position of the test string.
#[derive(Clone, Copy, Debug)]
struct CharDescriptor {
    /// The position of the mask character this position comes from.
    mask_position: usize,
    case_conversion: CaseConversion,
    char_type: CharType,
    is_assigned: bool,
}

impl CharDescriptor {
    #[inline]
    fn is_edit_position(&self) -> bool {
        matches!(self.char_type, CharType::EditRequired | CharType::EditOptional)
    }

    #[inline]
    fn is_literal_position(&self) -> bool {
        matches!(self.char_type, CharType::Literal | CharType::Separator)
    }
}

/// Provides functionality for formatting a test string against a mask string.
/// [`MaskedTextProvider::add`], [`MaskedTextProvider::insert_at`],
/// [`MaskedTextProvider::remove_at`] and the like modify the state of the
/// test string; `Clone` produces an independent copy with the same state.
#[derive(Clone, Debug)]
pub struct MaskedTextProvider {
    ascii_only: bool,
    allow_prompt_as_input: bool,
    include_prompt: bool,
    include_literals: bool,
    reset_on_prompt: bool,
    reset_on_space: bool,
    skip_literals: bool,
    culture: CultureInfo,
    separators: MaskSeparators,
    /// The formatted string: literals, separators, prompts and input.
    test_string: Vec<char>,
    assigned_char_count: i32,
    required_char_count: i32,
    required_edit_chars: i32,
    optional_edit_chars: i32,
    mask: String,
    mask_chars: Vec<char>,
    password_char: char,
    prompt_char: char,
    /// One descriptor per position of `test_string`.
    string_descriptor: Vec<CharDescriptor>,
}

impl MaskedTextProvider {
    /// The index returned by the `find_*` methods when nothing is found (-1).
    pub const INVALID_INDEX: i32 = INVALID_INDEX;

    /// The password character used by [`Self::set_is_password`] (`*`).
    pub const DEFAULT_PASSWORD_CHAR: char = '*';

    /// Creates a provider for `mask` with the current culture, the prompt
    /// allowed as input, `_` as prompt, no password character and no ASCII
    /// restriction.
    pub fn new(mask: &str) -> Result<Self, MaskedTextProviderError> {
        Self::new_full(mask, None, DEFAULT_ALLOW_PROMPT, DEFAULT_PROMPT_CHAR, NULL_PASSWORD_CHAR, false)
    }

    /// Creates a provider for `mask`, optionally restricted to ASCII input.
    pub fn new_with_ascii(mask: &str, restrict_to_ascii: bool) -> Result<Self, MaskedTextProviderError> {
        Self::new_full(mask, None, DEFAULT_ALLOW_PROMPT, DEFAULT_PROMPT_CHAR, NULL_PASSWORD_CHAR, restrict_to_ascii)
    }

    /// Creates a provider for `mask` with a culture.
    pub fn new_with_culture(mask: &str, culture: Option<CultureInfo>) -> Result<Self, MaskedTextProviderError> {
        Self::new_full(mask, culture, DEFAULT_ALLOW_PROMPT, DEFAULT_PROMPT_CHAR, NULL_PASSWORD_CHAR, false)
    }

    /// Creates a provider for `mask` with a culture, optionally restricted to
    /// ASCII input.
    pub fn new_with_culture_ascii(
        mask: &str,
        culture: Option<CultureInfo>,
        restrict_to_ascii: bool,
    ) -> Result<Self, MaskedTextProviderError> {
        Self::new_full(mask, culture, DEFAULT_ALLOW_PROMPT, DEFAULT_PROMPT_CHAR, NULL_PASSWORD_CHAR, restrict_to_ascii)
    }

    /// Creates a provider for `mask` with a password character (`'\0'` for
    /// none).
    pub fn new_with_password(mask: &str, password_char: char, allow_prompt_as_input: bool) -> Result<Self, MaskedTextProviderError> {
        Self::new_full(mask, None, allow_prompt_as_input, DEFAULT_PROMPT_CHAR, password_char, false)
    }

    /// Creates a provider for `mask` with a culture and a password character
    /// (`'\0'` for none).
    pub fn new_with_culture_password(
        mask: &str,
        culture: Option<CultureInfo>,
        password_char: char,
        allow_prompt_as_input: bool,
    ) -> Result<Self, MaskedTextProviderError> {
        Self::new_full(mask, culture, allow_prompt_as_input, DEFAULT_PROMPT_CHAR, password_char, false)
    }

    /// Creates a provider.
    ///
    /// * `mask` — the mask; must not be empty and must only contain valid
    ///   mask characters ([`Self::is_valid_mask_char`]).
    /// * `culture` — the culture providing the separators; `None` is the
    ///   current culture.
    /// * `allow_prompt_as_input` — whether the prompt character is accepted
    ///   as input.
    /// * `prompt_char` — the character shown at unassigned edit positions.
    /// * `password_char` — the character shown instead of the input, `'\0'`
    ///   for none.
    /// * `restrict_to_ascii` — whether letters and characters are restricted
    ///   to ASCII.
    ///
    /// As in the original, the prompt and password characters are not
    /// validated here (only their setters validate).
    pub fn new_full(
        mask: &str,
        culture: Option<CultureInfo>,
        allow_prompt_as_input: bool,
        prompt_char: char,
        password_char: char,
        restrict_to_ascii: bool,
    ) -> Result<Self, MaskedTextProviderError> {
        if mask.is_empty() {
            return Err(MaskedTextProviderError::MaskNullOrEmpty);
        }

        if !mask.chars().all(is_printable_char) {
            return Err(MaskedTextProviderError::MaskInvalidChar);
        }

        let culture = culture.unwrap_or_else(CultureInfo::current_culture);
        let mut provider = Self {
            ascii_only: restrict_to_ascii,
            allow_prompt_as_input,
            // Default values of the read/write properties.
            include_prompt: false,
            include_literals: true,
            reset_on_prompt: true,
            reset_on_space: true,
            skip_literals: true,
            separators: MaskSeparators::of(&culture),
            culture,
            test_string: Vec::new(),
            assigned_char_count: 0,
            required_char_count: 0,
            required_edit_chars: 0,
            optional_edit_chars: 0,
            mask: mask.to_owned(),
            mask_chars: mask.chars().collect(),
            password_char,
            prompt_char,
            string_descriptor: Vec::new(),
        };

        provider.initialize();
        Ok(provider)
    }

    /// Traverses the mask to generate the test string and the descriptors
    /// used to validate the input.
    fn initialize(&mut self) {
        let mut case_conversion = CaseConversion::None;
        let mut escaped_char = false;
        let mut char_type = CharType::Literal;
        let mut buffer = [0u8; 4];

        for mask_pos in 0..self.mask_chars.len() {
            let mut ch = self.mask_chars[mask_pos];
            // The culture string a separator stands for; it can be longer than one character.
            let mut loc_symbol: Option<&str> = None;

            if !escaped_char {
                match ch {
                    // Mask language placeholders.
                    '.' => {
                        loc_symbol = Some(&self.separators.decimal_separator);
                        char_type = CharType::Separator;
                    }
                    ',' => {
                        loc_symbol = Some(&self.separators.group_separator);
                        char_type = CharType::Separator;
                    }
                    ':' => {
                        loc_symbol = Some(&self.separators.time_separator);
                        char_type = CharType::Separator;
                    }
                    '/' => {
                        loc_symbol = Some(&self.separators.date_separator);
                        char_type = CharType::Separator;
                    }
                    '$' => {
                        loc_symbol = Some(&self.separators.currency_symbol);
                        char_type = CharType::Separator;
                    }

                    // Mask language modifiers: no position of their own, they
                    // set the conversion of the edit positions that follow.
                    '<' => {
                        case_conversion = CaseConversion::ToLower;
                        continue;
                    }
                    '>' => {
                        case_conversion = CaseConversion::ToUpper;
                        continue;
                    }
                    '|' => {
                        case_conversion = CaseConversion::None;
                        continue;
                    }
                    '\\' => {
                        // The next character is a literal.
                        escaped_char = true;
                        char_type = CharType::Literal;
                        continue;
                    }

                    // Mask language edit identifiers.
                    '0' | 'L' | '&' | 'A' => {
                        self.required_edit_chars += 1;
                        ch = self.prompt_char;
                        char_type = CharType::EditRequired;
                    }
                    '?' | '9' | '#' | 'C' | 'a' => {
                        self.optional_edit_chars += 1;
                        ch = self.prompt_char;
                        char_type = CharType::EditOptional;
                    }

                    // Literals are added to the test string as they are.
                    _ => {
                        char_type = CharType::Literal;
                    }
                }
            } else {
                // The escaped character is now added to the test string.
                escaped_char = false;
            }

            let mut descriptor = CharDescriptor {
                mask_position: mask_pos,
                case_conversion: CaseConversion::None,
                char_type,
                is_assigned: false,
            };

            if descriptor.is_edit_position() {
                descriptor.case_conversion = case_conversion;
            }

            let symbol: &str = match loc_symbol {
                Some(symbol) if char_type == CharType::Separator => symbol,
                _ => ch.encode_utf8(&mut buffer),
            };

            for ch_val in symbol.chars() {
                self.test_string.push(ch_val);
                self.string_descriptor.push(descriptor);
            }
        }

        self.test_string.shrink_to_fit();
        self.string_descriptor.shrink_to_fit();
    }

    // ---- Properties ----

    /// Whether the prompt character is accepted as valid input.
    pub fn allow_prompt_as_input(&self) -> bool {
        self.allow_prompt_as_input
    }

    /// The number of edit positions that have a character assigned.
    pub fn assigned_edit_position_count(&self) -> i32 {
        self.assigned_char_count
    }

    /// The number of edit positions that have no character assigned.
    pub fn available_edit_position_count(&self) -> i32 {
        self.edit_position_count() - self.assigned_char_count
    }

    /// The culture that provides the separators.
    pub fn culture(&self) -> &CultureInfo {
        &self.culture
    }

    /// The default password character (`*`).
    pub fn default_password_char() -> char {
        Self::DEFAULT_PASSWORD_CHAR
    }

    /// The number of edit positions in the test string.
    pub fn edit_position_count(&self) -> i32 {
        self.optional_edit_chars + self.required_edit_chars
    }

    /// The edit positions of the test string, in ascending order.
    pub fn edit_positions(&self) -> impl Iterator<Item = i32> + '_ {
        self.string_descriptor
            .iter()
            .enumerate()
            .filter(|(_, descriptor)| descriptor.is_edit_position())
            .map(|(position, _)| position as i32)
    }

    /// Whether literals are included in the formatted string ([`fmt::Display`]
    /// and the `to_string_*` methods without an explicit flag). Default `true`.
    pub fn include_literals(&self) -> bool {
        self.include_literals
    }

    /// Sets [`Self::include_literals`].
    pub fn set_include_literals(&mut self, value: bool) {
        self.include_literals = value;
    }

    /// Whether prompt characters are included in the formatted string.
    /// Default `false`.
    pub fn include_prompt(&self) -> bool {
        self.include_prompt
    }

    /// Sets [`Self::include_prompt`].
    pub fn set_include_prompt(&mut self, value: bool) {
        self.include_prompt = value;
    }

    /// Whether only ASCII characters are valid input for the letter,
    /// alphanumeric and "any character" mask elements.
    pub fn ascii_only(&self) -> bool {
        self.ascii_only
    }

    /// Whether the input is displayed with a password character.
    pub fn is_password(&self) -> bool {
        self.password_char != NULL_PASSWORD_CHAR
    }

    /// Turns the password display on (with [`Self::DEFAULT_PASSWORD_CHAR`])
    /// or off; does nothing when it already is in the requested state.
    pub fn set_is_password(&mut self, value: bool) {
        if self.is_password() != value {
            self.password_char = if value { Self::DEFAULT_PASSWORD_CHAR } else { NULL_PASSWORD_CHAR };
        }
    }

    /// The index returned by the `find_*` methods when nothing is found (-1).
    pub fn invalid_index() -> i32 {
        INVALID_INDEX
    }

    /// The position of the last assigned edit position, or
    /// [`Self::INVALID_INDEX`].
    pub fn last_assigned_position(&self) -> i32 {
        self.find_assigned_edit_position_from(self.length() - 1, BACKWARD)
    }

    /// The length of the test string (the mask without modifiers and with the
    /// separators replaced by the culture's).
    pub fn length(&self) -> i32 {
        self.test_string.len() as i32
    }

    /// The mask.
    pub fn mask(&self) -> &str {
        &self.mask
    }

    /// Whether all required edit positions have been assigned.
    pub fn mask_completed(&self) -> bool {
        self.required_char_count == self.required_edit_chars
    }

    /// Whether all edit positions (required and optional) have been assigned.
    pub fn mask_full(&self) -> bool {
        self.assigned_char_count == self.edit_position_count()
    }

    /// The character displayed instead of the input, `'\0'` for none.
    pub fn password_char(&self) -> char {
        self.password_char
    }

    /// Sets the password character (`'\0'` for none). It must differ from the
    /// prompt character and be a valid password character.
    pub fn set_password_char(&mut self, value: char) -> Result<(), MaskedTextProviderError> {
        if value == self.prompt_char {
            // Prompt and password chars must be different.
            return Err(MaskedTextProviderError::PasswordAndPromptCharEqual);
        }

        if !Self::is_valid_password_char(value) {
            // Same message as the prompt character.
            return Err(MaskedTextProviderError::InvalidChar);
        }

        self.password_char = value;
        Ok(())
    }

    /// The character displayed at unassigned edit positions.
    pub fn prompt_char(&self) -> char {
        self.prompt_char
    }

    /// Sets the prompt character. It must differ from the password character
    /// and be a valid input character. The unassigned edit positions of the
    /// test string are updated.
    pub fn set_prompt_char(&mut self, value: char) -> Result<(), MaskedTextProviderError> {
        if value == self.password_char {
            // Prompt and password chars must be different.
            return Err(MaskedTextProviderError::PasswordAndPromptCharEqual);
        }

        if !is_printable_char(value) {
            return Err(MaskedTextProviderError::InvalidChar);
        }

        if value != self.prompt_char {
            self.prompt_char = value;

            for position in 0..self.test_string.len() {
                let descriptor = &self.string_descriptor[position];
                if descriptor.is_edit_position() && !descriptor.is_assigned {
                    self.test_string[position] = value;
                }
            }
        }

        Ok(())
    }

    /// Whether a prompt character in the input resets the position it lands
    /// on instead of being tested against the mask. Default `true`.
    pub fn reset_on_prompt(&self) -> bool {
        self.reset_on_prompt
    }

    /// Sets [`Self::reset_on_prompt`].
    pub fn set_reset_on_prompt(&mut self, value: bool) {
        self.reset_on_prompt = value;
    }

    /// Whether a space in the input resets the position it lands on instead
    /// of being tested against the mask. Default `true`.
    pub fn reset_on_space(&self) -> bool {
        self.reset_on_space
    }

    /// Sets [`Self::reset_on_space`].
    pub fn set_reset_on_space(&mut self, value: bool) {
        self.reset_on_space = value;
    }

    /// Whether an input character equal to the literal at its position is
    /// accepted and skipped. Default `true`.
    pub fn skip_literals(&self) -> bool {
        self.skip_literals
    }

    /// Sets [`Self::skip_literals`].
    pub fn set_skip_literals(&mut self, value: bool) {
        self.skip_literals = value;
    }

    /// The character of the test string at `index`.
    ///
    /// # Panics
    ///
    /// Panics when `index` is out of range.
    pub fn char_at(&self, index: i32) -> char {
        if index < 0 || index >= self.length() {
            panic!("index {index} is out of range of the test string (length {})", self.length());
        }

        self.test_string[index as usize]
    }

    // ---- Methods ----

    /// Attempts to add `input` at the first edit position after the last
    /// assigned one.
    pub fn add_char(&mut self, input: char) -> bool {
        self.add_char_with_hint(input).0
    }

    /// As [`Self::add_char`]; also returns the position the character was
    /// tested at (the length of the test string when there is none) and the
    /// result hint.
    pub fn add_char_with_hint(&mut self, input: char) -> (bool, i32, MaskedTextResultHint) {
        let last_assigned_pos = self.last_assigned_position();

        if last_assigned_pos == self.length() - 1 {
            // At the last edit char position.
            return (false, self.length(), MaskedTextResultHint::UnavailableEditPosition);
        }

        // Get the position after the last assigned position.
        let test_position = self.find_edit_position_from(last_assigned_pos + 1, FORWARD);

        if test_position == INVALID_INDEX {
            return (false, self.length(), MaskedTextResultHint::UnavailableEditPosition);
        }

        let (result, hint) = self.test_set_char(input, test_position);
        (result, test_position, hint)
    }

    /// Attempts to add the characters of `input` after the last assigned
    /// position. All of them are added, or none.
    pub fn add(&mut self, input: &str) -> bool {
        self.add_with_hint(input).0
    }

    /// As [`Self::add`]; also returns the position of the last character
    /// tested (on failure: the failing position) and the result hint.
    pub fn add_with_hint(&mut self, input: &str) -> (bool, i32, MaskedTextResultHint) {
        let test_position = self.last_assigned_position() + 1;

        if input.is_empty() {
            // Nothing to add; the position is where the test would be performed.
            return (true, test_position, MaskedTextResultHint::NoEffect);
        }

        self.test_set_string(input, test_position)
    }

    /// Resets all edit positions.
    pub fn clear(&mut self) {
        self.clear_with_hint();
    }

    /// As [`Self::clear`]; returns [`MaskedTextResultHint::NoEffect`] when
    /// nothing was assigned, [`MaskedTextResultHint::Success`] otherwise.
    pub fn clear_with_hint(&mut self) -> MaskedTextResultHint {
        if self.assigned_char_count == 0 {
            return MaskedTextResultHint::NoEffect;
        }

        for position in 0..self.length() {
            self.reset_char(position);
        }

        MaskedTextResultHint::Success
    }

    /// The first assigned edit position from `position` in the direction
    /// (`true` forward, `false` backward), or [`Self::INVALID_INDEX`].
    pub fn find_assigned_edit_position_from(&self, position: i32, direction: bool) -> i32 {
        if self.assigned_char_count == 0 {
            return INVALID_INDEX;
        }

        let (start_position, end_position) = self.range_from(position, direction);
        self.find_assigned_edit_position_in_range(start_position, end_position, direction)
    }

    /// The first assigned edit position in the inclusive range, searching in
    /// the direction, or [`Self::INVALID_INDEX`].
    pub fn find_assigned_edit_position_in_range(&self, start_position: i32, end_position: i32, direction: bool) -> i32 {
        if self.assigned_char_count == 0 {
            return INVALID_INDEX;
        }

        self.find_edit_position_in_range_status(start_position, end_position, direction, EDIT_ASSIGNED)
    }

    /// The first edit position (assigned or not) from `position` in the
    /// direction, or [`Self::INVALID_INDEX`].
    pub fn find_edit_position_from(&self, position: i32, direction: bool) -> i32 {
        let (start_position, end_position) = self.range_from(position, direction);
        self.find_edit_position_in_range(start_position, end_position, direction)
    }

    /// The first edit position (assigned or not) in the inclusive range,
    /// searching in the direction, or [`Self::INVALID_INDEX`].
    pub fn find_edit_position_in_range(&self, start_position: i32, end_position: i32, direction: bool) -> i32 {
        self.find_position_in_range(start_position, end_position, direction, true)
    }

    /// The first edit position in the range with the given assigned status.
    fn find_edit_position_in_range_status(&self, mut start_position: i32, mut end_position: i32, direction: bool, assigned_status: u8) -> i32 {
        // Out of range positions are handled in find_edit_position_in_range.
        loop {
            let test_position = self.find_edit_position_in_range(start_position, end_position, direction);

            if test_position == INVALID_INDEX {
                break;
            }

            let is_assigned = self.string_descriptor[test_position as usize].is_assigned;

            match assigned_status {
                EDIT_UNASSIGNED => {
                    if !is_assigned {
                        return test_position;
                    }
                }
                EDIT_ASSIGNED => {
                    if is_assigned {
                        return test_position;
                    }
                }
                _ => return test_position,
            }

            if direction == FORWARD {
                start_position += 1;
            } else {
                end_position -= 1;
            }

            if start_position > end_position {
                break;
            }
        }

        INVALID_INDEX
    }

    /// The first non-edit position (literal or separator) from `position` in
    /// the direction, or [`Self::INVALID_INDEX`].
    pub fn find_non_edit_position_from(&self, position: i32, direction: bool) -> i32 {
        let (start_position, end_position) = self.range_from(position, direction);
        self.find_non_edit_position_in_range(start_position, end_position, direction)
    }

    /// The first non-edit position in the inclusive range, searching in the
    /// direction, or [`Self::INVALID_INDEX`].
    pub fn find_non_edit_position_in_range(&self, start_position: i32, end_position: i32, direction: bool) -> i32 {
        self.find_position_in_range(start_position, end_position, direction, false)
    }

    /// Finds a position in the range whose kind (edit / non-edit) matches.
    fn find_position_in_range(&self, mut start_position: i32, mut end_position: i32, direction: bool, edit: bool) -> i32 {
        if start_position < 0 {
            start_position = 0;
        }

        if end_position >= self.length() {
            end_position = self.length() - 1;
        }

        if start_position > end_position {
            return INVALID_INDEX;
        }

        while start_position <= end_position {
            let test_position = if direction == FORWARD {
                start_position += 1;
                start_position - 1
            } else {
                end_position -= 1;
                end_position + 1
            };

            if self.string_descriptor[test_position as usize].is_edit_position() == edit {
                return test_position;
            }
        }

        INVALID_INDEX
    }

    /// The first unassigned edit position from `position` in the direction,
    /// or [`Self::INVALID_INDEX`].
    pub fn find_unassigned_edit_position_from(&self, position: i32, direction: bool) -> i32 {
        let (start_position, end_position) = self.range_from(position, direction);
        self.find_edit_position_in_range_status(start_position, end_position, direction, EDIT_UNASSIGNED)
    }

    /// The first unassigned edit position in the inclusive range, searching
    /// in the direction, or [`Self::INVALID_INDEX`].
    pub fn find_unassigned_edit_position_in_range(&self, mut start_position: i32, mut end_position: i32, direction: bool) -> i32 {
        loop {
            let position = self.find_edit_position_in_range_status(start_position, end_position, direction, EDIT_ANY);

            if position == INVALID_INDEX {
                return INVALID_INDEX;
            }

            if !self.string_descriptor[position as usize].is_assigned {
                return position;
            }

            if direction == FORWARD {
                start_position += 1;
            } else {
                end_position -= 1;
            }
        }
    }

    /// The range a `*_from` search covers.
    #[inline]
    fn range_from(&self, position: i32, direction: bool) -> (i32, i32) {
        if direction == FORWARD {
            (position, self.length() - 1)
        } else {
            (0, position)
        }
    }

    /// Whether a hint describes a success (positive) or a failure.
    pub fn get_operation_result_from_hint(hint: MaskedTextResultHint) -> bool {
        (hint as i32) > 0
    }

    /// Attempts to insert `input` at `position` (or the first edit position
    /// after it), shifting the existing characters right.
    pub fn insert_at_char(&mut self, input: char, position: i32) -> bool {
        if position < 0 || position >= self.length() {
            return false;
        }

        let mut buffer = [0u8; 4];
        self.insert_at(input.encode_utf8(&mut buffer), position)
    }

    /// As [`Self::insert_at_char`]; also returns the position of the inserted
    /// character (on failure: the failing position) and the result hint.
    pub fn insert_at_char_with_hint(&mut self, input: char, position: i32) -> (bool, i32, MaskedTextResultHint) {
        let mut buffer = [0u8; 4];
        self.insert_at_with_hint(input.encode_utf8(&mut buffer), position)
    }

    /// Attempts to insert the characters of `input` from `position`, shifting
    /// the existing characters right. All of them are inserted, or none.
    pub fn insert_at(&mut self, input: &str, position: i32) -> bool {
        self.insert_at_with_hint(input, position).0
    }

    /// As [`Self::insert_at`]; also returns the position of the last inserted
    /// character (on failure: the failing position) and the result hint.
    pub fn insert_at_with_hint(&mut self, input: &str, position: i32) -> (bool, i32, MaskedTextResultHint) {
        if position < 0 || position >= self.length() {
            return (false, position, MaskedTextResultHint::PositionOutOfRange);
        }

        self.insert_at_int(input, position, false)
    }

    /// Inserts (or, with `test_only`, only tests the insertion of) `input` at
    /// `position`, which must be in range.
    fn insert_at_int(&mut self, input: &str, position: i32, test_only: bool) -> (bool, i32, MaskedTextResultHint) {
        debug_assert!(position >= 0 && position < self.length(), "input param out of range.");

        if input.is_empty() {
            // Nothing to insert.
            return (true, position, MaskedTextResultHint::NoEffect);
        }

        // Test the input string first. test_position is the position of the last inserted character.
        let (result, mut test_position, mut result_hint) = self.test_string(input, position);
        if !result {
            return (false, test_position, result_hint);
        }

        // Now check whether room has to be made for the input characters
        // (existing characters shift right) and if so test the shifting characters.
        let mut src_pos = self.find_edit_position_from(position, FORWARD);
        let shift_needed = self.find_assigned_edit_position_in_range(src_pos, test_position, FORWARD) != INVALID_INDEX;
        let last_assigned_pos = self.last_assigned_position();

        if shift_needed && test_position == self.length() - 1 {
            // No room for shifting.
            return (false, self.length(), MaskedTextResultHint::UnavailableEditPosition);
        }

        let mut dst_pos = self.find_edit_position_from(test_position + 1, FORWARD);

        if shift_needed {
            // Temp hint used not to overwrite the primary operation result hint (from test_string).
            let mut temp_hint = MaskedTextResultHint::Unknown;

            // Test the shifting characters.
            loop {
                if dst_pos == INVALID_INDEX {
                    return (false, self.length(), MaskedTextResultHint::UnavailableEditPosition);
                }

                // Only assigned positions are tested.
                if self.string_descriptor[src_pos as usize].is_assigned {
                    let (ok, hint) = self.test_char(self.test_string[src_pos as usize], dst_pos);
                    temp_hint = hint;
                    if !ok {
                        test_position = dst_pos;
                        return (false, test_position, temp_hint);
                    }
                }

                if src_pos == last_assigned_pos {
                    // All shifting positions tested.
                    break;
                }

                src_pos = self.find_edit_position_from(src_pos + 1, FORWARD);
                dst_pos = self.find_edit_position_from(dst_pos + 1, FORWARD);
            }

            if temp_hint > result_hint {
                result_hint = temp_hint;
            }
        }

        if test_only {
            return (true, test_position, result_hint);
        }

        // The tests passed: shift the existing characters to make room for
        // the new ones, then set those.
        if shift_needed {
            while src_pos >= position {
                if self.string_descriptor[src_pos as usize].is_assigned {
                    self.set_char(self.test_string[src_pos as usize], dst_pos);
                } else {
                    self.reset_char(dst_pos);
                }

                dst_pos = self.find_edit_position_from(dst_pos - 1, BACKWARD);
                src_pos = self.find_edit_position_from(src_pos - 1, BACKWARD);
            }
        }

        // Finally set the input characters.
        self.set_string(input, position);

        (true, test_position, result_hint)
    }

    /// Whether `position` is an edit position with no character assigned.
    pub fn is_available_position(&self, position: i32) -> bool {
        if position < 0 || position >= self.length() {
            return false;
        }

        let descriptor = &self.string_descriptor[position as usize];
        descriptor.is_edit_position() && !descriptor.is_assigned
    }

    /// Whether `position` is an edit position.
    pub fn is_edit_position(&self, position: i32) -> bool {
        if position < 0 || position >= self.length() {
            return false;
        }

        self.string_descriptor[position as usize].is_edit_position()
    }

    /// Whether `c` is a valid input character: a letter, digit, punctuation,
    /// symbol or the space.
    pub fn is_valid_input_char(c: char) -> bool {
        is_printable_char(c)
    }

    /// Whether `c` is a valid mask character (the same set as
    /// [`Self::is_valid_input_char`]).
    pub fn is_valid_mask_char(c: char) -> bool {
        is_printable_char(c)
    }

    /// Whether `c` is a valid password character: a valid input character, or
    /// `'\0'` (no password character).
    pub fn is_valid_password_char(c: char) -> bool {
        is_printable_char(c) || c == NULL_PASSWORD_CHAR
    }

    /// Removes the last assigned character. Always succeeds.
    pub fn remove(&mut self) -> bool {
        self.remove_with_hint().0
    }

    /// As [`Self::remove`]; also returns the position of the removed
    /// character (0 when there was none) and the result hint.
    pub fn remove_with_hint(&mut self) -> (bool, i32, MaskedTextResultHint) {
        let last_assigned_pos = self.last_assigned_position();

        if last_assigned_pos == INVALID_INDEX {
            // Nothing to remove.
            return (true, 0, MaskedTextResultHint::NoEffect);
        }

        self.reset_char(last_assigned_pos);
        (true, last_assigned_pos, MaskedTextResultHint::Success)
    }

    /// Removes the character at `position`, shifting the characters after it
    /// left.
    pub fn remove_at(&mut self, position: i32) -> bool {
        self.remove_at_range(position, position)
    }

    /// Removes the characters in the inclusive range, shifting the characters
    /// after it left.
    pub fn remove_at_range(&mut self, start_position: i32, end_position: i32) -> bool {
        self.remove_at_range_with_hint(start_position, end_position).0
    }

    /// As [`Self::remove_at_range`]; also returns the position the operation
    /// was performed at (on failure: the failing position) and the result
    /// hint.
    pub fn remove_at_range_with_hint(&mut self, start_position: i32, end_position: i32) -> (bool, i32, MaskedTextResultHint) {
        if end_position >= self.length() {
            return (false, end_position, MaskedTextResultHint::PositionOutOfRange);
        }

        if start_position < 0 || start_position > end_position {
            return (false, start_position, MaskedTextResultHint::PositionOutOfRange);
        }

        self.remove_at_int(start_position, end_position, false)
    }

    /// Removes (or, with `test_only`, only tests the removal of) the
    /// characters in the range, which must be valid.
    fn remove_at_int(&mut self, mut start_position: i32, end_position: i32, test_only: bool) -> (bool, i32, MaskedTextResultHint) {
        debug_assert!(
            start_position >= 0 && start_position <= end_position && end_position < self.length(),
            "Out of range input."
        );

        // Check whether characters have to shift left to occupy the positions
        // left by the characters being removed.
        let last_assigned_pos = self.last_assigned_position();
        let mut dst_pos = self.find_edit_position_in_range(start_position, end_position, FORWARD);

        let mut result_hint = MaskedTextResultHint::NoEffect;

        if dst_pos == INVALID_INDEX || dst_pos > last_assigned_pos {
            // Nothing to remove.
            return (true, start_position, result_hint);
        }

        // On remove, the test position remains the same.
        let test_position = start_position;

        // Does the last assigned position survive the removal?
        let shift_needed = end_position < last_assigned_pos;

        // If there are assigned characters to be removed (the range may have
        // none, in which case characters may just be shifted), the hint is success.
        if self.find_assigned_edit_position_in_range(start_position, end_position, FORWARD) != INVALID_INDEX {
            result_hint = MaskedTextResultHint::Success;
        }

        if shift_needed {
            // Test the shifting characters.
            let mut src_pos = self.find_edit_position_from(end_position + 1, FORWARD);
            let shift_start = src_pos;

            // The actual start position.
            start_position = dst_pos;

            loop {
                let src_ch = self.test_string[src_pos as usize];
                let is_assigned = self.string_descriptor[src_pos as usize].is_assigned;

                // A prompt at an unassigned position does not need to be tested.
                if src_ch != self.prompt_char || is_assigned {
                    let (ok, test_hint) = self.test_char(src_ch, dst_pos);
                    if !ok {
                        // dst_pos is the failed position.
                        return (false, dst_pos, test_hint);
                    }
                }

                if src_pos == last_assigned_pos {
                    break;
                }

                src_pos = self.find_edit_position_from(src_pos + 1, FORWARD);
                dst_pos = self.find_edit_position_from(dst_pos + 1, FORWARD);
            }

            // Shifting characters is a side effect; update the hint if no
            // characters are removed (which would be success).
            if MaskedTextResultHint::SideEffect > result_hint {
                result_hint = MaskedTextResultHint::SideEffect;
            }

            if test_only {
                return (true, test_position, result_hint);
            }

            // The test passed: shift the characters.
            src_pos = shift_start;
            dst_pos = start_position;

            loop {
                let src_ch = self.test_string[src_pos as usize];
                let is_assigned = self.string_descriptor[src_pos as usize].is_assigned;

                // A prompt at an unassigned position just resets the destination.
                if src_ch == self.prompt_char && !is_assigned {
                    self.reset_char(dst_pos);
                } else {
                    self.set_char(src_ch, dst_pos);
                    self.reset_char(src_pos);
                }

                if src_pos == last_assigned_pos {
                    break;
                }

                src_pos = self.find_edit_position_from(src_pos + 1, FORWARD);
                dst_pos = self.find_edit_position_from(dst_pos + 1, FORWARD);
            }

            // Reset the remaining characters from dst_pos + 1 to the end position.
            start_position = dst_pos + 1;
        }

        if start_position <= end_position {
            self.reset_string(start_position, end_position);
        }

        (true, test_position, result_hint)
    }

    /// Replaces the character at `position` (or the first edit position after
    /// it) with `input`.
    pub fn replace_char(&mut self, input: char, position: i32) -> bool {
        self.replace_char_with_hint(input, position).0
    }

    /// As [`Self::replace_char`]; also returns the position of the replaced
    /// character (on failure: the failing position) and the result hint.
    pub fn replace_char_with_hint(&mut self, input: char, position: i32) -> (bool, i32, MaskedTextResultHint) {
        if position < 0 || position >= self.length() {
            return (false, position, MaskedTextResultHint::PositionOutOfRange);
        }

        let mut test_position = position;

        // If the character is not to be escaped, find the first edit position to test it in.
        if !self.test_escape_char(input, test_position) {
            test_position = self.find_edit_position_from(test_position, FORWARD);
        }

        if test_position == INVALID_INDEX {
            return (false, position, MaskedTextResultHint::UnavailableEditPosition);
        }

        let (result, hint) = self.test_set_char(input, test_position);
        (result, test_position, hint)
    }

    /// Replaces the characters in the inclusive range with `input`, removing
    /// the rest of the range and shifting the characters after it left.
    pub fn replace_char_range_with_hint(&mut self, input: char, start_position: i32, end_position: i32) -> (bool, i32, MaskedTextResultHint) {
        if end_position >= self.length() {
            return (false, end_position, MaskedTextResultHint::PositionOutOfRange);
        }

        if start_position < 0 || start_position > end_position {
            return (false, start_position, MaskedTextResultHint::PositionOutOfRange);
        }

        if start_position == end_position {
            let (result, hint) = self.test_set_char(input, start_position);
            return (result, start_position, hint);
        }

        let mut buffer = [0u8; 4];
        self.replace_range_with_hint(input.encode_utf8(&mut buffer), start_position, end_position)
    }

    /// Replaces the characters from `position` with the characters of
    /// `input`, without shifting. An empty input removes the character at
    /// `position`.
    pub fn replace(&mut self, input: &str, position: i32) -> bool {
        self.replace_with_hint(input, position).0
    }

    /// As [`Self::replace`]; also returns the position of the last replaced
    /// character (on failure: the failing position) and the result hint.
    pub fn replace_with_hint(&mut self, input: &str, position: i32) -> (bool, i32, MaskedTextResultHint) {
        if position < 0 || position >= self.length() {
            return (false, position, MaskedTextResultHint::PositionOutOfRange);
        }

        if input.is_empty() {
            // Remove the character at position.
            return self.remove_at_range_with_hint(position, position);
        }

        // Replace the characters with the ones in the input.
        self.test_set_string(input, position)
    }

    /// Replaces the characters in the inclusive range with the characters of
    /// `input`: when the input is shorter than the range the rest of the
    /// range is removed, when it is longer the characters after the range
    /// shift right. An empty input removes the range.
    pub fn replace_range_with_hint(&mut self, input: &str, start_position: i32, end_position: i32) -> (bool, i32, MaskedTextResultHint) {
        if end_position >= self.length() {
            return (false, end_position, MaskedTextResultHint::PositionOutOfRange);
        }

        if start_position < 0 || start_position > end_position {
            return (false, start_position, MaskedTextResultHint::PositionOutOfRange);
        }

        if input.is_empty() {
            // Remove the characters of the range.
            return self.remove_at_range_with_hint(start_position, end_position);
        }

        // Three cases:
        // 1. The input covers exactly the range (or nothing is assigned): just replace.
        // 2. The input is shorter: replace, and remove the rest of the range.
        // 3. The input is longer: replace the range and insert the rest.

        // Test the input string first; the last test position tells which case it is.
        let (result, mut test_position, mut result_hint) = self.test_string(input, start_position);
        if !result {
            return (false, test_position, result_hint);
        }

        if self.assigned_char_count > 0 {
            if test_position < end_position {
                // Case 2. Replace + remove the remaining characters.
                let (ok, temp_pos, temp_hint) = self.remove_at_int(test_position + 1, end_position, false);
                if !ok {
                    return (false, temp_pos, temp_hint);
                }

                // If the current hint is not success and characters were removed, the hint is side effect.
                if temp_hint == MaskedTextResultHint::Success && result_hint != temp_hint {
                    result_hint = MaskedTextResultHint::SideEffect;
                }
            } else if test_position > end_position {
                // Case 3. Replace + insert: test shifting the existing
                // characters to make room for the rest of the input.
                let last_assigned_pos = self.last_assigned_position();
                let mut dst_pos = test_position + 1;
                let mut src_pos = end_position + 1;

                loop {
                    src_pos = self.find_edit_position_from(src_pos, FORWARD);
                    dst_pos = self.find_edit_position_from(dst_pos, FORWARD);

                    if dst_pos == INVALID_INDEX {
                        test_position = self.length();
                        return (false, test_position, MaskedTextResultHint::UnavailableEditPosition);
                    }

                    let (ok, temp_hint) = self.test_char(self.test_string[src_pos as usize], dst_pos);
                    if !ok {
                        return (false, dst_pos, temp_hint);
                    }

                    // If the current hint is not success and a character is actually shifted, the hint is success.
                    if temp_hint == MaskedTextResultHint::Success && result_hint != temp_hint {
                        result_hint = MaskedTextResultHint::Success;
                    }

                    if src_pos == last_assigned_pos {
                        break;
                    }

                    src_pos += 1;
                    dst_pos += 1;
                }

                // The shift test passed, now do it.
                while dst_pos > test_position {
                    self.set_char(self.test_string[src_pos as usize], dst_pos);

                    src_pos = self.find_edit_position_from(src_pos - 1, BACKWARD);
                    dst_pos = self.find_edit_position_from(dst_pos - 1, BACKWARD);
                }
            }

            // Otherwise end_position == test_position: replacing the range is the same as setting it.
        }

        // In all cases the input replaces the characters.
        self.set_string(input, start_position);

        (true, test_position, result_hint)
    }

    /// Resets the edit position (if it is one and it is assigned).
    fn reset_char(&mut self, test_position: i32) {
        let descriptor = &mut self.string_descriptor[test_position as usize];

        if descriptor.is_edit_position() && descriptor.is_assigned {
            descriptor.is_assigned = false;
            self.test_string[test_position as usize] = self.prompt_char;
            self.assigned_char_count -= 1;

            if descriptor.char_type == CharType::EditRequired {
                self.required_char_count -= 1;
            }

            debug_assert!(self.assigned_char_count >= 0, "Invalid count of assigned chars.");
        }
    }

    /// Resets the assigned edit positions in the inclusive range.
    fn reset_string(&mut self, mut start_position: i32, mut end_position: i32) {
        start_position = self.find_assigned_edit_position_from(start_position, FORWARD);

        if start_position != INVALID_INDEX {
            end_position = self.find_assigned_edit_position_from(end_position, BACKWARD);

            while start_position <= end_position {
                start_position = self.find_assigned_edit_position_from(start_position, FORWARD);
                self.reset_char(start_position);
                start_position += 1;
            }
        }
    }

    /// Sets the test string to `input`: the previous content is replaced.
    /// Nothing changes when the input does not fit the mask.
    pub fn set(&mut self, input: &str) -> bool {
        self.set_with_hint(input).0
    }

    /// As [`Self::set`]; also returns the position of the last character set
    /// (on failure: the failing position) and the result hint.
    pub fn set_with_hint(&mut self, input: &str) -> (bool, i32, MaskedTextResultHint) {
        if input.is_empty() {
            // Clearing the input text.
            let hint = self.clear_with_hint();
            return (true, 0, hint);
        }

        let (result, test_position, result_hint) = self.test_set_string(input, 0);
        if !result {
            return (false, test_position, result_hint);
        }

        // Reset the remaining characters (if any).
        let reset_pos = self.find_assigned_edit_position_from(test_position + 1, FORWARD);

        if reset_pos != INVALID_INDEX {
            self.reset_string(reset_pos, self.length() - 1);
        }

        (true, test_position, result_hint)
    }

    /// Sets (or, when the character is to be escaped, resets) the character
    /// at `position`. The character must have been tested.
    fn set_char(&mut self, mut input: char, position: i32) {
        // A space or prompt that is to be escaped resets the position if it
        // is assigned; this does not affect literal positions.
        if self.test_escape_char(input, position) {
            self.reset_char(position);
            return;
        }

        let descriptor = self.string_descriptor[position as usize];
        debug_assert!(!descriptor.is_literal_position(), "Setting char in literal position.");

        if is_letter(input) {
            if is_upper(input) {
                if descriptor.case_conversion == CaseConversion::ToLower {
                    input = to_lower(input);
                }
            } else if descriptor.case_conversion == CaseConversion::ToUpper {
                // The character is lower case.
                input = to_upper(input);
            }
        }

        self.test_string[position as usize] = input;

        if !descriptor.is_assigned {
            // The position was not counted as assigned yet.
            self.string_descriptor[position as usize].is_assigned = true;
            self.assigned_char_count += 1;

            if descriptor.char_type == CharType::EditRequired {
                self.required_char_count += 1;
            }
        }
    }

    /// Sets the characters of `input` from `test_position`. The string must
    /// have been tested.
    fn set_string(&mut self, input: &str, mut test_position: i32) {
        for ch in input.chars() {
            // If the character is not to be escaped, find the first edit position to set it in.
            if !self.test_escape_char(ch, test_position) {
                test_position = self.find_edit_position_from(test_position, FORWARD);
            }

            self.set_char(ch, test_position);
            test_position += 1;
        }
    }

    /// Tests `input` against the mask at `position` (which must be in range).
    fn test_char(&self, input: char, position: i32) -> (bool, MaskedTextResultHint) {
        if !is_printable_char(input) {
            return (false, MaskedTextResultHint::InvalidInput);
        }

        let descriptor = &self.string_descriptor[position as usize];
        let current = self.test_string[position as usize];

        // Test whether the character should be accepted as a literal.
        if descriptor.is_literal_position() {
            if self.skip_literals && input == current {
                return (true, MaskedTextResultHint::CharacterEscaped);
            }

            return (false, MaskedTextResultHint::NonEditPosition);
        }

        if input == self.prompt_char {
            if self.reset_on_prompt {
                // The test does not fail for the prompt when it is to be escaped.
                return if descriptor.is_edit_position() && descriptor.is_assigned {
                    // The position would be reset.
                    (true, MaskedTextResultHint::SideEffect)
                } else {
                    (true, MaskedTextResultHint::CharacterEscaped)
                };
            }

            // Escaping precedes allow_prompt_as_input.
            if !self.allow_prompt_as_input {
                return (false, MaskedTextResultHint::PromptCharNotAllowed);
            }
        }

        if input == SPACE_CHAR && self.reset_on_space {
            return if descriptor.is_edit_position() && descriptor.is_assigned {
                // The position would be reset.
                (true, MaskedTextResultHint::SideEffect)
            } else {
                (true, MaskedTextResultHint::CharacterEscaped)
            };
        }

        // The character was not escaped: test it against the mask
        // constraints. A space passes when the element is optional.
        match self.mask_chars[descriptor.mask_position] {
            // Digit or plus/minus sign, optional.
            '#' => {
                if !is_digit(input) && input != '-' && input != '+' && input != SPACE_CHAR {
                    return (false, MaskedTextResultHint::DigitExpected);
                }
            }
            // Digit, required.
            '0' => {
                if !is_digit(input) {
                    return (false, MaskedTextResultHint::DigitExpected);
                }
            }
            // Digit, optional.
            '9' => {
                if !is_digit(input) && input != SPACE_CHAR {
                    return (false, MaskedTextResultHint::DigitExpected);
                }
            }
            // Letter, required.
            'L' => {
                if !is_letter(input) {
                    return (false, MaskedTextResultHint::LetterExpected);
                }
                if !is_ascii_letter(input) && self.ascii_only {
                    return (false, MaskedTextResultHint::AsciiCharacterExpected);
                }
            }
            // Letter, optional.
            '?' => {
                if !is_letter(input) && input != SPACE_CHAR {
                    return (false, MaskedTextResultHint::LetterExpected);
                }
                if !is_ascii_letter(input) && self.ascii_only {
                    return (false, MaskedTextResultHint::AsciiCharacterExpected);
                }
            }
            // Any character, required.
            '&' => {
                if !is_ascii(input) && self.ascii_only {
                    return (false, MaskedTextResultHint::AsciiCharacterExpected);
                }
            }
            // Any character, optional.
            'C' => {
                if (!is_ascii(input) && self.ascii_only) && input != SPACE_CHAR {
                    return (false, MaskedTextResultHint::AsciiCharacterExpected);
                }
            }
            // Alphanumeric, required.
            'A' => {
                if !is_alphanumeric(input) {
                    return (false, MaskedTextResultHint::AlphanumericCharacterExpected);
                }
                if !is_ascii_alphanumeric(input) && self.ascii_only {
                    return (false, MaskedTextResultHint::AsciiCharacterExpected);
                }
            }
            // Alphanumeric, optional.
            'a' => {
                if !is_alphanumeric(input) && input != SPACE_CHAR {
                    return (false, MaskedTextResultHint::AlphanumericCharacterExpected);
                }
                if !is_ascii_alphanumeric(input) && self.ascii_only {
                    return (false, MaskedTextResultHint::AsciiCharacterExpected);
                }
            }
            _ => debug_assert!(false, "Invalid mask language character."),
        }

        // The test passed.
        if input == current && descriptor.is_assigned {
            // Setting the character would not make any difference.
            (true, MaskedTextResultHint::NoEffect)
        } else {
            (true, MaskedTextResultHint::Success)
        }
    }

    /// Whether `input` would be escaped at `position` (which must be in
    /// range): a literal equal to the input that is skipped, or a prompt or
    /// space that resets an edit position.
    fn test_escape_char(&self, input: char, position: i32) -> bool {
        let descriptor = &self.string_descriptor[position as usize];

        if descriptor.is_literal_position() {
            // Should the literal be skipped?
            return self.skip_literals && input == self.test_string[position as usize];
        }

        (self.reset_on_prompt && input == self.prompt_char) || (self.reset_on_space && input == SPACE_CHAR)
    }

    /// Tests the character and sets it when the test passes.
    fn test_set_char(&mut self, input: char, position: i32) -> (bool, MaskedTextResultHint) {
        let (result, hint) = self.test_char(input, position);

        if result {
            // The character is not to be escaped.
            if hint == MaskedTextResultHint::Success || hint == MaskedTextResultHint::SideEffect {
                self.set_char(input, position);
            }

            return (true, hint);
        }

        (false, hint)
    }

    /// Tests the string and sets it when the test passes.
    fn test_set_string(&mut self, input: &str, position: i32) -> (bool, i32, MaskedTextResultHint) {
        let (result, test_position, hint) = self.test_string(input, position);

        if result {
            self.set_string(input, position);
        }

        (result, test_position, hint)
    }

    /// Tests the characters of `input` from `position`. Returns the position
    /// of the last character tested (on failure: the failing position, or the
    /// length of the test string when it ran out of edit positions).
    fn test_string(&self, input: &str, position: i32) -> (bool, i32, MaskedTextResultHint) {
        debug_assert!(position >= 0, "Position out of range.");

        let mut result_hint = MaskedTextResultHint::Unknown;
        let mut test_position = position;

        if input.is_empty() {
            // Nothing to test.
            return (true, test_position, result_hint);
        }

        // If any character is actually accepted the hint is success,
        // otherwise whatever the result of the last character is.
        for ch in input.chars() {
            if test_position >= self.length() {
                return (false, test_position, MaskedTextResultHint::UnavailableEditPosition);
            }

            // If the character is not to be escaped, find an edit position to test it in.
            if !self.test_escape_char(ch, test_position) {
                test_position = self.find_edit_position_from(test_position, FORWARD);

                if test_position == INVALID_INDEX {
                    return (false, self.length(), MaskedTextResultHint::UnavailableEditPosition);
                }
            }

            // Test the character at the test position.
            let (ok, temp_hint) = self.test_char(ch, test_position);
            if !ok {
                return (false, test_position, temp_hint);
            }

            // Result precedence: Success, SideEffect, NoEffect, CharacterEscaped.
            if temp_hint > result_hint {
                result_hint = temp_hint;
            }

            test_position += 1;
        }

        // The last tested position.
        test_position -= 1;

        (true, test_position, result_hint)
    }

    /// The formatted string as it is to be displayed: prompts and literals
    /// included, and the input replaced by the password character if there is
    /// one.
    pub fn to_display_string(&self) -> String {
        if !self.is_password() || self.assigned_char_count == 0 {
            // The test string contains the formatted text.
            return self.test_string.iter().collect();
        }

        // Copy the test string, replacing the input with the password character.
        self.test_string
            .iter()
            .zip(&self.string_descriptor)
            .map(|(&ch, descriptor)| {
                if descriptor.is_edit_position() && descriptor.is_assigned {
                    self.password_char
                } else {
                    ch
                }
            })
            .collect()
    }

    /// The formatted string, with prompts and literals according to
    /// [`Self::include_prompt`] and [`Self::include_literals`]; the password
    /// character is used unless `ignore_password_char`.
    pub fn to_string_ignore_password(&self, ignore_password_char: bool) -> String {
        self.to_string_with(ignore_password_char, self.include_prompt, self.include_literals, 0, self.length())
    }

    /// A part of the formatted string, with prompts and literals according to
    /// [`Self::include_prompt`] and [`Self::include_literals`], ignoring the
    /// password character.
    pub fn to_string_range(&self, start_position: i32, length: i32) -> String {
        self.to_string_with(true, self.include_prompt, self.include_literals, start_position, length)
    }

    /// A part of the formatted string, with prompts and literals according to
    /// [`Self::include_prompt`] and [`Self::include_literals`]; the password
    /// character is used unless `ignore_password_char`.
    pub fn to_string_ignore_password_range(&self, ignore_password_char: bool, start_position: i32, length: i32) -> String {
        self.to_string_with(ignore_password_char, self.include_prompt, self.include_literals, start_position, length)
    }

    /// The formatted string with or without prompts and literals, ignoring
    /// the password character.
    pub fn to_string_include(&self, include_prompt: bool, include_literals: bool) -> String {
        self.to_string_with(true, include_prompt, include_literals, 0, self.length())
    }

    /// A part of the formatted string with or without prompts and literals,
    /// ignoring the password character.
    pub fn to_string_include_range(&self, include_prompt: bool, include_literals: bool, start_position: i32, length: i32) -> String {
        self.to_string_with(true, include_prompt, include_literals, start_position, length)
    }

    /// A part of the formatted string.
    ///
    /// * `ignore_password_char` — show the input even when there is a
    ///   password character.
    /// * `include_prompt` — keep the prompt at unassigned positions; without
    ///   it they become spaces, and the string ends at the last assigned
    ///   position (or the last literal, when literals are included).
    /// * `include_literals` — keep the literals and separators.
    /// * `start_position`, `length` — the part of the test string; clamped to
    ///   it.
    pub fn to_string_with(
        &self,
        ignore_password_char: bool,
        include_prompt: bool,
        include_literals: bool,
        mut start_position: i32,
        mut length: i32,
    ) -> String {
        if length <= 0 {
            return String::new();
        }

        if start_position < 0 {
            start_position = 0;
        }

        if start_position >= self.length() {
            return String::new();
        }

        let max_length = self.length() - start_position;

        if length > max_length {
            length = max_length;
        }

        // The text may not need to be formatted.
        if (!self.is_password() || ignore_password_char) && include_prompt && include_literals {
            // The test string contains just what is wanted.
            return self.test_string[start_position as usize..(start_position + length) as usize].iter().collect();
        }

        // Build the formatted string.
        let mut st = String::new();
        let mut last_position = start_position + length - 1;

        if !include_prompt {
            // Without prompts the string ends at the last assigned character
            // (or literal, when literals are included).
            let last_literal_pos = if include_literals {
                self.find_non_edit_position_in_range(start_position, last_position, BACKWARD)
            } else {
                INVALID_INDEX
            };
            let last_assigned_pos = self.find_assigned_edit_position_in_range(
                if last_literal_pos == INVALID_INDEX { start_position } else { last_literal_pos },
                last_position,
                BACKWARD,
            );

            // If there is an assigned position after the last literal, it is the last position to include.
            last_position = if last_assigned_pos != INVALID_INDEX { last_assigned_pos } else { last_literal_pos };

            if last_position == INVALID_INDEX {
                return String::new();
            }
        }

        for position in start_position..=last_position {
            let ch = self.test_string[position as usize];
            let descriptor = &self.string_descriptor[position as usize];

            match descriptor.char_type {
                CharType::EditOptional | CharType::EditRequired => {
                    if descriptor.is_assigned {
                        if self.is_password() && !ignore_password_char {
                            // Replace the input with the password character.
                            st.push(self.password_char);
                            continue;
                        }
                    } else if !include_prompt {
                        // Replace the prompt with a space.
                        st.push(SPACE_CHAR);
                        continue;
                    }

                    st.push(ch);
                }
                CharType::Separator | CharType::Literal => {
                    if include_literals {
                        st.push(ch);
                    }
                }
            }
        }

        st
    }

    /// Tests whether `input` would be accepted at `position`, without
    /// changing anything. Returns the result and its hint.
    pub fn verify_char(&self, input: char, position: i32) -> (bool, MaskedTextResultHint) {
        if position < 0 || position >= self.length() {
            return (false, MaskedTextResultHint::PositionOutOfRange);
        }

        self.test_char(input, position)
    }

    /// Tests whether `input` would be escaped at `position`: skipped as a
    /// matching literal, or taken as a reset (prompt or space).
    pub fn verify_escape_char(&self, input: char, position: i32) -> bool {
        if position < 0 || position >= self.length() {
            return false;
        }

        self.test_escape_char(input, position)
    }

    /// Tests whether `input` could be set ([`Self::set`]), without changing
    /// anything.
    pub fn verify_string(&self, input: &str) -> bool {
        self.verify_string_with_hint(input).0
    }

    /// As [`Self::verify_string`]; also returns the position of the last
    /// character tested (on failure: the failing position) and the result
    /// hint.
    pub fn verify_string_with_hint(&self, input: &str) -> (bool, i32, MaskedTextResultHint) {
        if input.is_empty() {
            // Nothing to verify.
            return (true, 0, MaskedTextResultHint::NoEffect);
        }

        self.test_string(input, 0)
    }
}

/// The formatted string with prompts and literals according to
/// [`MaskedTextProvider::include_prompt`] and
/// [`MaskedTextProvider::include_literals`], ignoring the password character.
impl fmt::Display for MaskedTextProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_string_with(true, self.include_prompt, self.include_literals, 0, self.length()))
    }
}

/// The character of the test string at an index; panics when it is out of
/// range.
impl std::ops::Index<i32> for MaskedTextProvider {
    type Output = char;

    fn index(&self, index: i32) -> &char {
        if index < 0 || index >= self.length() {
            panic!("index {index} is out of range of the test string (length {})", self.length());
        }

        &self.test_string[index as usize]
    }
}

// ---- Character classification (per UTF-16 code unit in the original) ----

/// The general category of a character; `None` outside the BMP, where the
/// original sees surrogates, which belong to none of the tested categories.
#[inline]
fn category(c: char) -> Option<GeneralCategory> {
    let value = c as u32;
    if value > 0xFFFF {
        return None;
    }

    Some(Codepoint::new(value).general_category())
}

/// Lu, Ll, Lt, Lm, Lo.
fn is_letter(c: char) -> bool {
    if c.is_ascii() {
        return c.is_ascii_alphabetic();
    }

    matches!(
        category(c),
        Some(
            GeneralCategory::UppercaseLetter
                | GeneralCategory::LowercaseLetter
                | GeneralCategory::TitlecaseLetter
                | GeneralCategory::ModifierLetter
                | GeneralCategory::OtherLetter
        )
    )
}

/// Nd.
fn is_digit(c: char) -> bool {
    if c.is_ascii() {
        return c.is_ascii_digit();
    }

    category(c) == Some(GeneralCategory::DecimalNumber)
}

/// Lu.
fn is_upper(c: char) -> bool {
    if c.is_ascii() {
        return c.is_ascii_uppercase();
    }

    category(c) == Some(GeneralCategory::UppercaseLetter)
}

/// Pc, Pd, Ps, Pe, Pi, Pf, Po.
fn is_punctuation(c: char) -> bool {
    matches!(
        category(c),
        Some(
            GeneralCategory::ConnectorPunctuation
                | GeneralCategory::DashPunctuation
                | GeneralCategory::OpenPunctuation
                | GeneralCategory::ClosePunctuation
                | GeneralCategory::InitialPunctuation
                | GeneralCategory::FinalPunctuation
                | GeneralCategory::OtherPunctuation
        )
    )
}

/// Sm, Sc, Sk, So.
fn is_symbol(c: char) -> bool {
    matches!(
        category(c),
        Some(GeneralCategory::MathSymbol | GeneralCategory::CurrencySymbol | GeneralCategory::ModifierSymbol | GeneralCategory::OtherSymbol)
    )
}

/// The characters that are valid in a mask and as input.
fn is_printable_char(c: char) -> bool {
    is_letter(c) || is_digit(c) || is_punctuation(c) || is_symbol(c) || c == SPACE_CHAR
}

/// The printable ASCII characters, the space excluded (`!` to `~`).
fn is_ascii(c: char) -> bool {
    ('!'..='~').contains(&c)
}

fn is_ascii_alphanumeric(c: char) -> bool {
    c.is_ascii_alphanumeric()
}

fn is_alphanumeric(c: char) -> bool {
    is_letter(c) || is_digit(c)
}

fn is_ascii_letter(c: char) -> bool {
    c.is_ascii_alphabetic()
}

/// The lower case of a character, when it is a single BMP character.
fn to_lower(c: char) -> char {
    let mut mapped = c.to_lowercase();
    match (mapped.next(), mapped.next()) {
        (Some(lower), None) if (lower as u32) <= 0xFFFF => lower,
        _ => c,
    }
}

/// The upper case of a character, when it is a single BMP character.
fn to_upper(c: char) -> char {
    // The Greek letters with an iota subscript have no single upper case
    // character; the simple case mapping the original uses gives their title
    // case form.
    match c {
        '\u{1F80}'..='\u{1F87}' | '\u{1F90}'..='\u{1F97}' | '\u{1FA0}'..='\u{1FA7}' => {
            return char::from_u32(c as u32 + 8).unwrap_or(c);
        }
        '\u{1FB3}' | '\u{1FC3}' | '\u{1FF3}' => return char::from_u32(c as u32 + 9).unwrap_or(c),
        _ => {}
    }

    let mut mapped = c.to_uppercase();
    match (mapped.next(), mapped.next()) {
        (Some(upper), None) if (upper as u32) <= 0xFFFF => upper,
        _ => c,
    }
}

#[cfg(test)]
mod tests {
    // The expectations of these tests were checked against the original
    // class running on .NET 10.
    use super::MaskedTextResultHint as H;
    use super::*;

    fn provider(mask: &str) -> MaskedTextProvider {
        MaskedTextProvider::new(mask).unwrap()
    }

    fn filled(mask: &str, input: &str) -> MaskedTextProvider {
        let mut p = provider(mask);
        assert!(p.add(input), "{input:?} does not fit {mask:?}");
        p
    }

    fn en_us() -> CultureInfo {
        culture_with_separators("en-US", ".", ",", "/", ":", "$")
    }

    // ---- Construction, defaults, properties ----

    #[test]
    fn result_hint_values() {
        assert_eq!(H::Unknown as i32, 0);
        assert_eq!(H::CharacterEscaped as i32, 1);
        assert_eq!(H::NoEffect as i32, 2);
        assert_eq!(H::SideEffect as i32, 3);
        assert_eq!(H::Success as i32, 4);
        assert_eq!(H::AsciiCharacterExpected as i32, -1);
        assert_eq!(H::AlphanumericCharacterExpected as i32, -2);
        assert_eq!(H::DigitExpected as i32, -3);
        assert_eq!(H::LetterExpected as i32, -4);
        assert_eq!(H::SignedDigitExpected as i32, -5);
        assert_eq!(H::InvalidInput as i32, -51);
        assert_eq!(H::PromptCharNotAllowed as i32, -52);
        assert_eq!(H::UnavailableEditPosition as i32, -53);
        assert_eq!(H::NonEditPosition as i32, -54);
        assert_eq!(H::PositionOutOfRange as i32, -55);
    }

    #[test]
    fn get_operation_result_from_hint_is_true_for_positive_hints() {
        for hint in [H::CharacterEscaped, H::NoEffect, H::SideEffect, H::Success] {
            assert!(MaskedTextProvider::get_operation_result_from_hint(hint));
        }
        for hint in [H::Unknown, H::AsciiCharacterExpected, H::DigitExpected, H::InvalidInput, H::PositionOutOfRange] {
            assert!(!MaskedTextProvider::get_operation_result_from_hint(hint));
        }
    }

    #[test]
    fn constructor_defaults() {
        let p = provider("00/00");

        assert_eq!(p.mask(), "00/00");
        assert!(p.allow_prompt_as_input());
        assert!(!p.ascii_only());
        assert_eq!(p.prompt_char(), '_');
        assert_eq!(p.password_char(), '\0');
        assert!(!p.is_password());
        assert!(p.include_literals());
        assert!(!p.include_prompt());
        assert!(p.reset_on_prompt());
        assert!(p.reset_on_space());
        assert!(p.skip_literals());
        assert_eq!(p.culture(), &CultureInfo::current_culture());
        assert_eq!(p.length(), 5);
        assert_eq!(p.edit_position_count(), 4);
        assert_eq!(p.assigned_edit_position_count(), 0);
        assert_eq!(p.available_edit_position_count(), 4);
        assert_eq!(p.last_assigned_position(), -1);
        assert!(!p.mask_completed());
        assert!(!p.mask_full());
        assert_eq!(p.to_display_string(), "__/__");
        assert_eq!(p.to_string(), "  /");
        assert_eq!(MaskedTextProvider::default_password_char(), '*');
        assert_eq!(MaskedTextProvider::DEFAULT_PASSWORD_CHAR, '*');
        assert_eq!(MaskedTextProvider::invalid_index(), -1);
        assert_eq!(MaskedTextProvider::INVALID_INDEX, -1);
    }

    #[test]
    fn constructor_overloads() {
        assert!(MaskedTextProvider::new_with_ascii("LL", true).unwrap().ascii_only());
        assert_eq!(MaskedTextProvider::new_with_culture("$0", Some(en_us())).unwrap().to_display_string(), "$_");
        let p = MaskedTextProvider::new_with_culture_ascii("$0", Some(en_us()), true).unwrap();
        assert!(p.ascii_only());
        assert_eq!(p.culture(), &en_us());

        let mut p = MaskedTextProvider::new_with_password("00", '*', false).unwrap();
        assert!(!p.allow_prompt_as_input());
        assert!(p.is_password());
        assert!(p.add("12"));
        assert_eq!(p.to_display_string(), "**");
        assert_eq!(p.to_string(), "12");

        let p = MaskedTextProvider::new_with_culture_password("$0", Some(en_us()), '#', true).unwrap();
        assert_eq!(p.password_char(), '#');
        assert!(p.allow_prompt_as_input());

        let p = MaskedTextProvider::new_full("00", None, false, '#', 'x', true).unwrap();
        assert_eq!(p.prompt_char(), '#');
        assert_eq!(p.password_char(), 'x');
        assert!(p.ascii_only());
        assert!(!p.allow_prompt_as_input());
        assert_eq!(p.to_display_string(), "##");
    }

    #[test]
    fn constructor_rejects_empty_and_invalid_masks() {
        assert_eq!(MaskedTextProvider::new("").unwrap_err(), MaskedTextProviderError::MaskNullOrEmpty);
        assert_eq!(MaskedTextProvider::new("0\n").unwrap_err(), MaskedTextProviderError::MaskInvalidChar);
        assert_eq!(MaskedTextProvider::new("0\t0").unwrap_err(), MaskedTextProviderError::MaskInvalidChar);
        // A character outside the BMP is a surrogate pair in the original, which is not valid.
        assert_eq!(MaskedTextProvider::new("0\u{1F600}").unwrap_err(), MaskedTextProviderError::MaskInvalidChar);
        assert!(!MaskedTextProviderError::MaskNullOrEmpty.to_string().is_empty());
    }

    #[test]
    fn constructor_does_not_validate_prompt_and_password_chars() {
        // Only the setters validate, as in the original.
        let p = MaskedTextProvider::new_full("00", None, true, '*', '*', false).unwrap();
        assert_eq!(p.to_display_string(), "**");
        assert!(MaskedTextProvider::new_full("00", None, true, '\n', '\0', false).is_ok());
        assert!(MaskedTextProvider::new_full("00", None, true, '_', '\n', false).is_ok());
    }

    #[test]
    fn valid_char_predicates() {
        for c in ['a', 'Z', '0', '-', '$', ' ', '_', '*', '\u{434}', '\u{A4}', '+', '(', '~'] {
            assert!(MaskedTextProvider::is_valid_input_char(c), "{c:?}");
            assert!(MaskedTextProvider::is_valid_mask_char(c), "{c:?}");
            assert!(MaskedTextProvider::is_valid_password_char(c), "{c:?}");
        }
        for c in ['\n', '\t', '\r', '\u{7F}', '\u{A0}', '\u{1F600}'] {
            assert!(!MaskedTextProvider::is_valid_input_char(c), "{c:?}");
            assert!(!MaskedTextProvider::is_valid_mask_char(c), "{c:?}");
            assert!(!MaskedTextProvider::is_valid_password_char(c), "{c:?}");
        }
        assert!(!MaskedTextProvider::is_valid_input_char('\0'));
        assert!(!MaskedTextProvider::is_valid_mask_char('\0'));
        assert!(MaskedTextProvider::is_valid_password_char('\0'));
    }

    #[test]
    fn password_char_setter_validates() {
        let mut p = provider("00");

        assert_eq!(p.set_password_char('_'), Err(MaskedTextProviderError::PasswordAndPromptCharEqual));
        assert_eq!(p.set_password_char('\n'), Err(MaskedTextProviderError::InvalidChar));
        assert_eq!(p.password_char(), '\0');
        assert_eq!(p.set_password_char('#'), Ok(()));
        assert!(p.is_password());
        assert_eq!(p.set_password_char('\0'), Ok(()));
        assert!(!p.is_password());
    }

    #[test]
    fn is_password_setter_uses_default_password_char() {
        let mut p = provider("00");

        p.set_is_password(true);
        assert_eq!(p.password_char(), '*');
        p.set_is_password(false);
        assert_eq!(p.password_char(), '\0');

        // An existing password character is kept.
        p.set_password_char('x').unwrap();
        p.set_is_password(true);
        assert_eq!(p.password_char(), 'x');
    }

    #[test]
    fn prompt_char_setter_validates_and_updates_unassigned_positions() {
        let mut p = MaskedTextProvider::new_full("00-00", None, true, '_', '*', false).unwrap();
        assert!(p.add("1"));

        assert_eq!(p.set_prompt_char('*'), Err(MaskedTextProviderError::PasswordAndPromptCharEqual));
        assert_eq!(p.set_prompt_char('\n'), Err(MaskedTextProviderError::InvalidChar));
        assert_eq!(p.prompt_char(), '_');

        assert_eq!(p.set_prompt_char('#'), Ok(()));
        assert_eq!(p.prompt_char(), '#');
        assert_eq!(p.to_string_include(true, true), "1#-##");
        assert_eq!(p.to_display_string(), "*#-##");
    }

    #[test]
    fn flag_setters() {
        let mut p = provider("00");
        p.set_include_literals(false);
        p.set_include_prompt(true);
        p.set_reset_on_prompt(false);
        p.set_reset_on_space(false);
        p.set_skip_literals(false);
        assert!(!p.include_literals());
        assert!(p.include_prompt());
        assert!(!p.reset_on_prompt());
        assert!(!p.reset_on_space());
        assert!(!p.skip_literals());
    }

    #[test]
    fn indexer_returns_the_test_string_characters() {
        let p = filled("00-00", "12");
        assert_eq!(p[0], '1');
        assert_eq!(p[2], '-');
        assert_eq!(p[4], '_');
        assert_eq!(p.char_at(1), '2');
    }

    #[test]
    #[should_panic]
    fn indexer_panics_past_the_end() {
        let p = provider("00");
        let _ = p[2];
    }

    #[test]
    #[should_panic]
    fn char_at_panics_for_negative_index() {
        let p = provider("00");
        let _ = p.char_at(-1);
    }

    #[test]
    fn clone_is_independent_and_keeps_the_state() {
        let mut p = MaskedTextProvider::new_full("00-00", Some(en_us()), false, '#', '*', true).unwrap();
        p.set_reset_on_prompt(false);
        p.set_reset_on_space(false);
        p.set_skip_literals(false);
        p.set_include_literals(false);
        p.set_include_prompt(true);
        assert!(p.add("123"));

        let mut clone = p.clone();
        assert_eq!(clone.mask(), "00-00");
        assert_eq!(clone.culture(), &en_us());
        assert_eq!(clone.prompt_char(), '#');
        assert_eq!(clone.password_char(), '*');
        assert!(clone.ascii_only());
        assert!(!clone.allow_prompt_as_input());
        assert!(!clone.reset_on_prompt());
        assert!(!clone.reset_on_space());
        assert!(!clone.skip_literals());
        assert!(!clone.include_literals());
        assert!(clone.include_prompt());
        assert_eq!(clone.to_string_include(true, true), "12-3#");
        assert_eq!(clone.assigned_edit_position_count(), 3);

        assert!(clone.add("4"));
        assert_eq!(clone.to_string_include(true, true), "12-34");
        assert_eq!(p.to_string_include(true, true), "12-3#");
    }

    // ---- The mask language ----

    #[test]
    fn mask_is_translated_into_the_test_string() {
        let p = provider("(999) 000-0000");
        assert_eq!(p.to_display_string(), "(___) ___-____");
        assert_eq!(p.edit_position_count(), 10);
        assert_eq!(p.edit_positions().collect::<Vec<_>>(), vec![1, 2, 3, 6, 7, 8, 10, 11, 12, 13]);

        // Modifiers do not take a position.
        let p = provider(">LL<LL|LL");
        assert_eq!(p.length(), 6);
        assert_eq!(p.to_display_string(), "______");

        // An escaped mask character is a literal.
        let p = provider("\\00");
        assert_eq!(p.to_display_string(), "0_");
        assert_eq!(p.edit_positions().collect::<Vec<_>>(), vec![1]);
        let p = provider("0\\\\0\\L\\<");
        assert_eq!(p.to_display_string(), "_\\_L<");
        assert_eq!(p.edit_position_count(), 2);

        // A mask without edit positions is complete and full.
        let p = provider("xbc");
        assert_eq!(p.to_display_string(), "xbc");
        assert_eq!(p.edit_position_count(), 0);
        assert!(p.mask_completed());
        assert!(p.mask_full());
    }

    #[test]
    fn required_and_optional_elements_drive_mask_completed_and_mask_full() {
        // Required: 0 L & A; optional: 9 # ? C a.
        let mut p = provider("0L&A9#?Ca");
        assert_eq!(p.edit_position_count(), 9);
        assert!(!p.mask_completed());

        assert!(p.add("1bc"));
        assert!(!p.mask_completed());
        assert!(p.add("d"));
        assert!(p.mask_completed());
        assert!(!p.mask_full());
        assert_eq!(p.available_edit_position_count(), 5);

        assert!(p.add("5+efg"));
        assert!(p.mask_full());
        assert_eq!(p.to_display_string(), "1bcd5+efg");
        assert_eq!(p.available_edit_position_count(), 0);

        assert!(p.remove());
        assert!(p.mask_completed());
        assert!(!p.mask_full());
        // Removing would shift a letter into the digit position.
        assert!(!p.remove_at(0));
        assert!(p.mask_completed());
        // Resetting a required position makes the mask incomplete.
        assert!(p.replace_char(' ', 0));
        assert!(!p.mask_completed());
    }

    #[test]
    fn mask_completed_with_only_optional_positions() {
        let p = provider("99");
        assert!(p.mask_completed());
        assert!(!p.mask_full());
    }

    /// The mask used to probe every edit element with `verify_char`.
    const ELEMENTS: &str = "09#L?&CAa";

    fn probe(ascii: bool) -> MaskedTextProvider {
        let mut p = MaskedTextProvider::new_with_ascii(ELEMENTS, ascii).unwrap();
        // The space and the prompt are tested against the mask instead of resetting.
        p.set_reset_on_space(false);
        p.set_reset_on_prompt(false);
        p
    }

    #[test]
    fn verify_char_digit_elements() {
        let p = probe(false);

        // 0: digit required.
        assert_eq!(p.verify_char('5', 0), (true, H::Success));
        assert_eq!(p.verify_char('a', 0), (false, H::DigitExpected));
        assert_eq!(p.verify_char(' ', 0), (false, H::DigitExpected));
        assert_eq!(p.verify_char('+', 0), (false, H::DigitExpected));
        // A non-ASCII decimal digit is a digit.
        assert_eq!(p.verify_char('\u{663}', 0), (true, H::Success));

        // 9: digit or space.
        assert_eq!(p.verify_char('5', 1), (true, H::Success));
        assert_eq!(p.verify_char(' ', 1), (true, H::Success));
        assert_eq!(p.verify_char('-', 1), (false, H::DigitExpected));

        // #: digit, space or sign.
        assert_eq!(p.verify_char('5', 2), (true, H::Success));
        assert_eq!(p.verify_char(' ', 2), (true, H::Success));
        assert_eq!(p.verify_char('+', 2), (true, H::Success));
        assert_eq!(p.verify_char('-', 2), (true, H::Success));
        assert_eq!(p.verify_char('a', 2), (false, H::DigitExpected));
    }

    #[test]
    fn verify_char_letter_elements() {
        let p = probe(false);

        // L: letter required.
        assert_eq!(p.verify_char('a', 3), (true, H::Success));
        assert_eq!(p.verify_char('\u{434}', 3), (true, H::Success));
        assert_eq!(p.verify_char('1', 3), (false, H::LetterExpected));
        assert_eq!(p.verify_char(' ', 3), (false, H::LetterExpected));

        // ?: letter or space.
        assert_eq!(p.verify_char('a', 4), (true, H::Success));
        assert_eq!(p.verify_char(' ', 4), (true, H::Success));
        assert_eq!(p.verify_char('1', 4), (false, H::LetterExpected));
    }

    #[test]
    fn verify_char_any_and_alphanumeric_elements() {
        let p = probe(false);

        // &: any character required (the space is a character).
        assert_eq!(p.verify_char('!', 5), (true, H::Success));
        assert_eq!(p.verify_char(' ', 5), (true, H::Success));
        assert_eq!(p.verify_char('\u{434}', 5), (true, H::Success));

        // C: any character optional.
        assert_eq!(p.verify_char('!', 6), (true, H::Success));
        assert_eq!(p.verify_char(' ', 6), (true, H::Success));

        // A: alphanumeric required.
        assert_eq!(p.verify_char('a', 7), (true, H::Success));
        assert_eq!(p.verify_char('1', 7), (true, H::Success));
        assert_eq!(p.verify_char('\u{DC}', 7), (true, H::Success));
        assert_eq!(p.verify_char('!', 7), (false, H::AlphanumericCharacterExpected));
        assert_eq!(p.verify_char(' ', 7), (false, H::AlphanumericCharacterExpected));

        // a: alphanumeric or space.
        assert_eq!(p.verify_char('a', 8), (true, H::Success));
        assert_eq!(p.verify_char(' ', 8), (true, H::Success));
        assert_eq!(p.verify_char('!', 8), (false, H::AlphanumericCharacterExpected));
    }

    #[test]
    fn verify_char_ascii_only() {
        let p = probe(true);

        // The digit elements are not affected.
        assert_eq!(p.verify_char('\u{663}', 0), (true, H::Success));

        assert_eq!(p.verify_char('a', 3), (true, H::Success));
        assert_eq!(p.verify_char('\u{434}', 3), (false, H::AsciiCharacterExpected));
        assert_eq!(p.verify_char('\u{434}', 4), (false, H::AsciiCharacterExpected));
        // The space passes the letter test of `?` but not its ASCII letter test.
        assert_eq!(p.verify_char(' ', 4), (false, H::AsciiCharacterExpected));

        assert_eq!(p.verify_char('!', 5), (true, H::Success));
        assert_eq!(p.verify_char('\u{434}', 5), (false, H::AsciiCharacterExpected));
        // "ASCII" is `!` to `~` for `&`: the space is not in it...
        assert_eq!(p.verify_char(' ', 5), (false, H::AsciiCharacterExpected));
        // ...but `C` accepts it explicitly.
        assert_eq!(p.verify_char(' ', 6), (true, H::Success));
        assert_eq!(p.verify_char('\u{434}', 6), (false, H::AsciiCharacterExpected));

        assert_eq!(p.verify_char('z', 7), (true, H::Success));
        assert_eq!(p.verify_char('9', 7), (true, H::Success));
        assert_eq!(p.verify_char('\u{DC}', 7), (false, H::AsciiCharacterExpected));
        assert_eq!(p.verify_char('\u{DC}', 8), (false, H::AsciiCharacterExpected));
        assert_eq!(p.verify_char(' ', 8), (false, H::AsciiCharacterExpected));
    }

    #[test]
    fn verify_char_position_and_input_validation() {
        let p = probe(false);

        assert_eq!(p.verify_char('1', -1), (false, H::PositionOutOfRange));
        assert_eq!(p.verify_char('1', 9), (false, H::PositionOutOfRange));
        assert_eq!(p.verify_char('\n', 0), (false, H::InvalidInput));
        assert_eq!(p.verify_char('\0', 6), (false, H::InvalidInput));
        assert_eq!(p.verify_char('\u{1F600}', 6), (false, H::InvalidInput));
    }

    #[test]
    fn verify_char_reports_no_effect_for_the_same_character() {
        let p = filled("00", "1");
        assert_eq!(p.verify_char('1', 0), (true, H::NoEffect));
        assert_eq!(p.verify_char('2', 0), (true, H::Success));
        assert_eq!(p.verify_char('1', 1), (true, H::Success));
    }

    #[test]
    fn verify_char_on_literals() {
        let mut p = provider("00-00");
        assert_eq!(p.verify_char('-', 2), (true, H::CharacterEscaped));
        assert_eq!(p.verify_char('1', 2), (false, H::NonEditPosition));
        assert!(p.verify_escape_char('-', 2));
        assert!(!p.verify_escape_char('1', 2));

        p.set_skip_literals(false);
        assert_eq!(p.verify_char('-', 2), (false, H::NonEditPosition));
        assert!(!p.verify_escape_char('-', 2));
    }

    #[test]
    fn verify_escape_char_on_edit_positions() {
        let mut p = provider("00-00");
        assert!(p.verify_escape_char(' ', 0));
        assert!(p.verify_escape_char('_', 0));
        assert!(!p.verify_escape_char('1', 0));
        assert!(!p.verify_escape_char(' ', -1));
        assert!(!p.verify_escape_char(' ', 5));

        p.set_reset_on_space(false);
        assert!(!p.verify_escape_char(' ', 0));
        assert!(p.verify_escape_char('_', 0));
        p.set_reset_on_prompt(false);
        assert!(!p.verify_escape_char('_', 0));
    }

    #[test]
    fn case_conversion_modifiers() {
        let mut p = provider(">LL<LL|LL");
        assert!(p.add("abCDeF"));
        assert_eq!(p.to_display_string(), "ABcdeF");

        // Digits and already converted letters are left alone.
        let mut p = provider(">AAA");
        assert!(p.add("a1B"));
        assert_eq!(p.to_display_string(), "A1B");

        // Non-ASCII letters are converted too.
        let mut p = provider("<L>L");
        assert!(p.add("\u{DC}\u{434}"));
        assert_eq!(p.to_display_string(), "\u{FC}\u{414}");

        // The conversion applies to characters that shift into a position.
        let mut p = provider(">L<L");
        assert!(p.add("a"));
        assert_eq!(p.to_display_string(), "A_");
        assert!(p.insert_at("b", 0));
        assert_eq!(p.to_display_string(), "Ba");
    }

    #[test]
    fn culture_separators() {
        assert_eq!(CultureInfo::current_culture(), CultureInfo::invariant_culture());

        let p = MaskedTextProvider::new_with_culture("$0,000.00 00/00 00:00", Some(CultureInfo::invariant_culture())).unwrap();
        assert_eq!(p.to_display_string(), "\u{A4}_,___.__ __/__ __:__");
        assert_eq!(provider("$0").to_display_string(), "\u{A4}_");

        // A German-like culture: the separators are swapped.
        let de = culture_with_separators("de-DE", ",", ".", ".", ":", "\u{20AC}");
        let p = MaskedTextProvider::new_with_culture("$0.0,0/0:0", Some(de)).unwrap();
        assert_eq!(p.to_display_string(), "\u{20AC}_,_._._:_");
        assert_eq!(p.length(), 10);

        // Separators can be longer than one character; every character is a non-edit position.
        let ch = culture_with_separators("de-CH", ".", "'", ".", ":", "CHF");
        let mut p = MaskedTextProvider::new_with_culture("$0.0,0/0:0", Some(ch)).unwrap();
        assert_eq!(p.to_display_string(), "CHF_._'_._:_");
        assert_eq!(p.length(), 12);
        assert_eq!(p.edit_positions().collect::<Vec<_>>(), vec![3, 5, 7, 9, 11]);
        assert!(!p.is_edit_position(1));
        assert_eq!(p.find_non_edit_position_from(3, true), 4);

        // The separator characters of the culture are skipped in the input, not the mask's.
        assert!(p.add("CHF1.2'3"));
        assert_eq!(p.to_display_string(), "CHF1.2'3._:_");
        assert_eq!(p.verify_char(',', 6), (false, H::NonEditPosition));
    }

    #[test]
    fn escaped_separator_is_not_localised() {
        let de = culture_with_separators("de-DE", ",", ".", ".", ":", "\u{20AC}");
        let p = MaskedTextProvider::new_with_culture("0\\.0.0", Some(de)).unwrap();
        assert_eq!(p.to_display_string(), "_._,_");
    }

    // ---- Add ----

    #[test]
    fn add_char_goes_after_the_last_assigned_position() {
        let mut p = provider("00-00");

        assert_eq!(p.add_char_with_hint('1'), (true, 0, H::Success));
        assert_eq!(p.add_char_with_hint('2'), (true, 1, H::Success));
        // The literal is skipped.
        assert_eq!(p.add_char_with_hint('3'), (true, 3, H::Success));
        assert_eq!(p.add_char_with_hint('a'), (false, 4, H::DigitExpected));
        assert_eq!(p.to_display_string(), "12-3_");
        assert!(p.add_char('4'));
        assert_eq!(p.add_char_with_hint('5'), (false, 5, H::UnavailableEditPosition));
        assert!(!p.add_char('5'));
        assert_eq!(p.to_display_string(), "12-34");
    }

    #[test]
    fn add_char_without_edit_position_after_the_last_assigned() {
        let mut p = filled("00-", "12");
        assert_eq!(p.add_char_with_hint('3'), (false, 3, H::UnavailableEditPosition));
    }

    #[test]
    fn add_char_escaped_space_assigns_nothing() {
        let mut p = provider("00");
        assert_eq!(p.add_char_with_hint(' '), (true, 0, H::CharacterEscaped));
        assert_eq!(p.assigned_edit_position_count(), 0);
        // The next character still goes to the first position.
        assert_eq!(p.add_char_with_hint('1'), (true, 0, H::Success));
    }

    #[test]
    fn add_string() {
        let mut p = provider("00-00");

        assert_eq!(p.add_with_hint("12-34"), (true, 4, H::Success));
        assert_eq!(p.to_display_string(), "12-34");
        assert!(p.mask_full());
        assert_eq!(p.add_with_hint(""), (true, 5, H::NoEffect));
        assert_eq!(p.add_with_hint("1"), (false, 5, H::UnavailableEditPosition));

        // Literals do not have to be part of the input.
        let mut p = provider("00-00");
        assert_eq!(p.add_with_hint("1234"), (true, 4, H::Success));
        assert_eq!(p.to_display_string(), "12-34");

        // Strings are appended after the last assigned position.
        let mut p = provider("00-00");
        assert_eq!(p.add_with_hint("12"), (true, 1, H::Success));
        assert_eq!(p.add_with_hint("3"), (true, 3, H::Success));
        assert_eq!(p.to_display_string(), "12-3_");
    }

    #[test]
    fn add_string_is_all_or_nothing() {
        let mut p = provider("00-00");

        assert_eq!(p.add_with_hint("12a"), (false, 3, H::DigitExpected));
        assert_eq!(p.to_display_string(), "__-__");
        assert_eq!(p.add_with_hint("12345"), (false, 5, H::UnavailableEditPosition));
        assert_eq!(p.assigned_edit_position_count(), 0);
        assert!(!p.add("x"));
    }

    #[test]
    fn add_string_without_skip_literals() {
        let mut p = provider("00-00");
        p.set_skip_literals(false);

        assert_eq!(p.add_with_hint("12-34"), (false, 3, H::DigitExpected));
        assert_eq!(p.add_with_hint("1234"), (true, 4, H::Success));
        assert_eq!(p.to_display_string(), "12-34");
    }

    // ---- Clear / Remove ----

    #[test]
    fn clear_resets_all_positions() {
        let mut p = filled("00-00", "123");

        assert_eq!(p.clear_with_hint(), H::Success);
        assert_eq!(p.to_display_string(), "__-__");
        assert_eq!(p.assigned_edit_position_count(), 0);
        assert!(!p.mask_completed());
        assert_eq!(p.clear_with_hint(), H::NoEffect);
        p.clear();
        assert_eq!(p.to_display_string(), "__-__");
    }

    #[test]
    fn remove_resets_the_last_assigned_position() {
        let mut p = filled("00-00", "123");

        assert_eq!(p.remove_with_hint(), (true, 3, H::Success));
        assert_eq!(p.to_display_string(), "12-__");
        assert!(p.remove());
        assert!(p.remove());
        assert_eq!(p.remove_with_hint(), (true, 0, H::NoEffect));
        assert_eq!(p.to_display_string(), "__-__");
    }

    #[test]
    fn remove_at_shifts_the_following_characters_left() {
        let mut p = filled("00-00", "1234");

        assert_eq!(p.remove_at_range_with_hint(0, 0), (true, 0, H::Success));
        assert_eq!(p.to_display_string(), "23-4_");
        assert!(p.remove_at(1));
        assert_eq!(p.to_display_string(), "24-__");
    }

    #[test]
    fn remove_at_last_position_does_not_shift() {
        // The backspace at the end of a programmatically set text.
        let mut p = provider("00:00:00.000");
        assert!(p.set("12:34:56.000"));
        assert!(p.remove_at(11));
        assert_eq!(p.to_display_string(), "12:34:56.00_");
    }

    #[test]
    fn remove_at_range_across_a_literal() {
        let mut p = filled("00-00", "1234");
        assert_eq!(p.remove_at_range_with_hint(1, 3), (true, 1, H::Success));
        assert_eq!(p.to_display_string(), "14-__");

        let mut p = filled("00-00", "1234");
        assert!(p.remove_at_range(0, 4));
        assert_eq!(p.to_display_string(), "__-__");
    }

    #[test]
    fn remove_at_a_literal_has_no_effect() {
        let mut p = filled("00-00", "1234");
        assert_eq!(p.remove_at_range_with_hint(2, 2), (true, 2, H::NoEffect));
        assert_eq!(p.to_display_string(), "12-34");
    }

    #[test]
    fn remove_at_after_the_last_assigned_position_has_no_effect() {
        let mut p = filled("00-00", "12");
        assert_eq!(p.remove_at_range_with_hint(3, 4), (true, 3, H::NoEffect));
        assert_eq!(p.to_display_string(), "12-__");
    }

    #[test]
    fn remove_at_unassigned_position_shifts_as_a_side_effect() {
        let mut p = provider("000");
        assert!(p.replace_char('5', 2));
        assert_eq!(p.to_display_string(), "__5");

        assert_eq!(p.remove_at_range_with_hint(0, 0), (true, 0, H::SideEffect));
        assert_eq!(p.to_display_string(), "_5_");
    }

    #[test]
    fn remove_at_fails_when_a_shifted_character_does_not_fit() {
        let mut p = provider("0L");
        assert!(p.replace_char('a', 1));
        assert_eq!(p.to_display_string(), "_a");

        assert_eq!(p.remove_at_range_with_hint(0, 0), (false, 0, H::DigitExpected));
        assert!(!p.remove_at(0));
        assert_eq!(p.to_display_string(), "_a");
    }

    #[test]
    fn remove_at_position_validation() {
        let mut p = filled("00-00", "1234");

        assert_eq!(p.remove_at_range_with_hint(0, 5), (false, 5, H::PositionOutOfRange));
        assert_eq!(p.remove_at_range_with_hint(-1, 2), (false, -1, H::PositionOutOfRange));
        assert_eq!(p.remove_at_range_with_hint(3, 2), (false, 3, H::PositionOutOfRange));
        assert!(!p.remove_at(5));
        assert!(!p.remove_at(-1));
        assert_eq!(p.to_display_string(), "12-34");
    }

    // ---- InsertAt ----

    #[test]
    fn insert_at_shifts_the_following_characters_right() {
        let mut p = filled("00000", "123");

        assert_eq!(p.insert_at_with_hint("9", 1), (true, 1, H::Success));
        assert_eq!(p.to_display_string(), "1923_");
        assert_eq!(p.insert_at_char_with_hint('8', 0), (true, 0, H::Success));
        assert_eq!(p.to_display_string(), "81923");
    }

    #[test]
    fn insert_at_shifts_across_literals() {
        let mut p = filled("00-00", "123");
        assert!(p.insert_at("9", 0));
        assert_eq!(p.to_display_string(), "91-23");
    }

    #[test]
    fn insert_at_a_literal_uses_the_next_edit_position() {
        let mut p = filled("00-00", "123");
        assert_eq!(p.insert_at_with_hint("9", 2), (true, 3, H::Success));
        assert_eq!(p.to_display_string(), "12-93");
    }

    #[test]
    fn insert_at_unassigned_position_does_not_shift() {
        let mut p = provider("000");
        assert_eq!(p.insert_at_with_hint("5", 1), (true, 1, H::Success));
        assert_eq!(p.to_display_string(), "_5_");
        assert!(p.insert_at_char('7', 2));
        assert_eq!(p.to_display_string(), "_57");
    }

    #[test]
    fn insert_at_fails_without_room() {
        let mut p = filled("000", "123");

        assert_eq!(p.insert_at_with_hint("9", 0), (false, 3, H::UnavailableEditPosition));
        assert_eq!(p.insert_at_with_hint("9", 2), (false, 3, H::UnavailableEditPosition));
        assert!(!p.insert_at_char('9', 1));
        assert_eq!(p.to_display_string(), "123");
    }

    #[test]
    fn insert_at_fails_when_a_shifted_character_does_not_fit() {
        let mut p = filled("00LL", "12");
        assert_eq!(p.insert_at_with_hint("3", 0), (false, 2, H::LetterExpected));
        assert_eq!(p.to_display_string(), "12__");
    }

    #[test]
    fn insert_at_fails_for_invalid_input() {
        let mut p = provider("00-00");
        assert_eq!(p.insert_at_with_hint("1a", 0), (false, 1, H::DigitExpected));
        assert_eq!(p.insert_at_with_hint("12345", 0), (false, 5, H::UnavailableEditPosition));
        assert_eq!(p.to_display_string(), "__-__");
    }

    #[test]
    fn insert_at_empty_string_and_position_validation() {
        let mut p = provider("000");

        assert_eq!(p.insert_at_with_hint("", 1), (true, 1, H::NoEffect));
        assert_eq!(p.insert_at_with_hint("1", 3), (false, 3, H::PositionOutOfRange));
        assert_eq!(p.insert_at_with_hint("1", -1), (false, -1, H::PositionOutOfRange));
        assert_eq!(p.insert_at_char_with_hint('1', 3), (false, 3, H::PositionOutOfRange));
        assert!(!p.insert_at_char('1', -1));
        assert!(!p.insert_at_char('1', 3));
        assert!(!p.insert_at("1", 3));
    }

    #[test]
    fn insert_at_space_opens_a_gap() {
        // The space key of a masked text box.
        let mut p = filled("0000", "123");
        assert_eq!(p.insert_at_with_hint(" ", 1), (true, 1, H::Success));
        assert_eq!(p.to_display_string(), "1_23");

        // There is no room for the gap in a full mask.
        let mut p = filled("000", "123");
        assert_eq!(p.insert_at_with_hint(" ", 1), (false, 3, H::UnavailableEditPosition));
        assert_eq!(p.to_display_string(), "123");
    }

    // ---- Replace ----

    #[test]
    fn replace_char_at_position() {
        let mut p = filled("00-00", "1234");

        assert_eq!(p.replace_char_with_hint('9', 0), (true, 0, H::Success));
        assert_eq!(p.replace_char_with_hint('9', 0), (true, 0, H::NoEffect));
        // On a literal the next edit position is replaced...
        assert_eq!(p.replace_char_with_hint('8', 2), (true, 3, H::Success));
        // ...unless the character is the literal.
        assert_eq!(p.replace_char_with_hint('-', 2), (true, 2, H::CharacterEscaped));
        assert_eq!(p.to_display_string(), "92-84");
        assert_eq!(p.replace_char_with_hint('a', 1), (false, 1, H::DigitExpected));
        assert_eq!(p.replace_char_with_hint('1', 5), (false, 5, H::PositionOutOfRange));
        assert_eq!(p.replace_char_with_hint('1', -1), (false, -1, H::PositionOutOfRange));
        assert!(!p.replace_char('a', 1));
        assert_eq!(p.to_display_string(), "92-84");
    }

    #[test]
    fn replace_char_without_edit_position() {
        let mut p = provider("00-");
        assert_eq!(p.replace_char_with_hint('1', 2), (false, 2, H::UnavailableEditPosition));
    }

    #[test]
    fn replace_string_at_position_does_not_shift() {
        let mut p = filled("00000", "12345");

        assert_eq!(p.replace_with_hint("99", 1), (true, 2, H::Success));
        assert_eq!(p.to_display_string(), "19945");
        // An empty string removes the character.
        assert_eq!(p.replace_with_hint("", 1), (true, 1, H::Success));
        assert_eq!(p.to_display_string(), "1945_");
        assert_eq!(p.replace_with_hint("1a", 0), (false, 1, H::DigitExpected));
        assert_eq!(p.replace_with_hint("1", 5), (false, 5, H::PositionOutOfRange));
        assert!(!p.replace("123456", 0));
        assert!(p.replace("7", 4));
        assert_eq!(p.to_display_string(), "19457");
    }

    #[test]
    fn replace_range_with_a_shorter_string_removes_the_rest() {
        let mut p = filled("00000", "12345");
        assert_eq!(p.replace_range_with_hint("9", 1, 3), (true, 1, H::Success));
        assert_eq!(p.to_display_string(), "195__");

        let mut p = filled("00000", "12345");
        assert_eq!(p.replace_char_range_with_hint('9', 1, 3), (true, 1, H::Success));
        assert_eq!(p.to_display_string(), "195__");
    }

    #[test]
    fn replace_range_with_a_longer_string_inserts_the_rest() {
        let mut p = filled("00000", "123");
        assert_eq!(p.replace_range_with_hint("789", 0, 0), (true, 2, H::Success));
        assert_eq!(p.to_display_string(), "78923");

        // No room for the shifted characters.
        let mut p = filled("00000", "1234");
        assert_eq!(p.replace_range_with_hint("789", 0, 0), (false, 5, H::UnavailableEditPosition));
        assert_eq!(p.to_display_string(), "1234_");
    }

    #[test]
    fn replace_range_with_a_string_of_the_same_length() {
        let mut p = filled("00-00", "1234");
        assert_eq!(p.replace_range_with_hint("98", 1, 3), (true, 3, H::Success));
        assert_eq!(p.to_display_string(), "19-84");
    }

    #[test]
    fn replace_range_with_an_empty_string_removes_the_range() {
        let mut p = filled("00000", "12345");
        assert_eq!(p.replace_range_with_hint("", 1, 2), (true, 1, H::Success));
        assert_eq!(p.to_display_string(), "145__");
    }

    #[test]
    fn replace_char_range_of_one_position_sets_that_position() {
        let mut p = filled("00-00", "1234");
        assert_eq!(p.replace_char_range_with_hint('9', 1, 1), (true, 1, H::Success));
        assert_eq!(p.to_display_string(), "19-34");
        // Unlike `replace_char`, a literal position is not skipped here.
        assert_eq!(p.replace_char_range_with_hint('9', 2, 2), (false, 2, H::NonEditPosition));
    }

    #[test]
    fn replace_range_position_validation() {
        let mut p = filled("00000", "12345");

        assert_eq!(p.replace_range_with_hint("1", 0, 5), (false, 5, H::PositionOutOfRange));
        assert_eq!(p.replace_range_with_hint("1", -1, 2), (false, -1, H::PositionOutOfRange));
        assert_eq!(p.replace_range_with_hint("1", 3, 2), (false, 3, H::PositionOutOfRange));
        assert_eq!(p.replace_char_range_with_hint('1', 0, 5), (false, 5, H::PositionOutOfRange));
        assert_eq!(p.replace_char_range_with_hint('1', 3, 2), (false, 3, H::PositionOutOfRange));
        assert_eq!(p.replace_range_with_hint("a", 1, 3), (false, 1, H::DigitExpected));
        assert_eq!(p.to_display_string(), "12345");
    }

    // ---- Set ----

    #[test]
    fn set_replaces_the_whole_content() {
        let mut p = provider("00/00/0000");

        assert_eq!(p.set_with_hint("12/10/2000"), (true, 9, H::Success));
        assert_eq!(p.to_display_string(), "12/10/2000");
        assert!(p.mask_full());

        // Setting the same text changes nothing.
        assert_eq!(p.set_with_hint("12/10/2000"), (true, 9, H::NoEffect));

        // Positions after the new text are reset.
        assert_eq!(p.set_with_hint("3"), (true, 0, H::Success));
        assert_eq!(p.to_display_string(), "3_/__/____");

        assert!(p.set("12102000"));
        assert_eq!(p.to_display_string(), "12/10/2000");
    }

    #[test]
    fn set_empty_string_clears() {
        let mut p = filled("000", "12");
        assert_eq!(p.set_with_hint(""), (true, 0, H::Success));
        assert_eq!(p.to_display_string(), "___");
        assert_eq!(p.set_with_hint(""), (true, 0, H::NoEffect));
    }

    #[test]
    fn set_invalid_text_keeps_the_previous_content() {
        let mut p = provider("000");
        assert!(p.set("123"));
        assert_eq!(p.set_with_hint("abc"), (false, 0, H::DigitExpected));
        assert!(!p.set("1234"));
        assert_eq!(p.to_display_string(), "123");

        // A text whose literal is replaced by an edit character does not fit.
        let mut p = provider("00:00:00.000");
        assert_eq!(p.set_with_hint("12:34:560000"), (false, 12, H::UnavailableEditPosition));
        assert_eq!(p.to_display_string(), "__:__:__.___");
    }

    #[test]
    fn set_accepts_its_own_display_string() {
        // A masked text box sets the provider from the text it displays.
        let mut p = provider("00/00/0000");
        assert!(p.set("12/1"));
        let display = p.to_display_string();
        assert_eq!(display, "12/1_/____");
        assert_eq!(p.set_with_hint(&display), (true, 9, H::NoEffect));
        assert_eq!(p.to_display_string(), "12/1_/____");
        assert_eq!(p.assigned_edit_position_count(), 3);
    }

    // ---- Prompt and space handling ----

    #[test]
    fn prompt_resets_the_position() {
        let mut p = filled("000", "123");

        assert_eq!(p.verify_char('_', 1), (true, H::SideEffect));
        assert_eq!(p.replace_char_with_hint('_', 1), (true, 1, H::SideEffect));
        assert_eq!(p.to_display_string(), "1_3");
        assert_eq!(p.assigned_edit_position_count(), 2);
        assert!(!p.mask_completed());
        assert_eq!(p.replace_char_with_hint('_', 1), (true, 1, H::CharacterEscaped));

        assert!(p.set("4_6"));
        assert_eq!(p.to_display_string(), "4_6");
        assert!(p.is_available_position(1));
    }

    #[test]
    fn prompt_is_input_when_reset_on_prompt_is_off() {
        let mut p = provider("0CC");
        p.set_reset_on_prompt(false);

        // It is tested against the mask...
        assert_eq!(p.replace_char_with_hint('_', 0), (false, 0, H::DigitExpected));
        // ...and assigned where it fits.
        assert_eq!(p.replace_char_with_hint('_', 1), (true, 1, H::Success));
        assert_eq!(p.to_display_string(), "___");
        assert_eq!(p.assigned_edit_position_count(), 1);
        assert!(!p.is_available_position(1));
    }

    #[test]
    fn prompt_is_rejected_when_not_allowed_as_input() {
        let mut p = MaskedTextProvider::new_full("CC", None, false, '_', '\0', false).unwrap();

        // Escaping precedes the check.
        assert_eq!(p.replace_char_with_hint('_', 0), (true, 0, H::CharacterEscaped));
        p.set_reset_on_prompt(false);
        assert_eq!(p.replace_char_with_hint('_', 0), (false, 0, H::PromptCharNotAllowed));
        assert_eq!(p.add_with_hint("a_"), (false, 1, H::PromptCharNotAllowed));
        assert_eq!(p.assigned_edit_position_count(), 0);
    }

    #[test]
    fn space_resets_the_position() {
        let mut p = filled("000", "123");

        assert_eq!(p.replace_char_with_hint(' ', 0), (true, 0, H::SideEffect));
        assert_eq!(p.to_display_string(), "_23");
        assert_eq!(p.replace_char_with_hint(' ', 0), (true, 0, H::CharacterEscaped));

        // A space in a string skips a position.
        let mut p = provider("AA#00");
        assert_eq!(p.add_with_hint("S2 33"), (true, 4, H::Success));
        assert_eq!(p.to_display_string(), "S2_33");
        assert_eq!(p.assigned_edit_position_count(), 4);
    }

    #[test]
    fn space_is_input_when_reset_on_space_is_off() {
        let mut p = provider("909");
        p.set_reset_on_space(false);

        assert_eq!(p.replace_char_with_hint(' ', 1), (false, 1, H::DigitExpected));
        assert_eq!(p.replace_char_with_hint(' ', 0), (true, 0, H::Success));
        assert_eq!(p.to_display_string(), " __");
        assert_eq!(p.assigned_edit_position_count(), 1);
        assert_eq!(p.last_assigned_position(), 0);
    }

    // ---- Find ----

    fn find_fixture() -> MaskedTextProvider {
        // "12-_4": 0, 1 and 4 are assigned.
        let mut p = filled("00-00", "12");
        assert!(p.replace_char('4', 4));
        assert_eq!(p.to_display_string(), "12-_4");
        p
    }

    #[test]
    fn find_assigned_edit_position() {
        let p = find_fixture();

        assert_eq!(p.find_assigned_edit_position_from(0, true), 0);
        assert_eq!(p.find_assigned_edit_position_from(2, true), 4);
        assert_eq!(p.find_assigned_edit_position_from(3, false), 1);
        assert_eq!(p.find_assigned_edit_position_from(5, true), -1);
        assert_eq!(p.find_assigned_edit_position_in_range(2, 3, true), -1);
        assert_eq!(p.find_assigned_edit_position_in_range(0, 4, false), 4);
        assert_eq!(p.find_assigned_edit_position_in_range(3, 1, true), -1);
        assert_eq!(p.last_assigned_position(), 4);

        let empty = provider("00-00");
        assert_eq!(empty.find_assigned_edit_position_from(0, true), -1);
        assert_eq!(empty.find_assigned_edit_position_in_range(0, 4, false), -1);
    }

    #[test]
    fn find_edit_position() {
        let p = find_fixture();

        assert_eq!(p.find_edit_position_from(0, true), 0);
        assert_eq!(p.find_edit_position_from(2, true), 3);
        assert_eq!(p.find_edit_position_from(2, false), 1);
        assert_eq!(p.find_edit_position_from(5, true), -1);
        // Out of range bounds are clamped.
        assert_eq!(p.find_edit_position_from(-3, true), 0);
        assert_eq!(p.find_edit_position_from(10, false), 4);
        assert_eq!(p.find_edit_position_from(-1, false), -1);
        assert_eq!(p.find_edit_position_in_range(2, 2, true), -1);
        assert_eq!(p.find_edit_position_in_range(1, 3, false), 3);
        assert_eq!(p.find_edit_position_in_range(-5, 50, true), 0);
    }

    #[test]
    fn find_non_edit_position() {
        let p = find_fixture();

        assert_eq!(p.find_non_edit_position_from(0, true), 2);
        assert_eq!(p.find_non_edit_position_from(4, false), 2);
        assert_eq!(p.find_non_edit_position_from(3, true), -1);
        assert_eq!(p.find_non_edit_position_from(1, false), -1);
        assert_eq!(p.find_non_edit_position_in_range(0, 1, true), -1);
        assert_eq!(p.find_non_edit_position_in_range(2, 2, false), 2);
    }

    #[test]
    fn find_unassigned_edit_position() {
        let p = find_fixture();

        assert_eq!(p.find_unassigned_edit_position_from(0, true), 3);
        assert_eq!(p.find_unassigned_edit_position_from(4, false), 3);
        assert_eq!(p.find_unassigned_edit_position_from(4, true), -1);
        assert_eq!(p.find_unassigned_edit_position_in_range(0, 1, true), -1);
        assert_eq!(p.find_unassigned_edit_position_in_range(0, 4, false), 3);

        let empty = provider("00-00");
        assert_eq!(empty.find_unassigned_edit_position_from(2, true), 3);
        assert_eq!(empty.find_unassigned_edit_position_from(2, false), 1);
    }

    #[test]
    fn edit_position_predicates() {
        let p = find_fixture();

        assert_eq!(p.edit_positions().collect::<Vec<_>>(), vec![0, 1, 3, 4]);
        assert!(p.is_edit_position(0));
        assert!(!p.is_edit_position(2));
        assert!(p.is_edit_position(3));
        assert!(!p.is_edit_position(-1));
        assert!(!p.is_edit_position(5));
        assert!(p.is_available_position(3));
        assert!(!p.is_available_position(0));
        assert!(!p.is_available_position(2));
        assert!(!p.is_available_position(-1));
        assert!(!p.is_available_position(5));
    }

    // ---- Formatting ----

    #[test]
    fn to_string_overloads() {
        let mut p = MaskedTextProvider::new_full("00/00", None, true, '_', '*', false).unwrap();
        assert!(p.add("12"));

        assert_eq!(p.to_display_string(), "**/__");
        // Display: no password char, no prompt, literals.
        assert_eq!(p.to_string(), "12/");
        assert_eq!(p.to_string_ignore_password(true), "12/");
        assert_eq!(p.to_string_ignore_password(false), "**/");
        assert_eq!(p.to_string_include(true, true), "12/__");
        assert_eq!(p.to_string_include(false, true), "12/");
        assert_eq!(p.to_string_include(true, false), "12__");
        assert_eq!(p.to_string_include(false, false), "12");
        assert_eq!(p.to_string_range(1, 3), "2/");
        assert_eq!(p.to_string_ignore_password_range(false, 1, 3), "*/");
        assert_eq!(p.to_string_include_range(true, true, 1, 3), "2/_");
        assert_eq!(p.to_string_with(false, true, true, 0, 5), "**/__");
        assert_eq!(p.to_string_with(false, true, false, 0, 5), "**__");
        assert_eq!(p.to_string_with(false, false, false, 0, 5), "**");

        p.set_include_prompt(true);
        assert_eq!(p.to_string(), "12/__");
        p.set_include_literals(false);
        assert_eq!(p.to_string(), "12__");
        assert_eq!(format!("{p}"), "12__");
    }

    #[test]
    fn to_string_range_is_clamped() {
        let p = filled("00/00", "1234");

        assert_eq!(p.to_string_include_range(true, true, 0, 0), "");
        assert_eq!(p.to_string_include_range(true, true, 0, -1), "");
        assert_eq!(p.to_string_include_range(true, true, 5, 1), "");
        assert_eq!(p.to_string_include_range(true, true, -2, 2), "12");
        assert_eq!(p.to_string_include_range(true, true, 3, 100), "34");
        assert_eq!(p.to_string_include_range(false, false, 1, 3), "23");
    }

    #[test]
    fn to_string_without_prompt_replaces_gaps_with_spaces() {
        let mut p = provider("000-000");
        assert!(p.replace_char('1', 1));
        assert!(p.replace_char('2', 5));

        assert_eq!(p.to_display_string(), "_1_-_2_");
        // The string ends at the last assigned position.
        assert_eq!(p.to_string_include(false, true), " 1 - 2");
        assert_eq!(p.to_string_include(false, false), " 1  2");
        assert_eq!(p.to_string_include(true, false), "_1__2_");
    }

    #[test]
    fn to_string_without_prompt_ends_at_the_last_literal_when_nothing_follows() {
        // What a masked text box shows when it hides the prompt on leave.
        let p = filled("00/00/0000", "12");
        assert_eq!(p.to_string_include(false, true), "12/  /");

        let empty = provider("00/00/0000");
        assert_eq!(empty.to_string_include(false, true), "  /  /");
        assert_eq!(empty.to_string_include(false, false), "");
        assert_eq!(provider("000").to_string_include(false, true), "");
        assert_eq!(filled("000", "1").to_string_include(false, true), "1");
    }

    #[test]
    fn to_display_string_with_password_char() {
        let mut p = MaskedTextProvider::new_full("00/00/0000", None, true, '_', '*', false).unwrap();
        assert_eq!(p.to_display_string(), "__/__/____");
        assert!(p.insert_at("12102000", 0));
        assert_eq!(p.to_display_string(), "**/**/****");
        assert_eq!(p.to_string(), "12/10/2000");

        let mut p = MaskedTextProvider::new_full("AA#00", None, true, '_', '*', false).unwrap();
        assert!(p.insert_at("S2 33", 0));
        assert_eq!(p.to_display_string(), "**_**");
    }

    // ---- VerifyString ----

    #[test]
    fn verify_string_does_not_change_anything() {
        let p = provider("00-00");

        assert_eq!(p.verify_string_with_hint("12-34"), (true, 4, H::Success));
        assert_eq!(p.verify_string_with_hint("1234"), (true, 4, H::Success));
        assert_eq!(p.verify_string_with_hint(""), (true, 0, H::NoEffect));
        assert_eq!(p.verify_string_with_hint("1a"), (false, 1, H::DigitExpected));
        assert_eq!(p.verify_string_with_hint("123456"), (false, 5, H::UnavailableEditPosition));
        assert_eq!(p.verify_string_with_hint(" "), (true, 0, H::CharacterEscaped));
        assert_eq!(p.verify_string_with_hint("1\n"), (false, 1, H::InvalidInput));
        assert!(p.verify_string("12"));
        assert!(!p.verify_string("x"));
        assert_eq!(p.assigned_edit_position_count(), 0);

        let p = filled("00-00", "12");
        assert_eq!(p.verify_string_with_hint("12"), (true, 1, H::NoEffect));
        assert_eq!(p.verify_string_with_hint("1 "), (true, 1, H::SideEffect));
        assert_eq!(p.to_display_string(), "12-__");
    }

    // ---- The scenarios of a masked text box ----

    /// Text input: the whole text is inserted at the caret.
    fn type_text(p: &mut MaskedTextProvider, text: &str) -> String {
        let caret = p.find_edit_position_from(0, true);
        p.insert_at(text, caret);
        p.to_display_string()
    }

    #[test]
    fn typed_text_follows_the_mask() {
        assert_eq!(type_text(&mut provider("00/00/0000"), "12102000"), "12/10/2000");
        assert_eq!(type_text(&mut provider("LLLL"), "\u{434}\u{431}s"), "\u{434}\u{431}s_");
        assert_eq!(type_text(&mut provider("AA#00"), "S2 33"), "S2_33");
    }

    #[test]
    fn typed_text_is_hidden_by_the_password_char() {
        let password = |mask: &str| MaskedTextProvider::new_full(mask, None, true, '_', '*', false).unwrap();

        assert_eq!(type_text(&mut password("00/00/0000"), "12102000"), "**/**/****");
        assert_eq!(type_text(&mut password("LLLL"), "\u{434}\u{431}s"), "***_");
        assert_eq!(type_text(&mut password("AA#00"), "S2 33"), "**_**");
    }

    #[test]
    fn typed_non_ascii_text_is_rejected_when_ascii_only() {
        let ascii = |mask: &str| MaskedTextProvider::new_full(mask, None, true, '_', '\0', true).unwrap();

        assert_eq!(type_text(&mut ascii("00/00/0000"), "12102000"), "12/10/2000");
        assert_eq!(type_text(&mut ascii("LLLL"), "\u{434}\u{431}s"), "____");
        assert_eq!(type_text(&mut ascii("AA"), "\u{DC}1"), "__");
    }

    #[test]
    fn pasted_text_is_inserted_character_by_character() {
        let mut p = provider("00/00/0000");
        let mut caret = 0;

        for item in "12x10-2000".chars() {
            let index = p.find_edit_position_from(caret, true);
            if p.insert_at_char(item, index) {
                caret = index + 1;
            }
        }

        assert_eq!(p.to_display_string(), "12/10/2000");
        // There is no edit position after the end.
        assert_eq!(p.find_edit_position_from(caret, true), -1);
        assert!(!p.insert_at_char('1', -1));
    }

    #[test]
    fn selection_is_removed_on_reset_input() {
        // A space or prompt typed over a selection removes the selected range.
        let mut p = filled("00/00/0000", "12102000");
        assert!(p.remove_at_range(3, 6));
        assert_eq!(p.to_display_string(), "12/00/0___");
    }

    #[test]
    fn upper_case_of_greek_letters_with_iota_subscript() {
        let mut p = provider(">LL");
        assert!(p.add("\u{1F80}\u{1FF3}"));
        assert_eq!(p.to_display_string(), "\u{1F88}\u{1FFC}");
    }
}
