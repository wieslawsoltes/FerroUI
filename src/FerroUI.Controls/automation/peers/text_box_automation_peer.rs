use super::{
    AutomationControlType, AutomationPeer, AutomationPeerImpl, ControlAutomationPeer, ControlAutomationPeerImpl,
};
use crate::automation::provider::{IValueProvider, ProviderError, ProviderAdapter};
use crate::automation::{ElementNotEnabledException, ValuePatternIdentifiers};
use crate::TextBox;
use ferroui_base::{
    ferro_class, instantiate, BoxedValue, FerroObjectImpl, FerroObjectImplExt, FerroPropertyChangedEventArgs, Ref,
};
use std::rc::Rc;

/// An automation peer which represents a [`TextBox`].
#[repr(C)]
pub struct TextBoxAutomationPeer {
    base: ControlAutomationPeer,
}

ferro_class! {
    TextBoxAutomationPeer: ControlAutomationPeer, virtuals TextBoxAutomationPeerImpl: ControlAutomationPeerImpl {
        /// Called when a property of the owning text box has changed.
        fn owner_property_changed(this, e: &FerroPropertyChangedEventArgs<'_>);
    }
}
ferroui_base::ferro_class_info!(TextBoxAutomationPeer {
    interfaces: [Rc<dyn IValueProvider> => ProviderAdapter::as_value_provider]
});

impl FerroObjectImpl for TextBoxAutomationPeer {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);

        let weak = this.to_ref().downgrade();
        this.owner().property_changed(move |e| {
            if let Some(this) = weak.upgrade() {
                this.owner_property_changed(e);
            }
        });
    }
}

impl ControlAutomationPeerImpl for TextBoxAutomationPeer {}

impl AutomationPeerImpl for TextBoxAutomationPeer {
    fn get_automation_control_type_core(_this: &Self) -> AutomationControlType {
        AutomationControlType::Edit
    }

    fn get_placeholder_text_core(this: &Self) -> Option<String> {
        this.owner().placeholder_text()
    }
}

impl TextBoxAutomationPeerImpl for TextBoxAutomationPeer {
    fn owner_property_changed(this: &Self, e: &FerroPropertyChangedEventArgs<'_>) {
        if e.property() == TextBox::text_property().as_property() {
            this.raise_property_changed_event(
                ValuePatternIdentifiers::value_property(),
                e.get_old_value::<Option<String>>().flatten().map(|value| Rc::new(value) as BoxedValue),
                e.get_new_value::<Option<String>>().map(|value| Rc::new(value) as BoxedValue),
            );
        }
    }
}

impl IValueProvider for ProviderAdapter<TextBoxAutomationPeer> {
    fn peer(&self) -> Ref<AutomationPeer> {
        self.0.clone().upcast()
    }

    fn is_read_only(&self) -> bool {
        self.0.is_read_only()
    }

    fn value(&self) -> Option<String> {
        self.0.value()
    }

    fn set_value(&self, value: Option<&str>) -> Result<(), ProviderError> {
        self.0.set_value(value).map_err(Into::into)
    }
}

impl TextBoxAutomationPeer {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct(owner: &TextBox) -> Self {
        Self { base: ControlAutomationPeer::construct(owner) }
    }

    /// Initializes a new peer of `owner`.
    pub fn new(owner: &TextBox) -> Ref<Self> {
        instantiate(Self::construct(owner))
    }

    /// Gets the owning text box.
    pub fn owner(&self) -> Ref<TextBox> {
        ControlAutomationPeer::owner(self).cast().expect("The owner of the peer is a text box.")
    }

    /// Whether the text box is read-only (the value provider contract).
    pub fn is_read_only(&self) -> bool {
        self.owner().is_read_only()
    }

    /// The text of the text box (the value provider contract).
    pub fn value(&self) -> Option<String> {
        self.owner().text()
    }

    /// Sets the text of the text box (the value provider contract).
    pub fn set_value(&self, value: Option<&str>) -> Result<(), ElementNotEnabledException> {
        self.owner().set_text(value);
        Ok(())
    }
}
