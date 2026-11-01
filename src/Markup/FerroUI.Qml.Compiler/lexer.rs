//! Lexical scanning. Newline trivia is retained for statement boundaries.
use crate::{Diagnostic, SourceSpan};

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Kind<'s> {
    Ident(&'s str),
    Number(&'s str),
    String(String),
    Symbol(&'s str),
    #[cfg(feature = "javascript")]
    Invalid(String),
    End,
}

#[derive(Clone, Debug)]
pub(crate) struct Token<'s> {
    pub kind: Kind<'s>,
    pub span: SourceSpan,
    pub line_break: bool,
}

pub(crate) fn tokenize(source: &str) -> Result<Vec<Token<'_>>, Diagnostic> {
    let mut scanner = Scanner { source, pos: 0 };
    let mut tokens = Vec::new();
    loop {
        let line_break = scanner.trivia()?;
        let start = scanner.pos;
        let kind = scanner.token()?;
        let done = kind == Kind::End;
        tokens.push(Token {
            kind,
            span: SourceSpan {
                start,
                end: scanner.pos,
            },
            line_break,
        });
        if done {
            return Ok(tokens);
        }
    }
}

/// Native parsing borrows its token vector. External parsing keeps only QML
/// tokens and advances past complete ECMAScript spans supplied by its parser.
pub(crate) enum Tokens<'s> {
    Native(Vec<Token<'s>>),
    #[cfg(feature = "javascript")]
    External(std::cell::RefCell<ExternalTokens<'s>>),
}

impl<'s> Tokens<'s> {
    pub fn get(&self, i: usize) -> Option<std::borrow::Cow<'_, Token<'s>>> {
        match self {
            Self::Native(tokens) => tokens.get(i).map(std::borrow::Cow::Borrowed),
            #[cfg(feature = "javascript")]
            Self::External(tokens) => tokens.borrow_mut().get(i).map(std::borrow::Cow::Owned),
        }
    }
    #[cfg(feature = "javascript")]
    pub fn external(source: &'s str) -> Self {
        Self::External(std::cell::RefCell::new(ExternalTokens {
            scanner: Scanner { source, pos: 0 },
            tokens: Vec::new(),
        }))
    }
    #[cfg(feature = "javascript")]
    pub fn skip_script(&mut self, index: usize, span: SourceSpan) {
        let Self::External(state) = self else {
            unreachable!("only the external parser consumes script spans")
        };
        let state = state.get_mut();
        state.tokens.truncate(index);
        state.tokens.push(Token {
            kind: Kind::Symbol("<JavaScript>"),
            span,
            line_break: false,
        });
        state.scanner.pos = span.end;
    }
}

#[cfg(feature = "javascript")]
pub(crate) struct ExternalTokens<'s> {
    scanner: Scanner<'s>,
    tokens: Vec<Token<'s>>,
}

#[cfg(feature = "javascript")]
impl<'s> ExternalTokens<'s> {
    fn get(&mut self, i: usize) -> Option<Token<'s>> {
        while self.tokens.len() <= i {
            if self
                .tokens
                .last()
                .is_some_and(|t| matches!(t.kind, Kind::End | Kind::Invalid(_)))
            {
                return None;
            }
            let s = &mut self.scanner;
            let token = (|| -> Result<Token<'s>, Diagnostic> {
                let line_break = s.trivia()?;
                let start = s.pos;
                let kind = s.token()?;
                Ok(Token {
                    kind,
                    span: SourceSpan { start, end: s.pos },
                    line_break,
                })
            })()
            .unwrap_or_else(|err| Token {
                kind: Kind::Invalid(err.message),
                span: err.span,
                line_break: false,
            });
            self.tokens.push(token);
        }
        self.tokens.get(i).cloned()
    }
}

fn newline(ch: char) -> bool {
    matches!(ch, '\r' | '\n' | '\u{2028}' | '\u{2029}')
}

pub(crate) fn identifier_start(ch: char) -> bool {
    ch == '_' || ch == '$' || ch.is_alphabetic()
}

