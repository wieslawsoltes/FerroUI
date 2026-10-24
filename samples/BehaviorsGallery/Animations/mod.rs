//! The animations of the gallery (namespace `BehaviorsGallery.Animations`): one module per
//! reference file of `Animations/`. A type only one page group uses is registered by its owner page
//! (its entry of the page table, `pages/mod.rs`); its module line stays a comment until that page
//! is ported (`PAGES.md`, "How to port a page").

use ferroui_base::metadata::MarkupType;
use ferroui_base::TypeInfo;

pub mod custom_string_animation_builder; // owner: AnimationBuilderView (also RunAnimationTriggerView, StartBuiltAnimationActionView)
pub mod custom_string_animator; // owner: AnimationBuilderView

/// The shared classes of this namespace: none.
pub(crate) const TYPES: &[&TypeInfo] = &[];

/// The shared types of this namespace declared with `ferro_markup_type!`: none.
pub(crate) const MARKUP_TYPES: &[&MarkupType] = &[];

/// What the untyped value conversions must know about the shared types of this namespace: nothing.
pub(crate) fn register_value_types() {}
