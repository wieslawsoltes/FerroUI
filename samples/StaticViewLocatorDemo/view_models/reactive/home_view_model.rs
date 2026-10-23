//! Port of `ViewModels/HomeViewModel.cs` of the ReactiveUI demo.

use crate::view_models::{identity_eq, ViewModelBase};

/// The home page of the ReactiveUI demo.
#[derive(Default)]
pub struct HomeViewModel;

identity_eq!(HomeViewModel);

impl HomeViewModel {
    pub fn description(&self) -> &str {
        "The generated data template creates HomeView through an exact static lookup and assigns this view model."
    }
}

impl ViewModelBase for HomeViewModel {
    fn title(&self) -> String {
        "Home view resolved statically".to_string()
    }
}
