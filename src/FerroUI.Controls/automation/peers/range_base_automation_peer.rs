use super::{AutomationPeer, AutomationPeerImpl, ControlAutomationPeer, ControlAutomationPeerImpl};
use crate::automation::provider::{IRangeValueProvider, ProviderAdapter};
use crate::automation::{ElementNotEnabledException, RangeValuePatternIdentifiers};
use crate::primitives::RangeBase;
use ferroui_base::{
    ferro_class, BoxedValue, FerroObjectImpl, FerroObjectImplExt, FerroPropertyChangedEventArgs, Ref,
};
use std::rc::Rc;

/// The base class of the automation peers which represent a [`RangeBase`].
#[repr(C)]
pub struct RangeBaseAutomationPeer {
    base: ControlAutomationPeer,
}

ferro_class! {
    RangeBaseAutomationPeer: ControlAutomationPeer, virtuals RangeBaseAutomationPeerImpl: ControlAutomationPeerImpl {
        /// Whether the value of the control is read-only (the range value
        /// provider contract).
        fn is_read_only(this) -> bool;
        /// Called when a property of the owning control has changed.
        fn owner_property_changed(this, e: &FerroPropertyChangedEventArgs<'_>);
    }
}
ferroui_base::ferro_class_info!(RangeBaseAutomationPeer {
    interfaces: [Rc<dyn IRangeValueProvider> => ProviderAdapter::as_range_value_provider]
});

impl FerroObjectImpl for RangeBaseAutomationPeer {
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

impl ControlAutomationPeerImpl for RangeBaseAutomationPeer {}
impl AutomationPeerImpl for RangeBaseAutomationPeer {}

impl RangeBaseAutomationPeerImpl for RangeBaseAutomationPeer {
    fn is_read_only(_this: &Self) -> bool {
        false
    }

    fn owner_property_changed(this: &Self, e: &FerroPropertyChangedEventArgs<'_>) {
        let property = e.property();

        let identifier = if property == RangeBase::minimum_property().as_property() {
            RangeValuePatternIdentifiers::minimum_property()
        } else if property == RangeBase::maximum_property().as_property() {
            RangeValuePatternIdentifiers::maximum_property()
        } else if property == RangeBase::value_property().as_property() {
            RangeValuePatternIdentifiers::value_property()
        } else {
            return;
        };

        this.raise_property_changed_event(
            identifier,
            e.get_old_value::<f64>().map(|value| Rc::new(value) as BoxedValue),
            Some(Rc::new(e.get_new_value::<f64>()) as BoxedValue),
        );
    }
}

impl IRangeValueProvider for ProviderAdapter<RangeBaseAutomationPeer> {
    fn peer(&self) -> Ref<AutomationPeer> {
        self.0.clone().upcast()
    }

    fn is_read_only(&self) -> bool {
        self.0.is_read_only()
    }

    fn minimum(&self) -> f64 {
        self.0.minimum()
    }

    fn maximum(&self) -> f64 {
        self.0.maximum()
    }

    fn value(&self) -> f64 {
        self.0.value()
    }

    fn large_change(&self) -> f64 {
        self.0.large_change()
    }

    fn small_change(&self) -> f64 {
        self.0.small_change()
    }

    fn set_value(&self, value: f64) -> Result<(), ElementNotEnabledException> {
        self.0.set_value(value)
    }
}

impl RangeBaseAutomationPeer {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    /// The class is abstract: it is the base of the peers of the range
    /// controls.
    pub fn construct(owner: &RangeBase) -> Self {
        Self { base: ControlAutomationPeer::construct(owner) }
    }

    /// Gets the owning range control.
    pub fn owner(&self) -> Ref<RangeBase> {
        ControlAutomationPeer::owner(self).cast().expect("The owner of the peer is a range control.")
    }

    /// The maximum value of the control (the range value provider contract).
    pub fn maximum(&self) -> f64 {
        self.owner().maximum()
    }

    /// The minimum value of the control (the range value provider contract).
    pub fn minimum(&self) -> f64 {
        self.owner().minimum()
    }

    /// The value of the control (the range value provider contract).
    pub fn value(&self) -> f64 {
        self.owner().value()
    }

    /// The small change of the control (the range value provider contract).
    pub fn small_change(&self) -> f64 {
        self.owner().small_change()
    }

    /// The large change of the control (the range value provider contract).
    pub fn large_change(&self) -> f64 {
        self.owner().large_change()
    }

    /// Sets the value of the control (the range value provider contract).
    pub fn set_value(&self, value: f64) -> Result<(), ElementNotEnabledException> {
        self.owner().set_range_value(value);
        Ok(())
    }
}
