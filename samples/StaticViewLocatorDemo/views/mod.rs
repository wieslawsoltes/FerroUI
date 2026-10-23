//! The views of the sample (`Views/` of the reference), one module per type. A view is a class of
//! the object model derived from `UserControl`; its content is written with the builders of
//! `ferroui-view` from the view model of its data context.

pub mod dashboard_view;
pub mod metric_view;
pub mod reactive;
pub mod report_view;
pub mod reports_view;
pub mod settings_view;
pub mod test_view;

pub use dashboard_view::DashboardView;
pub use metric_view::MetricView;
pub use report_view::ReportView;
pub use reports_view::ReportsView;
pub use settings_view::SettingsView;
pub use test_view::TestView;

use ferroui_base::BoxedValue;
use std::any::Any;
use std::rc::Rc;

/// The view model of the type `VM` a data context holds.
pub fn view_model_of<VM: 'static>(data_context: Option<BoxedValue>) -> Option<Rc<VM>> {
    let value: Rc<dyn Any> = data_context?;
    value.downcast::<VM>().ok()
}

/// A view model as the content of a control, which the data templates of the window show (a
/// `ContentControl` bound to `.` in the reference's markup).
pub fn content_of<VM: PartialEq + 'static>(view_model: &Rc<VM>) -> Option<BoxedValue> {
    Some(view_model.clone())
}
