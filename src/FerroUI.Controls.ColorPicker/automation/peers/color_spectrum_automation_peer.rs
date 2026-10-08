use crate::primitives::ColorSpectrum;
use crate::ColorChangedEventArgs;
use ferroui_base::media::Color;
use ferroui_base::utilities::FormatError;
use ferroui_base::{ferro_class, instantiate, BoxedValue, FerroObjectImpl, FerroObjectImplExt, Ref};
use ferroui_controls::automation::peers::{
    AutomationControlType, AutomationPeer, AutomationPeerImpl, ControlAutomationPeer, ControlAutomationPeerImpl,
};
use ferroui_controls::automation::provider::{IValueProvider, ProviderError};
use ferroui_controls::automation::ValuePatternIdentifiers;
use std::rc::Rc;

/// An automation peer which represents a [`ColorSpectrum`].
#[repr(C)]
pub struct ColorSpectrumAutomationPeer {
    base: ControlAutomationPeer,
}

ferro_class!(ColorSpectrumAutomationPeer: ControlAutomationPeer);
ferroui_base::ferro_class_info!(ColorSpectrumAutomationPeer {
    interfaces: [Rc<dyn IValueProvider> => ColorSpectrumAutomationPeer::as_value_provider]
});

impl FerroObjectImpl for ColorSpectrumAutomationPeer {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);

        let weak = this.to_ref().downgrade();
        // The subscription lives as long as the owner, as the event handler of the original.
        let _ = this.owner().color_changed(move |e| {
            if let Some(this) = weak.upgrade() {
                this.owner_on_color_changed(e);
            }
        });
    }
}

impl ControlAutomationPeerImpl for ColorSpectrumAutomationPeer {}

impl AutomationPeerImpl for ColorSpectrumAutomationPeer {
    fn get_automation_control_type_core(_this: &Self) -> AutomationControlType {
        AutomationControlType::Custom
    }

    fn get_class_name_core(_this: &Self) -> String {
        "ColorSpectrum".to_string()
    }
}

/// The handle through which the peer implements [`IValueProvider`] (the
/// provider adapter of `ferroui-controls` serves the peers of that crate).
struct ValueProviderAdapter(Ref<ColorSpectrumAutomationPeer>);

impl IValueProvider for ValueProviderAdapter {
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

impl ColorSpectrumAutomationPeer {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct(owner: &ColorSpectrum) -> Self {
        Self { base: ControlAutomationPeer::construct(owner) }
    }

    /// Initializes a new peer of `owner`.
    pub fn new(owner: &ColorSpectrum) -> Ref<Self> {
        instantiate(Self::construct(owner))
    }

    /// Converts the handle of a peer to its value provider.
    fn as_value_provider(peer: Ref<Self>) -> Rc<dyn IValueProvider> {
        Rc::new(ValueProviderAdapter(peer))
    }

    /// Whether the value is read-only (the value provider contract): never.
    pub fn is_read_only(&self) -> bool {
        false
    }

    /// Gets the owning color spectrum.
    pub fn owner(&self) -> Ref<ColorSpectrum> {
        ControlAutomationPeer::owner(self).cast().expect("The owner of the peer is a color spectrum.")
    }

    /// The color of the spectrum as text (the value provider contract).
    pub fn value(&self) -> Option<String> {
        Some(self.owner().color().to_string())
    }

    /// Sets the color of the spectrum from text (the value provider
    /// contract). The text is parsed as [`Color::try_parse`] parses it; text
    /// that is not a color is an error (a `FormatException` in the original;
    /// DEVIATIONS.md, Colour picker).
    pub fn set_value(&self, value: Option<&str>) -> Result<(), FormatError> {
        let Some(color) = value.and_then(Color::try_parse) else {
            return Err(FormatError::from_string(format!("Invalid color string: '{}'.", value.unwrap_or_default())));
        };

        self.owner().set_color(color);
        Ok(())
    }

    fn owner_on_color_changed(&self, e: &ColorChangedEventArgs) {
        self.raise_property_changed_event(
            ValuePatternIdentifiers::value_property(),
            Some(Rc::new(e.old_color().to_string()) as BoxedValue),
            Some(Rc::new(e.new_color().to_string()) as BoxedValue),
        );
    }
}

#[cfg(test)]
#[path = "color_spectrum_automation_peer_tests.rs"]
mod color_spectrum_automation_peer_tests;
