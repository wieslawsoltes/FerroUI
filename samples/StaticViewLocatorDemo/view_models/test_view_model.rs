//! Port of `ViewModels/TestViewModel.cs`.

use super::{identity_eq, ViewModelBase};

/// The test page.
#[derive(Default)]
pub struct TestViewModel;

identity_eq!(TestViewModel);

impl TestViewModel {
    pub fn message(&self) -> &str {
        "This page is located statically: the generated table maps TestViewModel to TestView."
    }
}

impl ViewModelBase for TestViewModel {
    fn title(&self) -> String {
        "Test".to_string()
    }
}
