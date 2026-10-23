//! The view models of the sample (`ViewModels/` of the reference), one module per type: the
//! generator finds the view of `view_models::dashboard_view_model::DashboardViewModel` in
//! `views::dashboard_view::DashboardView` by its default rules (`view_models=views`,
//! `_view_model=_view`, `ViewModel=View`).

pub mod dashboard_view_model;
pub mod main_window_view_model;
pub mod metric_view_model;
pub mod reactive;
pub mod report_view_model;
pub mod reports_view_model;
pub mod settings_view_model;
pub mod test_view_model;
pub mod view_model_base;

pub use dashboard_view_model::DashboardViewModel;
pub use main_window_view_model::MainWindowViewModel;
pub use metric_view_model::MetricViewModel;
pub use report_view_model::ReportViewModel;
pub use reports_view_model::ReportsViewModel;
pub use settings_view_model::SettingsViewModel;
pub use test_view_model::TestViewModel;
pub use view_model_base::ViewModelBase;

/// Compares view models by identity, as the reference's classes compare.
macro_rules! identity_eq {
    ($type:ty) => {
        impl PartialEq for $type {
            fn eq(&self, other: &Self) -> bool {
                std::ptr::eq(self, other)
            }
        }
    };
}

pub(crate) use identity_eq;
