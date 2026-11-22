//! Portable XAML/RSX translation for the visual designer. Parsed Rust is data,
//! and the mapping is tied to the source snapshot used to construct it.

mod animation;
mod blocks;
mod catalog;
mod commands;
mod child_collection;
mod columns;
mod image_source;
mod item_templates;
mod language;
mod layout_edits;
mod literal;
mod local_rust;
mod markup;
mod names;
mod placement;
mod programs;
mod project;
mod registered;
mod source_array;
mod source_edits;
mod source_plan;
mod spacing_edits;
mod state_triggers;
mod states;
mod styles;
mod tree;
mod wrap_edits;
mod xaml;

pub use animation::{RsxAnimation, RsxAnimationIssue};
pub use columns::RsxColumnDeclaration;
pub use commands::{RsxCommandEdit, RsxCommandSelection, RsxCommandSlot, RsxCommandValue};
pub use item_templates::{RsxItemTemplate, RsxTemplateBody, is_item_template_owner};
pub use language::{RsxLanguageDocument, analyze_rsx};
pub use layout_edits::{is_child_list_edit, RsxChildEdit};
pub use markup::{RsxPreviewMarkup, RsxSourceMap};
pub use programs::{RsxAnimationEdit, RsxAnimationPlan};
pub use project::{RsxNode, RsxProjection, project_rust};
pub use registered::RsxCatalog;
pub use source_edits::{RsxEvent, event_setter};
pub use source_plan::{RsxSourceEdit, RsxSourceLocation};
pub use wrap_edits::{RsxWrap, RsxWrapResult};
pub use state_triggers::{RsxStateTrigger, StateTriggerGroup, StateTriggerLink, StateTriggerReport};
pub use states::{RsxStateGroup, RsxStateSource};
pub use styles::{ResourceColorMode, RsxStyle, RsxStyleEdit, RsxStyleField, RsxStyleIssue, RsxStylePlan, RsxStyleResource, RsxStyleResourceFamily, RsxStyleSetter, SharedRustColor};
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
mod animation_effect_objects_tests;
#[cfg(test)]
mod layout_edit_tests;
#[cfg(test)]
mod program_tests;
#[cfg(test)]
mod registered_tests;

#[cfg(test)]
mod state_tests;
