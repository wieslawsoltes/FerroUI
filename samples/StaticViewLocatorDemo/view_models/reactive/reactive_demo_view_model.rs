//! Port of `ViewModels/MainWindowViewModel.cs` of the ReactiveUI demo: the navigation between the
//! home page, the settings page and the settings wrapped in a context host.

use super::{ContextHost, HomeViewModel, SettingsViewModel};
use crate::view_models::{identity_eq, ViewModelBase};
use ferroui_base::BoxedValue;
use std::rc::Rc;

/// What the page shows: the content of the data template and the view model of the view host.
#[derive(Clone)]
pub struct Navigation {
    /// The content of the content control (`CurrentContent`).
    pub content: BoxedValue,
    /// The view model of the view host of ReactiveUI (`CurrentViewModel`).
    pub view_model: BoxedValue,
}

/// The page of ReactiveUI.
pub struct ReactiveDemoViewModel {
    home: Rc<HomeViewModel>,
    settings: Rc<SettingsViewModel>,
    wrapped_settings: Rc<ContextHost>,
}

identity_eq!(ReactiveDemoViewModel);

impl ReactiveDemoViewModel {
    pub fn new() -> Self {
        let settings = Rc::new(SettingsViewModel);
        Self { home: Rc::new(HomeViewModel), wrapped_settings: Rc::new(ContextHost::new(settings.clone())), settings }
    }

    /// `ShowHome`.
    pub fn show_home(&self) -> Navigation {
        Navigation { content: self.home.clone(), view_model: self.home.clone() }
    }

    /// `ShowSettings`.
    pub fn show_settings(&self) -> Navigation {
        Navigation { content: self.settings.clone(), view_model: self.settings.clone() }
    }

    /// `ShowWrappedSettings`: the data template shows the host through the fallback hook, the view
    /// host the view model it holds.
    pub fn show_wrapped_settings(&self) -> Navigation {
        Navigation { content: self.wrapped_settings.clone(), view_model: self.settings.clone() }
    }
}

impl Default for ReactiveDemoViewModel {
    fn default() -> Self {
        Self::new()
    }
}

impl ViewModelBase for ReactiveDemoViewModel {
    fn title(&self) -> String {
        "ReactiveUI".to_string()
    }
}
