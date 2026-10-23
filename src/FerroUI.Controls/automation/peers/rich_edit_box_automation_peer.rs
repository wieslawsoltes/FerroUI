use super::{
    AutomationControlType, AutomationPeer, AutomationPeerImpl, AutomationPeerImplExt,
    ControlAutomationPeer, ControlAutomationPeerImpl,
};
use crate::automation::provider::{
    ITextEditProvider, ITextProvider, ITextProvider2, IValueProvider, ProviderAdapter,
    ProviderError, RichTextAutomationHost, RichTextAutomationPeer, RichTextAutomationState,
    TextProviderError,
};
use crate::automation::{ElementNotEnabledException, ValuePatternIdentifiers};
use crate::RichEditBox;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::{
    ferro_class, instantiate, BoxedValue, FerroObjectImpl, FerroObjectImplExt, Ref,
};
use std::rc::Rc;

/// Accessibility projection of the rich editor using the framework's existing provider ABI.
#[repr(C)]
pub struct RichEditBoxAutomationPeer {
    base: ControlAutomationPeer,
    text_automation: Rc<RichTextAutomationState>,
}
ferro_class!(RichEditBoxAutomationPeer: ControlAutomationPeer);
ferroui_base::ferro_class_info!(RichEditBoxAutomationPeer {
    interfaces: [
        Rc<dyn IValueProvider> => ProviderAdapter::as_value_provider,
        Rc<dyn ITextProvider> => ProviderAdapter::as_text_provider,
        Rc<dyn ITextProvider2> => ProviderAdapter::as_text_provider2,
        Rc<dyn ITextEditProvider> => ProviderAdapter::as_text_edit_provider,
    ]
});
impl ControlAutomationPeerImpl for RichEditBoxAutomationPeer {}
impl AutomationPeerImpl for RichEditBoxAutomationPeer {
    fn get_automation_control_type_core(_this: &Self) -> AutomationControlType {
        AutomationControlType::Edit
    }
    fn get_placeholder_text_core(this: &Self) -> Option<String> {
        this.try_owner()
            .map(|owner| owner.placeholder_text())
            .filter(|text| !text.is_empty())
    }
    fn get_name_core(this: &Self) -> Option<String> {
        let owner = this.try_owner()?;
        Self::parent_get_name_core(this)
            .filter(|name| !name.trim().is_empty())
            .or_else(|| presenter_name(owner.header_presenter(), owner.header()))
    }
    fn get_help_text_core(this: &Self) -> Option<String> {
        let owner = this.try_owner()?;
        Self::parent_get_help_text_core(this)
            .filter(|text| !text.trim().is_empty())
            .or_else(|| presenter_name(owner.description_presenter(), owner.description()))
    }
    fn get_or_create_children_core(this: &Self) -> Rc<Vec<Ref<AutomationPeer>>> {
        this.text_host().map_or_else(
            || Rc::new(Vec::new()),
            |host| {
                this.text_automation
                    .children(&host, &this.text_peer(), None)
            },
        )
    }
}
impl FerroObjectImpl for RichEditBoxAutomationPeer {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.text_automation.observe(this.to_ref());
        let weak = this.to_ref().downgrade();
        this.owner().property_changed(move |event| {
            if event.property() == RichEditBox::text_property().as_property() {
                if let Some(this) = weak.upgrade() {
                    this.raise_property_changed_event(
                        ValuePatternIdentifiers::value_property(),
                        event
                            .get_old_value::<String>()
                            .map(|value| Rc::new(value) as BoxedValue),
                        Some(Rc::new(event.get_new_value::<String>()) as BoxedValue),
                    );
                }
            }
        });
    }
}
impl RichEditBoxAutomationPeer {
    pub fn construct(owner: &RichEditBox) -> Self {
        Self {
            base: ControlAutomationPeer::construct(owner),
            text_automation: RichTextAutomationState::new(),
        }
    }
    pub fn new(owner: &RichEditBox) -> Ref<Self> {
        instantiate(Self::construct(owner))
    }
    pub fn owner(&self) -> Ref<RichEditBox> {
        self.try_owner()
            .expect("The owner of the peer is a rich editor.")
    }
    pub fn try_owner(&self) -> Option<Ref<RichEditBox>> {
        ControlAutomationPeer::try_owner(self).and_then(|owner| owner.cast())
    }
    pub fn is_read_only(&self) -> bool {
        self.try_owner().is_none_or(|owner| owner.is_read_only())
    }
    pub fn value(&self) -> String {
        self.owner().text()
    }
    pub fn set_value(&self, value: &str) -> Result<(), ProviderError> {
        let owner = ControlAutomationPeer::try_owner(self)
            .and_then(|owner| owner.cast::<RichEditBox>())
            .ok_or(TextProviderError::ElementNotAvailable)?;
        if !owner.is_effectively_enabled() {
            return Err(Box::new(ElementNotEnabledException::new()));
        }
        if owner.is_read_only() {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "The rich text document is read-only.",
            )));
        }
        let editor = owner.editor();
        editor
            .set_selection(crate::rich_text::EditorSelection::new(
                0,
                owner.document().text_length(),
            ))
            .map_err(|error| Box::new(std::io::Error::other(error.to_string())) as ProviderError)?;
        editor
            .insert_text(value, crate::rich_text::EditKind::Programmatic)
            .map_err(|error| Box::new(std::io::Error::other(error.to_string())) as ProviderError)?;
        let selection = editor.selection();
        owner.select(selection.anchor, selection.active);
        Ok(())
    }
}

impl RichTextAutomationPeer for RichEditBoxAutomationPeer {
    fn text_host(&self) -> Option<RichTextAutomationHost> {
        self.try_owner()
            .map(|owner| RichTextAutomationHost::Editor(owner.downgrade()))
    }
    fn text_peer(&self) -> Ref<AutomationPeer> {
        self.to_ref().upcast()
    }
    fn text_automation_state(&self) -> &Rc<RichTextAutomationState> {
        &self.text_automation
    }
}
impl IValueProvider for ProviderAdapter<RichEditBoxAutomationPeer> {
    fn peer(&self) -> Ref<AutomationPeer> {
        self.0.clone().upcast()
    }
    fn is_read_only(&self) -> bool {
        self.0.is_read_only()
    }
    fn value(&self) -> Option<String> {
        self.0.try_owner().map(|owner| owner.text())
    }
    fn set_value(&self, value: Option<&str>) -> Result<(), ProviderError> {
        self.0.set_value(value.unwrap_or_default())
    }
}

fn presenter_name(
    presenter: Option<Ref<crate::presenters::ContentPresenter>>,
    content: Option<BoxedValue>,
) -> Option<String> {
    presenter
        .and_then(|presenter| presenter.child())
        .map(|child| ControlAutomationPeer::create_peer_for_element(&child).get_name())
        .filter(|name| !name.trim().is_empty())
        .or_else(|| content.map(|content| ValueTypes::to_display_string(Some(&content))))
        .filter(|name| !name.trim().is_empty())
}
