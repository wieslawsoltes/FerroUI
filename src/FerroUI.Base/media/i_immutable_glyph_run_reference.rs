use std::cell::RefCell;
use std::rc::Rc;

use crate::platform::IGlyphRunImpl;

/// An immutable reference to the platform resources of a glyph run, usable
/// after the glyph run itself changed or was disposed.
pub trait IImmutableGlyphRunReference {
    /// The referenced platform glyph run, until disposed.
    fn glyph_run(&self) -> Option<Rc<dyn IGlyphRunImpl>>;

    /// Releases the reference (C# `IDisposable.Dispose`).
    fn dispose(&self);
}

pub(crate) struct ImmutableGlyphRunReference {
    glyph_run: RefCell<Option<Rc<dyn IGlyphRunImpl>>>,
}

impl ImmutableGlyphRunReference {
    pub fn new(glyph_run: Option<Rc<dyn IGlyphRunImpl>>) -> Self {
        Self { glyph_run: RefCell::new(glyph_run) }
    }
}

impl IImmutableGlyphRunReference for ImmutableGlyphRunReference {
    fn glyph_run(&self) -> Option<Rc<dyn IGlyphRunImpl>> {
        self.glyph_run.borrow().clone()
    }

    fn dispose(&self) {
        let glyph_run = self.glyph_run.borrow_mut().take();
        release_platform_impl(glyph_run);
    }
}

/// Drops one reference to a platform glyph run, disposing the implementation
/// when it was the last one (the counterpart of disposing an `IRef<T>`).
pub(crate) fn release_platform_impl(platform_impl: Option<Rc<dyn IGlyphRunImpl>>) {
    if let Some(platform_impl) = platform_impl {
        if Rc::strong_count(&platform_impl) == 1 {
            platform_impl.dispose();
        }
    }
}
