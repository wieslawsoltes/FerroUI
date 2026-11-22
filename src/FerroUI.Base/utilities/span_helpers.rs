//! UI-independent invariant numeric helpers live in FerroUI.System.
//! Preserve existing Base imports while sharing one implementation.
pub use ferroui_system::invariant_numbers::{
    parse_double, try_parse_byte, try_parse_double, try_parse_int, try_parse_uint, write_double, FixedF2,
    InvariantF64, NumberParseFlags, NumberParseFlags as NumberStyles,
};
