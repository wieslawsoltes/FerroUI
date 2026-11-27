//! The part of the tokenizer of CSS Syntax Level 3 that font descriptors need.
//!
//! It works on byte positions of UTF-8 text, is linear in its input, does not recurse, and
//! borrows from the input unless an escape has to be resolved.

use std::borrow::Cow;

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Token<'a> {
    Ident(Cow<'a, str>),
    Function(Cow<'a, str>),
    AtKeyword(Cow<'a, str>),
    String(Cow<'a, str>),
    BadString,
    Url(Cow<'a, str>),
    BadUrl,
    Number { value: f64, is_integer: bool },
    Percentage(f64),
    Dimension { value: f64, unit: Cow<'a, str> },
    Whitespace,
    Colon,
    Semicolon,
    Comma,
    LeftBrace,
    RightBrace,
    LeftParen,
    RightParen,
    LeftBracket,
    RightBracket,
    Delim(char),
    Eof,
}

pub(crate) struct Tokenizer<'a> {
    input: &'a str,
    position: usize,
}

fn is_name_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_' || !c.is_ascii()
}

fn is_name(c: char) -> bool {
    is_name_start(c) || c.is_ascii_digit() || c == '-'
}

fn is_whitespace(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\n' | '\r' | '\u{c}')
}

impl<'a> Tokenizer<'a> {
    pub(crate) fn new(input: &'a str) -> Self {
        Self { input, position: 0 }
    }

    /// The byte position of the next token.
    pub(crate) fn position(&self) -> usize {
        self.position
    }

    fn rest(&self) -> &'a str {
        &self.input[self.position..]
    }

    fn peek(&self) -> Option<char> {
        self.rest().chars().next()
    }

    fn peek_at(&self, count: usize) -> Option<char> {
        self.rest().chars().nth(count)
    }

    fn bump(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.position += c.len_utf8();
        Some(c)
    }

    fn skip_whitespace(&mut self) -> bool {
        let start = self.position;
        while self.peek().is_some_and(is_whitespace) {
            self.bump();
        }
        self.position != start
    }

    fn skip_comment(&mut self) -> bool {
        if !self.rest().starts_with("/*") {
            return false;
        }
        match self.rest()[2..].find("*/") {
            Some(end) => self.position += end + 4,
            None => self.position = self.input.len(),
        }
        true
    }

    fn starts_valid_escape(first: Option<char>, second: Option<char>) -> bool {
        first == Some('\\') && second.is_some_and(|c| c != '\n' && c != '\r' && c != '\u{c}')
    }

    fn starts_ident(&self) -> bool {
        let (first, second, third) = (self.peek(), self.peek_at(1), self.peek_at(2));
        match first {
            Some('-') => {
                second.is_some_and(|c| is_name_start(c) || c == '-') || Self::starts_valid_escape(second, third)
            }
            Some('\\') => Self::starts_valid_escape(first, second),
            Some(c) => is_name_start(c),
            None => false,
        }
    }

    fn starts_number(&self) -> bool {
        let digit = |c: Option<char>| c.is_some_and(|c| c.is_ascii_digit());
        match self.peek() {
            Some('+' | '-') => digit(self.peek_at(1)) || (self.peek_at(1) == Some('.') && digit(self.peek_at(2))),
            Some('.') => digit(self.peek_at(1)),
            c => digit(c),
        }
    }

    /// Consumes an escape, the backslash already consumed.
    fn consume_escape(&mut self) -> char {
        let Some(c) = self.bump() else {
            return '\u{fffd}';
        };
        if !c.is_ascii_hexdigit() {
            return c;
        }
        let mut value = c.to_digit(16).unwrap_or(0);
        for _ in 0..5 {
            match self.peek().and_then(|c| c.to_digit(16)) {
                Some(digit) => {
                    value = value * 16 + digit;
                    self.bump();
                }
                None => break,
            }
        }
        if self.peek().is_some_and(is_whitespace) {
            self.bump();
        }
        match char::from_u32(value) {
            Some(c) if c != '\0' => c,
            _ => '\u{fffd}',
        }
    }

    fn consume_name(&mut self) -> Cow<'a, str> {
        let start = self.position;
        while self.peek().is_some_and(is_name) {
            self.bump();
        }
        if self.peek() != Some('\\') || !Self::starts_valid_escape(self.peek(), self.peek_at(1)) {
            return Cow::Borrowed(&self.input[start..self.position]);
        }
        let mut name = self.input[start..self.position].to_owned();
        loop {
            match self.peek() {
                Some(c) if is_name(c) => {
                    name.push(c);
                    self.bump();
                }
                Some('\\') if Self::starts_valid_escape(self.peek(), self.peek_at(1)) => {
                    self.bump();
                    name.push(self.consume_escape());
                }
                _ => return Cow::Owned(name),
            }
        }
    }

    fn consume_string(&mut self, quote: char) -> Token<'a> {
        let start = self.position;
        loop {
            match self.peek() {
                None => return Token::String(Cow::Borrowed(&self.input[start..self.position])),
                Some(c) if c == quote => {
                    let text = &self.input[start..self.position];
                    self.bump();
                    return Token::String(Cow::Borrowed(text));
                }
                Some('\n' | '\r' | '\u{c}') => return Token::BadString,
                Some('\\') => break,
                Some(_) => {
                    self.bump();
                }
            }
        }
        let mut text = self.input[start..self.position].to_owned();
        loop {
            match self.peek() {
                None => return Token::String(Cow::Owned(text)),
                Some(c) if c == quote => {
                    self.bump();
                    return Token::String(Cow::Owned(text));
                }
                Some('\n' | '\r' | '\u{c}') => return Token::BadString,
                Some('\\') => {
                    self.bump();
                    match self.peek() {
                        None => {}
                        Some('\r') => {
                            self.bump();
                            if self.peek() == Some('\n') {
                                self.bump();
                            }
                        }
                        Some('\n' | '\u{c}') => {
                            self.bump();
                        }
                        Some(_) => text.push(self.consume_escape()),
                    }
                }
                Some(c) => {
                    text.push(c);
                    self.bump();
                }
            }
        }
    }

    fn consume_number(&mut self) -> (f64, bool) {
        let start = self.position;
        let mut is_integer = true;
        if matches!(self.peek(), Some('+' | '-')) {
            self.bump();
        }
        while self.peek().is_some_and(|c| c.is_ascii_digit()) {
            self.bump();
        }
        if self.peek() == Some('.') && self.peek_at(1).is_some_and(|c| c.is_ascii_digit()) {
            is_integer = false;
            self.bump();
            while self.peek().is_some_and(|c| c.is_ascii_digit()) {
                self.bump();
            }
        }
        if matches!(self.peek(), Some('e' | 'E')) {
            let sign = matches!(self.peek_at(1), Some('+' | '-')) as usize;
            if self.peek_at(1 + sign).is_some_and(|c| c.is_ascii_digit()) {
                is_integer = false;
                for _ in 0..=sign {
                    self.bump();
                }
                while self.peek().is_some_and(|c| c.is_ascii_digit()) {
                    self.bump();
                }
            }
        }
        (self.input[start..self.position].parse().unwrap_or(0.0), is_integer)
    }

    fn consume_numeric(&mut self) -> Token<'a> {
        let (value, is_integer) = self.consume_number();
        if self.starts_ident() {
            return Token::Dimension { value, unit: self.consume_name() };
        }
        if self.peek() == Some('%') {
            self.bump();
            return Token::Percentage(value);
        }
        Token::Number { value, is_integer }
    }

    /// Consumes what is left of a bad URL, through its closing parenthesis.
    fn consume_bad_url(&mut self) -> Token<'a> {
        loop {
            match self.bump() {
                None | Some(')') => return Token::BadUrl,
                Some('\\') => {
                    if self.peek().is_some() {
                        self.consume_escape();
                    }
                }
                Some(_) => {}
            }
        }
    }

    fn consume_url(&mut self) -> Token<'a> {
        self.skip_whitespace();
        if let Some(quote @ ('"' | '\'')) = self.peek() {
            self.bump();
            let Token::String(text) = self.consume_string(quote) else {
                return self.consume_bad_url();
            };
            self.skip_whitespace();
            return match self.peek() {
                Some(')') => {
                    self.bump();
                    Token::Url(text)
                }
                None => Token::Url(text),
                Some(_) => self.consume_bad_url(),
            };
        }
        let mut text = String::new();
        loop {
            match self.peek() {
                None => return Token::Url(Cow::Owned(text)),
                Some(')') => {
                    self.bump();
                    return Token::Url(Cow::Owned(text));
                }
                Some(c) if is_whitespace(c) => {
                    self.skip_whitespace();
                    return match self.peek() {
                        Some(')') => {
                            self.bump();
                            Token::Url(Cow::Owned(text))
                        }
                        None => Token::Url(Cow::Owned(text)),
                        Some(_) => self.consume_bad_url(),
                    };
                }
                Some('"' | '\'' | '(') => return self.consume_bad_url(),
                Some(c) if c.is_control() => return self.consume_bad_url(),
                Some('\\') => {
                    if !Self::starts_valid_escape(self.peek(), self.peek_at(1)) {
                        return self.consume_bad_url();
                    }
                    self.bump();
                    text.push(self.consume_escape());
                }
                Some(c) => {
                    text.push(c);
                    self.bump();
                }
            }
        }
    }

    fn consume_ident_like(&mut self) -> Token<'a> {
        let name = self.consume_name();
        if self.peek() != Some('(') {
            return Token::Ident(name);
        }
        self.bump();
        if name.eq_ignore_ascii_case("url") {
            return self.consume_url();
        }
        Token::Function(name)
    }

    pub(crate) fn next_token(&mut self) -> Token<'a> {
        while self.skip_comment() {}
        if self.skip_whitespace() {
            while self.skip_comment() {}
            self.skip_whitespace();
            return Token::Whitespace;
        }
        let Some(c) = self.peek() else {
            return Token::Eof;
        };
        match c {
            '"' | '\'' => {
                self.bump();
                self.consume_string(c)
            }
            '(' | ')' | '[' | ']' | '{' | '}' | ',' | ':' | ';' => {
                self.bump();
                match c {
                    '(' => Token::LeftParen,
                    ')' => Token::RightParen,
                    '[' => Token::LeftBracket,
                    ']' => Token::RightBracket,
                    '{' => Token::LeftBrace,
                    '}' => Token::RightBrace,
                    ',' => Token::Comma,
                    ':' => Token::Colon,
                    _ => Token::Semicolon,
                }
            }
            '@' => {
                self.bump();
                if self.starts_ident() {
                    Token::AtKeyword(self.consume_name())
                } else {
                    Token::Delim('@')
                }
            }
            _ if self.starts_number() => self.consume_numeric(),
            _ if self.starts_ident() => self.consume_ident_like(),
            _ => {
                self.bump();
                Token::Delim(c)
            }
        }
    }

    /// The next token that is not white space.
    pub(crate) fn next_significant(&mut self) -> Token<'a> {
        loop {
            match self.next_token() {
                Token::Whitespace => {}
                token => return token,
            }
        }
    }
}
