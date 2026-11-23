//! ferroui-controls-markdown
//!
//! [`MarkdownTextBlock`] shows Markdown (CommonMark with tables, task lists and strikethrough)
//! with the rich text elements of FerroUI, so its text is laid out, selected and drawn as the
//! text of a `RichTextBlock` is. An application that does not use the control links none of
//! this crate.
//!
//! The text is treated as untrusted input: HTML is shown as text, a link opens only with an
//! `http`, `https` or `mailto` address, and an image is loaded only by the resolver the
//! application sets ([`MarkdownTextBlock::set_image_resolver`]); without one its alternative
//! text is shown.

// The markup of other crates names the types of this crate by the name of the crate.
extern crate self as ferroui_controls_markdown;

mod builder;
mod markdown_text_block;
mod register_types;
mod rust_paths;

pub use markdown_text_block::{MarkdownImageResolver, MarkdownTextBlock};
pub use register_types::{register_types, ASSEMBLY};

#[cfg(test)]
mod tests;
