//! Weak widget callback scope. A local upgrade keeps synchronous callbacks alive across
//! reentrant shutdown; ending the scope disables its retained widgets.
use crate::settings_panel::SettingsPanel;
use ferroui_base::{Ref, WeakRef};
use std::{cell::RefCell, rc::Rc};
pub(crate) struct SettingsCallbackOwner {
    panel: RefCell<Option<WeakRef<SettingsPanel>>>,
}
impl SettingsCallbackOwner {
    pub fn new(panel: Ref<SettingsPanel>) -> Rc<Self> {
        Rc::new(Self {
            panel: RefCell::new(Some(panel.downgrade())),
        })
    }
    pub fn retain(&self) -> Option<Ref<SettingsPanel>> {
        self.panel.borrow().as_ref().and_then(WeakRef::upgrade)
    }
    pub fn release(&self) {
        self.panel.take();
    }
}
