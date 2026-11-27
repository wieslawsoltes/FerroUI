//! A theme run applies dictionary instructions in XAML declaration order. Row opcodes
//! 1 through 9 are shared with the homogeneous resource programs they invoke.
pub use crate::resource_opcodes::{
    ALIAS, COLOR, CORNER_RADIUS, DOUBLE, DYNAMIC_BRUSH, LITERAL_BRUSH, STATIC_BRUSH, TEXT,
    THICKNESS,
};
pub const VERSION: u8 = 1;
pub const DEFER_TEXT: u8 = 10;
pub const DEFER_TYPE: u8 = 11;
pub const CONSTANT: u8 = 12;
pub const PARSE: u8 = 13;
pub const INT: u8 = 14;
pub const BOOL: u8 = 15;
pub const FROM_DOUBLE: u8 = 16;
pub const FROM_I64: u8 = 17;
