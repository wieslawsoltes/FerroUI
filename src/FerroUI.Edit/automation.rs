use crate::TextEditor;
use ferroui_base::{FerroObjectImpl, Ref, ferro_class, instantiate};
use ferroui_controls::automation::peers::{
    AutomationControlType, AutomationPeer, AutomationPeerImpl, ControlAutomationPeer,
    ControlAutomationPeerImpl,
};
use ferroui_controls::automation::provider::{IValueProvider, ProviderError};
use std::rc::Rc;

#[repr(C)]
pub struct TextEditorAutomationPeer {
    base: ControlAutomationPeer,
}
ferro_class!(TextEditorAutomationPeer: ControlAutomationPeer);
ferroui_base::ferro_class_info!(TextEditorAutomationPeer {
    interfaces: [Rc<dyn IValueProvider> => TextEditorAutomationPeer::as_value_provider]
});
impl FerroObjectImpl for TextEditorAutomationPeer {}
impl ControlAutomationPeerImpl for TextEditorAutomationPeer {}
impl AutomationPeerImpl for TextEditorAutomationPeer {
    fn get_automation_control_type_core(_this: &Self) -> AutomationControlType {
        AutomationControlType::Edit
    }
}
impl TextEditorAutomationPeer {
    pub fn new(owner: &TextEditor) -> Ref<Self> {
        instantiate(Self {
            base: ControlAutomationPeer::construct(owner),
        })
    }
    pub fn owner(&self) -> Ref<TextEditor> {
        ControlAutomationPeer::owner(self)
            .cast()
            .expect("editor peer owner")
    }
    fn as_value_provider(peer: Ref<Self>) -> Rc<dyn IValueProvider> {
        Rc::new(EditorValueProvider(peer))
    }
}
struct EditorValueProvider(Ref<TextEditorAutomationPeer>);
impl IValueProvider for EditorValueProvider {
    fn peer(&self) -> Ref<AutomationPeer> {
        self.0.clone().upcast()
    }
    fn is_read_only(&self) -> bool {
        self.0.owner().is_read_only()
    }
    fn value(&self) -> Option<String> {
        Some(self.0.owner().text())
    }
    fn set_value(&self, value: Option<&str>) -> Result<(), ProviderError> {
        let owner = self.0.owner();
        if owner.is_read_only() || !owner.is_effectively_enabled() {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "Editor is read-only or disabled",
            )));
        }
        owner
            .set_text(value.unwrap_or_default())
            .map_err(|e| Box::new(std::io::Error::other(e.to_string())) as ProviderError)
    }
}
