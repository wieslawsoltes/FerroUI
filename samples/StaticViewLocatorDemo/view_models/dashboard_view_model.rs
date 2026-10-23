//! Port of `ViewModels/DashboardViewModel.cs`.

use super::{identity_eq, MetricViewModel, ViewModelBase};
use std::rc::Rc;

/// The dashboard: a summary and the metrics, each shown by its own located view.
pub struct DashboardViewModel {
    metrics: Vec<Rc<MetricViewModel>>,
}

identity_eq!(DashboardViewModel);

impl DashboardViewModel {
    pub fn new() -> Self {
        Self {
            metrics: vec![
                Rc::new(MetricViewModel::new("Active Users", "people", 1280.0, "Users currently signed in across all platforms.")),
                Rc::new(MetricViewModel::new("Conversion", "%", 3.7, "Percentage of visitors completing the onboarding journey.")),
                Rc::new(MetricViewModel::new("Support Tickets", "open", 5.0, "Items waiting in the support queue.")),
                Rc::new(MetricViewModel::new("Build Duration", "min", 7.4, "Average CI build time over the last 24 hours.")),
            ],
        }
    }

    pub fn summary(&self) -> &str {
        "A snapshot of the most important application metrics."
    }

    pub fn metrics(&self) -> &[Rc<MetricViewModel>] {
        &self.metrics
    }
}

impl Default for DashboardViewModel {
    fn default() -> Self {
        Self::new()
    }
}

impl ViewModelBase for DashboardViewModel {
    fn title(&self) -> String {
        "Dashboard".to_string()
    }
}
