//! Text elements: the inline content model of text controls.

mod bold;
mod i_inline_host;
mod inline;
mod inline_collection;
mod inline_run;
mod inline_ui_container;
mod italic;
mod line_break;
mod run;
mod span;
mod text_element;
mod underline;

pub use bold::Bold;
pub use i_inline_host::IInlineHost;
pub use inline::{Inline, InlineImpl, InlineImplExt, InlineVTable};
pub use inline_collection::InlineCollection;
pub use inline_run::EmbeddedControlRun;
pub use inline_ui_container::InlineUIContainer;
pub use italic::Italic;
pub use line_break::LineBreak;
pub use run::Run;
pub use span::Span;
pub use text_element::{TextElement, TextElementImpl, TextElementImplExt, TextElementVTable};
pub use underline::Underline;

#[cfg(test)]
mod inline_tests;
