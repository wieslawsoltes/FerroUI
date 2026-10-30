use super::{
    AutomationControlType, AutomationPeer, AutomationPeerImpl, ControlAutomationPeer,
    ControlAutomationPeerImpl,
};
use crate::automation::provider::{
    ITextProvider, ITextProvider2, ProviderAdapter, RichTextAutomationHost, RichTextAutomationPeer,
    RichTextAutomationState,
};
use crate::RichTextBlock;
use ferroui_base::{ferro_class, instantiate, FerroObjectImpl, FerroObjectImplExt, Ref};
use std::rc::Rc;

/// Accessibility projection of the rich view, with live document ranges,
/// selection, effective formatting, embedded children, and shaped geometry.
#[repr(C)]
pub struct RichTextBlockAutomationPeer {
    base: ControlAutomationPeer,
    text_automation: Rc<RichTextAutomationState>,
}
ferro_class!(RichTextBlockAutomationPeer: ControlAutomationPeer);
ferroui_base::ferro_class_info!(RichTextBlockAutomationPeer {
    interfaces: [
        Rc<dyn ITextProvider> => ProviderAdapter::as_text_provider,
        Rc<dyn ITextProvider2> => ProviderAdapter::as_text_provider2,
    ]
});
impl FerroObjectImpl for RichTextBlockAutomationPeer {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.text_automation.observe(this.to_ref());
    }
}
impl ControlAutomationPeerImpl for RichTextBlockAutomationPeer {}
impl AutomationPeerImpl for RichTextBlockAutomationPeer {
    fn get_automation_control_type_core(_this: &Self) -> AutomationControlType {
        AutomationControlType::Text
    }
    fn get_name_core(this: &Self) -> Option<String> {
        Some(this.owner().document().text())
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
impl RichTextAutomationPeer for RichTextBlockAutomationPeer {
    fn text_host(&self) -> Option<RichTextAutomationHost> {
        ControlAutomationPeer::try_owner(self)
            .and_then(|owner| owner.cast::<RichTextBlock>())
            .map(|view| RichTextAutomationHost::Block(view.downgrade()))
    }
    fn text_peer(&self) -> Ref<AutomationPeer> {
        self.to_ref().upcast()
    }
    fn text_automation_state(&self) -> &Rc<RichTextAutomationState> {
        &self.text_automation
    }
}
impl RichTextBlockAutomationPeer {
    pub fn construct(owner: &RichTextBlock) -> Self {
        Self {
            base: ControlAutomationPeer::construct(owner),
            text_automation: RichTextAutomationState::new(),
        }
    }
    pub fn new(owner: &RichTextBlock) -> Ref<Self> {
        instantiate(Self::construct(owner))
    }
    pub fn owner(&self) -> Ref<RichTextBlock> {
        ControlAutomationPeer::owner(self)
            .cast()
            .expect("The owner is a rich text view.")
    }
}
