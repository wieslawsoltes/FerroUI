//! Upstream's `Media/TextFormatting` folder of the Skia unit tests.

mod empty_shaped_buffer_tests;
mod multi_buffer_text_source;
mod shaped_buffer_shared_storage_tests;
mod shaping_capability_fallback_tests;
mod single_buffer_text_source;
mod split_text_runs_tests;
mod tables;
mod text_characters_tests;
mod text_collapsing_bidi_tests;
mod text_formatter_tests;
mod text_formatter_wrap_characterization_tests;
mod text_layout_tests;
mod text_line_tests;
mod text_run_cache_tests;
mod text_shaper_tests;

pub(crate) use multi_buffer_text_source::MultiBufferTextSource;
pub(crate) use single_buffer_text_source::SingleBufferTextSource;
