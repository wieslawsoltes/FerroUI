use super::{
    AutomationControlType, AutomationPeer, AutomationPeerImpl, AutomationPeerImplExt, ControlAutomationPeer, ControlAutomationPeerImpl,
};
use crate::automation::provider::{IValueProvider, ProviderError, ProviderAdapter};
use crate::automation::{ElementNotEnabledException, ValuePatternIdentifiers};
use crate::TextBox;
use ferroui_base::{
    ferro_class, instantiate, BoxedValue, FerroObjectImpl, FerroObjectImplExt, FerroPropertyChangedEventArgs, Ref,
};
use ferroui_base::data::core::ValueType;
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
    fn get_provider_core(this: &Self, provider_type: ValueType) -> Option<BoxedValue> {
        if provider_type == ValueType::of::<Rc<dyn IValueProvider>>() && this.accessible_text_owner().is_none() {
            None
        } else {
            Self::parent_get_provider_core(this, provider_type)
        }
    }

    fn get_or_create_children_core(this: &Self) -> Rc<Vec<Ref<AutomationPeer>>> {
        // Check the current owner even before this peer's property observer runs. A peer
        // may already have cached normal template children when PasswordChar changes.
        if this.accessible_text_owner().is_none() {
            Rc::default()
        } else {
            Self::parent_get_or_create_children_core(this)
        }
    }

    fn get_automation_control_type_core(_this: &Self) -> AutomationControlType {
        AutomationControlType::Edit
    }

    fn get_placeholder_text_core(this: &Self) -> Option<String> {
        this.owner().placeholder_text()
    }
}

impl TextBoxAutomationPeerImpl for TextBoxAutomationPeer {
    fn owner_property_changed(this: &Self, e: &FerroPropertyChangedEventArgs<'_>) {
        if e.property() == TextBox::password_char_property().as_property() {
            this.invalidate_children();
        }
        if e.property() == TextBox::text_property().as_property() && this.accessible_text_owner().is_some() {
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

    // RevealPassword changes rendering, not the confidentiality of the password field.
    fn accessible_text_owner(&self) -> Option<Ref<TextBox>> {
        let owner = self.try_owner()?.cast::<TextBox>()?;
        (owner.password_char() == '\0').then_some(owner)
    }

    /// The text of an ordinary text box; password fields never disclose a value.
    /// This guard also covers provider handles retained before entering password mode.
    pub fn value(&self) -> Option<String> {
        self.accessible_text_owner().and_then(|owner| owner.text())
    }

    /// Sets the text of the text box (the value provider contract).
    pub fn set_value(&self, value: Option<&str>) -> Result<(), ElementNotEnabledException> {
        self.owner().set_text(value);
        Ok(())
    }
}
