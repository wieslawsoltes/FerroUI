//! Upstream's `Media/TextFormatting` folder of the Skia unit tests.

mod multi_buffer_text_source;
mod single_buffer_text_source;
mod text_formatter_tests;

pub(crate) use multi_buffer_text_source::MultiBufferTextSource;
pub(crate) use single_buffer_text_source::SingleBufferTextSource;
