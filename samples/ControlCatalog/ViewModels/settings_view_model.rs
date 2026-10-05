//! Port of `ViewModels/SettingsViewModel.cs`.

use ferroui_base::data::model::{BindableList, Event, INotifyPropertyChanged};
use ferroui_base::ferro_markup_type;
use ferroui_controls::{ItemsSource, WindowState};
use mini_mvvm::ViewModelBase;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// The view model of the settings page.
pub struct SettingsViewModel {
    base: ViewModelBase,
    window_state: Cell<WindowState>,
    selected_decoration_index: Cell<i32>,
    selected_theme_variant_index: Cell<i32>,
    selected_transparency_level_index: Cell<i32>,
    selected_flow_direction_index: Cell<i32>,
    window_states: RefCell<Rc<BindableList<WindowState>>>,
}

impl PartialEq for SettingsViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl INotifyPropertyChanged for SettingsViewModel {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}

impl SettingsViewModel {
    pub fn new() -> Rc<SettingsViewModel> {
        Rc::new(Self {
            base: ViewModelBase::new(),
            window_state: Cell::new(WindowState::Normal),
            selected_decoration_index: Cell::new(0),
            selected_theme_variant_index: Cell::new(0),
            selected_transparency_level_index: Cell::new(0),
            selected_flow_direction_index: Cell::new(0),
            window_states: RefCell::new(BindableList::new([
                WindowState::Minimized,
                WindowState::Normal,
                WindowState::Maximized,
                WindowState::FullScreen,
            ])),
        })
    }

    pub fn window_state(&self) -> WindowState {
        self.window_state.get()
    }

    pub fn set_window_state(&self, value: WindowState) {
        self.base.raise_and_set_if_changed_cell(&self.window_state, value, "WindowState");
    }

    pub fn selected_decoration_index(&self) -> i32 {
        self.selected_decoration_index.get()
    }

    pub fn set_selected_decoration_index(&self, value: i32) {
        self.base.raise_and_set_if_changed_cell(&self.selected_decoration_index, value, "SelectedDecorationIndex");
    }

    pub fn selected_theme_variant_index(&self) -> i32 {
        self.selected_theme_variant_index.get()
    }

    pub fn set_selected_theme_variant_index(&self, value: i32) {
        self.base.raise_and_set_if_changed_cell(&self.selected_theme_variant_index, value, "SelectedThemeVariantIndex");
    }

    pub fn selected_transparency_level_index(&self) -> i32 {
        self.selected_transparency_level_index.get()
    }

    pub fn set_selected_transparency_level_index(&self, value: i32) {
        self.base.raise_and_set_if_changed_cell(
            &self.selected_transparency_level_index,
            value,
            "SelectedTransparencyLevelIndex",
        );
    }

    pub fn selected_flow_direction_index(&self) -> i32 {
        self.selected_flow_direction_index.get()
    }

    pub fn set_selected_flow_direction_index(&self, value: i32) {
        self.base.raise_and_set_if_changed_cell(&self.selected_flow_direction_index, value, "SelectedFlowDirectionIndex");
    }

    pub fn window_states(&self) -> Rc<BindableList<WindowState>> {
        self.window_states.borrow().clone()
    }

    /// An array is a reference: the notification is raised for another
    /// array, whatever its elements.
    pub fn set_window_states(&self, value: Rc<BindableList<WindowState>>) {
        self.base.raise_and_set_if_changed(&self.window_states, value, "WindowStates");
    }
}

ferro_markup_type!(class SettingsViewModel {
    this: Rc<SettingsViewModel>,
    handles: [SettingsViewModel, Rc<SettingsViewModel>, Option<Rc<SettingsViewModel>>],
    constructors: [() => SettingsViewModel::new],
    properties: [
        WindowState: WindowState {
            get: |this: &Rc<SettingsViewModel>| this.window_state(),
            set: |this: &Rc<SettingsViewModel>, value: WindowState| this.set_window_state(value)
        },
        SelectedDecorationIndex: i32 {
            get: |this: &Rc<SettingsViewModel>| this.selected_decoration_index(),
            set: |this: &Rc<SettingsViewModel>, value: i32| this.set_selected_decoration_index(value)
        },
        SelectedThemeVariantIndex: i32 {
            get: |this: &Rc<SettingsViewModel>| this.selected_theme_variant_index(),
            set: |this: &Rc<SettingsViewModel>, value: i32| this.set_selected_theme_variant_index(value)
        },
        SelectedTransparencyLevelIndex: i32 {
            get: |this: &Rc<SettingsViewModel>| this.selected_transparency_level_index(),
            set: |this: &Rc<SettingsViewModel>, value: i32| this.set_selected_transparency_level_index(value)
        },
        SelectedFlowDirectionIndex: i32 {
            get: |this: &Rc<SettingsViewModel>| this.selected_flow_direction_index(),
            set: |this: &Rc<SettingsViewModel>, value: i32| this.set_selected_flow_direction_index(value)
        },
        // A list a binding delivers to an items source property.
        WindowStates: ItemsSource { get: |this: &Rc<SettingsViewModel>| ItemsSource::from(this.window_states()) },
    ],
    notify_property_changed: SettingsViewModel,
});

#[cfg(test)]
mod tests {
    // Not ports: the upstream sample has no tests.
    use super::*;

    #[test]
    fn the_window_is_normal_and_the_states_are_listed_in_the_order_of_the_page() {
        let view_model = SettingsViewModel::new();
        assert_eq!(WindowState::Normal, view_model.window_state());
        assert_eq!(
            vec![WindowState::Minimized, WindowState::Normal, WindowState::Maximized, WindowState::FullScreen],
            view_model.window_states().items().to_vec()
        );
        assert_eq!(0, view_model.selected_theme_variant_index());
    }

    #[test]
    fn the_setters_notify_once_per_change() {
        let view_model = SettingsViewModel::new();
        let seen = Rc::new(RefCell::new(Vec::new()));
        let sink = seen.clone();
        view_model.property_changed().add(Rc::new(move |name: &str| sink.borrow_mut().push(name.to_string())));

        view_model.set_window_state(WindowState::Normal);
        view_model.set_window_state(WindowState::Maximized);
        view_model.set_selected_decoration_index(2);
        view_model.set_selected_decoration_index(2);
        view_model.set_selected_flow_direction_index(1);
        view_model.set_window_states(view_model.window_states());
        view_model.set_window_states(BindableList::new([WindowState::Normal]));
        assert_eq!(
            vec!["WindowState", "SelectedDecorationIndex", "SelectedFlowDirectionIndex", "WindowStates"],
            *seen.borrow()
        );
    }
}
