//! A control theme starts with its version and a byte indicating whether to push the theme onto
//! the parent stack. Members follow in source order until the end of the slice. Style instructions
//! carry a selector-present flag and open a member scope, which END_STYLE closes at its source
//! position. A standalone selector starts with VERSION and the same sequence of steps ending in
//! END that a control theme embeds after its selector-present flag. PUSH starts a fresh argument chain;
//! NOT consumes one argument and OR consumes its encoded number of alternatives. SET instructions
//! contain a property import and one value instruction. A template value contains parent and target
//! flags, an optional target type, its content type, a deferred-build import, and its source position.
//!
//! Import indices and counts use ULEB128; source positions are signed line and column deltas from
//! the preceding position anywhere in the program, initially (0, 0). The build and runtime share the
//! primitive encodings in [`crate::compiled_operands`].
pub const VERSION: u8 = 1;
pub mod member {
    pub const TARGET: u8 = 1;
    pub const BASE_TEXT: u8 = 2;
    pub const BASE_TYPE: u8 = 3;
    pub const SET: u8 = 4;
    pub const SET_CREATED: u8 = 5;
    pub const SET_PARENTED: u8 = 6;
    pub const STYLE: u8 = 7;
    pub const STYLE_PARENTED: u8 = 8;
    pub const END_STYLE: u8 = 9;
}
pub mod selector {
    pub const END: u8 = 0;
    pub const NESTING: u8 = 1;
    pub const CLASS: u8 = 2;
    pub const NAME: u8 = 3;
    pub const TEMPLATE: u8 = 4;
    pub const CHILD: u8 = 5;
    pub const DESCENDANT: u8 = 6;
    pub const OF_TYPE: u8 = 7;
    pub const IS: u8 = 8;
    pub const NTH: u8 = 9;
    pub const NTH_LAST: u8 = 10;
    pub const EQUALS: u8 = 11;
    pub const PUSH: u8 = 12;
    pub const NOT: u8 = 13;
    pub const OR: u8 = 14;
    /// Property import, then lexical operand slot. Standalone selectors evaluate this conversion
    /// in their original scope at this instruction; whole-theme selectors carry closed values.
    pub const CAPTURED_EQUALS: u8 = 15;
}
pub mod value {
    pub const DYNAMIC: u8 = 1;
    pub const STATIC: u8 = 2;
    pub const TEMPLATE: u8 = 3;
    pub const NULL: u8 = 4;
    pub const FALSE: u8 = 5;
    pub const TRUE: u8 = 6;
    pub const INT: u8 = 7;
    pub const DOUBLE: u8 = 8;
    pub const TEXT: u8 = 9;
    pub const THICKNESS: u8 = 10;
    pub const CORNER: u8 = 11;
    pub const EXPRESSION: u8 = 12;
    pub const COMPUTED: u8 = 13;
    pub const ABSENT: u8 = 14;
}
