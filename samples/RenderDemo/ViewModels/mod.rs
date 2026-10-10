//! The view models of the sample (namespace `RenderDemo.ViewModels`): one module per upstream file.

use crate::markup::XamlClass;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::metadata::{MarkupType, MarkupTyped};
use ferroui_base::TypeInfo;

mod animation_speed_page_view_model;
mod animations_page_view_model;
mod main_window_view_model;
mod transform_3d_page_view_model;

pub use animation_speed_page_view_model::AnimationSpeedPageViewModel;
pub use animations_page_view_model::AnimationsPageViewModel;
pub use main_window_view_model::MainWindowViewModel;
pub use transform_3d_page_view_model::Transform3DPageViewModel;

/// The classes of this namespace (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[];

/// The classes of this namespace that have a document (`&X::XAML_CLASS`).
pub(crate) const CLASSES: &[&XamlClass] = &[];

/// The types of this namespace declared with `ferro_markup_type!` / `ferro_markup_enum!`
/// (`<X as MarkupTyped>::MARKUP`).
pub(crate) const MARKUP_TYPES: &[&MarkupType] = &[
    <AnimationSpeedPageViewModel as MarkupTyped>::MARKUP,
    <AnimationsPageViewModel as MarkupTyped>::MARKUP,
    <MainWindowViewModel as MarkupTyped>::MARKUP,
    <Transform3DPageViewModel as MarkupTyped>::MARKUP,
];

/// What the untyped value conversions must know about the types of this namespace
/// (`ValueTypes::register_reference::<X>()`, nullable forms, casts to contracts).
pub(crate) fn register_value_types() {
    ValueTypes::register_reference::<AnimationSpeedPageViewModel>();
    ValueTypes::register_reference::<AnimationsPageViewModel>();
    ValueTypes::register_reference::<MainWindowViewModel>();
    ValueTypes::register_reference::<Transform3DPageViewModel>();
}

/// Makes the typed lists of the view models known to markup (`ferro_markup_list!`).
pub(crate) fn register_lists() {}
