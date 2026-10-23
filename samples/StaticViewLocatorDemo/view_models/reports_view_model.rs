//! Port of `ViewModels/ReportsViewModel.cs`.

use super::{identity_eq, ReportViewModel, ViewModelBase};
use std::rc::Rc;

/// The reports page: the reports, each shown by its own located view.
pub struct ReportsViewModel {
    reports: Vec<Rc<ReportViewModel>>,
}

identity_eq!(ReportsViewModel);

impl ReportsViewModel {
    pub fn new() -> Self {
        Self {
            reports: vec![
                Rc::new(ReportViewModel::new("Usage", "Daily active user counts split by platform.", "32 minutes ago", "Up to date")),
                Rc::new(ReportViewModel::new("Infrastructure", "Server health with CPU and memory trends.", "3 hours ago", "Requires review")),
                Rc::new(ReportViewModel::new("Commerce", "Payments accepted and refunds processed in the last week.", "Yesterday", "Regenerating")),
            ],
        }
    }

    pub fn reports(&self) -> &[Rc<ReportViewModel>] {
        &self.reports
    }
}

impl Default for ReportsViewModel {
    fn default() -> Self {
        Self::new()
    }
}

impl ViewModelBase for ReportsViewModel {
    fn title(&self) -> String {
        "Reports".to_string()
    }
}