struct Scanner<'s> {
    source: &'s str,
    pos: usize,
}

impl<'s> Scanner<'s> {
    fn remaining(&self) -> &'s str {
        &self.source[self.pos..]
    }

    fn peek(&self) -> Option<char> {
        self.remaining().chars().next()
    }

    fn next(&mut self) -> Option<char> {
        let ch = self.peek()?;
        self.pos += ch.len_utf8();
        Some(ch)
    }

    fn take_while(&mut self, predicate: impl Fn(char) -> bool) {
        let len = self
            .remaining()
            .char_indices()
            .find(|(_, ch)| !predicate(*ch))
            .map_or(self.remaining().len(), |(index, _)| index);
        self.pos += len;
    }

    fn error(&self, start: usize, message: &str) -> Diagnostic {
        Diagnostic::new(
            self.source,
            SourceSpan {
                start,
                end: self.pos,
            },
            message,
        )
    }

    fn trivia(&mut self) -> Result<bool, Diagnostic> {
        let mut line_break = false;
        loop {
            match self.peek() {
                Some(ch) if ch.is_whitespace() || ch == '\u{feff}' => {
                    line_break |= newline(ch);
                    self.next();
                }
                Some('/') if self.remaining().starts_with("//") => {
                    self.take_while(|ch| !newline(ch));
                }
                Some('/') if self.remaining().starts_with("/*") => {
                    let start = self.pos;
                    self.pos += 2;
                    let Some(len) = self.remaining().find("*/") else {
                        self.pos = self.source.len();
                        return Err(self.error(start, "unterminated block comment"));
                    };
                    line_break |= self.remaining()[..len].chars().any(newline);
                    self.pos += len + 2;
                }
                _ => return Ok(line_break),
            }
        }
    }

    fn token(&mut self) -> Result<Kind<'s>, Diagnostic> {
        let start = self.pos;
        let Some(ch) = self.peek() else {
            return Ok(Kind::End);
        };
        if identifier_start(ch) {
            self.next();
            self.take_while(|c| {
                identifier_start(c) || c.is_numeric() || c == '\u{200c}' || c == '\u{200d}'
            });
            return Ok(Kind::Ident(&self.source[start..self.pos]));
        }
        if ch.is_ascii_digit()
            || (ch == '.'
                && self
                    .remaining()
                    .as_bytes()
                    .get(1)
                    .is_some_and(u8::is_ascii_digit))
        {
            return self.number();
        }
        if ch == '"' || ch == '\'' {
            return self.string(ch).map(Kind::String);
        }
        // Structural punctuation never starts a multi-character operator.
        if matches!(
            ch,
            '{' | '}' | '[' | ']' | '(' | ')' | ';' | ':' | ',' | '.' | '~'
        ) {
            self.pos += 1;
            return Ok(Kind::Symbol(&self.source[start..self.pos]));
        }
        const OPERATORS: &[&str] = &[
            ">>>=", "===", "!==", "**=", "<<=", ">>=", ">>>", "??=", "&&=", "||=", "=>", "==",
            "!=", "<=", ">=", "&&", "||", "??", "++", "--", "+=", "-=", "*=", "/=", "%=", "&=",
            "|=", "^=", "**", "<<", ">>",
        ];
        if let Some(op) = OPERATORS
            .iter()
            .copied()
            .find(|op| self.remaining().starts_with(op))
        {
            self.pos += op.len();
            return Ok(Kind::Symbol(op));
        }
        self.next();
        if "{}[]();:,.?+-*/%<>=!~&|^".contains(ch) {
            return Ok(Kind::Symbol(&self.source[start..self.pos]));
        }
        Err(self.error(start, "unexpected character"))
    }

    fn number(&mut self) -> Result<Kind<'s>, Diagnostic> {
        let start = self.pos;
        let radix = match self.remaining().as_bytes().get(..2) {
            Some(b"0x" | b"0X") => Some(16),
            Some(b"0o" | b"0O") => Some(8),
            Some(b"0b" | b"0B") => Some(2),
            _ => None,
        };
        if let Some(radix) = radix {
            self.pos += 2;
            let digits = self.pos;
            self.take_while(|ch| ch.is_digit(radix));
            if digits == self.pos {
                return Err(self.error(start, "expected digits after radix prefix"));
            }
        } else {
            self.take_while(|ch| ch.is_ascii_digit());
            if self.peek() == Some('.') {
                self.next();
                self.take_while(|ch| ch.is_ascii_digit());
            }
            if matches!(self.peek(), Some('e' | 'E')) {
                self.next();
                if matches!(self.peek(), Some('+' | '-')) {
                    self.next();
                }
                let digits = self.pos;
                self.take_while(|ch| ch.is_ascii_digit());
                if digits == self.pos {
                    return Err(self.error(start, "expected exponent digits"));
                }
            }
        }
        if self
            .peek()
            .is_some_and(|ch| identifier_start(ch) || ch.is_ascii_digit())
        {
            return Err(self.error(start, "invalid numeric literal"));
        }
        Ok(Kind::Number(&self.source[start..self.pos]))
    }

    fn string(&mut self, quote: char) -> Result<String, Diagnostic> {
        let start = self.pos;
        self.next();
        let mut value = String::new();
        while let Some(ch) = self.next() {
            match ch {
                ch if ch == quote => return Ok(value),
                '\\' => {
                    let escape = self.pos - 1;
                    let Some(ch) = self.next() else { break };
                    match ch {
                        'b' => value.push('\x08'),
                        'f' => value.push('\x0c'),
                        'n' => value.push('\n'),
                        'r' => value.push('\r'),
                        't' => value.push('\t'),
                        'v' => value.push('\x0b'),
                        '0' if !self.peek().is_some_and(|c| c.is_ascii_digit()) => value.push('\0'),
                        '0'..='9' => {
                            return Err(self.error(escape, "legacy octal escape is unsupported"))
                        }
                        'x' => value.push(char::from_u32(self.hex(2, escape)?).unwrap()),
                        'u' => value.push(self.unicode_escape(escape)?),
                        '\r' => {
                            if self.peek() == Some('\n') {
                                self.next();
                            }
                        }
                        ch if newline(ch) => (),
                        ch => value.push(ch),
                    }
                }
                ch => value.push(ch),
            }
        }
        Err(self.error(start, "unterminated string literal"))
    }

    fn hex(&mut self, count: usize, start: usize) -> Result<u32, Diagnostic> {
        let mut code = 0;
        for _ in 0..count {
            let digit = self
                .next()
                .and_then(|ch| ch.to_digit(16))
                .ok_or_else(|| self.error(start, "invalid hexadecimal escape"))?;
            code = code * 16 + digit;
        }
        Ok(code)
    }

    fn unicode_escape(&mut self, start: usize) -> Result<char, Diagnostic> {
        let code = if self.peek() == Some('{') {
            self.next();
            let digits = self.pos;
            self.take_while(|ch| ch.is_ascii_hexdigit());
            let value = u32::from_str_radix(&self.source[digits..self.pos], 16)
                .map_err(|_| self.error(start, "invalid Unicode code point"))?;
            if self.next() != Some('}') {
                return Err(self.error(start, "expected '}' in Unicode escape"));
            }
            value
        } else {
            let first = self.hex(4, start)?;
            if (0xd800..=0xdbff).contains(&first) && self.remaining().starts_with("\\u") {
                self.pos += 2;
                let second = self.hex(4, start)?;
                if !(0xdc00..=0xdfff).contains(&second) {
                    return Err(self.error(start, "invalid Unicode surrogate pair"));
                }
                0x10000 + (first - 0xd800) * 1024 + second - 0xdc00
            } else {
                first
            }
        };
        char::from_u32(code).ok_or_else(|| self.error(start, "invalid Unicode scalar value"))
    }
}
