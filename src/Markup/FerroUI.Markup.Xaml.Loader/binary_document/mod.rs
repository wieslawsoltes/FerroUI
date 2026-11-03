//! FXD v1 stores XamlX's parsed document graph. Shared type references, lexical
//! namespaces, source positions and markup-extension syntax survive replay. Type
//! resolution, transformations and execution remain the responsibility of XamlX.
use xamlx::ast::XamlDocument;
use xamlx::exceptions::{XamlError, XamlResult};

mod decode;
mod encode;
pub mod pack;

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

    fn include(&mut self, child: Self) -> DecodeResult<()> {
        self.height = self.height.max(child.height + 1);
        self.visits += child.visits;
        if self.height > MAX_DEPTH || self.visits > MAX_VISITS {
            return Err(DecodeError::GraphLimit);
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

/// Carry one discriminant through operand and graph validation. Construct the
/// public diagnostic only when a failure leaves the decoder.
#[derive(Clone, Copy, Debug)]
enum DecodeError {
    GraphLimit,
    Signature,
    CompressionHeader,
    ImageLimit,
    CompressionMode,
    TruncatedInteger,
    InvalidInteger,
    IntegerOverflow,
    TruncatedImage,
    Count,
    Flag,
    StringIndex,
    StringLimit,
    NamespaceLimit,
    DuplicatePrefix,
    BackReference,
    NodeRole,
    SharedOccurrence,
    ScopeIndex,
    ObjectChild,
    Opcode,
    LineOverflow,
    ColumnOverflow,
    CompressedPayload,
    PayloadLength,
    Utf8,
    TrailingColumns,
    MissingRoot,
    InvalidRoot,
}

type DecodeResult<T> = Result<T, DecodeError>;

impl From<DecodeError> for XamlError {
    #[cold]
    fn from(error: DecodeError) -> Self {
        let message = match error {
            DecodeError::GraphLimit => "expanded graph exceeds limit",
            DecodeError::Signature => "unexpected signature, version or length",
            DecodeError::CompressionHeader => "truncated compression header",
            DecodeError::ImageLimit => "expanded image exceeds limit",
            DecodeError::CompressionMode => "unknown compression mode",
            DecodeError::TruncatedInteger => "truncated integer",
            DecodeError::InvalidInteger => "invalid unsigned integer",
            DecodeError::IntegerOverflow => "integer overflow",
            DecodeError::TruncatedImage => "truncated image",
            DecodeError::Count => "invalid count",
            DecodeError::Flag => "invalid flag",
            DecodeError::StringIndex => "string index out of range",
            DecodeError::StringLimit => "expanded strings exceed limit",
            DecodeError::NamespaceLimit => "namespace maps exceed limit",
            DecodeError::DuplicatePrefix => "duplicate namespace prefix",
            DecodeError::BackReference => "invalid backwards node reference",
            DecodeError::NodeRole => "node in wrong role",
            DecodeError::SharedOccurrence => "shared mutable occurrence",
            DecodeError::ScopeIndex => "scope index out of range",
            DecodeError::ObjectChild => "invalid object child",
            DecodeError::Opcode => "unknown node opcode",
            DecodeError::LineOverflow => "source line overflow",
            DecodeError::ColumnOverflow => "source column overflow",
            DecodeError::CompressedPayload => "invalid compressed payload",
            DecodeError::PayloadLength => "compressed payload length mismatch",
            DecodeError::Utf8 => "malformed UTF-8",
            DecodeError::TrailingColumns => "trailing bytes in source columns",
            DecodeError::MissingRoot => "missing root",
            DecodeError::InvalidRoot => "invalid root, unreachable nodes or trailing bytes",
        };
        invalid(message)
    }
}

fn invalid(detail: &str) -> XamlError {
    XamlError::invalid_operation(format!("Invalid FXD document: {detail}"))
}

/// Identify a document container independently of its version so future versions
/// receive a binary-format diagnostic instead of being interpreted as XML.
pub fn is_binary(bytes: &[u8]) -> bool {
    bytes.starts_with(MAGIC)
}

/// Inspect just the envelope, before reserving decompression or AST storage.
pub(crate) fn expanded_size(bytes: &[u8]) -> XamlResult<usize> {
    envelope_size(bytes).map_err(Into::into)
}

fn envelope_size(bytes: &[u8]) -> DecodeResult<usize> {
    if bytes.len() > MAX_DOCUMENT_BYTES || !is_binary(bytes) || bytes.get(4) != Some(&VERSION) {
        return Err(DecodeError::Signature);
    }
    match bytes.get(5) {
        Some(0 | 2) => Ok(bytes.len()),
        Some(1 | 3) => {
            let size = bytes
                .get(6..10)
                .ok_or(DecodeError::CompressionHeader)?;
            let size = u32::from_le_bytes(size.try_into().unwrap()) as usize;
            if size > MAX_DOCUMENT_BYTES - 6 {
                return Err(DecodeError::ImageLimit);
            }
            Ok(size + 6)
        }
        _ => Err(DecodeError::CompressionMode),
    }
}

/// Compile markup to portable syntax without resolving a framework or application type.
pub fn compile(xaml: &str) -> XamlResult<Vec<u8>> {
    let parsed: XamlDocument =
        crate::compiler_extensions::FerroXamlIlCompiler::parse_source_syntax(xaml)?;
    encode(&parsed)
}

#[cfg(test)]
mod tests;
