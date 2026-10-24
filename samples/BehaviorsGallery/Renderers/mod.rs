//! The renderers of the gallery (namespace `BehaviorsGallery.Renderers`): one module per reference
//! file of `Renderers/`. A renderer is registered by its owner page (its entry of the page table,
//! `pages/mod.rs`); its module line stays a comment until that page is ported (`PAGES.md`).

use ferroui_base::metadata::MarkupType;
use ferroui_base::TypeInfo;

pub mod rotating_square_renderer; // owner: RenderTargetBitmapView
pub mod sample_writeable_bitmap_renderer; // owner: WriteableBitmapView
pub mod solid_square_renderer; // owner: RenderTargetBitmapView

/// The shared classes of this namespace: none.
pub(crate) const TYPES: &[&TypeInfo] = &[];

/// The shared types of this namespace declared with `ferro_markup_type!`: none.
pub(crate) const MARKUP_TYPES: &[&MarkupType] = &[];

/// What the untyped value conversions must know about the shared types of this namespace: nothing.
pub(crate) fn register_value_types() {}
