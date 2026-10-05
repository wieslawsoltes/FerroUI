//! Port of `Parsers/SystemXamlMarkupExtensionParser/MeScanner.cs`.
// Licensed to the .NET Foundation under one or more agreements.
// The .NET Foundation licenses this file to you under the MIT license.

use std::fmt;
use std::rc::Rc;

use crate::ast::XamlAstNamePropertyReference;
use crate::exceptions::XamlError;

use super::{
    MeScannerContext, MeScannerError, MeScannerKnownStrings, MeScannerParseException,
    MeScannerSRID, MeScannerSpecialBracketCharacters, MeScannerSr, MeScannerTypeName,
};

/// Markup Extension Tokenizer AKA Scanner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeTokenType {
    None,
    Open,
    Close,
    EqualSign,
    Comma,
    /// String - Follows a '{' space delimited
    TypeName,
    /// String - Preceeds a '='.  {},= delimited, can (but shouldn't) contain spaces.
    PropertyName,
    /// String - all other strings, {},= delimited can contain spaces.
    String,
    /// String - must be recursivly parsed as a MarkupExtension.
    QuotedMarkupExtension,
}

impl fmt::Display for MeTokenType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

// 1) Value and (propertynames for compatibility with WPF 3.0) can also have
// escaped character with '\' to include '{' '}' ',' '=', and '\'.
// 2) Value strings can also be quoted (w/ ' or ") in their entirity to escape all
// uses of the above characters.
// 3) All strings are trimmed of whitespace front and back unless they were quoted.
// 4) Quote characters can only appear at the start and end of strings.
// 5) TypeNames cannot be quoted.

#[derive(Clone, Copy, PartialEq, Eq)]
enum StringState {
    Value,
    Type,
    Property,
}

pub struct MeScanner<'c, 'a> {
    context: &'c mut MeScannerContext<'a>,
    input_text: Vec<char>,
    idx: isize,
    token: MeTokenType,
    token_xaml_type: Option<MeScannerTypeName>,
    token_property: Option<Rc<XamlAstNamePropertyReference>>,
    token_namespace: Option<String>,
    token_text: String,
    state: StringState,
    has_trailing_whitespace: bool,
    line_number: i32,
    start_position: i32,
    current_parameter_name: Option<String>,
    current_special_bracket_characters: Option<MeScannerSpecialBracketCharacters>,
}

impl<'c, 'a> MeScanner<'c, 'a> {
    pub const SPACE: char = ' ';
    pub const OPEN_CURLIE: char = '{';
    pub const CLOSE_CURLIE: char = '}';
    pub const COMMA: char = ',';
    pub const EQUAL_SIGN: char = '=';
    pub const QUOTE1: char = '\'';
    pub const QUOTE2: char = '"';
    pub const BACKSLASH: char = '\\';
    pub const NULL_CHAR: char = '\0';

    pub fn new(
        context: &'c mut MeScannerContext<'a>,
        text: &str,
        line_number: i32,
        line_position: i32,
    ) -> Self {
        Self {
            context,
            input_text: text.chars().collect(),
            idx: -1,
            token: MeTokenType::None,
            token_xaml_type: None,
            token_property: None,
            token_namespace: None,
            token_text: String::new(),
            state: StringState::Value,
            has_trailing_whitespace: false,
            line_number,
            start_position: line_position,
            current_parameter_name: None,
            current_special_bracket_characters: None,
        }
    }

