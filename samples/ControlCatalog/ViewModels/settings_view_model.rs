//! Port of `ViewModels/SettingsViewModel.cs`.

use ferroui_base::data::model::{BindableList, Event, INotifyPropertyChanged};
use ferroui_base::ferro_markup_type;
use ferroui_controls::{ItemsSource, WindowState};
use mini_mvvm::ViewModelBase;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// Not in the original: the item of the theme variant list (Default, Light, Dark) of the
/// variant a start option selected (`App::initial_theme_variant`), so the list shows it.
fn initial_theme_variant_index() -> i32 {
    use ferroui_base::styling::ThemeVariant;
    let is_catalog = ferroui_controls::Application::current().is_some_and(|app| app.cast::<crate::App>().is_some());
    if !is_catalog {
        return 0;
    }
    let variant = crate::App::initial_theme_variant();
    if variant == ThemeVariant::light() {
        1
    } else if variant == ThemeVariant::dark() {
        2
    } else {
        0
    }
}

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
            selected_theme_variant_index: Cell::new(initial_theme_variant_index()),
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
    /// These settings are intentionally optional: the standard catalog does not link a second theme.
    pub fn macos_theme_settings_visible(&self) -> bool {
        #[cfg(feature = "macos-theme")]
        {
            return ferroui_controls::Application::current()
                .and_then(|application| application.cast::<crate::App>())
                .is_some_and(|_| crate::App::current_theme() == crate::models::CatalogTheme::MacOS);
        }
        #[cfg(not(feature = "macos-theme"))]
        false
    }

    pub fn macos_reduce_motion(&self) -> bool {
        #[cfg(feature = "macos-theme")]
        if let Some(theme) = crate::App::macos_theme() {
            return theme.reduce_motion();
        }
        false
    }

    pub fn set_macos_reduce_motion(&self, value: bool) {
        #[cfg(feature = "macos-theme")]
        if let Some(theme) = crate::App::macos_theme() {
            if theme.reduce_motion() != value {
                theme.set_reduce_motion(value);
                self.base.raise_property_changed("MacOSReduceMotion");
            }
        }
        #[cfg(not(feature = "macos-theme"))]
        let _ = value;
    }

    pub fn macos_reduce_transparency(&self) -> bool {
        #[cfg(feature = "macos-theme")]
        if let Some(theme) = crate::App::macos_theme() {
            return theme.reduce_transparency();
        }
        false
    }

    pub fn set_macos_reduce_transparency(&self, value: bool) {
        #[cfg(feature = "macos-theme")]
        if let Some(theme) = crate::App::macos_theme() {
            if theme.reduce_transparency() != value {
                theme.set_reduce_transparency(value);
                self.base.raise_property_changed("MacOSReduceTransparency");
            }
        }
        #[cfg(not(feature = "macos-theme"))]
        let _ = value;
    }

    pub fn macos_increase_contrast(&self) -> bool {
        #[cfg(feature = "macos-theme")]
        if let Some(theme) = crate::App::macos_theme() {
            return theme.increase_contrast();
        }
        false
    }

    pub fn set_macos_increase_contrast(&self, value: bool) {
        #[cfg(feature = "macos-theme")]
        if let Some(theme) = crate::App::macos_theme() {
            if theme.increase_contrast() != value {
                theme.set_increase_contrast(value);
                self.base.raise_property_changed("MacOSIncreaseContrast");
            }
        }
        #[cfg(not(feature = "macos-theme"))]
        let _ = value;
    }

    pub fn macos_window_active(&self) -> bool {
        #[cfg(feature = "macos-theme")]
        if let Some(theme) = crate::App::macos_theme() {
            return theme.window_active();
        }
        true
    }

    pub fn set_macos_window_active(&self, value: bool) {
        #[cfg(feature = "macos-theme")]
        if let Some(theme) = crate::App::macos_theme() {
            if theme.window_active() != value {
                theme.set_window_active(value);
                self.base.raise_property_changed("MacOSWindowActive");
            }
        }
        #[cfg(not(feature = "macos-theme"))]
        let _ = value;
    }

    pub fn macos_control_size_index(&self) -> i32 {
        #[cfg(feature = "macos-theme")]
        if let Some(theme) = crate::App::macos_theme() {
            use ferroui_themes_macos::ControlSize;
            return match theme.control_size() {
                ControlSize::Mini => 0,
                ControlSize::Small => 1,
                ControlSize::Regular => 2,
                ControlSize::Large => 3,
                ControlSize::ExtraLarge => 4,
            };
        }
        2
    }

    pub fn set_macos_control_size_index(&self, value: i32) {
        #[cfg(feature = "macos-theme")]
        if let Some(theme) = crate::App::macos_theme() {
            use ferroui_themes_macos::ControlSize;
            let size = match value {
                0 => ControlSize::Mini,
                1 => ControlSize::Small,
                2 => ControlSize::Regular,
                3 => ControlSize::Large,
                4 => ControlSize::ExtraLarge,
                _ => return,
            };
            if theme.control_size() != size {
                theme.set_control_size(size);
                self.base.raise_property_changed("MacOSControlSizeIndex");
            }
        }
        #[cfg(not(feature = "macos-theme"))]
        let _ = value;
    }

    pub fn macos_accent_index(&self) -> i32 {
        #[cfg(feature = "macos-theme")]
        if let Some(theme) = crate::App::macos_theme() {
            return Self::macos_accents().iter().position(|color| *color == theme.accent_color()).unwrap_or(0) as i32;
        }
        0
    }

    pub fn set_macos_accent_index(&self, value: i32) {
        #[cfg(feature = "macos-theme")]
        if let (Some(theme), Some(color)) = (crate::App::macos_theme(), Self::macos_accents().get(value as usize)) {
            if theme.accent_color() != *color {
                theme.set_accent_color(*color);
                self.base.raise_property_changed("MacOSAccentIndex");
            }
        }
        #[cfg(not(feature = "macos-theme"))]
        let _ = value;
    }

    #[cfg(feature = "macos-theme")]
    fn macos_accents() -> [Option<ferroui_base::media::Color>; 9] {
        use ferroui_base::media::Color;
        [None, Some(Color::from_rgb(0, 122, 255)), Some(Color::from_rgb(175, 82, 222)),
         Some(Color::from_rgb(255, 45, 85)), Some(Color::from_rgb(255, 59, 48)),
         Some(Color::from_rgb(255, 149, 0)), Some(Color::from_rgb(255, 204, 0)),
         Some(Color::from_rgb(52, 199, 89)), Some(Color::from_rgb(142, 142, 147))]
    }

}

