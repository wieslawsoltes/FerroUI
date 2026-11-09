//! Shared presentation counterpart of SettingsPanel._keyboard and _refreshing.
//! Retained source callbacks resolve the current target when invoked, not when Build created them.
use ferroboard_core::{engine::LanguageModels, settings::KeyboardSettings};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::Arc,
};
pub(crate) struct PanelTarget {
    settings: Rc<KeyboardSettings>,
    models: Arc<LanguageModels>,
}
pub(crate) struct SettingsPanelContext {
    target: RefCell<Option<PanelTarget>>,
    refreshing: Cell<bool>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MissingPanelTarget;
impl std::fmt::Display for MissingPanelTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "settings panel has no current keyboard")
    }
}
impl std::error::Error for MissingPanelTarget {}
pub(crate) struct PanelRefreshGuard<'a>(&'a Cell<bool>);
impl Drop for PanelRefreshGuard<'_> {
    fn drop(&mut self) {
        self.0.set(false);
    } // Source Boolean/finally semantics, not a nesting count.
}
impl SettingsPanelContext {
    pub(crate) fn new(settings: Rc<KeyboardSettings>, models: Arc<LanguageModels>) -> Self {
        let context = Self::detached();
        context.retarget(Some((settings, models)));
        context
    }
    pub(crate) fn detached() -> Self {
        Self {
            target: RefCell::new(None),
            refreshing: Cell::new(false),
        }
    }
    pub(crate) fn retarget(&self, target: Option<(Rc<KeyboardSettings>, Arc<LanguageModels>)>) {
        let previous = self
            .target
            .replace(target.map(|(settings, models)| PanelTarget { settings, models }));
        // Dropping the last service reference can drop user event captures. The new target is
        // already visible, and no context borrow may remain while those destructors run.
        drop(previous);
    }
    pub(crate) fn try_settings(&self) -> Result<Rc<KeyboardSettings>, MissingPanelTarget> {
        self.target
            .borrow()
            .as_ref()
            .map(|t| t.settings.clone())
            .ok_or(MissingPanelTarget)
    }
    pub(crate) fn settings(&self) -> Rc<KeyboardSettings> {
        self.try_settings()
            .expect("source Settings getter requires a current keyboard")
    }
    pub(crate) fn models(&self) -> Arc<LanguageModels> {
        self.target
            .borrow()
            .as_ref()
            .map(|t| t.models.clone())
            .expect("source Engine getter requires a current keyboard")
    }
    pub(crate) fn settings_source(self: &Rc<Self>) -> crate::languages::SettingsSource {
        let context = self.clone();
        Rc::new(move || context.try_settings().map_err(|e| e.to_string()))
    }
    pub(crate) fn reset_settings(&self) {
        self.settings()
            .copy_from(&KeyboardSettings::default())
            .expect("source reset");
    }
    pub(crate) fn forget_learned_words(&self) {
        for (_, dictionary) in self.models().user_dictionaries().iter() {
            dictionary.clear();
        }
    }
    pub(crate) fn is_refreshing(&self) -> bool {
        self.refreshing.get()
    }
    pub(crate) fn begin_refresh(&self) -> PanelRefreshGuard<'_> {
        self.refreshing.set(true);
        PanelRefreshGuard(&self.refreshing)
    }
}
