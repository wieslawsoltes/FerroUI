//! Port of `ViewModels/SettingsViewModel.cs` of the ReactiveUI demo.

use crate::view_models::{identity_eq, ViewModelBase};

/// The settings page of the ReactiveUI demo.
#[derive(Default)]
pub struct SettingsViewModel;

identity_eq!(SettingsViewModel);

impl SettingsViewModel {
    pub fn description(&self) -> &str {
        "This view is also discovered from IControlViewFor (IViewFor<TViewModel>) without a scan of the views at run time."
    }
}

impl ViewModelBase for SettingsViewModel {
    fn title(&self) -> String {
        "Settings view".to_string()
    }
}
