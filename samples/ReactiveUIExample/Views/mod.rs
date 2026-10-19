//! The views of the example (namespace `ReactiveUIExample.Views`): one class per reference view,
//! each with its document, the view of the DynamicData page (an addition of the port), and what the
//! port adds for the bindings of the views ([`BindableView`]) and for the list of the DynamicData
//! page ([`ObservableCollectionItems`]).

use crate::markup::XamlClass;
use ferroui_base::metadata::MarkupType;
use ferroui_base::TypeInfo;

mod bindable_view;
mod command_lab_view;
mod dynamic_data_view;
mod feature_view;
mod main_view;
mod main_window;
mod memory_metric_card_view;
mod metrics_view;
mod observable_collection_items;
mod overview_view;
mod performance_metric_card_view;

pub use bindable_view::{element_control, element_property, owned_element_property, BindableView};
pub use command_lab_view::CommandLabView;
pub use dynamic_data_view::DynamicDataView;
pub use feature_view::FeatureView;
pub use main_view::MainView;
pub use main_window::MainWindow;
pub use memory_metric_card_view::MemoryMetricCardView;
pub use metrics_view::MetricsView;
pub use observable_collection_items::ObservableCollectionItems;
pub use overview_view::OverviewView;
pub use performance_metric_card_view::PerformanceMetricCardView;

/// The classes of this namespace (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[
    CommandLabView::TYPE,
    DynamicDataView::TYPE,
    FeatureView::TYPE,
    MainView::TYPE,
    MainWindow::TYPE,
    MemoryMetricCardView::TYPE,
    MetricsView::TYPE,
    OverviewView::TYPE,
    PerformanceMetricCardView::TYPE,
];

/// The classes of this namespace that have a document (`&X::XAML_CLASS`).
pub(crate) const CLASSES: &[&XamlClass] = &[
    &CommandLabView::XAML_CLASS,
    &DynamicDataView::XAML_CLASS,
    &FeatureView::XAML_CLASS,
    &MainView::XAML_CLASS,
    &MainWindow::XAML_CLASS,
    &MemoryMetricCardView::XAML_CLASS,
    &MetricsView::XAML_CLASS,
    &OverviewView::XAML_CLASS,
    &PerformanceMetricCardView::XAML_CLASS,
];

/// The types of this namespace declared with `ferro_markup_type!` / `ferro_markup_enum!`
/// (`<X as MarkupTyped>::MARKUP`).
pub(crate) const MARKUP_TYPES: &[&MarkupType] = &[];

/// What the untyped value conversions must know about the types of this namespace.
pub(crate) fn register_value_types() {}
