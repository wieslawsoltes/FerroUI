//! Composition-root adapter to the original native docking theme manager.
use super::DockTabThemeSelection;
use crate::services::IThemeSelection;
use ferrodock_ferro::themes::IDockThemeManager;
use std::rc::Rc;
pub(crate) struct DockThemeSelectionService {
    manager: Rc<dyn IDockThemeManager>,
    tab_themes: Rc<DockTabThemeSelection>,
}
impl DockThemeSelectionService {
    pub(crate) fn new(
        manager: Rc<dyn IDockThemeManager>,
        tab_themes: Rc<DockTabThemeSelection>,
    ) -> Rc<Self> {
        Rc::new(Self {
            manager,
            tab_themes,
        })
    }
}
impl IThemeSelection for DockThemeSelectionService {
    fn select_variant(&self, index: i32) {
        self.manager.switch(index)
    }
    fn select_preset(&self, index: i32) {
        self.manager.switch_preset(index)
    }
    fn selected_tab_theme(&self) -> i32 {
        self.tab_themes.selected()
    }
    fn select_tab_theme(&self, index: i32) {
        self.tab_themes.select(index);
    }
}
