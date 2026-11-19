//! Portable XAML/RSX translation for the visual designer. Parsed Rust is data,
//! and the mapping is tied to the source snapshot used to construct it.

mod animation;
mod blocks;
mod catalog;
mod image_source;
mod language;
mod literal;
mod markup;
mod placement;
mod project;
mod registered;
mod source_edits;
mod states;
mod tree;
mod xaml;

pub use animation::{RsxAnimation, RsxAnimationIssue};
pub use language::{RsxLanguageDocument, analyze_rsx};
pub use markup::{RsxPreviewMarkup, RsxSourceMap};
pub use project::{RsxNode, RsxProjection, project_rust};
pub use registered::RsxCatalog;
pub use source_edits::{RsxEvent, event_setter};
pub use states::{RsxStateGroup, RsxStateSource};
pub use xaml::{apply_rsx, xaml_to_rsx};

/// Count source-authored invocations without requiring registered control types.
pub fn invocation_count(source: &str) -> usize {
    if !source.contains("rsx") {
        return 0;
    }
    blocks::blocks(source).map_or(0, |matches| matches.len())
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod animation_tests;
#[cfg(test)]
mod registered_tests;

#[cfg(test)]
mod state_tests;