    pub fn context(&mut self) -> &mut MeScannerContext<'a> {
        self.context
    }

    pub fn line_number(&self) -> i32 {
        self.line_number
    }

    pub fn line_position(&self) -> i32 {
        let offset = if self.idx < 0 { 0 } else { self.idx };
        self.start_position + offset as i32
    }

    pub fn namespace(&self) -> Option<&str> {
        self.token_namespace.as_deref()
    }

    pub fn token(&self) -> MeTokenType {
        self.token
    }

    pub fn token_type(&self) -> Option<&MeScannerTypeName> {
        self.token_xaml_type.as_ref()
    }

    pub fn token_property(&self) -> Option<&Rc<XamlAstNamePropertyReference>> {
        self.token_property.as_ref()
    }

    pub fn token_text(&self) -> &str {
        &self.token_text
    }

    pub fn is_at_end_of_input(&self) -> bool {
        self.idx >= self.input_text.len() as isize
    }

    pub fn has_trailing_whitespace(&self) -> bool {
        self.has_trailing_whitespace
    }

    pub fn read(&mut self) -> Result<(), MeScannerError> {
        let mut is_quoted_markup_extension = false;
        let mut read_string = false;

        self.token_text = String::new();
        self.token_xaml_type = None;
        self.token_property = None;
        self.token_namespace = None;

        self.advance();
        self.advance_over_whitespace();

        if self.is_at_end_of_input() {
            self.token = MeTokenType::None;
            return Ok(());
        }

        match self.current_char() {
            Self::OPEN_CURLIE => {
                if self.next_char() == Self::CLOSE_CURLIE {
                    // the {} escapes the ME.  return the string.
                    self.token = MeTokenType::String;
                    self.state = StringState::Value;
                    read_string = true; // ReadString() will strip the leading {}
                } else {
                    self.token = MeTokenType::Open;
                    self.state = StringState::Type; // types follow '{'
                }
            }

            Self::QUOTE1 | Self::QUOTE2 => {
                if self.next_char() == Self::OPEN_CURLIE {
                    self.advance(); // read ahead one character
                    if self.next_char() != Self::CLOSE_CURLIE {
                        // check for the '}' of a {}
                        is_quoted_markup_extension = true;
                    }
                    self.push_back(); // put back the read-ahead.
                }
                read_string = true; // read substring"
            }

            Self::CLOSE_CURLIE => {
                self.token = MeTokenType::Close;
                self.state = StringState::Value;
            }

            Self::EQUAL_SIGN => {
                self.token = MeTokenType::EqualSign;
                self.state = StringState::Value;
                self.context
                    .current_bracket_mode_parse_parameters
                    .is_constructor_parsing_mode = false;
            }

            Self::COMMA => {
                self.token = MeTokenType::Comma;
                self.state = StringState::Value;
                let parameters = &mut self.context.current_bracket_mode_parse_parameters;
                if parameters.is_constructor_parsing_mode {
                    parameters.current_constructor_param =
                        parameters.current_constructor_param.saturating_add(1);
                    parameters.is_constructor_parsing_mode =
                        parameters.current_constructor_param < parameters.max_constructor_params;
                }
            }

            _ => {
                read_string = true;
            }
        }

        if read_string {
            if self.context.current_type.is_markup_extension()
                && self
                    .context
                    .current_bracket_mode_parse_parameters
                    .is_constructor_parsing_mode
            {
                self.current_special_bracket_characters = Self::get_bracket_character_for_property(
                    self.current_parameter_name.as_deref(),
                );
            }

            let str = self.read_string()?;
            self.token = if is_quoted_markup_extension {
                MeTokenType::QuotedMarkupExtension
            } else {
                MeTokenType::String
            };

            match self.state {
                StringState::Value => {}

                StringState::Type => {
                    self.token = MeTokenType::TypeName;
                    self.resolve_type_name(&str)?;
                }

                StringState::Property => {
                    self.token = MeTokenType::PropertyName;
                    self.resolve_property_name(&str)?;
                }
            }
            self.state = StringState::Value;
            self.token_text = Self::remove_escapes(&str);
        }

        Ok(())
    }

    fn remove_escapes(value: &str) -> String {
        let value = value.strip_prefix("{}").unwrap_or(value);

        if !value.contains(Self::BACKSLASH) {
            return value.to_string();
        }

        let mut builder = String::with_capacity(value.len());
        let mut chars = value.chars();
        while let Some(ch) = chars.next() {
            if ch == Self::BACKSLASH {
                // Add the character after the backslash
                if let Some(escaped) = chars.next() {
                    builder.push(escaped);
                }
            } else {
                // Copy Clear Text
                builder.push(ch);
            }
        }

        builder
    }

    fn resolve_type_name(&mut self, long_name: &str) -> Result<(), MeScannerError> {
        let type_name = MeScannerTypeName::parse_internal(long_name, self.context)?;

        // Original System.Xaml resolves types in the tokenizer, we don't

        self.token_namespace = type_name.namespace();
        self.token_xaml_type = Some(type_name);
        Ok(())
    }

    fn resolve_property_name(&mut self, long_name: &str) -> Result<(), MeScannerError> {
        self.token_property = Some(self.context.resolve_property_name(long_name)?);
        Ok(())
    }

    fn read_string(&mut self) -> Result<String, MeScannerError> {
        let mut escaped = false;
        let mut quote_char = Self::NULL_CHAR;
        let mut at_start = true;
        let mut was_quoted = false;
        let mut brace_count: u32 = 0; // To be compat with v3 which allowed balanced {} inside of strings

        let mut sb = String::new();

        while !self.is_at_end_of_input() {
            let ch = self.current_char();

            // handle escaping and quoting first.
            if escaped {
                sb.push('\\');
                sb.push(ch);
                escaped = false;
            } else if quote_char != Self::NULL_CHAR {
                if ch == Self::BACKSLASH {
                    escaped = true;
                } else if ch != quote_char {
                    sb.push(ch);
                } else {
                    quote_char = Self::NULL_CHAR;
                    break; // we are done.
                }
            }
            // If we are inside of MarkupExtensionBracketCharacters for a particular property or position parameter,
            // scoop up everything inside one by one, and keep track of nested Bracket Characters in the stack.
            else if self
                .context
                .current_bracket_mode_parse_parameters
                .is_bracket_escape_mode
            {
                let special = self
                    .current_special_bracket_characters
                    .as_ref()
                    .ok_or_else(|| {
                        XamlError::internal(
                            "NullReferenceException",
                            "No special bracket characters are active",
                        )
                    })?;
                if special.starts_escape_sequence(ch)? {
                    self.context
                        .current_bracket_mode_parse_parameters
                        .bracket_character_stack
                        .push(ch);
                } else if special.ends_escape_sequence(ch)? {
                    let stack = &mut self
                        .context
                        .current_bracket_mode_parse_parameters
                        .bracket_character_stack;
                    let top = stack.last().copied().unwrap_or(Self::NULL_CHAR);
                    if special.r#match(top, ch)? {
                        stack.pop();
                    } else {
                        return Err(MeScannerParseException::new(MeScannerSr::get_with_arg(
                            MeScannerSRID::INVALID_CLOSING_BRACKET_CHARACERS,
                            &ch.to_string(),
                        ))
                        .into());
                    }
                } else if ch == Self::BACKSLASH {
                    escaped = true;
                }

                if self
                    .context
                    .current_bracket_mode_parse_parameters
                    .bracket_character_stack
                    .is_empty()
                {
                    self.context
                        .current_bracket_mode_parse_parameters
                        .is_bracket_escape_mode = false;
                }

                if !escaped {
                    sb.push(ch);
                }
            } else {
                let mut done = false;
                match ch {
                    Self::SPACE => {
                        if self.state == StringState::Type {
                            done = true; // we are done.
                        } else {
                            sb.push(ch);
                        }
                    }

                    Self::OPEN_CURLIE => {
                        brace_count += 1;
                        sb.push(ch);
                    }
                    Self::CLOSE_CURLIE => {
                        if brace_count == 0 {
                            done = true;
                        } else {
                            brace_count -= 1;
                            sb.push(ch);
                        }
                    }

                    Self::COMMA => {
                        done = true; // we are done.
                    }

                    Self::EQUAL_SIGN => {
                        self.state = StringState::Property;
                        done = true; // we are done.
                    }

                    Self::BACKSLASH => {
                        escaped = true;
                    }

                    Self::QUOTE1 | Self::QUOTE2 => {
                        if !at_start {
                            return Err(MeScannerParseException::new(MeScannerSr::get(
                                MeScannerSRID::QUOTE_CHARACTERS_OUT_OF_PLACE,
                            ))
                            .into());
                        }
                        quote_char = ch;
                        was_quoted = true;
                    }

                    _ => {
                        // All other character (including whitespace)
                        if let Some(special) = &self.current_special_bracket_characters {
                            if special.starts_escape_sequence(ch)? {
                                let parameters =
                                    &mut self.context.current_bracket_mode_parse_parameters;
                                parameters.bracket_character_stack.clear();
                                parameters.bracket_character_stack.push(ch);
                                parameters.is_bracket_escape_mode = true;
                            }
                        }

                        sb.push(ch);
                    }
                }

                if done {
                    if brace_count > 0 {
                        return Err(MeScannerParseException::new(MeScannerSr::get(
                            MeScannerSRID::UNEXPECTED_TOKEN_AFTER_ME,
                        ))
                        .into());
                    } else if !self
                        .context
                        .current_bracket_mode_parse_parameters
                        .bracket_character_stack
                        .is_empty()
                    {
                        return Err(MeScannerParseException::new(MeScannerSr::get_with_arg(
                            MeScannerSRID::MALFORMED_BRACKET_CHARACTERS,
                            &ch.to_string(),
                        ))
                        .into());
                    }

                    self.push_back();
                    break; // we are done.
                }
            }
            at_start = false;
            self.advance();
        }

        if quote_char != Self::NULL_CHAR {
            return Err(MeScannerParseException::new(MeScannerSr::get(
                MeScannerSRID::UNCLOSED_QUOTE,
            ))
            .into());
        }

        let mut result = sb;
        if !was_quoted {
            result = result
                .trim_matches(|c| MeScannerKnownStrings::WHITESPACE_CHARS.contains(&c))
                .to_string();
        }

        if self.state == StringState::Property {
            self.current_special_bracket_characters =
                Self::get_bracket_character_for_property(Some(&result));
            self.current_parameter_name = Some(result.clone());
        }

        Ok(result)
    }

    fn current_char(&self) -> char {
        if self.idx < 0 {
            return Self::NULL_CHAR;
        }
        self.input_text
            .get(self.idx as usize)
            .copied()
            .unwrap_or(Self::NULL_CHAR)
    }

    fn next_char(&self) -> char {
        self.input_text
            .get((self.idx + 1) as usize)
            .copied()
            .unwrap_or(Self::NULL_CHAR)
    }

    fn advance(&mut self) -> bool {
        self.idx += 1;
        if self.is_at_end_of_input() {
            self.idx = self.input_text.len() as isize;
            return false;
        }
        true
    }

    fn is_whitespace_char(ch: char) -> bool {
        MeScannerKnownStrings::WHITESPACE_CHARS.contains(&ch)
    }

    fn advance_over_whitespace(&mut self) {
        let mut saw_whitespace = false;

        while !self.is_at_end_of_input() && Self::is_whitespace_char(self.current_char()) {
            saw_whitespace = true;
            self.advance();
        }

        // WFP 3.0 errors on trailing whitespace.
        // [note: very first compat workaround in the new XAML parser]
        // Noticing trailing whitespace is not very natural in this parser.
        // so this extra code is here to implement this error.
        if self.is_at_end_of_input() && saw_whitespace {
            self.has_trailing_whitespace = true;
        }
    }

    fn push_back(&mut self) {
        self.idx -= 1;
    }

    fn get_bracket_character_for_property(
        _property_name: Option<&str>,
    ) -> Option<MeScannerSpecialBracketCharacters> {
        // Xaml resolves markup extension types after parsing the initial AST,
        // so custom brackets aren't supported
        None
    }
}
