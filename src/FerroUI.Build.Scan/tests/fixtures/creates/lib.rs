//! The fixture crate of the creates edges and of the source references of the source scanner
//! (#568, step 4 of docs/porting/theme-trimming.md): classes whose code constructs other
//! classes, and code that names classes without constructing them. It is read as files and
//! never compiled.

use ferroui_base::{ferro_class, Control, Ref};

mod extensions;
pub mod item;
pub mod list;
pub mod parts;

pub use item::Item;
pub use list::List;
pub use parts::{Glyph, Handle, Marker, Separator, Spacer};

/// A plain value, which is no class: what constructs it makes no edge.
#[derive(Default)]
pub struct Thickness;

/// A class whose code names classes and constructs none of them.
#[repr(C)]
pub struct Plain {
    base: Control,
}

ferro_class!(Plain: Control);

impl Plain {
    // `Item::new()` in a comment is no construction.
    /// Nor is `Handle::new()` in documentation, and "DocumentedKey" is no string literal.
    pub fn describe(&self) -> &'static str {
        "Item::new()"
    }

    /// The type of a value is no construction.
    pub fn item(&self) -> Option<Ref<Item>> {
        None
    }

    /// A name that is no class, and a value that is no class.
    pub fn unknown(&self) -> Thickness {
        let _ = Unknown::new();
        Thickness::default()
    }

    /// A function of another name.
    pub fn handle(&self) -> Ref<Handle> {
        Handle::create()
    }

    /// A key looked up by its name, in the body of a macro.
    pub fn key(&self) -> String {
        format!("{}", "TextFillColorPrimaryBrush")
    }

    #[cfg(test)]
    fn test_only(&self) -> Ref<Spacer> {
        let _ = "OnlyInTestFunctions";
        Spacer::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_code_is_left_out() {
        let _ = "OnlyInTests";
        let _ = Marker::new();
    }
}
