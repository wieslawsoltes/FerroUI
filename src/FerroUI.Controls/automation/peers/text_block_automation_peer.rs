use super::{
    AutomationControlType, AutomationPeerImpl, AutomationPeerImplExt, ControlAutomationPeer, ControlAutomationPeerImpl,
};
use crate::automation::AutomationElementIdentifiers;
use crate::TextBlock;
use ferroui_base::{ferro_class, instantiate, BoxedValue, FerroObjectImpl, FerroObjectImplExt, Ref};
use std::rc::Rc;

/// An automation peer which represents a [`TextBlock`].
#[repr(C)]
pub struct TextBlockAutomationPeer {
    base: ControlAutomationPeer,
}

ferro_class!(TextBlockAutomationPeer: ControlAutomationPeer);
ferroui_base::ferro_class_info!(TextBlockAutomationPeer {});

impl FerroObjectImpl for TextBlockAutomationPeer {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);

        let weak = this.to_ref().downgrade();
        this.owner().property_changed(move |e| {
            if e.property() == TextBlock::text_property().as_property() {
                let Some(this) = weak.upgrade() else { return };
                this.raise_property_changed_event(
                    AutomationElementIdentifiers::name_property(),
                    e.get_old_value::<Option<String>>().flatten().map(|value| Rc::new(value) as BoxedValue),
                    e.get_new_value::<Option<String>>().map(|value| Rc::new(value) as BoxedValue),
                );
            }
        });
    }
}

impl ControlAutomationPeerImpl for TextBlockAutomationPeer {}

impl AutomationPeerImpl for TextBlockAutomationPeer {
    fn get_automation_control_type_core(_this: &Self) -> AutomationControlType {
        AutomationControlType::Text
    }

    fn get_name_core(this: &Self) -> Option<String> {
        let owner = this.owner();
        owner.inlines().and_then(|inlines| inlines.text()).or_else(|| owner.text())
    }

    fn is_control_element_core(this: &Self) -> bool {
        // Return false if the control is part of a control template.
        this.owner().templated_parent().is_none() && Self::parent_is_control_element_core(this)
    }
}

impl TextBlockAutomationPeer {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct(owner: &TextBlock) -> Self {
        Self { base: ControlAutomationPeer::construct(owner) }
    }

    /// Initializes a new peer of `owner`.
    pub fn new(owner: &TextBlock) -> Ref<Self> {
        instantiate(Self::construct(owner))
    }

    /// Gets the owning text block.
    pub fn owner(&self) -> Ref<TextBlock> {
        ControlAutomationPeer::owner(self).cast().expect("The owner of the peer is a text block.")
    }
}
