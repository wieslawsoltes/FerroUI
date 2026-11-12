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

    /// Shapes `text`, an edit of the text `previous` is the shaping of (with the same options and
    /// its concatenation flags, `TextShaperOptions::with_concat_flags`): the glyphs of `previous`
    /// the edit leaves as they were are kept and only the text around the change is shaped again.
    /// The result is the shaping of the whole of `text` with these options, with its own
    /// concatenation flags for the next edit. `None` when nothing of `previous` can be kept (a
    /// right-to-left text, a text that is a range of a longer one, a shaping without flags, a
    /// change at the start of the text): the caller shapes `text` whole.
    pub fn shape_edited_text(
        &self,
        previous: &ShapedBuffer,
        text: &ReadOnlyMemory<u16>,
        options: &TextShaperOptions,
    ) -> Option<Rc<ShapedBuffer>> {
        crate::media::text_formatting::shaping_edit::shape_edited(previous, text, options, self)
    }
}
