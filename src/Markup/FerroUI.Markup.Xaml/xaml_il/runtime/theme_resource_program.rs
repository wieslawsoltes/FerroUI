//! Compiler-produced resource bytecode. The version and operation identify a homogeneous
//! run. Its call site names the one typed host it needs. Local import tables refer directly
//! to strings and namespace declarations. Bytecode never performs member or type lookup.
//!
//! A block starts with version 1, its one-byte opcode, and a ULEB128 row count. Each row
//! has a key import and two ZigZag deltas from the previous declaration (initially 0,0).
//! Deferred rows also specify a namespace import (0 for the document, otherwise index + 1)
//! and, except for literal brushes, the value's line/column deltas from that declaration.
//! Operands are string-import indices, little-endian ARGB, or one/four doubles. Integers
//! from -32 to 222 with identical IEEE bits occupy one byte; 255 introduces a full f64.
//! All imports and row bounds are validated before registering the first dictionary entry.
use super::*;
use ferroui_markup::{compiled_operands as wire, resource_opcodes as op};
use std::sync::OnceLock;

type DecodeResult<T> = Result<T, &'static str>;

/// A program in one theme unit. Its immutable, validated rows are shared by all activations;
/// each activation still captures its own declaration context and owns its resource objects.
/// Generated code gives each run its own static so unused units and their imports can be trimmed.
pub struct TypedResourceProgram<R: 'static> {
    code: &'static [u8],
    strings: &'static [&'static str],
    namespaces: &'static [XmlNamespaceTable],
    decoded: OnceLock<DecodeResult<Box<[R]>>>,
}

impl<R> TypedResourceProgram<R> {
    /// Describes the code and the local imports at its compiled call site.
    pub const fn new(code: &'static [u8], strings: &'static [&'static str], namespaces: &'static [XmlNamespaceTable]) -> Self {
        Self { code, strings, namespaces, decoded: OnceLock::new() }
    }

    fn rows(&'static self, opcode: u8, read: fn(&mut Reader, Header) -> DecodeResult<R>) -> Result<&'static [R], XamlLoadError> {
        let invalid = |reason| XamlLoadError::with_message(format!("Invalid compiled XAML resource program: {reason}"));
        let Some(bytes) = self.code.strip_prefix(&[op::VERSION, opcode]) else {
            return Err(invalid("version or opcode differs from its compiled call site"));
        };
        let rows = self.decoded.get_or_init(|| {
            let mut input = Reader { bytes, strings: self.strings, namespaces: self.namespaces, line: 0, column: 0 };
            let count = input.uint()? as usize;
            // Each row has at least a key, two coordinates and a value. Validate the
            // count before allowing it to determine an allocation size.
            if count > input.bytes.len() / 4 {
                return Err("truncated rows");
            }
            let mut rows = Vec::with_capacity(count);
            for _ in 0..count {
                let key = input.string()?;
                input.line = input.delta(input.line)?;
                input.column = input.delta(input.column)?;
                let header = Header { key, line: input.line, position: input.column };
                rows.push(read(&mut input, header)?);
            }
            if !input.bytes.is_empty() { return Err("trailing bytes"); }
            Ok(rows.into_boxed_slice())
        });
        rows.as_deref().map_err(|reason| invalid(*reason))
    }
}

#[derive(Clone, Copy)]
struct Header {
    key: &'static str,
    line: i32,
    position: i32,
}

struct Reader {
    bytes: &'static [u8],
    strings: &'static [&'static str],
    namespaces: &'static [XmlNamespaceTable],
    line: i32,
    column: i32,
}

impl Reader {
    fn uint(&mut self) -> DecodeResult<u32> { wire::read_unsigned(&mut self.bytes) }

    fn delta(&mut self, base: i32) -> DecodeResult<i32> {
        Ok(base.wrapping_add(wire::read_signed(&mut self.bytes)?))
    }

    fn string(&mut self) -> DecodeResult<&'static str> {
        let index = self.uint()? as usize;
        self.strings.get(index).copied().ok_or("unknown string import")
    }

    fn namespace(&mut self) -> DecodeResult<Option<XmlNamespaceTable>> {
        match self.uint()? {
            0 => Ok(None),
            index => self.namespaces.get(index as usize - 1).copied().map(Some).ok_or("unknown namespace import"),
        }
    }

    fn number(&mut self) -> DecodeResult<f64> { wire::read_number(&mut self.bytes) }

    fn components(&mut self) -> DecodeResult<[f64; 4]> {
        Ok([self.number()?, self.number()?, self.number()?, self.number()?])
    }

    fn color(&mut self) -> DecodeResult<u32> { Ok(u32::from_le_bytes(wire::take(&mut self.bytes)?)) }
}

// A method on the typed program names its own runtime builder. Keeping these calls separate
// lets the linker discard a brush, binding or value factory along with the last unit using it.
macro_rules! deferred_string {
    ($kind:ident, $method:ident, $opcode:expr, $operand:ident) => {
        impl TypedResourceProgram<$kind> {
            pub fn $method(&'static self, dict: &Ref<ResourceDictionary>, context: &Rc<XamlIlContext>,
                           doc: &'static DocumentInfo) -> Result<(), XamlLoadError> {
                let rows = self.rows($opcode, |input, Header { key, line, position }| {
                    Ok($kind {
                        key, line, position, namespaces: input.namespace()?, value_line: input.delta(line)?,
                        value_position: input.delta(position)?, $operand: input.string()?,
                    })
                })?;
                $method(dict, context, doc, rows)
            }
        }
    };
}
deferred_string!(ResourceAliasRow, add_resource_aliases, op::ALIAS, target);
deferred_string!(DynamicBrushRow, add_dynamic_brushes, op::DYNAMIC_BRUSH, color_key);
deferred_string!(StaticBrushRow, add_static_brushes, op::STATIC_BRUSH, color_key);

impl TypedResourceProgram<LiteralBrushRow> {
    pub fn add_literal_brushes(&'static self, dict: &Ref<ResourceDictionary>, context: &Rc<XamlIlContext>,
                               doc: &'static DocumentInfo) -> Result<(), XamlLoadError> {
        let rows = self.rows(op::LITERAL_BRUSH, |input, Header { key, line, position }| {
            Ok(LiteralBrushRow { key, line, position, namespaces: input.namespace()?, color: input.color()? })
        })?;
        add_literal_brushes(dict, context, doc, rows)
    }
}

macro_rules! plain {
    ($type:ty, $method:ident, $opcode:expr, $operand:ident) => {
        impl TypedResourceProgram<ValueRow<$type>> {
            pub fn $method(&'static self, dict: &Ref<ResourceDictionary>) -> Result<(), XamlLoadError> {
                let rows = self.rows($opcode, |input, Header { key, line, position }| {
                    Ok(ValueRow { key, line, position, value: input.$operand()? })
                })?;
                $method(dict, rows)
            }
        }
    };
}
plain!(f64, add_double_resources, op::DOUBLE, number);
plain!([f64; 4], add_thickness_resources, op::THICKNESS, components);
plain!([f64; 4], add_corner_radius_resources, op::CORNER_RADIUS, components);
plain!(u32, add_color_resources, op::COLOR, color);
plain!(&'static str, add_string_resources, op::TEXT, string);
