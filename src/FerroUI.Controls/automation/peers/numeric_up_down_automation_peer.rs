use super::{
    AutomationControlType, AutomationPeer, AutomationPeerImpl, ControlAutomationPeer, ControlAutomationPeerImpl,
};
use crate::automation::provider::{IRangeValueProvider, ProviderAdapter};
use crate::automation::{ElementNotEnabledException, RangeValuePatternIdentifiers};
use crate::NumericUpDown;
use ferroui_base::utilities::Decimal;
use ferroui_base::{
    ferro_class, instantiate, BoxedValue, FerroObjectImpl, FerroObjectImplExt, FerroPropertyChangedEventArgs, Ref,
};
use std::rc::Rc;

/// An automation peer which represents a [`NumericUpDown`].
#[repr(C)]
pub struct NumericUpDownAutomationPeer {
    base: ControlAutomationPeer,
}

ferro_class!(NumericUpDownAutomationPeer: ControlAutomationPeer);
ferroui_base::ferro_class_info!(NumericUpDownAutomationPeer {
    interfaces: [Rc<dyn IRangeValueProvider> => ProviderAdapter::as_range_value_provider]
});

impl FerroObjectImpl for NumericUpDownAutomationPeer {
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

impl ControlAutomationPeerImpl for NumericUpDownAutomationPeer {}

impl AutomationPeerImpl for NumericUpDownAutomationPeer {
    fn get_automation_control_type_core(_this: &Self) -> AutomationControlType {
        AutomationControlType::Spinner
    }

    fn get_class_name_core(_this: &Self) -> String {
        "NumericUpDown".to_owned()
    }
}

impl IRangeValueProvider for ProviderAdapter<NumericUpDownAutomationPeer> {
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

impl NumericUpDownAutomationPeer {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct(owner: &NumericUpDown) -> Self {
        Self { base: ControlAutomationPeer::construct(owner) }
    }

    /// Initializes a new peer of `owner`.
    pub fn new(owner: &NumericUpDown) -> Ref<Self> {
        instantiate(Self::construct(owner))
    }

    /// Gets the owning numeric up-down control.
    pub fn owner(&self) -> Ref<NumericUpDown> {
        ControlAutomationPeer::owner(self).cast().expect("The owner of the peer is a numeric up-down control.")
    }

    /// Whether the control is read-only (the range value provider contract).
    pub fn is_read_only(&self) -> bool {
        self.owner().is_read_only()
    }

    /// The maximum value of the control (the range value provider contract).
    pub fn maximum(&self) -> f64 {
        self.owner().maximum().to_f64()
    }

    /// The minimum value of the control (the range value provider contract).
    pub fn minimum(&self) -> f64 {
        self.owner().minimum().to_f64()
    }

    /// The value of the control, or zero clamped to the range of the control
    /// if it has no value (the range value provider contract).
    pub fn value(&self) -> f64 {
        let owner = self.owner();
        match owner.value() {
            Some(value) => value.to_f64(),
            None => Decimal::ZERO.clamp(owner.minimum(), owner.maximum()).to_f64(),
        }
    }

    /// The increment of the control (the range value provider contract).
    pub fn small_change(&self) -> f64 {
        self.owner().increment().to_f64()
    }

    /// The increment of the control (the range value provider contract).
    pub fn large_change(&self) -> f64 {
        self.owner().increment().to_f64()
    }

    /// Sets the value of the control (the range value provider contract).
    pub fn set_value(&self, value: f64) -> Result<(), ElementNotEnabledException> {
        self.owner().set_numeric_value(Some(Decimal::from_f64(value)));
        Ok(())
    }

    fn owner_property_changed(&self, e: &FerroPropertyChangedEventArgs<'_>) {
        let property = e.property();

        if property == NumericUpDown::minimum_property().as_property() {
            self.raise_property_changed_event(
                RangeValuePatternIdentifiers::minimum_property(),
                e.get_old_value::<Decimal>().map(|value| Rc::new(value) as BoxedValue),
                Some(Rc::new(e.get_new_value::<Decimal>()) as BoxedValue),
            );
        } else if property == NumericUpDown::maximum_property().as_property() {
            self.raise_property_changed_event(
                RangeValuePatternIdentifiers::maximum_property(),
                e.get_old_value::<Decimal>().map(|value| Rc::new(value) as BoxedValue),
                Some(Rc::new(e.get_new_value::<Decimal>()) as BoxedValue),
            );
        } else if property == NumericUpDown::value_property().as_property() {
            self.raise_property_changed_event(
                RangeValuePatternIdentifiers::value_property(),
                e.get_old_value::<Option<Decimal>>().flatten().map(|value| Rc::new(value) as BoxedValue),
                e.get_new_value::<Option<Decimal>>().map(|value| Rc::new(value) as BoxedValue),
            );
        } else if property == NumericUpDown::is_read_only_property().as_property() {
            self.raise_property_changed_event(
                RangeValuePatternIdentifiers::is_read_only_property(),
                e.get_old_value::<bool>().map(|value| Rc::new(value) as BoxedValue),
                Some(Rc::new(e.get_new_value::<bool>()) as BoxedValue),
            );
        }
    }
}
