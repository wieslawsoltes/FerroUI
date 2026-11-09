//! The classes the others construct. They construct nothing.

use ferroui_base::{ferro_class, Control};

/// The handle an item creates with its builder.
#[repr(C)]
pub struct Handle {
    base: Control,
}

ferro_class!(Handle: Control);

/// A marker an item creates by its default value.
#[repr(C)]
pub struct Marker {
    base: Control,
}

ferro_class!(Marker: Control);

/// A spacer a list creates by a path with its module.
#[repr(C)]
pub struct Spacer {
    base: Control,
}

ferro_class!(Spacer: Control);

/// A separator a list creates through a qualified type.
#[repr(C)]
pub struct Separator {
    base: Control,
}

ferro_class!(Separator: Control);

/// A glyph the separator creates, in a block of another file.
#[repr(C)]
pub struct Glyph {
    base: Control,
}

ferro_class!(Glyph: Control);
