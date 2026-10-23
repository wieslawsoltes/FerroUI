//! Port of `ViewModels/MetricViewModel.cs`.

use super::{identity_eq, ViewModelBase};

/// A metric of the dashboard.
pub struct MetricViewModel {
    name: String,
    unit: String,
    value: f64,
    description: String,
}

identity_eq!(MetricViewModel);

impl MetricViewModel {
    pub fn new(name: &str, unit: &str, value: f64, description: &str) -> Self {
        Self { name: name.to_string(), unit: unit.to_string(), value, description: description.to_string() }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn unit(&self) -> &str {
        &self.unit
    }

    pub fn value(&self) -> f64 {
        self.value
    }

    pub fn description(&self) -> &str {
        &self.description
    }
}

impl ViewModelBase for MetricViewModel {
    fn title(&self) -> String {
        self.name.clone()
    }
}
