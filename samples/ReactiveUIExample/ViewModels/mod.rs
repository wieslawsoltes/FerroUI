//! The view models of the example (namespace `ReactiveUIExample.ViewModels`): one module per
//! reference file, and the view models of the DynamicData page, an addition of the port.

use crate::markup::XamlClass;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::metadata::{MarkupType, MarkupTyped};
use ferroui_base::TypeInfo;

mod command_lab_view_model;
mod dynamic_data_view_model;
mod feature_item_view_model;
mod feature_view_model;
mod main_view_model;
mod metric_card_view_model;
mod metrics_view_model;
mod overview_view_model;
mod page_view_model;
mod view_model_base;

pub use command_lab_view_model::{CommandLabViewModel, FAILURE_MESSAGE};
pub use dynamic_data_view_model::{DynamicDataViewModel, ADDED_FEATURE_AREA, SEARCH_THROTTLE};
pub use feature_item_view_model::FeatureItemViewModel;
pub use feature_view_model::FeatureViewModel;
pub use main_view_model::MainViewModel;
pub use metric_card_view_model::MetricCardViewModel;
pub use metrics_view_model::MetricsViewModel;
pub use overview_view_model::OverviewViewModel;
pub use page_view_model::PageViewModel;
pub use view_model_base::ViewModelBase;

/// The classes of this namespace (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[];

/// The classes of this namespace that have a document (`&X::XAML_CLASS`).
pub(crate) const CLASSES: &[&XamlClass] = &[];

/// The types of this namespace declared with `ferro_markup_type!` / `ferro_markup_enum!`
/// (`<X as MarkupTyped>::MARKUP`).
pub(crate) const MARKUP_TYPES: &[&MarkupType] = &[
    <CommandLabViewModel as MarkupTyped>::MARKUP,
    <DynamicDataViewModel as MarkupTyped>::MARKUP,
    <FeatureItemViewModel as MarkupTyped>::MARKUP,
    <FeatureViewModel as MarkupTyped>::MARKUP,
    <MainViewModel as MarkupTyped>::MARKUP,
    <MetricCardViewModel as MarkupTyped>::MARKUP,
    <MetricsViewModel as MarkupTyped>::MARKUP,
    <OverviewViewModel as MarkupTyped>::MARKUP,
];

/// What the untyped value conversions must know about the types of this namespace.
pub(crate) fn register_value_types() {
    ValueTypes::register_reference::<CommandLabViewModel>();
    ValueTypes::register_reference::<DynamicDataViewModel>();
    ValueTypes::register_reference::<FeatureItemViewModel>();
    ValueTypes::register_reference::<FeatureViewModel>();
    ValueTypes::register_reference::<MainViewModel>();
    ValueTypes::register_reference::<MetricCardViewModel>();
    ValueTypes::register_reference::<MetricsViewModel>();
    ValueTypes::register_reference::<OverviewViewModel>();
}
