//! A block of a class of another file: what it constructs is an edge of that class.

use crate::parts::{Glyph, Separator};

impl Separator {
    pub fn glyph(&self) -> ferroui_base::Ref<Glyph> {
        Glyph::new()
    }
}
