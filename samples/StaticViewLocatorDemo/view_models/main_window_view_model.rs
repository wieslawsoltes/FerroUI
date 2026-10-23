//! Port of `ViewModels/MainWindowViewModel.cs`: the pages of the window.
//!
//! The window has no view of its own (`MainWindowView`): the generated locators list the view
//! model among their missing views, as the reference's do.

use super::reactive::ReactiveDemoViewModel;
use super::{identity_eq, DashboardViewModel, ReportsViewModel, SettingsViewModel, TestViewModel, ViewModelBase};
use ferroui_base::BoxedValue;
use std::rc::Rc;

/// A page: its title and its view model as the value of a property.
pub struct Page {
    pub title: String,
    pub view_model: BoxedValue,
}

/// The view model of the window: its pages.
pub struct MainWindowViewModel {
    pages: Vec<Page>,
}

identity_eq!(MainWindowViewModel);

impl MainWindowViewModel {
    pub fn new() -> Self {
        let pages = vec![
            page(Rc::new(DashboardViewModel::new())),
            page(Rc::new(TestViewModel)),
            page(Rc::new(SettingsViewModel::new())),
            page(Rc::new(ReportsViewModel::new())),
            page(Rc::new(ReactiveDemoViewModel::new())),
        ];
        Self { pages }
    }

    pub fn pages(&self) -> &[Page] {
        &self.pages
    }
}

impl Default for MainWindowViewModel {
    fn default() -> Self {
        Self::new()
    }
}

fn page<VM: ViewModelBase + PartialEq + 'static>(view_model: Rc<VM>) -> Page {
    Page { title: view_model.title(), view_model }
}
