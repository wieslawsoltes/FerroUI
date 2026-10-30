//! The automation peer of WinUI's `NumberBox` (`controls/dev/NumberBox/NumberBoxAutomationPeer.cpp`
//! of microsoft-ui-xaml, commit `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`; MIT, Copyright (c)
//! Microsoft Corporation, see NOTICE.md).

use super::{
    AutomationControlType, AutomationPeer, AutomationPeerImpl, AutomationPeerImplExt, ControlAutomationPeer,
    ControlAutomationPeerImpl,
};
use crate::automation::provider::{IRangeValueProvider, ProviderAdapter};
use crate::automation::{ElementNotEnabledException, RangeValuePatternIdentifiers};
use crate::NumberBox;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::{
    ferro_class, instantiate, BoxedValue, FerroObjectImpl, FerroObjectImplExt, FerroPropertyChangedEventArgs, Ref,
};
use std::rc::Rc;

/// An automation peer which represents a [`NumberBox`]: a spinner with the range value pattern,
/// named by its header when it has no name.
///
/// Deviation (docs/porting/winui-controls.md, Input): WinUI's peer raises the change of `Value`
/// only; this one also raises the changes of `Minimum`, `Maximum`, `SmallChange` and
/// `LargeChange` (the identifiers of W1), as the range peers of the framework do.
#[repr(C)]
pub struct NumberBoxAutomationPeer {
    base: ControlAutomationPeer,
}

ferro_class!(NumberBoxAutomationPeer: ControlAutomationPeer);
ferroui_base::ferro_class_info!(NumberBoxAutomationPeer {
    interfaces: [Rc<dyn IRangeValueProvider> => ProviderAdapter::as_range_value_provider]
});

impl FerroObjectImpl for NumberBoxAutomationPeer {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        let weak = this.to_ref().downgrade();
        this.owner().property_changed(move |change| {
            if let Some(this) = weak.upgrade() {
                this.owner_property_changed(change);
            }
        });
    }
}

impl ControlAutomationPeerImpl for NumberBoxAutomationPeer {}

impl AutomationPeerImpl for NumberBoxAutomationPeer {
    fn get_automation_control_type_core(_this: &Self) -> AutomationControlType {
        AutomationControlType::Spinner
    }

    fn get_class_name_core(_this: &Self) -> String {
        "NumberBox".to_owned()
    }

    fn get_name_core(this: &Self) -> Option<String> {
        // The name, or else the text of the header.
        Self::parent_get_name_core(this)
            .filter(|name| !name.is_empty())
            .or_else(|| this.owner().header().and_then(|header| ValueTypes::try_to_string(&*header)))
    }
}

impl IRangeValueProvider for ProviderAdapter<NumberBoxAutomationPeer> {
    fn peer(&self) -> Ref<AutomationPeer> {
        self.0.clone().upcast()
    }

    // NumberBoxAutomationPeer.h has no IsReadOnly of its own: the range is always writable.
    fn is_read_only(&self) -> bool {
        false
    }

    fn minimum(&self) -> f64 {
        self.0.owner().minimum()
    }

    fn maximum(&self) -> f64 {
        self.0.owner().maximum()
    }

    fn value(&self) -> f64 {
        self.0.owner().value()
    }

    fn small_change(&self) -> f64 {
        self.0.owner().small_change()
    }

    fn large_change(&self) -> f64 {
        self.0.owner().large_change()
    }

    fn set_value(&self, value: f64) -> Result<(), ElementNotEnabledException> {
        self.0.owner().set_numeric_value(value);
        Ok(())
    }
}

impl NumberBoxAutomationPeer {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct(owner: &NumberBox) -> Self {
        Self { base: ControlAutomationPeer::construct(owner) }
    }

    /// Initializes a new peer of `owner`.
    pub fn new(owner: &NumberBox) -> Ref<Self> {
        instantiate(Self::construct(owner))
    }

    /// Gets the owning number box.
    pub fn owner(&self) -> Ref<NumberBox> {
        ControlAutomationPeer::owner(self).cast().expect("The owner of the peer is a number box.")
    }

    /// Raises the change of the value of the range value pattern (WinUI's
    /// `RaiseValueChangedEvent`), after the `ValueChanged` event of the box, with the coerced
    /// value.
    pub(crate) fn raise_value_changed_event(&self, old_value: f64, new_value: f64) {
        self.raise_property_changed_event(
            RangeValuePatternIdentifiers::value_property(),
            Some(Rc::new(old_value) as BoxedValue),
            Some(Rc::new(new_value) as BoxedValue),
        );
    }

    fn owner_property_changed(&self, change: &FerroPropertyChangedEventArgs<'_>) {
        let property = change.property();
        let automation_property = if property == NumberBox::minimum_property().as_property() {
            RangeValuePatternIdentifiers::minimum_property()
        } else if property == NumberBox::maximum_property().as_property() {
            RangeValuePatternIdentifiers::maximum_property()
        } else if property == NumberBox::small_change_property().as_property() {
            RangeValuePatternIdentifiers::small_change_property()
        } else if property == NumberBox::large_change_property().as_property() {
            RangeValuePatternIdentifiers::large_change_property()
        } else {
            return;
        };
        let old_value = change.get_old_value::<f64>();
        let new_value = change.get_new_value::<f64>();
        if old_value.is_some_and(|old_value| old_value == new_value) {
            return;
        }
        self.raise_property_changed_event(
            automation_property,
            old_value.map(|value| Rc::new(value) as BoxedValue),
            Some(Rc::new(new_value) as BoxedValue),
        );
    }
}
