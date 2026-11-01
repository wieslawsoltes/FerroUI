//! Clean-room QML frontend for FerroUI. Source offsets are UTF-8 byte offsets.
mod ast;
mod link;
mod scope;
pub use link::{link, ComponentResolver, ComponentSource, LinkError};
#[cfg(feature = "javascript")]
pub mod editor;
mod lexer;
mod parser;

pub use ast::*;
pub use parser::parse;
#[cfg(feature = "javascript")]
pub use parser::parse_external;
#[cfg(feature = "javascript")]
pub mod async_functions;
#[cfg(feature = "javascript")]
pub mod javascript;
#[cfg(feature = "javascript")]
pub mod script_resource;
#[cfg(feature = "javascript")]
mod system_module;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    pub span: SourceSpan,
    pub message: String,
    pub line: usize,
    pub column: usize,
}

impl Diagnostic {
    pub(crate) fn new(source: &str, span: SourceSpan, message: impl Into<String>) -> Self {
        let mut line = 1;
        let mut column = 1;
        let mut was_cr = false;
        // Public validators can receive an externally constructed IR.
        let mut start = span.start.min(source.len());
        while !source.is_char_boundary(start) {
            start -= 1;
        }
        for ch in source[..start].chars() {
            match ch {
                '\r' => {
                    line += 1;
                    column = 1;
                }
                '\n' if was_cr => (),
                '\n' | '\u{2028}' | '\u{2029}' => {
                    line += 1;
                    column = 1;
                }
                _ => column += 1,
            }
            was_cr = ch == '\r';
        }
        Self {
            span,
            message: message.into(),
            line,
            column,
        }
    }
}

impl std::fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}: {}", self.line, self.column, self.message)
    }
}

impl std::error::Error for Diagnostic {}

mod validate;
pub use validate::validate;

#[cfg(test)]
#[path = "tests/language.rs"]
mod tests;

pub mod qmldir;
