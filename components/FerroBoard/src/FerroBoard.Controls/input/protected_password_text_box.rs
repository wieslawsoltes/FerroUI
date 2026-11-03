//! Native password-field companion for the selected framework, which lacks PasswordBox.
//!
//! The isolated SDK now hides masked TextBox values, notifications and child queries.
//! This companion keeps an always-opaque peer, including when a caller changes PasswordChar.
//! It retains native TextBox editing and styling without a value provider or internal text
//! children. Full platform secure-text roles and assistive-technology behavior remain unverified.
use ferroui_base::{
    FerroObjectImpl, FerroObjectImplExt, Ref, StyledElementImpl, TypeInfo, VisualImpl,
    data::BindingPriority,
    input::{
        InputElementImpl,
        text_input::{TextInputContentType, TextInputOptions},
    },
    interactivity::InteractiveImpl,
    layout::LayoutableImpl,
    reactive::IDisposable,
};
use ferroui_controls::{
    ControlImpl, TextBox, TextBoxImpl,
    automation::peers::{
        AutomationControlType, AutomationPeer, AutomationPeerImpl, ControlAutomationPeer,
        ControlAutomationPeerImpl,
    },
    primitives::TemplatedControlImpl,
};
use std::{cell::RefCell, rc::Rc};

#[repr(C)]
pub struct ProtectedPasswordTextBox {
    base: TextBox,
    input_privacy: RefCell<Option<Rc<dyn IDisposable>>>,
}
ferroui_base::ferro_class!(ProtectedPasswordTextBox:TextBox);
ferroui_base::ferro_class_info!(ProtectedPasswordTextBox {
    new: ProtectedPasswordTextBox::new
});
ferroui_base::ferro_impl_classes!(ProtectedPasswordTextBox:VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,TemplatedControlImpl,TextBoxImpl);
impl FerroObjectImpl for ProtectedPasswordTextBox {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        // The companion remains private even if a caller removes its visual mask.
        // A removable native priority value preserves all underlying value sources.
        let value = this
            .set_value_with_priority(
                TextInputOptions::is_sensitive_property(),
                true,
                BindingPriority::Animation,
            )
            .expect("native password privacy requires an owned priority value");
        *this.input_privacy.borrow_mut() = Some(value);
    }
}
impl Drop for ProtectedPasswordTextBox {
    fn drop(&mut self) {
        if let Some(value) = self.input_privacy.get_mut().take() {
            value.dispose();
        }
    }
}
impl StyledElementImpl for ProtectedPasswordTextBox {
    fn style_key_override(_this: &Self) -> &'static TypeInfo {
        TextBox::TYPE
    }
}
impl ControlImpl for ProtectedPasswordTextBox {
    fn on_create_automation_peer(this: &Self) -> Ref<AutomationPeer> {
        ProtectedPasswordPeer::new(this).upcast()
    }
}
impl ProtectedPasswordTextBox {
    fn static_constructor() {
        TextBox::password_char_property().override_default_value::<Self>('●');
        TextInputOptions::content_type_property()
            .override_default_value::<Self>(TextInputContentType::Password);
        TextInputOptions::show_suggestions_property().override_default_value::<Self>(Some(false));
    }
    pub fn new() -> Ref<Self> {
        ferroui_base::instantiate(Self {
            base: TextBox::construct(),
            input_privacy: RefCell::new(None),
        })
    }
}

#[repr(C)]
pub(crate) struct ProtectedPasswordPeer {
    base: ControlAutomationPeer,
}
ferroui_base::ferro_class!(ProtectedPasswordPeer:ControlAutomationPeer);
ferroui_base::ferro_class_info!(ProtectedPasswordPeer {});
ferroui_base::ferro_impl_classes!(ProtectedPasswordPeer:FerroObjectImpl);
impl ProtectedPasswordPeer {
    fn new(owner: &ProtectedPasswordTextBox) -> Ref<Self> {
        ferroui_base::instantiate(Self {
            base: ControlAutomationPeer::construct(owner),
        })
    }
}
impl ControlAutomationPeerImpl for ProtectedPasswordPeer {
    fn get_children_core(_this: &Self) -> Option<Vec<Ref<AutomationPeer>>> {
        None
    }
}
impl AutomationPeerImpl for ProtectedPasswordPeer {
    fn get_automation_control_type_core(_this: &Self) -> AutomationControlType {
        AutomationControlType::Edit
    }
    fn get_placeholder_text_core(this: &Self) -> Option<String> {
        this.owner().cast::<TextBox>().unwrap().placeholder_text()
    }
}
