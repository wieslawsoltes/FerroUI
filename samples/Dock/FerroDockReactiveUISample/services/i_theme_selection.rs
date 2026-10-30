//! Framework-independent application command port for the original theme actions.
/// Receives theme-selection actions from the passive view's model.
pub trait IThemeSelection {
    /// Selects the original light/dark index.
    fn select_variant(&self, index: i32);
    /// Selects the original preset index.
    fn select_preset(&self, index: i32);
    /// Gets the document tab style: Fluent (0) or Browser (1).
    fn selected_tab_theme(&self) -> i32;
    /// Selects the document tab style without replacing the current layout.
    fn select_tab_theme(&self, index: i32);
}