ferro_markup_type!(class SettingsViewModel {
    this: Rc<SettingsViewModel>,
    handles: [SettingsViewModel, Rc<SettingsViewModel>, Option<Rc<SettingsViewModel>>],
    constructors: [() => SettingsViewModel::new],
    properties: [
        MacOSThemeSettingsVisible: bool { get: |this: &Rc<SettingsViewModel>| this.macos_theme_settings_visible() },
        MacOSReduceMotion: bool {
            get: |this: &Rc<SettingsViewModel>| this.macos_reduce_motion(),
            set: |this: &Rc<SettingsViewModel>, value: bool| this.set_macos_reduce_motion(value)
        },
        MacOSReduceTransparency: bool {
            get: |this: &Rc<SettingsViewModel>| this.macos_reduce_transparency(),
            set: |this: &Rc<SettingsViewModel>, value: bool| this.set_macos_reduce_transparency(value)
        },
        MacOSIncreaseContrast: bool {
            get: |this: &Rc<SettingsViewModel>| this.macos_increase_contrast(),
            set: |this: &Rc<SettingsViewModel>, value: bool| this.set_macos_increase_contrast(value)
        },
        MacOSWindowActive: bool {
            get: |this: &Rc<SettingsViewModel>| this.macos_window_active(),
            set: |this: &Rc<SettingsViewModel>, value: bool| this.set_macos_window_active(value)
        },
        MacOSControlSizeIndex: i32 {
            get: |this: &Rc<SettingsViewModel>| this.macos_control_size_index(),
            set: |this: &Rc<SettingsViewModel>, value: i32| this.set_macos_control_size_index(value)
        },
        MacOSAccentIndex: i32 {
            get: |this: &Rc<SettingsViewModel>| this.macos_accent_index(),
            set: |this: &Rc<SettingsViewModel>, value: i32| this.set_macos_accent_index(value)
        },
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
