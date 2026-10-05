//! Test infrastructure: hand-built fonts and table data.

mod big_endian_buffer;
mod synthetic_font;

pub(crate) use big_endian_buffer::BigEndianBuffer;
#[allow(unused_imports)] // the snapshot type is exported for the font tests built on top
pub(crate) use synthetic_font::SyntheticFontMemory;
pub(crate) use synthetic_font::SyntheticFont;
