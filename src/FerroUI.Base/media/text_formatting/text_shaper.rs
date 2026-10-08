use std::rc::Rc;

use crate::media::text_formatting::{ShapedBuffer, TextShaperOptions};
use crate::platform::ITextShaperImpl;
use crate::{FerroLocator, LocatorExtensions};
use crate::utilities::ReadOnlyMemory;

/// A class that is responsible for text shaping.
pub struct TextShaper {
    platform_impl: Rc<dyn ITextShaperImpl>,
}

impl TextShaper {
    pub fn new(platform_impl: Rc<dyn ITextShaperImpl>) -> Self {
        Self { platform_impl }
    }

    /// Gets the current text shaper.
    ///
    /// Panics when no text shaper backend is registered.
    pub fn current() -> Rc<TextShaper> {
        if let Some(current) = FerroLocator::current().get_service::<TextShaper>() {
            return current;
        }

        let text_shaper_impl = FerroLocator::current().get_required_service::<dyn ITextShaperImpl>();

        let current = Rc::new(TextShaper::new(text_shaper_impl));

        FerroLocator::current_mutable().bind::<TextShaper>().to_constant(current.clone());

        current
    }

    /// Shapes UTF-16 text.
    pub fn shape_text(&self, text: &ReadOnlyMemory<u16>, options: &TextShaperOptions) -> Rc<ShapedBuffer> {
        crate::perf_count!(TextRunsShaped);
        self.platform_impl.shape_text(text, options)
    }

    /// Shapes a string.
    pub fn shape_str(&self, text: &str, options: &TextShaperOptions) -> Rc<ShapedBuffer> {
        self.shape_text(&ReadOnlyMemory::<u16>::from_str(text), options)
    }
}
