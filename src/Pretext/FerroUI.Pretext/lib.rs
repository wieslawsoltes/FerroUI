//! Prepare text once, then wrap, measure, stream or shape it many times.
//!
//! Measurement and shaping backends belong to an explicit [`PretextLayout`]
//! instance. Prepared values retain their metrics when backend caches change.
//! The portable engine does not create threads, access files or read clocks.

#![forbid(unsafe_code)]

mod generated;
mod pretext_layout;
mod pretext_layout_analysis;
mod pretext_layout_backends;
mod pretext_layout_bidi;
mod pretext_layout_line_break;
mod pretext_layout_line_text;
mod pretext_layout_locale;
mod pretext_layout_measurement;
mod pretext_layout_preparation;
mod pretext_layout_rich_inline;
mod pretext_layout_shaping;

pub use ferroui_pretext_contracts::*;
pub use pretext_layout::*;
pub use pretext_layout_backends::*;
pub use pretext_layout_rich_inline::*;
pub use pretext_layout_shaping::*;

#[cfg(test)]
mod layout_tests;
#[cfg(test)]
mod preparation_tests;
#[cfg(test)]
mod reference_tests;
#[cfg(test)]
mod rich_inline_tests;
#[cfg(test)]
mod shaping_tests;
#[cfg(test)]
mod test_support;
