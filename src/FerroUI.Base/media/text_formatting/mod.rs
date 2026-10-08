//! Text formatting: shaping, line breaking, bidi reordering and text layout.
//!
//! # Index unit: UTF-16 code units
//!
//! Every index and length in this subsystem — `CharacterHit`, text source
//! indices, `TextRun::length`, glyph clusters, text ranges, caret positions —
//! counts **UTF-16 code units**, exactly as upstream, whose text is
//! `ReadOnlyMemory<char>`. Text is stored as
//! [`ReadOnlyMemory<u16>`](crate::utilities::ReadOnlyMemory) (a shared
//! `Rc<[u16]>` plus a range), so slicing a run or splitting a line never
//! copies text and indices need no translation anywhere inside the formatter.
//!
//! This keeps positions identical to upstream (XAML `SelectionStart`,
//! `CaretIndex`, hit testing and the text shaper's clusters all agree) and
//! keeps the bidi, line break and grapheme algorithms literal ports.
//!
//! The cost is at the boundary: a Rust `String`/`&str` is transcoded once
//! into UTF-16 when it becomes text of a run (`ReadOnlyMemory::<u16>::from_str`,
//! one allocation of 2 bytes per code unit), and callers that keep text as
//! UTF-8 convert positions with [`utf16_index_to_utf8`] / [`utf8_index_to_utf16`]
//! (O(n) scans; controls that edit text should keep their buffer in UTF-16 or
//! cache the mapping).

pub mod unicode;

mod bidi_reorderer;
mod drawable_text_run;
mod formatted_text_source;
pub(crate) mod formatting_buffer_helper;
mod formatting_object_pool;
mod generic_text_paragraph_properties;
mod generic_text_run_properties;
mod glyph_info;
mod i_text_drawing_sink;
mod i_text_source;
mod indexed_text_run;
mod inter_word_justification;
mod justification_properties;
mod logical_direction;
mod logical_text_run_enumerator;
mod shaped_buffer;
mod shaped_text_run;
mod split_result;
mod text_bounds;
mod text_characters;
mod text_collapsing_properties;
mod text_ellipsis_helper;
mod text_end_of_line;
mod text_end_of_paragraph;
mod text_formatter;
mod text_formatter_impl;
mod text_index;
mod text_layout;
mod text_leading_prefix_character_ellipsis;
mod text_line;
mod text_line_break;
mod text_line_impl;
mod text_line_metrics;
mod text_metrics;
mod text_paragraph_properties;
mod text_run;
mod text_run_bounds;
mod text_run_cache;
mod text_run_properties;
mod text_shaper;
mod text_shaper_options;
mod text_trailing_character_ellipsis;
mod text_trailing_word_ellipsis;
mod unshaped_text_run;
mod wrapping_text_line_break;

#[cfg(test)]
mod shaped_buffer_tests;
#[cfg(test)]
mod split_text_runs_tests;
#[cfg(test)]
mod text_characters_tests;
#[cfg(test)]
mod text_collapsing_bidi_tests;
#[cfg(test)]
mod text_formatter_tests;
#[cfg(test)]
mod text_formatter_wrap_characterization_tests;
#[cfg(test)]
mod text_layout_tests;
#[cfg(test)]
mod text_line_tests;
#[cfg(test)]
mod text_run_cache_tests;
#[cfg(any(test, feature = "testing"))]
pub mod testing;

pub use drawable_text_run::DrawableTextRun;
pub use generic_text_paragraph_properties::GenericTextParagraphProperties;
pub use generic_text_run_properties::GenericTextRunProperties;
pub use glyph_info::GlyphInfo;
pub use i_text_drawing_sink::ITextDrawingSink;
pub use i_text_source::ITextSource;
pub use formatted_text_source::FormattedTextSource;
// Internal upstream; public so the Skia unit tests reach it.
pub use formatting_object_pool::{FormattingObjectPool, ListPool, RentedList};
pub use logical_text_run_enumerator::LogicalTextRunEnumerator;
pub use inter_word_justification::InterWordJustification;
pub use justification_properties::JustificationProperties;
pub use logical_direction::LogicalDirection;
pub use shaped_buffer::ShapedBuffer;
pub use shaped_text_run::ShapedTextRun;
pub use split_result::SplitResult;
pub use text_bounds::TextBounds;
pub use text_characters::TextCharacters;
pub use text_collapsing_properties::TextCollapsingProperties;
pub use text_end_of_line::TextEndOfLine;
pub use text_end_of_paragraph::TextEndOfParagraph;
pub use text_formatter::TextFormatter;
pub use text_formatter_impl::TextFormatterImpl;
pub use text_index::{utf16_index_to_utf8, utf8_index_to_utf16};
pub use text_layout::{TextLayout, TextLayoutOptions};
pub use text_leading_prefix_character_ellipsis::TextLeadingPrefixCharacterEllipsis;
pub use text_line::TextLine;
pub use text_line_impl::TextLineImpl;
pub use text_line_break::TextLineBreak;
pub use text_line_metrics::TextLineMetrics;
pub use text_metrics::TextMetrics;
pub use text_paragraph_properties::TextParagraphProperties;
pub use text_run::{TextRun, DEFAULT_TEXT_SOURCE_LENGTH};
pub use text_run_bounds::TextRunBounds;
pub use text_run_cache::TextRunCache;
#[allow(unused_imports)] // used by tests
pub(crate) use text_run_cache::CachedShapingResult;
pub use text_run_properties::TextRunProperties;
pub use text_shaper::TextShaper;
pub use text_shaper_options::TextShaperOptions;
pub use text_trailing_character_ellipsis::TextTrailingCharacterEllipsis;
pub use text_trailing_word_ellipsis::TextTrailingWordEllipsis;
pub use unshaped_text_run::UnshapedTextRun;
pub use wrapping_text_line_break::WrappingTextLineBreak;
