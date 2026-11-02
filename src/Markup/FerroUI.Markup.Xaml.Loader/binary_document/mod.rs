//! FXD v1 stores XamlX's parsed document graph. Shared type references, lexical
//! namespaces, source positions and markup-extension syntax survive replay. Type
//! resolution, transformations and execution remain the responsibility of XamlX.
use xamlx::ast::XamlDocument;
use xamlx::exceptions::{XamlError, XamlResult};

mod decode;
mod encode;

pub use decode::decode;
pub use encode::{encode, encode_stored};

const MAGIC: &[u8; 4] = b"FXD\0";
const VERSION: u8 = 1;
/// Maximum stored bytes accepted for a single portable document.
pub const MAX_DOCUMENT_BYTES: usize = 32 << 20;
const MAX_NODES: usize = 1 << 19;
// A property element can represent an additional AST level between two XML elements.
const MAX_DEPTH: usize = 768;
const MAX_VISITS: usize = 4 << 20;
const MAX_EXPANDED_TEXT: usize = 64 << 20;

/// Cost of traversing a node through all its references, including shared types.
#[derive(Clone, Copy)]
struct Expansion {
    height: usize,
    visits: usize,
}

impl Expansion {
    const LEAF: Self = Self {
        height: 1,
        visits: 1,
    };

    fn include(&mut self, child: Self) -> XamlResult<()> {
        self.height = self.height.max(child.height + 1);
        self.visits += child.visits;
        if self.height > MAX_DEPTH || self.visits > MAX_VISITS {
            return Err(invalid("expanded graph exceeds limit"));
        }
        Ok(())
    }
}

const TYPE: u8 = 0;
const PROPERTY: u8 = 1;
const TEXT: u8 = 2;
const OBJECT: u8 = 3;
const ASSIGN: u8 = 4;
const DIRECTIVE: u8 = 5;

fn invalid(detail: &str) -> XamlError {
    XamlError::invalid_operation(format!("Invalid FXD document: {detail}"))
}

/// Identify a document container independently of its version so future versions
/// receive a binary-format diagnostic instead of being interpreted as XML.
pub fn is_binary(bytes: &[u8]) -> bool {
    bytes.starts_with(MAGIC)
}

/// Compile markup to portable syntax without resolving a framework or application type.
pub fn compile(xaml: &str) -> XamlResult<Vec<u8>> {
    let parsed: XamlDocument =
        crate::compiler_extensions::FerroXamlIlCompiler::parse_source_syntax(xaml)?;
    encode(&parsed)
}

#[cfg(test)]
mod tests;
