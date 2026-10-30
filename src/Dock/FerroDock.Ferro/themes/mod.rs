mod dock_density_style;
pub use dock_density_style::DockDensityStyle;
mod i_dock_theme_manager;
pub use i_dock_theme_manager::IDockThemeManager;
mod dock_preset_theme_manager_base;
pub use dock_preset_theme_manager_base::{
    DockPresetThemeManagerBase, DockPresetThemeManagerHooks, PresetThemeManagerArgumentError,
};
