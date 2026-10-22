//! Range-value automation for the floating-point number editor.

use crate::NumberBox;
use crate::automation::peers::{
    AutomationControlType, AutomationPeer, AutomationPeerImpl, AutomationPeerImplExt,
    ControlAutomationPeer, ControlAutomationPeerImpl,
};
use crate::automation::provider::{IRangeValueProvider, ProviderAdapter};
use crate::automation::{ElementNotEnabledException, RangeValuePatternIdentifiers};
use ferroui_base::{
    ferro_class, ferro_class_info, instantiate, BoxedValue, FerroObjectImpl, FerroObjectImplExt,
    FerroPropertyChangedEventArgs, Ref,
};
use std::rc::Rc;

#[repr(C)]
pub struct NumberBoxAutomationPeer {
    base: ControlAutomationPeer,
}

ferro_class!(NumberBoxAutomationPeer: ControlAutomationPeer);
ferro_class_info!(NumberBoxAutomationPeer {
    interfaces: [Rc<dyn IRangeValueProvider> => ProviderAdapter::as_range_value_provider]
});

impl FerroObjectImpl for NumberBoxAutomationPeer {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        let weak = this.to_ref().downgrade();
        this.owner().property_changed(move |change| {
            if let Some(this) = weak.upgrade() {
                this.owner_changed(change);
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
        Self::parent_get_name_core(this)
            .filter(|name| !name.is_empty())
            .or_else(|| {
                this.owner()
                    .header()
                    .and_then(|header| header.downcast_ref::<String>().cloned())
            })
    }
}

impl NumberBoxAutomationPeer {
    pub fn new(owner: &NumberBox) -> Ref<Self> {
        instantiate(Self {
            base: ControlAutomationPeer::construct(owner),
        })
    }
    pub fn owner(&self) -> Ref<NumberBox> {
        ControlAutomationPeer::owner(self)
            .cast()
            .expect("NumberBoxAutomationPeer owns a NumberBox")
    }
    fn owner_changed(&self, change: &FerroPropertyChangedEventArgs<'_>) {
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
        let old = change.get_old_value::<f64>();
        let new = change.get_new_value::<f64>();
        if old.is_some_and(|old| old == new || old.is_nan() && new.is_nan()) {
            return;
        }
        self.raise_property_changed_event(
            automation_property,
            old.map(|value| Rc::new(value) as BoxedValue),
            Some(Rc::new(new) as BoxedValue),
        );
    }
    pub(crate) fn raise_value_changed(&self, old_value: f64, new_value: f64) {
        // Called after the user-facing ValueChanged event, once coercion has
        // completed. Raw property notifications can include an unclamped value.
        self.raise_property_changed_event(
            RangeValuePatternIdentifiers::value_property(),
            Some(Rc::new(old_value) as BoxedValue),
            Some(Rc::new(new_value) as BoxedValue),
        );
    }
}

impl IRangeValueProvider for ProviderAdapter<NumberBoxAutomationPeer> {
    fn peer(&self) -> Ref<AutomationPeer> {
        self.0.clone().upcast()
    }
    // NumberBoxAutomationPeer.h always exposes a writable range. IsEnabled
    // governs interactive input; programmatic pattern writes follow SetValue.
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
        let owner = self.0.owner();
        owner.set_numeric_value(value);
        Ok(())
    }
}
