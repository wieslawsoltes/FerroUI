//! The pages of the sample (namespace `RenderDemo.Pages`): one module per upstream
//! code-behind file, next to its document.

use crate::markup::XamlClass;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::metadata::{MarkupType, MarkupTyped};
use ferroui_base::animation::ICustomAnimator;
use ferroui_base::TypeInfo;
use std::rc::Rc;

mod animation_speed_page;
mod animations_page;
mod brushes_page;
mod clipping_page;
mod custom_animator_page;
mod custom_skia_page;
mod custom_string_animator;
mod drawing_page;
mod formatted_text_page;
mod geometry_hit_testing_page;
mod glyph_run_page;
mod hit_testing_page;
mod line_bounds_page;
mod path_measurement_page;
mod render_target_bitmap_page;
mod resize_pattern_page;
mod spring_animations_page;
mod text_formatter_page;
mod transform_3d_page;
mod transitions_page;
mod writeable_bitmap_page;

pub use animation_speed_page::AnimationSpeedPage;
pub use animations_page::AnimationsPage;
pub use brushes_page::BrushesPage;
pub use clipping_page::ClippingPage;
pub use custom_animator_page::CustomAnimatorPage;
pub use custom_skia_page::CustomSkiaPage;
pub use custom_string_animator::CustomStringAnimator;
pub use drawing_page::DrawingPage;
pub use formatted_text_page::FormattedTextPage;
pub use geometry_hit_testing_page::GeometryHitTestingPage;
pub use glyph_run_page::{GlyphRunControl, GlyphRunGeometryControl, GlyphRunPage};
pub use hit_testing_page::HitTestingPage;
pub use line_bounds_page::LineBoundsPage;
pub use path_measurement_page::PathMeasurementPage;
pub use render_target_bitmap_page::RenderTargetBitmapPage;
pub use resize_pattern_page::ResizePatternPage;
pub use spring_animations_page::SpringAnimationsPage;
pub use text_formatter_page::TextFormatterPage;
pub use transform_3d_page::Transform3DPage;
pub use transitions_page::TransitionsPage;
pub use writeable_bitmap_page::WriteableBitmapPage;

/// The classes of this namespace (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[
    AnimationSpeedPage::TYPE,
    AnimationsPage::TYPE,
    BrushesPage::TYPE,
    ClippingPage::TYPE,
    CustomAnimatorPage::TYPE,
    CustomSkiaPage::TYPE,
    DrawingPage::TYPE,
    FormattedTextPage::TYPE,
    GeometryHitTestingPage::TYPE,
    GlyphRunControl::TYPE,
    GlyphRunGeometryControl::TYPE,
    GlyphRunPage::TYPE,
    HitTestingPage::TYPE,
    LineBoundsPage::TYPE,
    PathMeasurementPage::TYPE,
    RenderTargetBitmapPage::TYPE,
    ResizePatternPage::TYPE,
    SpringAnimationsPage::TYPE,
    TextFormatterPage::TYPE,
    Transform3DPage::TYPE,
    TransitionsPage::TYPE,
    WriteableBitmapPage::TYPE,
];

/// The classes of this namespace that have a document (`&X::XAML_CLASS`).
pub(crate) const CLASSES: &[&XamlClass] = &[
    &AnimationSpeedPage::XAML_CLASS,
    &AnimationsPage::XAML_CLASS,
    &BrushesPage::XAML_CLASS,
    &ClippingPage::XAML_CLASS,
    &CustomAnimatorPage::XAML_CLASS,
    &DrawingPage::XAML_CLASS,
    &FormattedTextPage::XAML_CLASS,
    &GeometryHitTestingPage::XAML_CLASS,
    &GlyphRunPage::XAML_CLASS,
    &LineBoundsPage::XAML_CLASS,
    &ResizePatternPage::XAML_CLASS,
    &SpringAnimationsPage::XAML_CLASS,
    &TextFormatterPage::XAML_CLASS,
    &Transform3DPage::XAML_CLASS,
    &TransitionsPage::XAML_CLASS,
];

/// The types of this namespace declared with `ferro_markup_type!` / `ferro_markup_enum!`
/// (`<X as MarkupTyped>::MARKUP`).
pub(crate) const MARKUP_TYPES: &[&MarkupType] = &[<CustomStringAnimator as MarkupTyped>::MARKUP];

/// What the untyped value conversions must know about the types of this namespace
/// (`ValueTypes::register_reference::<X>()`, nullable forms, casts to contracts).
pub(crate) fn register_value_types() {
    ValueTypes::register_reference::<CustomStringAnimator>();
    ValueTypes::register_cast::<CustomStringAnimator, Rc<dyn ICustomAnimator>>(CustomStringAnimator::as_custom_animator);
}

/// Makes the typed lists of this namespace known to markup (`ferro_markup_list!`).
pub(crate) fn register_lists() {}
