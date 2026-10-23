//! Port of `ViewModels/ReportViewModel.cs`.

use super::{identity_eq, ViewModelBase};

/// A report of the reports page.
pub struct ReportViewModel {
    title: String,
    summary: String,
    last_generated: String,
    status: String,
}

identity_eq!(ReportViewModel);

impl ReportViewModel {
    /// `last_generated` is shown as written (the reference shows a `DateTime`).
    pub fn new(title: &str, summary: &str, last_generated: &str, status: &str) -> Self {
        Self { title: title.to_string(), summary: summary.to_string(), last_generated: last_generated.to_string(), status: status.to_string() }
    }

    pub fn summary(&self) -> &str {
        &self.summary
    }

    pub fn last_generated(&self) -> &str {
        &self.last_generated
    }

    pub fn status(&self) -> &str {
        &self.status
    }
}

impl ViewModelBase for ReportViewModel {
    fn title(&self) -> String {
        self.title.clone()
    }
}
