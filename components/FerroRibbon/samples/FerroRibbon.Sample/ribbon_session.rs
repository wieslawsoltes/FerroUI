//! Composition adapter: application commands call the library state implementation.
use crate::view_model::RibbonSession;
use ferroribbon::controls::{Ribbon, RibbonCustomizePage};
use ferroribbon_core::RibbonString;
use ferroui_base::{Ref, WeakRef};
use std::cell::RefCell;
pub struct NativeRibbonSession {
    root: WeakRef<Ribbon>,
    saved: RefCell<Option<RibbonString>>,
}
impl NativeRibbonSession {
    pub fn new(root: &Ref<Ribbon>) -> Self {
        Self {
            root: root.downgrade(),
            saved: RefCell::new(None),
        }
    }
}
impl RibbonSession for NativeRibbonSession {
    fn execute(&self, action: &str) -> Result<String, String> {
        let root = self.root.upgrade().ok_or("Ribbon window has closed")?;
        match action {
            "Customize ribbon" | "Customize quick access" => {
                let page = if action == "Customize ribbon" {
                    RibbonCustomizePage::Ribbon
                } else {
                    RibbonCustomizePage::QuickAccessToolbar
                };
                root.show_customize_dialog(page)
                    .map_err(|error| error.to_string())?;
                Ok("Ribbon customization requested.".into())
            }
            "Save ribbon" => {
                self.saved.replace(Some(root.save_state_to_json()?));
                Ok("Ribbon layout saved for this session.".into())
            }
            "Restore ribbon" => {
                let json = self
                    .saved
                    .borrow()
                    .clone()
                    .ok_or("Save a ribbon layout first")?;
                if !root.load_state_from_json(Some(json))? {
                    return Err("Saved ribbon state could not be read".into());
                }
                Ok("Ribbon layout restored.".into())
            }
            "Reset ribbon" => {
                root.reset_customization()?;
                Ok("Ribbon customization reset.".into())
            }
            "Reset quick access" => {
                root.reset_quick_access()?;
                Ok("Quick Access Toolbar reset.".into())
            }
            _ => Err(format!("Unknown ribbon action: {action}")),
        }
    }
}
